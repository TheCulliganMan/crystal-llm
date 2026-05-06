import { gameEngine, type GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import { nextFrame } from "@pokecrystal/core/ui/async-loop";
import { GameState } from "@pokecrystal/core/core/state";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { pushDebugLog } from "@pokecrystal/core/core/debug-log";
import { isDebugEnabled } from "@pokecrystal/core/core/debug-flags";
import { getMapMetadataByGroup } from "@pokecrystal/core/engine/world/maps";
import {
  Event,
  EventManager,
  close_text,
  open_text,
  show_text,
  wait_for_input,
} from "@pokecrystal/core/engine/world/events";
import { record_last_pokecenter_heal, type Overworld as HealRecordOverworld } from "@pokecrystal/core/engine/world/special-events/map";
import { HealMachineAnimator } from "@pokecrystal/core/ui/screens/heal-machine-animation";
import type { PokemonCenterOverworld } from "@pokecrystal/core/engine/events/misc";
import type { SpecialOverworld } from "@pokecrystal/core/engine/world/special-events/special-types";

const FRAME_DURATION_MS = GB_FRAME_DURATION_MS;

const POKEMON_WORD = "#MON";

type DataLoaderLike = {
  move_data?: Record<string, { pp?: number }>;
  moveData?: Map<string, { pp?: number }> | Record<string, { pp?: number }>;
};

type PokemonLike = {
  species?: { id?: string | null } | null;
  nickname?: string | null;
  hp: number;
  max_hp?: number;
  status?: string | null;
  sleep_turns?: number;
  flinching?: boolean;
  rampage_turns?: number;
  confusion_turns?: number;
  turns_in_battle?: number;
  moves?: Array<
    | {
        name?: string;
        move?: string;
        pp?: number;
        current_pp?: number;
      }
    | null
  >;
};

type OverworldLike = Parameters<HealMachineAnimator["play"]>[2] & {
  current_map_name?: string;
  input_capture_active?: boolean;
  lock_player_movement?: () => void;
  unlock_player_movement?: () => void;
  lockPlayerMovement?: () => void;
  unlockPlayerMovement?: () => void;
  dialogue?: {
    visible?: boolean;
    active?: boolean;
    waiting_for_input?: boolean;
    pending_waits?: number;
    clear_script_waits?: () => void;
    window?: {
      has_more_pages?: () => boolean;
      is_complete?: () => boolean;
    };
  };
  ui?: { eventQueue?: GameEngineEventQueue; update?: () => void };
};


type ScriptRunnerLike = {
  variables?: Record<string, unknown>;
  last_yes_no_result?: boolean;
  last_condition_result?: boolean;
  command_map?: {
    yesno?: () => YesNoCommandLike;
  };
};

type YesNoCommandLike = {
  runner?: ScriptRunnerLike | null;
  on_result?: (value: boolean) => void;
  execute: (
    gameState: GameState,
    eventManager: EventManager | null | undefined,
    overworld: OverworldLike | null | undefined
  ) => void;
};

export class PartyHealingSummary {
  healedSlots: number[] = [];

  get anyChanges(): boolean {
    return this.healedSlots.length > 0;
  }
}

export class PokemonCenterSystem {
  private readonly healMachineAnimator = new HealMachineAnimator();
  private activeNurseInteraction: Promise<void> | null = null;

  private readonly greetingText: Record<string, string> = {
    morn: `Good morning!\nWelcome to our ${POKEMON_WORD} CENTER.`,
    day: `Hello!\nWelcome to our ${POKEMON_WORD} CENTER.`,
    nite: `Good evening!\nYou're out late.\n\nWelcome to our ${POKEMON_WORD} CENTER.`,
  };

  private readonly askHealText = `We can heal your ${POKEMON_WORD} to perfect health.\n\nShall we heal your ${POKEMON_WORD}?`;
  private readonly takePokemonText = `OK, may I see your ${POKEMON_WORD}?`;
  private readonly returnPokemonText = `Thank you for waiting.\n\nYour ${POKEMON_WORD} are fully healed.`;
  private readonly pokerusText = `Your ${POKEMON_WORD}\nappear to be\n\ninfected by tiny\nlife forms.\n\nYour ${POKEMON_WORD} are\nhealthy and seem\nto be fine.\n\nBut we can't tell\nyou anything more\n\nat a ${POKEMON_WORD}\nCENTER.`;
  private readonly goodbyeText = "We hope to see you again.";

  constructor(
    private readonly gameState: GameState,
    private readonly dataLoader: DataLoaderLike | null = null
  ) {}

  healParty(): PartyHealingSummary {
    const summary = new PartyHealingSummary();
    const party = this.gameState.sram.party.pokemon ?? [];
    party.forEach((mon, index) => {
      if (!mon) {
        return;
      }
      if (this.isEgg(mon as PokemonLike)) {
        return;
      }
      if (this.healSinglePokemon(mon as PokemonLike)) {
        summary.healedSlots.push(index);
      }
    });
    return summary;
  }

  heal_party(): { healed_slots: number[] } {
    const summary = this.healParty();
    return { healed_slots: summary.healedSlots };
  }

  runNurseInteraction(
    runner: ScriptRunnerLike,
    eventManager: EventManager,
    overworld: OverworldLike | null
  ): Promise<void> {
    if (this.activeNurseInteraction) {
      return this.activeNurseInteraction;
    }
    const queueTask =
      (runner as { _queue_overworld_task?: (task: (callback: () => void) => boolean | void) => void })
        ?._queue_overworld_task ??
      (runner as { _queueOverworldTask?: (task: (callback: () => void) => boolean | void) => void })
        ?._queueOverworldTask ??
      null;

    const execute = async (): Promise<void> => {
      await this.withInputCapture(overworld, async () => {
        await this.runNurseInteractionAsync(runner, eventManager, overworld);
      });
    };

    const interaction = typeof queueTask === "function"
      ? new Promise<void>((resolve) => {
        queueTask.call(runner, (callback: () => void) => {
          void execute()
            .catch((error) => {
              console.error("Pokemon Center nurse interaction failed:", error);
            })
            .finally(() => {
              callback();
              resolve();
            });
          return true;
        });
      })
      : execute();
    this.activeNurseInteraction = interaction.finally(() => {
      if (this.activeNurseInteraction === interaction) {
        this.activeNurseInteraction = null;
      }
    });
    return this.activeNurseInteraction;
  }

  playHealMachineAnimation(
    animationId: string | null,
    overworld: OverworldLike | PokemonCenterOverworld | SpecialOverworld | null
  ): void {
    if (this.isInstantMode()) {
      return;
    }
    const partySlots = this.determinePartyCount();
    const resolvedOverworld = overworld as OverworldLike | null;
    const resolvedAnimationId = this.resolveHealMachineAnimationId(animationId, resolvedOverworld);
    this.runHealMachineAnimationWithControls(
      resolvedOverworld,
      () => this.healMachineAnimator.play(resolvedAnimationId, partySlots, resolvedOverworld)
    );
  }

  async playHealMachineAnimationAsync(
    animationId: string | null,
    overworld: OverworldLike | PokemonCenterOverworld | SpecialOverworld | null
  ): Promise<void> {
    if (this.isInstantMode()) {
      return;
    }
    const partySlots = this.determinePartyCount();
    const resolvedOverworld = overworld as OverworldLike | null;
    const resolvedAnimationId = this.resolveHealMachineAnimationId(animationId, resolvedOverworld);
    await this.runHealMachineAnimationWithControlsAsync(
      resolvedOverworld,
      async () => {
        await this.healMachineAnimator.playAsync(
          resolvedAnimationId,
          partySlots,
          resolvedOverworld
        );
      }
    );
  }

  private resolveHealMachineAnimationId(
    animationId: string | null,
    overworld: OverworldLike | null
  ): string | null {
    const mapName = this.resolveCurrentMapName(overworld);
    const normalizedMapName = mapName.toUpperCase().replace(/[^A-Z0-9]/g, "");
    if (
      normalizedMapName.includes("POKECENTER") ||
      normalizedMapName.includes("POKEMONCENTER") ||
      normalizedMapName.includes("POKECOMCENTER")
    ) {
      // ASM mapping: std_scripts.asm::PokecenterNurseScript always uses HEALMACHINE_POKECENTER.
      return "HEALMACHINE_POKECENTER";
    }
    if (
      normalizedMapName.includes("ELMSLAB") ||
      normalizedMapName.includes("ELMLAB")
    ) {
      // ASM mapping: maps/ElmsLab.asm uses HEALMACHINE_ELMS_LAB.
      return "HEALMACHINE_ELMS_LAB";
    }
    if (normalizedMapName.includes("HALLOFFAME")) {
      // ASM mapping: maps/HallOfFame.asm uses HEALMACHINE_HALL_OF_FAME.
      return "HEALMACHINE_HALL_OF_FAME";
    }
    return animationId;
  }

  private resolveCurrentMapName(overworld: OverworldLike | null): string {
    const directName = String(overworld?.current_map_name ?? "").trim();
    if (directName) {
      return directName;
    }

    const wram = this.gameState.wram;
    const candidates: Array<[number, number]> = [
      [Number(wram.current_map_group ?? -1), Number(wram.current_map_id ?? -1)],
      [Number(wram.wMapGroup ?? -1), Number(wram.wMapNumber ?? -1)],
    ];

    for (const [group, id] of candidates) {
      if (!Number.isFinite(group) || !Number.isFinite(id) || group <= 0 || id <= 0) {
        continue;
      }
      const metadata = getMapMetadataByGroup(group, id);
      if (metadata?.name) {
        return metadata.name;
      }
    }
    return "";
  }

  private healSinglePokemon(mon: PokemonLike): boolean {
    let changed = false;
    if (typeof mon.max_hp === "number" && mon.hp !== mon.max_hp) {
      mon.hp = mon.max_hp;
      changed = true;
    }
    // ASM mapping: pokecrystal_disassembly/engine/pokemon/health.asm (HealPartyMon):
    // clear MON_STATUS, restore max HP, then RestoreAllPP.
    if (mon.status) {
      mon.status = null;
      changed = true;
    }

    (mon.moves ?? []).forEach((move) => {
      if (!move) {
        return;
      }
      const restored = this.determineMovePp(move);
      if (restored === null || restored === undefined) {
        return;
      }
      if (typeof move.current_pp === "number") {
        if (move.current_pp !== restored) {
          move.current_pp = restored;
          changed = true;
        }
      } else if (typeof move.pp === "number" && move.pp !== restored) {
        move.pp = restored;
        changed = true;
      }
    });

    return changed;
  }

  private isEgg(mon: PokemonLike): boolean {
    return mon.species?.id === "EGG" || mon.nickname === "EGG";
  }

  private determineMovePp(move: { name?: string; move?: string; pp?: number; current_pp?: number }): number | null {
    const moveName = move.name ?? move.move;
    if (!moveName) {
      return null;
    }
    const moveData = this.dataLoader?.move_data ?? null;
    const moveDataMap = this.dataLoader?.moveData ?? null;
    if (!moveData && !moveDataMap) {
      return move.current_pp ?? move.pp ?? null;
    }
    const definition =
      moveData?.[moveName] ??
      (moveDataMap instanceof Map ? moveDataMap.get(moveName) : moveDataMap?.[moveName]);
    if (!definition) {
      return move.current_pp ?? move.pp ?? null;
    }
    return definition.pp ?? move.current_pp ?? move.pp ?? null;
  }

  private selectGreeting(): string {
    const period = (this.gameState.wram.time_of_day ?? "day").toLowerCase();
    return this.greetingText[period] ?? this.greetingText.day;
  }

  private displayText(eventManager: EventManager, text: string): void {
    show_text(eventManager, text);
  }

  private openText(eventManager: EventManager): void {
    open_text(eventManager);
  }

  private closeText(eventManager: EventManager): void {
    close_text(eventManager);
  }

  private async waitForButton(
    eventManager: EventManager,
    overworld: OverworldLike | null,
    context: string
  ): Promise<void> {
    const dialogue = this.requireDialogue(overworld);
    const window = dialogue.window;
    const isWaitingForInput = (): boolean =>
      Boolean(dialogue.waiting_for_input) || Number(dialogue.pending_waits ?? 0) > 0;
    this.clearDialogueInputState(overworld);
    if (window && typeof window.is_complete === "function" && window.is_complete()) {
      return;
    }
    wait_for_input(eventManager);
    // EventManager dispatch is synchronous. If an already-queued confirm input consumes
    // the wait immediately, the start edge can be missed entirely; treat that as a
    // completed wait instead of stalling forever on a start predicate.
    if (!isWaitingForInput()) {
      return;
    }
    await this.waitForPredicateAsync(overworld, () => !isWaitingForInput(), `wait_for_input:end:${context}`);
  }

  private clearPokecomFlag(): void {
    const flags = this.gameState.wram.event_flags as Record<string, boolean>;
    if ("EVENT_WELCOMED_TO_POKECOM_CENTER" in flags) {
      flags["EVENT_WELCOMED_TO_POKECOM_CENTER"] = false;
    }
  }

  private async promptYesNo(
    runner: ScriptRunnerLike,
    eventManager: EventManager,
    overworld: OverworldLike | null,
    context: string
  ): Promise<boolean> {
    if (!overworld || !overworld.dialogue || !overworld.handleInput) {
      return this.runFallbackYesNoPrompt(runner, eventManager, overworld);
    }
    this.clearDialogueInputState(overworld);
    wait_for_input(eventManager);
    await this.awaitQuestionPage(overworld, context);
    const result: { value: boolean | null } = { value: null };
    const capture = (choice: boolean): void => {
      result.value = choice;
    };
    eventManager.dispatch(new Event("prompt_yes_no", { callback: capture }));
    await this.waitForPredicateAsync(
      overworld,
      () => result.value !== null,
      `prompt_yes_no:${context}`
    );
    const finalValue = Boolean(result.value);
    runner.last_yes_no_result = finalValue;
    runner.last_condition_result = finalValue;
    return finalValue;
  }

  private runFallbackYesNoPrompt(
    runner: ScriptRunnerLike,
    eventManager: EventManager,
    overworld: OverworldLike | null
  ): boolean {
    const commandFactory = runner.command_map?.yesno;
    if (typeof commandFactory === "function") {
      const command = commandFactory();
      command.runner = runner;
      const resultHolder: boolean[] = [];
      command.on_result = (value: boolean) => {
        resultHolder.splice(0, resultHolder.length, Boolean(value));
      };
      command.execute(this.gameState, eventManager, overworld);
      if (resultHolder.length > 0) {
        const finalValue = resultHolder[0];
        runner.last_yes_no_result = finalValue;
        runner.last_condition_result = finalValue;
        return finalValue;
      }
      const finalValue = Boolean(runner.last_yes_no_result ?? false);
      runner.last_yes_no_result = finalValue;
      runner.last_condition_result = finalValue;
      return finalValue;
    }

    if (!eventManager) {
      runner.last_yes_no_result = false;
      runner.last_condition_result = false;
      return false;
    }

    const result: { value: boolean | null } = { value: null };
    eventManager.dispatch(
      new Event("prompt_yes_no", {
        callback: (choice: boolean) => {
          result.value = Boolean(choice);
        },
      })
    );

    const finalValue = result.value ?? Boolean(runner.last_yes_no_result ?? false);
    runner.last_yes_no_result = finalValue;
    runner.last_condition_result = finalValue;
    return finalValue;
  }

  private async awaitQuestionPage(overworld: OverworldLike, context: string): Promise<void> {
    const dialogue = overworld.dialogue;
    if (!dialogue || !dialogue.window) {
      return;
    }
    const window = dialogue.window;
    const hasMorePages = window.has_more_pages?.bind(window);
    const isComplete = window.is_complete?.bind(window);
    if (typeof hasMorePages !== "function" || typeof isComplete !== "function") {
      return;
    }
    await this.waitForPredicateAsync(
      overworld,
      () => {
        if (hasMorePages()) {
          return false;
        }
        return Boolean(isComplete());
      },
      `await_question:${context}`
    );
  }

  private requireDialogue(
    overworld: OverworldLike | null
  ): {
    waiting_for_input?: boolean;
    pending_waits?: number;
    window?: { has_more_pages?: () => boolean; is_complete?: () => boolean };
  } {
    if (!overworld) {
      throw new Error("PokemonCenterSystem requires an overworld context to await input.");
    }
    const dialogue = overworld.dialogue;
    if (!dialogue) {
      throw new Error("Overworld is missing a dialogue manager; cannot wait.");
    }
    return dialogue;
  }

  private async waitForPredicateAsync(
    overworld: OverworldLike | null,
    predicate: () => boolean,
    context: string
  ): Promise<void> {
    if (!overworld) {
      throw new Error("PokemonCenterSystem requires an overworld context to await input.");
    }
    const start = Date.now();
    const shouldTimeout = typeof process !== "undefined" && process.env?.NODE_ENV === "test";
    const maxMs = shouldTimeout ? 5000 : Number.POSITIVE_INFINITY;
    const debugEnabled = isDebugEnabled("pokecenter");
    if (debugEnabled) {
      pushDebugLog("[pokecenter] processOverworldUntil start", { context });
    }
    const schedule =
      typeof window !== "undefined" && typeof window.requestAnimationFrame === "function"
        ? window.requestAnimationFrame.bind(window)
        : (callback: FrameRequestCallback) =>
            setTimeout(() => callback(Date.now()), FRAME_DURATION_MS);

    return new Promise((resolve, reject) => {
      const tick = (): void => {
        this.processOverworldInput(overworld);
        if (predicate()) {
          resolve();
          return;
        }
        if (Number.isFinite(maxMs) && Date.now() - start > maxMs) {
          if (debugEnabled) {
            pushDebugLog("[pokecenter] processOverworldUntil timeout", {
              context,
              elapsed_ms: Date.now() - start,
              waiting_for_input: Boolean(overworld.dialogue?.waiting_for_input),
              pending_waits: Number(overworld.dialogue?.pending_waits ?? 0),
            });
          }
          reject(
            new Error(
              `Pokemon Center interaction stalled (${context}). Check input handling and dialogue state.`
            )
          );
          return;
        }
        schedule(() => tick());
      };
      tick();
    });
  }

  private processOverworldInput(overworld: OverworldLike | null): void {
    if (typeof overworld?.handleInput !== "function") {
      return;
    }
    const queues: GameEngineEventQueue[] = [];
    const overworldQueue = overworld.ui?.eventQueue ?? null;
    if (overworldQueue) {
      queues.push(overworldQueue);
    }
    const activeQueue = gameEngine.event.getActiveQueue?.() ?? null;
    if (activeQueue && activeQueue !== overworldQueue) {
      queues.push(activeQueue);
    }
    if (queues.length === 0) {
      return;
    }
    for (const queue of queues) {
      const events = gameEngine.event.get(queue);
      if (!events.length) {
        continue;
      }
      for (const event of events) {
        overworld.handleInput(event);
      }
    }
  }

  private async runNurseInteractionAsync(
    runner: ScriptRunnerLike,
    eventManager: EventManager,
    overworld: OverworldLike | null
  ): Promise<void> {
    const debugEnabled = isDebugEnabled("pokecenter");
    if (debugEnabled) {
      pushDebugLog("[pokecenter] nurse interaction start", {
        map: String((overworld as { current_map_name?: string } | null)?.current_map_name ?? ""),
      });
    }
    this.clearDialogueInputState(overworld);
    this.openText(eventManager);
    const greeting = this.selectGreeting();
    this.displayText(eventManager, greeting);
    await this.waitForButton(eventManager, overworld, "greeting");
    this.clearPokecomFlag();

    this.displayText(eventManager, this.askHealText);
    const wantsHeal = await this.promptYesNo(runner, eventManager, overworld, "ask-heal");
    if (!wantsHeal) {
      this.displayText(eventManager, this.goodbyeText);
      await this.waitForButton(eventManager, overworld, "goodbye");
      this.closeText(eventManager);
      return;
    }

    this.displayText(eventManager, this.takePokemonText);
    await this.pauseFrames(20);
    // ASM: PokecenterNurseScript pause 10 after turning left.
    await this.pauseFrames(10);
    // Clear the prompt/text while the heal animation runs.
    this.closeText(eventManager);

    this.healParty();
    const overworldForRecord =
      overworld && typeof (overworld as HealRecordOverworld).load_map === "function"
        ? (overworld as HealRecordOverworld)
        : undefined;
    record_last_pokecenter_heal(this.gameState, { overworld: overworldForRecord });

    const audioEngine = overworld?.audio_engine as AudioEngine | null | undefined;
    // ASM: playmusic MUSIC_NONE before HealMachineAnim.
    audioEngine?.playMusic?.("MUSIC_NONE", "heal");

    // ASM: engine/events/std_scripts.asm::PokecenterNurseScript setval HEALMACHINE_POKECENTER.
    const healMachineType = HealMachineAnimator.HEAL_MACHINE_TYPE_MAP.HEALMACHINE_POKECENTER;
    if (runner.variables) {
      runner.variables["_value"] = healMachineType;
    }
    // ASM: engine/events/std_scripts.asm::PokecenterNurseScript -> HealMachineAnim
    await this.withPlayerMovementLocked(overworld, async () => {
      await this.playHealMachineAnimationAsync(String(healMachineType), overworld);
    });
    await this.pauseFrames(30);

    audioEngine?.restartMapMusic?.();

    // ASM: PokecenterNurseScript turnobject LAST_TALKED, DOWN; pause 10.
    await this.pauseFrames(10);

    if (this.shouldTriggerPokerusNurseBranch()) {
      this.openText(eventManager);
      this.displayText(eventManager, this.pokerusText);
      await this.waitForButton(eventManager, overworld, "pokerus");
      this.closeText(eventManager);
      this.markPokerusDiscovered();
      return;
    }

    this.openText(eventManager);
    this.displayText(eventManager, this.returnPokemonText);
    // ASM: PokecenterNurseScript pause 20 before NurseGoodbyeText.
    await this.pauseFrames(20);
    // ASM: farwritetext NurseGoodbyeText replaces the current textbox before the
    // single trailing waitbutton, so do not queue an extra confirm step here.
    this.closeText(eventManager);
    this.openText(eventManager);
    this.displayText(eventManager, this.goodbyeText);
    // ASM: PokecenterNurseScript turnobject LAST_TALKED, UP/DOWN with pause 10 each.
    await this.pauseFrames(10);
    await this.pauseFrames(10);
    await this.waitForButton(eventManager, overworld, "goodbye");
    this.closeText(eventManager);
  }

  private async pauseFrames(frames: number): Promise<void> {
    if (frames <= 0 || this.isInstantMode()) {
      return;
    }
    let remaining = Math.max(0, Math.ceil(frames));
    while (remaining > 0) {
      remaining -= 1;
      await nextFrame();
    }
  }

  private isInstantMode(): boolean {
    return Boolean(this.gameState.wram?.instant_mode);
  }

  private clearDialogueInputState(overworld: OverworldLike | null): void {
    const dialogue = overworld?.dialogue;
    if (!dialogue) {
      return;
    }
    const isActive = Boolean(dialogue.visible || dialogue.active);
    if (!isActive) {
      const waiting = Boolean(dialogue.waiting_for_input);
      const pending = Number(dialogue.pending_waits ?? 0);
      if (waiting || pending > 0) {
        dialogue.clear_script_waits?.();
        if (!dialogue.clear_script_waits) {
          dialogue.waiting_for_input = false;
          dialogue.pending_waits = 0;
        }
      }
    }
  }

  private async withPlayerMovementLocked(
    overworld: OverworldLike | null,
    action: () => Promise<void>
  ): Promise<void> {
    const lock =
      overworld?.lock_player_movement ??
      overworld?.lockPlayerMovement ??
      null;
    const unlock =
      overworld?.unlock_player_movement ??
      overworld?.unlockPlayerMovement ??
      null;
    lock?.call(overworld);
    try {
      await action();
    } finally {
      unlock?.call(overworld);
    }
  }

  private runHealMachineAnimationWithControls(
    overworld: OverworldLike | null,
    action: () => void
  ): void {
    const previousCapture = Boolean(overworld?.input_capture_active);
    const lock =
      overworld?.lock_player_movement ??
      overworld?.lockPlayerMovement ??
      null;
    const unlock =
      overworld?.unlock_player_movement ??
      overworld?.unlockPlayerMovement ??
      null;
    if (overworld) {
      overworld.input_capture_active = true;
    }
    lock?.call(overworld);
    try {
      action();
    } finally {
      unlock?.call(overworld);
      if (overworld) {
        overworld.input_capture_active = previousCapture;
      }
    }
  }

  private async runHealMachineAnimationWithControlsAsync(
    overworld: OverworldLike | null,
    action: () => Promise<void>
  ): Promise<void> {
    const previousCapture = Boolean(overworld?.input_capture_active);
    const lock =
      overworld?.lock_player_movement ??
      overworld?.lockPlayerMovement ??
      null;
    const unlock =
      overworld?.unlock_player_movement ??
      overworld?.unlockPlayerMovement ??
      null;
    if (overworld) {
      overworld.input_capture_active = true;
    }
    lock?.call(overworld);
    try {
      await action();
    } finally {
      unlock?.call(overworld);
      if (overworld) {
        overworld.input_capture_active = previousCapture;
      }
    }
  }

  private async withInputCapture(
    overworld: OverworldLike | null,
    action: () => Promise<void>
  ): Promise<void> {
    const previousCapture = Boolean(overworld?.input_capture_active);
    if (overworld) {
      overworld.input_capture_active = true;
    }
    try {
      await action();
    } finally {
      if (overworld) {
        overworld.input_capture_active = previousCapture;
      }
    }
  }

  private determinePartyCount(): number {
    const party = this.gameState.sram.party.pokemon ?? [];
    return Math.max(0, Math.min(6, party.filter((mon) => mon !== null).length));
  }

  private shouldTriggerPokerusNurseBranch(): boolean {
    const pendingCalls = this.gameState.wram.scheduled_phone_calls ?? [];
    if (pendingCalls.some((call) => String(call).toUpperCase().startsWith("SPECIALCALL_"))) {
      return false;
    }
    if (this.gameState.wram.engine_flags?.ENGINE_CAUGHT_POKERUS) {
      return false;
    }
    const party = this.gameState.sram.party.pokemon ?? [];
    return party.some((mon) => mon && !this.isEgg(mon as PokemonLike) && Boolean((mon as PokemonLike & { pokerus?: unknown }).pokerus));
  }

  private markPokerusDiscovered(): void {
    this.gameState.wram.engine_flags.ENGINE_CAUGHT_POKERUS = true;
    const queue = this.gameState.wram.scheduled_phone_calls ?? [];
    if (!queue.includes("SPECIALCALL_POKERUS")) {
      queue.push("SPECIALCALL_POKERUS");
    }
    this.gameState.wram.scheduled_phone_calls = queue;
  }
}

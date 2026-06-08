// ASM mapping: pokecrystal_disassembly/engine/events/pokecenter_pc.asm
// plus pokecrystal_disassembly/engine/pokemon/bills_pc_top.asm hub dispatch behavior.
import { GameState } from "../../core/state";
import { countPokedexEntries } from "../../core/pokedex";
import { AudioEngine } from "../../engine/systems/audio";
import { POKEMON_WORD } from "@pokecrystal/assets/content/text-constants";
import { Box, BoxSchema, clearSlot, formatDefaultBoxName, setSlot } from "../../core/models/box";
import { HallOfFameEntry, HallOfFamePokemon } from "../../core/models";
import { Pokemon, toPokemon } from "../../core/models/pokemon";
import { MAX_BOX_MONS, MAX_PC_BOXES } from "../../core/constants";
import { getFilledSlots } from "../../core/models/party";
import { saveGame } from "../../core/save";
import { MailMessage } from "../../core/mail";
import { BaseUI } from "../base-ui";
import { Surface } from "../surface";
import type { KeyEvent } from "../../input/buttons";
import { SelectionPrompt, isPromptUI } from "../text/prompts";
import { gameEngine, type GameEngineEventQueue } from "../game-engine";
import { PokedexRatingScreen, HallOfFameViewer, MailboxMenu } from "./pc-auxiliary";
import { PCHubMenu } from "./pc-hub-prompt";
import {
  PCHubAction,
  pcHubEntries as buildAsmPcHubEntries,
  type PCHubEntry,
} from "../../engine/world/special-events/pc-helpers";
import {
  PCMenuActionPayload,
  PCMenuActionResult,
  PCMenuInteractiveResponse,
  PokemonPCMenu,
  SupportsPokemonPCUI,
} from "./pc-components";
import { PlayerPCMenu, PlayerPCMenuActionResult, PlayerPCUI } from "./pc-player-menu";
import { playPcBootSound, playPcShutdownSound } from "./pc-views";
import { MenuUI } from "./types";
import type { ItemSystemDataLoader } from "@pokecrystal/core/engine/systems/items";
import { pushDebugLog, type DebugLogDetails } from "@pokecrystal/core/core/debug-log";
import { isDebugEnabled } from "@pokecrystal/core/core/debug-flags";

type HallOfFameSummary = {
  entries: string[][];
  status?: "empty";
  selected_index?: number;
  selected?: string[];
};

const canAssignDialoguePendingWaits = (dialogue: object): boolean => {
  let target: object | null = dialogue;
  while (target) {
    const descriptor = Object.getOwnPropertyDescriptor(target, "pending_waits");
    if (descriptor) {
      return Boolean(descriptor.writable || descriptor.set);
    }
    target = Object.getPrototypeOf(target);
  }
  return true;
};

const PC_TEXT_FALLBACKS: Record<string, string> = {
  PokecenterPCTurnOnText: "<PLAYER> turned on\nthe PC.",
  PokecenterPCWhoseText: "Access whose PC?",
  PokecenterPCCantUseText: "Bzzzzt! You must\nhave a #MON to\nuse this!",
  PokecenterPCOaksClosedText: "Turning off the PC.",
  PCString_NoReleasingEGGS: "No releasing EGGS!",
  PlayersPCTurnOnText: "<PLAYER> turned on\nthe PC.",
  PlayersPCAskWhatDoText: "What do you want\nto do?",
};

const asPokemonPCUI = (ui: unknown): SupportsPokemonPCUI | null => {
  if (!ui || typeof ui !== "object") {
    return null;
  }
  const candidate = ui as Partial<SupportsPokemonPCUI> & {
    draw_window?: SupportsPokemonPCUI["drawWindow"];
    draw_sprite?: SupportsPokemonPCUI["drawSprite"];
    getPokemonFrontSurface?: (speciesId: string, frame?: number) => unknown;
    get_sprite_surface?: (spriteId: string, spriteType: string) => unknown;
  };
  if (!candidate.screen || !candidate.font || typeof candidate.update !== "function") {
    return null;
  }
  const drawWindow =
    typeof candidate.drawWindow === "function"
      ? candidate.drawWindow.bind(candidate)
      : typeof candidate.draw_window === "function"
        ? candidate.draw_window.bind(candidate)
        : null;
  if (!drawWindow) {
    return null;
  }
  const drawSprite =
    typeof candidate.drawSprite === "function"
      ? candidate.drawSprite.bind(candidate)
      : typeof candidate.draw_sprite === "function"
        ? candidate.draw_sprite.bind(candidate)
        : (
            speciesId: string,
            x: number,
            y: number,
            spriteType: string = "pokemon",
            frame: number = 0,
          ): void => {
            const normalizedType = String(spriteType || "").trim().toLowerCase();
            const sprite =
              (normalizedType === "pokemon" || normalizedType === "pokemon_front") &&
              typeof candidate.getPokemonFrontSurface === "function"
                ? candidate.getPokemonFrontSurface.call(candidate, speciesId, frame)
                : typeof candidate.get_sprite_surface === "function"
                  ? candidate.get_sprite_surface.call(candidate, speciesId, normalizedType)
                  : null;
            if (sprite && candidate.screen && typeof candidate.screen.blit === "function") {
              candidate.screen.blit(sprite as Surface, [x, y]);
            }
          };
  const drawRect =
    typeof candidate.drawRect === "function"
      ? candidate.drawRect.bind(candidate)
      : () => undefined;
  const pollEvents =
    typeof candidate.pollEvents === "function"
      ? candidate.pollEvents.bind(candidate)
      : () => gameEngine.event.get((candidate as { eventQueue?: GameEngineEventQueue }).eventQueue);
  return {
    screen: candidate.screen,
    font: candidate.font,
    drawWindow,
    drawSprite,
    drawRect,
    update: candidate.update.bind(candidate),
    pollEvents,
    renderSnapshot:
      typeof candidate.renderSnapshot === "function"
        ? candidate.renderSnapshot.bind(candidate)
        : undefined,
  };
};

const isPlayerPCUI = (ui: unknown): ui is PlayerPCUI => {
  if (!ui || typeof ui !== "object") {
    return false;
  }
  return (
    typeof (ui as PlayerPCUI).drawWindow === "function" &&
    typeof (ui as PlayerPCUI).update === "function" &&
    typeof (ui as PlayerPCUI).font?.renderText === "function"
  );
};

const toMenuUI = (ui: SupportsPokemonPCUI | BaseUI | null): MenuUI | null => {
  if (!ui || typeof ui !== "object") {
    return null;
  }
  const candidate = ui as Partial<MenuUI> & { tileSize?: number };
  if (!candidate.screen || typeof candidate.drawWindow !== "function" || !candidate.font) {
    return null;
  }
  return {
    ...(candidate as MenuUI),
    tileSize: candidate.tileSize ?? 8,
    renderSnapshot:
      typeof candidate.renderSnapshot === "function"
        ? candidate.renderSnapshot.bind(candidate)
        : undefined,
  };
};

const resolveItemDataLoader = (
  loader: { getText?: (label: string) => string } | null | undefined,
): ItemSystemDataLoader | undefined => {
  if (!loader || typeof loader !== "object") {
    return undefined;
  }
  const candidate = loader as ItemSystemDataLoader & { itemData?: unknown; item_data?: unknown };
  if ("itemData" in candidate || "item_data" in candidate) {
    return candidate;
  }
  return undefined;
};

export class PokemonCenterPCSession {
  private activeBoxIndex = 0;
  private interactiveMode = false;
  private hubOptions: PCHubEntry[];

  constructor(
    private readonly gameState: GameState,
    opts?: {
      ui?: SupportsPokemonPCUI | PlayerPCUI | BaseUI | null;
      audioEngine?: AudioEngine | null;
      playersPc?: boolean;
      dataLoader?: { getText?: (label: string) => string };
      eventManager?: { showText?: (text: string) => void; waitForInput?: () => void };
      dialogue?: {
        visible?: boolean;
        waiting_for_input?: boolean;
        pending_waits?: number;
        clear_script_waits?: () => void;
        handle_input?: (event: unknown) => boolean;
        handleInput?: (event: unknown) => boolean;
      };
      drawCallback?: () => void;
    },
  ) {
    this.ui = opts?.ui ?? null;
    this.audioEngine = opts?.audioEngine ?? null;
    this.playersPc = Boolean(opts?.playersPc);
    this.dataLoader = opts?.dataLoader ?? null;
    this.eventManager = opts?.eventManager ?? null;
    this.dialogue = opts?.dialogue ?? null;
    this.drawCallback = opts?.drawCallback ?? null;
    this.ensureBoxes();
    this.hubOptions = buildAsmPcHubEntries(gameState);
  }

  private readonly ui: SupportsPokemonPCUI | PlayerPCUI | BaseUI | null;
  private readonly audioEngine: AudioEngine | null;
  private readonly playersPc: boolean;
  private readonly dataLoader: { getText?: (label: string) => string } | null;
  private readonly eventManager: { showText?: (text: string) => void; waitForInput?: () => void } | null;
  private readonly dialogue: {
    visible?: boolean;
    waiting_for_input?: boolean;
    pending_waits?: number;
    clear_script_waits?: () => void;
    handle_input?: (event: unknown) => boolean;
    handleInput?: (event: unknown) => boolean;
  } | null;
  private readonly drawCallback: (() => void) | null;

  public setHubOptions(entries: Array<{ label: string; action: string }>): void {
    this.hubOptions = entries.map((entry) => ({
      label: entry.label,
      action: entry.action as PCHubAction,
    }));
  }

  public async runAsync(opts?: {
    selection?: unknown;
    scriptedActions?: Record<string, Iterable<Record<string, unknown>> | Record<string, unknown> | null>;
  }): Promise<Record<string, unknown>> {
    const scriptedActions = opts?.scriptedActions ?? {};
    const hasSelection = opts?.selection !== undefined && opts?.selection !== null;
    const scriptedPath = hasSelection || Object.values(scriptedActions).some((value) => value);
    if (scriptedPath) {
      return this.run(opts);
    }
    this.logDebug("session start", {
      mode: "async",
      selection: opts?.selection === undefined || opts?.selection === null ? null : String(opts?.selection),
      scripted: scriptedPath,
      players_pc: this.playersPc,
    });
    this.activeBoxIndex = 0;
    this.interactiveMode = false;
    if (!this.dialogueAllowsInput()) {
      return { status: "dialogue_blocked" };
    }
    const [bootSuccess, bootText] = this.runPcBootSequence();
    this.logDebug("boot sequence", {
      success: bootSuccess,
      text: bootText ? bootText.slice(0, 120) : null,
    });
    if (!bootSuccess) {
      const fallback = this.resolvePcText("PokecenterPCCantUseText");
      return { status: "no_party", result_text: bootText || fallback || "" };
    }
    return this.runInteractiveSessionAsync(scriptedActions);
  }

  resolveSelection(selection: unknown): number {
    if (selection === null || selection === undefined) {
      return this.promptSelection();
    }
    if (typeof selection === "string") {
      const normalized = selection.trim().toUpperCase();
      for (let index = 0; index < this.hubOptions.length; index += 1) {
        if (this.hubOptions[index].label.trim().toUpperCase() === normalized) {
          return index;
        }
      }
    }
    const parsed = Number(selection);
    const index = Number.isNaN(parsed) ? this.hubOptions.length - 1 : parsed;
    const clamped = Math.max(0, Math.min(index, this.hubOptions.length - 1));
    this.logDebug("resolve selection", {
      selection: selection === null || selection === undefined ? null : String(selection),
      index: clamped,
      label: this.hubOptions[clamped]?.label ?? "",
    });
    return clamped;
  }

  run(opts?: {
    selection?: unknown;
    scriptedActions?: Record<string, Iterable<Record<string, unknown>> | Record<string, unknown> | null>;
  }): Record<string, unknown> {
    const scriptedActions = opts?.scriptedActions ?? {};
    this.activeBoxIndex = 0;
    this.interactiveMode = false;
    this.logDebug("session start", {
      mode: "sync",
      selection: opts?.selection === undefined || opts?.selection === null ? null : String(opts?.selection),
      scripted: (opts?.selection !== undefined && opts?.selection !== null) || Object.values(scriptedActions).some((value) => value),
      players_pc: this.playersPc,
    });
    if (!this.dialogueAllowsInput()) {
      return { status: "dialogue_blocked" };
    }
    const [bootSuccess, bootText] = this.runPcBootSequence();
    this.logDebug("boot sequence", {
      success: bootSuccess,
      text: bootText ? bootText.slice(0, 120) : null,
    });
    if (!bootSuccess) {
      const fallback = this.resolvePcText("PokecenterPCCantUseText");
      return { status: "no_party", result_text: bootText || fallback || "" };
    }
    const scriptedPath =
      (opts?.selection !== undefined && opts?.selection !== null) ||
      Object.values(scriptedActions).some((value) => value);
    if (scriptedPath) {
      const index = this.resolveSelection(opts?.selection);
      const option = this.hubOptions[index];
      const summary = this.executeSelection(option, scriptedActions, false);
      summary.selection_index = index;
      summary.selection_name = option.label;
      summary.result_text = summary.result_text ?? this.buildResultMessage(summary);
      return summary;
    }
    return this.runInteractiveSession(scriptedActions);
  }

  private executeSelection(
    option: PCHubEntry,
    scriptedActions: Record<string, Iterable<Record<string, unknown>> | Record<string, unknown> | null>,
    interactive: boolean,
  ): Record<string, unknown> {
    const summary: Record<string, unknown> = {};
    const action = option.action;
    this.logDebug("execute selection", {
      label: option.label,
      action,
      interactive,
    });
    if (action !== PCHubAction.TURN_OFF) {
      this.audioEngine?.playSound("SFX_CHOOSE_PC_OPTION");
    }
    if (action === PCHubAction.BILLS_PC) {
      this.showPcText("PokecenterBillsPCText");
      summary.bill = interactive ? this.runBillPcInteractive() : this.runBillPc(scriptedActions.bill);
    } else if (action === PCHubAction.PLAYER_PC) {
      this.showPcText("PokecenterPlayersPCText");
      const mailboxActions = scriptedActions.mail;
      if (mailboxActions) {
        summary.mailbox = interactive ? this.runMailboxInteractive() : this.runMailbox(mailboxActions);
      } else {
        summary.player_pc = interactive ? this.runPlayerPcInteractive() : this.runPlayerPc(scriptedActions.player);
      }
    } else if (action === PCHubAction.OAK_PC) {
      this.showPcText("PokecenterOaksPCText");
      summary.oak = this.runOakRating();
    } else if (action === PCHubAction.HALL_OF_FAME) {
      summary.hall_of_fame = interactive ? this.runHallOfFameInteractive() : this.runHallOfFame(scriptedActions.hall);
    } else if (action === PCHubAction.TURN_OFF) {
      this.showPcText("PokecenterPCOaksClosedText");
      summary.shutdown = true;
      playPcShutdownSound(this.audioEngine);
    } else {
      throw new Error(`Unhandled PC hub action ${action}`);
    }
    return summary;
  }

  private async executeSelectionAsync(
    option: PCHubEntry,
    scriptedActions: Record<string, Iterable<Record<string, unknown>> | Record<string, unknown> | null>,
  ): Promise<Record<string, unknown>> {
    const summary: Record<string, unknown> = {};
    const action = option.action;
    this.logDebug("execute selection", {
      label: option.label,
      action,
      interactive: true,
    });
    if (action !== PCHubAction.TURN_OFF) {
      this.audioEngine?.playSound("SFX_CHOOSE_PC_OPTION");
    }
    if (action === PCHubAction.BILLS_PC) {
      this.showPcText("PokecenterBillsPCText");
      summary.bill = await this.runBillPcInteractiveAsync();
    } else if (action === PCHubAction.PLAYER_PC) {
      this.showPcText("PokecenterPlayersPCText");
      const mailboxActions = scriptedActions.mail;
      if (mailboxActions) {
        summary.mailbox = await this.runMailboxInteractiveAsync();
      } else {
        summary.player_pc = await this.runPlayerPcInteractiveAsync();
      }
    } else if (action === PCHubAction.OAK_PC) {
      this.showPcText("PokecenterOaksPCText");
      summary.oak = this.runOakRating();
    } else if (action === PCHubAction.HALL_OF_FAME) {
      summary.hall_of_fame = await this.runHallOfFameInteractiveAsync();
    } else if (action === PCHubAction.TURN_OFF) {
      this.showPcText("PokecenterPCOaksClosedText");
      summary.shutdown = true;
      playPcShutdownSound(this.audioEngine);
    } else {
      throw new Error(`Unhandled PC hub action ${action}`);
    }
    return summary;
  }

  public async runHallOfFameInteractiveAsync(): Promise<Record<string, unknown>> {
    const menuUi = this.resolveMenuUI();
    if (!menuUi) {
      return { status: "no_ui" };
    }
    const [, displayEntries] = this.hallOfFameEntries();
    const viewer = new HallOfFameViewer(menuUi, this.audioEngine);
    return viewer.runInteractiveAsync(displayEntries, this.drawCallback ?? undefined);
  }

  private runInteractiveSession(scriptedActions: Record<string, Iterable<Record<string, unknown>> | Record<string, unknown> | null>): Record<string, unknown> {
    let summary: Record<string, unknown> = {};
    let firstSelection: [number, string] | null = null;
    while (true) {
      const index = this.promptSelection();
      const option = this.hubOptions[index];
      if (!firstSelection) {
        firstSelection = [index, option.label];
      }
      summary = this.executeSelection(option, scriptedActions, true);
      summary.selection_index = index;
      summary.selection_name = option.label;
      if (option.action === PCHubAction.TURN_OFF || summary.shutdown) {
        break;
      }
    }
    if (firstSelection) {
      summary.first_selection_index = firstSelection[0];
      summary.first_selection_name = firstSelection[1];
    }
    summary.result_text = summary.result_text ?? this.buildResultMessage(summary);
    return summary;
  }

  private async runInteractiveSessionAsync(
    scriptedActions: Record<string, Iterable<Record<string, unknown>> | Record<string, unknown> | null>,
  ): Promise<Record<string, unknown>> {
    let summary: Record<string, unknown> = {};
    let firstSelection: [number, string] | null = null;
    while (true) {
      const index = await this.promptSelectionAsync();
      const option = this.hubOptions[index];
      if (!firstSelection) {
        firstSelection = [index, option.label];
      }
      summary = await this.executeSelectionAsync(option, scriptedActions);
      summary.selection_index = index;
      summary.selection_name = option.label;
      if (option.action === PCHubAction.TURN_OFF || summary.shutdown) {
        break;
      }
    }
    if (firstSelection) {
      summary.first_selection_index = firstSelection[0];
      summary.first_selection_name = firstSelection[1];
    }
    summary.result_text = summary.result_text ?? this.buildResultMessage(summary);
    return summary;
  }

  private runBillPc(actions: Iterable<Record<string, unknown>> | Record<string, unknown> | null): Record<string, unknown> {
    const billUi = asPokemonPCUI(this.ui);
    let menu: PokemonPCMenu | null = null;
    if (billUi) {
      menu = new PokemonPCMenu(billUi, this.gameState, this.audioEngine, this.dataLoader ?? undefined);
      menu.draw();
    }
    const results: PCMenuActionResult[] = [];
    const iterable = actions
      ? Array.isArray(actions)
        ? actions
        : [actions as Record<string, unknown>]
      : [];
    for (const entry of iterable) {
      if (!entry || typeof entry !== "object") {
        results.push({ status: "invalid" });
        continue;
      }
      const payload = entry as PCMenuActionPayload;
      results.push(this.executeBillAction(payload));
    }
    menu?.draw();
    return { actions: results, party: this.partySnapshot(), boxes: this.boxesSnapshot() };
  }

  private runBillPcInteractive(): Record<string, unknown> {
    const billUi = asPokemonPCUI(this.ui);
    if (!billUi) {
      return { status: "no_ui" };
    }
    const eventQueue = (billUi as SupportsPokemonPCUI & { eventQueue?: GameEngineEventQueue | null }).eventQueue ?? null;
    if (!billUi.pollEvents) {
      if (!eventQueue) {
        return { status: "no_ui" };
      }
      billUi.pollEvents = () => gameEngine.event.get(eventQueue);
    }
    const menu = new PokemonPCMenu(billUi, this.gameState, this.audioEngine, this.dataLoader ?? undefined);
    const actions: PCMenuInteractiveResponse[] = [];
    const originalPollEvents = billUi.pollEvents;
    if (originalPollEvents) {
      billUi.pollEvents = this.dialogueAwareEventProvider(originalPollEvents);
    }
    this.interactiveMode = true;
    try {
      while (true) {
        const option = this.promptBillPcOption();
        if (!option || option === "see_ya") {
          menu.stopMoveSession();
          break;
        }
        if (option === "move") {
          if (!menu.startMoveSession()) {
            continue;
          }
          menu.showBillAction("move");
        } else if (option === "deposit") {
          menu.showBillAction("deposit");
          menu.stopMoveSession();
        } else if (option === "change_box") {
          menu.stopMoveSession();
        } else {
          menu.showBillAction("withdraw");
          menu.stopMoveSession();
        }
        if (option === "change_box") {
          const boxIndex = this.promptBillPcBoxSelection(menu);
          if (boxIndex === null) {
            continue;
          }
          this.activeBoxIndex = boxIndex;
          menu.setActiveBox(boxIndex);
          menu.draw();
          billUi.update();
          continue;
        }
        const batch = menu.runInteractive({
          actionHandler: (entry) => this.announceAndExecuteBillAction(entry, menu),
          drawCallback: this.drawCallback ?? undefined,
        });
        actions.push(...batch);
      }
    } finally {
      billUi.pollEvents = originalPollEvents;
      this.interactiveMode = false;
    }
    return { actions, party: this.partySnapshot(), boxes: this.boxesSnapshot() };
  }

  private async runBillPcInteractiveAsync(): Promise<Record<string, unknown>> {
    const billUi = asPokemonPCUI(this.ui);
    if (!billUi) {
      return { status: "no_ui" };
    }
    const menu = new PokemonPCMenu(billUi, this.gameState, this.audioEngine, this.dataLoader ?? undefined);
    const actions: PCMenuInteractiveResponse[] = [];
    const originalPollEvents = billUi.pollEvents;
    if (originalPollEvents) {
      billUi.pollEvents = this.dialogueAwareEventProvider(originalPollEvents);
    }
    this.interactiveMode = true;
    try {
      while (true) {
        const option = await this.promptBillPcOptionAsync();
        if (!option || option === "see_ya") {
          menu.stopMoveSession();
          break;
        }
        if (option === "move") {
          if (!(await menu.startMoveSessionAsync())) {
            continue;
          }
          menu.showBillAction("move");
        } else if (option === "deposit") {
          menu.showBillAction("deposit");
          menu.stopMoveSession();
        } else if (option === "change_box") {
          menu.stopMoveSession();
        } else {
          menu.showBillAction("withdraw");
          menu.stopMoveSession();
        }
        if (option === "change_box") {
          const boxIndex = await this.promptBillPcBoxSelectionAsync(menu);
          if (boxIndex === null) {
            continue;
          }
          this.activeBoxIndex = boxIndex;
          menu.setActiveBox(boxIndex);
          menu.draw();
          billUi.update();
          continue;
        }
        const batch = await menu.runInteractiveAsync({
          actionHandler: async (entry) => this.announceAndExecuteBillActionAsync(entry, menu),
          drawCallback: this.drawCallback ?? undefined,
        });
        actions.push(...batch);
      }
    } finally {
      billUi.pollEvents = originalPollEvents;
      this.interactiveMode = false;
    }
    return { actions, party: this.partySnapshot(), boxes: this.boxesSnapshot() };
  }

  private promptBillPcOption(): string | null {
    if (!isPromptUI(this.ui)) {
      return null;
    }
    this.releaseDialogueInputForPcMenu();
    this.drawBillPromptBackdrop();
    const eventProvider = this.promptEventProvider();
    const prompt = new SelectionPrompt(this.ui, PokemonCenterPCSession.BILL_PC_MENU.map((entry) => entry[0]), {
      audioEngine: this.audioEngine ?? undefined,
      windowOriginTiles: [0, 0],
      windowMinWidth: 20,
      windowMinHeight: 18,
    });
    const index = prompt.run({ drawCallback: this.billPromptDrawCallback(), eventProvider });
    if (index === null || index < 0 || index >= PokemonCenterPCSession.BILL_PC_MENU.length) {
      return null;
    }
    return PokemonCenterPCSession.BILL_PC_MENU[index][1];
  }

  private async promptBillPcOptionAsync(): Promise<string | null> {
    if (!isPromptUI(this.ui)) {
      return null;
    }
    this.releaseDialogueInputForPcMenu();
    this.drawBillPromptBackdrop();
    const eventProvider = this.promptEventProvider();
    const prompt = new SelectionPrompt(this.ui, PokemonCenterPCSession.BILL_PC_MENU.map((entry) => entry[0]), {
      audioEngine: this.audioEngine ?? undefined,
      windowOriginTiles: [0, 0],
      windowMinWidth: 20,
      windowMinHeight: 18,
    });
    const index = await prompt.runAsync({ drawCallback: this.billPromptDrawCallback(), eventProvider });
    if (index === null || index < 0 || index >= PokemonCenterPCSession.BILL_PC_MENU.length) {
      return null;
    }
    return PokemonCenterPCSession.BILL_PC_MENU[index][1];
  }

  private promptBillPcBoxSelection(menu?: PokemonPCMenu | null): number | null {
    const boxes = this.gameState.sram.pc_boxes;
    if (!boxes.length) {
      this.ensureBoxes();
    }
    const titles = boxes.map((box, idx) => box.name || formatDefaultBoxName(idx));
    if (!isPromptUI(this.ui)) {
      return null;
    }
    this.releaseDialogueInputForPcMenu();
    const eventProvider = this.promptEventProvider();
    const prompt = new SelectionPrompt(this.ui, titles, {
      audioEngine: this.audioEngine ?? undefined,
      windowOriginTiles: [0, 1],
      windowMinWidth: 14,
    });
    const index = prompt.run({ drawCallback: this.billPromptDrawCallback(menu), eventProvider });
    if (index === null || index < 0 || index >= titles.length) {
      return null;
    }
    return this.setActiveBox(index);
  }

  private async promptBillPcBoxSelectionAsync(menu?: PokemonPCMenu | null): Promise<number | null> {
    const boxes = this.gameState.sram.pc_boxes;
    if (!boxes.length) {
      this.ensureBoxes();
    }
    const titles = boxes.map((box, idx) => box.name || formatDefaultBoxName(idx));
    if (!isPromptUI(this.ui)) {
      return null;
    }
    this.releaseDialogueInputForPcMenu();
    const eventProvider = this.promptEventProvider();
    const prompt = new SelectionPrompt(this.ui, titles, {
      audioEngine: this.audioEngine ?? undefined,
      windowOriginTiles: [0, 1],
      windowMinWidth: 14,
    });
    const index = await prompt.runAsync({ drawCallback: this.billPromptDrawCallback(menu), eventProvider });
    if (index === null || index < 0 || index >= titles.length) {
      return null;
    }
    return this.setActiveBox(index);
  }

  private announceAndExecuteBillAction(entry: PCMenuActionPayload, menu: PokemonPCMenu | null): PCMenuActionResult | null {
    const action = String(entry.action ?? "").toLowerCase();
    if (action === "release" && !this.confirmRelease()) {
      return null;
    }
    const result = this.executeBillAction(entry);
    const normalizedAction = String(result.action ?? "").toLowerCase();
    const status = result.status;
    if (["withdraw", "deposit"].includes(normalizedAction) && status === "ok") {
      this.playSwitchSound();
    }
    if (normalizedAction === "move" && ["ok", "swapped"].includes(String(status))) {
      this.playSwitchSound();
    }
    if (normalizedAction === "release" && status === "ok") {
      menu?.setOverlay("Release!");
      this.showActionText(`Released ${this.formatSpecies(result.species)}!`);
    } else if (normalizedAction === "withdraw" && status === "ok") {
      this.showActionText(`${this.formatSpecies(result.species)} was withdrawn!`);
    } else if (normalizedAction === "deposit" && status === "ok") {
      this.showActionText(`Stored ${this.formatSpecies(result.species)}.`);
    } else if (normalizedAction === "move" && ["ok", "swapped"].includes(String(status))) {
      menu?.setOverlay("Move!");
      this.showActionText("The Pok\u00e9mon was moved.");
    } else if (normalizedAction === "move" && status === "party_size") {
      this.showActionText("You need at least one Pok\u00e9mon in your party.");
    } else if (normalizedAction === "deposit" && status === "party_size") {
      this.showActionText("You need at least one Pok\u00e9mon in your party.");
    } else if (normalizedAction === "deposit" && status === "mail") {
      this.showActionText("Give the mail to someone else before you store this Pok\u00e9mon.");
    } else if (normalizedAction === "deposit" && status === "egg") {
      this.showActionText("You cannot deposit an egg.");
    } else if (normalizedAction === "move" && status === "mail") {
      this.showActionText("Give the mail to someone else before you move this Pok\u00e9mon.");
    } else if (normalizedAction === "release" && status === "mail") {
      this.showActionText("Give the mail to someone else before you release this Pok\u00e9mon.");
    } else if (normalizedAction === "release" && status === "egg") {
      const text = this.resolvePcText("PCString_NoReleasingEGGS");
      this.showActionText(text || "No releasing EGGS!");
    } else if (normalizedAction && status && status !== "ok") {
      this.showActionText(String(status).replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase()));
    }
    return result;
  }

  private async announceAndExecuteBillActionAsync(
    entry: PCMenuActionPayload,
    menu: PokemonPCMenu | null,
  ): Promise<PCMenuActionResult | null> {
    const action = String(entry.action ?? "").toLowerCase();
    if (action === "release" && !(await this.confirmReleaseAsync())) {
      return null;
    }
    const result = await this.executeBillActionAsync(entry);
    const normalizedAction = String(result.action ?? "").toLowerCase();
    const status = result.status;
    if (["withdraw", "deposit"].includes(normalizedAction) && status === "ok") {
      this.playSwitchSound();
    }
    if (normalizedAction === "move" && ["ok", "swapped"].includes(String(status))) {
      this.playSwitchSound();
    }
    if (normalizedAction === "release" && status === "ok") {
      menu?.setOverlay("Release!");
      this.showActionText(`Released ${this.formatSpecies(result.species)}!`);
    } else if (normalizedAction === "withdraw" && status === "ok") {
      this.showActionText(`${this.formatSpecies(result.species)} was withdrawn!`);
    } else if (normalizedAction === "deposit" && status === "ok") {
      this.showActionText(`Stored ${this.formatSpecies(result.species)}.`);
    } else if (normalizedAction === "move" && ["ok", "swapped"].includes(String(status))) {
      menu?.setOverlay("Move!");
      this.showActionText("The Pok\u00e9mon was moved.");
    } else if (normalizedAction === "move" && status === "party_size") {
      this.showActionText("You need at least one Pok\u00e9mon in your party.");
    } else if (normalizedAction === "deposit" && status === "party_size") {
      this.showActionText("You need at least one Pok\u00e9mon in your party.");
    } else if (normalizedAction === "deposit" && status === "mail") {
      this.showActionText("Give the mail to someone else before you store this Pok\u00e9mon.");
    } else if (normalizedAction === "deposit" && status === "egg") {
      this.showActionText("You cannot deposit an egg.");
    } else if (normalizedAction === "move" && status === "mail") {
      this.showActionText("Give the mail to someone else before you move this Pok\u00e9mon.");
    } else if (normalizedAction === "release" && status === "mail") {
      this.showActionText("Give the mail to someone else before you release this Pok\u00e9mon.");
    } else if (normalizedAction === "release" && status === "egg") {
      const text = this.resolvePcText("PCString_NoReleasingEGGS");
      this.showActionText(text || "No releasing EGGS!");
    } else if (normalizedAction && status && status !== "ok") {
      this.showActionText(String(status).replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase()));
    }
    return result;
  }

  private executeBillAction(entry: PCMenuActionPayload): PCMenuActionResult {
    const action = String(entry.action ?? "").toLowerCase();
    let result: PCMenuActionResult;
    if (action === "set_box") {
      this.activeBoxIndex = this.normalizeBoxIndex(entry.box ?? this.activeBoxIndex);
      result = { action: "set_box", box: this.activeBoxIndex, status: "ok" };
      this.logDebug("bill action", {
        action: "set_box",
        status: "ok",
        box: this.toDebugNumber(result.box),
      });
      return result;
    }
    if (action === "withdraw") {
      result = this.finalizeBillActionResult("withdraw", this.withdraw(entry));
      this.logDebug("bill action", {
        action: "withdraw",
        status: String(result.status ?? ""),
        box: this.toDebugNumber(result.box),
        slot: this.toDebugNumber(result.slot),
      });
      return result;
    }
    if (action === "deposit") {
      result = this.finalizeBillActionResult("deposit", this.deposit(entry));
      this.logDebug("bill action", {
        action: "deposit",
        status: String(result.status ?? ""),
        box: this.toDebugNumber(result.box),
        slot: this.toDebugNumber(result.slot),
        party_slot: this.toDebugNumber(result.party_slot),
      });
      return result;
    }
    if (action === "move") {
      if (this.anyPokemonHoldingMail()) {
        result = { action: "move", status: "mail" };
        this.logDebug("bill action", { action: "move", status: "mail" });
        return result;
      }
      if (!this.confirmMoveWithoutMail()) {
        result = { action: "move", status: "cancelled" };
        this.logDebug("bill action", { action: "move", status: "cancelled" });
        return result;
      }
      result = this.finalizeBillActionResult("move", this.move(entry));
      this.logDebug("bill action", {
        action: "move",
        status: String(result.status ?? ""),
        source_box: this.toDebugNumber(result.source_box),
        source_slot: this.toDebugNumber(result.source_slot),
        target_box: this.toDebugNumber(result.target_box),
        target_slot: this.toDebugNumber(result.target_slot),
      });
      return result;
    }
    if (action === "release") {
      result = this.release(entry);
      this.logDebug("bill action", {
        action: "release",
        status: String(result.status ?? ""),
        box: this.toDebugNumber(result.box),
        slot: this.toDebugNumber(result.slot),
      });
      return result;
    }
    result = { status: "unknown_action", action };
    this.logDebug("bill action", { action, status: "unknown_action" });
    return result;
  }

  private async executeBillActionAsync(entry: PCMenuActionPayload): Promise<PCMenuActionResult> {
    const action = String(entry.action ?? "").toLowerCase();
    let result: PCMenuActionResult;
    if (action === "set_box") {
      this.activeBoxIndex = this.normalizeBoxIndex(entry.box ?? this.activeBoxIndex);
      result = { action: "set_box", box: this.activeBoxIndex, status: "ok" };
      this.logDebug("bill action", {
        action: "set_box",
        status: "ok",
        box: this.toDebugNumber(result.box),
      });
      return result;
    }
    if (action === "withdraw") {
      result = this.finalizeBillActionResult("withdraw", this.withdraw(entry));
      this.logDebug("bill action", {
        action: "withdraw",
        status: String(result.status ?? ""),
        box: this.toDebugNumber(result.box),
        slot: this.toDebugNumber(result.slot),
      });
      return result;
    }
    if (action === "deposit") {
      result = this.finalizeBillActionResult("deposit", this.deposit(entry));
      this.logDebug("bill action", {
        action: "deposit",
        status: String(result.status ?? ""),
        box: this.toDebugNumber(result.box),
        slot: this.toDebugNumber(result.slot),
        party_slot: this.toDebugNumber(result.party_slot),
      });
      return result;
    }
    if (action === "move") {
      if (this.anyPokemonHoldingMail()) {
        result = { action: "move", status: "mail" };
        this.logDebug("bill action", { action: "move", status: "mail" });
        return result;
      }
      if (!(await this.confirmMoveWithoutMailAsync())) {
        result = { action: "move", status: "cancelled" };
        this.logDebug("bill action", { action: "move", status: "cancelled" });
        return result;
      }
      result = this.finalizeBillActionResult("move", this.move(entry));
      this.logDebug("bill action", {
        action: "move",
        status: String(result.status ?? ""),
        source_box: this.toDebugNumber(result.source_box),
        source_slot: this.toDebugNumber(result.source_slot),
        target_box: this.toDebugNumber(result.target_box),
        target_slot: this.toDebugNumber(result.target_slot),
      });
      return result;
    }
    if (action === "release") {
      result = this.release(entry);
      this.logDebug("bill action", {
        action: "release",
        status: String(result.status ?? ""),
        box: this.toDebugNumber(result.box),
        slot: this.toDebugNumber(result.slot),
      });
      return result;
    }
    result = { status: "unknown_action", action };
    this.logDebug("bill action", { action, status: "unknown_action" });
    return result;
  }

  private finalizeBillActionResult(action: string, result: PCMenuActionResult): PCMenuActionResult {
    if (!this.interactiveMode) {
      if (["withdraw", "deposit"].includes(action) && result.status === "ok") {
        this.playSwitchSound();
      } else if (action === "move" && ["ok", "swapped"].includes(String(result.status))) {
        this.playSwitchSound();
      }
    }
    return result;
  }

  private withdraw(entry: PCMenuActionPayload): PCMenuActionResult {
    const boxIndex = this.setActiveBox(entry.box ?? this.activeBoxIndex);
    const slotIndex = this.normalizeBoxSlot(entry.slot ?? 0);
    const box = this.ensureBox(boxIndex);
    const pokemon = box.pokemon[slotIndex];
    if (!pokemon) {
      return { action: "withdraw", status: "empty_slot", box: boxIndex, slot: slotIndex };
    }
    const target = this.resolvePartySlot(entry.party_slot);
    if (target === null) {
      return { action: "withdraw", status: "party_full", box: boxIndex, slot: slotIndex };
    }
    this.gameState.sram.party.pokemon[target] = pokemon;
    clearSlot(box, slotIndex);
    this.compactBox(box);
    this.syncPartyCount();
    return {
      action: "withdraw",
      status: "ok",
      box: boxIndex,
      slot: slotIndex,
      party_slot: target,
      species: pokemon.species?.id ?? "",
    };
  }

  private deposit(entry: PCMenuActionPayload): PCMenuActionResult {
    const partySlot = this.normalizePartyIndex(entry.party_slot ?? 0);
    const party = this.gameState.sram.party.pokemon;
    const pokemon = party[partySlot];
    if (!pokemon) {
      return { action: "deposit", status: "empty_slot", party_slot: partySlot };
    }
    if (this.partyCount() <= 1) {
      return { action: "deposit", status: "party_size", party_slot: partySlot };
    }
    if (this.pokemonHasMail(toPokemon(pokemon))) {
      return { action: "deposit", status: "mail", party_slot: partySlot };
    }
    const boxIndex = this.setActiveBox(entry.box ?? this.activeBoxIndex);
    const box = this.ensureBox(boxIndex);
    const slotValue = entry.slot;
    let slotIndex: number | null = null;
    if (slotValue !== undefined && slotValue !== null) {
      slotIndex = this.normalizeBoxSlot(slotValue);
      if (box.pokemon[slotIndex]) {
        return { action: "deposit", status: "occupied", box: boxIndex, slot: slotIndex };
      }
    } else {
      slotIndex = this.findEmptySlot(box.pokemon.map((p) => (p ? toPokemon(p) : null)));
      if (slotIndex === null) {
        return { action: "deposit", status: "box_full", box: boxIndex };
      }
    }
    setSlot(box, slotIndex, pokemon);
    party[partySlot] = null;
    this.compactParty();
    return {
      action: "deposit",
      status: "ok",
      box: boxIndex,
      slot: slotIndex,
      species: pokemon.species?.id ?? "",
    };
  }

  private move(entry: PCMenuActionPayload): PCMenuActionResult {
    const sourceBox = entry.box === null ? null : this.normalizeBoxIndex(entry.box ?? this.activeBoxIndex);
    const sourceSlot = sourceBox === null
      ? this.normalizePartyIndex(entry.slot ?? entry.party_slot ?? 0)
      : this.normalizeBoxSlot(entry.slot ?? 0);
    const sourceParty = this.gameState.sram.party.pokemon;
    const source = sourceBox === null ? null : this.ensureBox(sourceBox);
    const pokemon = source === null ? sourceParty[sourceSlot] : source.pokemon[sourceSlot];
    if (!pokemon) {
      return sourceBox === null
        ? { action: "move", status: "empty_slot", party_slot: sourceSlot }
        : { action: "move", status: "empty_slot", box: sourceBox, slot: sourceSlot };
    }
    if (this.pokemonHasMail(toPokemon(pokemon))) {
      return sourceBox === null
        ? { action: "move", status: "mail", party_slot: sourceSlot }
        : { action: "move", status: "mail", box: sourceBox, slot: sourceSlot };
    }
    const targetBox = entry.target_box === null
      ? null
      : this.normalizeBoxIndex(entry.target_box ?? sourceBox ?? this.activeBoxIndex);
    if (sourceBox === null && targetBox !== null && this.partyCount() <= 1) {
      return { action: "move", status: "party_size", party_slot: sourceSlot, target_box: targetBox };
    }
    if (targetBox === null) {
      const filledSlots = getFilledSlots(this.gameState.sram.party);
      const requested = entry.target_slot ?? entry.party_slot;
      const targetSlot = requested === undefined || requested === null
        ? filledSlots
        : Math.max(0, Math.min(Number(requested) || 0, this.gameState.sram.party.pokemon.length - 1));
      const party = this.gameState.sram.party.pokemon;
      const targetPokemon = party[targetSlot] ?? null;
      if (!targetPokemon && filledSlots >= party.length) {
        return { action: "move", status: "party_full", box: sourceBox ?? undefined, slot: sourceSlot };
      }

      if (sourceBox === null) {
        if (targetSlot === sourceSlot) {
          return { action: "move", status: "ok", party_slot: sourceSlot, target_slot: targetSlot };
        }
        if (targetPokemon) {
          party[targetSlot] = pokemon;
          party[sourceSlot] = targetPokemon;
        } else {
          party[sourceSlot] = null;
          this.insertPartyPokemon(toPokemon(pokemon), Math.min(targetSlot, getFilledSlots(this.gameState.sram.party)));
        }
      } else {
        const sourceBoxState = source;
        if (!sourceBoxState) {
          throw new Error("PC move source box was not loaded.");
        }
        clearSlot(sourceBoxState, sourceSlot);
        if (targetPokemon) {
          party[targetSlot] = pokemon;
          setSlot(sourceBoxState, sourceSlot, targetPokemon);
        } else {
          this.insertPartyPokemon(toPokemon(pokemon), Math.min(targetSlot, getFilledSlots(this.gameState.sram.party)));
          this.compactBox(sourceBoxState);
        }
      }
      this.syncPartyCount();
      return {
        action: "move",
        status: targetPokemon ? "swapped" : "ok",
        source_box: sourceBox ?? undefined,
        source_slot: sourceSlot,
        target_slot: targetSlot,
        party_slot: targetSlot,
      };
    }

    const target = this.ensureBox(targetBox);
    const targetSlotValue = entry.target_slot;
    let targetSlot: number | null = null;
    if (targetSlotValue === undefined || targetSlotValue === null) {
      targetSlot = this.findEmptySlot(target.pokemon.map((p) => (p ? toPokemon(p) : null)));
      if (targetSlot === null) {
        return { action: "move", status: "box_full", box: targetBox };
      }
    } else {
      targetSlot = this.normalizeBoxSlot(targetSlotValue);
    }
    const swapped = target.pokemon[targetSlot];
    setSlot(target, targetSlot, pokemon);
    if (sourceBox === null) {
      sourceParty[sourceSlot] = swapped ?? null;
      if (!swapped) {
        this.compactParty();
      } else {
        this.syncPartyCount();
      }
    } else {
      const sourceBoxState = source;
      if (!sourceBoxState) {
        throw new Error("PC move source box was not loaded.");
      }
      setSlot(sourceBoxState, sourceSlot, swapped ?? null);
      if (!swapped) {
        this.compactBox(sourceBoxState);
      }
    }
    const status = swapped ? "swapped" : "ok";
    this.setActiveBox(targetBox);
    return {
      action: "move",
      status,
      source_box: sourceBox ?? undefined,
      source_slot: sourceSlot,
      target_box: targetBox,
      target_slot: targetSlot,
      party_slot: sourceBox === null ? sourceSlot : undefined,
    };
  }

  private release(entry: PCMenuActionPayload): PCMenuActionResult {
    if (entry.box === null) {
      const partySlot = this.normalizePartyIndex(entry.slot ?? entry.party_slot ?? 0);
      const party = this.gameState.sram.party.pokemon;
      const pokemon = party[partySlot];
      if (!pokemon) {
        return { action: "release", status: "empty_slot", party_slot: partySlot };
      }
      if (this.partyCount() <= 1) {
        return { action: "release", status: "party_size", party_slot: partySlot };
      }
      if (this.isEgg(toPokemon(pokemon))) {
        return { action: "release", status: "egg", party_slot: partySlot };
      }
      if (this.pokemonHasMail(toPokemon(pokemon))) {
        return { action: "release", status: "mail", party_slot: partySlot };
      }
      party[partySlot] = null;
      this.compactParty();
      return { action: "release", status: "ok", party_slot: partySlot, species: pokemon.species?.id ?? "" };
    }

    const boxIndex = this.setActiveBox(entry.box ?? this.activeBoxIndex);
    const slotIndex = this.normalizeBoxSlot(entry.slot ?? 0);
    const box = this.ensureBox(boxIndex);
    const pokemon = box.pokemon[slotIndex];
    if (!pokemon) {
      return { action: "release", status: "empty_slot", box: boxIndex, slot: slotIndex };
    }
    if (this.isEgg(toPokemon(pokemon))) {
      return { action: "release", status: "egg", box: boxIndex, slot: slotIndex };
    }
    if (this.pokemonHasMail(toPokemon(pokemon))) {
      return { action: "release", status: "mail", box: boxIndex, slot: slotIndex };
    }
    clearSlot(box, slotIndex);
    this.compactBox(box);
    return { action: "release", status: "ok", box: boxIndex, slot: slotIndex, species: pokemon.species?.id ?? "" };
  }

  private runMailbox(actions: Iterable<Record<string, unknown>> | Record<string, unknown> | null): Record<string, unknown> {
    const menuUi = this.resolveMenuUI();
    if (!menuUi) {
      return { status: "no_ui" };
    }
    const mailbox = this.gameState.sram.mailbox as MailMessage[];
    const menu = new MailboxMenu(menuUi, mailbox, this.audioEngine);
    menu.draw();
    const iterable = actions
      ? Array.isArray(actions)
        ? actions
        : [actions as Record<string, unknown>]
      : [];
    const results = menu.scriptedActions(iterable);
    menu.draw();
    this.logDebug("mailbox actions", { count: results.length });
    return { messages: mailbox.map((mail) => mail.message), actions: results };
  }

  private runMailboxInteractive(): Record<string, unknown> {
    const menuUi = this.resolveMenuUI();
    if (!menuUi || !menuUi.eventQueue) {
      return { status: "no_ui" };
    }
    const mailbox = this.gameState.sram.mailbox as MailMessage[];
    const menu = new MailboxMenu(menuUi, mailbox, this.audioEngine);
    const actions = menu.runInteractive({
      handler: (action) => {
        const result = menu.executeAction(action);
        this.announceMailAction(result);
        return result;
      },
    });
    this.logDebug("mailbox actions", { count: actions.length });
    return { messages: mailbox.map((mail) => mail.message), actions };
  }

  public async runMailboxInteractiveAsync(): Promise<Record<string, unknown>> {
    const menuUi = this.resolveMenuUI();
    if (!menuUi) {
      return { status: "no_ui" };
    }
    const mailbox = this.gameState.sram.mailbox as MailMessage[];
    const menu = new MailboxMenu(menuUi, mailbox, this.audioEngine);
    const actions = await menu.runInteractiveAsync({
      handler: (action) => {
        const result = menu.executeAction(action);
        this.announceMailAction(result);
        return result;
      },
    });
    this.logDebug("mailbox actions", { count: actions.length });
    return { messages: mailbox.map((mail) => mail.message), actions };
  }

  private runPlayerPc(actions: Iterable<Record<string, unknown>> | Record<string, unknown> | null): Record<string, unknown> {
    const iterable = actions
      ? Array.isArray(actions)
        ? actions
        : [actions as Record<string, unknown>]
      : [];
    if (!isPlayerPCUI(this.ui)) {
      return { status: "no_ui" };
    }
    const itemLoader = resolveItemDataLoader(this.dataLoader ?? undefined);
    const menu = new PlayerPCMenu(this.ui, this.gameState, itemLoader, this.audioEngine);
    menu.draw();
    const results = menu.scriptedActions(iterable);
    menu.draw();
    results.forEach((result) => this.logPlayerAction(result));
    return { actions: results, pc_items: this.pcItemsSnapshot() };
  }

  private runPlayerPcInteractive(): Record<string, unknown> {
    if (!isPlayerPCUI(this.ui)) {
      return { status: "no_ui" };
    }
    this.releaseDialogueInputForPcMenu();
    const itemLoader = resolveItemDataLoader(this.dataLoader ?? undefined);
    const menu = new PlayerPCMenu(this.ui, this.gameState, itemLoader, this.audioEngine);
    const pokemonPcUi = asPokemonPCUI(this.ui);
    const eventQueue = (this.ui as { eventQueue?: GameEngineEventQueue | null }).eventQueue ?? null;
    const rawEventProvider = pokemonPcUi?.pollEvents ?? (eventQueue ? () => gameEngine.event.get(eventQueue) : null);
    const eventProvider = rawEventProvider ? this.dialogueAwareEventProvider(rawEventProvider) : null;
    if (!eventProvider) {
      return { status: "no_ui" };
    }
    const actions = menu.runInteractive({
      actionHandler: (result) => {
        if (result?.action === "mail_box") {
          const mailbox = this.runMailboxInteractive();
          return { ...result, mailbox };
        }
        this.announcePlayerAction(result);
        return result;
      },
      drawCallback: this.drawCallback ?? undefined,
      eventProvider,
    });
    actions.forEach((result) => this.logPlayerAction(result));
    return { actions, pc_items: this.pcItemsSnapshot() };
  }

  private async runPlayerPcInteractiveAsync(): Promise<Record<string, unknown>> {
    if (!isPlayerPCUI(this.ui)) {
      return { status: "no_ui" };
    }
    this.releaseDialogueInputForPcMenu();
    const itemLoader = resolveItemDataLoader(this.dataLoader ?? undefined);
    const menu = new PlayerPCMenu(this.ui, this.gameState, itemLoader, this.audioEngine);
    const pokemonPcUi = asPokemonPCUI(this.ui);
    const eventProvider = pokemonPcUi?.pollEvents
      ? this.dialogueAwareEventProvider(pokemonPcUi.pollEvents)
      : null;
    if (!eventProvider) {
      return { status: "no_ui" };
    }
    const actions = await menu.runInteractiveAsync({
      actionHandler: async (result) => {
        if (result?.action === "mail_box") {
          const mailbox = await this.runMailboxInteractiveAsync();
          return { ...result, mailbox };
        }
        this.announcePlayerAction(result);
        return result;
      },
      drawCallback: this.drawCallback ?? undefined,
      eventProvider,
    });
    actions.forEach((result) => this.logPlayerAction(result));
    return { actions, pc_items: this.pcItemsSnapshot() };
  }

  private announceMailAction(result: Record<string, unknown> | null): void {
    if (!result) {
      return;
    }
    this.logDebug("mailbox action", {
      action: result.action ? String(result.action) : "",
      status: result.status ? String(result.status) : "",
    });
    const action = result.action;
    if (action === "read" && result.message) {
      this.showActionText(String(result.message));
    } else if (action === "delete" && result.status === "ok") {
      this.showActionText("Mail deleted.");
    } else if (action === "take") {
      this.showActionText(result.status === "ok" ? "Mail taken from the PC." : "No mail to take.");
    } else if (action === "give") {
      if (result.status === "ok") {
        this.showActionText("Mail was placed in the PC.");
      } else if (result.status === "no_message") {
        this.showActionText("Mail must be composed before giving.");
      }
    }
  }

  private announcePlayerAction(result: PlayerPCMenuActionResult | null): PlayerPCMenuActionResult | null {
    if (!result) {
      return null;
    }
    this.logPlayerAction(result);
    const action = result.action;
    const status = result.status;
    const display = (result.display as string) || this.formatItemName(result.item);
    const quantity = Number(result.quantity ?? 0);
    let text: string | null = null;
    if (action === "deposit") {
      if (status === "ok") {
        text = `Stored ${quantity} ${display}.`;
      } else if (status === "empty") {
        text = `You have no ${display}.`;
      } else if (status === "pc_full") {
        text = "The PC is full.";
      } else if (status === "bag_error" || status === "invalid") {
        text = "You can't store that item.";
      }
    } else if (action === "withdraw") {
      if (status === "ok") {
        text = `Withdrew ${quantity} ${display} from the PC.`;
      } else if (status === "empty") {
        text = `No ${display} stored.`;
      } else if (status === "bag_full") {
        text = "You can't hold the item.";
      }
    } else if (action === "toss") {
      if (status === "ok") {
        text = `Tossed ${quantity} ${display}.`;
      } else if (status === "empty") {
        text = `You have no ${display}.`;
      }
    } else if (action === "decoration") {
      text = "The decoration feature is not available yet.";
    }
    if (text) {
      this.showActionText(text);
    }
    return result;
  }

  private runOakRating(): Record<string, unknown> {
    const caught = countPokedexEntries(this.gameState.sram.pokedex_owned);
    const label = this.ratingForCount(caught);
    const text = this.resolveRatingText(label);
    const menuUi = this.resolveMenuUI();
    if (!menuUi) {
      return { status: "no_ui", caught, label, text };
    }
    const screen = new PokedexRatingScreen(menuUi, this.audioEngine);
    screen.draw(text);
    if (this.eventManager && text) {
      this.eventManager.showText?.(text);
      this.eventManager.waitForInput?.();
    }
    return { caught, label, text };
  }

  private hallOfFameEntries(): [HallOfFameEntry[], string[][]] {
    const entries = this.gameState.sram.hall_of_fame as HallOfFameEntry[];
    const displayEntries = entries.map((entry) => {
      const team = this.getHallOfFameTeam(entry);
      return team.map((mon) => this.formatHallOfFamePokemonName(mon));
    });
    return [entries, displayEntries];
  }

  private getHallOfFameTeam(entry: HallOfFameEntry): HallOfFamePokemon[] {
    if (entry.team && entry.team.length > 0) {
      return entry.team;
    }
    if (entry.pokemon && entry.pokemon.length > 0) {
      return entry.pokemon;
    }
    return [];
  }

  private formatHallOfFamePokemonName(mon: HallOfFamePokemon | null | undefined): string {
    if (!mon) {
      return "";
    }
    if (mon.nickname) {
      return mon.nickname;
    }
    const speciesUpper = mon.species?.toUpperCase();
    if (speciesUpper === "EGG") {
      return "";
    }
    return mon.species ?? "";
  }

  public runHallOfFame(actions: Iterable<Record<string, unknown>> | Record<string, unknown> | null): Record<string, unknown> {
    const menuUi = this.resolveMenuUI();
    if (!menuUi) {
      return { status: "no_ui" };
    }
    const [entries, displayEntries] = this.hallOfFameEntries();
    const summary: HallOfFameSummary = { entries: displayEntries.map((entry) => [...entry]) };
    if (!entries.length) {
      summary.status = "empty";
      return summary;
    }
    const iterable = actions
      ? Array.isArray(actions)
        ? actions
        : [actions as Record<string, unknown>]
      : [];
    let entryIndex = 0;
    for (const action of iterable) {
      if (!action || typeof action !== "object") {
        continue;
      }
      if (action.entry !== undefined) {
        entryIndex = this.normalizeEntryIndex(action.entry, entries.length);
      }
    }
    entryIndex = this.normalizeEntryIndex(entryIndex, entries.length);
    const viewer = new HallOfFameViewer(menuUi, this.audioEngine);
    viewer.draw(displayEntries[entryIndex], entryIndex);
    summary.selected_index = entryIndex;
    summary.selected = [...displayEntries[entryIndex]];
    return summary;
  }

  public runHallOfFameInteractive(): Record<string, unknown> {
    const menuUi = this.resolveMenuUI();
    if (!menuUi || !menuUi.eventQueue) {
      return { status: "no_ui" };
    }
    const [, displayEntries] = this.hallOfFameEntries();
    const viewer = new HallOfFameViewer(menuUi, this.audioEngine);
    return viewer.runInteractive(displayEntries, this.drawCallback ?? undefined);
  }

  private promptSelection(): number {
    if (!this.hubOptions.length) {
      return 0;
    }
    if (isPromptUI(this.ui)) {
      this.releaseDialogueInputForPcMenu();
      const eventProvider = this.promptEventProvider();
      const index = this.shouldUseAsmPCHubMenu()
        ? new PCHubMenu(
            this.ui,
            this.hubOptions.map((entry) => entry.label),
            this.audioEngine ?? null,
            {
              promptText: this.resolvePcText("PokecenterPCWhoseText") ?? undefined,
              eventProvider,
            },
          ).run(this.drawCallback ?? undefined)
        : new SelectionPrompt(this.ui, this.hubOptions.map((entry) => entry.label), {
            audioEngine: this.audioEngine ?? undefined,
          }).run({ drawCallback: this.drawCallback ?? undefined, eventProvider });
      if (index === null) {
        const fallback = Math.max(0, this.hubOptions.length - 1);
        this.logDebug("prompt selection", {
          index: fallback,
          label: this.hubOptions[fallback]?.label ?? "",
          fallback: true,
        });
        return fallback;
      }
      const clamped = Math.max(0, Math.min(index, this.hubOptions.length - 1));
      this.logDebug("prompt selection", {
        index: clamped,
        label: this.hubOptions[clamped]?.label ?? "",
        fallback: false,
      });
      return clamped;
    }
    const fallback = Math.max(0, this.hubOptions.length - 1);
    this.logDebug("prompt selection", {
      index: fallback,
      label: this.hubOptions[fallback]?.label ?? "",
      fallback: true,
    });
    return fallback;
  }

  private async promptSelectionAsync(): Promise<number> {
    if (!this.hubOptions.length) {
      return 0;
    }
    if (isPromptUI(this.ui)) {
      this.releaseDialogueInputForPcMenu();
      const eventProvider = this.promptEventProvider();
      const index = this.shouldUseAsmPCHubMenu()
        ? await new PCHubMenu(
            this.ui,
            this.hubOptions.map((entry) => entry.label),
            this.audioEngine ?? null,
            {
              promptText: this.resolvePcText("PokecenterPCWhoseText") ?? undefined,
              eventProvider,
            },
          ).runAsync(this.drawCallback ?? undefined)
        : await new SelectionPrompt(this.ui, this.hubOptions.map((entry) => entry.label), {
            audioEngine: this.audioEngine ?? undefined,
          }).runAsync({ drawCallback: this.drawCallback ?? undefined, eventProvider });
      if (index === null) {
        const fallback = Math.max(0, this.hubOptions.length - 1);
        this.logDebug("prompt selection", {
          index: fallback,
          label: this.hubOptions[fallback]?.label ?? "",
          fallback: true,
        });
        return fallback;
      }
      const clamped = Math.max(0, Math.min(index, this.hubOptions.length - 1));
      this.logDebug("prompt selection", {
        index: clamped,
        label: this.hubOptions[clamped]?.label ?? "",
        fallback: false,
      });
      return clamped;
    }
    const fallback = Math.max(0, this.hubOptions.length - 1);
    this.logDebug("prompt selection", {
      index: fallback,
      label: this.hubOptions[fallback]?.label ?? "",
      fallback: true,
    });
    return fallback;
  }

  private resolveMenuUI(): MenuUI | null {
    const pokemonUi = asPokemonPCUI(this.ui);
    if (!pokemonUi && !isPromptUI(this.ui)) {
      return null;
    }
    const menuCandidate = pokemonUi ?? (isPlayerPCUI(this.ui) ? asPokemonPCUI(this.ui) : this.ui);
    return toMenuUI(menuCandidate);
  }

  private dialogueAllowsInput(): boolean {
    if (!this.dialogue) {
      return true;
    }
    const visible = Boolean(this.dialogue.visible);
    const waiting = Boolean(this.dialogue.waiting_for_input);
    const pending = Number(this.dialogue.pending_waits ?? 0);
    return !(visible && (waiting || pending > 0));
  }

  private dialogueIsWaiting(): boolean {
    if (!this.dialogue) {
      return false;
    }
    const waiting = Boolean(this.dialogue.waiting_for_input);
    const pending = Number(this.dialogue.pending_waits ?? 0);
    return waiting || pending > 0;
  }

  private releaseDialogueInputForPcMenu(): void {
    if (!this.dialogue) {
      return;
    }
    this.dialogue.visible = false;
    this.dialogue.waiting_for_input = false;
    if (typeof this.dialogue.clear_script_waits === "function") {
      this.dialogue.clear_script_waits();
    } else if (canAssignDialoguePendingWaits(this.dialogue)) {
      this.dialogue.pending_waits = 0;
    }
  }

  private dialogueAwareEventProvider<T>(provider: () => T[]): () => T[] {
    return () => {
      const events = provider();
      if (!this.dialogueIsWaiting()) {
        return events;
      }
      this.releaseDialogueInputForPcMenu();
      return events;
    };
  }

  private promptEventProvider(): (() => KeyEvent[]) | undefined {
    const pokemonUi = asPokemonPCUI(this.ui);
    if (pokemonUi?.pollEvents) {
      return this.dialogueAwareEventProvider(pokemonUi.pollEvents);
    }
    const eventQueue = (this.ui as { eventQueue?: GameEngineEventQueue | null } | null)?.eventQueue;
    if (!eventQueue) {
      return undefined;
    }
    return this.dialogueAwareEventProvider(() => gameEngine.event.get(eventQueue) as KeyEvent[]);
  }

  private ensureBoxes(): void {
    const boxes = this.gameState.sram.pc_boxes;
    if (!boxes.length) {
      for (let index = 0; index < MAX_PC_BOXES; index += 1) {
        boxes.push(this.createCanonicalBox(index));
      }
    }
    boxes.forEach((box, index) => {
      const name = box?.name?.trim() ? box.name : formatDefaultBoxName(index);
      boxes[index] = this.normalizeBox({ ...box, name }, index);
    });
    this.setActiveBox(this.activeBoxIndex);
  }

  private ensureBox(index: number): Box {
    const boxes = this.gameState.sram.pc_boxes;
    while (boxes.length <= index) {
      boxes.push(this.createCanonicalBox(boxes.length));
    }
    const normalized = this.normalizeBox(boxes[index], index);
    boxes[index] = normalized;
    return normalized;
  }

  private normalizeBoxIndex(value: unknown): number {
    const index = Number(value);
    const maxIndex = Math.max(this.gameState.sram.pc_boxes.length - 1, 0);
    if (Number.isNaN(index)) {
      return this.activeBoxIndex;
    }
    const asmIndex = Math.max(0, Math.trunc(index)) & 0x0f;
    return Math.max(0, Math.min(asmIndex, maxIndex));
  }

  private setActiveBox(value: unknown): number {
    const index = this.normalizeBoxIndex(value);
    this.activeBoxIndex = index;
    this.gameState.sram.current_pc_box = index;
    return index;
  }

  private normalizeBoxSlot(value: unknown): number {
    const slot = Number(value);
    if (Number.isNaN(slot)) {
      return 0;
    }
    return Math.max(0, Math.min(slot, MAX_BOX_MONS - 1));
  }

  private createCanonicalBox(index: number): Box {
    return BoxSchema.parse({ name: formatDefaultBoxName(index) });
  }

  private normalizeBox(box: Partial<Box> | null | undefined, index: number): Box {
    const current = box ?? {};
    const name = current?.name?.trim() ? current.name : formatDefaultBoxName(index);
    try {
      return BoxSchema.parse({ ...current, name });
    } catch (err) {
      throw new Error(
        `Invalid PC box '${name}': ${err instanceof Error ? err.message : String(err)}`
      );
    }
  }

  private normalizePartyIndex(value: unknown): number {
    const index = Number(value);
    if (Number.isNaN(index)) {
      return 0;
    }
    return Math.max(0, Math.min(index, this.gameState.sram.party.pokemon.length - 1));
  }

  private syncPartyCount(): void {
    this.gameState.wram.wPartyCount = getFilledSlots(this.gameState.sram.party);
  }

  private compactParty(): void {
    const compacted: Array<Pokemon | null> = this.gameState.sram.party.pokemon.filter(
      (pokemon): pokemon is Pokemon => pokemon !== null
    );
    while (compacted.length < this.gameState.sram.party.pokemon.length) {
      compacted.push(null);
    }
    this.gameState.sram.party.pokemon = compacted;
    this.syncPartyCount();
  }

  private compactBox(box: Box): void {
    const compacted = box.pokemon.filter((pokemon) => pokemon !== null).map((pokemon) => toPokemon(pokemon));
    for (let index = 0; index < box.pokemon.length; index += 1) {
      setSlot(box, index, compacted[index] ?? null);
    }
  }

  private insertPartyPokemon(pokemon: Pokemon, index: number): number | null {
    const slots = this.gameState.sram.party.pokemon;
    const filledSlots = getFilledSlots(this.gameState.sram.party);
    if (filledSlots >= slots.length) {
      return null;
    }
    const targetIndex = Math.max(0, Math.min(index, filledSlots));
    for (let cursor = filledSlots; cursor > targetIndex; cursor -= 1) {
      slots[cursor] = slots[cursor - 1];
    }
    slots[targetIndex] = pokemon;
    this.syncPartyCount();
    return targetIndex;
  }

  private normalizeEntryIndex(value: unknown, count: number): number {
    const index = Number(value);
    if (count <= 0) {
      return 0;
    }
    if (Number.isNaN(index)) {
      return 0;
    }
    return Math.max(0, Math.min(index, count - 1));
  }

  private resolvePartySlot(value: unknown): number | null {
    const party = this.gameState.sram.party.pokemon;
    if (value !== undefined && value !== null) {
      const index = this.normalizePartyIndex(value);
      if (!party[index]) {
        return index;
      }
      return null;
    }
    for (let index = 0; index < party.length; index += 1) {
      if (!party[index]) {
        return index;
      }
    }
    return null;
  }

  private pokemonHasMail(mon: Pokemon | null): boolean {
    const pokemon = mon ? toPokemon(mon) : null;
    if (!pokemon?.item) {
      return false;
    }
    return String(pokemon.item).toUpperCase().includes("MAIL");
  }

  private anyPokemonHoldingMail(): boolean {
    for (const member of this.gameState.sram.party.pokemon) {
      if (member && this.pokemonHasMail(toPokemon(member))) {
        return true;
      }
    }
    for (const box of this.gameState.sram.pc_boxes) {
      for (const member of box.pokemon) {
        if (member && this.pokemonHasMail(toPokemon(member))) {
          return true;
        }
      }
    }
    return false;
  }

  private isEgg(mon: Pokemon | null): boolean {
    const pokemon = mon ? toPokemon(mon) : null;
    if (!pokemon) {
      return false;
    }
    const nickname = (pokemon.nickname ?? "").trim().toUpperCase();
    return nickname === "EGG";
  }

  private findEmptySlot(slots: Array<Pokemon | null>): number | null {
    const index = slots.findIndex((entry) => entry === null);
    return index === -1 ? null : index;
  }

  private partySnapshot(): Array<Record<string, unknown> | null> {
    return this.gameState.sram.party.pokemon.map((member) =>
      member
        ? {
            species: member.species?.id ?? "",
            level: member.level ?? 0,
            nickname: member.nickname ?? "",
          }
        : null,
    );
  }

  private boxesSnapshot(): Record<string, unknown>[] {
    return this.gameState.sram.pc_boxes.map((box, index) => ({
      name: box.name || formatDefaultBoxName(index),
      pokemon: box.pokemon.map((mon) => (mon ? mon.species?.id ?? "" : null)),
    }));
  }

  private pcItemsSnapshot(): Record<string, unknown>[] {
    const snapshot: Record<string, unknown>[] = [];
    for (const slot of this.gameState.sram.pc_items) {
      if (slot.quantity > 0) {
        snapshot.push({ item: slot.item, quantity: slot.quantity });
      }
    }
    return snapshot;
  }

  private partyCount(): number {
    return this.gameState.sram.party.pokemon.filter(Boolean).length;
  }

  private ratingForCount(caught: number): string {
    const ratings: Array<[number, string]> = [
      [9, "OakRating01"],
      [19, "OakRating02"],
      [34, "OakRating03"],
      [49, "OakRating04"],
      [64, "OakRating05"],
      [79, "OakRating06"],
      [94, "OakRating07"],
      [109, "OakRating08"],
      [124, "OakRating09"],
      [139, "OakRating10"],
      [154, "OakRating11"],
      [169, "OakRating12"],
      [184, "OakRating13"],
      [199, "OakRating14"],
      [214, "OakRating15"],
      [229, "OakRating16"],
      [239, "OakRating17"],
      [248, "OakRating18"],
      [255, "OakRating19"],
    ];
    for (const [limit, label] of ratings) {
      if (caught <= limit) {
        return label;
      }
    }
    return ratings[ratings.length - 1][1];
  }

  private resolveRatingText(label: string): string {
    if (!this.dataLoader?.getText) {
      return label;
    }
    try {
      return this.dataLoader.getText(label) ?? label;
    } catch {
      return label;
    }
  }

  private formatSpecies(speciesId: unknown): string {
    if (!speciesId) {
      return "POK\u00c9MON";
    }
    const result = String(speciesId).replace(/_/g, " ").replace(/  +/g, " ").trim();
    return result || "POK\u00c9MON";
  }

  private formatItemName(itemId: unknown): string {
    if (!itemId) {
      return "ITEM";
    }
    return String(itemId).replace(/_/g, " ").trim().replace(/\b\w/g, (c) => c.toUpperCase());
  }

  private showActionText(message?: string | null): void {
    if (!message || !this.eventManager) {
      return;
    }
    this.eventManager.showText?.(message);
    this.eventManager.waitForInput?.();
  }

  private playSwitchSound(): void {
    this.audioEngine?.playSound("SFX_SWITCH_POKEMON");
  }

  private runPcBootSequence(): [boolean, string | null] {
    if (this.playersPc) {
      playPcBootSound(this.audioEngine);
      this.showPcText("PlayersPCTurnOnText");
      return [true, null];
    }
    if (this.partyCount() <= 0) {
      this.audioEngine?.playSound("SFX_CHOOSE_PC_OPTION");
      const text = this.showPcText("PokecenterPCCantUseText");
      return [false, text];
    }
    playPcBootSound(this.audioEngine);
    this.showPcText("PokecenterPCTurnOnText");
    if (!this.shouldUseAsmPCHubMenu()) {
      this.showPcText("PokecenterPCWhoseText");
    }
    return [true, null];
  }

  private shouldUseAsmPCHubMenu(): boolean {
    return !this.playersPc && isPromptUI(this.ui);
  }

  private showPcText(label: string): string | null {
    const text = this.resolvePcText(label);
    if (!text) {
      return null;
    }
    if (this.eventManager) {
      this.eventManager.showText?.(text);
      this.eventManager.waitForInput?.();
    }
    return text;
  }

  private resolvePcText(label: string): string | null {
    if (!label) {
      return null;
    }
    let text = "";
    if (this.dataLoader?.getText) {
      try {
        text = this.dataLoader.getText(label) ?? "";
      } catch {
        text = "";
      }
    }
    if (!text) {
      text = PC_TEXT_FALLBACKS[label] ?? "";
    }
    if (!text) {
      return null;
    }
    return this.formatPcText(text);
  }

  private formatPcText(text: string): string {
    const playerName = this.playerName();
    return text.replace(/<PLAYER>/g, playerName).replace(/#MON/g, POKEMON_WORD).replace(/<PK><MN>/g, POKEMON_WORD);
  }

  private playerName(): string {
    const name = this.gameState.sram.player_name || "PLAYER";
    return name.trim() || "PLAYER";
  }

  private confirmMoveWithoutMail(): boolean {
    if (!this.interactiveMode) {
      return true;
    }
    const text = this.resolveMoveSaveText();
    this.showActionText(text);
    if (!isPromptUI(this.ui)) {
      return true;
    }
    const eventProvider = this.promptEventProvider();
    const prompt = new SelectionPrompt(this.ui, ["YES", "NO"], {
      audioEngine: this.audioEngine ?? undefined,
    });
    const selection = prompt.run({ drawCallback: this.drawCallback ?? undefined, eventProvider });
    if (selection !== 0) {
      return false;
    }
    this.forceSaveGame();
    return true;
  }

  private async confirmMoveWithoutMailAsync(): Promise<boolean> {
    if (!this.interactiveMode) {
      return true;
    }
    const text = this.resolveMoveSaveText();
    this.showActionText(text);
    if (!isPromptUI(this.ui)) {
      return true;
    }
    const eventProvider = this.promptEventProvider();
    const prompt = new SelectionPrompt(this.ui, ["YES", "NO"], {
      audioEngine: this.audioEngine ?? undefined,
    });
    const selection = await prompt.runAsync({ drawCallback: this.drawCallback ?? undefined, eventProvider });
    if (selection !== 0) {
      return false;
    }
    this.forceSaveGame();
    return true;
  }

  private resolveMoveSaveText(): string {
    const defaultText = `Each time you move a ${POKEMON_WORD}, data will be saved. OK?`;
    if (!this.dataLoader?.getText) {
      return defaultText;
    }
    try {
      const text = this.dataLoader.getText("MoveMonWOMailSaveText");
      if (text) {
        return text;
      }
    } catch {
      return defaultText;
    }
    return defaultText;
  }

  private forceSaveGame(): void {
    void saveGame(this.gameState, "savegame");
  }

  private confirmRelease(): boolean {
    this.eventManager?.showText?.("Release this Pok\u00e9mon?");
    this.eventManager?.waitForInput?.();
    if (!isPromptUI(this.ui)) {
      return false;
    }
    const eventProvider = this.promptEventProvider();
    const prompt = new SelectionPrompt(this.ui, ["YES", "NO"], {
      audioEngine: this.audioEngine ?? undefined,
    });
    const selection = prompt.run({ drawCallback: this.drawCallback ?? undefined, eventProvider });
    return selection === 0;
  }

  private async confirmReleaseAsync(): Promise<boolean> {
    this.eventManager?.showText?.("Release this Pok\u00e9mon?");
    this.eventManager?.waitForInput?.();
    if (!isPromptUI(this.ui)) {
      return false;
    }
    const eventProvider = this.promptEventProvider();
    const prompt = new SelectionPrompt(this.ui, ["YES", "NO"], {
      audioEngine: this.audioEngine ?? undefined,
    });
    const selection = await prompt.runAsync({ drawCallback: this.drawCallback ?? undefined, eventProvider });
    return selection === 0;
  }

  private buildResultMessage(summary: Record<string, unknown>): string {
    const label = (summary.first_selection_name ?? summary.selection_name) as string | undefined;
    if (!label) {
      return "Accessing PC";
    }
    if (label.toUpperCase() === "HALL OF FAME") {
      return "Linking to the Hall of Fame...";
    }
    if (label.toUpperCase() === "TURN OFF") {
      return "Turning off the PC.";
    }
    return `Accessing ${label}`;
  }

  private logDebug(message: string, details?: DebugLogDetails): void {
    if (!isDebugEnabled("pc")) {
      return;
    }
    pushDebugLog(`[pc] ${message}`, details);
  }

  private toDebugNumber(value: unknown): number | undefined {
    const parsed = Number(value);
    if (Number.isNaN(parsed)) {
      return undefined;
    }
    return parsed;
  }

  private logPlayerAction(result: PlayerPCMenuActionResult | null): void {
    if (!result) {
      return;
    }
    this.logDebug("player pc action", {
      action: result.action ? String(result.action) : "",
      status: result.status ? String(result.status) : "",
      item: result.item ? String(result.item) : "",
      quantity: this.toDebugNumber(result.quantity),
    });
  }

  private drawBillPromptBackdrop(): void {
    const screen = this.ui && "screen" in this.ui ? this.ui.screen : null;
    if (!screen) {
      return;
    }
    screen.fill([255, 255, 255, 255]);
  }

  private billPromptDrawCallback(menu?: PokemonPCMenu | null): () => void {
    return () => {
      if (menu) {
        menu.draw();
        return;
      }
      this.drawBillPromptBackdrop();
    };
  }

  static readonly BILL_PC_MENU: Array<[string, string]> = [
    ["WITHDRAW <PK><MN>", "withdraw"],
    ["DEPOSIT <PK><MN>", "deposit"],
    ["CHANGE BOX", "change_box"],
    ["MOVE <PK><MN> W/O MAIL", "move"],
    ["SEE YA!", "see_ya"],
  ];
}

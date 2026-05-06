import { GameState } from "@pokecrystal/core/core/state";
import { EventManager, Event, openText } from "@pokecrystal/core/engine/events/events";
import { MartInterface, type MartUI } from "@pokecrystal/core/ui/menus/mart";
import type { KeyEvent } from "@pokecrystal/core/input/buttons";
import {
  TMHMResolutionError,
  tmhmIndex,
  isHmIndex,
} from "@pokecrystal/core/engine/systems/tmhm";
import { defaultMusicTokenForMap } from "@pokecrystal/core/engine/world/map-music";
import { getFilledSlots as getFilledBoxSlots } from "@pokecrystal/core/core/models/box";
import { getFilledSlots as getFilledPartySlots } from "@pokecrystal/core/core/models/party";
import { countPokedexEntries } from "@pokecrystal/core/core/pokedex";
import { countOwnedBadgesAsm } from "@pokecrystal/core/core/badges";
import { ItemSystem } from "@pokecrystal/core/engine/systems/items";
import {
  LOGGER,
  normalizePhoneNumber,
  resolvePhoneContactId,
  tryAddPhoneNumber,
  showText,
  waitForInput,
} from "../common";
import { applyEventFlag } from "../event-flags";
import {
  loadGlobalConstants,
  resolveScriptConstantExpression,
} from "../script-constants";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";
import {
  Command,
  addItemToBag,
  populateItemStringBuffers,
  queueStandardScript,
  resolveDisplayName,
  resolveItemSystem,
  selectPackFullScript,
  OverworldContext,
  hasItemInSystem,
  removeItemFromSystem,
} from "./base";
import type { ScriptRunner } from "../runner";

const COMPARE_LABELS = ["HAVE_MORE", "HAVE_AMOUNT", "HAVE_LESS"] as const;
const COMPARE_CODES = Object.fromEntries(COMPARE_LABELS.map((label, index) => [label, index]));

type AudioEngineLike = {
  play_sound?: (name: string) => void;
  playSound?: (name: string) => void;
  play_music?: (name: string, role?: string | { role?: string }) => void;
  playMusic?: (name: string, role: string) => void;
  restart_map_music?: () => void;
  restartMapMusic?: () => void;
  requestMapMusic?: (mapName: string, playerState?: number) => void;
};

type ScriptOverworldExtras = {
  dialogue?:
    | Record<string, unknown>
    | { suspend?: () => void; resume?: () => void; _suspended?: boolean };
  audio_engine?: AudioEngineLike | null;
  audioEngine?: AudioEngineLike | null;
  data_loader?: DataLoader | null;
  dataLoader?: DataLoader | null;
  ui?: MartUI;
  pollEvents?: () => KeyEvent[];
  draw?: () => void;
  _mart_interface?: MartInterface;
  _active_coord_event?: unknown;
  reload_current_map?: () => void;
  reloadCurrentMap?: () => void;
  remove_object?: (index: number, options?: boolean | { update_event_flag?: boolean }) => void;
  removeObject?: (index: number, options?: boolean | { update_event_flag?: boolean }) => void;
  consume_active_background_event?: () => unknown;
  consumeActiveBackgroundEvent?: () => unknown;
  remove_background_event?: (event: unknown) => void;
  removeBackgroundEvent?: (event: unknown) => void;
  stop_player_movement?: () => void;
  stopPlayerMovement?: () => void;
  _sync_player_state?: () => void;
  syncPlayerState?: () => void;
  player_px_x?: number;
  player_px_y?: number;
  target_px_x?: number;
  target_px_y?: number;
  player_x?: number;
  player_y?: number;
  target_tile_x?: number;
  target_tile_y?: number;
  player_direction?: string | number;
};

type ScriptOverworld = OverworldContext & ScriptOverworldExtras;

const toScriptOverworld = (overworld?: OverworldContext | null): ScriptOverworld | null =>
  (overworld ?? null) as ScriptOverworld | null;

const resolveMapName = (
  runner?: ScriptRunner | null,
  overworld?: ScriptOverworld | null,
): string | null => {
  if (overworld?.current_map_name) {
    return overworld.current_map_name;
  }
  const targetOverworld = runner?.overworld ?? null;
  if (targetOverworld?.current_map_name) {
    return targetOverworld.current_map_name;
  }
  return null;
};

const resolveScriptAmount = (
  tokens: string[],
  runner?: ScriptRunner | null,
  overworld?: ScriptOverworld | null,
): number => {
  const expr = tokens.map((token) => String(token).trim().replace(/,+$/, "")).filter(Boolean).join(" ");
  if (!expr) {
    throw new Error("Amount expression cannot be empty.");
  }
  const mapName = resolveMapName(runner, overworld);
  return resolveScriptConstantExpression(expr, mapName);
};

const compareAmount = (current: number, target: number): [string, number, boolean] => {
  let label: keyof typeof COMPARE_CODES;
  if (current < target) {
    label = "HAVE_LESS";
  } else if (current === target) {
    label = "HAVE_AMOUNT";
  } else {
    label = "HAVE_MORE";
  }
  return [label, COMPARE_CODES[label], current >= target];
};

const maxCoins = (): number => {
  return loadGlobalConstants()["MAX_COINS"] ?? 9999;
};

const maxMoney = (): number => {
  return loadGlobalConstants()["MAX_MONEY"] ?? 999999;
};

const readRunnerNumericVariable = (runner: ScriptRunner | undefined, name: string): number => {
  const value = runner?.variables?.[name] ?? runner?.variables?.[name.replace(/,+$/, "")] ?? runner?.last_value ?? 0;
  const numeric = Number(value);
  return Number.isFinite(numeric) ? Math.trunc(numeric) : 0;
};

const resolveCurrentObjectEventFlag = (
  runner?: ScriptRunner | null,
  overworld?: ScriptOverworld | null,
): string | null => {
  const gameState = runner?.gameState ?? runner?.game_state ?? null;
  if (!gameState) {
    return null;
  }
  const targetOverworld = overworld ?? (runner?.overworld as ScriptOverworld) ?? null;
  const resolver =
    targetOverworld?.get_event_flag_for_object_index ??
    targetOverworld?.getEventFlagForObjectIndex;
  const lookupObject = targetOverworld?.get_object_by_id ?? targetOverworld?.getObjectById;

  const normalizeFlag = (flagValue: unknown): string | null => {
    const flag = String(flagValue ?? "").trim();
    if (!flag || flag === "-1") {
      return null;
    }
    return flag;
  };

  const candidates: number[] = [];
  const lastTalked = gameState.wram.last_talked;
  if (lastTalked && lastTalked > 0) {
    candidates.push(lastTalked);
  }
  const fallbackIndex = runner?.last_interaction_object_index ?? null;
  if (fallbackIndex && fallbackIndex > 0 && fallbackIndex !== lastTalked) {
    candidates.push(fallbackIndex);
  }

  for (const index of candidates) {
    if (typeof resolver === "function") {
      const flag = normalizeFlag(resolver.call(targetOverworld, index));
      if (flag) {
        return flag;
      }
    }
    if (typeof lookupObject === "function") {
      const object = lookupObject.call(targetOverworld, index) as { event?: { event_flag?: string } } | null;
      const flag = normalizeFlag(object?.event?.event_flag);
      if (flag) {
        return flag;
      }
    }
  }
  return null;
};

export class VerboseGiveItemCommand extends Command {
  constructor(private itemName: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const itemSystem = resolveItemSystem(runner, overworld);
    const success = addItemToBag(gameState, itemSystem, this.itemName);
    const displayName = resolveDisplayName(itemSystem, this.itemName);
    if (runner) {
      populateItemStringBuffers(runner, displayName);
      runner.last_condition_result = success;
      const script = success ? "ReceiveItemScript" : selectPackFullScript(gameState);
      queueStandardScript(runner, script);
    }
  }
}

export class VerboseGiveItemVarCommand extends Command {
  constructor(private itemName: string, private quantityVar: string) {
    super();
    this.quantityVar = this.quantityVar.replace(/,+$/, "").trim();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const itemSystem = resolveItemSystem(runner, overworld);
    const quantity = Math.max(1, readRunnerNumericVariable(runner, this.quantityVar));
    const success = addItemToBag(gameState, itemSystem, this.itemName, quantity);
    const displayName = resolveDisplayName(itemSystem, this.itemName);
    if (runner) {
      populateItemStringBuffers(runner, displayName);
      runner.last_condition_result = success;
      runner.last_value = quantity;
      const script = success ? "ReceiveItemScript" : selectPackFullScript(gameState);
      queueStandardScript(runner, script);
    }
  }
}

export class AskForPhoneNumberCommand extends Command {
  constructor(private phoneNumber: string) {
    super();
  }

  public execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const scriptOverworld = toScriptOverworld(overworld);
    if (!runner) {
      return;
    }
    const numbers = gameState.sram.phone_numbers;
    let accepted = true;

    const consumeChoice = runner._consume_script_choice ?? runner._consumeScriptChoice;
    if (typeof consumeChoice === "function") {
      const override = consumeChoice("_askforphonenumber_choice", null);
      if (override !== null && override !== undefined) {
        accepted = Boolean(override);
      }
    } else if (eventManager && scriptOverworld?.dialogue) {
      const capture: { value: boolean | null } = { value: null };
      eventManager.dispatch(new Event("prompt_yes_no", { callback: (value: boolean) => (capture.value = Boolean(value)) }));
      if (capture.value !== null) {
        accepted = capture.value;
      }
    }

    if (accepted) {
      const added = tryAddPhoneNumber(numbers, this.phoneNumber);
      const scriptValue = added ? 0 : 1;
      gameState.wram.script_memory["wScriptVar"] = scriptValue;
      runner.last_condition_result = Boolean(scriptValue);
      runner.last_value = added ? "PHONE_CONTACT_GOT" : "PHONE_CONTACTS_FULL";
    } else {
      gameState.wram.script_memory["wScriptVar"] = 2;
      runner.last_value = "PHONE_CONTACT_REFUSED";
      runner.last_condition_result = true;
    }
    runner.last_yes_no_result = accepted;
  }
}

export class CheckCellNumCommand extends Command {
  constructor(private phoneNumber: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner;
    const normalized = resolvePhoneContactId(this.phoneNumber);
    const numbers = gameState.sram.phone_numbers.map((entry) => resolvePhoneContactId(entry));
    const scriptValue = normalized && numbers.includes(normalized) ? 1 : 0;
    gameState.wram.script_memory["wScriptVar"] = scriptValue;
    if (runner) {
      runner.last_condition_result = Boolean(scriptValue);
      runner.last_value = scriptValue;
    }
  }
}

export class CheckItemCommand extends Command {
  constructor(private itemName: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    if (!runner) {
      return;
    }
    const itemSystem = resolveItemSystem(runner, overworld);
    if (itemSystem) {
      runner.last_condition_result = hasItemInSystem(itemSystem, this.itemName);
      return;
    }
    const inventories = [gameState.sram.items, gameState.sram.key_items, gameState.sram.balls];
    runner.last_condition_result = inventories.some((source) => (source[this.itemName] ?? 0) > 0);
    if (!runner.last_condition_result) {
      const tmhmFlags = gameState.sram.tm_hm;
      let tmhmMatch = false;
      try {
        const index = tmhmIndex(this.itemName);
        if (index >= 0 && index < tmhmFlags.length) {
          tmhmMatch = Boolean(tmhmFlags[index]);
        }
      } catch (error) {
        if (!(error instanceof TMHMResolutionError)) {
          throw error;
        }
      }
      runner.last_condition_result = tmhmMatch;
    }
  }
}

export class CheckCoinsCommand extends Command {
  constructor(...amountTokens: string[]) {
    super();
    this.amountTokens = amountTokens;
  }
  private amountTokens: string[];

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const amount = resolveScriptAmount(this.amountTokens, runner, toScriptOverworld(overworld));
    const coins = Number(gameState.sram.coins ?? 0);
    const [label, code, enough] = compareAmount(coins, amount);
    gameState.wram.script_memory["wScriptVar"] = code;
    if (runner) {
      runner.last_value = label;
      if (!runner.variables) {
        runner.variables = {};
      }
      runner.variables["_value"] = label;
      runner.last_condition_result = enough;
    }
  }
}

export class CheckMoneyCommand extends Command {
  constructor(private readonly account: string, ...amountTokens: string[]) {
    super();
    this.amountTokens = amountTokens;
  }
  private amountTokens: string[];

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const amount = resolveScriptAmount(this.amountTokens, runner, toScriptOverworld(overworld));
    const account = String(this.account ?? "").replace(/,+$/, "").trim().toUpperCase();
    const current = account === "MOMS_MONEY"
      ? Number(gameState.sram.moms_money ?? 0)
      : Number(gameState.sram.money ?? 0);
    const [label, code, enough] = compareAmount(current, amount);
    gameState.wram.script_memory["wScriptVar"] = code;
    if (runner) {
      runner.last_value = label;
      if (!runner.variables) {
        runner.variables = {};
      }
      runner.variables["_value"] = label;
      runner.last_condition_result = enough;
    }
  }
}

export class GiveCoinsCommand extends Command {
  constructor(...amountTokens: string[]) {
    super();
    this.amountTokens = amountTokens;
  }
  private amountTokens: string[];

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const amount = resolveScriptAmount(this.amountTokens, runner, toScriptOverworld(overworld));
    let coins = Number(gameState.sram.coins ?? 0) + amount;
    coins = Math.min(maxCoins(), Math.max(0, coins));
    gameState.sram.coins = coins;
    if (runner) {
      runner.last_value = coins;
      if (!runner.variables) {
        runner.variables = {};
      }
      runner.variables["_value"] = coins;
      runner.last_condition_result = true;
    }
  }
}

export class TakeCoinsCommand extends Command {
  constructor(...amountTokens: string[]) {
    super();
    this.amountTokens = amountTokens;
  }
  private amountTokens: string[];

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const amount = resolveScriptAmount(this.amountTokens, runner, toScriptOverworld(overworld));
    let coins = Number(gameState.sram.coins ?? 0) - amount;
    coins = Math.max(0, coins);
    gameState.sram.coins = coins;
    if (runner) {
      runner.last_value = coins;
      if (!runner.variables) {
        runner.variables = {};
      }
      runner.variables["_value"] = coins;
      runner.last_condition_result = true;
    }
  }
}

export class TakeMoneyCommand extends Command {
  constructor(private readonly account: string, ...amountTokens: string[]) {
    super();
    this.amountTokens = amountTokens;
  }
  private amountTokens: string[];

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const amount = resolveScriptAmount(this.amountTokens, runner, toScriptOverworld(overworld));
    const account = String(this.account ?? "").replace(/,+$/, "").trim().toUpperCase();
    if (account === "MOMS_MONEY") {
      gameState.sram.moms_money = Math.max(0, Number(gameState.sram.moms_money ?? 0) - amount);
      runner && (runner.last_value = gameState.sram.moms_money);
    } else {
      gameState.sram.money = Math.max(0, Math.min(maxMoney(), Number(gameState.sram.money ?? 0) - amount));
      runner && (runner.last_value = gameState.sram.money);
    }
    if (runner) {
      if (!runner.variables) {
        runner.variables = {};
      }
      runner.variables["_value"] = runner.last_value;
      runner.last_condition_result = true;
    }
  }
}

export class CheckPokeCommand extends Command {
  constructor(private readonly speciesName: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner;
    const target = String(this.speciesName ?? "").trim().toUpperCase();
    const found = (gameState.sram.party?.pokemon ?? []).some((pokemon) => {
      const species = pokemon?.species;
      const id = typeof species === "string" ? species : species?.id;
      return String(id ?? "").toUpperCase() === target;
    });
    gameState.wram.script_memory["wScriptVar"] = found ? 1 : 0;
    if (runner) {
      runner.last_value = found ? target : null;
      runner.last_condition_result = found;
    }
  }
}

export class ItemBallCommand extends Command {
  constructor(private itemName: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    if (!runner) {
      throw new Error("ItemBallCommand requires an active ScriptRunner.");
    }
    const itemSystem = resolveItemSystem(runner, overworld);
    const success = addItemToBag(gameState, itemSystem, this.itemName);
    const displayName = resolveDisplayName(itemSystem, this.itemName);
    populateItemStringBuffers(runner, displayName);
    if (!runner.string_buffers) {
      runner.string_buffers = {};
    }
    runner.string_buffers["STRING_BUFFER_3"] = displayName;
    runner.last_condition_result = success;
    const targetOverworld = toScriptOverworld(overworld ?? runner.overworld ?? null);
    if (success) {
      const runnerState = runner.gameState ?? runner.game_state ?? null;
      const lastTalked = runnerState?.wram?.last_talked ?? 0;
      if (lastTalked > 0) {
        if (!targetOverworld) {
          throw new Error("ItemBallCommand cannot remove the collected object without an overworld controller.");
        }
        const flag = resolveCurrentObjectEventFlag(runner, targetOverworld);
        if (!flag) {
          throw new Error(`Missing event flag mapping for overworld object index ${lastTalked}.`);
        }
        applyEventFlag(gameState, flag, { value: true, overworld: targetOverworld });
        const removeMethod = targetOverworld.remove_object ?? targetOverworld.removeObject;
        if (typeof removeMethod === "function") {
          if (removeMethod.length >= 2) {
            try {
              removeMethod.call(targetOverworld, lastTalked, { update_event_flag: false });
            } catch {
              removeMethod.call(targetOverworld, lastTalked, false);
            }
          } else {
            removeMethod.call(targetOverworld, lastTalked);
          }
        }
      }
    }
    queueStandardScript(runner, "FindItemInBallScript");
  }
}

export class HiddenItemCommand extends Command {
  constructor(private itemName: string, private eventFlag: string) {
    super();
  }

  public execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const scriptOverworld = toScriptOverworld(overworld);
    if (gameState.wram.event_flags[this.eventFlag]) {
      if (runner) {
        runner.last_condition_result = false;
      }
      return;
    }

    const itemSystem = resolveItemSystem(runner, overworld);
    const displayName = resolveDisplayName(itemSystem, this.itemName);
    populateItemStringBuffers(runner, displayName);
    if (runner) {
      if (!runner.string_buffers) {
        runner.string_buffers = {};
      }
      runner.string_buffers["STRING_BUFFER_3"] = displayName;
    }
    let message = `<PLAYER> found\n${displayName}!`;
    if (runner?.formatText) {
      message = runner.formatText(message);
    } else {
      const playerName = gameState.sram.player_name.trim() || "PLAYER";
      message = message.replace("<PLAYER>", playerName);
    }

    openText(eventManager);
    showText(eventManager, message, { auto_close_after_wait: true });

    const success = addItemToBag(gameState, itemSystem, this.itemName);
    let audioEngine = scriptOverworld?.audio_engine ?? scriptOverworld?.audioEngine ?? null;
    if (!audioEngine && runner) {
      audioEngine = runner.audio_engine ?? runner.audioEngine ?? null;
    }
    if (success) {
      const targetOverworld = toScriptOverworld(overworld ?? runner?.overworld ?? null);
      applyEventFlag(gameState, this.eventFlag, { value: true, overworld: targetOverworld });
      if (targetOverworld) {
        const consumeEvent = targetOverworld.consume_active_background_event ?? targetOverworld.consumeActiveBackgroundEvent;
        if (typeof consumeEvent === "function") {
          const bgEvent = consumeEvent.call(targetOverworld);
          const removeEvent = targetOverworld.remove_background_event ?? targetOverworld.removeBackgroundEvent;
          if (typeof removeEvent === "function") {
            removeEvent.call(targetOverworld, bgEvent);
          }
        }
      }
      if (audioEngine?.play_sound) {
        audioEngine.play_sound("SFX_ITEM");
      } else if (audioEngine?.playSound) {
        audioEngine.playSound("SFX_ITEM");
      }
      if (runner?._script_stack) {
        runner.pause?.();
      }
      waitForInput(eventManager);
      if (runner) {
        runner.last_condition_result = true;
        runner.last_sound_effect = "SFX_ITEM";
      }
      return;
    }

    if (runner) {
      runner.last_condition_result = false;
    }
    let failureMessage = "But <PLAYER> has\nno space left...";
    if (runner?.formatText) {
      failureMessage = runner.formatText(failureMessage);
    } else {
      const playerName = gameState.sram.player_name.trim() || "PLAYER";
      failureMessage = failureMessage.replace("<PLAYER>", playerName);
    }
    showText(eventManager, failureMessage, { auto_close_after_wait: true });
    if (runner?._script_stack) {
      runner.pause?.();
    }
    waitForInput(eventManager);
  }
}

export class ItemNotifyCommand extends Command {
  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner;
    if (runner) {
      runner.last_condition_result = true;
    }
  }
}

export class LoadVarCommand extends Command {
  private varName: string;
  private value: string;

  constructor(varName: string, value: string) {
    super();
    this.varName = varName.replace(/,+$/, "").trim();
    this.value = value.replace(/,+$/, "").trim();
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner;
    if (runner) {
      if (!runner.variables) {
        runner.variables = {};
      }
      runner.variables[this.varName] = this.value;
    }
    if (this.varName.toUpperCase() === "VAR_BATTLETYPE") {
      gameState.wram.battle_type = this.value;
    }
  }
}

export class ReadVarCommand extends Command {
  private static readonly BATTLE_RESULT_MASK = (1 << 6) | (1 << 7);
  private static readonly TIME_OF_DAY_MASKS: Record<string, number> = {
    morn: 0b001,
    day: 0b010,
    nite: 0b100,
    darkness: 0b100,
  };
  private static readonly TIME_OF_DAY_ALIASES: Record<string, string> = {
    morning: "morn",
    night: "nite",
    evening: "nite",
  };
  private static readonly FACING_LABELS: Record<string, string> = {
    up: "UP",
    down: "DOWN",
    left: "LEFT",
    right: "RIGHT",
  };

  constructor(private varName: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const scriptOverworld = toScriptOverworld(overworld);
    if (!runner) {
      return;
    }
    const value = this.resolveVarValue(gameState, scriptOverworld, runner);
    const resolved = value === undefined || value === null ? runner.variables?.[this.varName] : value;
    if (value !== undefined && value !== null) {
      if (!runner.variables) {
        runner.variables = {};
      }
      runner.variables[this.varName] = value;
    }
    runner.last_value = resolved;
    runner.last_condition_result = Boolean(resolved);
    gameState.wram.script_memory["wScriptVar"] = typeof resolved === "number" ? resolved : Number(Boolean(resolved));
  }

  private resolveVarValue(gameState: GameState, overworld: ScriptOverworld | null, runner: ScriptRunner): unknown {
    const name = this.varName.toUpperCase();
    if (name === "VAR_STRINGBUFFER2") {
      return runner.string_buffers?.["STRING_BUFFER_2"];
    }
    if (name === "VAR_PARTYCOUNT") {
      const party = gameState.sram.party;
      return party ? getFilledPartySlots(party) : 0;
    }
    if (name === "VAR_BATTLERESULT") {
      return Number(gameState.wram.battle_result ?? 0) & ~ReadVarCommand.BATTLE_RESULT_MASK;
    }
    if (name === "VAR_BATTLETYPE") {
      return gameState.wram.battle_type;
    }
    if (name === "VAR_TIMEOFDAY") {
      return this.resolveTimeOfDay(gameState);
    }
    if (name === "VAR_DEXCAUGHT") {
      return countPokedexEntries(gameState.sram.pokedex_owned);
    }
    if (name === "VAR_DEXSEEN") {
      return countPokedexEntries(gameState.sram.pokedex_seen);
    }
    if (name === "VAR_UNOWNCOUNT") {
      return this.countUnown(gameState);
    }
    if (name === "VAR_BADGES") {
      return this.countBadges(gameState);
    }
    if (name === "VAR_BLUECARDBALANCE") {
      return Number(gameState.wram.blue_card_balance ?? 0);
    }
    if (name === "VAR_FACING") {
      return this.resolveFacing(overworld);
    }
    if (name === "VAR_HOUR") {
      return gameState.hram.hHours ?? 0;
    }
    if (name === "VAR_WEEKDAY") {
      return this.resolveWeekday(gameState);
    }
    if (name === "VAR_MAPGROUP") {
      return gameState.wram.wMapGroup ?? 0;
    }
    if (name === "VAR_MAPNUMBER") {
      return gameState.wram.wMapNumber ?? 0;
    }
    if (name === "VAR_BOXSPACE") {
      return this.resolveBoxSpace(gameState);
    }
    if (name === "VAR_CONTESTMINUTES") {
      const minutes = gameState.wram.bug_contest_timer?.mins_remaining;
      return minutes ?? 0;
    }
    if (name === "VAR_XCOORD") {
      return gameState.wram.wXCoord ?? 0;
    }
    if (name === "VAR_YCOORD") {
      return gameState.wram.wYCoord ?? 0;
    }
    if (name === "VAR_SPECIALPHONECALL") {
      const queue = gameState.wram.scheduled_phone_calls;
      if (!queue.length) {
        return null;
      }
      const entry = queue[0];
      if (typeof entry !== "string") {
        return entry ?? null;
      }
      const trimmed = entry.trim();
      return trimmed ? trimmed.toUpperCase() : null;
    }
    if (name === "VAR_CALLERID") {
      return runner.variables?.["VAR_CALLERID"];
    }
    return null;
  }

  private resolveTimeOfDay(gameState: GameState): number | string {
    let label = gameState.wram.time_of_day;
    if (!label && gameState.sram.start_time) {
      const hour = Number(gameState.sram.start_time.hour ?? 0);
      if (hour < 4) {
        label = "nite";
      } else if (hour < 10) {
        label = "morn";
      } else if (hour < 18) {
        label = "day";
      } else {
        label = "nite";
      }
    }
    const normalized = String(label ?? "day").trim().toLowerCase();
    const alias = ReadVarCommand.TIME_OF_DAY_ALIASES[normalized] ?? normalized;
    return ReadVarCommand.TIME_OF_DAY_MASKS[alias] ?? alias;
  }

  private resolveWeekday(gameState: GameState): number {
    const dayCounter = gameState.wram.wCurDay;
    if (typeof dayCounter === "number") {
      return dayCounter % 7;
    }
    const weekday = gameState.sram.day_of_week ?? 0;
    const numeric = Number(weekday);
    return Number.isFinite(numeric) ? numeric % 7 : 0;
  }

  private countBadges(gameState: GameState): number {
    // ASM mapping: pokecrystal_disassembly/engine/overworld/variables.asm::VAR_BADGES
    const badges = gameState.sram.badges;
    if (!badges) {
      return 0;
    }
    return countOwnedBadgesAsm(badges, "ReadVarCommand VAR_BADGES");
  }

  private countUnown(gameState: GameState): number {
    const dex = gameState.wram.wUnownDex ?? [];
    return dex.filter(Boolean).length;
  }

  private resolveFacing(overworld: ScriptOverworld | null): string | null {
    if (!overworld?.player_direction) {
      return null;
    }
    const direction = String(overworld.player_direction).toLowerCase();
    return ReadVarCommand.FACING_LABELS[direction] ?? null;
  }

  private resolveBoxSpace(gameState: GameState): number {
    const boxes = gameState.sram.pc_boxes ?? [];
    if (!boxes.length) {
      return 0;
    }
    const index = Number(gameState.sram.current_pc_box ?? 0);
    if (!Number.isInteger(index) || index < 0 || index >= boxes.length) {
      throw new Error(
        `ASM-backed current PC box index is invalid: ${String(gameState.sram.current_pc_box)}.`
      );
    }
    const box = boxes[index];
    const totalSlots = box?.pokemon?.length ?? 0;
    const filled = box ? getFilledBoxSlots(box) : 0;
    return Math.max(0, totalSlots - filled);
  }
}

export class SetValCommand extends Command {
  constructor(private value: string) {
    super();
  }

  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner;
    if (runner) {
      if (!runner.variables) {
        runner.variables = {};
      }
      runner.variables["_value"] = this.value;
      runner.last_value = this.value;
    }
  }
}

export class WriteVarCommand extends Command {
  constructor(private varName: string) {
    super();
    this.varName = this.varName.replace(/,+$/, "").trim();
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner;
    if (!runner) {
      return;
    }
    const value = Number(runner.variables?.["_value"] ?? runner.last_value ?? 0);
    const numeric = Number.isFinite(value) ? Math.trunc(value) : 0;
    if (this.varName.toUpperCase() === "VAR_BLUECARDBALANCE") {
      gameState.wram.blue_card_balance = Math.max(0, Math.min(30, numeric));
    }
    if (!runner.variables) {
      runner.variables = {};
    }
    runner.variables[this.varName] = numeric;
    runner.last_value = numeric;
    runner.last_condition_result = numeric !== 0;
  }
}

export class GetNumCommand extends Command {
  constructor(private bufferName: string) {
    super();
  }

  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner;
    if (!runner) {
      return;
    }
    const value = Number(runner.variables?.["_value"] ?? runner.last_value ?? 0);
    if (!runner.string_buffers) {
      runner.string_buffers = {};
    }
    runner.string_buffers[this.bufferName] = String(Number.isFinite(value) ? Math.trunc(value) : 0);
  }
}

export class PlayMapMusicCommand extends Command {
  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const scriptOverworld = toScriptOverworld(overworld);
    if (!scriptOverworld) {
      return;
    }
    if (!scriptOverworld.audio_engine && !scriptOverworld.audioEngine) {
      return;
    }
    const audio = scriptOverworld.audio_engine ?? scriptOverworld.audioEngine;
    if (typeof audio?.requestMapMusic === "function") {
      audio.requestMapMusic(scriptOverworld.current_map_name);
      return;
    }
    if (gameState.wram.dont_restart_map_music) {
      gameState.wram.dont_restart_map_music = false;
      return;
    }
    const token = defaultMusicTokenForMap(scriptOverworld.current_map_name);
    if (String(gameState.wram.wMapMusic ?? "").trim() === token) {
      return;
    }
    if (typeof audio?.playMusic === "function") {
      audio.playMusic(token, "map");
    } else {
      audio?.play_music?.(token, { role: "map" });
    }
    gameState.wram.wMapMusic = token;
  }
}

export class ReloadMapCommand extends Command {
  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const scriptOverworld = toScriptOverworld(overworld);
    const mapName = scriptOverworld?.current_map_name;
    LOGGER.debug(
      "Reloading map %s via ReloadMapCommand (active_coord=%s)",
      mapName ?? "<unknown>",
      scriptOverworld?._active_coord_event,
    );

    if (runner?.just_battled) {
      runner.pending_reload_map = mapName ?? scriptOverworld?.current_map_name ?? null;
      return;
    }

    const activeCoord = scriptOverworld?._active_coord_event ?? null;
    if (scriptOverworld) {
      const reload = scriptOverworld.reload_current_map ?? scriptOverworld.reloadCurrentMap;
      if (typeof reload === "function") {
        reload.call(scriptOverworld);
      }
      if (activeCoord !== null) {
        scriptOverworld._active_coord_event = activeCoord;
      }
    }
  }
}

export class ReanchorMapCommand extends Command {
  constructor(private anchor: string | null = null) {
    super();
  }

  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const scriptOverworld = toScriptOverworld(this.runner?.overworld ?? null);
    if (!scriptOverworld) {
      return;
    }
    const stopPlayer = scriptOverworld.stop_player_movement ?? scriptOverworld.stopPlayerMovement;
    if (typeof stopPlayer === "function") {
      stopPlayer.call(scriptOverworld);
      return;
    }
    const syncPlayer = scriptOverworld._sync_player_state ?? scriptOverworld.syncPlayerState;
    if (typeof syncPlayer === "function") {
      syncPlayer.call(scriptOverworld);
    }
    if (scriptOverworld.player_px_x !== undefined && "target_px_x" in scriptOverworld) {
      scriptOverworld.target_px_x = scriptOverworld.player_px_x;
    }
    if (scriptOverworld.player_px_y !== undefined && "target_px_y" in scriptOverworld) {
      scriptOverworld.target_px_y = scriptOverworld.player_px_y;
    }
    if ("player_x" in scriptOverworld && "target_tile_x" in scriptOverworld) {
      scriptOverworld.target_tile_x = scriptOverworld.player_x;
    }
    if ("player_y" in scriptOverworld && "target_tile_y" in scriptOverworld) {
      scriptOverworld.target_tile_y = scriptOverworld.player_y;
    }
  }
}

export class TakeItemCommand extends Command {
  constructor(private itemName: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const itemSystem = resolveItemSystem(runner, overworld);
    if (itemSystem) {
      removeItemFromSystem(itemSystem, this.itemName);
      return;
    }
    for (const inventory of [gameState.sram.items, gameState.sram.key_items, gameState.sram.balls, gameState.sram.tm_hm]) {
      if (Array.isArray(inventory)) {
        try {
          const index = tmhmIndex(this.itemName);
          if (index < inventory.length && inventory[index]) {
            if (isHmIndex(index)) {
              continue;
            }
            inventory[index] = 0;
            break;
          }
        } catch (error) {
          if (!(error instanceof TMHMResolutionError)) {
            throw error;
          }
        }
      } else if (this.itemName in inventory) {
        inventory[this.itemName] -= 1;
        if (inventory[this.itemName] <= 0) {
          delete inventory[this.itemName];
        }
        break;
      }
    }
  }
}

export class GetItemNameCommand extends Command {
  constructor(private bufferName: string, private itemName: string) {
    super();
  }

  public execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    if (!runner) {
      return;
    }
    const scriptOverworld = toScriptOverworld(overworld);
    const itemSystem = resolveItemSystem(runner, overworld);
    let name: string;
    if (itemSystem) {
      name = resolveDisplayName(itemSystem, this.itemName);
    } else {
      const dataLoader =
        scriptOverworld?.data_loader ??
        scriptOverworld?.dataLoader ??
        runner?.data_loader ??
        runner?.dataLoader ??
        null;
      if (dataLoader) {
        const item =
          dataLoader.get_item?.(this.itemName) ??
          (dataLoader as DataLoader & { getItem?: DataLoader["get_item"] }).getItem?.(
            this.itemName,
          );
        const resolved = String(item?.name ?? "").trim();
        if (!resolved) {
          throw new Error(`Missing ASM item name for '${this.itemName}'.`);
        }
        name = resolved;
      } else {
        throw new Error(`Script_getitemname requires item data for '${this.itemName}'.`);
      }
    }
    if (!runner.string_buffers) {
      runner.string_buffers = {};
    }
    runner.string_buffers[this.bufferName] = name;
  }
}

export class GetTrainerNameCommand extends Command {
  constructor(private bufferName: string, private trainerClass: string, private trainerName: string) {
    super();
  }

  public execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    if (!runner) {
      return;
    }
    const scriptOverworld = toScriptOverworld(overworld);
    const dataLoader =
      scriptOverworld?.data_loader ??
      scriptOverworld?.dataLoader ??
      runner?.data_loader ??
      runner?.dataLoader ??
      null;
    if (!dataLoader) {
      throw new Error(
        `Script_gettrainername requires trainer data for '${this.trainerClass}/${this.trainerName}'.`,
      );
    }
    const trainer =
      dataLoader.get_trainer?.(this.trainerName) ??
      (dataLoader as DataLoader & { getTrainer?: DataLoader["get_trainer"] }).getTrainer?.(
        this.trainerName,
      );
    const resolved = String(trainer?.name ?? "").trim();
    if (!resolved) {
      throw new Error(`Missing ASM trainer name for '${this.trainerClass}/${this.trainerName}'.`);
    }
    if (!runner.string_buffers) {
      runner.string_buffers = {};
    }
    runner.string_buffers[this.bufferName] = resolved;
  }
}

export class GiveItemCommand extends Command {
  constructor(private itemName: string, private quantity: number = 1) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const itemSystem = resolveItemSystem(runner, overworld);
    const success = addItemToBag(gameState, itemSystem, this.itemName, this.quantity);
    const displayName = resolveDisplayName(itemSystem, this.itemName);
    if (runner) {
      populateItemStringBuffers(runner, displayName);
      runner.last_condition_result = success;
      const script = success ? "ReceiveItemScript" : selectPackFullScript(gameState);
      queueStandardScript(runner, script);
    }
  }
}

export class PokemartCommand extends Command {
  constructor(private martType: string, private martIdentifier: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const scriptOverworld = toScriptOverworld(overworld);
    let itemSystem = resolveItemSystem(runner, overworld);
    const dataLoader =
      scriptOverworld?.data_loader ??
      scriptOverworld?.dataLoader ??
      runner?.data_loader ??
      runner?.dataLoader ??
      null;
    if (!dataLoader) {
      return;
    }
    if (!itemSystem) {
      itemSystem = new ItemSystem(gameState, dataLoader);
      if (scriptOverworld) {
        scriptOverworld.item_system = itemSystem;
      }
      if (runner) {
        runner.item_system = itemSystem;
      }
    }
    let martInterface = scriptOverworld?._mart_interface ?? null;
    if (!(martInterface instanceof MartInterface)) {
      const martOverworld = scriptOverworld ?? (overworld as ScriptOverworld);
      martInterface = new MartInterface(martOverworld, gameState, dataLoader, itemSystem);
      if (scriptOverworld) {
        scriptOverworld._mart_interface = martInterface;
      }
    } else {
      martInterface.updateContext(gameState, dataLoader, itemSystem);
    }
    const queueTask = runner?._queue_overworld_task ?? runner?._queueOverworldTask;
    const openAsync = typeof martInterface.openAsync === "function" ? martInterface.openAsync.bind(martInterface) : null;
    if (typeof queueTask === "function" && openAsync) {
      queueTask.call(runner, (callback: () => void) => {
        void (async () => {
          try {
            await openAsync(this.martType, this.martIdentifier);
          } finally {
            callback();
          }
        })();
        return true;
      });
      return;
    }
    if (openAsync) {
      runner?.pause?.();
      void openAsync(this.martType, this.martIdentifier)
        .catch((error) => {
          console.error("PokemartCommand async flow failed:", error);
        })
        .finally(() => {
          runner?.resume?.();
        });
      return;
    }
    martInterface.open(this.martType, this.martIdentifier);
  }
}

export class AddCellNumCommand extends Command {
  constructor(private phoneNumber: string) {
    super();
    this.phoneNumber = this.phoneNumber.replace(/,+$/, "");
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const added = tryAddPhoneNumber(gameState.sram.phone_numbers, this.phoneNumber);
    const scriptValue = added ? 0 : 1;
    gameState.wram.script_memory["wScriptVar"] = scriptValue;
    const runner = this.runner;
    if (runner) {
      runner.last_condition_result = Boolean(scriptValue);
      runner.last_value = scriptValue;
    }
  }
}

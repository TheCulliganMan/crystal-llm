import { GameState } from "@pokecrystal/core/core/state";
import { EventManager, Event, openText, closeText } from "@pokecrystal/core/engine/events/events";
import { getDecorationConstant } from "@pokecrystal/assets/content/decorations";
import { getMapMetadataByConstant, mapConstantToName } from "@pokecrystal/core/engine/world/maps";
import { OverworldMap } from "@pokecrystal/core/engine/world/overworld";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { LOGGER, showText, waitForInput } from "../common";
import { applyEventFlag, clearEventFlag } from "../event-flags";
import { resolveText } from "../text-helpers";
import { resolveScriptConstantExpression } from "../script-constants";
import { Command, normalizeScriptName, ScriptFrame, OverworldContext } from "./base";
import type { ScriptRunner } from "../runner";
import { pushDebugLog } from "@pokecrystal/core/core/debug-log";
import { setOwnedBadgeByEngineFlagAsm } from "@pokecrystal/core/core/badges";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";

const isEngineFlagName = (flagName: string): boolean =>
  flagName.startsWith("ENGINE_") || flagName.startsWith("STATUSFLAGS_");

const isFlagSet = (gameState: GameState, flagName: string): boolean => {
  const wram = gameState.wram;
  if (isEngineFlagName(flagName)) {
    return Boolean(wram.engine_flags[flagName]);
  }
  return Boolean(wram.event_flags[flagName]);
};

const setEngineFlag = (gameState: GameState, flagName: string, value: boolean): void => {
  gameState.wram.engine_flags[flagName] = value;
  setOwnedBadgeByEngineFlagAsm(
    gameState.sram.badges,
    flagName,
    value,
    `Script engine flag ${flagName}`
  );
};

const isSpecialCallIdentifier = (value: string | null | undefined): boolean => {
  const token = value ? String(value).trim() : "";
  return Boolean(token) && token.toUpperCase().startsWith("SPECIALCALL_");
};

const removeSpecialCallsFromQueue = (queue: string[]): void => {
  const filtered = queue.filter((entry) => !isSpecialCallIdentifier(entry));
  queue.splice(0, queue.length, ...filtered);
};

const normalizeMapName = (mapName: string): string => {
  if (!mapName) {
    return mapName;
  }
  if (mapName.includes("_") && mapName.toUpperCase() === mapName) {
    return mapConstantToName(mapName);
  }
  return mapName;
};

type OverworldNameAccessor = {
  mapName?: string;
  map_name?: string;
  current_map_name?: string;
  currentMapName?: string;
  map?: {
    mapName?: string;
    map_name?: string;
  } | null;
};

const resolveOverworldMapName = (
  overworld: OverworldMap | OverworldEngine | null | undefined,
): string => {
  if (!overworld) {
    return "";
  }
  const candidate = overworld as OverworldNameAccessor;
  const direct =
    candidate.mapName ??
    candidate.map_name ??
    candidate.current_map_name ??
    candidate.currentMapName ??
    "";
  if (direct) {
    return String(direct);
  }
  const nestedMap = candidate.map ?? null;
  const nested = nestedMap?.mapName ?? nestedMap?.map_name ?? "";
  return nested ? String(nested) : "";
};

const ensureMapSceneInitialized = (
  runner: ScriptRunner,
  gameState: GameState,
  mapName: string,
): { name: string; index: number } | null => {
  const mapKey = normalizeMapName(mapName);
  const wram = gameState.wram;
  if (mapKey in wram.map_scene_indices) {
    const index = wram.map_scene_indices[mapKey];
    let name = wram.map_scenes[mapKey] ?? "";
    if (!name) {
      const order = runner.dataLoader?.map_scene_order?.get?.(mapKey);
      if (Array.isArray(order) && order.length) {
        name = order[0];
        wram.map_scenes[mapKey] = name;
      }
    }
    return { name: name ?? "", index: typeof index === "number" ? index : 0 };
  }

  const order = runner.dataLoader?.map_scene_order?.get?.(mapKey);
  if (!Array.isArray(order) || !order.length) {
    return null;
  }

  const existing = wram.map_scenes[mapKey];
  let sceneName = existing;
  let index = 0;
  if (sceneName && order.includes(sceneName)) {
    index = order.indexOf(sceneName);
  } else {
    const defaultScene = runner.dataLoader?.map_default_scene?.[mapKey] ?? order[0];
    sceneName = order.includes(defaultScene) ? defaultScene : order[0];
    index = order.indexOf(sceneName);
  }

  wram.map_scenes[mapKey] = sceneName;
  wram.map_scene_indices[mapKey] = index;
  if (runner.overworld?.current_map_name === mapKey) {
    wram.scene_name = sceneName;
  }
  return { name: sceneName, index };
};

type EventCommandRunner = ScriptRunner & {
  _set_map_scene?: (mapName: string, sceneName: string) => void;
  _script_stack?: ScriptFrame[];
  jump?: (scriptName: string, parentScript?: string | null) => void;
  defer?: (scriptName: string) => void;
  queue_phone_call?: (contact: string) => void;
  just_battled?: boolean;
  stop_execution?: boolean;
  run_standard_script?: (scriptName: string) => void;
};

type OverworldObjectResolver = {
  resolve_object_index?: (identifier: string) => number | null;
  resolveObjectIndex?: (identifier: string) => number | null;
};

export class SetMapSceneCommand extends Command {
  constructor(private mapName: string, private sceneName: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (!runner) {
      throw new Error("SetMapSceneCommand requires an active script runner.");
    }
    const resolvedMap = mapConstantToName(this.mapName);
    LOGGER.debug("SetMapSceneCommand targeting map %s scene %s", resolvedMap, this.sceneName);
    if (typeof runner._set_map_scene === "function") {
      runner._set_map_scene(resolvedMap, this.sceneName);
      return;
    }
    ensureMapSceneInitialized(runner, gameState, resolvedMap);
    gameState.wram.map_scenes[resolvedMap] = this.sceneName;
  }
}

export class CheckSceneCommand extends Command {
  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (!runner || !overworld) {
      throw new Error("CheckSceneCommand requires overworld and runner context.");
    }
    const mapName = resolveOverworldMapName(overworld);
    const mapKey = normalizeMapName(mapName);
    ensureMapSceneInitialized(runner, gameState, mapKey);
    const sceneName = gameState.wram.map_scenes[mapKey] ?? "";
    const sceneIndex = gameState.wram.map_scene_indices[mapKey] ?? 0;
    gameState.wram.scene_name = sceneName;
    pushDebugLog(`[script] checkscene ${mapKey} -> ${sceneName || "none"} (${sceneIndex})`);
    runner.last_value = sceneIndex;
    runner.last_condition_result = sceneIndex !== 0;
  }
}

export class ClearFlagCommand extends Command {
  constructor(private flagName: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldMap): void {
    // ASM: engine/overworld/scripting.asm::Script_setflag (clearflag mirrors engine flags)
    if (isEngineFlagName(this.flagName)) {
      setEngineFlag(gameState, this.flagName, false);
      if (this.flagName === "ENGINE_UNOWN_DEX") {
        gameState.sram.unown_dex = false;
        gameState.wram.wUnlockedUnownMode = false;
      }
      return;
    }
    clearEventFlag(gameState, this.flagName, { overworld });
  }
}

export class SetFlagCommand extends Command {
  constructor(private flagName: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldMap): void {
    // ASM: engine/overworld/scripting.asm::Script_setflag
    if (isEngineFlagName(this.flagName)) {
      setEngineFlag(gameState, this.flagName, true);
      if (this.flagName === "ENGINE_UNOWN_DEX") {
        gameState.sram.unown_dex = true;
        gameState.wram.wUnlockedUnownMode = true;
      }
      return;
    }
    applyEventFlag(gameState, this.flagName, { value: true, overworld });
  }
}

export class SetEngineFlagCommand extends Command {
  constructor(private flagName: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldMap): void {
    setEngineFlag(gameState, this.flagName, true);
    if (this.flagName === "ENGINE_UNOWN_DEX") {
      gameState.sram.unown_dex = true;
      gameState.wram.wUnlockedUnownMode = true;
    }
    LOGGER.debug("SetEngineFlagCommand set %s", this.flagName);
  }
}

export class VariableSpriteCommand extends Command {
  constructor(private spriteIdentifier: string, private replacementSprite: string) {
    super();
    this.spriteIdentifier = this.spriteIdentifier.replace(/,+$/, "");
    this.replacementSprite = this.replacementSprite.replace(/,+$/, "");
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldMap): void {
    gameState.wram.variable_sprites[this.spriteIdentifier] = this.replacementSprite;
    LOGGER.debug(
      "VariableSpriteCommand remapped %s -> %s",
      this.spriteIdentifier,
      this.replacementSprite,
    );
  }
}

export class DescribeDecorationCommand extends Command {
  private static readonly DESCRIPTIONS: Record<string, string> = {
    DECODESC_LEFT_DOLL: "A cute doll. You feel watched whenever you move.",
    DECODESC_RIGHT_DOLL: "It's your favorite doll from childhood.",
    DECODESC_BIG_DOLL: "A giant plush doll. It's incredibly soft.",
    DECODESC_CONSOLE: "A compact game console. Better save before playing!",
  };
  private static readonly POSTER_TEXT_LABELS: Record<string, string> = {
    DECO_PIKACHU_POSTER: "LookPikachuPosterText",
    DECO_CLEFAIRY_POSTER: "LookClefairyPosterText",
    DECO_JIGGLYPUFF_POSTER: "LookJigglypuffPosterText",
  };

  constructor(private descriptor: string) {
    super();
    this.descriptor = this.descriptor.trim();
  }

  public execute(gameState: GameState, eventManager: EventManager, overworld: OverworldMap): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (this.descriptor === "DECODESC_POSTER") {
      this.describePoster(gameState, eventManager, overworld, runner);
      return;
    }
    const text =
      DescribeDecorationCommand.DESCRIPTIONS[this.descriptor];
    if (!text) {
      throw new Error(`Missing ASM decoration description for '${this.descriptor}'.`);
    }
    this.renderTextBox(text, eventManager, runner);
  }

  private describePoster(
    gameState: GameState,
    eventManager: EventManager,
    overworld: OverworldMap,
    runner: EventCommandRunner | undefined,
  ): boolean {
    const posterId = Number(gameState.wram.wDecoPoster ?? 0);
    const posterConstant = getDecorationConstant(posterId);
    if (!posterConstant) {
      return false;
    }
    if (posterConstant === "DECO_TOWN_MAP") {
      this.showTownMap(eventManager, overworld, runner);
      return true;
    }
    const label = DescribeDecorationCommand.POSTER_TEXT_LABELS[posterConstant];
    if (!label) {
      return false;
    }
    const message = resolveText(runner ?? null, overworld, label);
    this.renderTextBox(message, eventManager, runner);
    if (runner) {
      runner.last_value = {
        poster: {
          label,
          message,
          constant: posterConstant,
        },
      };
    }
    return true;
  }

  private renderTextBox(
    text: string,
    eventManager: EventManager,
    runner: EventCommandRunner | undefined,
  ): void {
    openText(eventManager);
    showText(eventManager, text);
    waitForInput(eventManager);
    closeText(eventManager);
    if (runner) {
      runner.last_condition_result = true;
    }
  }

  private showTownMap(
    eventManager: EventManager,
    overworld: OverworldMap,
    runner: EventCommandRunner | undefined,
  ): void {
  const message = resolveText(runner ?? null, overworld, "LookTownMapText");
    openText(eventManager);
    showText(eventManager, message);
    waitForInput(eventManager);
    eventManager.dispatch(new Event("show_town_map", { source: "TownMapScript", runner }));
    closeText(eventManager);
    if (runner) {
      runner.last_value = { town_map: { opened: true, message } };
      runner.last_condition_result = true;
    }
  }
}

export class CheckFlagCommand extends Command {
  constructor(
    private flagName: string,
    private scriptIfTrue?: string,
    private scriptIfFalse?: string,
  ) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldMap): void {
    const result = isFlagSet(gameState, this.flagName);
    const runner = this.runner as EventCommandRunner | undefined;
    if (runner) {
      runner.last_condition_result = result;
      if (this.scriptIfTrue && this.scriptIfFalse) {
        const target = result ? this.scriptIfTrue : this.scriptIfFalse;
        runner.run(target);
      }
    }
  }
}

export class ConditionalEventCommand extends Command {
  private readonly scriptName: string;

  constructor(private eventName: string, scriptName: string) {
    super();
    this.scriptName = normalizeScriptName(scriptName);
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldMap): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (!runner) {
      throw new Error("ConditionalEventCommand requires an active script runner.");
    }
    // conditional_event is attached to an object event whose event flag hides
    // that object. Its script is active while the flag is clear.
    const scriptActive = !isFlagSet(gameState, this.eventName);
    runner.last_condition_result = scriptActive;
    runner.last_value = scriptActive;
    LOGGER.debug(
      "ConditionalEventCommand %s -> %s (target %s)",
      this.eventName,
      scriptActive,
      this.scriptName,
    );

    const stack = runner._script_stack as ScriptFrame[] | undefined;
    if (stack && stack.length) {
      stack[stack.length - 1].allowFallthrough = false;
    }

    if (!scriptActive) {
      return;
    }
    if (this.scriptName.startsWith(".")) {
      runner.jump?.(this.scriptName);
    } else {
      runner.defer?.(this.scriptName);
    }
  }
}

export class SetEventCommand extends Command {
  constructor(private eventName: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldMap): void {
    let targetOverworld: OverworldMap | OverworldEngine | null = overworld;
    if (!targetOverworld) {
      const runner = this.runner as EventCommandRunner | undefined;
      targetOverworld = runner?.overworld ?? null;
    }
    applyEventFlag(gameState, this.eventName, { value: true, overworld: targetOverworld });
    LOGGER.debug("SetEventCommand set %s", this.eventName);
  }
}

export class CheckEventCommand extends Command {
  constructor(private eventName: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldMap): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (runner) {
      runner.last_condition_result = isFlagSet(gameState, this.eventName);
      LOGGER.debug("CheckEventCommand %s -> %s", this.eventName, runner.last_condition_result);
    }
  }
}

export class SpecialPhoneCallCommand extends Command {
  constructor(private callId: string) {
    super();
    this.callId = this.callId.trim();
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldMap): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (!runner) {
      throw new Error("SpecialPhoneCallCommand requires an active script runner.");
    }
    const callId = this.callId.trim();
    if (!callId) {
      return;
    }
    const queue = gameState.wram.scheduled_phone_calls;
    const normalized = callId.toUpperCase();
    if (normalized === "SPECIALCALL_NONE") {
      removeSpecialCallsFromQueue(queue);
      runner.last_condition_result = false;
      runner.last_value = { special_phone_call: null, queue: [...queue] };
      return;
    }

    removeSpecialCallsFromQueue(queue);
    if (typeof runner.queue_phone_call === "function") {
      runner.queue_phone_call(normalized);
    } else if (typeof runner.queuePhoneCall === "function") {
      runner.queuePhoneCall(normalized);
    } else if (!queue.includes(normalized)) {
      queue.push(normalized);
    }
    runner.last_condition_result = true;
    runner.last_value = { special_phone_call: normalized, queue: [...queue] };
  }
}

export class IfTrueCommand extends Command {
  private readonly scriptName: string;

  constructor(scriptName: string) {
    super();
    this.scriptName = normalizeScriptName(scriptName);
  }

  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldMap): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (!runner) {
      throw new Error("IfTrueCommand requires an active script runner.");
    }
    LOGGER.debug("IfTrueCommand evaluating %s -> %s", this.scriptName, runner.last_condition_result);
    pushDebugLog(
      `[script] iftrue ${this.scriptName} -> ${runner.last_condition_result ? "jump" : "skip"}`,
    );
    if (runner.last_condition_result) {
      runner.jump?.(this.scriptName);
    }
  }
}

export class IfFalseCommand extends Command {
  private readonly scriptName: string;

  constructor(scriptName: string) {
    super();
    this.scriptName = normalizeScriptName(scriptName);
  }

  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldMap): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (!runner) {
      throw new Error("IfFalseCommand requires an active script runner.");
    }
    LOGGER.debug("IfFalseCommand evaluating %s -> %s", this.scriptName, !runner.last_condition_result);
    pushDebugLog(
      `[script] iffalse ${this.scriptName} -> ${runner.last_condition_result ? "skip" : "jump"}`,
    );
    if (!runner.last_condition_result) {
      runner.jump?.(this.scriptName);
    }
  }
}

export class SetSceneCommand extends Command {
  constructor(private sceneName: string) {
    super();
    this.sceneName = this.sceneName.replace(/,+$/, "");
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldMap): void {
    // ASM: engine/overworld/scripting.asm::Script_setscene
    const runner = this.runner as EventCommandRunner | undefined;
    const activeOverworld = overworld ?? runner?.overworld ?? null;
    if (!runner || !activeOverworld) {
      throw new Error("SetSceneCommand requires overworld and runner context.");
    }
    const mapName = resolveOverworldMapName(activeOverworld);
    const resolvedMapName = normalizeMapName(mapName);
    if (!resolvedMapName) {
      throw new Error("SetSceneCommand cannot resolve the current map name.");
    }
    ensureMapSceneInitialized(runner, gameState, resolvedMapName);
    const previous = gameState.wram.map_scenes[resolvedMapName] ?? "";
    LOGGER.debug(
      "SetSceneCommand moving map %s scene %s -> %s",
      resolvedMapName,
      previous || "<unset>",
      this.sceneName,
    );
    if (typeof runner._set_map_scene === "function") {
      runner._set_map_scene(resolvedMapName, this.sceneName);
    } else {
      gameState.wram.map_scenes[resolvedMapName] = this.sceneName;
    }
    const sceneIndex = gameState.wram.map_scene_indices[resolvedMapName] ?? 0;
    pushDebugLog(`[script] setscene ${resolvedMapName} -> ${this.sceneName} (${sceneIndex})`);
  }
}

export class ClearEventCommand extends Command {
  constructor(private eventName: string) {
    super();
    this.eventName = this.eventName.replace(/,+$/, "");
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldMap): void {
    let targetOverworld: OverworldMap | OverworldEngine | null = overworld;
    if (!targetOverworld) {
      const runner = this.runner as EventCommandRunner | undefined;
      targetOverworld = runner?.overworld ?? null;
    }
    clearEventFlag(gameState, this.eventName, { overworld: targetOverworld });
  }
}

export class BlackoutModCommand extends Command {
  constructor(private mapConstant: string) {
    super();
    this.mapConstant = this.mapConstant.replace(/,+$/, "");
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldMap): void {
    const mapConstant = this.mapConstant.toUpperCase();
    const metadata = getMapMetadataByConstant(mapConstant);
    if (!metadata) {
      throw new Error(`Unknown map constant '${this.mapConstant}' for blackoutmod command.`);
    }

    gameState.sram.last_spawn_map_group = metadata.groupId;
    gameState.sram.last_spawn_map_number = metadata.mapId;
    gameState.wram.wLastSpawnMapGroup = metadata.groupId;
    gameState.wram.wLastSpawnMapNumber = metadata.mapId;

    const runner = this.runner as EventCommandRunner | undefined;
    if (runner) {
      runner.last_value = {
        respawn_map: mapConstantToName(mapConstant),
        group: metadata.groupId,
        map_id: metadata.mapId,
      };
    }
  }
}

export class SetLastTalkedCommand extends Command {
  constructor(private objectId: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldMap): void {
    if (!overworld) {
      throw new Error("SetLastTalkedCommand requires overworld context.");
    }
    const normalized = this.objectId.toUpperCase();
    if (normalized === "LAST_TALKED") {
      return;
    }
    if (normalized === "PLAYER") {
      gameState.wram.last_talked = 0;
      LOGGER.debug("Set last talked: PLAYER -> 0");
      return;
    }

    const parseNumeric = (token: string): number | null => {
      const trimmed = token.trim();
      if (!trimmed) {
        return null;
      }
      let base = 10;
      let value = trimmed;
      if (value.startsWith("$")) {
        base = 16;
        value = value.slice(1);
      } else if (value.toLowerCase().startsWith("0x")) {
        base = 16;
      }
      const parsed = Number.parseInt(value, base);
      return Number.isNaN(parsed) ? null : parsed;
    };

    const parsed = parseNumeric(this.objectId);
    if (parsed !== null) {
      const index = parsed;
      if (index === -2 || index === 0) {
        gameState.wram.last_talked = 0;
        LOGGER.debug("Set last talked: %s -> 0", this.objectId);
        return;
      }
      gameState.wram.last_talked = index;
      LOGGER.debug("Set last talked: %s -> %d", this.objectId, index);
      return;
    }
    const resolverHost = overworld as OverworldMap & OverworldObjectResolver;
    let index: number | null = null;
    if (typeof resolverHost.resolve_object_index === "function") {
      index = resolverHost.resolve_object_index(this.objectId);
    } else if (typeof resolverHost.resolveObjectIndex === "function") {
      index = resolverHost.resolveObjectIndex(this.objectId);
    }
    if (index === null || index === undefined) {
      throw new Error(`Unable to resolve object '${this.objectId}' for setlasttalked`);
    }
    gameState.wram.last_talked = index;
    LOGGER.debug("Set last talked: %s -> %d", this.objectId, index);
  }
}

export class EndIfJustBattledCommand extends Command {
  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldMap): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (!runner) {
      return;
    }

    if ((gameState?.wram?.wRunningTrainerBattleScript ?? 0) !== 0) {
      // ASM: engine/overworld/scripting.asm::Script_endifjustbattled.
      gameState.wram.wRunningTrainerBattleScript = 0;
      runner.just_battled = false;
      runner.stop_execution = true;
      return;
    }

    if (runner.just_battled) {
      runner.just_battled = false;
      runner.stop_execution = true;
    }
  }
}

export class IfEqualCommand extends Command {
  private readonly scriptName: string;

  constructor(private value: string, scriptName: string) {
    super();
    this.scriptName = normalizeScriptName(scriptName);
  }

  public execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (runner && compareScriptValue(runner.last_value, this.value, runner, overworld)) {
      runner.jump?.(this.scriptName);
    }
  }
}

export class IfNotEqualCommand extends Command {
  private readonly scriptName: string;

  constructor(private value: string, scriptName: string) {
    super();
    this.scriptName = normalizeScriptName(scriptName);
  }

  public execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (runner && !compareScriptValue(runner.last_value, this.value, runner, overworld)) {
      runner.jump?.(this.scriptName);
    }
  }
}

const resolveEqualityValue = (
  token: string,
  runner: EventCommandRunner | undefined,
  overworld: OverworldContext | null,
): number | string => {
  const trimmed = String(token ?? "").trim().replace(/,+$/, "");
  try {
    return resolveComparisonValue(trimmed, runner, overworld);
  } catch (error) {
    if (/^NUM_[A-Z0-9_]+$/.test(trimmed)) {
      throw error;
    }
    return trimmed;
  }
};

const compareScriptValue = (
  actual: unknown,
  expectedToken: string,
  runner: EventCommandRunner | undefined,
  overworld: OverworldContext | null,
): boolean => {
  const expected = resolveEqualityValue(expectedToken, runner, overworld);
  if (typeof expected === "number") {
    return Number(actual ?? 0) === expected;
  }
  return String(actual) === expected;
};

const resolveComparisonValue = (
  token: string,
  runner: EventCommandRunner | undefined,
  overworld: OverworldContext | null,
): number => {
  const mapName = overworld?.current_map_name ?? runner?.overworld?.current_map_name ?? null;
  return resolveScriptConstantExpression(String(token ?? "").trim().replace(/,+$/, ""), mapName ?? undefined);
};

export class IfGreaterCommand extends Command {
  private readonly scriptName: string;

  constructor(private value: string, scriptName: string) {
    super();
    this.scriptName = normalizeScriptName(scriptName);
  }

  public execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (!runner) {
      throw new Error("IfGreaterCommand requires an active script runner.");
    }
    if (Number(runner.last_value ?? 0) > resolveComparisonValue(this.value, runner, overworld)) {
      runner.jump?.(this.scriptName);
    }
  }
}

export class IfLessCommand extends Command {
  private readonly scriptName: string;

  constructor(private value: string, scriptName: string) {
    super();
    this.scriptName = normalizeScriptName(scriptName);
  }

  public execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (!runner) {
      throw new Error("IfLessCommand requires an active script runner.");
    }
    if (Number(runner.last_value ?? 0) < resolveComparisonValue(this.value, runner, overworld)) {
      runner.jump?.(this.scriptName);
    }
  }
}

export class RandomCommand extends Command {
  constructor(private readonly upperBound: number) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    if (this.upperBound <= 0) {
      throw new Error(`random upper bound must be positive, got ${this.upperBound}.`);
    }
    const runner = this.runner as EventCommandRunner | undefined;
    const value = new HardwareRNG(gameState).randrange(this.upperBound);
    gameState.wram.script_memory["wScriptVar"] = value;
    if (runner) {
      runner.last_value = value;
      runner.last_condition_result = value !== 0;
    }
  }
}

export class CheckVersionCommand extends Command {
  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (runner) {
      // ASM checkver is only used by Bill's grandfather to pick the alternate
      // species request. This runtime targets Crystal's content pack, whose
      // exported script keeps the non-alternate path as the default branch.
      runner.last_value = 0;
      runner.last_condition_result = false;
    }
  }
}

export class CallAsmCommand extends Command {
  constructor(private readonly label: string) {
    super();
  }

  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (runner) {
      runner.last_value = this.label;
      runner.last_condition_result = true;
    }
  }
}

export class JumpStandardCommand extends Command {
  constructor(private scriptName: string) {
    super();
  }

  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldMap): void {
    const runner = this.runner as EventCommandRunner | undefined;
    if (!runner) {
      throw new Error("JumpStandardCommand requires an active script runner.");
    }
    if (typeof runner.run_standard_script !== "function") {
      throw new Error("JumpStandardCommand requires standard-script dispatch support.");
    }
    runner.run_standard_script(this.scriptName);
    runner.stop_execution = true;
  }
}

import { GameState } from "@pokecrystal/core/core/state";
import { getFilledSlots } from "@pokecrystal/core/core/models/party";
import { MAIL_MSG_LENGTH } from "@pokecrystal/core/core/constants";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { MailMessageSchema } from "@pokecrystal/core/core/mail";
import { type DataLoader, type ScriptData } from "@pokecrystal/core/core/data-loader";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import { canonicaliseTimeOfDay } from "@pokecrystal/core/engine/systems/time";
import { getMapMetadataByConstant, getMapMetadataByGroup } from "@pokecrystal/core/engine/world/maps";
import { METATILE_WIDTH } from "@pokecrystal/core/engine/world/tile/constants";
import { sampleCollision } from "@pokecrystal/core/engine/world/overworld/collision-rules";
import { scaleTileCoord, scaleTileCoords } from "@pokecrystal/core/engine/world/overworld/tile-coords";
import { resolveSwarmDefinition } from "@pokecrystal/core/engine/world/overworld/swarm";
import { playOverworldSound } from "@pokecrystal/core/engine/world/overworld/audio-guards";
import { warpSoundForPermission } from "@pokecrystal/core/engine/world/overworld/tile-events";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { pushDebugLog } from "@pokecrystal/core/core/debug-log";
import { OverworldObject } from "@pokecrystal/core/engine/world/overworld/overworld-object";
import { PlayerCharacter } from "@pokecrystal/core/engine/world/overworld/playable-character";
import { Command } from "./base";
import { extractStringFromScript } from "./text";
import { LOGGER } from "../common";

// ASM: engine/overworld/scripting.asm::Script_follow, Script_stopfollow, Script_lock,
// Script_release, Script_lockall, Script_releaseall, Script_stop, Script_playsound,
// Script_waitsfx, Script_pause, Script_earthquake, Script_cry, Script_pokepic,
// Script_closepokepic, Script_getmonname, Script_appear, Script_disappear,
// Script_checkpokemail, Script_checktime, Script_moveobject, Script_changeblock,
// Script_refreshmap, Script_warp, Script_warpfacing, Script_warpcheck, Script_warpsound,
// Script_newloadmap, Script_swarm, Script_elevator, Script_turnobject,
// Script_showemote, Script_applymovement, Script_faceplayer.

type ScriptDataEntry = {
  command?: unknown;
  args?: unknown;
};

type AudioEngineLike = {
  play_sound?: (soundId: string) => void;
  playSound?: (soundId: string) => void;
  isSoundPlaying?: (soundId?: string) => boolean;
};

type DataLoaderLike = {
  get_script?: (label: string, parentScript?: string) => ScriptDataEntry[] | null;
  getScript?: (label: string, parentScript?: string) => ScriptDataEntry[] | null;
  get_pokemon?: (speciesName: string) => { name?: string | null } | null;
  getPokemon?: (speciesName: string) => { name?: string | null } | null;
  get_pokemon_species?: (speciesName: string) => { name?: string | null } | null;
  getPokemonSpecies?: (speciesName: string) => { name?: string | null } | null;
  get_pokemon_cry?: (speciesName: string) => { cry_id?: string | null } | null;
  getPokemonCry?: (speciesName: string) => { cry_id?: string | null } | null;
  map_events?: { get: (name: string) => any };
  ensure_map_scripts?: (name: string) => void;
};

type PokePicOverlayLike = {
  show: (speciesName: string) => void;
  hide: () => void;
};

type OverworldObjectLike = OverworldObject | PlayerCharacter | null;

type OverworldScriptingContext = {
  data_loader?: DataLoaderLike | null;
  dataLoader?: DataLoaderLike | null;
  audio_engine?: AudioEngineLike | null;
  audioEngine?: AudioEngineLike | null;
  pokepic_overlay?: PokePicOverlayLike | null;
  pokepicOverlay?: PokePicOverlayLike | null;
  get_object_by_id?: (objectId: string | number | null) => OverworldObjectLike;
  getObjectById?: (objectId: string | number | null) => OverworldObjectLike;
  start_earthquake?: (intensity: number, duration: number) => void;
  startEarthquake?: (intensity: number, duration: number) => void;
  appear_object?: (objectId: string | number, options?: unknown) => void;
  appearObject?: (objectId: string | number, options?: unknown) => void;
  remove_object?: (objectId: string) => void;
  removeObject?: (objectId: string) => void;
  move_object?: (objectId: string, x: string | number, y: string | number) => void;
  moveObject?: (objectId: string, x: string | number, y: string | number) => void;
  resolve_object_index?: (identifier: string) => number | null;
  resolveObjectIndex?: (identifier: string) => number | null;
  show_emote?: (emoteId: string, obj: OverworldObjectLike, duration: number) => void;
  showEmote?: (emoteId: string, obj: OverworldObjectLike, duration: number) => void;
  wait_sfx?: (callback: () => void) => void;
  waitSFX?: (callback: () => void) => void;
  get_movement_data?: (label: string, parentScript?: string | null) => Iterable<string> | null;
  getMovementData?: (label: string, parentScript?: string | null) => Iterable<string> | null;
  queue_movement_task?: (obj: OverworldObjectLike, movement: Iterable<string>, options?: { onComplete?: (() => void) | null }) => void;
  queueMovementTask?: (obj: OverworldObjectLike, movement: Iterable<string>, options?: { onComplete?: (() => void) | null }) => void;
  start_elevator_ride?: (origin: string, destination: string, options: Record<string, unknown>) => void;
  startElevatorRide?: (origin: string, destination: string, options: Record<string, unknown>) => void;
  _write_metatile?: (metatile_x: number, metatile_y: number, block_id: number) => void;
  _refresh_warp_permissions?: () => void;
  refresh_map_sprites?: (options?: { reload_standing?: boolean; reload_walking?: boolean }) => void;
  refreshMapSprites?: (options?: { reload_standing?: boolean; reload_walking?: boolean }) => void;
  map?: unknown;
  tileset?: unknown;
  current_map_name?: string;
  player_direction?: string;
  player_object?: OverworldObjectLike;
  _current_tile_permission?: () => number | null;
  check_for_warp_event?: (options?: {
    allow_script?: boolean;
    allowScript?: boolean;
    ignore_cooldown?: boolean;
    ignoreCooldown?: boolean;
  }) => boolean;
  checkForWarpEvent?: (options?: {
    allow_script?: boolean;
    allowScript?: boolean;
    ignore_cooldown?: boolean;
    ignoreCooldown?: boolean;
  }) => boolean;
  reload_current_map?: () => void;
  reloadCurrentMap?: () => void;
  _logger?: { debug?: (message: string, ...args: unknown[]) => void };

  leader?: OverworldObjectLike;
  follower?: OverworldObjectLike;
  player_x?: number;
  player_y?: number;
  prev_player_x?: number;
  prev_player_y?: number;
  target_tile_x?: number;
  target_tile_y?: number;
  is_moving?: boolean;
  step_progress_px?: number;
  step_dx_px?: number;
  step_dy_px?: number;
  _queued_direction?: unknown;
  _active_warp_tile?: [string, number, number];
  _warp_cooldown?: number;
  WALK_FRAMES?: number;
  TILES_PER_COLLISION?: number;
  load_map?: (mapName: string) => void;
  loadMap?: (mapName: string) => void;
  _sync_player_state?: () => void;
  clear_pending_white_fade?: () => void;

  stop_following?: () => void;
  stopFollowing?: () => void;
  lock_player_movement?: () => void;
  lockPlayerMovement?: () => void;
  unlock_player_movement?: () => void;
  unlockPlayerMovement?: () => void;
  lock_all_movement?: () => void;
  lockAllMovement?: () => void;
  unlock_all_movement?: () => void;
  unlockAllMovement?: () => void;
  stop_player_movement?: () => void;
  stopPlayerMovement?: () => void;

  queue_follow?: (follower: unknown, leader: unknown, options?: Record<string, unknown>) => void;
  queueFollow?: (follower: unknown, leader: unknown, options?: Record<string, unknown>) => void;
  queue_follow_task?: (follower: unknown, leader: unknown, options?: Record<string, unknown>) => void;
  queueFollowTask?: (follower: unknown, leader: unknown, options?: Record<string, unknown>) => void;
  start_following?: (follower: unknown, leader: unknown, options?: Record<string, unknown>) => void;
  startFollowing?: (follower: unknown, leader: unknown, options?: Record<string, unknown>) => void;
};

type ScriptRunnerLike = {
  pause: () => void;
  resume: () => void;
  last_sound_effect?: string | null;
  last_condition_result?: boolean;
  last_value?: unknown;
  variables?: Record<string, unknown> | Map<string, unknown>;
  _queue_overworld_task?: (task: (callback: () => void) => boolean) => void;
  _queueOverworldTask?: (task: (callback: () => void) => boolean) => void;
  _consume_script_choice?: (key: string, defaultValue: number) => unknown;
  _consumeScriptChoice?: (key: string, defaultValue: number) => unknown;
  _find_parent_script_name?: () => string | null;
  _script_stack?: Array<{ name?: string; index?: number; parent?: string }>;
  dataLoader?: DataLoaderLike | null;
  data_loader?: DataLoaderLike | null;
  stop_all_scripts?: () => void;
  string_buffers?: Record<string, string>;
};

type ElevatorFloor = {
  label: string;
  warpId: number;
  mapConstant: string;
};

type ElevatorChoice = string | number | null | undefined;

type LoadedScriptMenu = {
  label: string;
  options: string[];
};

type ElevatorSelectionPromptPayload = {
  key: "_elevator_selection";
  title: string;
  options: string[];
  initial_index: number;
  cancel_index: number;
  callback: (selection: number) => void;
};

const FLOOR_NAME_MAP: Record<string, string> = {
  FLOOR_B4F: "B4F",
  FLOOR_B3F: "B3F",
  FLOOR_B2F: "B2F",
  FLOOR_B1F: "B1F",
  FLOOR_1F: "1F",
  FLOOR_2F: "2F",
  FLOOR_3F: "3F",
  FLOOR_4F: "4F",
  FLOOR_5F: "5F",
  FLOOR_6F: "6F",
  FLOOR_7F: "7F",
  FLOOR_8F: "8F",
  FLOOR_9F: "9F",
  FLOOR_10F: "10F",
  FLOOR_11F: "11F",
  FLOOR_ROOF: "ROOF",
};

// ASM: constants/map_setup_constants.asm (hMapEntryMethod values).
const MAPSETUP_CONSTANTS: Record<string, number> = {
  MAPSETUP_WARP: 0xf1,
  MAPSETUP_CONTINUE: 0xf2,
  MAPSETUP_RELOADMAP: 0xf3,
  MAPSETUP_TELEPORT: 0xf4,
  MAPSETUP_DOOR: 0xf5,
  MAPSETUP_FALL: 0xf6,
  MAPSETUP_CONNECTION: 0xf7,
  MAPSETUP_LINKRETURN: 0xf8,
  MAPSETUP_TRAIN: 0xf9,
  MAPSETUP_SUBMENU: 0xfa,
  MAPSETUP_BADWARP: 0xfb,
  MAPSETUP_FLY: 0xfc,
};
const SCRIPT_WAIT_POLL_MS = GB_FRAME_DURATION_MS;
// ASM: Script_pause decrements once per loop after `DelayFrames` with c=2.
const SCRIPT_PAUSE_DELAY_FRAMES = 2;

const getAudioEngine = (overworld: unknown): AudioEngineLike | null => {
  const overworldAny = overworld as OverworldScriptingContext;
  return overworldAny?.audio_engine ?? overworldAny?.audioEngine ?? null;
};

const normalizeFacingDirection = (direction: string): string => {
  const normalized = String(direction ?? "").trim().toLowerCase();
  if (!["up", "down", "left", "right"].includes(normalized)) {
    throw new Error(`Invalid facing direction '${direction}'.`);
  }
  return normalized;
};

const blocksPerMetatile = (): number => {
  const stride = METATILE_WIDTH / 2;
  if (!Number.isFinite(stride) || stride <= 0 || !Number.isInteger(stride)) {
    throw new Error(`METATILE_WIDTH must be an even positive integer, got ${METATILE_WIDTH}.`);
  }
  return stride;
};

// ASM: Script_changeblock block coords are in 2x2 tiles; engine metatiles are METATILE_WIDTH.
const resolveChangeBlockCoords = (blockX: number, blockY: number): [number, number] => {
  const stride = blocksPerMetatile();
  return [Math.trunc(blockX / stride), Math.trunc(blockY / stride)];
};

const parseElevatorInt = (token: string): number => {
  let normalized = String(token ?? "").trim();
  if (!normalized) {
    return 0;
  }
  let base = 10;
  if (normalized.startsWith("$")) {
    base = 16;
    normalized = normalized.slice(1);
  } else if (normalized.toLowerCase().startsWith("0x")) {
    base = 16;
    normalized = normalized.slice(2);
  }
  const value = parseInt(normalized, base);
  if (Number.isNaN(value)) {
    throw new Error(`Invalid elevator token '${token}'.`);
  }
  return value;
};

const parseMapSetupToken = (token: string): number => {
  const normalized = String(token ?? "").trim();
  if (!normalized) {
    throw new Error("Map setup token may not be empty.");
  }
  const upper = normalized.toUpperCase();
  const mapped = MAPSETUP_CONSTANTS[upper];
  if (mapped !== undefined) {
    return mapped;
  }
  return parseElevatorInt(upper);
};

const parseElevatorData = (dataLoader: unknown, dataLabel: string, parentScript?: string | null): ElevatorFloor[] => {
  const loader = dataLoader as DataLoaderLike | null;
  const scriptData =
    loader?.get_script?.(dataLabel, parentScript ?? undefined)
    ?? loader?.getScript?.(dataLabel, parentScript ?? undefined);
  if (!scriptData) {
    return [];
  }

  const floors: ElevatorFloor[] = [];
  for (const entry of scriptData) {
    if (!entry || typeof entry !== "object") {
      continue;
    }
    const entryData = entry as ScriptDataEntry;
    const command = String(entryData.command ?? "").trim().toLowerCase();
    const args = Array.isArray(entryData.args) ? entryData.args : [];
    if (command === "db") {
      if (Array.isArray(args) && args.length && String(args[0]).trim() === "-1") {
        break;
      }
      continue;
    }
    if (command !== "elevfloor") {
      continue;
    }
    if (!Array.isArray(args) || args.length < 3) {
      continue;
    }
    const floorLabel = String(args[0]).trim();
    const warpId = parseElevatorInt(String(args[1]));
    const mapConstant = String(args[2]).trim();
    floors.push({ label: floorLabel, warpId, mapConstant });
  }
  return floors;
};

const findOriginFloorIndex = (floors: ElevatorFloor[], gameState: GameState): number | null => {
  for (let index = 0; index < floors.length; index += 1) {
    const floor = floors[index];
    const metadata = getMapMetadataByConstant(floor.mapConstant);
    if (!metadata) {
      continue;
    }
    if (metadata.groupId === gameState.wram.wBackupMapGroup &&
        metadata.mapId === gameState.wram.wBackupMapNumber) {
      return index;
    }
  }
  return null;
};

const coerceFloorSelection = (
  choice: ElevatorChoice,
  floors: ElevatorFloor[],
  fallback: number,
): number => {
  if (typeof choice === "string") {
    const normalized = choice.trim().toUpperCase();
    for (let index = 0; index < floors.length; index += 1) {
      if (floors[index].label.toUpperCase() === normalized) {
        return index;
      }
    }
    try {
      const parsed = parseElevatorInt(normalized);
      if (parsed >= 0 && parsed < floors.length) {
        return parsed;
      }
    } catch {
      // Ignore parsing errors and fall back.
    }
  } else {
    const parsed = Number(choice);
    if (Number.isFinite(parsed)) {
      const value = Math.trunc(parsed);
      if (value >= 0 && value < floors.length) {
        return value;
      }
    }
  }
  return fallback;
};

const formatFloorName = (label: string): string => {
  const normalized = label.trim().toUpperCase();
  const resolved = FLOOR_NAME_MAP[normalized] ?? "";
  if (!resolved) {
    throw new Error(`Missing ASM floor label '${label}'.`);
  }
  return resolved;
};

const POKEMAIL_WRONG_MAIL = 0;
const POKEMAIL_CORRECT = 1;
const POKEMAIL_REFUSED = 2;
const POKEMAIL_NO_MAIL = 3;
const POKEMAIL_LAST_MON = 4;

const normalizeMailItem = (itemName: string | null | undefined): string =>
  String(itemName ?? "")
    .trim()
    .replace(/,$/, "")
    .toUpperCase();

const isMailItem = (itemName: string | null | undefined): itemName is string => {
  const normalized = normalizeMailItem(itemName);
  if (!normalized) {
    return false;
  }
  const parsed = MailMessageSchema.safeParse({
    message: "",
    author: "PLAYER",
    author_id: 0,
    species_id: 0,
    mail_type: normalized,
  });
  return parsed.success;
};

const normalizeMailPayload = (value: string | null | undefined): string =>
  String(value ?? "").slice(0, MAIL_MSG_LENGTH);

const compareMailPayload = (expected: string, actual: string): boolean => {
  const expectedValue = normalizeMailPayload(expected);
  const actualValue = normalizeMailPayload(actual);
  if (expectedValue.length === 0) {
    return true;
  }
  return actualValue.startsWith(expectedValue);
};

const resolveRunnerVariable = (runner: ScriptRunnerLike | undefined, key: string): unknown => {
  const variables = runner?.variables;
  if (!variables) {
    return undefined;
  }
  if (variables instanceof Map) {
    return variables.get(key);
  }
  if (Object.prototype.hasOwnProperty.call(variables, key)) {
    return variables[key];
  }
  return undefined;
};

const resolvePartySelection = (gameState: GameState, runner: ScriptRunnerLike | undefined): number | null => {
  const partySize = resolvePartySize(gameState);
  if (partySize <= 0) {
    return null;
  }

  const selectedParty = resolveRunnerVariable(runner, "_selected_party_index");
  const selectedNumber = typeof selectedParty === "number"
    ? selectedParty
    : typeof selectedParty === "string"
      ? Number.parseInt(selectedParty, 10)
      : undefined;

  if (selectedNumber !== undefined && Number.isInteger(selectedNumber) && selectedNumber >= 0 && selectedNumber < partySize) {
    return selectedNumber;
  }

  const cursorIndex = Number(gameState.wram.wCurPartyMon);
  if (Number.isInteger(cursorIndex) && cursorIndex >= 0 && cursorIndex < partySize) {
    return cursorIndex;
  }

  return null;
};

const hasOtherConsciousPartyMon = (
  gameState: GameState,
  selection: number,
  partySize: number,
): boolean => {
  const party = gameState.sram.party.pokemon;
  for (let index = 0; index < partySize; index += 1) {
    if (index === selection) {
      continue;
    }
    const mon = party[index];
    if (mon && (mon.hp ?? 0) > 0) {
      return true;
    }
  }
  return false;
};

const resolvePartySize = (gameState: GameState): number => {
  const party = gameState.sram.party.pokemon;
  const filledSlots = getFilledSlots(gameState.sram.party);
  const declared = Number(gameState.wram.wPartyCount);
  if (!Number.isFinite(declared) || declared <= 0) {
    if (gameState.wram.wPartyCount !== filledSlots) {
      gameState.wram.wPartyCount = filledSlots;
    }
    return filledSlots;
  }
  const count = Math.max(0, Math.min(party.length, declared));
  if (count <= filledSlots) {
    if (count !== gameState.wram.wPartyCount) {
      gameState.wram.wPartyCount = count;
    }
    return count;
  }
  gameState.wram.wPartyCount = filledSlots;
  return filledSlots;
};

const resolveMailLabelScriptData = (
  runner: ScriptRunnerLike | undefined,
  label: string,
): ScriptData | null => {
  const loader = runner?.dataLoader ?? runner?.data_loader;
  if (!loader) {
    return null;
  }
  const parent = typeof (runner as { _find_parent_script_name?: () => string | null })._find_parent_script_name === "function"
    ? (runner as { _find_parent_script_name?: () => string | null })._find_parent_script_name?.() ?? null
    : null;
  return (
    loader.get_script?.(label, parent ?? undefined) as ScriptData
    ?? loader.getScript?.(label, parent ?? undefined) as ScriptData
    ?? null
  );
};

const extractMailMessage = (runner: ScriptRunnerLike | undefined, label: string): string => {
  const scriptData = resolveMailLabelScriptData(runner, label);
  if (!scriptData) {
    throw new Error(`Could not load mail definition '${label}'.`);
  }
  const message = extractStringFromScript(scriptData);
  return message ?? "";
};

const resolveMailDefinition = (runner: ScriptRunnerLike | undefined, label: string): {
  item: string;
  message: string;
} => {
  const scriptData = resolveMailLabelScriptData(runner, label);
  if (!scriptData || !scriptData.length) {
    throw new Error(`Could not load mail definition '${label}'.`);
  }
  const firstEntry = scriptData[0] as ScriptDataEntry;
  if (!firstEntry || typeof firstEntry !== "object") {
    throw new Error(`Invalid mail definition '${label}': no data entries.`);
  }
  const command = String(firstEntry.command ?? "").trim().toLowerCase();
  if (command !== "db") {
    throw new Error(`Invalid mail definition '${label}': expected db item declaration first.`);
  }
  const args = Array.isArray(firstEntry.args) ? firstEntry.args : [];
  const item = normalizeMailItem(args[0]);
  if (!isMailItem(item)) {
    throw new Error(`Invalid mail item '${item}' in definition '${label}'.`);
  }
  return {
    item,
    message: extractStringFromScript(scriptData) ?? "",
  };
};

export class FollowCommand extends Command {
  constructor(private readonly leaderId: string, private readonly followerId: string) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      return;
    }
    const follower = overworld.get_object_by_id(this.followerId);
    const leader = overworld.get_object_by_id(this.leaderId);
    const runner = this.runner as ScriptRunnerLike | undefined;
    if (!follower || !leader) {
      if (runner) {
        runner.last_condition_result = false;
      }
      return;
    }

    const overworldAny = overworld as OverworldScriptingContext;
    const startFollow = overworldAny.start_following ?? overworldAny.startFollowing;
    if (typeof startFollow !== "function") {
      throw new Error("Overworld implementation missing start_following().");
    }

    // ASM: Script_follow calls StartFollow immediately without pausing script execution.
    startFollow.call(overworldAny, follower, leader, {
      follower_id: this.followerId,
      leader_id: this.leaderId,
      followerId: this.followerId,
      leaderId: this.leaderId,
    });
    if (runner) {
      runner.last_condition_result = true;
    }
  }
}

export class StopFollowCommand extends Command {
  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      return;
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const stopFollow = overworldAny.stop_following ?? overworldAny.stopFollowing;
    if (typeof stopFollow === "function") {
      stopFollow.call(overworldAny);
    }
  }
}

export class LockCommand extends Command {
  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      return;
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const lock = overworldAny.lock_player_movement ?? overworldAny.lockPlayerMovement;
    if (typeof lock === "function") {
      lock.call(overworldAny);
    }
  }
}

export class ReleaseCommand extends Command {
  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      return;
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const unlock = overworldAny.unlock_player_movement ?? overworldAny.unlockPlayerMovement;
    if (typeof unlock === "function") {
      unlock.call(overworldAny);
    }
  }
}

export class LockAllCommand extends Command {
  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      return;
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const lockAll = overworldAny.lock_all_movement ?? overworldAny.lockAllMovement;
    const lockPlayer = overworldAny.lock_player_movement ?? overworldAny.lockPlayerMovement;
    if (typeof lockAll === "function") {
      lockAll.call(overworldAny);
    } else if (typeof lockPlayer === "function") {
      lockPlayer.call(overworldAny);
    }
  }
}

export class ReleaseAllCommand extends Command {
  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      return;
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const unlockAll = overworldAny.unlock_all_movement ?? overworldAny.unlockAllMovement;
    const unlockPlayer = overworldAny.unlock_player_movement ?? overworldAny.unlockPlayerMovement;
    if (typeof unlockAll === "function") {
      unlockAll.call(overworldAny);
    } else if (typeof unlockPlayer === "function") {
      unlockPlayer.call(overworldAny);
    }
  }
}

export class StopCommand extends Command {
  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      return;
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const stop = overworldAny.stop_player_movement ?? overworldAny.stopPlayerMovement;
    if (typeof stop === "function") {
      stop.call(overworldAny);
    }
  }
}

export class PlaySoundCommand extends Command {
  constructor(private readonly soundId: string) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    const runner = this.runner as ScriptRunnerLike | undefined;
    if (runner) {
      runner.last_sound_effect = this.soundId;
    }
    if (_gameState.wram?.instant_mode) {
      return;
    }
    const audioEngine = getAudioEngine(overworld);
    if (typeof audioEngine?.playSound === "function") {
      audioEngine.playSound(this.soundId);
      return;
    }
    audioEngine?.play_sound?.(this.soundId);
  }
}

export class WaitSFXCommand extends Command {
  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    const runner = this.runner as ScriptRunnerLike | undefined;
    if (runner) {
      if (_gameState.wram?.instant_mode) {
        runner.resume();
        return;
      }
      const overworldAny = overworld as OverworldScriptingContext;
      const wait_sfx = overworldAny.wait_sfx ?? overworldAny.waitSFX;
      const audioEngine = getAudioEngine(overworldAny);
      const isSoundPlaying = audioEngine?.isSoundPlaying;
      if (typeof wait_sfx === "function" && typeof runner.pause === "function") {
        runner.pause();
        wait_sfx.call(overworldAny, () => runner.resume());
        return;
      }
      if (typeof isSoundPlaying === "function" && typeof runner.pause === "function") {
        runner.pause();
        const poll = () => {
          if (isSoundPlaying.call(audioEngine)) {
            setTimeout(poll, SCRIPT_WAIT_POLL_MS);
            return;
          }
          runner.resume();
        };
        setTimeout(poll, SCRIPT_WAIT_POLL_MS);
        return;
      }
      throw new Error("WaitSFXCommand requires overworld wait_sfx()/waitSFX() or audioEngine.isSoundPlaying().");
    }
  }
}

export class PauseCommand extends Command {
  constructor(private readonly frames: number) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldEngine): void {
    const runner = this.runner as ScriptRunnerLike | undefined;
    if (runner) {
      if (_gameState.wram?.instant_mode) {
        runner.resume();
        return;
      }
      if (typeof runner.pause === "function") {
        runner.pause();
        setTimeout(() => runner.resume(), this.frames * SCRIPT_PAUSE_DELAY_FRAMES * GB_FRAME_DURATION_MS);
      } else {
        runner.resume();
      }
    }
  }
}

export class EarthquakeCommand extends Command {
  constructor(private readonly intensity: number, private readonly duration: number) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      return;
    }
    const intensity = Math.max(1, this.intensity);
    const duration = _gameState.wram.instant_mode ? 1 : this.duration;
    const overworldAny = overworld as OverworldScriptingContext;
    const start = overworldAny.start_earthquake ?? overworldAny.startEarthquake;
    if (typeof start === "function") {
      start.call(overworldAny, intensity, duration);
    }
  }
}

export class CryCommand extends Command {
  constructor(private readonly speciesName: string) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    const runner = this.runner as ScriptRunnerLike | undefined;
    const soundId = this.resolveSoundId(overworld, runner);
    if (runner) {
      runner.last_sound_effect = soundId;
    }
    if (_gameState.wram?.instant_mode) {
      return;
    }
    const audioEngine = getAudioEngine(overworld);
    const playCry = (overworld as OverworldEngine & { playCry?: (soundId: string) => void }).playCry;
    if (typeof playCry === "function") {
      playCry.call(overworld, soundId);
      return;
    }
    if (typeof audioEngine?.playSound === "function") {
      audioEngine.playSound(soundId);
      return;
    }
    audioEngine?.play_sound?.(soundId);
  }

  private resolveSoundId(overworld: unknown, runner?: ScriptRunnerLike): string {
    const overworldAny = overworld as OverworldScriptingContext;
    const dataLoader =
      overworldAny?.data_loader
      ?? overworldAny?.dataLoader
      ?? runner?.data_loader
      ?? runner?.dataLoader
      ?? null;
    const resolver = dataLoader?.get_pokemon_cry ?? dataLoader?.getPokemonCry;
    if (typeof resolver === "function") {
      const cry = resolver.call(dataLoader, this.speciesName);
      if (cry?.cry_id) {
        return cry.cry_id;
      }
    }
    return `CRY_${this.speciesName}`;
  }
}

export class PokePicCommand extends Command {
  constructor(private readonly speciesName: string) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    const overworldAny = overworld as OverworldScriptingContext;
    const overlay = overworldAny.pokepic_overlay ?? overworldAny.pokepicOverlay;
    if (!overlay) {
      throw new Error("PokePicCommand requires an overworld pokepic overlay.");
    }
    overlay.show(this.speciesName);
  }
}

export class ClosePokePicCommand extends Command {
  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    const overworldAny = overworld as OverworldScriptingContext;
    const overlay = overworldAny.pokepic_overlay ?? overworldAny.pokepicOverlay;
    if (!overlay) {
      throw new Error("ClosePokePicCommand requires an overworld pokepic overlay.");
    }
    overlay.hide();
  }
}

export class GetMonNameCommand extends Command {
  constructor(private readonly bufferName: string, private readonly speciesName: string) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    const runner = this.runner as ScriptRunnerLike | undefined;
    if (!runner) {
      return;
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const dataLoader =
      overworldAny?.data_loader ??
      overworldAny?.dataLoader ??
      runner?.data_loader ??
      runner?.dataLoader ??
      null;
    if (!dataLoader) {
      throw new Error(`Script_getmonname requires Pokemon data for '${this.speciesName}'.`);
    }
    const pokemon =
      dataLoader.get_pokemon?.(this.speciesName) ??
      dataLoader.get_pokemon_species?.(this.speciesName) ??
      (dataLoader as DataLoader & { getPokemonSpecies?: DataLoader["get_pokemon_species"] }).getPokemonSpecies?.(
        this.speciesName,
      );
    const rawName =
      pokemon && "name" in pokemon && typeof pokemon.name === "string"
        ? pokemon.name
        : pokemon && "id" in pokemon && typeof pokemon.id === "string"
          ? pokemon.id.replace(/_/g, " ")
          : "";
    const displayName = String(rawName).trim();
    if (!displayName) {
      throw new Error(`Missing ASM Pokemon name for '${this.speciesName}'.`);
    }
    if (!runner.string_buffers) {
      runner.string_buffers = {};
    }
    runner.string_buffers[this.bufferName] = displayName;
  }
}

export class AppearCommand extends Command {
  constructor(private readonly objectId: string) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      throw new Error("AppearCommand requires an active overworld.");
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const appear = overworldAny.appear_object ?? overworldAny.appearObject;
    if (typeof appear !== "function") {
      throw new Error("Overworld implementation missing appear_object().");
    }
    appear.call(overworldAny, this.objectId);
  }
}

export class DisappearCommand extends Command {
  constructor(private readonly objectId: string) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      throw new Error("DisappearCommand requires an active overworld.");
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const remove = overworldAny.remove_object ?? overworldAny.removeObject;
    if (typeof remove !== "function") {
      throw new Error("Overworld implementation missing remove_object().");
    }
    remove.call(overworldAny, this.objectId);
  }
}

export class CheckPokeMailCommand extends Command {
  constructor(private readonly messageLabel: string) {
    super();
  }

  execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldEngine): void {
    const runner = this.runner as ScriptRunnerLike | undefined;
    const party = gameState.sram.party.pokemon;
    const partySize = resolvePartySize(gameState);
    const selection = resolvePartySelection(gameState, runner);
    let result = POKEMAIL_REFUSED;

    if (selection !== null && selection < partySize) {
      gameState.wram.wCurPartyMon = selection;
      const mon = party[selection];
      if (!mon) {
        result = POKEMAIL_REFUSED;
      } else if (!isMailItem(mon.item)) {
        result = POKEMAIL_NO_MAIL;
      } else {
        const expected = extractMailMessage(runner, this.messageLabel);
        const actual = mon.mail?.message ?? "";
        if (compareMailPayload(expected, actual)) {
          if (!hasOtherConsciousPartyMon(gameState, selection, partySize)) {
            result = POKEMAIL_LAST_MON;
          } else {
            for (let index = selection; index < partySize - 1; index += 1) {
              party[index] = party[index + 1];
            }
            party[partySize - 1] = null;
            const newPartyCount = Math.max(0, partySize - 1);
            gameState.wram.wPartyCount = newPartyCount;
            if (gameState.wram.wCurPartyMon >= gameState.wram.wPartyCount) {
              gameState.wram.wCurPartyMon = gameState.wram.wPartyCount - 1;
            }
            result = POKEMAIL_CORRECT;
          }
        } else {
          result = POKEMAIL_WRONG_MAIL;
        }
      }
    }

    if (!gameState.wram.script_memory) {
      gameState.wram.script_memory = {};
    }
    gameState.wram.script_memory["wScriptVar"] = result;
    if (runner) {
      runner.last_value = result;
      runner.last_condition_result = result === POKEMAIL_CORRECT;
    }
  }
}

export class GivePokeMailCommand extends Command {
  constructor(private readonly mailLabel: string) {
    super();
  }

  execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldEngine): void {
    const party = gameState.sram.party.pokemon;
    const partySize = resolvePartySize(gameState);
    if (partySize <= 0) {
      throw new Error("GivePokeMailCommand requires a party Pokémon to target.");
    }
    const index = partySize - 1;
    const mon = party[index];
    if (!mon) {
      throw new Error("GivePokeMailCommand expected a valid Pokémon at the last party slot.");
    }

    const definition = resolveMailDefinition(this.runner as ScriptRunnerLike | undefined, this.mailLabel);
    const mail = MailMessageSchema.parse({
      message: normalizeMailPayload(definition.message),
      author: mon.original_trainer_name || "PLAYER",
      author_id: Number(mon.original_trainer_id ?? gameState.sram.player_id ?? 0),
      species_id: Number(mon.species?.int_id ?? 0),
      mail_type: definition.item,
    });

    mon.item = definition.item;
    mon.mail = mail;
  }
}

const _TIME_PERIOD_FLAGS: { [key: string]: number } = {
  morn: 1,
  day: 2,
  nite: 4,
};

const stripAsmMenuText = (token: string): string | null => {
  const trimmed = String(token ?? "").trim();
  if (!trimmed.includes("\"") && !trimmed.includes("@")) {
    return null;
  }
  let text = trimmed;
  const firstQuote = text.indexOf("\"");
  const lastQuote = text.lastIndexOf("\"");
  if (firstQuote >= 0 && lastQuote > firstQuote) {
    text = text.slice(firstQuote + 1, lastQuote);
  }
  text = text.replace(/@+$/g, "").trim();
  return text ? text : null;
};

const extractVerticalMenuOptions = (
  dataLoader: DataLoaderLike | null,
  label: string,
  parentScript: string | null,
): string[] => {
  const scriptData =
    dataLoader?.get_script?.(label, parentScript ?? undefined)
    ?? dataLoader?.getScript?.(label, parentScript ?? undefined)
    ?? null;
  if (!scriptData) {
    throw new Error(`Missing menu data '${label}'.`);
  }
  const options: string[] = [];
  for (const entry of scriptData) {
    const args = Array.isArray(entry?.args) ? entry.args : [];
    for (const arg of args) {
      const option = stripAsmMenuText(String(arg ?? ""));
      if (option) {
        options.push(option);
      }
    }
  }
  if (!options.length) {
    for (const entry of scriptData) {
      const command = String(entry?.command ?? "").trim().toLowerCase();
      if (command !== "dw") {
        continue;
      }
      const dataLabel = Array.isArray(entry?.args) ? String(entry.args[0] ?? "").trim() : "";
      if (!dataLabel) {
        continue;
      }
      const dataParent = label.startsWith(".") ? parentScript : label;
      const dataScript =
        dataLoader?.get_script?.(dataLabel, dataParent ?? undefined)
        ?? dataLoader?.getScript?.(dataLabel, dataParent ?? undefined)
        ?? null;
      if (!Array.isArray(dataScript)) {
        continue;
      }
      for (const dataEntry of dataScript) {
        const args = Array.isArray(dataEntry?.args) ? dataEntry.args : [];
        for (const arg of args) {
          const option = stripAsmMenuText(String(arg ?? ""));
          if (option) {
            options.push(option);
          }
        }
      }
      if (options.length) {
        break;
      }
    }
  }
  return options;
};

const getRunnerVariable = (runner: ScriptRunnerLike, key: string): unknown => {
  const variables = runner.variables;
  if (variables instanceof Map) {
    return variables.get(key);
  }
  return variables?.[key];
};

const setRunnerVariable = (runner: ScriptRunnerLike, key: string, value: unknown): void => {
  if (!runner.variables) {
    runner.variables = {};
  }
  if (runner.variables instanceof Map) {
    runner.variables.set(key, value);
    return;
  }
  runner.variables[key] = value;
};

const coerceMenuSelection = (value: unknown, optionCount: number): number => {
  const numeric = typeof value === "string" ? Number.parseInt(value.trim(), 10) : Number(value);
  if (!Number.isInteger(numeric)) {
    return 1;
  }
  return Math.max(1, Math.min(optionCount, numeric));
};

const recordVerticalMenuSelection = (
  gameState: GameState,
  runner: ScriptRunnerLike,
  selection: number,
): void => {
  if (!gameState.wram.script_memory) {
    gameState.wram.script_memory = {};
  }
  gameState.wram.script_memory["wScriptVar"] = selection;
  runner.last_value = selection;
  runner.last_condition_result = selection !== 0;
};

const resolveCurrentParentScript = (runner: ScriptRunnerLike, frame?: { name?: string; parent?: string } | null): string | null => {
  if (typeof runner._find_parent_script_name === "function") {
    const parent = runner._find_parent_script_name();
    if (parent) {
      return parent;
    }
  }
  if (frame?.parent) {
    return frame.parent;
  }
  if (frame?.name && !frame.name.startsWith(".")) {
    return frame.name;
  }
  const stack = runner._script_stack ?? [];
  for (let index = stack.length - 1; index >= 0; index -= 1) {
    const candidate = stack[index];
    if (candidate.parent) {
      return candidate.parent;
    }
    if (candidate.name && !candidate.name.startsWith(".")) {
      return candidate.name;
    }
  }
  return null;
};

const restoreLoadedMenuFromScriptHistory = (
  runner: ScriptRunnerLike,
  dataLoader: DataLoaderLike | null,
): LoadedScriptMenu | null => {
  const frame = runner._script_stack?.[runner._script_stack.length - 1] ?? null;
  if (!frame?.name || typeof frame.index !== "number" || !dataLoader) {
    return null;
  }
  const scriptData =
    dataLoader.get_script?.(frame.name, frame.parent)
    ?? dataLoader.getScript?.(frame.name, frame.parent)
    ?? null;
  if (!Array.isArray(scriptData)) {
    return null;
  }
  const verticalMenuIndex = Math.max(0, Math.min(frame.index - 1, scriptData.length - 1));
  for (let index = verticalMenuIndex - 1; index >= 0; index -= 1) {
    const entry = scriptData[index];
    const command = String(entry?.command ?? "").trim().toLowerCase();
    if (command === "verticalmenu") {
      return null;
    }
    if (command !== "loadmenu") {
      continue;
    }
    const label = Array.isArray(entry?.args) ? String(entry.args[0] ?? "").trim() : "";
    if (!label) {
      return null;
    }
    const parentScript = label.startsWith(".")
      ? resolveCurrentParentScript(runner, frame)
      : null;
    return {
      label,
      options: extractVerticalMenuOptions(dataLoader, label, parentScript),
    };
  }
  return null;
};

export class CheckTimeCommand extends Command {
  private readonly period: string;

  constructor(period: string) {
    super();
    this.period = period.toLowerCase();
  }

  execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldEngine): void {
    const runner = this.runner as ScriptRunnerLike | undefined;
    if (!runner) {
      return;
    }
    const currentTimeStr = canonicaliseTimeOfDay(gameState.wram.time_of_day).toLowerCase();
    const currentFlag = _TIME_PERIOD_FLAGS[currentTimeStr] || 0;
    const expectedFlag = _TIME_PERIOD_FLAGS[this.period] || 0;
    runner.last_condition_result = (currentFlag & expectedFlag) !== 0;
  }
}

export class LoadMenuCommand extends Command {
  constructor(private readonly menuHeaderLabel: string) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    const runner = this.runner as ScriptRunnerLike | undefined;
    if (!runner) {
      throw new Error("LoadMenuCommand requires an active ScriptRunner.");
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const dataLoader =
      runner.dataLoader
      ?? runner.data_loader
      ?? overworldAny?.dataLoader
      ?? overworldAny?.data_loader
      ?? null;
    const frame = runner._script_stack?.[runner._script_stack.length - 1] ?? null;
    const parentScript = this.menuHeaderLabel.startsWith(".")
      ? resolveCurrentParentScript(runner, frame)
      : null;
    const options = extractVerticalMenuOptions(dataLoader, this.menuHeaderLabel, parentScript);
    setRunnerVariable(runner, "_loaded_menu", {
      label: this.menuHeaderLabel,
      options,
    } satisfies LoadedScriptMenu);
  }
}

export class CloseWindowCommand extends Command {
  execute(): void {
    // ASM closes the menu window while leaving the surrounding text box open.
    // The text UI has no separate menu-window layer here, so there is no state
    // to mutate beyond treating the command as implemented.
  }
}

export class VerticalMenuCommand extends Command {
  execute(gameState: GameState, eventManager: EventManager, _overworld: OverworldEngine): void {
    const runner = this.runner as ScriptRunnerLike | undefined;
    if (!runner) {
      throw new Error("VerticalMenuCommand requires an active ScriptRunner.");
    }
    let menu = getRunnerVariable(runner, "_loaded_menu") as LoadedScriptMenu | undefined;
    let options = Array.isArray(menu?.options) ? menu.options : [];
    if (!options.length) {
      const overworldAny = _overworld as OverworldScriptingContext;
      const dataLoader =
        runner.dataLoader
        ?? runner.data_loader
        ?? overworldAny?.dataLoader
        ?? overworldAny?.data_loader
        ?? null;
      const restoredMenu = restoreLoadedMenuFromScriptHistory(runner, dataLoader);
      if (restoredMenu?.options.length) {
        menu = restoredMenu;
        options = restoredMenu.options;
        setRunnerVariable(runner, "_loaded_menu", restoredMenu);
      }
    }
    if (!options.length) {
      throw new Error("VerticalMenuCommand requires a preceding loadmenu with options.");
    }

    const consumeChoice = runner._consume_script_choice ?? runner._consumeScriptChoice;
    if (typeof consumeChoice === "function") {
      const selection = coerceMenuSelection(consumeChoice("_vertical_menu_choice", 1), options.length);
      recordVerticalMenuSelection(gameState, runner, selection);
      return;
    }

    runner.pause();
    eventManager.dispatch(
      new Event("prompt_selection", {
        key: "_vertical_menu_choice",
        title: menu?.label ?? "Choose",
        options,
        initial_index: 0,
        cancel_index: 0,
        callback: (selectionIndex: number) => {
          const selection = coerceMenuSelection(selectionIndex + 1, options.length);
          recordVerticalMenuSelection(gameState, runner, selection);
          runner.resume();
        },
      }),
    );
  }
}

const directionToward = (from: OverworldObjectLike, to: OverworldObjectLike): string | null => {
  const fromX = Number((from as { x?: unknown })?.x ?? (from as { event?: { x?: unknown } })?.event?.x);
  const fromY = Number((from as { y?: unknown })?.y ?? (from as { event?: { y?: unknown } })?.event?.y);
  const toX = Number((to as { x?: unknown })?.x ?? (to as { event?: { x?: unknown } })?.event?.x);
  const toY = Number((to as { y?: unknown })?.y ?? (to as { event?: { y?: unknown } })?.event?.y);
  if (!Number.isFinite(fromX) || !Number.isFinite(fromY) || !Number.isFinite(toX) || !Number.isFinite(toY)) {
    return null;
  }
  const dx = toX - fromX;
  const dy = toY - fromY;
  if (Math.abs(dx) > Math.abs(dy)) {
    return dx < 0 ? "LEFT" : "RIGHT";
  }
  if (dy !== 0) {
    return dy < 0 ? "UP" : "DOWN";
  }
  return null;
};

export class FaceObjectCommand extends Command {
  constructor(private readonly objectId: string, private readonly targetId: string) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    const object = overworld.get_object_by_id(this.objectId);
    const target = overworld.get_object_by_id(this.targetId);
    const direction = directionToward(object, target);
    if (object && direction) {
      object.turn?.(direction);
    }
  }
}

export class ApplyMovementLastTalkedCommand extends Command {
  constructor(private readonly movementDataLabel: string) {
    super();
  }

  execute(gameState: GameState, eventManager: EventManager, overworld: OverworldEngine): void {
    const runner = this.runner as ScriptRunnerLike | undefined;
    const objectId = gameState.wram.last_talked || (runner as { last_interaction_object_index?: number | null } | undefined)?.last_interaction_object_index;
    if (!objectId) {
      throw new Error("applymovementlasttalked requires a last-talked object.");
    }
    const command = new ApplyMovementCommand(String(objectId), this.movementDataLabel);
    command.runner = this.runner;
    command.execute(gameState, eventManager, overworld);
  }
}

export class BattleTowerTextCommand extends Command {
  constructor(private readonly textId: string) {
    super();
  }

  execute(_gameState: GameState, eventManager: EventManager): void {
    eventManager.dispatch(new Event("show_text", { text: this.textId }));
  }
}

export class MoveObjectCommand extends Command {
  constructor(private readonly objectId: string, private readonly mapX: string, private readonly mapY: string) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      throw new Error("MoveObjectCommand requires an active overworld.");
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const move = overworldAny.move_object ?? overworldAny.moveObject;
    if (typeof move !== "function") {
      throw new Error("Overworld does not support object relocation.");
    }
    move.call(overworldAny, this.objectId, this.mapX, this.mapY);
  }
}

export class ChangeBlockCommand extends Command {
  constructor(
    private readonly blockX: number,
    private readonly blockY: number,
    private readonly blockId: number
  ) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    const overworldAny = overworld as OverworldScriptingContext;
    const writeMetatile = overworldAny?._write_metatile;
    if (typeof writeMetatile !== "function") {
      throw new Error("ChangeBlockCommand requires an overworld with _write_metatile.");
    }
    const [metatileX, metatileY] = resolveChangeBlockCoords(this.blockX, this.blockY);
    pushDebugLog(`[script] changeblock (${this.blockX},${this.blockY}) -> metatile (${metatileX},${metatileY}) block=$${this.blockId.toString(16)}`);
    writeMetatile.call(overworldAny, metatileX, metatileY, this.blockId & 0xff);
  }
}

export class RefreshMapCommand extends Command {
  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      throw new Error("RefreshMapCommand requires an active overworld.");
    }
    const overworldAny = overworld as OverworldScriptingContext;
    // ASM: Script_refreshmap recomputes movement permissions after block changes.
    const refreshWarpPermissions = overworldAny._refresh_warp_permissions;
    if (typeof refreshWarpPermissions === "function") {
      refreshWarpPermissions.call(overworldAny);
    }
    // Keep sprite caches in sync with map mutations when supported.
    const refreshSprites = overworldAny.refresh_map_sprites ?? overworldAny.refreshMapSprites;
    if (typeof refreshSprites === "function") {
      refreshSprites.call(overworldAny, { reload_standing: false, reload_walking: false });
    }
  }
}

export class WarpCommand extends Command {
  constructor(private readonly mapConstant: string, private readonly mapX: number, private readonly mapY: number) {
    super();
  }

  execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      throw new Error("WarpCommand requires an active overworld.");
    }

    const overworldAny = overworld as unknown as OverworldScriptingContext & Record<string, unknown>;
    const normalized = String(this.mapConstant ?? "").trim().replace(/,$/, "");
    const normalizedConstant = normalized.toUpperCase();
    if (normalizedConstant === "NONE") {
      // ASM: Script_warp bad-warp branch (engine/overworld/scripting.asm::Script_warp).
      gameState.wram.wDefaultSpawnpoint = 0;
      if (gameState.hram) {
        gameState.hram.hMapEntryMethod = MAPSETUP_CONSTANTS.MAPSETUP_BADWARP;
      }
      const fallback = getMapMetadataByGroup(gameState.wram.wMapGroup, gameState.wram.wMapNumber)?.name ?? null;
      const mapName = overworldAny.current_map_name ?? fallback;
      const reload = overworldAny.reload_current_map ?? overworldAny.reloadCurrentMap;
      if (typeof reload === "function") {
        reload.call(overworldAny);
      } else {
        const loadMap = overworldAny.load_map ?? overworldAny.loadMap;
        if (typeof loadMap !== "function") {
          throw new Error("WarpCommand requires an overworld that can load maps.");
        }
        if (!mapName) {
          throw new Error("WarpCommand could not resolve current map for bad warp.");
        }
        loadMap.call(overworldAny, mapName);
      }
      overworldAny._sync_player_state?.();
      overworldAny.clear_pending_white_fade?.();
      const runner = this.runner as ScriptRunnerLike | undefined;
      if (runner?.stop_all_scripts) {
        runner.stop_all_scripts();
      }
      return;
    }

    const metadata = getMapMetadataByConstant(normalizedConstant);
    if (!metadata) {
      throw new Error(
        `Unknown warp destination '${this.mapConstant}'. Verify the target map exists in map_constants.asm.`
      );
    }

    // ASM: Script_warp writes tile coordinates directly to wXCoord/wYCoord.
    const stride = overworldAny.TILES_PER_COLLISION ?? 2;
    let destX = Math.trunc(this.mapX);
    let destY = Math.trunc(this.mapY);
    if (!Number.isFinite(destX) || !Number.isFinite(destY)) {
      throw new Error(`Invalid warp coordinates (${this.mapX}, ${this.mapY}).`);
    }

    const maxTileX = metadata.width * stride - 1;
    const maxTileY = metadata.height * stride - 1;
    destX = Math.max(0, Math.min(maxTileX, destX));
    destY = Math.max(0, Math.min(maxTileY, destY));
    const scaledX = scaleTileCoord(destX, stride);
    const scaledY = scaleTileCoord(destY, stride);

    const wram = gameState.wram;
    const prevGroup = wram.wMapGroup;
    const prevNumber = wram.wMapNumber;
    wram.wMapGroup = metadata.groupId;
    wram.wMapNumber = metadata.mapId;
    wram.current_map_group = metadata.groupId;
    wram.current_map_id = metadata.mapId;
    wram.wPrevMapGroup = prevGroup;
    wram.wPrevMapNumber = prevNumber;
    wram.wPrevWarp = 0;
    wram.wNextWarp = 0;
    wram.wXCoord = destX;
    wram.wYCoord = destY;
    wram.player_x = Math.floor(destX / stride);
    wram.player_y = Math.floor(destY / stride);
    wram.player_subtile_x = destX % stride;
    wram.player_subtile_y = destY % stride;
    wram.wDefaultSpawnpoint = 0;
    wram.scene_name = "";
    if (gameState.hram) {
      gameState.hram.hMapEntryMethod = MAPSETUP_CONSTANTS.MAPSETUP_WARP;
    }

    overworldAny.player_x = scaledX;
    overworldAny.player_y = scaledY;
    overworldAny.prev_player_x = scaledX;
    overworldAny.prev_player_y = scaledY;
    overworldAny.target_tile_x = scaledX;
    overworldAny.target_tile_y = scaledY;
    overworldAny.is_moving = false;
    overworldAny.step_progress_px = 0.0;
    overworldAny.step_dx_px = 0.0;
    overworldAny.step_dy_px = 0.0;
    overworldAny._queued_direction = null;

    const loadMap = overworldAny.load_map ?? overworldAny.loadMap;
    if (typeof loadMap !== "function") {
      throw new Error("WarpCommand requires an overworld that can load maps.");
    }
    loadMap.call(overworldAny, metadata.name);

    overworldAny._sync_player_state?.();
    overworldAny.clear_pending_white_fade?.();

    if ("_active_warp_tile" in overworldAny) {
      overworldAny._active_warp_tile = [metadata.name, scaledX, scaledY];
    }
    if ("_warp_cooldown" in overworldAny && typeof overworldAny.WALK_FRAMES === "number") {
      overworldAny._warp_cooldown = overworldAny.WALK_FRAMES;
    }

    const runner = this.runner as ScriptRunnerLike | undefined;
    if (runner?.stop_all_scripts) {
      runner.stop_all_scripts();
    }
  }
}

export class WarpSoundCommand extends Command {
  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      throw new Error("WarpSoundCommand requires an active overworld.");
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const permission =
      overworldAny._current_tile_permission?.()
      ?? (() => {
        const map = overworldAny.map;
        const tileset = overworldAny.tileset;
        if (!map || !tileset) {
          return null;
        }
        if (typeof overworldAny.player_x !== "number" || typeof overworldAny.player_y !== "number") {
          return null;
        }
        return sampleCollision(map as any, tileset as any, overworldAny.player_x, overworldAny.player_y).permission;
      })();
    if (permission === null || permission === undefined) {
      throw new Error("WarpSoundCommand could not resolve tile collision.");
    }
    // ASM: engine/overworld/tile_events.asm::GetWarpSFX.
    const soundId = warpSoundForPermission(permission);
    if (!soundId) {
      return;
    }
    const runner = this.runner as ScriptRunnerLike | undefined;
    if (runner) {
      runner.last_sound_effect = soundId;
    }
    const audioEngine = getAudioEngine(overworldAny);
    playOverworldSound(audioEngine, soundId, { logger: overworldAny._logger ?? null, context: "warp sound" });
  }
}

export class WarpFacingCommand extends Command {
  constructor(
    private readonly direction: string,
    private readonly mapConstant: string,
    private readonly mapX: number,
    private readonly mapY: number
  ) {
    super();
  }

  execute(gameState: GameState, eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      throw new Error("WarpFacingCommand requires an active overworld.");
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const facing = normalizeFacingDirection(this.direction);
    overworldAny.player_direction = facing;
    const player = overworldAny.player_object ?? overworldAny.get_object_by_id?.("PLAYER") ?? null;
    player?.turn?.(facing);
    // ASM: engine/overworld/scripting.asm::Script_warpfacing falls through to Script_warp.
    const warp = new WarpCommand(this.mapConstant, this.mapX, this.mapY);
    warp.runner = this.runner;
    warp.execute(gameState, eventManager, overworld);
  }
}

export class WarpCheckCommand extends Command {
  execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      throw new Error("WarpCheckCommand requires an active overworld.");
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const warpCheck = overworldAny.check_for_warp_event ?? overworldAny.checkForWarpEvent;
    if (typeof warpCheck !== "function") {
      throw new Error("Overworld implementation missing check_for_warp_event().");
    }
    const warped = warpCheck.call(overworldAny, { allow_script: true, ignore_cooldown: true });
    if (warped) {
      // ASM: EnableEvents (engine/overworld/events.asm) sets all player events enabled.
      gameState.wram.wEnabledPlayerEvents = 0xff;
    }
  }
}

export class NewLoadMapCommand extends Command {
  constructor(private readonly entryMethod: string) {
    super();
  }

  execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      throw new Error("NewLoadMapCommand requires an active overworld.");
    }
    const overworldAny = overworld as OverworldScriptingContext;
    const methodValue = parseMapSetupToken(this.entryMethod);
    if (gameState.hram) {
      gameState.hram.hMapEntryMethod = methodValue;
    }
    const metadata = getMapMetadataByGroup(gameState.wram.wMapGroup, gameState.wram.wMapNumber);
    const mapName = metadata?.name ?? overworldAny.current_map_name ?? null;
    if (!mapName) {
      throw new Error("NewLoadMapCommand could not resolve map name.");
    }

    if (methodValue === MAPSETUP_CONSTANTS.MAPSETUP_LINKRETURN) {
      const runner = this.runner as ScriptRunnerLike | undefined;
      const dataLoader = runner?.dataLoader ?? runner?.data_loader ?? null;
      let currentEvents = (dataLoader as any)?.map_events?.get?.(mapName);
      if (!currentEvents) {
        (dataLoader as any)?.ensure_map_scripts?.(mapName);
        currentEvents = (dataLoader as any)?.map_events?.get?.(mapName);
      }
      const linkReturnWarp = currentEvents?.warps?.[0] ?? null;
      if (linkReturnWarp) {
        const targetMetadata = getMapMetadataByConstant(linkReturnWarp.target_map_constant);
        if (!targetMetadata) {
          throw new Error(
            `NewLoadMapCommand could not resolve LINKRETURN destination '${linkReturnWarp.target_map_constant}'.`
          );
        }
        let targetEvents = (dataLoader as any)?.map_events?.get?.(targetMetadata.name);
        if (!targetEvents) {
          (dataLoader as any)?.ensure_map_scripts?.(targetMetadata.name);
          targetEvents = (dataLoader as any)?.map_events?.get?.(targetMetadata.name);
        }
        const destinationWarpIndex = Math.max(0, Number(linkReturnWarp.target_warp_id ?? 1) - 1);
        const destinationWarp = targetEvents?.warps?.[destinationWarpIndex] ?? null;
        if (!destinationWarp) {
          throw new Error(
            `NewLoadMapCommand missing LINKRETURN warp ${linkReturnWarp.target_warp_id} on '${targetMetadata.name}'.`
          );
        }

        const stride = overworldAny.TILES_PER_COLLISION ?? 2;
        const [scaledX, scaledY] = scaleTileCoords(destinationWarp.x, destinationWarp.y, stride);
        gameState.wram.wMapGroup = targetMetadata.groupId;
        gameState.wram.wMapNumber = targetMetadata.mapId;
        gameState.wram.current_map_group = targetMetadata.groupId;
        gameState.wram.current_map_id = targetMetadata.mapId;
        gameState.wram.wXCoord = destinationWarp.x;
        gameState.wram.wYCoord = destinationWarp.y;
        gameState.wram.player_x = Math.floor(destinationWarp.x / stride);
        gameState.wram.player_y = Math.floor(destinationWarp.y / stride);
        gameState.wram.player_subtile_x = destinationWarp.x % stride;
        gameState.wram.player_subtile_y = destinationWarp.y % stride;

        overworldAny.player_x = scaledX;
        overworldAny.player_y = scaledY;
        overworldAny.prev_player_x = scaledX;
        overworldAny.prev_player_y = scaledY;
        overworldAny.target_tile_x = scaledX;
        overworldAny.target_tile_y = scaledY;
        overworldAny.is_moving = false;
        overworldAny.step_progress_px = 0.0;
        overworldAny.step_dx_px = 0.0;
        overworldAny.step_dy_px = 0.0;
        overworldAny._queued_direction = null;

        const reload = overworldAny.reload_current_map ?? overworldAny.reloadCurrentMap;
        if (typeof reload === "function" && targetMetadata.name === overworldAny.current_map_name) {
          reload.call(overworldAny);
        } else {
          const loadMap = overworldAny.load_map ?? overworldAny.loadMap;
          if (typeof loadMap !== "function") {
            throw new Error("NewLoadMapCommand requires an overworld that can load maps.");
          }
          loadMap.call(overworldAny, targetMetadata.name);
        }

        overworldAny._sync_player_state?.();
        overworldAny.clear_pending_white_fade?.();

        if (runner?.stop_all_scripts) {
          runner.stop_all_scripts();
        }
        return;
      }
    }

    const tileX = gameState.wram.wXCoord;
    const tileY = gameState.wram.wYCoord;
    if (Number.isFinite(tileX) && Number.isFinite(tileY)) {
      const destX = Math.trunc(tileX);
      const destY = Math.trunc(tileY);
      const stride = overworldAny.TILES_PER_COLLISION ?? 2;
      const scaledX = scaleTileCoord(destX, stride);
      const scaledY = scaleTileCoord(destY, stride);
      overworldAny.player_x = scaledX;
      overworldAny.player_y = scaledY;
      overworldAny.prev_player_x = scaledX;
      overworldAny.prev_player_y = scaledY;
      overworldAny.target_tile_x = scaledX;
      overworldAny.target_tile_y = scaledY;
      overworldAny.is_moving = false;
      overworldAny.step_progress_px = 0.0;
      overworldAny.step_dx_px = 0.0;
      overworldAny.step_dy_px = 0.0;
      overworldAny._queued_direction = null;
    }

    const reload = overworldAny.reload_current_map ?? overworldAny.reloadCurrentMap;
    if (typeof reload === "function") {
      reload.call(overworldAny);
    } else {
      const loadMap = overworldAny.load_map ?? overworldAny.loadMap;
      if (typeof loadMap !== "function") {
        throw new Error("NewLoadMapCommand requires an overworld that can load maps.");
      }
      loadMap.call(overworldAny, mapName);
    }

    overworldAny._sync_player_state?.();
    overworldAny.clear_pending_white_fade?.();

    const runner = this.runner as ScriptRunnerLike | undefined;
    if (runner?.stop_all_scripts) {
      runner.stop_all_scripts();
    }
  }
}

export class SwarmCommand extends Command {
  constructor(private readonly swarmToken: string, private readonly mapConstant: string) {
    super();
  }

  execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldEngine): void {
    const definition = resolveSwarmDefinition(this.swarmToken.trim().replace(/,$/, ""));
    const normalizedMap = this.mapConstant.trim().replace(/,$/, "").toUpperCase();
    if (normalizedMap !== definition.mapConstant) {
      throw new Error(
        `Swarm '${this.swarmToken}' expects '${definition.mapConstant}', got '${this.mapConstant}'.`
      );
    }
    const metadata = getMapMetadataByConstant(definition.mapConstant);
    if (!metadata) {
      throw new Error(`Unknown swarm destination '${definition.mapConstant}'.`);
    }
    const wram = gameState.wram as unknown as { swarm_flags: number } & Record<string, number>;
    wram[definition.mapGroupAttr] = metadata.groupId;
    wram[definition.mapNumberAttr] = metadata.mapId;
    wram.swarm_flags |= definition.bitMask;
  }
}

export class ElevatorCommand extends Command {
  constructor(private readonly dataLabel: string) {
    super();
  }

  execute(gameState: GameState, eventManager: EventManager, overworld: OverworldEngine): void {
    const runner = this.runner as ScriptRunnerLike | undefined;
    const dataLoader = runner?.dataLoader ?? runner?.data_loader ?? null;
    if (!runner || !dataLoader) {
      throw new Error("ElevatorCommand requires an active script runner and data.");
    }

    let parentScript: string | null = null;
    if (Array.isArray(runner._script_stack) && runner._script_stack.length) {
      parentScript = runner._script_stack[runner._script_stack.length - 1]?.name ?? null;
    }
    const floors = parseElevatorData(dataLoader, this.dataLabel, parentScript);

    if (!gameState.wram.script_memory) {
      gameState.wram.script_memory = {};
    }
    gameState.wram.script_memory["wScriptVar"] = 0;

    if (!floors.length) {
      runner.last_condition_result = false;
      return;
    }

    const originIndex = findOriginFloorIndex(floors, gameState);
    const defaultSelection = originIndex !== null ? originIndex : 0;
    let selection: unknown = defaultSelection;
    if (typeof runner._consume_script_choice === "function") {
      selection = runner._consume_script_choice("_elevator_selection", defaultSelection);
    } else {
      runner.pause();
      eventManager.dispatch(
        new Event<ElevatorSelectionPromptPayload>("prompt_selection", {
          key: "_elevator_selection",
          title: "Which floor?",
          options: floors.map((floor) => formatFloorName(floor.label)),
          initial_index: defaultSelection,
          cancel_index: defaultSelection,
          callback: (selectionIndex: number) => {
            this.applySelection(gameState, overworld, runner, floors, defaultSelection, selectionIndex);
          },
        }),
      );
      return;
    }
    this.applySelection(gameState, overworld, runner, floors, defaultSelection, selection as ElevatorChoice);
  }

  private applySelection(
    gameState: GameState,
    overworld: OverworldEngine,
    runner: ScriptRunnerLike,
    floors: ElevatorFloor[],
    defaultSelection: number,
    selection: ElevatorChoice,
  ): void {
    const selectionIndex = coerceFloorSelection(selection, floors, defaultSelection);
    const originIndex = findOriginFloorIndex(floors, gameState);
    const origin = originIndex !== null ? originIndex : selectionIndex;
    if (selectionIndex < 0 || selectionIndex >= floors.length || selectionIndex === origin) {
      runner.last_condition_result = false;
      gameState.wram.script_memory["wScriptVar"] = 0;
      return;
    }

    const chosenFloor = floors[selectionIndex];
    const metadata = getMapMetadataByConstant(chosenFloor.mapConstant);
    if (!metadata) {
      throw new Error(
        `Unknown elevator destination '${chosenFloor.mapConstant}' for ${this.dataLabel}.`
      );
    }

    const wram = gameState.wram;
    wram.wBackupMapGroup = metadata.groupId;
    wram.wBackupMapNumber = metadata.mapId;
    wram.wBackupWarpNumber = chosenFloor.warpId;
    wram.script_memory["wScriptVar"] = 1;
    runner.last_condition_result = true;

    const originName = formatFloorName(floors[origin].label);
    const destinationName = formatFloorName(chosenFloor.label);
    runner.last_value = {
      elevator: {
        origin: originName,
        destination: destinationName,
        warp_id: chosenFloor.warpId,
        map_constant: chosenFloor.mapConstant,
      },
    };

    const overworldAny = overworld as OverworldScriptingContext;
    const startRide = overworldAny?.start_elevator_ride ?? overworldAny?.startElevatorRide;
    if (typeof startRide === "function") {
      startRide.call(overworldAny, originName, destinationName, {
        door_close_frames: 10,
        door_open_frames: 10,
        fade_frames: 8,
        travel_frames: 60,
        trigger_sound: true,
        trigger_earthquake: false,
      });
    }
  }
}

export class TurnObjectCommand extends Command {
  constructor(private readonly objectId: string, private readonly direction: string) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    const obj = overworld.get_object_by_id(this.objectId);
    if (obj) {
      if (LOGGER.debug) {
        LOGGER.debug("TurnObjectCommand turning %s toward %s", this.objectId, this.direction);
      }
      obj.turn?.(this.direction);
    }
  }
}
export class ShowEmoteCommand extends Command {
  constructor(private readonly emoteId: string, private readonly objectId: string, private readonly duration: number) {
    super();
  }
  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    const obj = overworld.get_object_by_id(this.objectId);
    if (obj) {
      if (LOGGER.debug) {
        LOGGER.debug(
          "ShowEmoteCommand showing %s for %s (%d frames)",
          this.emoteId,
          this.objectId,
          this.duration
        );
      }
      const overworldWithMixin = overworld as OverworldEngine & { show_emote: Function };
      if (typeof overworldWithMixin.show_emote !== "function") {
        throw new Error("Overworld implementation missing show_emote().");
      }
      const duration = _gameState.wram.instant_mode ? 1 : this.duration;
      overworldWithMixin.show_emote(this.emoteId, obj, duration);
    }
  }
}

export class ApplyMovementCommand extends Command {
  constructor(private readonly objectId: string, private readonly movementDataLabel: string) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      return;
    }
    const overworldAny = overworld as OverworldScriptingContext;
    let obj = overworld.get_object_by_id(this.objectId);
    if (!obj) {
      const appear = overworldAny.appear_object ?? overworldAny.appearObject;
      if (typeof appear === "function") {
        appear.call(overworldAny, this.objectId);
      }
      obj = overworld.get_object_by_id(this.objectId);
    }
    if (!obj) {
      const appear = overworldAny.appear_object ?? overworldAny.appearObject;
      if (typeof appear === "function") {
        appear.call(overworldAny, this.objectId, { force_spawn: true });
      }
      obj = overworld.get_object_by_id(this.objectId);
    }
    if (!obj) {
      const resolveObjectIndex = overworldAny.resolve_object_index ?? overworldAny.resolveObjectIndex;
      const index = typeof resolveObjectIndex === "function"
        ? resolveObjectIndex.call(overworldAny, this.objectId)
        : null;
      if (index !== null && index !== undefined) {
        const appear = overworldAny.appear_object ?? overworldAny.appearObject;
        if (typeof appear === "function") {
          appear.call(overworldAny, index, { force_spawn: true });
        }
        obj = overworld.get_object_by_id(index) ?? overworld.get_object_by_id(this.objectId);
      }
    }
    if (!obj) {
      throw new Error(`Unknown movement target '${this.objectId}'`);
    }

    if (
      this.objectId.toUpperCase() === "CHERRYGROVECITY_RIVAL"
      && this.movementDataLabel === "CherrygroveCity_RivalWalksToYou"
      && overworld.current_map_name === "CherrygroveCity"
    ) {
      // ASM: CherrygroveRivalSceneSouth (pokecrystal_disassembly/maps/CherrygroveCity.asm) aligns the rival's Y row.
      const playerRawY = _gameState?.wram?.wYCoord;
      const event = obj.event;
      const currentRawX = Number(event?.x);
      const currentRawY = Number(event?.y);
      const stride = (overworld as any).TILES_PER_COLLISION ?? 2;
      const targetRawY = Number.isFinite(playerRawY) ? Math.trunc(playerRawY / stride) : NaN;
      if (Number.isFinite(currentRawX) && Number.isFinite(targetRawY) && currentRawY !== targetRawY) {
        overworld.move_object(this.objectId, currentRawX, targetRawY);
        obj = overworld.get_object_by_id(this.objectId) ?? obj;
      }
    }

    const parentScript = this.movementDataLabel.startsWith(".")
      ? this.runner?._find_parent_script_name?.() ?? null
      : null;
    const movementData = overworldAny.get_movement_data?.(this.movementDataLabel, parentScript)
      ?? overworldAny.getMovementData?.(this.movementDataLabel, parentScript);
    if (!movementData) {
      throw new Error(`Missing movement data '${this.movementDataLabel}'.`);
    }
    pushDebugLog(
      `[script] applymovement ${this.objectId} -> ${this.movementDataLabel}`,
      { x: obj.x, y: obj.y }
    );

    if (LOGGER.debug) {
      LOGGER.debug(
        "ApplyMovementCommand queueing %s (%s) from (%s,%s) with steps=%s",
        this.objectId,
        this.movementDataLabel,
        obj.x ?? "?",
        obj.y ?? "?",
        Array.from(movementData)
      );
    }

    const runner = this.runner as ScriptRunnerLike | undefined;
    const movement = Array.from(movementData);
    if (!runner) {
      pushDebugLog(`[script] applymovement ${this.objectId} (sync)`);
      obj.applyMovement?.(movement);
      if (LOGGER.debug) {
        LOGGER.debug("ApplyMovementCommand applied movement synchronously for %s", this.objectId);
      }
      return;
    }

    const queueMovementTask = overworldAny.queue_movement_task ?? overworldAny.queueMovementTask;
    const schedule = (callback: () => void): void => {
      if (typeof queueMovementTask === "function") {
        queueMovementTask.call(overworldAny, obj, movement, { onComplete: callback });
        return;
      }
      obj.applyMovement?.(movement);
      // ASM: ApplyMovementToFollower (engine/overworld/map_objects.asm) handles follower steps.
      callback();
    };

    const queueOverworldTask = runner._queue_overworld_task ?? runner._queueOverworldTask;
    if (typeof queueOverworldTask === "function") {
      queueOverworldTask.call(runner, (callback: () => void) => {
        schedule(callback);
        return true;
      });
      return;
    }
    runner.pause();
    schedule(() => runner.resume());
  }
}

export class FacePlayerCommand extends Command {
  execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldEngine): void {
    if (!overworld) {
      if (LOGGER.debug) {
        LOGGER.debug("FacePlayerCommand invoked without overworld; skipping.");
      }
      return;
    }
    const lastTalked = Number(gameState.wram.last_talked ?? 0);
    let obj = overworld.get_object_by_id(lastTalked);
    if (!obj) {
      const fallbackLastTalked = Number((overworld as any)?.game_state?.wram?.last_talked ?? 0);
      if (fallbackLastTalked && fallbackLastTalked !== lastTalked) {
        obj = overworld.get_object_by_id(fallbackLastTalked);
      }
    }
    if (obj) {
      if (LOGGER.debug) {
        LOGGER.debug(
          "FacePlayerCommand rotating %s to face player",
          obj.objectId ?? String(lastTalked)
        );
      }
      const player =
        (overworld as any).player_object ?? overworld.get_object_by_id(0);
      if (!player) {
        throw new Error("FacePlayerCommand requires an active player object on the overworld.");
      }
      if (typeof obj.facePlayer === "function") {
        obj.facePlayer(player.x, player.y);
      } else if (typeof (obj as any).face_player === "function") {
        (obj as any).face_player(player.x, player.y);
      }
    }
  }
}

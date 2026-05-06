// ASM mapping: pokecrystal_disassembly/engine/overworld/overworld.asm (overworld main loop, PlayerEvents, map entry).
import { gameEngine, type GameEngineEvent, type GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { clearJoypad } from "@pokecrystal/core/core/home";
import { BackgroundEvent, MapAttributes, MapEvents, ObjectEvent } from "@pokecrystal/core/core/models/map";
import { FacingDirection, PlayerState, facingDirectionFromString } from "@pokecrystal/core/core/enums/overworld";
import { JOY_A } from "@pokecrystal/core/core/constants";
import type { GameState } from "@pokecrystal/core/core/state";
import { resolveTextboxFrameRenderId } from "@pokecrystal/core/core/textbox-frame";
import { SpriteAnimation, primeWalkStride } from "@pokecrystal/core/engine/systems/animation";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { DailyEventSystem } from "@pokecrystal/core/engine/systems/daily-events";
import { ItemSystem } from "@pokecrystal/core/engine/systems/items";
import { StepEventSystem } from "@pokecrystal/core/engine/systems/step-events";
import { Event, EventManager, close_text, open_text, show_text, wait_for_input } from "@pokecrystal/core/engine/world/events";
import type { BlockFeedbackDetails, Overworld as OverworldType } from "@pokecrystal/core/types/overworld";
import { ElevatorRideStateMachine } from "@pokecrystal/core/engine/world/overworld/events";
import { getMapEnvironment, getMapMetadataByGroup } from "@pokecrystal/core/engine/world/maps";
import { defaultMusicTokenForMap } from "@pokecrystal/core/engine/world/map-music";
import { describeCollision, Terrain } from "@pokecrystal/core/engine/world/overworld/collision-data";
import { determineQuadrantIndex, getCollisionStdScript } from "@pokecrystal/core/engine/world/overworld/collision-rules";
import type { PokemonData } from "@pokecrystal/core/core/models";
import {
  CUTTABLE_COLLISIONS,
  DEFAULT_ENABLED_PLAYER_EVENTS,
  PLAYEREVENTS_COORD_EVENTS,
  PLAYEREVENTS_COUNT_STEPS,
  PLAYEREVENTS_WARPS_AND_CONNECTIONS,
  PLAYEREVENTS_WILD_ENCOUNTERS,
  TALL_GRASS_COLLISIONS,
  WATERFALL_COLLISIONS,
  WHIRLPOOL_COLLISIONS,
} from "@pokecrystal/core/engine/world/overworld/constants";
import { DialogueEventController } from "@pokecrystal/core/engine/world/overworld/dialogue-controller";
import {
  FieldMoveAnimationController,
  bind_tile_animation_timer,
  tile_animation_timer,
} from "@pokecrystal/core/engine/world/overworld/field-move-animation";
import { getBooleanFlag, setBooleanFlag } from "@pokecrystal/core/engine/world/overworld/flag-collection";
import { GrassRustleController } from "@pokecrystal/core/engine/world/overworld/grass-rustle";
import { MapNameSignController } from "@pokecrystal/core/engine/world/overworld/map-sign";
import { NpcAutonomousController } from "@pokecrystal/core/engine/world/overworld/npc-autonomous-controller";
import { NpcSpriteCache } from "@pokecrystal/core/engine/world/overworld/npc-sprites";
import { OverworldFieldMoveMixin } from "@pokecrystal/core/engine/world/overworld/overworld-field-moves";
import { OverworldInputMixin } from "@pokecrystal/core/engine/world/overworld/overworld-input";
import { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import { EmoteSurfaceCache, OverworldMapManagerMixin } from "@pokecrystal/core/engine/world/overworld/overworld-map-manager";
import { OverworldMovement } from "@pokecrystal/core/engine/world/overworld/overworld-movement";
import { OverworldNpcManagerMixin } from "@pokecrystal/core/engine/world/overworld/overworld-npc-manager";
import { OverworldRenderingMixin } from "@pokecrystal/core/engine/world/overworld/overworld-rendering";
import { OverworldScriptQueueMixin } from "@pokecrystal/core/engine/world/overworld/overworld-script-queue";
import { NpcPaletteManager } from "@pokecrystal/core/engine/world/overworld/palette";
import type { PendingEventFlagUpdate } from "@pokecrystal/core/engine/world/overworld/pending-event-flag-updates";
import {
  flagForPlayerState,
  normalizePlayerState,
  playerStateFromFlag,
} from "@pokecrystal/core/engine/world/overworld/player-state-flags";
import { MovementTask, type MovementOverworldContext } from "@pokecrystal/core/engine/world/overworld/script-tasks/movement-task";
import { OverworldTimeSystem } from "@pokecrystal/core/engine/world/overworld/time-system";
import { TilesetAnimationController } from "@pokecrystal/core/engine/world/overworld/tileset-animation";
import { TrainerSightlineMixin } from "@pokecrystal/core/engine/world/overworld/trainer-sightlines";
import type { TrainerSighting } from "@pokecrystal/core/engine/world/overworld/trainer-sightlines";
import { WildEncounterManager } from "@pokecrystal/core/engine/world/overworld/wild-encounters";
import { pushDebugLog } from "@pokecrystal/core/core/debug-log";
import { isDebugEnabled } from "@pokecrystal/core/core/debug-flags";
import { applyPoisonToParty, PoisonDamageResult } from "@pokecrystal/core/engine/world/poison";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";
import type { ScriptTask } from "@pokecrystal/core/engine/world/overworld/script-tasks/script-task";
import { advanceSafariTimer } from "@pokecrystal/core/engine/world/safari-zone";
import { check_bug_contest_timer } from "@pokecrystal/core/engine/world/special-events/bug-contest";
import { ScriptRunner, ScriptRunnerImpl } from "@pokecrystal/core/engine/world/story-events/runner";
import { WarpCommand } from "@pokecrystal/core/engine/world/story-events/commands/overworld";
import { METATILE_WIDTH, TILE_SIZE } from "@pokecrystal/core/engine/world/tile/constants";
import type { OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";
import { PokePicOverlay, type UI as PokePicUI } from "@pokecrystal/core/ui/overlays/pokepic";
import { EggHatchAnimation, type UI as EggHatchUI } from "@pokecrystal/core/ui/overlays/egg-hatch";
import { PhoneCallOverlay, type PhoneOverlayUI } from "@pokecrystal/core/ui/overlays/phone-call-overlay";
import { TownMapOverlay } from "@pokecrystal/core/ui/overlays/town-map-overlay";
import { FieldDialogueManager } from "@pokecrystal/core/ui/text/dialogue";
import { SelectionPrompt } from "@pokecrystal/core/ui/text/prompts";
import type { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { update_player_sprite } from "@pokecrystal/core/engine/world/special-events/sprites";
import { OverworldObject } from "@pokecrystal/core/engine/world/overworld/overworld-object";
import { PlayerCharacter } from "@pokecrystal/core/engine/world/overworld/playable-character";
import type { RemoteOverworldPlayer } from "@pokecrystal/core/types/overworld";
import { OverworldAudioController } from "@pokecrystal/core/engine/world/overworld/audio-controller";

type ScriptStatusSnapshot = [boolean, number, number, boolean, number, boolean, boolean];

const canonicalKeyItemName = (itemName: string): string =>
  itemName.normalize("NFKD").replace(/[^A-Za-z0-9]/g, "").toUpperCase();

const scriptKeyItemName = (canonical: string): string => {
  if (canonical === "OLDROD") {
    return "OLD_ROD";
  }
  if (canonical === "GOODROD") {
    return "GOOD_ROD";
  }
  if (canonical === "SUPERROD") {
    return "SUPER_ROD";
  }
  return canonical;
};

type DialoguePromptTransitionState = {
  yes_no_prompt?: unknown;
  pending_yes_no_request?: boolean;
};

const scriptStatusSnapshotEquals = (
  a: ScriptStatusSnapshot | null,
  b: ScriptStatusSnapshot | null,
): boolean => {
  if (a === b) {
    return true;
  }
  if (!a || !b) {
    return false;
  }
  return (
    a[0] === b[0] &&
    a[1] === b[1] &&
    a[2] === b[2] &&
    a[3] === b[3] &&
    a[4] === b[4] &&
    a[5] === b[5] &&
    a[6] === b[6]
  );
};

// Exported for regression tests only (guards against log-spam freezes in dev).
export const __test__scriptStatusSnapshotEquals = scriptStatusSnapshotEquals;

const dialoguePromptTransitionPending = (
  dialogue: DialoguePromptTransitionState | null | undefined
): boolean =>
  Boolean(dialogue?.yes_no_prompt) || Boolean(dialogue?.pending_yes_no_request);

const POISON_OVERLAY_COLOR: [number, number, number, number] = [230, 173, 255, 255];
const POISON_OVERLAY_ALPHA = 176;
const POISON_FLASH_DURATION = 4;

const TALL_GRASS_COLLISIONS_SET = new Set(TALL_GRASS_COLLISIONS);

const SCREEN_WIDTH_TILES = 20;
const SCREEN_HEIGHT_TILES = 18;
const _BIKE_ALLOWED_ENVIRONMENTS = new Set(["ROUTE", "TOWN", "CAVE", "GATE"]);
const _NO_BIKE_ENVIRONMENTS = new Set(["INDOOR", "ENVIRONMENT_5", "DUNGEON"]);
const _BIKEFLAG_STRENGTH_ACTIVE = 1 << 0;
const _BIKEFLAG_ALWAYS_ON_BIKE = 1 << 1;
const _BIKEFLAG_DOWNHILL = 1 << 2;

const syncBikeFlags = (wram: GameState["wram"] | null | undefined): number => {
  if (!wram) {
    return 0;
  }
  let flags = Number(wram.wBikeFlags ?? 0) & 0xff;
  if (getBooleanFlag(wram.engine_flags, "ENGINE_STRENGTH_ACTIVE")) {
    flags |= _BIKEFLAG_STRENGTH_ACTIVE;
  }
  if (getBooleanFlag(wram.engine_flags, "ENGINE_ALWAYS_ON_BIKE")) {
    flags |= _BIKEFLAG_ALWAYS_ON_BIKE;
  }
  if (getBooleanFlag(wram.engine_flags, "ENGINE_DOWNHILL")) {
    flags |= _BIKEFLAG_DOWNHILL;
  }
  wram.wBikeFlags = flags;
  setBooleanFlag(wram.engine_flags, "ENGINE_STRENGTH_ACTIVE", Boolean(flags & _BIKEFLAG_STRENGTH_ACTIVE));
  setBooleanFlag(wram.engine_flags, "ENGINE_ALWAYS_ON_BIKE", Boolean(flags & _BIKEFLAG_ALWAYS_ON_BIKE));
  setBooleanFlag(wram.engine_flags, "ENGINE_DOWNHILL", Boolean(flags & _BIKEFLAG_DOWNHILL));
  return flags;
};

const FOLLOWER_STEP_INSTRUCTIONS = new Set([
  "step",
  "slow_step",
  "slow_jump_step",
  "slow_slide_step",
  "jump_step",
  "slide_step",
  "big_step",
  "fast_jump_step",
  "fast_slide_step",
]);

type DataLoaderLike = DataLoader & {
  get_hidden_item_event_flag?: (scriptName: string) => string | null;
  get_bg_event_script_flag?: (scriptName: string) => string | null;
  movement_data?: Record<string, string[]>;
};

type DialogueUIContext = ConstructorParameters<typeof FieldDialogueManager>[0];

type OverworldUIContext = BaseUI &
  PokePicUI &
  EggHatchUI &
  PhoneOverlayUI &
  DialogueUIContext & {
    eventQueue?: GameEngineEventQueue;
    screen?: { get_size?: () => [number, number] };
    update?: () => void;
  };

type LegacyPoisonResult = Partial<PoisonDamageResult> & {
  damaged_names?: string[];
  fainted_names?: string[];
  survivors?: Array<PokemonData | null>;
};

type OverworldInputHandler = {
  handle_input: (event: GameEngineEvent) => void;
};

const getMetatileAt = (map_obj: OverworldMap, metatile_x: number, metatile_y: number): number => {
  return map_obj.getMetatileAt(metatile_x, metatile_y);
};

const getMetatileIds = (map_obj: OverworldMap): number[] => {
  return map_obj.metatileIds ?? [];
};

const normalizePoisonResult = (
  result: PoisonDamageResult | LegacyPoisonResult,
  party: Array<PokemonData | null>,
): { damagedNames: string[]; faintedNames: string[]; survivors: Array<PokemonData | null> } => {
  const damagedNames = "damagedNames" in result
    ? result.damagedNames ?? []
    : ("damaged_names" in result ? result.damaged_names ?? [] : []);
  const faintedNames = "faintedNames" in result
    ? result.faintedNames ?? []
    : ("fainted_names" in result ? result.fainted_names ?? [] : []);
  const survivors = "survivors" in result && Array.isArray(result.survivors)
    ? result.survivors
    : party.filter((pokemon) => pokemon && (pokemon.hp ?? 0) > 0);
  return { damagedNames, faintedNames, survivors };
};

const applyMixins = (derivedCtor: { prototype: unknown }, baseCtors: Array<{ prototype: unknown }>): void => {
  for (const baseCtor of baseCtors) {
    const propertyNames = Object.getOwnPropertyNames(baseCtor.prototype);
    for (const name of propertyNames) {
      if (name === "constructor") {
        continue;
      }
      const descriptor = Object.getOwnPropertyDescriptor(baseCtor.prototype, name);
      if (!descriptor) {
        continue;
      }
      Object.defineProperty(derivedCtor.prototype, name, descriptor);
    }
  }
};

const hasUpdatePixelPosition = (
  target: unknown,
): target is { updatePixelPosition: () => void } => {
  return typeof (target as { updatePixelPosition?: unknown }).updatePixelPosition === "function";
};

const resolveCollisionStride = (target: unknown, fallback: number): number => {
  const stride = (target as { collisionStride?: number }).collisionStride;
  return typeof stride === "number" ? stride : fallback;
};

export class OverworldEngine extends OverworldMapManagerMixin {
  [key: string]: any;
  public ui: BaseUI | null = null;
  public pokepic_overlay!: PokePicOverlay;
  public _egg_hatch_animation: EggHatchAnimation | null = null;
  public _egg_hatch_display_name: string | null = null;
  public _hatch_text_pending = false;
  public _phone_call_overlay: PhoneCallOverlay | null = null;
  public _player_state: PlayerState | null = null;
  public item_system!: ItemSystem;
  public _step_events!: StepEventSystem;
  public _daily_event_system!: DailyEventSystem;
  public _overworld_time_system!: OverworldTimeSystem;
  public _town_map_overlay!: TownMapOverlay;
  public _dialogue_controller!: DialogueEventController;
  public audio_controller!: OverworldAudioController;
  public _last_pending_script_status: ScriptStatusSnapshot | null = null;
  public _last_wait_status_message: string | null = null;
  public _map_has_tall_grass = true;
  public _map_has_trainer_sightlines = true;
  public _skip_wild_encounter_for_step = false;
  public _map_prefix_lookup: Record<string, string> = {};
  public _map_prefixes: Array<[string, string]> = [];
  public _follower_step_queue: string[] = [];
  public _follower_queue_length = 0;
  public _follower_movement_task: MovementTask | null = null;
  public _follower_buffer_step: string | null = null;
  public _follower_release_tokens = 0;
  public _active_follower_target: [number, number] | null = null;
  public _pending_follower_origin: [number, number] | null = null;
  public _last_step_direction: string | null = null;
  public _movement_lock_count = 0;
  public _text_lock_active = false;
  public _turn_frames_remaining = 0;
  public _turning_direction: string | null = null;
  public _turn_should_force_step = false;
  public _pending_auto_step: [string, boolean] | null = null;
  public player_px_x = 0;
  public player_px_y = 0;
  public target_px_x = 0;
  public target_px_y = 0;
  public _current_step_distance_px = 0;
  public _current_step_speed_px = 0;
  public _ledge_jump_active = false;
  public _ledge_jump_total_distance_px = 0;
  public _ledge_jump_animation_progress_px = 0;
  public _pending_ledge_landing: {
    tile_x: number;
    tile_y: number;
    dx: number;
    dy: number;
    direction: string;
  } | null = null;
  public _trainer_cutscene_active = false;
  public _active_trainer_sighting: TrainerSighting | null = null;
  public check_for_trainer_sightlines!: () => boolean;
  public _block_feedback_tracking = false;
  protected _last_block_feedback: BlockFeedbackDetails | null = null;
  public _ascii_overlay_cache_key: string | null = null;
  public _ascii_overlay_cached_viewport: string[] | null = null;
  public _ascii_overlay_cached_info: string[] | null = null;
  public _ascii_overlay_last_event_identity: MapEvents | null = null;
  public _ascii_overlay_last_event_counts: [number, number, number] | null = null;
  public _ascii_overlay_last_npc_positions: Array<[number, number, number]> = [];
  public _sprite_root = "";
  public _ledge_shadow_surface: InstanceType<typeof gameEngine.Surface> | null = null;
  public _field_move_animation_renderer: FieldMoveAnimationController | null = null;
  public _fade_overlay_color: [number, number, number] = [0, 0, 0];
  public _fade_alpha = 0;
  public _fade_start_alpha = 0;
  public _fade_end_alpha = 0;
  public _fade_steps_total = 0;
  public _fade_progress = 0;
  public _fade_active = false;
  public _white_fade_pending_clear = false;
  public _poison_overlay: InstanceType<typeof gameEngine.Surface> | null = null;
  public _poison_flash_remaining = 0;
  public _poison_overlay_alpha = 0;
  public _pending_poison_whiteout = false;
  public _debug_sightlines = false;
  public _palette_manager: NpcPaletteManager | null = null;
  public _npc_sprite_cache!: NpcSpriteCache;
  public movement_data: Record<string, string[]> = {};
  public player_sprite_id = "";
  public player_palette_id = 0;
  public player_animations: Record<string, SpriteAnimation> = {};
  public _multiplayer_remote_players: RemoteOverworldPlayer[] = [];
  public _multiplayer_remote_render_enabled = true;
  public _multiplayer_remote_crowd_view = false;
  public static readonly WALK_FRAMES = 8;
  public get WALK_FRAMES(): number {
    if (this._walk_frames_override !== null) {
      return this._walk_frames_override;
    }
    return this.game_state.wram.instant_mode ? 1 : OverworldEngine.WALK_FRAMES;
  }
  public set WALK_FRAMES(value: number) {
    const normalized = this._normalize_walk_frames(value);
    if (normalized === null) {
      return;
    }
    if (normalized === 1) {
      this._walk_frames_override = null;
      this.game_state.wram.instant_mode = true;
      return;
    }
    if (normalized === OverworldEngine.WALK_FRAMES) {
      this._walk_frames_override = null;
      this.game_state.wram.instant_mode = false;
      return;
    }
    this._walk_frames_override = normalized;
  }
  public get TURN_FRAMES(): number {
    return this.game_state.wram.instant_mode ? 1 : 4;
  }
  public readonly STEP_PIXELS = TILE_SIZE * 2;
  public readonly TILES_PER_COLLISION = 2;
  public get STEP_SPEED_PX(): number {
    return this.STEP_PIXELS / this.WALK_FRAMES;
  }

  private static readonly BG_EVENT_DIRECTION_MAP: Record<string, string> = {
    BGEVENT_UP: "up",
    BGEVENT_DOWN: "down",
    BGEVENT_LEFT: "left",
    BGEVENT_RIGHT: "right",
  };
  private static readonly BG_EVENT_CONDITIONAL_TYPES = new Set(["BGEVENT_IFSET", "BGEVENT_IFNOTSET"]);

  private static readonly OUTDOOR_ENVIRONMENTS = new Set(["ROUTE", "TOWN"]);

  protected _active_bg_event: BackgroundEvent | null;

  private static readonly SPECIAL_CALL_HANDLERS: Record<string, [string, string, boolean]> = {
    SPECIALCALL_POKERUS: ["PHONE_ELM", "ElmPhoneCallerScript", true],
    SPECIALCALL_ROBBED: ["PHONE_ELM", "ElmPhoneCallerScript", true],
    SPECIALCALL_ASSISTANT: ["PHONE_ELM", "ElmPhoneCallerScript", true],
    SPECIALCALL_WEIRDBROADCAST: ["PHONE_ELM", "ElmPhoneCallerScript", true],
    SPECIALCALL_SSTICKET: ["PHONE_ELM", "ElmPhoneCallerScript", false],
    SPECIALCALL_BIKESHOP: ["PHONE_OAK", "BikeShopPhoneCallerScript", false],
    SPECIALCALL_WORRIED: ["PHONE_MOM", "MomPhoneLectureScript", false],
    SPECIALCALL_MASTERBALL: ["PHONE_ELM", "ElmPhoneCallerScript", true],
  };

  public elevator_state: ElevatorRideStateMachine | null = null;
  public input_capture_active = false;

  protected _tile_to_pixels(tile_coordinate: number): number {
    return super._tile_to_pixels(tile_coordinate);
  }

  protected _object_should_spawn(npc: OverworldObject, { ignore_event_flag = true }: { ignore_event_flag?: boolean } = {}): boolean {
    return super._object_should_spawn(npc, { ignore_event_flag });
  }

  public _refresh_tileset_for_current_map(attributes: MapAttributes | null = null): void {
    super._refresh_tileset_for_current_map(attributes);
  }

  protected _apply_event_flag_update(event_name: string, value: boolean): void {
    super._apply_event_flag_update(event_name, value);
  }

  public _sync_player_state(): void {
    const footprint = this.TILES_PER_COLLISION - 1;
    const origin_x = this.player_x - footprint;
    const origin_y = this.player_y - footprint;
    this.player_px_x = this._tile_to_pixels(origin_x);
    this.player_px_y = this._tile_to_pixels(origin_y);
    this.target_px_x = this.player_px_x;
    this.target_px_y = this.player_px_y;
    this.game_state.wram.wXCoord = this.player_x;
    this.game_state.wram.wYCoord = this.player_y;
    this.game_state.wram.player_x = Math.trunc(this.player_x / METATILE_WIDTH);
    this.game_state.wram.player_y = Math.trunc(this.player_y / METATILE_WIDTH);
    this.game_state.wram.player_subtile_x = this.player_x % METATILE_WIDTH;
    this.game_state.wram.player_subtile_y = this.player_y % METATILE_WIDTH;

    this._update_player_overhead_flag();
    this._update_npc_overhead_flags();
  }

  protected _is_tile_grass(tile_x: number, tile_y: number): boolean {
    if (!this._map_has_tall_grass) {
      return false;
    }
    const map_obj = this.map;
    if (!map_obj) {
      return false;
    }

    const meta_x = Math.trunc(tile_x / METATILE_WIDTH);
    const meta_y = Math.trunc(tile_y / METATILE_WIDTH);

    if (meta_x < 0 || meta_y < 0 || meta_x >= map_obj.width || meta_y >= map_obj.height) {
      return false;
    }

    let metatile_index: number;
    try {
      metatile_index = getMetatileAt(map_obj, meta_x, meta_y);
    } catch {
      return false;
    }

    const tileset = this.tileset;
    if (!tileset) {
      return false;
    }

    if (metatile_index >= tileset.metatiles.length) {
      return false;
    }

    const metatile = tileset.metatiles[metatile_index];
    return metatile.collision.some((collision: number) => TALL_GRASS_COLLISIONS_SET.has(collision));
  }

  protected _refresh_map_environment_flags(npc_data_list: ObjectEvent[] | null): void {
    this._map_has_trainer_sightlines = this._detect_trainer_presence(npc_data_list);
    this._map_has_tall_grass = this._detect_map_grass();
  }

  protected _detect_trainer_presence(npc_data_list: ObjectEvent[] | null): boolean {
    if (!npc_data_list || !npc_data_list.length) {
      return false;
    }
    for (const entry of npc_data_list) {
      const object_type = String(entry.object_type ?? "").toUpperCase();
      if (object_type === "OBJECTTYPE_TRAINER") {
        return true;
      }
    }
    return false;
  }

  protected _detect_map_grass(): boolean {
    const map_obj = this.map;
    const tileset = this.tileset;
    const metatiles = tileset?.metatiles ?? null;
    if (!map_obj || !metatiles || !metatiles.length) {
      return false;
    }
    const used_ids = getMetatileIds(map_obj);
    if (!used_ids.length) {
      throw new Error(`Map '${this.current_map_name}' has no metatile IDs for grass detection.`);
    }
    for (const metatile_id of used_ids) {
      if (!(0 <= metatile_id && metatile_id < metatiles.length)) {
        throw new Error(
          `Map '${this.current_map_name}' references invalid metatile ${metatile_id} during grass detection.`,
        );
      }
      const collision = metatiles[metatile_id]?.collision ?? [];
      if (collision.some((value: number) => TALL_GRASS_COLLISIONS_SET.has(value))) {
        return true;
      }
    }
    return false;
  }

  protected _update_player_overhead_flag(): void {
    const player_object = this.player_object ?? null;
    if (!player_object) {
      return;
    }
    player_object.overhead = Boolean(
      this._map_has_tall_grass && this._is_tile_grass(this.player_x, this.player_y)
    );
  }

  protected _update_npc_overhead_flags(): void {
    const npcs = this.npcs ?? null;
    if (!npcs) {
      return;
    }

    const has_grass = this._map_has_tall_grass;
    for (const npc of npcs) {
      npc.overhead = has_grass && this._is_tile_grass(npc.x, npc.y);
    }
  }

  protected _handle_time_of_day_change(_previous: string, _current: string): void {
    const normalized = this._normalise_time_of_day_label?.(_current) ?? _current;
    if (normalized && this._grass_rustle?.set_time_of_day) {
      this._grass_rustle.set_time_of_day(normalized);
    }
    if (normalized && this._field_move_animation_renderer?.set_time_of_day) {
      this._field_move_animation_renderer.set_time_of_day(normalized);
    }
    this._refresh_tileset_for_current_map();
    this.refresh_map_sprites({ reload_standing: true, reload_walking: true });
  }

  public get player_state(): PlayerState {
    return this._player_state ?? PlayerState.NORMAL;
  }

  public set player_state(value: PlayerState | string | number) {
    const state = normalizePlayerState(value);
    this._player_state = state;
    const flag = flagForPlayerState(state);
    if (this.game_state) {
      this.game_state.wram.wPlayerState = flag;
    }
  }

  protected _handle_overworld_step(): boolean {
    this._daily_event_system.process({});
    if (this.event_manager) {
      this.event_manager.dispatch(new Event("player_step", { count: 1 }));
    }

    if (!this._player_events_enabled(PLAYEREVENTS_COUNT_STEPS)) {
      return false;
    }
    if (this._process_special_phone_call()) {
      return true;
    }
    if (this._process_repel_step()) {
      return true;
    }
    const result = this._step_events.process_step();
    if (result.egg_hatched) {
      if (this._logger?.debug) {
        this._logger.debug("Player hatched an egg while walking.");
      }
      this._start_egg_hatch_sequence(result.hatched_species);
      return true;
    }
    this._process_bug_contest_timer();
    if (result.poison_result) {
      this._apply_poison_damage(result.poison_result);
      return true;
    }
    return false;
  }

  protected _player_events_enabled(mask: number): boolean {
    const enabled = this.game_state.wram.wEnabledPlayerEvents;
    return Boolean(enabled & mask);
  }

  protected _discard_pending_wild_encounter(): void {
    const wild_encounters = this._wild_encounters ?? null;
    if (!wild_encounters) {
      return;
    }
    wild_encounters.skip_pending_step?.();
  }

  protected _player_events_blocked(): boolean {
    const moving = this.is_moving || this._turn_frames_remaining > 0 || this._ledge_jump_active;
    this.game_state.wram.wMapEventStatus = moving ? 1 : 0;
    if (moving) {
      return true;
    }
    if (this.player_movement_locked()) {
      return true;
    }
    if (this.script_tasks_active()) {
      return true;
    }
    const runner = this.script_runner;
    if (runner) {
      if (runner._script_stack?.length) {
        return true;
      }
      if (runner._awaiting_resume) {
        return true;
      }
    }
    const dialogue = this.dialogue;
    if (dialogue && (dialogue.active || dialogue.waiting_for_input)) {
      return true;
    }
    return false;
  }

  protected _process_player_events(): void {
    if (this._player_events_blocked()) {
      this._skip_wild_encounter_for_step = false;
      this._discard_pending_wild_encounter();
      return;
    }

    const trainerSightlinesTriggered = this._map_has_trainer_sightlines
      && (typeof this.check_for_trainer_sightlines === "function"
        ? this.check_for_trainer_sightlines()
        : TrainerSightlineMixin.prototype.check_for_trainer_sightlines.call(this));
    if (trainerSightlinesTriggered) {
      this._skip_wild_encounter_for_step = false;
      this._discard_pending_wild_encounter();
      return;
    }

    if (this._player_events_enabled(PLAYEREVENTS_WARPS_AND_CONNECTIONS)) {
      if (this.check_for_map_transition()) {
        this._skip_wild_encounter_for_step = false;
        this._discard_pending_wild_encounter();
        return;
      }
      if (this.check_for_warp_event()) {
        this._skip_wild_encounter_for_step = false;
        this._discard_pending_wild_encounter();
        return;
      }
    } else {
      pushDebugLog(`[events] Warps disabled (mask=${PLAYEREVENTS_WARPS_AND_CONNECTIONS}, enabled=${this.game_state.wram.wEnabledPlayerEvents})`);
    }

    if (this._player_events_enabled(PLAYEREVENTS_COORD_EVENTS)) {
      if (this.check_for_coord_events()) {
        this._skip_wild_encounter_for_step = false;
        this._discard_pending_wild_encounter();
        return;
      }
    }

    if (!this._player_events_enabled(PLAYEREVENTS_WILD_ENCOUNTERS)) {
      this._skip_wild_encounter_for_step = false;
      this._discard_pending_wild_encounter();
      return;
    }

    if (this._skip_wild_encounter_for_step) {
      this._skip_wild_encounter_for_step = false;
      this._discard_pending_wild_encounter();
      return;
    }

    this.check_for_wild_encounter();
  }

  protected _process_repel_step(): boolean {
    const wram = this.game_state.wram;
    let remaining = wram.repel_steps ?? 0;
    if (remaining <= 0) {
      return false;
    }
    remaining = Math.max(remaining - 1, 0);
    wram.repel_steps = remaining;
    if (remaining === 0) {
      this._show_field_move_text("RepelWoreOffText");
      return true;
    }
    return false;
  }

  protected _process_bug_contest_timer(): void {
    const flags = this.game_state.wram.engine_flags;
    if (!flags?.ENGINE_BUG_CONTEST_TIMER) {
      return;
    }
    const timer_active = check_bug_contest_timer(this.game_state);
    if (timer_active) {
      return;
    }
    if (this.current_map_name !== "NationalParkBugContest") {
      return;
    }
    this._finish_bug_contest_due_to_timeout();
  }

  protected _finish_bug_contest_due_to_timeout(): void {
    const runner = this.script_runner;
    if (!runner) {
      throw new Error("Bug contest sequel requires an active script runner.");
    }

    const handler = runner.standard_scripts?.BugContestResultsWarpScript;
    if (!handler) {
      throw new Error("BugContestResultsWarpScript handler missing when timer expired.");
    }

    handler(runner);
    const warp_command = new WarpCommand("ROUTE_36_NATIONAL_PARK_GATE", 0, 4);
    warp_command.runner = runner;
    const event_manager = this.event_manager;
    if (!event_manager) {
      throw new Error("Bug contest sequel requires an active event manager.");
    }
    warp_command.execute(this.game_state, event_manager, this);
  }

  protected _process_special_phone_call(): boolean {
    const queue = this.game_state.wram.scheduled_phone_calls ?? [];
    if (!queue.length) {
      return false;
    }

    let call_id = "";
    let call_index: number | null = null;
    for (let idx = 0; idx < queue.length; idx += 1) {
      const token = String(queue[idx] ?? "").trim();
      if (!token) {
        continue;
      }
      if (OverworldEngine.SPECIAL_CALL_HANDLERS[token.toUpperCase()]) {
        call_id = token;
        call_index = idx;
        break;
      }
    }

    if (!call_id) {
      if (this._logger?.info) {
        this._logger.info("Special phone call queue present but no valid id: %s", queue);
      }
      return false;
    }

    if (call_index !== null && call_index !== 0) {
      const entry = queue.splice(call_index, 1)[0];
      queue.unshift(entry);
    }

    if (this._logger?.info) {
      this._logger.info(
        "Processing special phone call: %s queue=%s map=%s runner_stack=%d awaiting=%d",
        call_id,
        [...queue],
        this.current_map_name,
        this.script_runner ? this.script_runner._script_stack?.length ?? -1 : -1,
        this.script_runner ? this.script_runner._awaiting_resume ?? 0 : 0,
      );
    }

    const handler = OverworldEngine.SPECIAL_CALL_HANDLERS[call_id.toUpperCase()];
    if (!handler) {
      this._logger?.debug?.(`No handler found for phone call: ${call_id}`);
      return false;
    }

    const [contact_id, script_name, outdoors_only] = handler;
    if (outdoors_only) {
      const environment = getMapEnvironment(this.current_map_name);
      if (!environment || !OverworldEngine.OUTDOOR_ENVIRONMENTS.has(environment.toUpperCase())) {
        if (this._logger?.info) {
          this._logger.info(
            "Phone call %s requires outdoors; environment=%s current_map=%s",
            call_id,
            environment,
            this.current_map_name,
          );
        }
        return false;
      }
    }

    if (!this.event_manager) {
      throw new Error("Special phone calls require an active event manager.");
    }

    const runner = this.script_runner;
    if (!runner || runner._script_stack?.length) {
      if (this._logger?.info) {
        this._logger.info(
          "Cannot process phone call %s: runner=%s stack_depth=%s awaiting=%s",
          call_id,
          runner,
          runner ? runner._script_stack?.length ?? null : null,
          runner ? runner._awaiting_resume ?? 0 : null,
        );
      }
      return false;
    }

    this._logger?.info?.(`Triggering phone call: ${call_id} -> ${contact_id}:${script_name}`);
    runner.variables = runner.variables ?? {};
    runner.variables.VAR_CALLERID = contact_id;
    runner.variables.VAR_SPECIALPHONECALL = call_id.toUpperCase();
    const overlay = this._phone_call_overlay;
    if (overlay) {
      overlay.show(contact_id);
    }
    try {
      if (typeof runner.run_phone_script !== "function") {
        throw new Error("Special phone calls require ScriptRunner.run_phone_script.");
      }
      runner.run_phone_script(script_name);
    } finally {
      this._maybe_hide_phone_overlay();
    }
    return true;
  }

  protected _maybe_hide_phone_overlay(): void {
    const overlay = this._phone_call_overlay;
    if (!overlay || !overlay.active) {
      return;
    }
    const dialogue = this.dialogue;
    if (!dialogue || !dialogue.active) {
      overlay.hide();
    }
  }

  protected _start_egg_hatch_sequence(species_id: string | null): void {
    const ui = this.ui;
    if (!ui || !ui.screen) {
      return;
    }
    this._egg_hatch_animation = new EggHatchAnimation(ui, {
      audioEngine: this.audio_engine ?? null,
      speciesId: species_id ?? "",
    });
    // Egg hatch animation starts immediately on construction.
    this._egg_hatch_display_name = this._format_species_display(species_id);
  }

  protected _trigger_hatch_text(display_name: string | null): void {
    const event_manager = this.event_manager;
    if (!event_manager) {
      return;
    }
    const name = display_name ?? "It";
    open_text(event_manager);
    show_text(event_manager, `${name} hatched from the EGG!`);
    wait_for_input(event_manager);
    this._hatch_text_pending = true;
  }

  protected _finalize_hatch_sequence(): void {
    const dialogue = this.dialogue;
    if (!dialogue) {
      return;
    }
    dialogue.waiting_for_input = false;
    const pending_scripts = dialogue.pending_script_waits_count ?? 0;
    dialogue.clear_script_waits?.();
    if (pending_scripts) {
      dialogue.script_runner_instance?.resume?.();
    }
  }

  protected _format_species_display(species_id: string | null): string {
    if (!species_id) {
      return "It";
    }
    return String(species_id).replace(/_/g, " ");
  }

  protected _restore_saved_scene_state(): void {
    const map_name = this.current_map_name ?? "";
    if (!map_name) {
      return;
    }
    this.script_runner?._ensure_map_scene_initialized?.(map_name);
    let scene_name = this.game_state.wram.map_scenes?.[map_name] ?? "";
    if (!scene_name) {
      const stored_scene = this.game_state.wram.scene_name ?? "";
      if (stored_scene) {
        scene_name = stored_scene;
        this.game_state.wram.map_scenes[map_name] = stored_scene;
        if (!(map_name in this.game_state.wram.map_scene_indices)) {
          const order = this.data_loader?.map_scene_order?.get?.(map_name) ?? [];
          const index = order.indexOf(stored_scene);
          this.game_state.wram.map_scene_indices[map_name] = index >= 0 ? index : 0;
        }
      }
    }
    if (scene_name) {
      this.game_state.wram.scene_name = scene_name;
    }
  }

  protected _resolve_initial_metadata() {
    const metadata = getMapMetadataByGroup(
      this.game_state.wram.wMapGroup,
      this.game_state.wram.wMapNumber,
    );
    if (!metadata) {
      throw new Error(
        `Unknown initial map identifiers: group=${this.game_state.wram.wMapGroup}, map=${this.game_state.wram.wMapNumber}`
      );
    }
    return metadata;
  }

  constructor(
    game_state: GameState,
    data_loader: DataLoader,
    event_manager: EventManager,
    tileset: OverworldTilesetLike,
    audio_engine: AudioEngine,
    ui: unknown,
    options: { suppressInitialMapEntryEffects?: boolean; suppressInitialMapMusic?: boolean } = {},
  ) {
    super();
    (this as unknown as { _initialize_field_move_state?: () => void })._initialize_field_move_state?.();
    this._logger = console;
    this._suppress_initial_map_entry_effects_once = Boolean(options.suppressInitialMapEntryEffects);
    this._suppress_initial_map_music_once = Boolean(options.suppressInitialMapMusic);
    this.game_state = game_state;
    this.profiler = null;
    this._debug_inputs_enabled = false;
    const metadata = this._resolve_initial_metadata();
    const uiContext = ui as OverworldUIContext;
    this.ui = uiContext as BaseUI;
    this.screen = uiContext?.screen ?? null;
    this.pokepic_overlay = new PokePicOverlay(uiContext);
    this._egg_hatch_animation = null;
    this._egg_hatch_display_name = null;
    this._hatch_text_pending = false;
    this._phone_call_overlay = new PhoneCallOverlay(uiContext, () => {
      return resolveTextboxFrameRenderId(this.game_state?.sram?.options?.frame, 1);
    });
    this._player_state = PlayerState.NORMAL;
    const initial_state = playerStateFromFlag(this.game_state.wram?.wPlayerState ?? 0);
    this.player_state = initial_state;
    this.data_loader = data_loader;
    this.data_loader?.ensure_overworld_data?.({ map_name: metadata.name });
    this.event_manager = event_manager;
    this._script_task_queue = [] as ScriptTask[];
    this._active_script_task = null;
    this._blocking_task_count = 0;
    this._blocking_movement_lock_active = false;
    this._pending_event_flag_updates = [] as PendingEventFlagUpdate[];
    this._map_has_tall_grass = true;
    this._map_has_trainer_sightlines = true;
    this._npc_blueprints = new Map();
    this.current_map_name = "";
    this.item_system = new ItemSystem(game_state, data_loader);
    this.script_runner = new ScriptRunnerImpl(
      game_state,
      event_manager,
      data_loader,
      this,
    );
    this._blocked_coord_events = new Set();
    this.elevator_state = new ElevatorRideStateMachine();
    this._step_events = new StepEventSystem(game_state, { day_care: this.script_runner?.day_care ?? null });
    this._daily_event_system = new DailyEventSystem(game_state);
    this._overworld_time_system = new OverworldTimeSystem(game_state, {
      dailyEventSystem: this._daily_event_system,
      onTimeOfDayChange: (previous: string, current: string) => this._handle_time_of_day_change(previous, current),
    });
    this._town_map_overlay = new TownMapOverlay(uiContext, game_state, {
      script_runner: this.script_runner,
      lock_movement: () => this.lock_player_movement(),
      unlock_movement: () => this.unlock_player_movement(),
    });
    if (event_manager && this._town_map_overlay) {
      this._town_map_overlay.register(event_manager);
    }
    const [screenWidth, screenHeight] = this.screen?.get_size?.() ?? [0, 0];
    this._fade_overlay = new gameEngine.Surface(screenWidth, screenHeight);
    if (this._fade_overlay) {
      this._fade_overlay.fill([0, 0, 0, 255]);
    }
    this._fade_overlay_color = [0, 0, 0] as [number, number, number];
    this._fade_alpha = 0;
    this._fade_start_alpha = 0;
    this._fade_end_alpha = 0;
    this._fade_steps_total = 0;
    this._fade_progress = 0;
    this._fade_active = false;
    this._white_fade_pending_clear = false;
    this._poison_overlay = new gameEngine.Surface(screenWidth, screenHeight);
    if (this._poison_overlay) {
      this._poison_overlay.fill(POISON_OVERLAY_COLOR);
    }
    this._poison_flash_remaining = 0;
    this._poison_overlay_alpha = 0;
    this._pending_poison_whiteout = false;
    this._debug_sightlines = false;
    this._wild_encounters = new WildEncounterManager(
      game_state,
      data_loader,
      event_manager,
    );
    this._skip_wild_encounter_for_step = false;
    this._emote_sprite_cache = new EmoteSurfaceCache();
    this.dialogue = new FieldDialogueManager(uiContext, game_state, this.script_runner, audio_engine, {
      mask_text: () => this._should_mask_dialogue_text(),
    });

    const show_text_callback = (event: Event, state: GameState) => {
      if (event.name === "show_text") {
        this._handle_text_visibility_event(event, state);
      }
    };

    this.dialogue.register_event_callback?.(show_text_callback);
    this.fly_prompt_class = SelectionPrompt;
    this._fly_menu_selector = null;
    this._field_move_confirm_callback = null;
    this._field_move_auto_advance = false;
    this._dialogue_controller = new DialogueEventController(event_manager, this.dialogue);
    this._dialogue_controller?.register();
    this._last_pending_script_status = null;
    this._last_wait_status_message = null;
    if (!this._suppress_initial_map_entry_effects_once && !this.game_state.wram.event_flags?.EVENT_INITIALIZED_EVENTS) {
      this.script_runner.run("InitializeEventsScript");
    }
    const initial_metadata = this._resolve_initial_metadata();
    this.current_map_name = initial_metadata.name;
    this._restore_saved_scene_state();
    const metatile_x = game_state.wram.player_x ?? 0;
    const metatile_y = game_state.wram.player_y ?? 0;
    const subtile_x = game_state.wram.player_subtile_x ?? 0;
    const subtile_y = game_state.wram.player_subtile_y ?? 0;
    let tile_x = game_state.wram.wXCoord ?? this._tile_from_components(metatile_x, subtile_x);
    let tile_y = game_state.wram.wYCoord ?? this._tile_from_components(metatile_y, subtile_y);
    const max_tile_x = initial_metadata.width * METATILE_WIDTH - 1;
    const max_tile_y = initial_metadata.height * METATILE_WIDTH - 1;
    let min_tile = this.TILES_PER_COLLISION - 1;
    min_tile = Math.max(min_tile, 0);
    tile_x = Math.max(min_tile, Math.min(tile_x, max_tile_x));
    tile_y = Math.max(min_tile, Math.min(tile_y, max_tile_y));
    this.player_x = tile_x;
    this.player_y = tile_y;
    this._sync_player_state();
    this.prev_player_x = this.player_x;
    this.prev_player_y = this.player_y;
    this.player_direction = "down";
    this.tileset = tileset;
    this._tileset_cache = new Map<string, OverworldTilesetLike>();
    const mapAttrs = this._current_map_attributes?.() ?? null;
    const timeOfDay = this._normalise_time_of_day_label?.(mapAttrs?.time_of_day) ?? "day";
    this._tileset_cache.set(`${tileset.tilesetName}:${timeOfDay}`, tileset);
    this._map_sign = null;
    this.audio_engine = audio_engine ?? new AudioEngine();
    this.audio_controller = new OverworldAudioController(this.game_state, this.audio_engine);
    this.movement_data = (data_loader as DataLoaderLike)?.movement_data ?? {};
    this.map = new OverworldMap(this.current_map_name, initial_metadata.width, initial_metadata.height);
    this.map_surface = null;
    this.priority_surface = null;
    this._composite_surface = null;
    this._composite_priority_surface = null;
    this._composite_origin = [0, 0];
    this._composite_segments = [];
    this._palette_manager = new NpcPaletteManager();
    this._npc_sprite_cache = new NpcSpriteCache(10, { palette_manager: this._palette_manager });
    this._grass_rustle = null;
    this.npcs = [] as OverworldObject[];
    this._map_prefix_lookup = {} as Record<string, string>;
    const npc_data = data_loader?.npc_data ?? {};
    for (const name of Object.keys(npc_data)) {
      this._map_prefix_lookup[name.replace(/\s+/g, "").toUpperCase()] = name;
    }
    const mapPrefixEntries = Object.entries(this._map_prefix_lookup) as Array<[string, string]>;
    this._map_prefixes = mapPrefixEntries.sort((a, b) => b[0].length - a[0].length);
    this._npc_index_lookup = new Map();
    this.follower = null;
    this.leader = null;
    this._follower_step_queue = [] as string[];
    this._follower_queue_length = 0;
    this._follower_movement_task = null;
    this._follower_buffer_step = null;
    this._follower_release_tokens = 0;
    this._active_follower_target = null;
    this._pending_follower_origin = null;
    this._last_step_direction = null;
    this._active_emotes = new Map();
    this.player_object = new PlayerCharacter(this as unknown as OverworldType);
    this.is_moving = false;
    this._movement_lock_count = 0;
    this._set_map_events(MapEvents.parse({}));
    this._active_bg_event = null;
    this._warp_cooldown = 0;
    this._active_warp_tile = null;
    this._active_coord_event = null;
    this._text_lock_active = false;
    this._earthquake_remaining_frames = 0;
    this._earthquake_intensity = 0;
    this._earthquake_phase = 0;
    this._earthquake_offset = [0, 0];
    this._held_directions = new Map();
    this._queued_direction = null;
    this._turning_direction = null;
    this._turn_frames_remaining = 0;
    this._turn_should_force_step = false;
    this._pending_auto_step = null;
    this.step_progress_px = 0;
    this.step_dx_px = 0;
    this.step_dy_px = 0;
    this._current_step_distance_px = this.STEP_PIXELS;
    this._current_step_speed_px = this.STEP_SPEED_PX;
    this._ledge_jump_active = false;
    this._ledge_jump_total_distance_px = 0;
    this._ledge_jump_animation_progress_px = 0;
    this._pending_ledge_landing = null;
    this._trainer_cutscene_active = false;
    this._active_trainer_sighting = null;
    this._last_block_feedback = null;
    this._block_feedback_tracking = false;
    this._ascii_overlay_cache_key = null;
    this._ascii_overlay_cached_viewport = null;
    this._ascii_overlay_cached_info = null;
    this._ascii_overlay_last_event_identity = null;
    this._ascii_overlay_last_event_counts = null;
    this._ascii_overlay_last_npc_positions = [] as Array<[number, number, number]>;
    this.target_tile_x = this.player_x;
    this.target_tile_y = this.player_y;
    this._sprite_root = getAssetPath("gfx", "sprites");
    this._ledge_shadow_surface = null;
    this._coord_skip_log = {} as Record<string, string>;
    this._coord_miss_log = {} as Record<string, [number, number]>;
    this._npc_autonomous_controller = new NpcAutonomousController(
      this as unknown as ConstructorParameters<typeof NpcAutonomousController>[0],
    );
    // ASM only sets SHOWN_MAP_NAME_SIGN in specific entry flows before OverworldLoop;
    // preserve the caller-provided WRAM flag instead of suppressing every fresh boot.
    bind_tile_animation_timer(game_state);
    this._tileset_animator = new TilesetAnimationController(this, game_state);
    this.load_map(this.current_map_name);

    this.player_sprite_id = "chris";
    this.player_palette_id = 0;
    this.player_animations = {};
    update_player_sprite(this.game_state, { overworld: this as any });
    this._field_move_animation_renderer = null;
    this._register_event_listeners();
  }

  public async init_assets(): Promise<void> {
    await this._preload_ledge_shadow_surface();
    this._ledge_shadow_surface = this._load_ledge_shadow_surface();
    this._map_sign = await MapNameSignController.create(this.game_state);
    this._map_sign?.on_map_loaded?.(this.current_map_name);
    const initialTimeOfDay =
      this._normalise_time_of_day_label?.(this.game_state?.wram?.time_of_day ?? "day") ?? "day";
    this._grass_rustle = new GrassRustleController({
      palette_manager: this._palette_manager,
      time_of_day: initialTimeOfDay,
    });
    this._field_move_animation_renderer = new FieldMoveAnimationController(this.game_state, {
      time_of_day: initialTimeOfDay,
    });
  }

  public handle_input(event: GameEngineEvent): void {
    if (this._town_map_overlay?.handle_input?.(event)) {
      return;
    }
    const handler = OverworldInputMixin.prototype as OverworldInputHandler;
    handler.handle_input.call(this, event);
  }

  public handleInput(event: GameEngineEvent): void {
    this.handle_input(event);
  }

  public start_map_music(): void {
    if (!this.audio_controller) {
      let token = defaultMusicTokenForMap(this.current_map_name);
      if (this.player_state === PlayerState.BIKE) {
        token = "MUSIC_BICYCLE";
      } else if ([PlayerState.SURF, PlayerState.SURF_PIKA].includes(this.player_state)) {
        token = "MUSIC_SURF";
      }
      if (String(this.game_state?.wram?.wMapMusic ?? "").trim() === token) {
        return;
      }
      if (typeof this.audio_engine?.playMusic === "function") {
        this.audio_engine.playMusic(token, "map");
      } else {
        this.audio_engine?.play_music?.(token, "map");
      }
      if (this.game_state?.wram) {
        this.game_state.wram.wMapMusic = token;
      }
      return;
    }
    try {
      this.audio_controller.requestMapMusic(this.current_map_name, this.player_state);
    } catch (exc) {
      this._logger?.debug?.("Skipping map music for %s: %s", this.current_map_name, exc);
    }
  }

  public restart_map_music(): void {
    if (!this.audio_controller) {
      if (this.game_state?.wram?.dont_restart_map_music) {
        this.game_state.wram.dont_restart_map_music = false;
        return;
      }
      this.audio_engine?.restartMapMusic();
      return;
    }
    this.audio_controller.restartMapMusic();
  }

  public restartMapMusic(): void {
    this.restart_map_music();
  }

  public hasTemporaryMusicOverride(): boolean {
    const controller = this.audio_controller as
      | { hasTemporaryMusicOverride?: () => boolean }
      | null
      | undefined;
    if (typeof controller?.hasTemporaryMusicOverride === "function") {
      return controller.hasTemporaryMusicOverride();
    }
    const engine = this.audio_engine as
      | { hasTemporaryMusicOverride?: () => boolean }
      | null
      | undefined;
    return Boolean(engine?.hasTemporaryMusicOverride?.());
  }

  public requestMusic(token: string, role: string = "general"): void {
    this.audio_controller.requestMusic(token, role);
  }

  public requestEncounterMusic(trainerClass: string): void {
    this.audio_controller.requestEncounterMusic(trainerClass);
  }

  public startRadioChannel(station: string, durationFrames: number = 0): void {
    this.audio_controller.startRadioChannel(station, durationFrames);
  }

  public stopRadioChannel(): void {
    this.audio_controller.stopRadioChannel();
  }

  public playCry(cryId: string): void {
    this.audio_controller.playCry(cryId);
  }

  public fadeToMusic(token: string, durationFrames: number, role: string = "general"): void {
    this.audio_controller.fadeToMusic(token, durationFrames, role);
  }

  protected _apply_map_entry_player_state(): void {
    const wram = this.game_state?.wram;
    const bikeFlags = syncBikeFlags(wram);
    const previous = this.player_state;
    let next = previous;
    if (bikeFlags & _BIKEFLAG_ALWAYS_ON_BIKE) {
      next = PlayerState.BIKE;
    } else if (previous === PlayerState.BIKE && this._map_disallows_bike()) {
      next = PlayerState.NORMAL;
    }
    if (next === previous) {
      return;
    }
    this.player_state = next;
    update_player_sprite(this.game_state, { overworld: this as any });
  }

  protected _map_disallows_bike(): boolean {
    const environment = String(getMapEnvironment(this.current_map_name) ?? "").trim().toUpperCase();
    return _NO_BIKE_ENVIRONMENTS.has(environment);
  }

  protected _register_event_listeners(): void {
    if (!this.event_manager) {
      return;
    }

    for (const event_name of ["open_text", "wait_for_input", "prompt_yes_no", "prompt_selection", "close_text"]) {
      this.event_manager.on(event_name, (event: Event, state: GameState) => this._handle_text_visibility_event(event, state));
    }
    this.event_manager.on("field_move_animation", (event: Event, state: GameState) => this._handle_field_move_animation_event(event, state));
  }

  public suspend_dialogue_events(): void {
    const controller = this._dialogue_controller ?? null;
    if (controller) {
      controller.suspend();
    }
    if (this._text_lock_active) {
      this.unlock_player_movement();
      this._text_lock_active = false;
    }
  }

  public resume_dialogue_events(): void {
    const controller = this._dialogue_controller ?? null;
    if (controller) {
      controller.resume();
    }
  }

  protected _handle_text_visibility_event(event: Event, state: GameState): void {
    if (["open_text", "show_text", "wait_for_input", "prompt_yes_no", "prompt_selection"].includes(event.name)) {
      if (!this._text_lock_active) {
        this.stop_player_movement();
        this.lock_player_movement();
        this._text_lock_active = true;
      }
    } else if (event.name === "close_text") {
      if (this._text_lock_active) {
        this.unlock_player_movement();
        this._text_lock_active = false;
      }
      const joypad = state?.hram?.joypad ?? this.game_state?.hram?.joypad;
      if (joypad && (joypad.hJoyDown & JOY_A) !== 0) {
        this._ignore_a_until_release = true;
      }
      if (this._pending_poison_whiteout) {
        this._perform_poison_whiteout();
      }
    }
  }

  protected _should_mask_dialogue_text(): boolean {
    const mapName = this.current_map_name ?? "";
    if (!mapName) {
      return false;
    }
    const attributes = this._current_map_attributes?.() ?? null;
    if (!attributes?.time_of_day) {
      return false;
    }
    const label = this._normalise_time_of_day_label?.(attributes.time_of_day) ?? "";
    if (label !== "dark") {
      return false;
    }
    const wram = this.game_state?.wram;
    if (!wram) {
      return false;
    }
    if (getBooleanFlag(wram.engine_flags, "STATUSFLAGS_FLASH")) {
      return false;
    }
    if (getBooleanFlag(wram.flash_active_maps, mapName)) {
      return false;
    }
    return true;
  }

  protected _handle_field_move_animation_event(event: Event, _state: GameState): void {
    const renderer = this._field_move_animation_renderer;
    if (!renderer) {
      return;
    }
    const data = event.data ?? {};
    const animation = String(data.animation ?? "").toLowerCase();
    const variant = String(data.variant ?? animation).toLowerCase();
    const tile_x = Number(data.x ?? 0);
    const tile_y = Number(data.y ?? 0);
    const phase = data.phase;
    const direction = String(data.direction ?? "down").toLowerCase();
    const tileset_animator = this._tileset_animator;
    if (tileset_animator && animation === "whirlpool") {
      if (phase === "start") {
        tileset_animator.set_whirlpool_active?.(true);
      } else if (phase === "complete") {
        tileset_animator.set_whirlpool_active?.(false);
      }
    }
    if (phase === "start") {
      renderer.start(animation, variant, tile_x, tile_y, { direction });
    } else if (phase === "complete") {
      renderer.complete(animation, tile_x, tile_y);
    }
  }

  protected _apply_poison_damage(result: PoisonDamageResult | null = null): void {
    const party = this.game_state.sram?.party?.pokemon ?? [];
    const poison_result = result ?? applyPoisonToParty(party);
    const { damagedNames, faintedNames, survivors } = normalizePoisonResult(poison_result, party);
    if (!damagedNames.length && !faintedNames.length) {
      return;
    }
    this._start_poison_flash();
    this._play_poison_sound();
    if (faintedNames.length) {
      this._announce_poison_faints(faintedNames);
      if (!survivors.length) {
        this._schedule_poison_whiteout();
      }
    }
  }

  protected _play_poison_sound(): void {
    const engine = this.audio_engine;
    if (!engine) {
      return;
    }
    try {
      if (engine.play_sound) {
        engine.play_sound("SFX_POISON");
      } else if (engine.playSound) {
        engine.playSound("SFX_POISON");
      }
    } catch (exc) {
      this._logger?.debug?.("Poison SFX unavailable: %s", exc);
    }
  }

  protected _announce_poison_faints(names: string[]): void {
    if (!names.length || !this.event_manager) {
      return;
    }
    const message = this._poison_faint_message(names);
    open_text(this.event_manager);
    show_text(this.event_manager, message, { auto_close_after_wait: true });
    wait_for_input(this.event_manager);
  }

  protected _poison_faint_message(names: string[]): string {
    if (names.length === 1) {
      return `${names[0]} fainted!`;
    }
    if (names.length === 2) {
      return `${names[0]} and ${names[1]} fainted!`;
    }
    return `${names.slice(0, -1).join(", ")}, and ${names[names.length - 1]} fainted!`;
  }

  protected _schedule_poison_whiteout(): void {
    if (!this.event_manager) {
      this._pending_poison_whiteout = true;
      return;
    }
    this._pending_poison_whiteout = true;
  }

  protected _perform_poison_whiteout(): void {
    if (!this._pending_poison_whiteout) {
      return;
    }
    const event_manager = this.event_manager;
    if (!event_manager) {
      return;
    }
    this._pending_poison_whiteout = false;
    this.game_state.wram.battle_result = 1;
    event_manager.dispatch(new Event("battle_complete", { result: 1 }));
  }

  public get_movement_data(movement_data_label: string, parentScript?: string | null): string[] {
    if (this.movement_data?.[movement_data_label]) {
      return this.movement_data[movement_data_label];
    }

    const script = this.data_loader?.get_script?.(
      movement_data_label,
      parentScript ?? undefined
    );
    if (!script) {
      throw new Error(`Unknown movement data label '${movement_data_label}'`);
    }

    const formatted: string[] = [];
    for (const entry of script) {
      const command = entry.command ?? "";
      if (!command) {
        continue;
      }
      const raw_args = entry.args ?? "";
      const args = Array.isArray(raw_args)
        ? raw_args.map((arg) => String(arg)).filter(Boolean).join(" ")
        : String(raw_args);
      const combined = [String(command).toLowerCase(), args.trim()].filter(Boolean).join(" ");
      formatted.push(combined);
    }
    return formatted;
  }

  protected _create_player_animations(): Record<string, SpriteAnimation> {
    const time_of_day = this.game_state.wram?.time_of_day ?? "day";
    return this._npc_sprite_cache.instantiate(this.player_sprite_id, this.player_palette_id, time_of_day);
  }

  public reload_sprites_without_palette_changes({ reload_standing = true, reload_walking = true } = {}): void {
    const previous_player_anims = this.player_animations ?? null;
    let animations = this._create_player_animations();
    if (!reload_standing || !reload_walking) {
      const merge = this._preserve_animation_frames;
      if (typeof merge === "function") {
        const merged: Record<string, SpriteAnimation> = {};
        for (const [direction, animation] of Object.entries(animations)) {
          const existing = previous_player_anims ? previous_player_anims[direction] : null;
          merged[direction] = merge(existing, animation, { reload_standing, reload_walking });
        }
        animations = merged;
      }
    }
    this.player_animations = animations;

    const previous_by_index = new Map(this.npcs.map((npc: OverworldObject) => [npc.objectIndex, npc]));
    for (const npc of this.npcs) {
      this._initialise_npc_object(npc, {
        previous: previous_by_index.get(npc.objectIndex) ?? null,
        reload_standing,
        reload_walking,
      });
    }

    const follower = this.follower ?? null;
    if (follower && "objectIndex" in follower && !this.npcs.includes(follower as OverworldObject)) {
      const npcFollower = follower as OverworldObject;
      this._initialise_npc_object(npcFollower, {
        previous: previous_by_index.get(npcFollower.objectIndex) ?? null,
        reload_standing,
        reload_walking,
      });
    }
  }

  public get_facing_tile_coords(): [number, number] {
    let x = this.player_x;
    let y = this.player_y;
    if (this.player_direction === "up") {
      y -= this.TILES_PER_COLLISION;
    } else if (this.player_direction === "down") {
      y += this.TILES_PER_COLLISION;
    } else if (this.player_direction === "left") {
      x -= this.TILES_PER_COLLISION;
    } else if (this.player_direction === "right") {
      x += this.TILES_PER_COLLISION;
    }
    return [x, y];
  }

  public use_key_item(item_name: string): boolean | Promise<boolean> {
    if (!item_name) {
      return false;
    }
    const canonical = canonicalKeyItemName(item_name);
    if (canonical === "TOWNMAP") {
      return this._use_town_map();
    }
    if (canonical === "COINCASE") {
      return this._use_coin_case();
    }
    if (canonical === "BLUECARD") {
      return this._use_blue_card();
    }
    if (canonical === "BICYCLE") {
      return this._use_bicycle();
    }
    if (["OLDROD", "GOODROD", "SUPERROD"].includes(canonical)) {
      return this.handle_fishing?.(scriptKeyItemName(canonical)) ?? false;
    }
    if (canonical === "BASEMENTKEY") {
      return this._use_basement_key();
    }
    if (canonical === "CARDKEY") {
      return this._use_card_key();
    }
    if (canonical === "ITEMFINDER") {
      return this._use_itemfinder();
    }
    if (canonical === "SQUIRTBOTTLE") {
      return this._use_squirtbottle();
    }
    return false;
  }

  protected _canonical_key_item_name(item_name: string): string {
    return canonicalKeyItemName(item_name);
  }

  protected _script_key_item_name(canonical: string): string {
    return scriptKeyItemName(canonical);
  }

  protected _use_town_map(): boolean {
    const event_manager = this.event_manager;
    if (!event_manager) {
      return false;
    }
    event_manager.dispatch(new Event("show_town_map", { source: "key_item" }));
    return true;
  }

  protected _use_coin_case(): boolean {
    const event_manager = this.event_manager;
    if (!event_manager) {
      return false;
    }
    const coins = Math.max(0, Number(this.game_state?.sram?.coins ?? 0));
    event_manager.dispatch(
      new Event("show_coin_case_balance", {
        source: "key_item",
        overlay: {
          width: 7,
          height: 1,
          x: 11,
          y: 0,
          label: "COIN",
          value: coins,
        },
      }),
    );
    return true;
  }

  protected _use_blue_card(): boolean {
    const event_manager = this.event_manager;
    if (!event_manager) {
      return false;
    }
    const balance = Math.max(0, Math.min(30, Number(this.game_state?.wram?.blue_card_balance ?? 0)));
    event_manager.dispatch(
      new Event("show_blue_card_balance", {
        source: "key_item",
        overlay: {
          width: 7,
          height: 1,
          x: 11,
          y: 0,
          label: "POINT",
          value: balance,
        },
      }),
    );
    return true;
  }

  protected _use_bicycle(): boolean | Promise<boolean> {
    if (!this._can_toggle_bicycle_here()) {
      return false;
    }
    const finishBicycleUse = async (
      textLabel: string,
      options: { restartMapMusic?: boolean } = {},
    ): Promise<boolean> => {
      const showTextAsync = this._show_field_move_text_async as
        | ((label: string, actorName?: string | null) => Promise<void>)
        | undefined;
      if (typeof showTextAsync === "function") {
        await showTextAsync.call(this, textLabel);
      } else {
        this._show_field_move_text(textLabel);
      }
      if (options.restartMapMusic) {
        this.start_map_music();
      }
      return true;
    };
    const bikeFlags = syncBikeFlags(this.game_state?.wram);
    if (this.player_state === PlayerState.NORMAL) {
      this.player_state = PlayerState.BIKE;
      update_player_sprite(this.game_state, { overworld: this as any });
      return finishBicycleUse("_GotOnBikeText", { restartMapMusic: true });
    }
    if (this.player_state !== PlayerState.BIKE) {
      return false;
    }
    if (bikeFlags & _BIKEFLAG_ALWAYS_ON_BIKE) {
      return finishBicycleUse("_CantGetOffBikeText");
    }
    this.player_state = PlayerState.NORMAL;
    update_player_sprite(this.game_state, { overworld: this as any });
    return finishBicycleUse("_GotOffBikeText", { restartMapMusic: true });
  }

  protected _can_toggle_bicycle_here(): boolean {
    const environment = String(getMapEnvironment(this.current_map_name) ?? "").trim().toUpperCase();
    if (!_BIKE_ALLOWED_ENVIRONMENTS.has(environment)) {
      return false;
    }
    const permission = this._current_tile_permission();
    if (permission === null || permission === undefined) {
      return false;
    }
    // ASM: BikeFunction.CheckEnvironment requires FLOOR_TILE (low nibble 0).
    return (permission & 0x0f) === 0;
  }

  protected _use_basement_key(): boolean {
    if (this.current_map_name !== "GoldenrodUnderground") {
      return false;
    }
    if (!this.script_runner) {
      return false;
    }
    const [x, y] = this.get_facing_tile_coords();
    if (x !== 22 || y !== 10) {
      return false;
    }
    this.script_runner.run("BasementDoorScript");
    return true;
  }

  protected _use_card_key(): boolean {
    if (this.current_map_name !== "RadioTower3F") {
      return false;
    }
    let facing_direction: FacingDirection;
    try {
      facing_direction = facingDirectionFromString(this.player_direction);
    } catch {
      return false;
    }
    if (facing_direction !== FacingDirection.UP) {
      return false;
    }
    const [x, y] = this.get_facing_tile_coords();
    if (x !== 18 || y !== 6) {
      return false;
    }
    if (!this.script_runner) {
      return false;
    }
    this.script_runner.run("CardKeySlotScript");
    return true;
  }

  protected _use_itemfinder(): boolean | Promise<boolean> {
    const event_manager = this.event_manager;
    if (!event_manager) {
      return false;
    }
    const hidden_event = this._find_hidden_item_event();
    const text_label = hidden_event ? "ItemfinderItemNearbyText" : "ItemfinderNopeText";
    if (hidden_event) {
      this._play_itemfinder_sound();
    }
    return this._display_itemfinder_text(text_label).then(() => true);
  }

  protected _find_hidden_item_event(): BackgroundEvent | null {
    const map_events = this._map_events ?? null;
    if (!map_events) {
      return null;
    }
    const data_loader = this.data_loader ?? null;
    if (!data_loader) {
      return null;
    }
    const player_block_x = Math.trunc(this.player_x / this.TILES_PER_COLLISION);
    const player_block_y = Math.trunc(this.player_y / this.TILES_PER_COLLISION);
    const x_margin = Math.trunc(SCREEN_WIDTH_TILES / 4);
    const y_margin = Math.trunc(SCREEN_HEIGHT_TILES / 4);
    const half_width = Math.trunc(SCREEN_WIDTH_TILES / 2);
    const half_height = Math.trunc(SCREEN_HEIGHT_TILES / 2);
    for (const event of map_events.bg_events ?? []) {
      if (String(event.event_type ?? "").toUpperCase() !== "BGEVENT_ITEM") {
        continue;
      }
      const script_key = String(event.script ?? "").split(";", 1)[0].trim();
      if (!script_key) {
        continue;
      }
      const event_flag = (data_loader as DataLoaderLike | null)?.get_hidden_item_event_flag?.(script_key) ?? null;
      if (!event_flag) {
        continue;
      }
      if (this.game_state.wram.event_flags?.[event_flag]) {
        continue;
      }
      if (!this._event_in_itemfinder_range(
        event,
        player_block_x,
        player_block_y,
        x_margin,
        y_margin,
        half_width,
        half_height,
      )) {
        continue;
      }
      return event;
    }
    return null;
  }

  protected _event_in_itemfinder_range(
    event: BackgroundEvent,
    player_block_x: number,
    player_block_y: number,
    x_margin: number,
    y_margin: number,
    half_width: number,
    half_height: number,
  ): boolean {
    const dx = player_block_x + x_margin - event.x;
    if (dx < 0 || dx >= half_width) {
      return false;
    }
    const dy = player_block_y + y_margin - event.y;
    if (dy < 0 || dy >= half_height) {
      return false;
    }
    return true;
  }

  protected async _display_itemfinder_text(label: string): Promise<void> {
    const event_manager = this.event_manager;
    if (!event_manager) {
      return;
    }
    const text = this._resolve_itemfinder_text(label);
    if (!text) {
      return;
    }
    open_text(event_manager);
    show_text(event_manager, text);
    await this._wait_for_dialogue_render_async();
    wait_for_input(event_manager);
    await this._wait_for_dialogue_ack_async();
    close_text(event_manager);
    await this._wait_for_dialogue_closed_async();
  }

  protected _resolve_itemfinder_text(label: string): string {
    const text = this.data_loader?.get_text?.(label) ?? null;
    if (text) {
      return text;
    }
    throw new Error(`Missing ASM itemfinder text for label '${label}'.`);
  }

  protected _play_itemfinder_sound(): void {
    const audio_engine = this.audio_engine;
    if (!audio_engine) {
      return;
    }
    for (let i = 0; i < 4; i += 1) {
      if (audio_engine.play_sound) {
        audio_engine.play_sound("SFX_SECOND_PART_OF_ITEMFINDER");
        audio_engine.play_sound("SFX_TRANSACTION");
      } else if (audio_engine.playSound) {
        audio_engine.playSound("SFX_SECOND_PART_OF_ITEMFINDER");
        audio_engine.playSound("SFX_TRANSACTION");
      }
    }
  }

  protected _use_squirtbottle(): boolean | Promise<boolean> {
    if (this._can_use_squirtbottle_on_facing_object()) {
      const script_runner = this.script_runner ?? null;
      if (!script_runner) {
        return false;
      }
      script_runner.run("WateredWeirdTreeScript");
      return true;
    }
    return this._show_squirtbottle_nothing_text();
  }

  protected _can_use_squirtbottle_on_facing_object(): boolean {
    const map_name = String(this.current_map_name ?? "").toUpperCase();
    if (map_name !== "ROUTE36" && map_name !== "ROUTE_36") {
      return false;
    }
    const target = this._squirtbottle_facing_object();
    const movement = String(target?.event?.spritemovedata ?? "").toUpperCase();
    return movement === "SPRITEMOVEDATA_SUDOWOODO";
  }

  protected _squirtbottle_facing_object(): OverworldObject | null {
    const [tile_x, tile_y] = this.get_facing_tile_coords();
    const npcs = [...(this.npcs ?? [])].sort((a, b) => a.objectIndex - b.objectIndex);
    for (const npc of npcs) {
      if (this._squirtbottle_object_covers_tile(npc, tile_x, tile_y)) {
        return npc;
      }
    }
    return null;
  }

  protected _squirtbottle_object_covers_tile(
    npc: OverworldObject,
    tile_x: number,
    tile_y: number,
  ): boolean {
    const stride = Math.max(1, Number(npc.collisionStride ?? this.TILES_PER_COLLISION) || 1);
    const footprint = stride - 1;
    const positions: Array<[number, number]> = [
      [npc.x, npc.y],
      [npc.prevX ?? npc.x, npc.prevY ?? npc.y],
    ];
    for (const [x, y] of positions) {
      const origin_x = x - footprint;
      const origin_y = y - footprint;
      if (
        tile_x >= origin_x &&
        tile_x < origin_x + stride &&
        tile_y >= origin_y &&
        tile_y < origin_y + stride
      ) {
        return true;
      }
    }
    return false;
  }

  protected _show_squirtbottle_nothing_text(): boolean | Promise<boolean> {
    const label = "_SquirtbottleNothingText";
    const showTextAsync = this._show_field_move_text_async as
      | ((textLabel: string, actorName?: string | null) => Promise<void>)
      | undefined;
    if (typeof showTextAsync === "function") {
      return showTextAsync.call(this, label).then(() => true);
    }
    this._show_field_move_text(label);
    return true;
  }

  protected _update_blueprint_coordinates(map_name: string, object_index: number, map_x: number, map_y: number): void {
    const blueprint_map = this._npc_blueprints?.get(map_name) ?? null;
    if (!blueprint_map) {
      throw new Error(`Missing NPC blueprint cache for map '${map_name}'`);
    }
    let updated = false;
    for (const [identifier, entry] of blueprint_map.entries()) {
      const [event, index] = entry;
      if (index !== object_index) {
        continue;
      }
      event.x = map_x;
      event.y = map_y;
      blueprint_map.set(identifier, [event, index]);
      updated = true;
    }
    if (!updated) {
      throw new Error(`Blueprint missing object index ${object_index} on map '${map_name}'`);
    }
  }

  public move_object(object_id: string | number, map_x: number, map_y: number): void {
    const normalized_id = String(object_id).toUpperCase();
    const target = this.get_object_by_id(object_id);

    const map_x_int = Number(map_x);
    const map_y_int = Number(map_y);
    if (Number.isNaN(map_x_int) || Number.isNaN(map_y_int)) {
      throw new Error(`Invalid moveobject coordinates: ${map_x}, ${map_y}`);
    }

    if (!target) {
      const [map_name, entry] = this._find_blueprint_entry(normalized_id);
      if (!entry || map_name !== this.current_map_name) {
        throw new Error(`Unknown overworld object '${object_id}'`);
      }
      const [_event, index] = entry;
      this._update_blueprint_coordinates(this.current_map_name, index, map_x_int, map_y_int);
      return;
    }

    if (!("event" in target)) {
      throw new Error(`move_object target '${object_id}' is not an overworld NPC.`);
    }
    const npc = target as OverworldObject;
    npc.event.x = map_x_int;
    npc.event.y = map_y_int;
    this._initialise_object_coordinates(npc);
    this._npc_index_lookup.set(npc.objectIndex, npc);

    this._update_blueprint_coordinates(this.current_map_name, npc.objectIndex, map_x_int, map_y_int);

    npc.prevX = npc.x;
    npc.prevY = npc.y;
  }

  public move_strength_boulder_object(object_id: string | number, map_x: number, map_y: number): void {
    const target = this.get_object_by_id(object_id);

    const map_x_int = Number(map_x);
    const map_y_int = Number(map_y);
    if (Number.isNaN(map_x_int) || Number.isNaN(map_y_int)) {
      throw new Error(`Invalid Strength boulder coordinates: ${map_x}, ${map_y}`);
    }
    if (!target) {
      throw new Error(`Unknown Strength boulder object '${object_id}'`);
    }
    if (!("event" in target)) {
      throw new Error(`Strength boulder target '${object_id}' is not an overworld NPC.`);
    }

    const npc = target as OverworldObject;
    npc.event = {
      ...npc.event,
      x: map_x_int,
      y: map_y_int,
    };
    this._initialise_object_coordinates(npc);
    this._npc_index_lookup.set(npc.objectIndex, npc);

    npc.prevX = npc.x;
    npc.prevY = npc.y;
  }

  public lock_player_movement(): void {
    this._movement_lock_count += 1;
    this.stop_player_movement();
    pushDebugLog(`[lock] player movement (${this._movement_lock_count})`);
  }

  public unlock_player_movement(): void {
    if (this._movement_lock_count > 0) {
      this._movement_lock_count -= 1;
    }
    pushDebugLog(`[lock] player release (${this._movement_lock_count})`);
  }

  public lock_all_movement(): void {
    this.lock_player_movement();
  }

  public unlock_all_movement(): void {
    this.unlock_player_movement();
  }

  public stop_player_movement(): void {
    if (this._debug_inputs_enabled) {
      console.error("[Overworld] stop_player_movement()");
    }
    this.is_moving = false;
    this.step_progress_px = 0;
    this.step_dx_px = 0;
    this.step_dy_px = 0;
    this.target_tile_x = this.player_x;
    this.target_tile_y = this.player_y;
    this._held_directions.clear();
    this._queued_direction = null;
    this._turn_frames_remaining = 0;
    this._turning_direction = null;
    this._turn_should_force_step = false;
    this._pending_auto_step = null;
    this._last_step_direction = null;
    this._ledge_jump_active = false;
    this._ledge_jump_total_distance_px = 0;
    this._ledge_jump_animation_progress_px = 0;
    this._pending_ledge_landing = null;
    const player = this.player_object ?? null;
    if (player) {
      if ("walking" in player) {
        player.walking = false;
      }
      if ("jumping" in player) {
        player.jumping = false;
      }
      if ("step_frames_remaining" in player) {
        player.step_frames_remaining = 0;
      }
      if ("step_total_frames" in player) {
        player.step_total_frames = 0;
      }
      if ("step_dx_px" in player) {
        player.step_dx_px = 0;
      }
      if ("step_dy_px" in player) {
        player.step_dy_px = 0;
      }
      if ("sprite_y_offset" in player) {
        player.sprite_y_offset = 0;
      }
    }
    this._sync_player_state();
  }

  protected _frames_for_current_step(): number {
    const distance_px = Number(this._current_step_distance_px ?? 0);
    const speed_px = Number(this._current_step_speed_px ?? 0);
    if (distance_px <= 0 || speed_px <= 0) {
      throw new Error("Grass rustle requires a positive step distance/speed.");
    }
    return Math.max(1, Math.round(distance_px / speed_px));
  }

  protected _maybe_spawn_grass_rustle(target: OverworldObject | PlayerCharacter | null, tile_x: number, tile_y: number): void {
    const controller = this._grass_rustle ?? null;
    if (!controller || !target) {
      return;
    }
    if (!this._is_tile_grass(tile_x, tile_y)) {
      return;
    }
    controller.spawn?.(target, this._frames_for_current_step());
  }

  protected _prime_player_walk_cycle(): void {
    const animations = this.player_animations ?? null;
    if (!animations) {
      return;
    }
    const animation = animations[this.player_direction];
    if (!animation) {
      return;
    }
    primeWalkStride(animation);
  }

  public reset_input_state(): void {
    this.stop_player_movement();
    if (this.game_state) {
      clearJoypad(this.game_state);
    }
  }

  protected _apply_animation_state(animation: SpriteAnimation | null, { moving }: { moving: boolean }): void {
    if (!animation) {
      return;
    }
    const desired_facing = moving ? 0 : -1;
    if (animation.facing !== desired_facing) {
      animation.setFacing(desired_facing);
      if (moving) {
        animation.setFrame(animation.currentFrameIndex);
      }
    } else if (moving && animation.animate) {
      animation.setFrame(animation.currentFrameIndex);
    }
    animation.update();
  }

  public handle_a_button(): void {
    const traceInteraction =
      isDebugEnabled("overworld:interaction") || isDebugEnabled("interaction");
    if (this.check_for_npc_interaction()) {
      if (traceInteraction) {
        pushDebugLog("[interaction] A -> npc interaction", {
          map: this.current_map_name,
          tile: [this.player_x, this.player_y],
          facing: this.player_direction,
        });
      }
      return;
    }
    let [tile_x, tile_y] = this.get_facing_tile_coords();
    if (tile_x < 0 || tile_y < 0) {
      if (traceInteraction) {
        pushDebugLog("[interaction] A -> facing tile out of bounds", {
          map: this.current_map_name,
          tile: [tile_x, tile_y],
        });
      }
      return;
    }
    [tile_x, tile_y] = this._counter_adjusted_tile(tile_x, tile_y);
    if (traceInteraction) {
      pushDebugLog("[interaction] A pressed", {
        map: this.current_map_name,
        tile: [tile_x, tile_y],
        player_tile: [this.player_x, this.player_y],
        facing: this.player_direction,
      });
    }
    const bg_event = this._bg_event_at?.(tile_x, tile_y);
    if (bg_event && this._handle_bg_event(bg_event)) {
      if (traceInteraction) {
        pushDebugLog("[interaction] A -> bg event", {
          map: this.current_map_name,
          tile: [tile_x, tile_y],
          type: bg_event.event_type,
          script: bg_event.script,
        });
      }
      return;
    }
    const metatile_x = Math.trunc(tile_x / METATILE_WIDTH);
    const metatile_y = Math.trunc(tile_y / METATILE_WIDTH);
    if (!(0 <= metatile_x && metatile_x < this.map.width && 0 <= metatile_y && metatile_y < this.map.height)) {
      if (traceInteraction) {
        pushDebugLog("[interaction] A -> out of bounds", {
          map: this.current_map_name,
          metatile: [metatile_x, metatile_y],
        });
      }
      return;
    }
    const metatile_id = getMetatileAt(this.map, metatile_x, metatile_y);
    if (!(0 <= metatile_id && metatile_id < this.tileset.metatiles.length)) {
      if (traceInteraction) {
        pushDebugLog("[interaction] A -> missing metatile", {
          map: this.current_map_name,
          metatile: [metatile_x, metatile_y],
          metatile_id,
        });
      }
      return;
    }
    const metatile = this.tileset.metatiles[metatile_id];

    const permissions: number[] = [];
    const quadrant = determineQuadrantIndex(tile_x, tile_y);
    const permission = Number(metatile.collision[quadrant]);
    if (Number.isFinite(permission)) {
      permissions.push(permission);
    }
    if (!permissions.length) {
      if (traceInteraction) {
        pushDebugLog("[interaction] A -> no permissions", {
          map: this.current_map_name,
          metatile: [metatile_x, metatile_y],
          metatile_id,
        });
      }
      return;
    }

    const script_runner = this.script_runner ?? null;
    if (script_runner) {
      for (const permission of permissions) {
        const script = getCollisionStdScript(permission);
        if (!script) {
          continue;
        }
        if (traceInteraction) {
          pushDebugLog("[interaction] A -> std script", {
            map: this.current_map_name,
            metatile: [metatile_x, metatile_y],
            metatile_id,
            permission,
            script,
          });
        }
        this.game_state.wram.last_talked = 0;
        this._play_interaction_sound();
        script_runner.run(script);
        return;
      }
    }

    if (permissions.some((permission) => CUTTABLE_COLLISIONS.has(permission))) {
      if (traceInteraction) {
        pushDebugLog("[interaction] A -> cut", {
          map: this.current_map_name,
          metatile: [metatile_x, metatile_y],
          metatile_id,
          permissions,
        });
      }
      this._play_interaction_sound();
      this.handle_cut?.(metatile_x, metatile_y);
      return;
    }
    if (permissions.some((permission) => WHIRLPOOL_COLLISIONS.has(permission))) {
      if (traceInteraction) {
        pushDebugLog("[interaction] A -> whirlpool", {
          map: this.current_map_name,
          metatile: [metatile_x, metatile_y],
          metatile_id,
          permissions,
        });
      }
      this._play_interaction_sound();
      this.handle_whirlpool?.(metatile_x, metatile_y);
      return;
    }
    if (permissions.some((permission) => WATERFALL_COLLISIONS.has(permission))) {
      if (traceInteraction) {
        pushDebugLog("[interaction] A -> waterfall", {
          map: this.current_map_name,
          metatile: [metatile_x, metatile_y],
          metatile_id,
          permissions,
        });
      }
      this._play_interaction_sound();
      this.handle_waterfall?.(metatile_x, metatile_y);
      return;
    }
    if (this._tile_is_headbutt_tree?.(tile_x, tile_y)) {
      if (traceInteraction) {
        pushDebugLog("[interaction] A -> headbutt", {
          map: this.current_map_name,
          tile: [tile_x, tile_y],
          metatile: [metatile_x, metatile_y],
        });
      }
      if (!this._party_has_move?.("HEADBUTT")) {
        return;
      }
      this._play_interaction_sound();
      this.handle_headbutt?.(null, { prompt: true });
      return;
    }

    for (const permission of permissions) {
      const attributes = describeCollision(permission);
      if (attributes.terrain === Terrain.WATER) {
        if (traceInteraction) {
          pushDebugLog("[interaction] A -> surf", {
            map: this.current_map_name,
            metatile: [metatile_x, metatile_y],
            metatile_id,
            permission,
          });
        }
        this._play_interaction_sound();
        this.handle_surf?.(metatile_x, metatile_y);
        return;
      }
    }
    if (traceInteraction) {
      pushDebugLog("[interaction] A -> no action", {
        map: this.current_map_name,
        metatile: [metatile_x, metatile_y],
        metatile_id,
        permissions,
      });
    }
  }

  private _record_wait_status(options: {
    dialogue_waiting: boolean;
    dialogue_paused: boolean;
    pending_waits: number;
    script_runner_stop: boolean;
    awaiting_resume: number;
    stack_depth: number;
    queued_overworld_tasks: number;
    tasks_active: boolean;
  }): void {
    const messages: string[] = [];
    if (this.input_capture_active) {
      messages.push("input capture active");
    }
    const movementLocks = Number(this._movement_lock_count ?? 0);
    if (movementLocks > 0) {
      messages.push(`movement locked (count=${movementLocks})`);
    }
    if (options.dialogue_waiting || options.pending_waits > 0) {
      messages.push(`dialogue waiting (pending=${options.pending_waits})`);
    }
    if (options.dialogue_paused) {
      messages.push("dialogue paused");
    }
    if (options.awaiting_resume > 0) {
      messages.push(`script awaiting resume (count=${options.awaiting_resume})`);
    }
    if (options.script_runner_stop && options.stack_depth > 0) {
      messages.push(`script runner paused (stack=${options.stack_depth})`);
    }
    if (options.queued_overworld_tasks > 0) {
      messages.push(`queued overworld tasks active (count=${options.queued_overworld_tasks})`);
    }
    if (options.tasks_active) {
      const blocking = Number(this._blocking_task_count ?? 0);
      messages.push(`script tasks active (blocking=${blocking})`);
    }

    if (!messages.length) {
      if (this._last_wait_status_message !== null) {
        pushDebugLog("Idle: no waits");
      }
      this._last_wait_status_message = null;
      return;
    }

    const message = `Wait: ${messages.join(" | ")}`;
    if (message !== this._last_wait_status_message) {
      pushDebugLog(message);
      this._last_wait_status_message = message;
    }
  }

  protected _update_dialogue_and_scripts(): boolean {
    let pending_waits = 0;
    let dialogue_waiting = false;
    let dialogue_paused = false;
    let dialogue_visible = false;
    const dialogue = this.dialogue;
    if (!dialogue) {
      return false;
    }
    dialogue.update();
    if (this._hatch_text_pending) {
      if (dialogue.visible || dialogue.waiting_for_input) {
        return true;
      }
      this._hatch_text_pending = false;
      this._finalize_hatch_sequence();
      return true;
    }
    pending_waits = dialogue.pending_waits ?? 0;
    dialogue_waiting = Boolean(dialogue.waiting_for_input);
    dialogue_paused = Boolean(dialogue.is_script_paused);
    dialogue_visible = Boolean(dialogue.active);
    const prompt_transition_pending = dialoguePromptTransitionPending(
      dialogue as DialoguePromptTransitionState
    );
    const script_runner = this.script_runner;
    const awaiting_resume = script_runner?._awaiting_resume ?? 0;
    const stack_depth = script_runner?._script_stack?.length ?? 0;
    const queued_overworld_tasks = script_runner?._queued_overworld_task_count ?? 0;
    const tasks_active = this.script_tasks_active();
    if (
      this._text_lock_active &&
      !dialogue_visible &&
      !dialogue_waiting &&
      !dialogue_paused &&
      pending_waits === 0 &&
      awaiting_resume === 0 &&
      stack_depth === 0 &&
      !tasks_active
    ) {
      this._logger?.debug?.("Clearing stale dialogue lock after text window closed");
      this.unlock_player_movement();
      this._text_lock_active = false;
    }

    this._record_wait_status({
      dialogue_waiting,
      dialogue_paused,
      pending_waits,
      script_runner_stop: Boolean(script_runner?.stop_execution),
      awaiting_resume,
      stack_depth,
      queued_overworld_tasks,
      tasks_active,
    });
    if (script_runner && script_runner.stop_execution) {
      const status_snapshot: ScriptStatusSnapshot = [
        script_runner.stop_execution,
        awaiting_resume,
        stack_depth,
        tasks_active,
        pending_waits,
        dialogue_waiting,
        dialogue_paused,
      ];
      if (this._logger?.debug && !scriptStatusSnapshotEquals(status_snapshot, this._last_pending_script_status)) {
        this._logger.debug(
          `Script pending stop=${script_runner.stop_execution} awaiting=${awaiting_resume} stack=${stack_depth} tasks=${tasks_active} waits=${pending_waits} waiting=${dialogue_waiting} paused=${dialogue_paused}`,
        );
        this._last_pending_script_status = status_snapshot;
      }
    } else if (this._last_pending_script_status !== null) {
      this._last_pending_script_status = null;
    }

    if (
      script_runner &&
      script_runner.stop_execution &&
      awaiting_resume === 0 &&
      queued_overworld_tasks === 0 &&
      !tasks_active &&
      !dialogue_waiting &&
      !prompt_transition_pending &&
      pending_waits === 0 &&
      !dialogue_paused &&
      stack_depth > 0
    ) {
      this._logger?.debug?.(
        "Auto-resuming script runner (awaiting=%d stack=%d)",
        script_runner._awaiting_resume,
        stack_depth,
      );
      script_runner.resume?.();
    }

    // If the runner is stopped with an outstanding resume counter but there is no visible
    // dialogue/prompt and nothing else is running, treat this as a stale latch and resume.
    if (
      script_runner &&
      script_runner.stop_execution &&
      awaiting_resume > 0 &&
      queued_overworld_tasks === 0 &&
      !tasks_active &&
      !dialogue_waiting &&
      !prompt_transition_pending &&
      pending_waits === 0 &&
      !dialogue_paused
    ) {
      pushDebugLog("[script] auto-resume stale stop_execution", {
        awaiting: awaiting_resume,
        stack: stack_depth,
        map: this.current_map_name,
      });
      script_runner.resume?.();
    }

    // If we have an outstanding resume counter but no visible dialogue/tasks/waits,
    // treat it as a stale pause and resume anyway.
    if (
      script_runner &&
      !script_runner.stop_execution &&
      awaiting_resume > 0 &&
      queued_overworld_tasks === 0 &&
      !tasks_active &&
      !dialogue_waiting &&
      !prompt_transition_pending &&
      pending_waits === 0 &&
      !dialogue_paused
    ) {
      pushDebugLog("[script] auto-resume stale awaiting_resume", {
        awaiting: awaiting_resume,
        stack: stack_depth,
        map: this.current_map_name,
      });
      script_runner.resume?.();
    }

    return false;
  }

  protected _tick_turning(): void {
    if (this._turn_frames_remaining <= 0) {
      return;
    }
    this._turn_frames_remaining = Math.max(0, this._turn_frames_remaining - 1);
    if (this._turn_frames_remaining > 0) {
      return;
    }
    const direction = this._turning_direction;
    const forced = this._turn_should_force_step;
    this._turning_direction = null;
    this._turn_should_force_step = false;
    if (!direction) {
      return;
    }
    if (this.is_moving || this.player_movement_locked()) {
      return;
    }
    if (forced || this._held_directions.has(direction)) {
      this._pending_auto_step = [direction, forced];
    }
  }

  protected _start_idle_held_direction_step(): void {
    if (this.is_moving || this._pending_auto_step || this.player_movement_locked()) {
      return;
    }
    if (this._turn_frames_remaining > 0) {
      return;
    }
    const direction =
      this._queued_direction
      ?? this._held_directions.keys().next().value
      ?? null;
    if (!direction) {
      return;
    }
    this.move_player(String(direction).toLowerCase());
  }

  public update(): void {
    this.audio_controller?.update?.();
    const timekeeper = this._overworld_time_system;
    if (timekeeper) {
      timekeeper.tick();
    }
    this._tick_tile_animation_timer();
    this._tick_emotes();
    this._tick_grass_rustle();
    this._tick_field_move_states?.();
    if (this._warp_cooldown > 0) {
      this._warp_cooldown = Math.max(0, this._warp_cooldown - 1);
    }
    this._tick_field_move_animation_queue?.();
    this._tick_fishing_session?.();
    this._tick_turning();
    const renderer = this._field_move_animation_renderer;
    if (renderer) {
      renderer.advance();
    }
    if (this._egg_hatch_animation) {
      this._egg_hatch_animation.advance();
      if (this._egg_hatch_animation.isFinished()) {
        const display_name = this._egg_hatch_display_name;
        this._egg_hatch_animation = null;
        this._trigger_hatch_text(display_name);
      }
      return;
    }
    if (this.is_moving) {
      this.player_px_x += Math.trunc(this.step_dx_px);
      this.player_px_y += Math.trunc(this.step_dy_px);
      this.step_progress_px += this._current_step_speed_px;
      if (this._ledge_jump_active && this._ledge_jump_total_distance_px > 0) {
        this._ledge_jump_animation_progress_px = Math.min(
          this._ledge_jump_animation_progress_px + this._current_step_speed_px,
          this._ledge_jump_total_distance_px,
        );
      }
      if (this.step_progress_px >= this._current_step_distance_px) {
        this.player_x = this.target_tile_x;
        this.player_y = this.target_tile_y;
        this.player_px_x = this.target_px_x;
        this.player_px_y = this.target_px_y;
        this._sync_player_state();
        this._skip_wild_encounter_for_step = this._handle_overworld_step();
        this.is_moving = false;
        this._current_step_speed_px = this.STEP_SPEED_PX;
        this._current_step_distance_px = this.STEP_PIXELS;
        const landing = this._pending_ledge_landing;
        if (landing) {
          this._pending_ledge_landing = null;
          this._last_step_direction = landing.direction;
          this._begin_step(landing.dx, landing.dy, landing.tile_x, landing.tile_y);
        } else {
          this._ledge_jump_active = false;
          this._ledge_jump_total_distance_px = 0;
          this._ledge_jump_animation_progress_px = 0;
          const queued_before = this._queued_direction !== null;
          const wild_encounters = this._wild_encounters;
          if (wild_encounters) {
            wild_encounters.notify_step_complete();
          }
          if (advanceSafariTimer(this.game_state, { eventManager: this.event_manager ?? undefined })) {
            this._skip_wild_encounter_for_step = true;
          }
          if (this._last_step_direction) {
            this._enqueue_follower_step(
              this._last_step_direction,
              this.prev_player_x,
              this.prev_player_y,
            );
          }
          if (!this._queue_forced_waterfall_step?.() && !this._queue_ice_slide_step()) {
            const next_direction = this._next_direction_to_continue?.();
            if (next_direction) {
              this._pending_auto_step = [next_direction, queued_before];
            } else {
              this._pending_auto_step = null;
            }
          }
        }
      }
    }
    this._update_follower_movement();
    const animator = this._tileset_animator;
    animator?.update?.();
    this._refresh_warp_state();
    const map_sign = this._map_sign;
    map_sign?.update?.();
    const controller = this._npc_autonomous_controller;
    controller?.update?.();
    const pending_auto_step = this._pending_auto_step;
    let pending_walk_cycle = false;
    if (pending_auto_step && !this.player_movement_locked()) {
      const [pending_direction, forced] = pending_auto_step;
      pending_walk_cycle = pending_direction === this.player_direction &&
        (forced || this._held_directions.has(pending_direction));
    }
    const player_moving = this.is_moving || Boolean(this.player_object?.walking) || pending_walk_cycle;
    const player_anim = this.player_animations?.[this.player_direction];
    this._apply_animation_state(player_anim, { moving: player_moving });

    const has_grass = this._map_has_tall_grass;
    for (const npc of this.npcs ?? []) {
      const animation = npc.animations?.[npc.direction] ?? npc.animations?.down;
      if (animation) {
        const walking = Boolean(npc.walking);
        this._apply_animation_state(animation, { moving: walking });
        npc.facing = walking ? 0 : -1;
      }
      npc.overhead = has_grass && this._is_tile_grass(npc.x, npc.y);
    }
    this._process_player_events();
    this._process_script_tasks();
    this._process_pending_event_flag_updates();
    this._update_earthquake_state();
    this._update_elevator_state();
    if (this._update_dialogue_and_scripts()) {
      return;
    }
    this._update_fade();
    this._update_poison_flash();
    this._queue_forced_waterfall_step?.();
    this._start_idle_held_direction_step();
    this._queue_downhill_idle_step();
    this._start_pending_auto_step();
  }

  public set_multiplayer_remote_players(players: RemoteOverworldPlayer[]): void {
    this._multiplayer_remote_players = players.map((player) => ({
      userId: player.userId,
      playerName: player.playerName,
      entityType: player.entityType,
      mapName: player.mapName,
      tileX: Math.trunc(player.tileX),
      tileY: Math.trunc(player.tileY),
      direction: player.direction,
      updatedAtMs: Math.trunc(player.updatedAtMs),
    }));
  }

  public set_multiplayer_remote_render_enabled(enabled: boolean): void {
    this._multiplayer_remote_render_enabled = Boolean(enabled);
  }

  public set_multiplayer_remote_crowd_view(enabled: boolean): void {
    this._multiplayer_remote_crowd_view = Boolean(enabled);
  }

  protected _handle_bg_event(event: BackgroundEvent): boolean {
    const event_type = String(event.event_type ?? "").trim().toUpperCase();
    const direction = OverworldEngine.BG_EVENT_DIRECTION_MAP[event_type];
    if (direction && direction !== this.player_direction) {
      return false;
    }
    const script = String(event.script ?? "").trim();
    if (!script) {
      return false;
    }
    const data_loader = this.data_loader as DataLoaderLike | null;
    const is_conditional = OverworldEngine.BG_EVENT_CONDITIONAL_TYPES.has(event_type);
    const event_flag = (data_loader as DataLoaderLike | null)?.get_bg_event_script_flag?.(script)
      ?? (data_loader as DataLoaderLike | null)?.get_hidden_item_event_flag?.(script)
      ?? null;
    if (!this._bg_event_allowed_by_flags(event_type, script)) {
      return false;
    }
    if (is_conditional) {
      // ASM mapping: pokecrystal_disassembly/engine/overworld/events.asm (TryBGEvent .ifset/.ifnotset):
      // evaluate conditional_event in the wrapper and dispatch only the target script.
      const target_script = this._get_conditional_bg_event_target(script);
      if (!target_script) {
        if (isDebugEnabled("script")) {
          pushDebugLog("[bg] conditional event missing target", {
            eventType: event_type,
            script,
          });
        }
        return false;
      }
      const script_runner = this.script_runner;
      if (!script_runner) {
        return false;
      }
      this.game_state.wram.last_talked = 0;
      this._active_bg_event = event;
      this._play_interaction_sound();
      pushDebugLog(`[bg] ${script} -> ${target_script} @ ${event.x},${event.y}`, { type: event.event_type });
      try {
        if (typeof script_runner.call === "function") {
          script_runner.call(target_script, script);
        } else {
          script_runner.run(target_script);
        }
      } finally {
        this._active_bg_event = null;
      }
      return true;
    }
    if ((event_type === "BGEVENT_ITEM" || event_type === "BGEVENT_COPY") && event_flag && this.game_state.wram.event_flags?.[event_flag]) {
      return false;
    }
    if (event_type === "BGEVENT_COPY") {
      return false;
    }
    this.game_state.wram.last_talked = 0;
    const script_runner = this.script_runner;
    if (!script_runner) {
      return false;
    }
    this._active_bg_event = event;
    this._play_interaction_sound();
    pushDebugLog(`[bg] ${script} @ ${event.x},${event.y}`, { type: event.event_type });
    try {
      script_runner.run(script);
    } finally {
      this._active_bg_event = null;
    }
    return true;
  }

  protected _get_conditional_bg_event_target(script_name: string): string | null {
    const data_loader = this.data_loader as DataLoaderLike | null;
    if (!data_loader) {
      return null;
    }
    const script_data = data_loader.get_script?.(script_name);
    if (!script_data || script_data.length === 0) {
      return null;
    }
    const first_entry = script_data[0];
    if (!first_entry || typeof first_entry !== "object") {
      return null;
    }
    const command = String((first_entry as { command?: unknown }).command ?? "")
      .trim()
      .toLowerCase();
    if (command !== "conditional_event") {
      return null;
    }
    const rawArgs = (first_entry as { args?: unknown[] }).args;
    const args = Array.isArray(rawArgs) ? rawArgs : [];
    const target = String(args[1] ?? "").trim();
    return target || null;
  }

  protected _bg_event_allowed_by_flags(event_type: string, script_name: string): boolean {
    const normalized = String(event_type ?? "").trim().toUpperCase();
    if (!OverworldEngine.BG_EVENT_CONDITIONAL_TYPES.has(normalized)) {
      return true;
    }
    const script = script_name.trim();
    if (!script) {
      return normalized === "BGEVENT_IFNOTSET";
    }
    const data_loader = this.data_loader as DataLoaderLike | null;
    if (!data_loader) {
      return normalized === "BGEVENT_IFNOTSET";
    }
    const event_flag = data_loader.get_bg_event_script_flag?.(script) ?? null;
    if (event_flag) {
      const is_set = Boolean(this.game_state.wram.event_flags?.[event_flag]);
      return normalized === "BGEVENT_IFSET" ? is_set : !is_set;
    }
    const legacy_flags = data_loader.get_script_event_flags?.(script) ?? [];
    if (!legacy_flags.length) {
      return normalized === "BGEVENT_IFNOTSET";
    }
    const any_set = legacy_flags.some((flag: string) => this.game_state.wram.event_flags?.[flag]);
    if (normalized === "BGEVENT_IFSET") {
      return any_set;
    }
    return !any_set;
  }

  public consume_active_background_event(): BackgroundEvent | null {
    const event = this._active_bg_event ?? null;
    this._active_bg_event = null;
    return event;
  }

  public start_following(
    follower: OverworldObject,
    leader: OverworldObject,
    { follower_id = null, leader_id = null }: { follower_id?: string | null; leader_id?: string | null } = {},
  ): void {
    super.start_following(follower, leader, { follower_id, leader_id });
    this._reset_follower_path();
    this._queue_follower_first_step(leader, follower);
  }

  public stop_following(): void {
    super.stop_following();
    this._reset_follower_path();
  }

  public queue_follow_task(
    follower: OverworldObject,
    leader: OverworldObject,
    options: { onComplete?: (() => void) | null } = {},
  ): void {
    this.queueFollowTask?.(follower, leader, options);
  }

  protected _finalise_follower_position(): void {
    const task = this._follower_movement_task;
    const follower = task?.getTarget?.() ?? this.follower ?? null;
    if (!follower) {
      this._active_follower_target = null;
      this._follower_buffer_step = null;
      this._follower_step_queue = [];
      this._follower_queue_length = 0;
      this._follower_release_tokens = 0;
      this._pending_follower_origin = null;
      this._follower_movement_task = null;
      return;
    }

    if (task) {
      task.finish(this as unknown as MovementOverworldContext);
    }
    const target = this._active_follower_target;
    if (target) {
      follower.x = target[0];
      follower.y = target[1];
      if (hasUpdatePixelPosition(follower)) {
        follower.updatePixelPosition();
      }
    }
    if ("walking" in follower) {
      follower.walking = false;
    }
    if ("jumping" in follower) {
      follower.jumping = false;
    }
    if ("sprite_y_offset" in follower) {
      follower.sprite_y_offset = 0;
    }
    this._active_follower_target = null;
    this._pending_follower_origin = null;
    this._follower_movement_task = null;
  }

  protected _reset_follower_path(): void {
    this._finalise_follower_position();
    this._follower_step_queue = [];
    this._follower_queue_length = 0;
    this._follower_buffer_step = null;
    this._follower_release_tokens = 0;
    this._pending_follower_origin = null;
    this._last_step_direction = null;
    if (this.follower) {
      if (hasUpdatePixelPosition(this.follower)) {
        this.follower.updatePixelPosition();
      }
    }
  }

  protected _compute_initial_follow_step(leader: OverworldObject | null, follower: OverworldObject | null): string | null {
    if (!leader || !follower) {
      return null;
    }
    const leader_x = leader.x;
    const leader_y = leader.y;
    const follower_x = follower.x;
    const follower_y = follower.y;
    if (leader_x === undefined || leader_y === undefined || follower_x === undefined || follower_y === undefined) {
      return null;
    }
    if (leader_x !== follower_x) {
      return leader_x > follower_x ? "right" : "left";
    }
    if (leader_y !== follower_y) {
      return leader_y > follower_y ? "down" : "up";
    }
    return null;
  }

  protected _normalize_follower_step(step: string): { command: string; direction: string } | null {
    const trimmed = String(step ?? "").trim().toLowerCase();
    if (!trimmed) {
      return null;
    }
    const parts = trimmed.split(/\s+/).filter(Boolean);
    if (parts.length === 1) {
      const direction = parts[0];
      if (!["up", "down", "left", "right"].includes(direction)) {
        return null;
      }
      return { command: `step ${direction}`, direction };
    }
    const instruction = parts[0];
    const direction = parts[1];
    if (!["up", "down", "left", "right"].includes(direction)) {
      return null;
    }
    if (!FOLLOWER_STEP_INSTRUCTIONS.has(instruction)) {
      return null;
    }
    return { command: `${instruction} ${direction}`, direction };
  }

  protected _queue_follower_first_step(leader: OverworldObject | null, follower: OverworldObject | null): void {
    // ASM: QueueFollowerFirstStep (engine/overworld/player_object.asm).
    const direction = this._compute_initial_follow_step(leader, follower);
    this._follower_step_queue = [];
    this._follower_queue_length = 0;
    if (!direction) {
      this._follower_queue_length = -1;
      return;
    }
    const normalized = this._normalize_follower_step(direction);
    if (!normalized) {
      return;
    }
    this._follower_step_queue = [normalized.command];
  }

  protected _cancel_follow_if_leader_missing(): boolean {
    const leader = this.leader as OverworldObject | null;
    if (!leader) {
      this.stop_following();
      return true;
    }
    if (leader === this.player_object) {
      return false;
    }
    if (this.npcs?.includes(leader)) {
      return false;
    }
    this.stop_following();
    return true;
  }

  protected _get_next_follower_step(): string | null {
    // ASM: GetFollowerNextMovementIndex (engine/overworld/map_objects.asm).
    const length = this._follower_queue_length ?? 0;
    if (length <= 0) {
      this._cancel_follow_if_leader_missing();
      return null;
    }
    const command = this._follower_step_queue[0] ?? null;
    this._follower_step_queue.shift();
    this._follower_queue_length = length - 1;
    return command;
  }

  protected _enqueue_follower_step(direction: string, origin_x: number, origin_y: number): void {
    const follower = this.follower;
    const leader = this.leader;
    if (!follower || !leader) {
      return;
    }
    const normalized = this._normalize_follower_step(direction);
    if (!normalized) {
      return;
    }
    this._pending_follower_origin = [Number(origin_x), Number(origin_y)];
    const nextLength = (this._follower_queue_length ?? 0) + 1;
    this._follower_queue_length = nextLength;
    this._follower_step_queue[nextLength] = normalized.command;
    this._last_step_direction = null;
    this._try_start_follower_step();
  }

  protected _update_follower_movement(): void {
    const follower = this.follower;
    if (!follower) {
      this._follower_step_queue = [];
      this._follower_queue_length = 0;
      this._follower_movement_task = null;
      this._active_follower_target = null;
      this._follower_buffer_step = null;
      this._follower_release_tokens = 0;
      this._pending_follower_origin = null;
      return;
    }

    const task = this._follower_movement_task;
    if (task) {
      task.update(this as unknown as MovementOverworldContext);
      if (task.completed) {
        task.finish(this as unknown as MovementOverworldContext);
        this._follower_movement_task = null;
        const target = this._active_follower_target;
        if (target) {
          follower.x = target[0];
          follower.y = target[1];
          if (hasUpdatePixelPosition(follower)) {
            follower.updatePixelPosition();
          }
        }
        this._active_follower_target = null;
      }
    }
    if (this._follower_movement_task) {
      return;
    }
    this._try_start_follower_step();
  }

  protected _try_start_follower_step(): void {
    const follower = this.follower;
    if (!follower || this._follower_movement_task) {
      return;
    }

    const nextStep = this._get_next_follower_step();
    if (!nextStep) {
      return;
    }
    const normalized = this._normalize_follower_step(nextStep);
    if (!normalized) {
      return;
    }
    const direction = normalized.direction;
    if (!("objectIndex" in follower)) {
      return;
    }
    const npcFollower = follower as OverworldObject;
    const [dx, dy] = this._direction_to_vector(direction);
    const stride = resolveCollisionStride(npcFollower, this.TILES_PER_COLLISION);
    const target_x = npcFollower.x + dx * stride;
    const target_y = npcFollower.y + dy * stride;
    const task = new MovementTask(npcFollower, [normalized.command], {
      blocking: false,
      respectPlayerCollision: true,
    });
    task.start(this as unknown as MovementOverworldContext);
    if (task.completed) {
      task.finish(this as unknown as MovementOverworldContext);
      this._active_follower_target = null;
      this._follower_movement_task = null;
      return;
    }
    this._active_follower_target = [target_x, target_y];
    this._follower_movement_task = task;
  }

  protected _start_pending_auto_step(): void {
    const entry = this._pending_auto_step ?? null;
    if (!entry) {
      return;
    }
    const [direction, forced] = entry;
    const normalized = String(direction ?? "").toLowerCase();
    if (this.is_moving || this.player_movement_locked()) {
      return;
    }
    if (!forced && !this._held_directions.has(normalized) && !this._is_downhill_coast_direction(normalized)) {
      this._pending_auto_step = null;
      return;
    }
    this._pending_auto_step = null;
    this.move_player(normalized, forced);
  }

  protected _queue_downhill_idle_step(): void {
    if (this.is_moving || this._pending_auto_step || this.player_movement_locked()) {
      return;
    }
    if (!this._is_downhill_coast_direction("down")) {
      return;
    }
    this._pending_auto_step = ["down", true];
  }

  protected _is_downhill_coast_direction(direction: string): boolean {
    if (String(direction ?? "").toLowerCase() !== "down") {
      return false;
    }
    if (![PlayerState.BIKE, PlayerState.SKATE].includes(this.player_state)) {
      return false;
    }
    const bikeFlags = syncBikeFlags(this.game_state?.wram);
    return Boolean(bikeFlags & _BIKEFLAG_DOWNHILL);
  }

  protected _tick_tile_animation_timer(): void {
    tile_animation_timer().tick();
  }

  protected _tick_grass_rustle(): void {
    const controller = this._grass_rustle;
    controller?.tick?.();
  }

  public start_elevator_ride(
    origin: string | null = null,
    destination: string | null = null,
    {
      door_close_frames = null,
      door_open_frames = null,
      fade_frames = null,
      travel_frames = null,
      trigger_sound = false,
      trigger_earthquake = false,
    }: {
      door_close_frames?: number | null;
      door_open_frames?: number | null;
      fade_frames?: number | null;
      travel_frames?: number | null;
      trigger_sound?: boolean;
      trigger_earthquake?: boolean;
    } = {},
  ): void {
    if (!this.elevator_state) {
      this.elevator_state = new ElevatorRideStateMachine();
    }
    if (door_close_frames !== null) {
      this.elevator_state.door_close_frames = Number(door_close_frames);
    }
    if (door_open_frames !== null) {
      this.elevator_state.door_open_frames = Number(door_open_frames);
    }
    if (fade_frames !== null) {
      this.elevator_state.fade_frames = Number(fade_frames);
    }
    if (travel_frames !== null) {
      this.elevator_state.travel_frames = Number(travel_frames);
    }
    this.elevator_state.play_sound = Boolean(trigger_sound);
    this.elevator_state.start_earthquake = Boolean(trigger_earthquake);
    const elevatorOverworld =
      this as unknown as Parameters<ElevatorRideStateMachine["start"]>[0];
    this.elevator_state.start(elevatorOverworld, { origin, destination });
  }

  protected _update_elevator_state(): void {
    if (!this.elevator_state) {
      return;
    }
    const elevatorOverworld =
      this as unknown as Parameters<ElevatorRideStateMachine["update"]>[0];
    this.elevator_state.update(elevatorOverworld);
  }

  /**
   * Return an NPC `OverworldObject` by its script, sprite, or object ID.
   * @param object_id - The identifier for the object.
   * @returns The matching `OverworldObject` or `null`.
   */
  public get_object_by_id(object_id: string | number): OverworldObject | PlayerCharacter | null {
    if (object_id === null || object_id === undefined) {
      return null;
    }
    if (typeof object_id === "number" && Number.isFinite(object_id)) {
      if (object_id === 0 || object_id === -2) {
        return this.player_object ?? null;
      }
      if (object_id <= 0) {
        return null;
      }
      return this._npc_index_lookup.get(object_id) ?? null;
    }
    const normalised = String(object_id).toUpperCase();
    if (normalised === "PLAYER") {
      return this.player_object ?? null;
    }
    if (normalised === "LAST_TALKED") {
      const last = this.game_state?.wram?.last_talked ?? 0;
      return this.get_object_by_id(last) ?? null;
    }
    if (/^-?\d+$/.test(normalised)) {
      return this.get_object_by_id(Number.parseInt(normalised, 10));
    }
    for (const npc of this.npcs ?? []) {
      if (npc.matchesIdentifier(normalised, this.current_map_name)) {
        return npc;
      }
    }
    const follower = this.follower as OverworldObject | null;
    if (follower && follower.matchesIdentifier(normalised, this.current_map_name)) {
      return follower;
    }
    return null;
  }
}

applyMixins(OverworldEngine, [
  OverworldMovement,
  OverworldInputMixin,
  OverworldNpcManagerMixin,
  OverworldRenderingMixin,
  OverworldScriptQueueMixin,
  OverworldFieldMoveMixin,
  TrainerSightlineMixin,
]);

const canonicalWriteMetatile = Object.getOwnPropertyDescriptor(
  OverworldMapManagerMixin.prototype,
  "_write_metatile"
);
if (canonicalWriteMetatile) {
  Object.defineProperty(OverworldEngine.prototype, "_write_metatile", canonicalWriteMetatile);
}

export type Overworld = OverworldEngine;

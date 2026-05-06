// ASM mapping: pokecrystal_disassembly/engine/overworld/field_moves.asm (HM field move handlers).
// Additional mappings: engine/events/overworld.asm (Fishing), engine/events/sweet_scent.asm (Sweet Scent).
import { gameEngine, Surface, Rect } from "@pokecrystal/core/ui/game-engine";
import { createGbFrameAwaiter } from "@pokecrystal/core/ui/async-loop";
import { asmTextLoader } from "@pokecrystal/core/core/asm-text-loader";
import { Region } from "@pokecrystal/core/core/constants";
import { isInJohto } from "@pokecrystal/core/core/home";
import { Event, StartBattleEvent, closeText, openText, showText, waitForInput } from "@pokecrystal/core/engine/events/events";
import { PlayerState, FacingDirection } from "@pokecrystal/core/core/enums/overworld";
import { GB_FRAME_DURATION_MS, GB_FRAME_RATE } from "@pokecrystal/core/core/gb-timing";
import type { MapAttributes, MapEvents, WarpEvent } from "@pokecrystal/core/core/models/map";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { jumpRoamMons } from "@pokecrystal/core/engine/world/roamers";
import {
  MapMetadata,
  Spawn,
  applySpawn,
  findSpawnForMap,
  getMapEnvironment,
  getMapMetadataByConstant,
  getMapMetadataByGroup,
  getMapMetadataByName,
  getSpawnPoint,
} from "@pokecrystal/core/engine/world/maps";
import { choose_wild_encounter_bug_contest } from "@pokecrystal/core/engine/world/special-events/bug-contest";
import { warp_to_spawn_point } from "@pokecrystal/core/engine/world/special-events/map";
import { aerodactyl_chamber, kabuto_chamber } from "@pokecrystal/core/engine/world/special-events/unown";
import { Terrain, describeCollision, resolveCollisionValue } from "@pokecrystal/core/engine/world/overworld/collision-data";
import { getCoordCollision, isDirectionBlocked } from "@pokecrystal/core/engine/world/overworld/collision-rules";
import { scaleTileCoord } from "@pokecrystal/core/engine/world/overworld/tile-coords";
import {
  CUTTABLE_COLLISIONS,
  TALL_GRASS_COLLISIONS,
  WATERFALL_COLLISIONS,
  WHIRLPOOL_COLLISIONS,
  _BADGE_FLAG_NAMES,
  _CUT_BLOCKS,
  _FLY_DESTINATIONS,
  _WHIRLPOOL_BLOCKS,
} from "@pokecrystal/core/engine/world/overworld/constants";
import { METATILE_SIZE, METATILE_WIDTH } from "@pokecrystal/core/engine/world/tile";
import { flagForPlayerState } from "@pokecrystal/core/engine/world/overworld/player-state-flags";
import { chooseTreeEncounter, computeTreeScore, getTreeSetForMap } from "@pokecrystal/assets/content/tree-encounters";
import { EncounterSurface, WildEncounterManager, type OverworldLike as WildEncounterOverworld } from "@pokecrystal/core/engine/world/overworld/wild-encounters";
import { WildEncounter, type WildEncounterData } from "@pokecrystal/assets/content/wild-encounter-data";
import {
  DoFishing,
  FishingBattleTrigger,
  FishingBite,
  FishingRodState,
  FishingSession,
} from "@pokecrystal/core/engine/world/overworld/fishing";
import { update_player_sprite } from "@pokecrystal/core/engine/world/special-events/sprites";
import { FieldMoveVramLoader, HEADBUTT_SHAKE_FRAMES } from "@pokecrystal/core/engine/world/overworld/field-move-animation";
import { CutAnimationState, FieldMoveSpriteManager, FlyAnimationState } from "@pokecrystal/core/engine/world/overworld/field-move-sprite-anim";
import { YesNoPrompt } from "@pokecrystal/core/ui/text/dialogue";
import type { OverworldDialogue } from "@pokecrystal/core/engine/world/overworld/dialogue-types";
import { FlyMapPrompt } from "@pokecrystal/core/ui/overlays/fly-map-prompt";
import { SelectionPrompt } from "@pokecrystal/core/ui/text/prompts";
import type { GameState } from "@pokecrystal/core/core/state";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import type { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import type { OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";
import type { OverworldObject } from "@pokecrystal/core/engine/world/overworld/overworld-object";
import { Pokemon, PokemonSpecies, toPokemon } from "@pokecrystal/core/core/models";
import { NUM_BADGES, hasOwnedBadgeAsm } from "@pokecrystal/core/core/badges";
import type { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { createPokemon } from "@pokecrystal/core/engine/systems/pokemon";
import { performRockSmash, type RockSmashPending } from "@pokecrystal/core/engine/world/story-events/specials/helpers";
import { playOverworldSound } from "./audio-guards";
import { getBooleanFlag, setBooleanFlag } from "@pokecrystal/core/engine/world/overworld/flag-collection";
import { STANDARD_TEXT_FALLBACKS } from "@pokecrystal/core/engine/world/story-events/common";
import { TextFormatter } from "@pokecrystal/core/engine/world/story-events/text-formatter";

type FieldDialogueInternals = {
  window?: {
    is_complete?: () => boolean;
    complete?: () => void;
  } | null;
  pendingWaits?: number;
  pending_script_waits?: number;
  script_runner?: {
    resume?: () => void;
  } | null;
  script_paused?: boolean;
  _suppress_orphan_close?: boolean;
};

const _DIG_ALLOWED_ENVIRONMENTS = new Set(["CAVE", "DUNGEON"]);
const _FLY_ALLOWED_ENVIRONMENTS = new Set(["ROUTE", "TOWN"]);
const _HEADBUTT_COLLISIONS = new Set([
  resolveCollisionValue("COLL_HEADBUTT_TREE"),
  resolveCollisionValue("COLL_HEADBUTT_TREE_1D"),
]);
const _BUG_CONTEST_TIMER_FLAG = "ENGINE_BUG_CONTEST_TIMER";
const _BIKEFLAG_STRENGTH_ACTIVE = 1 << 0;
const _BIKEFLAG_ALWAYS_ON_BIKE = 1 << 1;
const _BIKEFLAG_DOWNHILL = 1 << 2;
const _SWEET_SCENT_TIME_OF_DAY: Record<string, "morning" | "day" | "night"> = {
  morn: "morning",
  morning: "morning",
  day: "day",
  nite: "night",
  night: "night",
  dark: "night",
  darkness: "night",
};
const FIELD_MOVE_FRAME_DURATION_MS = GB_FRAME_DURATION_MS;

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

type MapEventCollection = Map<string, MapEvents> | Record<string, MapEvents | undefined>;

type MapEventLoader = {
  map_events?: MapEventCollection;
  mapEvents?: MapEventCollection;
};

const isMapEvents = (value: unknown): value is MapEvents =>
  Boolean(value) && typeof value === "object";

const getMapEvents = (dataLoader: MapEventLoader | null | undefined, mapName: string): MapEvents | null => {
  if (!dataLoader || !mapName) {
    return null;
  }
  const events = dataLoader.map_events ?? dataLoader.mapEvents ?? null;
  if (!events) {
    return null;
  }
  const candidate =
    events instanceof Map ? events.get(mapName) ?? null : (events as Record<string, MapEvents | undefined>)[mapName] ?? null;
  return isMapEvents(candidate) ? candidate : null;
};

const warpTilePosition = (warp: { x: number; y: number }, stride: number): [number, number] => {
  const offset = Math.max(0, stride - 1);
  return [warp.x * stride + offset, warp.y * stride + offset];
};

type TilesetLike = {
  tilesetName?: string | null;
  tileset_name?: string | null;
};

type MetatileTileEntry =
  | number
  | {
      tile_index?: number | null;
      tileIndex?: number | null;
    };

type MetatileTiles = MetatileTileEntry[][];

type TilesetMetatileEntry = {
  tiles?: MetatileTiles;
  collision?: number[];
};

type PaletteResetter = {
  reset_to_defaults?: () => void;
  resetToDefaults?: () => void;
};

const resolveTilesetName = (tileset: TilesetLike | null | undefined): string => {
  return String(tileset?.tilesetName ?? tileset?.tileset_name ?? "").trim();
};

const resolveMetatileTileIndex = (entry: MetatileTileEntry | null | undefined): number | null => {
  if (typeof entry === "number" && Number.isFinite(entry)) {
    return entry;
  }
  if (entry && typeof entry === "object") {
    const candidate = entry.tile_index ?? entry.tileIndex;
    if (typeof candidate === "number" && Number.isFinite(candidate)) {
      return candidate;
    }
  }
  return null;
};

type PokemonSpeciesLookup = {
  get_pokemon_species?: (name: string) => PokemonSpecies | null | undefined;
  getPokemonSpecies?: (name: string) => PokemonSpecies | null | undefined;
  getSpecies?: (name: string) => PokemonSpecies | null | undefined;
  pokemon_data?: Map<string, PokemonSpecies> | Record<string, PokemonSpecies>;
};

type FieldMoveAnimationMetadata = Record<string, unknown>;

type FlyPendingState = {
  tile_x: number;
  tile_y: number;
  direction?: string;
};

type SweetScentPendingState = {
  encounter?: WildEncounter | null;
  battle_type?: string;
};

type FieldMoveAnimationEntry = {
  animation: string;
  remaining: number;
  x: number;
  y: number;
  restore?: number | Array<[number, number, number]> | null;
  variant?: string;
  metadata?: FieldMoveAnimationMetadata;
  direction?: string;
  state?: { tick: () => void; completed: boolean };
};

type FieldMoveDialogue = OverworldDialogue;

type FlyDestination = {
  label: string;
  landmark: string;
  spawn: Spawn;
  default: boolean;
};

type PromptWithRunner = {
  runAsync?: (options?: { drawCallback?: () => void }) => Promise<number>;
};

const getMetatileRows = (metatile: unknown): MetatileTiles | null => {
  if (!metatile || typeof metatile !== "object") {
    return null;
  }
  const entry = metatile as TilesetMetatileEntry;
  if (!Array.isArray(entry.tiles)) {
    return null;
  }
  return entry.tiles as MetatileTiles;
};

const hasNamedMoveEntry = (value: unknown): value is { name?: unknown } =>
  Boolean(value && typeof value === "object" && "name" in value);

const normalizeMoveName = (value: unknown): string | null => {
  if (hasNamedMoveEntry(value)) {
    const candidate = value.name;
    if (candidate === null || candidate === undefined) {
      return null;
    }
    return String(candidate);
  }
  if (value === null || value === undefined) {
    return null;
  }
  return String(value);
};

export class OverworldFieldMoveMixin {
  private static readonly _TREE_REPLACEMENT_SIZE: [number, number] = [2, 2];
  private static readonly _TREE_RELATIVE_OFFSETS: Record<string, [number, number]> = {
    right: [0, 2],
    left: [0, -2],
    down: [-2, 0],
    up: [2, 0],
  };
  private static readonly _GRASS_METATILE_CACHE: Map<string, number> = new Map();

  private _field_move_loader?: FieldMoveVramLoader | null = null;
  private _field_move_sprite_manager?: FieldMoveSpriteManager | null = null;
  private _field_move_frame_awaiter = createGbFrameAwaiter();
  private _field_move_auto_advance_flag = false;
  private _active_fishing_session: FishingSession | null = null;
  private _headbutt_sequence_active = false;
  protected _wild_encounters?: WildEncounterManager | null;

  public _field_move_animations: FieldMoveAnimationEntry[] | null = null;
  public _fly_animation_state: FlyAnimationState | null = null;
  public _fly_pending_to_metadata: FlyPendingState | null = null;
  public _sweet_scent_pending?: SweetScentPendingState;
  public _pending_rock_smash?: RockSmashPending;

  constructor() {
    this._initialize_field_move_state();
  }

  protected _initialize_field_move_state(): void {
    this._field_move_loader = null;
    this._field_move_sprite_manager = null;
    this._field_move_frame_awaiter = createGbFrameAwaiter();
    this._field_move_auto_advance_flag = false;
    this._active_fishing_session = null;
    this._headbutt_sequence_active = false;
    this._field_move_animations = null;
    this._fly_animation_state = null;
    this._fly_pending_to_metadata = null;
    this._sweet_scent_pending = undefined;
    this._pending_rock_smash = undefined;
  }

  public get _SWEET_SCENT_TRIGGER_FRAMES(): number {
    return 20;
  }

  public get _ROCK_SMASH_BREAK_FRAMES(): number {
    return 0x0b;
  }

  public map: OverworldMap | null = null;
  public map_surface: InstanceType<typeof gameEngine.Surface> | null = null;
  public priority_surface: InstanceType<typeof gameEngine.Surface> | null = null;
  public tileset: OverworldTilesetLike | null = null;
  public game_state: GameState | null = null;
  public data_loader: DataLoader | null = null;
  public ui: BaseUI | null = null;
  public audio_engine: AudioEngine | null = null;
  public event_manager: EventManager | null = null;
  public dialogue: FieldMoveDialogue | null = null;
  public fly_prompt_class?: typeof SelectionPrompt | null;
  public _fly_menu_selector?: ((labels: string[]) => number) | null;
  public _field_move_confirm_callback?: ((moveName: string) => boolean) | null;
  public _current_map_attributes?: () => MapAttributes | undefined;
  public _counter_adjusted_tile?: (tile_x: number, tile_y: number) => [number, number];
  public current_map_name: string = "";

  public player_direction: string = "down";
  public player_x: number = 0;
  public player_y: number = 0;
  public prev_player_x: number = 0;
  public prev_player_y: number = 0;
  public target_tile_x: number = 0;
  public target_tile_y: number = 0;
  public is_moving: boolean = false;
  public step_progress_px: number = 0;
  public step_dx_px: number = 0;
  public step_dy_px: number = 0;
  public TILES_PER_COLLISION: number = 2;
  public player_state: PlayerState = PlayerState.NORMAL;
  public player_object: OverworldObject | null = null;
  public _queued_direction: string | null = null;

  public refresh_composite_surfaces?: (dirty: Map<Surface, Rect[]>) => void;
  public draw?: () => void;
  public load_map?: (mapName: string) => void;
  public remove_object?: (objectId: string | number, options?: { update_event_flag?: boolean }) => void;
  public stop_player_movement?: () => void;
  public _direction_to_vector?: (direction: string) => [number, number];
  public _normalise_time_of_day_label?: (label: string | null | undefined) => string;
  public _refresh_tileset_for_current_map?: (attributes?: MapAttributes | null) => void;
  public get_facing_tile_coords?: () => [number, number];
  public _sync_player_state?: () => void;
  public clear_pending_white_fade?: () => void;
  public start_map_music?: () => void;
  public fade_to_white?: (frames?: number) => void;
  public fade_from_white?: (frames?: number) => void;

  protected _require_game_state(context: string): GameState {
    if (!this.game_state) {
      throw new Error(`${context} requires a loaded game state.`);
    }
    return this.game_state;
  }

  protected _require_wram(context: string): GameState["wram"] {
    const game_state = this._require_game_state(context);
    const wram = game_state.wram;
    if (!wram) {
      throw new Error(`${context} requires WRAM to be initialized.`);
    }
    return wram;
  }

  protected _overworld_like(context: string): WildEncounterOverworld | null {
    if (!this.map || !this.tileset) {
      return null;
    }
    if (!this.current_map_name) {
      return null;
    }
    return this as unknown as WildEncounterOverworld;
  }

  protected _write_metatile(metatile_x: number, metatile_y: number, block_id: number): void {
    if (!this.map) {
      throw new Error("No overworld map is currently loaded.");
    }
    if (!this.tileset) {
      throw new Error("No overworld tileset is currently loaded.");
    }
    const width = this.map.width ?? 0;
    const height = this.map.height ?? 0;
    if (metatile_x < 0 || metatile_x >= width || metatile_y < 0 || metatile_y >= height) {
      throw new Error(
        `Metatile target (${metatile_x}, ${metatile_y}) outside map bounds ${width}x${height}.`
      );
    }
    const index = metatile_y * width + metatile_x;
    if (index >= (this.map.metatileIds ?? []).length) {
      throw new Error(
        `Metatile index ${index} exceeds loaded map data (${this.map.metatileIds.length} entries).`
      );
    }
    const old_block = this.map.metatileIds[index];
    this.map.metatileIds[index] = block_id & 0xff;

    const dirty_rect = new gameEngine.Rect(
      metatile_x * METATILE_SIZE,
      metatile_y * METATILE_SIZE,
      METATILE_SIZE,
      METATILE_SIZE
    );
    const metatile_id = block_id & 0xff;
    if (this.map_surface) {
      this.tileset.renderMetatile(
        metatile_id,
        this.map_surface,
        dirty_rect.x,
        dirty_rect.y,
        { vram: this.game_state?.vram ?? null }
      );
    }
    if (this.priority_surface) {
      this.priority_surface.fill([0, 0, 0, 0], dirty_rect);
      this.tileset.renderPriorityMetatile(
        metatile_id,
        this.priority_surface,
        dirty_rect.x,
        dirty_rect.y
      );
    }
    if (this.refresh_composite_surfaces) {
      const dirty = new Map<Surface, Rect[]>();
      if (this.map_surface) {
        dirty.set(this.map_surface, [dirty_rect]);
      }
      if (this.priority_surface) {
        dirty.set(this.priority_surface, [dirty_rect]);
      }
      this.refresh_composite_surfaces(dirty);
    }

    if (old_block === (block_id & 0xff)) {
      return;
    }
  }

  protected _restore_field_move_tiles(
    restore: number | Array<[number, number, number]> | null,
    default_x: number,
    default_y: number
  ): void {
    if (restore === null || restore === undefined) {
      return;
    }
    const entries = Array.isArray(restore) ? restore : [[default_x, default_y, restore]];
    for (const entry of entries) {
      if (entry.length !== 3) {
        throw new Error(`Expected (x, y, block_id) tuples for restores, got ${entry}`);
      }
      const [x, y, block_id] = entry;
      this._write_metatile(x, y, block_id);
    }
  }

  protected _field_move_vram_loader(): FieldMoveVramLoader | null {
    if (this._field_move_loader) {
      return this._field_move_loader;
    }
    if (!this.game_state) {
      return null;
    }
    const loader = new FieldMoveVramLoader(this.game_state);
    this._field_move_loader = loader;
    return loader;
  }

  protected _load_field_move_tiles(animation: string): void {
    const loader = this._field_move_vram_loader();
    if (!loader) {
      return;
    }
    const palettes =
      (this.game_state as (GameState & { palettes?: PaletteResetter | null }) | null)?.palettes ?? null;
    if (palettes?.reset_to_defaults) {
      palettes.reset_to_defaults();
    } else if (palettes?.resetToDefaults) {
      palettes.resetToDefaults();
    }
    if (animation === "headbutt") {
      loader.request_headbutt_tiles();
      return;
    }
    loader.request_cut_tiles();
  }

  protected _metatile_permissions(metatile_x: number, metatile_y: number): number[] {
    const overworld_map = this.map;
    const tileset = this.tileset;
    if (!overworld_map || !tileset) {
      return [];
    }
    if (metatile_x < 0 || metatile_x >= (overworld_map.width ?? 0)) {
      return [];
    }
    if (metatile_y < 0 || metatile_y >= (overworld_map.height ?? 0)) {
      return [];
    }
    let metatile_id: number;
    try {
      metatile_id = overworld_map.getMetatileAt(metatile_x, metatile_y);
    } catch {
      return [];
    }
    const collisions: number[] = [];
    const metatile = tileset.metatiles?.[metatile_id] as TilesetMetatileEntry | undefined;
    if (!metatile) {
      return collisions;
    }
    const permissions = metatile.collision ?? [];
    for (const permission of permissions ?? []) {
      const value = Number(permission);
      if (Number.isFinite(value)) {
        collisions.push(value);
      }
    }
    return collisions;
  }

  protected _metatile_contains_collision(
    metatile_x: number,
    metatile_y: number,
    permitted: Set<number>
  ): boolean {
    for (const permission of this._metatile_permissions(metatile_x, metatile_y)) {
      if (permitted.has(permission)) {
        return true;
      }
    }
    return false;
  }

  protected _metatile_has_terrain(metatile_x: number, metatile_y: number, terrain: Terrain): boolean {
    for (const permission of this._metatile_permissions(metatile_x, metatile_y)) {
      if (describeCollision(permission).terrain === terrain) {
        return true;
      }
    }
    return false;
  }

  protected _align_player_to_metatile(metatile_x: number, metatile_y: number): void {
    const stride = this.TILES_PER_COLLISION ?? 1;
    const offset = Math.max(stride - 1, 0);
    const new_x = metatile_x * METATILE_WIDTH + offset;
    const new_y = metatile_y * METATILE_WIDTH + offset;
    this.player_x = new_x;
    this.player_y = new_y;
    this.prev_player_x = new_x;
    this.prev_player_y = new_y;
    this.target_tile_x = new_x;
    this.target_tile_y = new_y;
    this._sync_player_state?.();
  }

  protected _field_move_direction(): string {
    const direction = this.player_direction ?? "down";
    const normalized = String(direction).toLowerCase();
    return ["down", "up", "left", "right"].includes(normalized) ? normalized : "down";
  }

  protected _player_tile_position(): [number, number] {
    return [Number(this.player_x ?? 0), Number(this.player_y ?? 0)];
  }

  protected _player_pixel_position(): [number, number] {
    const [tile_x, tile_y] = this._player_tile_position();
    return [tile_x * METATILE_SIZE, tile_y * METATILE_SIZE];
  }

  protected _dispatch_field_move_animation(
    animation: string,
    frames: number,
    tile_x: number,
    tile_y: number,
    {
      variant = null,
      metadata = null,
      direction = null,
    }: {
      variant?: string | null;
      metadata?: FieldMoveAnimationMetadata | null;
      direction?: string | null;
    } = {}
  ): void {
    if (!this.event_manager) {
      return;
    }
    const resolved_direction = direction ?? this._field_move_direction();
    const payload = {
      animation: animation.toLowerCase(),
      frames,
      x: tile_x,
      y: tile_y,
      phase: "start",
      variant: (variant ?? animation).toLowerCase(),
      metadata: metadata ?? {},
      direction: resolved_direction,
    };
    this.event_manager.dispatch(new Event("field_move_animation", payload));
  }

  protected _start_field_move_animation(
    animation: string,
    frames: number,
    tile_x: number,
    tile_y: number,
    {
      restore_block_id = null,
      variant = null,
      metadata = null,
      direction = null,
    }: {
      restore_block_id?: number | null;
      variant?: string | null;
      metadata?: FieldMoveAnimationMetadata | null;
      direction?: string | null;
    } = {}
  ): FieldMoveAnimationEntry {
    const resolved_direction = direction ?? this._field_move_direction();
    this._dispatch_field_move_animation(animation, frames, tile_x, tile_y, {
      variant,
      metadata,
      direction: resolved_direction,
    });
    if (!this._field_move_animations) {
      this._field_move_animations = [];
    }
    const entry: FieldMoveAnimationEntry = {
      animation: animation.toLowerCase(),
      remaining: frames,
      x: tile_x,
      y: tile_y,
      restore: restore_block_id,
      variant: (variant ?? animation).toLowerCase(),
      metadata: metadata ?? {},
      direction: resolved_direction,
    };
    this._field_move_animations.push(entry);
    return entry;
  }

  protected _ensure_field_move_sprite_manager(): FieldMoveSpriteManager {
    if (!this._field_move_sprite_manager) {
      this._field_move_sprite_manager = new FieldMoveSpriteManager();
    }
    return this._field_move_sprite_manager;
  }

  protected _advance_field_move_animations(): void {
    if (!this._field_move_animations) {
      this._field_move_animations = [];
    }
    const survivors: FieldMoveAnimationEntry[] = [];
    for (const entry of this._field_move_animations) {
      const state = entry.state;
      if (state && !state.completed) {
        state.tick();
      }
      entry.remaining -= 1;
      if (this.event_manager) {
        const tick = {
          animation: entry.animation,
          frames: entry.remaining,
          x: entry.x,
          y: entry.y,
          phase: "tick",
          variant: entry.variant ?? entry.animation,
          metadata: entry.metadata ?? {},
          direction: entry.direction ?? "down",
        };
        this.event_manager.dispatch(new Event("field_move_animation", tick));
      }
      if (entry.remaining <= 0) {
        this._restore_field_move_tiles(entry.restore ?? null, entry.x, entry.y);
        if (this.event_manager) {
          const complete = {
            animation: entry.animation,
            frames: 0,
            x: entry.x,
            y: entry.y,
            phase: "complete",
            variant: entry.variant ?? entry.animation,
            direction: entry.direction ?? "down",
            metadata: entry.metadata ?? {},
          };
          this.event_manager.dispatch(new Event("field_move_animation", complete));
        }
        continue;
      }
      survivors.push(entry);
    }
    this._field_move_animations = survivors;
    const fly_state = this._fly_animation_state;
    if (fly_state) {
      fly_state.tick();
      if (fly_state.completed) {
        this._fly_animation_state = null;
        if (this._fly_pending_to_metadata) {
          const pending = this._fly_pending_to_metadata;
          this._fly_pending_to_metadata = null;
          this._start_fly_to_animation(
            pending.tile_x,
            pending.tile_y,
            pending.direction ?? "down"
          );
        }
      }
    }
  }

  protected _tick_field_move_animation_queue(): void {
    this._advance_field_move_animations();
  }

  protected _run_cut_animation(tile_x: number, tile_y: number, variant: string): void {
    this._load_field_move_tiles("cut");
    this._play_field_move_sound("SFX_PLACE_PUZZLE_PIECE_DOWN");
    const direction = this._field_move_direction();
    const manager = this._ensure_field_move_sprite_manager();
    const game_state = this._require_game_state("Cut animation");
    manager.clear_sprite_anims(game_state);
    manager.reserve_oam(game_state);
    const entry = this._start_field_move_animation("CUT", 32, tile_x, tile_y, {
      variant: variant.toLowerCase(),
      direction,
    });
    entry.state = new CutAnimationState(manager, game_state, {
      player_x: this.player_x ?? 0,
      player_y: this.player_y ?? 0,
      target_tile_x: tile_x,
      target_tile_y: tile_y,
      direction,
      variant,
    });
  }

  protected _run_headbutt_animation(tile_x: number, tile_y: number): void {
    this._load_field_move_tiles("headbutt");
    this._play_field_move_sound("SFX_SANDSTORM");
    const direction = this._field_move_direction();
    let replacements: Array<[number, number, number]> | null = null;
    if (this.map && this.tileset) {
      replacements = this._replace_headbutt_tree_tiles(tile_x, tile_y, direction);
    }
    const entry = this._start_field_move_animation("HEADBUTT", HEADBUTT_SHAKE_FRAMES, tile_x, tile_y, {
      variant: "headbutt",
      direction,
    });
    if (replacements) {
      entry.restore = replacements;
    }
  }

  protected _run_whirlpool_animation(tile_x: number, tile_y: number): void {
    this._play_field_move_sound("SFX_SURF");
    const direction = this._field_move_direction();
    this._start_field_move_animation("WHIRLPOOL", 32, tile_x, tile_y, {
      variant: "whirlpool",
      direction,
    });
  }

  protected _start_fly_from_animation(): void {
    const manager = this._ensure_field_move_sprite_manager();
    const game_state = this._require_game_state("Fly animation");
    manager.clear_sprite_anims(game_state);
    manager.reserve_oam(game_state, { base_addr: 0 });
    const [px, py] = this._player_pixel_position();
    const [tile_x, tile_y] = this._player_tile_position();
    this._fly_animation_state = new FlyAnimationState(manager, game_state, {
      player_x: px,
      player_y: py,
      variant: "from",
      sound_player: this._play_field_move_sound.bind(this),
    });
    this._start_field_move_animation("FLY", 129, tile_x, tile_y, {
      variant: "from",
      direction: this.player_direction,
    });
  }

  protected _start_fly_to_animation(tile_x: number, tile_y: number, direction: string): void {
    const manager = this._ensure_field_move_sprite_manager();
    const game_state = this._require_game_state("Fly animation");
    manager.clear_sprite_anims(game_state);
    manager.reserve_oam(game_state, { base_addr: 0 });
    const px = tile_x * METATILE_SIZE;
    const py = tile_y * METATILE_SIZE;
    this._fly_animation_state = new FlyAnimationState(manager, game_state, {
      player_x: px,
      player_y: py,
      variant: "to",
      sound_player: this._play_field_move_sound.bind(this),
    });
    this._start_field_move_animation("FLY", 65, tile_x, tile_y, {
      variant: "to",
      direction,
    });
  }

  protected _queue_fly_to_animation(tile_x: number, tile_y: number, direction: string): void {
    this._fly_pending_to_metadata = { tile_x, tile_y, direction };
  }

  protected _headbutt_tree_anchor(tile_x: number, tile_y: number, direction: string): [number, number] {
    const offset = OverworldFieldMoveMixin._TREE_RELATIVE_OFFSETS[direction.toLowerCase()] ?? [0, 0];
    return [tile_x + offset[0], tile_y + offset[1]];
  }

  protected _headbutt_tree_coords(tile_x: number, tile_y: number, direction: string): Array<[number, number]> {
    let [anchor_x, anchor_y] = this._headbutt_tree_anchor(tile_x, tile_y, direction);
    const [width, height] = OverworldFieldMoveMixin._TREE_REPLACEMENT_SIZE;
    if (this.map) {
      if (this.map.width < width || this.map.height < height) {
        throw new Error(
          `Map ${this.map.mapName ?? "<unknown>"} is too small for a headbutt tree footprint (${width}x${height}).`
        );
      }
      const max_x = this.map.width - width;
      const max_y = this.map.height - height;
      const clamped_x = Math.max(0, Math.min(anchor_x, max_x));
      const clamped_y = Math.max(0, Math.min(anchor_y, max_y));
      anchor_x = clamped_x;
      anchor_y = clamped_y;
    }
    const coords: Array<[number, number]> = [];
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        coords.push([anchor_x + x, anchor_y + y]);
      }
    }
    return coords;
  }

  protected _headbutt_grass_metatile_id(): number {
    if (!this.tileset) {
      throw new Error("Cannot replace headbutt tree tiles without a loaded tileset.");
    }
    const cache_key = resolveTilesetName(this.tileset) || "<unknown>";
    const cached = OverworldFieldMoveMixin._GRASS_METATILE_CACHE.get(cache_key);
    if (cached !== undefined) {
      return cached;
    }
    const metatiles = this.tileset.metatiles ?? [];
    for (let index = 0; index < metatiles.length; index += 1) {
      const metatile = metatiles[index];
      const rows = getMetatileRows(metatile);
      if (!Array.isArray(rows)) {
        continue;
      }
      let matches = true;
      for (const row of rows) {
        if (!Array.isArray(row)) {
          matches = false;
          break;
        }
        for (const entry of row) {
          const tileIndex = resolveMetatileTileIndex(entry);
          if (tileIndex !== 0x05) {
            matches = false;
            break;
          }
        }
        if (!matches) {
          break;
        }
      }
      if (matches) {
        OverworldFieldMoveMixin._GRASS_METATILE_CACHE.set(cache_key, index);
        return index;
      }
    }
    throw new Error(
      `Tileset '${cache_key}' lacks a $05 grass metatile required for headbutt visuals.`
    );
  }

  protected _replace_headbutt_tree_tiles(tile_x: number, tile_y: number, direction: string): Array<[number, number, number]> {
    if (!this.map) {
      throw new Error("Map must be loaded to modify headbutt tiles.");
    }
    const grass_block = this._headbutt_grass_metatile_id();
    const replacements: Array<[number, number, number]> = [];
    for (const [coord_x, coord_y] of this._headbutt_tree_coords(tile_x, tile_y, direction)) {
      const original_block = this.map.getMetatileAt(coord_x, coord_y);
      replacements.push([coord_x, coord_y, original_block]);
      this._write_metatile(coord_x, coord_y, grass_block);
    }
    return replacements;
  }

  protected _format_landmark(label: string): string {
    const cleaned = label.startsWith("LANDMARK_") ? label.slice("LANDMARK_".length) : label;
    const parts = cleaned.split("_");
    return parts.filter(Boolean).map((part) => part.toUpperCase()).join(" ");
  }

  protected _available_fly_destinations(): FlyDestination[] {
    const game_state = this.game_state;
    const destinations: FlyDestination[] = [];
    if (!game_state) {
      return destinations;
    }
    const flags = this.game_state?.wram?.engine_flags;
    const inKanto = isInJohto(game_state) === Region.KANTO;
    const hasIndigo = getBooleanFlag(flags, "ENGINE_FLYPOINT_INDIGO_PLATEAU");
    const useKantoMap = inKanto && hasIndigo;
    const start = useKantoMap ? 12 : 0;
    const end = useKantoMap ? _FLY_DESTINATIONS.length : 12;
    const defaultIndex = useKantoMap ? end - 1 : start;
    for (let index = start; index < end; index += 1) {
      const [flag_name, landmark, spawn] = _FLY_DESTINATIONS[index];
      const isDefault = index === defaultIndex;
      if (!isDefault && !getBooleanFlag(flags, flag_name)) {
        continue;
      }
      const caption = this._format_landmark(landmark);
      destinations.push({ label: caption, landmark, spawn, default: isDefault });
    }
    return destinations;
  }

  protected async _select_fly_destination_async(
    labels: string[],
    initialIndex = 0,
    destinations: FlyDestination[] | null = null,
  ): Promise<number> {
    if (this._fly_menu_selector) {
      return this._fly_menu_selector(labels);
    }
    if (!labels.length) {
      throw new Error("Cannot choose a Fly destination when no options are available.");
    }
    const ui = this.ui;
    if (!ui) {
      throw new Error("Fly prompt requires an active UI.");
    }
    if (destinations) {
      const game_state = this._require_game_state("Fly");
      const prompt = new FlyMapPrompt(ui, game_state, destinations, initialIndex);
      return await this._with_field_move_input_capture_async(async () => prompt.runAsync()) ?? -1;
    }
    const promptClass = this.fly_prompt_class ?? SelectionPrompt;
    const prompt = new promptClass(ui, labels, {
      audioEngine: this.audio_engine ?? undefined,
      title: "FLY TO WHERE?",
      initialIndex,
      cancelResult: -1,
    });
    const runner = prompt as PromptWithRunner;
    if (typeof runner.runAsync !== "function") {
      return 0;
    }
    try {
      return await this._with_field_move_input_capture_async(async () =>
        runner.runAsync?.({ drawCallback: this.draw?.bind(this) }) ?? Promise.resolve(0)
      ) ?? 0;
    } catch {
      return 0;
    }
  }

  public check_badge(badge_id: number): boolean {
    // ASM mapping: pokecrystal_disassembly/engine/events/overworld.asm::CheckBadge
    if (!Number.isInteger(badge_id)) {
      throw new Error(`Badge id ${badge_id} must be an integer.`);
    }
    if (badge_id < 0 || badge_id >= NUM_BADGES) {
      throw new Error(`Badge id ${badge_id} is out of ASM range 0-${NUM_BADGES - 1}.`);
    }
    const flag_name = _BADGE_FLAG_NAMES[badge_id];
    if (!flag_name) {
      throw new Error(`Missing engine flag mapping for badge id ${badge_id}.`);
    }
    if (getBooleanFlag(this.game_state?.wram?.engine_flags, flag_name)) {
      return true;
    }
    const badges = this.game_state?.sram?.badges ?? null;
    if (!badges) {
      return false;
    }
    return hasOwnedBadgeAsm(badges, badge_id, "Overworld check_badge");
  }

  public async handle_cut(x: number, y: number, pokemon: Pokemon | null = null): Promise<boolean> {
    if (!this.map || !this.tileset) {
      return false;
    }
    if (!this.event_manager) {
      return false;
    }
    const tileset_key = resolveTilesetName(this.tileset).toLowerCase();
    const replacements = _CUT_BLOCKS[tileset_key];
    if (!replacements) {
      await this._show_field_move_text_async("CutNothingText");
      return false;
    }
    if (
      !this._metatile_contains_collision(x, y, CUTTABLE_COLLISIONS) &&
      !this._metatile_contains_collision(x, y, TALL_GRASS_COLLISIONS)
    ) {
      await this._show_field_move_text_async("CutNothingText");
      return false;
    }

    let current: number;
    try {
      current = this.map.getMetatileAt(x, y);
    } catch {
      await this._show_field_move_text_async("CutNothingText");
      return false;
    }
    const entry = replacements[current];
    if (!entry) {
      await this._show_field_move_text_async("CutNothingText");
      return false;
    }
    const [replacement, variant] = entry;

    let actor = pokemon;
    if (!actor) {
      [actor] = this._get_party_move_holder("CUT");
    }
    const actor_name = this._field_move_actor_name(actor);

    if (!actor || !this.check_badge(1)) {
      await this._show_field_move_text_async("CanCutText");
      return false;
    }

    const dialogue = this.dialogue as (OverworldDialogue & FieldDialogueInternals) | null;
    const previous_suppress = dialogue?._suppress_orphan_close ?? null;
    if (dialogue) {
      dialogue._suppress_orphan_close = true;
    }
    try {
      const ask_text = this._resolve_field_move_text("AskCutText");
      openText(this.event_manager);
      showText(this.event_manager, ask_text);
      await this._wait_for_dialogue_render_async();
      const confirmed = await this._prompt_field_move_confirmation_async("CUT");
      if (!confirmed) {
        closeText(this.event_manager);
        await this._wait_for_dialogue_closed_async();
        return false;
      }

      this.stop_player_movement?.();
      const clean_name = actor_name || "POKEMON";
      const use_text = this._format_field_move_text("UseCutText", clean_name);
      showText(this.event_manager, use_text);
      await this._wait_for_dialogue_render_async();
      this._run_cut_animation(x, y, variant);
      this._write_metatile(x, y, replacement);
      await this._auto_close_field_move_dialogue_async();
      return true;
    } finally {
      if (dialogue) {
        dialogue._suppress_orphan_close = Boolean(previous_suppress);
      }
    }
  }

  public handle_headbutt(
    pokemon: Pokemon | null = null,
    options: { prompt?: boolean } = {}
  ): Promise<boolean> {
    return this._handle_headbutt_async(pokemon, options);
  }

  private async _handle_headbutt_async(
    pokemon: Pokemon | null,
    options: { prompt?: boolean }
  ): Promise<boolean> {
    if (!this.map || !this.tileset) {
      return false;
    }
    if (!this.event_manager) {
      return false;
    }
    const game_state = this._require_game_state("Headbutt");
    const wram = this._require_wram("Headbutt");

    const target = this._headbutt_target_tile();
    if (!target) {
      await this._show_field_move_text_async("CantUseItemText");
      return false;
    }
    const [tile_x, tile_y] = target;
    if (!this._tile_is_headbutt_tree(tile_x, tile_y)) {
      await this._show_field_move_text_async("CantUseItemText");
      return false;
    }

    let actor = pokemon;
    if (!actor) {
      [actor] = this._get_party_move_holder("HEADBUTT");
    }
    if (!actor) {
      return false;
    }
    const actor_name = this._field_move_actor_name(actor);

    const prompt = Boolean(options.prompt);
    if (prompt) {
      openText(this.event_manager);
      showText(this.event_manager, this._resolve_field_move_text("AskHeadbuttText"));
      await this._wait_for_dialogue_render_async();
      const confirmed = await this._prompt_field_move_confirmation_async("HEADBUTT");
      if (!confirmed) {
        closeText(this.event_manager);
        await this._wait_for_dialogue_closed_async();
        return false;
      }
    }

    if (this._headbutt_sequence_active) {
      return false;
    }
    if (wram.wHeadbuttState) {
      wram.wHeadbuttState = 0;
    }
    this._headbutt_sequence_active = true;
    wram.wHeadbuttState = 1;
    wram.wHeadbuttCount = (Number(wram.wHeadbuttCount ?? 0) + 1) & 0xff;
    wram.wHeadbuttLastStep = wram.step_count;

    const use_text = this._format_field_move_text("UseHeadbuttText", actor_name);
    if (!prompt) {
      openText(this.event_manager);
    }
    showText(this.event_manager, use_text);
    await this._wait_for_dialogue_render_async();

    const metatile_x = Math.trunc(tile_x / METATILE_WIDTH);
    const metatile_y = Math.trunc(tile_y / METATILE_WIDTH);
    const metadata = this._current_map_metadata();
    const tree_set = metadata ? getTreeSetForMap(metadata.constant) : null;
    try {
      this._run_headbutt_animation(metatile_x, metatile_y);
      await this._run_field_move_animation_frames_async(HEADBUTT_SHAKE_FRAMES);
      const rng = new HardwareRNG(game_state);
      const player_id = game_state.sram?.player_id ?? 0;
      const score = computeTreeScore(tile_x, tile_y, player_id);
      let encounter: [string, number] | null = null;
      if (tree_set) {
        encounter = chooseTreeEncounter(tree_set, score, rng.randrange.bind(rng));
      }
      if (!encounter) {
        showText(this.event_manager, this._resolve_field_move_text("HeadbuttNothingText"));
        await this._wait_for_dialogue_render_async();
        waitForInput(this.event_manager);
        await this._wait_for_dialogue_ack_async();
        closeText(this.event_manager);
        await this._wait_for_dialogue_closed_async();
        return true;
      }
      const [species, level] = encounter;
      closeText(this.event_manager);
      await this._wait_for_dialogue_closed_async();
      this._start_headbutt_battle(species, level);
      return true;
    } finally {
      this._headbutt_sequence_active = false;
      wram.wHeadbuttState = 0;
    }
  }

  public async handle_fishing(rod_item: string, pokemon: Pokemon | null = null): Promise<boolean> {
    const rod_key = String(rod_item ?? "").trim().toUpperCase();
    if (!["OLD_ROD", "GOOD_ROD", "SUPER_ROD"].includes(rod_key)) {
      throw new Error(`Unknown fishing rod '${rod_item}'.`);
    }
    if ([PlayerState.SURF, PlayerState.SURF_PIKA].includes(this.player_state)) {
      await this._show_field_move_text_async("CantFishHereText");
      return false;
    }

    const target = this._facing_metatile_coordinates();
    if (!target) {
      await this._show_field_move_text_async("CantFishHereText");
      return false;
    }
    const [metatile_x, metatile_y] = target;
    if (!this._metatile_has_terrain(metatile_x, metatile_y, Terrain.WATER)) {
      await this._show_field_move_text_async("CantFishHereText");
      return false;
    }

    if (!this.event_manager) {
      return false;
    }
    const actor_name = this._field_move_actor_name(pokemon);
    const text = this._format_field_move_text("UseFishingRodText", actor_name);
    openText(this.event_manager);
    showText(this.event_manager, text);
    await this._wait_for_dialogue_render_async();
    waitForInput(this.event_manager);
    await this._wait_for_dialogue_ack_async();

    if (!this.data_loader) {
      throw new Error("Cannot resolve fishing encounters without data.");
    }
    const game_state = this._require_game_state("Fishing");
    const rng = new HardwareRNG(game_state);
    const session = DoFishing(
      game_state,
      this.data_loader,
      this.current_map_name ?? "",
      rod_key,
      rng
    );
    this.stop_player_movement?.();
    this._active_fishing_session = session;
    return true;
  }

  protected _tick_fishing_session(): void {
    const game_state = this._require_game_state("Fishing");
    const session = this._active_fishing_session ?? null;
    if (!session) {
      return;
    }
    const bite = FishingBite(game_state, session);
    if (bite === null) {
      return;
    }
    void this._resolve_fishing_outcome_async(session, bite).catch((error) => {
      const logger = (this as unknown as { _logger?: { error?: (...args: unknown[]) => void } })._logger;
      logger?.error?.("[field-move] Fishing outcome failed", error);
    });
  }

  protected async _resolve_fishing_outcome_async(session: FishingSession, bite: boolean | null): Promise<void> {
    const wram = this._require_wram("Fishing");
    if (!this.event_manager) {
      this._active_fishing_session = null;
      wram.wFishingRodState = FishingRodState.IDLE;
      return;
    }
    this._active_fishing_session = null;
    if (!bite) {
      showText(this.event_manager, this._resolve_field_move_text("RodNothingText"));
      await this._wait_for_dialogue_render_async();
      waitForInput(this.event_manager);
      await this._wait_for_dialogue_ack_async();
      closeText(this.event_manager);
      await this._wait_for_dialogue_closed_async();
      wram.wFishingRodState = FishingRodState.IDLE;
      return;
    }

    showText(this.event_manager, this._resolve_field_move_text("RodBiteText"));
    await this._wait_for_dialogue_render_async();
    waitForInput(this.event_manager);
    await this._wait_for_dialogue_ack_async();
    closeText(this.event_manager);
    await this._wait_for_dialogue_closed_async();
    const encounter = session.outcome.encounter;
    if (!encounter) {
      wram.wFishingRodState = FishingRodState.IDLE;
      return;
    }
    FishingBattleTrigger(this._require_game_state("Fishing"), session);
    this._start_wild_battle(encounter.species, encounter.level, "BATTLETYPE_FISH");
    wram.wFishingRodState = FishingRodState.IDLE;
  }

  protected _headbutt_target_tile(): [number, number] | null {
    if (!this.get_facing_tile_coords) {
      return null;
    }
    const [tile_x, tile_y] = this.get_facing_tile_coords();
    if (tile_x < 0 || tile_y < 0) {
      return null;
    }
    return [tile_x, tile_y];
  }

  protected _tile_is_headbutt_tree(tile_x: number, tile_y: number): boolean {
    try {
      if (!this.map || !this.tileset) {
        return false;
      }
      const permission = getCoordCollision(this.map, this.tileset, tile_x, tile_y);
      return _HEADBUTT_COLLISIONS.has(permission);
    } catch {
      return false;
    }
  }

  protected _current_map_metadata(): MapMetadata | null {
    const names = [this.current_map_name ?? "", this.map?.mapName ?? ""];
    const seen = new Set<string>();
    for (const map_name of names) {
      if (!map_name || seen.has(map_name)) {
        continue;
      }
      seen.add(map_name);
      const metadata = getMapMetadataByName(map_name) ?? getMapMetadataByConstant(map_name);
      if (metadata) {
        return metadata;
      }
    }
    return null;
  }

  protected _start_headbutt_battle(species_name: string, level: number): void {
    this._start_wild_battle(species_name, level, "BATTLETYPE_TREE");
  }

  protected _resolve_species(species_name: string): PokemonSpecies {
    const loader = this.data_loader as (DataLoader & PokemonSpeciesLookup) | null;
    const lookup = loader?.get_pokemon_species ?? loader?.getPokemonSpecies ?? loader?.getSpecies;
    const upper = String(species_name).toUpperCase();
    if (typeof lookup === "function") {
      const species = lookup.call(loader, upper);
      if (!species) {
        throw new Error(`Unknown wild species '${species_name}'.`);
      }
      return species;
    }
    const table = loader?.pokemonData ?? loader?.pokemon_data ?? {};
    if (table instanceof Map) {
      const species = table.get(upper);
      if (!species) {
        throw new Error(`Unknown wild species '${species_name}'.`);
      }
      return species;
    }
    if (table && typeof table === "object" && upper in table) {
      return table[upper];
    }
    throw new Error(`Unknown wild species '${species_name}'.`);
  }

  protected _start_wild_battle(species_name: string, level: number, battle_type: string): void {
    if (!this.event_manager) {
      throw new Error("Cannot start a wild battle without an event manager.");
    }
    if (!this.data_loader) {
      throw new Error("Cannot start a wild battle without data loader support.");
    }
    const game_state = this._require_game_state("Wild battle");
    const wram = this._require_wram("Wild battle");
    const species = this._resolve_species(species_name);
    const wild_pokemon = createPokemon(game_state, species, level);
    wild_pokemon.original_trainer_name = "WILD";
    wild_pokemon.original_trainer_id = 0;

    const party_members = game_state.sram?.party?.pokemon ?? [];
    const player_party = party_members.filter((pokemon): pokemon is Pokemon => Boolean(pokemon));
    if (!player_party.length) {
      throw new Error("Cannot start a wild battle without at least one Pokemon.");
    }
    const player_pokemon = player_party[0];
    if (!player_pokemon) {
      throw new Error("Cannot start a wild battle without a lead Pokemon.");
    }

    wram.wild_pokemon = { species: species_name, level };
    wram.other_trainer = undefined;
    wram.other_trainer_party = [];
    wram.other_trainer_class = "";
    wram.other_trainer_id = "";
    wram.battle_type = battle_type;
    wram.reload_map_after_battle = true;

    const event = new StartBattleEvent({
      player_pokemon,
      enemy_pokemon: wild_pokemon,
      player_party,
      enemy_party: [wild_pokemon],
    });
    this.event_manager.dispatch(event);
  }

  protected _select_surface_encounter(rng: HardwareRNG): WildEncounter | null {
    const manager = this._wild_encounters ?? null;
    if (!manager) {
      return null;
    }
    const overworldContext = this._overworld_like("Wild encounter") ?? null;
    if (!overworldContext) {
      return null;
    }
    const data = manager._lookup_map_data?.(this.current_map_name);
    if (!data) {
      return null;
    }
    const surface = manager._resolve_surface?.(overworldContext);
    if (!surface) {
      return null;
    }
    const table = manager._resolve_table?.(data, surface);
    if (!table || !table.length) {
      return null;
    }
    if (!manager._passes_encounter_roll?.(data, surface, rng, overworldContext)) {
      return null;
    }
    const slot = manager._choose_slot?.(surface, table.length, rng);
    if (slot === null || slot === undefined) {
      return null;
    }
    const encounter = table[slot];
    const level = manager._apply_grass_level_variance?.(encounter.level, surface, rng);
    return { level, species: encounter.species };
  }

  protected _sweet_scent_encounter_rate(
    data: WildEncounterData,
    surface: EncounterSurface
  ): number {
    if (surface === EncounterSurface.WATER) {
      return Math.max(0, Math.trunc(Number(data.water_rate ?? 0)));
    }
    const rates = data.grass_rates ?? null;
    if (!rates) {
      return 0;
    }
    const token = String(this.game_state?.wram?.time_of_day ?? "day").toLowerCase();
    const key = _SWEET_SCENT_TIME_OF_DAY[token] ?? "day";
    return Math.max(0, Math.trunc(Number(rates[key] ?? 0)));
  }

  protected _sweet_scent_encounter(rng: HardwareRNG): [WildEncounter, string] | null {
    const manager = this._wild_encounters ?? null;
    if (!manager) {
      return null;
    }
    const overworldContext = this._overworld_like("Sweet scent") ?? null;
    if (!overworldContext) {
      return null;
    }
    const data = manager._lookup_map_data?.(this.current_map_name);
    if (!data) {
      return null;
    }
    const surface = manager._resolve_surface?.(overworldContext);
    if (!surface) {
      return null;
    }

    // ASM: SweetScentEncounter calls CanEncounterWildMon first for both contest/non-contest paths.
    if (this._bug_contest_active()) {
      const [species, level] = choose_wild_encounter_bug_contest(this._require_game_state("Sweet scent"), {
        overworld: this,
        event_manager: this.event_manager ?? undefined,
        rng,
      });
      return [{ level, species }, "BATTLETYPE_CONTEST"];
    }

    // ASM: SweetScentEncounter checks GetMapEncounterRate and aborts on zero.
    if (this._sweet_scent_encounter_rate(data, surface) <= 0) {
      return null;
    }

    if (typeof manager.choose_forced_encounter === "function") {
      return manager.choose_forced_encounter(overworldContext, rng);
    }
    const table = manager._resolve_table?.(data, surface);
    if (!table || !table.length) {
      return null;
    }
    const slot = manager._choose_slot?.(surface, table.length, rng);
    if (slot === null || slot === undefined) {
      return null;
    }
    const encounter = table[slot];
    const level = manager._apply_grass_level_variance?.(encounter.level, surface, rng);
    const wram = this._require_wram("Sweet scent");
    wram.battle_type = "BATTLETYPE_NORMAL";
    wram.wTempBattleMonSpecies = encounter.species;
    return [{ level, species: encounter.species }, "BATTLETYPE_NORMAL"];
  }

  protected _select_water_encounter(rng: HardwareRNG): WildEncounter | null {
    const manager = this._wild_encounters ?? null;
    if (!manager) {
      return null;
    }
    const overworldContext = this._overworld_like("Water encounter") ?? null;
    if (!overworldContext) {
      return null;
    }
    const data = manager._lookup_map_data?.(this.current_map_name);
    if (!data) {
      return null;
    }
    const table = manager._resolve_table?.(data, EncounterSurface.WATER);
    if (!table || !table.length) {
      return null;
    }
    if (!manager._passes_encounter_roll?.(data, EncounterSurface.WATER, rng, overworldContext)) {
      return null;
    }
    const slot = manager._choose_slot?.(EncounterSurface.WATER, table.length, rng);
    if (slot === null || slot === undefined) {
      return null;
    }
    const encounter = table[slot];
    const level = manager._apply_grass_level_variance?.(encounter.level, EncounterSurface.WATER, rng);
    return { level, species: encounter.species };
  }

  protected async _handle_hm(move_name: string, x: number, y: number, _player_state: PlayerState): Promise<boolean> {
    const normalized = move_name.trim().toUpperCase();
    if (normalized === "STRENGTH") {
      return this.handle_strength(x, y);
    }
    if (normalized === "WATERFALL") {
      return await this.handle_waterfall(x, y, { from_menu: true });
    }
    if (normalized === "WHIRLPOOL") {
      return await this.handle_whirlpool(x, y);
    }
    throw new Error(`Unsupported field move '${move_name}'.`);
  }

  public async use_hm_from_menu(move_name: string, pokemon: Pokemon | null): Promise<boolean> {
    const normalized = String(move_name ?? "").trim().toUpperCase();
    if (normalized === "CUT") {
      const coords = this._facing_metatile_coordinates();
      if (!coords) {
        await this._show_field_move_text_async("CutNothingText");
        return false;
      }
      return await this.handle_cut(coords[0], coords[1], pokemon);
    }
    if (normalized === "SURF") {
      const coords = this._facing_metatile_coordinates();
      if (!coords) {
        await this._show_field_move_text_async("CantUseItemText");
        return false;
      }
      return await this.handle_surf(coords[0], coords[1]);
    }
    if (normalized === "STRENGTH") {
      const used = this.handle_strength(0, 0);
      if (!used) {
        await this._show_field_move_text_async("CantUseItemText");
        return false;
      }
      const [actor] = this._get_party_move_holder("STRENGTH");
      const actor_name = this._field_move_actor_name(pokemon ?? actor);
      await this._show_field_move_text_async("UseStrengthText", actor_name);
      this._play_field_move_sound("SFX_STRENGTH");
      return true;
    }
    if (normalized === "FLASH") {
      return await this.handle_flash();
    }
    if (normalized === "WATERFALL") {
      const coords = this._facing_metatile_coordinates();
      if (!coords) {
        await this._show_field_move_text_async("CantUseItemText");
        return false;
      }
      return await this.handle_waterfall(coords[0], coords[1], { from_menu: true });
    }
    if (normalized === "WHIRLPOOL") {
      const coords = this._facing_metatile_coordinates();
      if (!coords) {
        await this._show_field_move_text_async("CantUseItemText");
        return false;
      }
      return await this.handle_whirlpool(coords[0], coords[1]);
    }
    if (normalized === "FLY") {
      return await this.handle_fly(0, 0);
    }
    return false;
  }

  public async handle_surf(x: number, y: number): Promise<boolean> {
    // ASM: engine/events/overworld.asm::TrySurfOW + UsedSurfScript.
    if (!this.map || !this.tileset || !this.event_manager) {
      return false;
    }
    if ([PlayerState.SURF, PlayerState.SURF_PIKA].includes(this.player_state)) {
      return false;
    }
    const facing = this.get_facing_tile_coords?.();
    if (!facing) {
      return false;
    }
    const [tile_x, tile_y] = facing;
    const permission = getCoordCollision(this.map, this.tileset, tile_x, tile_y);
    if (describeCollision(permission).terrain !== Terrain.WATER) {
      return false;
    }
    if (isDirectionBlocked(permission, FacingDirection.fromString(this.player_direction))) {
      return false;
    }
    const npc_lookup = this as unknown as { _npc_occupying_subtile?: (x: number, y: number) => unknown };
    if (typeof npc_lookup._npc_occupying_subtile === "function") {
      if (npc_lookup._npc_occupying_subtile(tile_x, tile_y)) {
        return false;
      }
    }
    const wram = this._require_wram("Surf");
    const bikeFlags = syncBikeFlags(wram);
    if (bikeFlags & _BIKEFLAG_ALWAYS_ON_BIKE) {
      return false;
    }
    if (!this.check_badge(3)) {
      return false;
    }
    const [actor] = this._get_party_move_holder("SURF");
    if (!actor) {
      return false;
    }
    const party = this.game_state?.sram?.party?.pokemon ?? [];
    // ASM: engine/events/overworld.asm::GetSurfType uses wCurPartyMon to pick SURF_PIKA.
    const surfIndex = Math.max(0, Math.trunc(wram.wCurPartyMon ?? 0));
    const surfMon = party[surfIndex] ?? null;
    const surfState = surfMon?.species?.id === "PIKACHU" ? PlayerState.SURF_PIKA : PlayerState.SURF;
    wram.wSurfingPlayerState = flagForPlayerState(surfState);

    this.stop_player_movement?.();
    const ask_text = this._resolve_field_move_text("AskSurfText");
    openText(this.event_manager);
    showText(this.event_manager, ask_text);
    await this._wait_for_dialogue_render_async();
    const confirmed = await this._prompt_field_move_confirmation_async("SURF");
    if (!confirmed) {
      closeText(this.event_manager);
      await this._wait_for_dialogue_closed_async();
      return false;
    }
    const actor_name = this._field_move_actor_name(actor);
    const use_text = this._format_field_move_text("UsedSurfText", actor_name);
    showText(this.event_manager, use_text);
    await this._wait_for_dialogue_render_async();
    await this._auto_close_field_move_dialogue_async();

    this._align_player_to_metatile(x, y);
    this.player_state = surfState;
    update_player_sprite(this._require_game_state("Surf"), { overworld: this });
    this.start_map_music?.();
    wram.surfing = true;

    const direction = this._field_move_direction();
    const commands = [`slow_step ${direction}`, "step_end"];
    const queueMovementTask = (this as unknown as { queue_movement_task?: Function; queueMovementTask?: Function })
      .queue_movement_task ?? (this as unknown as { queueMovementTask?: Function }).queueMovementTask;
    if (queueMovementTask && this.player_object) {
      queueMovementTask.call(this, this.player_object, commands, { onComplete: () => {} });
    }
    return true;
  }

  public async handle_whirlpool(x: number, y: number): Promise<boolean> {
    if (!this.check_badge(6)) {
      return false;
    }
    const [actor] = this._get_party_move_holder("WHIRLPOOL");
    if (!actor) {
      return false;
    }
    if (!this._metatile_contains_collision(x, y, WHIRLPOOL_COLLISIONS)) {
      await this._show_field_move_text_async("CantUseItemText");
      return false;
    }
    if (!this.map || !this.tileset) {
      return false;
    }
    const wram = this._require_wram("Whirlpool");
    const tileset_key = resolveTilesetName(this.tileset).toLowerCase();
    const replacements = _WHIRLPOOL_BLOCKS[tileset_key] ?? {};
    let current: number | null = null;
    try {
      current = this.map.getMetatileAt(x, y);
    } catch {
      current = null;
    }
    if (current === null) {
      await this._show_field_move_text_async("CantUseItemText");
      return false;
    }
    const entry = replacements[current];
    if (!entry) {
      await this._show_field_move_text_async("CantUseItemText");
      return false;
    }
    const [new_block] = entry;
    wram.wCutWhirlpoolOverworldBlockAddr = [x, y];
    wram.wCutWhirlpoolReplacementBlock = new_block;
    wram.wCutWhirlpoolAnimationType = current;

    if (this.event_manager) {
      const actor_name = this._field_move_actor_name(actor);
      const text = this._format_field_move_text("UseWhirlpoolText", actor_name);
      openText(this.event_manager);
      showText(this.event_manager, text);
      await this._wait_for_dialogue_render_async();
      await this._auto_close_field_move_dialogue_async();
    }

    this._write_metatile(x, y, new_block);
    this._run_whirlpool_animation(x, y);
    this.stop_player_movement?.();
    return true;
  }

  public async handle_waterfall(x: number, y: number, options: { from_menu?: boolean } = {}): Promise<boolean> {
    if (!this.map || !this.tileset) {
      return false;
    }
    const from_menu = Boolean(options.from_menu);
    const failure_text = from_menu ? "CantUseItemText" : "HugeWaterfallText";
    if (![PlayerState.SURF, PlayerState.SURF_PIKA].includes(this.player_state)) {
      await this._show_field_move_text_async(failure_text);
      return false;
    }
    if (!this._metatile_contains_collision(x, y, WATERFALL_COLLISIONS)) {
      if (from_menu) {
        await this._show_field_move_text_async(failure_text);
      }
      return false;
    }
    if (!this._party_has_move("WATERFALL") || !this.check_badge(7)) {
      await this._show_field_move_text_async(failure_text);
      return false;
    }
    if (String(this.player_direction ?? "").toLowerCase() !== "up") {
      await this._show_field_move_text_async(failure_text);
      return false;
    }

    if (!this.event_manager) {
      return false;
    }
    const [actor] = this._get_party_move_holder("WATERFALL");
    const actor_name = this._field_move_actor_name(actor);
    const ask_text = this._resolve_field_move_text("AskWaterfallText");
    openText(this.event_manager);
    showText(this.event_manager, ask_text);
    await this._wait_for_dialogue_render_async();
    const confirmed = await this._prompt_field_move_confirmation_async("WATERFALL");
    if (!confirmed) {
      closeText(this.event_manager);
      await this._wait_for_dialogue_closed_async();
      return false;
    }
    const use_text = this._format_field_move_text("UseWaterfallText", actor_name);
    showText(this.event_manager, use_text);
    await this._wait_for_dialogue_render_async();
    await this._auto_close_field_move_dialogue_async();
    this.audio_engine?.play_sound?.("SFX_BUBBLEBEAM");

    const steps_taken = this._run_waterfall_steps();
    if (steps_taken > 0) {
      this.player_state = PlayerState.SURF;
      const wram = this._require_wram("Waterfall");
      wram.surfing = true;
      this.stop_player_movement?.();
    }
    return steps_taken > 0;
  }

  protected _run_waterfall_steps(): number {
    const mover = (this as unknown as { move_player?: (direction: string, forced?: boolean) => void; movePlayer?: (direction: string, forced?: boolean) => void }).move_player
      ?? (this as unknown as { movePlayer?: (direction: string, forced?: boolean) => void }).movePlayer;
    if (typeof mover !== "function") {
      throw new Error("Waterfall requires the overworld movement hooks.");
    }
    const updater = (this as unknown as { update?: () => void }).update;
    if (typeof updater !== "function") {
      throw new Error("Waterfall requires the overworld update loop.");
    }

    const originalWalkFrames = (this as unknown as { WALK_FRAMES?: number }).WALK_FRAMES;
    const hasWalkFrames = typeof originalWalkFrames === "number";
    if (hasWalkFrames) {
      const bikeFrames = Math.max(1, Math.trunc(originalWalkFrames / 2));
      (this as unknown as { WALK_FRAMES?: number }).WALK_FRAMES = bikeFrames;
    }

    const movementHost = this as unknown as { _waterfall_movement_active?: boolean };
    const previousWaterfallMovementActive = movementHost._waterfall_movement_active;
    movementHost._waterfall_movement_active = true;

    let steps = 0;
    const maxSteps = Math.max(4, (this.map?.height ?? 0) * 2);
    try {
      while (steps < maxSteps) {
        mover.call(this, "up", true);
        if (!this.is_moving) {
          throw new Error("Waterfall movement failed to start.");
        }
        this._wait_for_waterfall_step(updater);
        steps += 1;
        const permission = this._current_player_collision();
        if (!WATERFALL_COLLISIONS.has(permission)) {
          break;
        }
      }
    } finally {
      movementHost._waterfall_movement_active = previousWaterfallMovementActive;
      if (hasWalkFrames) {
        (this as unknown as { WALK_FRAMES?: number }).WALK_FRAMES = originalWalkFrames;
      }
    }
    return steps;
  }

  protected _current_player_collision(): number {
    if (!this.map || !this.tileset) {
      throw new Error("Waterfall collision lookup requires map + tileset.");
    }
    return getCoordCollision(this.map, this.tileset, this.player_x, this.player_y);
  }

  protected _wait_for_waterfall_step(update: () => void): void {
    const maxFrames = Math.max(2, Math.trunc((this as unknown as { WALK_FRAMES?: number }).WALK_FRAMES ?? 8) * 4);
    const clock = new gameEngine.time.Clock();
    let frames = 0;
    while (this.is_moving) {
      update.call(this);
      this.draw?.();
      if (gameEngine.display.get_init()) {
        gameEngine.display.flip();
        clock.tick(GB_FRAME_RATE);
      }
      frames += 1;
      if (frames > maxFrames) {
        throw new Error("Waterfall step timed out; movement loop stalled.");
      }
    }
  }

  protected _run_blocking_frames(frameCount: number): void {
    const frames = Math.max(0, Math.trunc(frameCount));
    if (frames <= 0) {
      return;
    }
    const clock = new gameEngine.time.Clock();
    const fadeUpdater = (this as unknown as { _update_fade?: () => void })._update_fade;
    for (let frame = 0; frame < frames; frame += 1) {
      fadeUpdater?.call(this);
      this.draw?.();
      if (gameEngine.display.get_init()) {
        gameEngine.display.flip();
        clock.tick(GB_FRAME_RATE);
      }
    }
  }

  protected _wait_for_sfx_completion(soundId?: string | null): void {
    const engine = this.audio_engine;
    if (!engine) {
      return;
    }
    const isSoundPlaying = engine.isSoundPlaying ?? (engine as { is_sound_playing?: (name: string) => boolean }).is_sound_playing;
    if (typeof isSoundPlaying !== "function") {
      return;
    }
    const ids: string[] = [];
    const normalized = String(soundId ?? "").trim();
    if (normalized) {
      ids.push(normalized);
    } else {
      const activeIds =
        engine.getActiveSoundIds?.() ??
        (engine as { get_active_sound_ids?: () => string[] }).get_active_sound_ids?.();
      if (Array.isArray(activeIds)) {
        for (const key of activeIds) {
          if (key) {
            ids.push(String(key));
          }
        }
      }
    }
    if (!ids.length) {
      return;
    }
    let guard = 0;
    while (ids.some((name) => isSoundPlaying.call(engine, name))) {
      this._run_blocking_frames(1);
      guard += 1;
      if (guard > 600) {
        throw new Error(`SFX wait exceeded guard for ${ids.join(",")}.`);
      }
    }
  }

  public handle_strength(_x: number, _y: number): boolean {
    const [actor] = this._get_party_move_holder("STRENGTH");
    if (!actor) {
      return false;
    }
    if (!this.check_badge(2)) {
      return false;
    }
    const wram = this._require_wram("Strength");
    const bikeFlags = syncBikeFlags(wram);
    wram.wBikeFlags = bikeFlags | _BIKEFLAG_STRENGTH_ACTIVE;
    setBooleanFlag(wram.engine_flags, "ENGINE_STRENGTH_ACTIVE", true);
    const party = this.game_state?.sram?.party?.pokemon ?? [];
    const partyIndex = Math.max(0, Math.trunc(wram.wCurPartyMon ?? 0));
    const species = party[partyIndex]?.species?.id ?? actor.species?.id ?? "";
    wram.wStrengthSpecies = String(species ?? "");
    return true;
  }

  public async handle_flash(): Promise<boolean> {
    if (!this._party_has_move("FLASH")) {
      return false;
    }
    if (!this.check_badge(0)) {
      return false;
    }
    const game_state = this._require_game_state("Flash");
    const allow_flash = aerodactyl_chamber(
      game_state,
      { overworld: this } as unknown as Parameters<typeof aerodactyl_chamber>[1],
    );
    const map_attrs = this._current_map_attributes?.() ?? null;
    if (map_attrs && !allow_flash) {
      const label = this._normalise_time_of_day_label?.(map_attrs.time_of_day) ?? "";
      if (label !== "dark") {
        await this._show_field_move_text_async("CantUseItemText");
        return false;
      }
    }

    const map_name = this.current_map_name ?? "";
    if (!map_name) {
      return false;
    }
    const applyFlash = (): void => {
      this.fade_to_white?.(8);
      this._run_blocking_frames(8);
      const wram = this._require_wram("Flash");
      setBooleanFlag(wram.flash_active_maps, map_name, true);
      setBooleanFlag(wram.engine_flags, "STATUSFLAGS_FLASH", true);
      this._refresh_tileset_for_current_map?.(map_attrs ?? null);
      this.fade_from_white?.(8);
      this._run_blocking_frames(8);
    };

    if (this.event_manager) {
      let text = this._resolve_field_move_text("BlindingFlashText");
      if (text === "BlindingFlashText") {
        text = "A blinding FLASH lights the area!";
      }
      openText(this.event_manager);
      showText(this.event_manager, text);
      await this._wait_for_dialogue_render_async();
      this._wait_for_sfx_completion("SFX_FLASH");
      this.audio_engine?.play_sound?.("SFX_FLASH");
      this._wait_for_sfx_completion("SFX_FLASH");
      applyFlash();
      closeText(this.event_manager);
      await this._wait_for_dialogue_closed_async();
      return true;
    } else {
      applyFlash();
      return true;
    }
  }

  public async handle_fly(_x: number, _y: number): Promise<boolean> {
    if (!this._party_has_move("FLY")) {
      return false;
    }
    if (!this.check_badge(5)) {
      return false;
    }
    const environment = getMapEnvironment(this.current_map_name);
    if (!_FLY_ALLOWED_ENVIRONMENTS.has(String(environment ?? "").toUpperCase())) {
      return false;
    }
    const destinations = this._available_fly_destinations();
    if (!destinations.length) {
      return false;
    }
    const labels = destinations.map((destination) => destination.label);
    const initialIndex = Math.max(0, destinations.findIndex((destination) => destination.default));
    const selection = await this._select_fly_destination_async(labels, initialIndex, destinations);
    if (selection < 0 || selection >= destinations.length) {
      return false;
    }

    const { spawn } = destinations[selection];
    const game_state = this._require_game_state("Fly");
    const wram = this._require_wram("Fly");
    this._start_fly_from_animation();
    applySpawn(game_state, spawn);
    const spawn_point = getSpawnPoint(spawn);
    this.load_map?.(spawn_point.mapName);
    jumpRoamMons(game_state);
    this.clear_pending_white_fade?.();

    const tile_x = wram.wXCoord;
    const tile_y = wram.wYCoord;
    const stride = Math.max(1, Math.trunc(this.TILES_PER_COLLISION ?? 1));
    const min_tile = stride - 1;
    let max_tile_x = tile_x;
    let max_tile_y = tile_y;
    if (this.map) {
      max_tile_x = this.map.width * METATILE_WIDTH - 1;
      max_tile_y = this.map.height * METATILE_WIDTH - 1;
    }
    const scaled_x = scaleTileCoord(tile_x, stride);
    const scaled_y = scaleTileCoord(tile_y, stride);
    this.player_x = Math.max(min_tile, Math.min(scaled_x, max_tile_x));
    this.player_y = Math.max(min_tile, Math.min(scaled_y, max_tile_y));
    this.prev_player_x = this.player_x;
    this.prev_player_y = this.player_y;
    this.stop_player_movement?.();
    this.player_state = PlayerState.NORMAL;
    wram.surfing = false;
    this.player_direction = "down";
    this._queue_fly_to_animation(this.player_x, this.player_y, this.player_direction);
    return true;
  }

  public async handle_teleport(): Promise<boolean> {
    if (!this._teleport_ready()) {
      await this._show_field_move_text_async("CantUseTeleportText");
      return false;
    }
    if (this.event_manager) {
      const text = this._resolve_field_move_text("TeleportReturnText");
      openText(this.event_manager);
      showText(this.event_manager, text);
      await this._wait_for_dialogue_render_async();
      gameEngine.time.delay(60 * FIELD_MOVE_FRAME_DURATION_MS);
      closeText(this.event_manager);
      await this._wait_for_dialogue_closed_async();
    }
    this._perform_teleport_sequence();
    return true;
  }

  protected _queue_sweet_scent_battle(encounter: WildEncounter, battle_type: string): void {
    const wram = this._require_wram("Sweet scent");
    wram.wSweetScentState = 1;
    wram.wSweetScentTarget = {
      species: encounter.species,
      level: encounter.level,
      battle_type,
    };
    wram.wSweetScentStepTimer = this._SWEET_SCENT_TRIGGER_FRAMES;
    this._sweet_scent_pending = { encounter, battle_type };

    const tile_x = Math.trunc((this.player_x ?? 0) / METATILE_WIDTH);
    const tile_y = Math.trunc((this.player_y ?? 0) / METATILE_WIDTH);
    try {
      this._start_field_move_animation("SWEET_SCENT", this._SWEET_SCENT_TRIGGER_FRAMES, tile_x, tile_y, {
        metadata: { palette: "fade" },
      });
    } catch {
      // Keep timing even if animation dispatch fails.
    }
    this.audio_engine?.play_sound?.("SFX_SWEET_SCENT");
    this._run_field_move_delay(this._SWEET_SCENT_TRIGGER_FRAMES);
  }

  public async handle_sweet_scent(pokemon: Pokemon | null = null): Promise<boolean> {
    if (!this.event_manager) {
      return false;
    }
    const actor_name = this._field_move_actor_name(pokemon);
    const text = this._format_field_move_text("UseSweetScentText", actor_name);
    openText(this.event_manager);
    showText(this.event_manager, text);
    await this._wait_for_dialogue_render_async();
    waitForInput(this.event_manager);
    await this._wait_for_dialogue_ack_async();

    const game_state = this._require_game_state("Sweet scent");
    const wram = this._require_wram("Sweet scent");
    const rng = new HardwareRNG(game_state);
    const encounter_data = this._sweet_scent_encounter(rng);
    if (!encounter_data) {
      wram.battle_type = "BATTLETYPE_NORMAL";
      wram.wSweetScentState = 0;
      wram.wSweetScentTarget = undefined;
      wram.wSweetScentStepTimer = 0;
      showText(this.event_manager, this._resolve_field_move_text("SweetScentNothingText"));
      await this._wait_for_dialogue_render_async();
      waitForInput(this.event_manager);
      await this._wait_for_dialogue_ack_async();
      closeText(this.event_manager);
      await this._wait_for_dialogue_closed_async();
      return false;
    }
    closeText(this.event_manager);
    await this._wait_for_dialogue_closed_async();
    const [encounter, battle_type] = encounter_data;
    wram.battle_type = battle_type;
    this._queue_sweet_scent_battle(encounter, battle_type);
    return true;
  }

  public async handle_dig(pokemon: Pokemon | null = null): Promise<boolean> {
    const wram = this._require_wram("Dig");
    wram.wEscapeRopeOrDigType = 2;
    const actor_name = this._field_move_actor_name(pokemon);
    if (!this._dig_ready()) {
      await this._show_field_move_text_async("CantUseDigText");
      return false;
    }
    await this._show_field_move_text_async("UseDigText", actor_name);
    this._perform_dig_escape_rope_sequence();
    return true;
  }

  public async handle_escape_rope(pokemon: Pokemon | null = null): Promise<boolean> {
    const game_state = this._require_game_state("Escape Rope");
    const wram = this._require_wram("Escape Rope");
    wram.wEscapeRopeOrDigType = 1;
    const actor_name = this._field_move_actor_name(pokemon);
    if (!this._dig_ready()) {
      await this._show_field_move_text_async("CantUseDigText");
      return false;
    }
    await this._show_field_move_text_async("UseEscapeRopeText", actor_name);
    const overworld = this as unknown as NonNullable<Parameters<typeof kabuto_chamber>[1]>["overworld"];
    kabuto_chamber(game_state, { overworld });
    this._perform_dig_escape_rope_sequence();
    return true;
  }

  public async handle_rock_smash(pokemon: Pokemon | null = null): Promise<boolean> {
    const wram = this._require_wram("Rock Smash");
    if (wram.wRockSmashState !== 0) {
      return false; // Rock Smash already in progress
    }

    let actor = pokemon;
    if (!actor) {
      [actor] = this._get_party_move_holder("ROCK_SMASH");
    }
    if (!actor) {
      await this._show_field_move_text_async("CantUseRockSmashText");
      return false;
    }

    const facing = this.get_facing_tile_coords?.();
    if (!facing) {
      return false;
    }
    let [facing_x, facing_y] = facing;
    const adjust = this._counter_adjusted_tile ?? null;
    if (typeof adjust === "function") {
      [facing_x, facing_y] = adjust(facing_x, facing_y);
    }
    const npc_lookup = this as unknown as {
      _npc_occupying_subtile?: (x: number, y: number) => OverworldObject | null;
    };
    const rock = npc_lookup._npc_occupying_subtile?.(facing_x, facing_y) ?? null;
    if (!rock) {
      return false;
    }
    const rock_state = rock as unknown as {
      walking?: boolean;
      jumping?: boolean;
    };
    // ASM: CheckFacingObject ignores walking/jumping map objects.
    if (rock_state.walking || rock_state.jumping) {
      return false;
    }
    const movement = String(rock.event?.spritemovedata ?? "").toUpperCase();
    if (!movement.includes("SMASHABLE_ROCK")) {
      return false;
    }
    if (typeof rock.objectIndex === "number" && rock.objectIndex > 0) {
      wram.last_talked = rock.objectIndex;
    }

    const actor_name = this._field_move_actor_name(actor);
    await this._show_field_move_text_async("UseRockSmashText", actor_name);
    this._play_field_move_sound("SFX_STRENGTH");

    const runner = {
      overworld: this,
      game_state: this._require_game_state("Rock Smash"),
    } as unknown as Parameters<typeof performRockSmash>[0];
    const result = performRockSmash(runner);
    if (!result.smashed) {
      return false;
    }

    this._run_field_move_delay(this._ROCK_SMASH_BREAK_FRAMES);
    return true;
  }

  protected _select_rock_smash_encounter(rng: HardwareRNG): WildEncounter | null {
    const manager = this._wild_encounters ?? null;
    if (manager === null) {
      return null;
    }
    const data = manager._lookup_map_data(this.current_map_name);
    if (data === null) {
      return null;
    }
    const table = manager._resolve_table(data, EncounterSurface.ROCK);
    if (!table) {
      return null;
    }
    const slot = manager._choose_slot(EncounterSurface.ROCK, table.length, rng);
    if (slot === null) {
      return null;
    }
    const encounter = table[slot];
    return { level: encounter.level, species: encounter.species };
  }

  protected _party_has_move(move_name: string): boolean {
    const [pokemon] = this._get_party_move_holder(move_name);
    return pokemon !== null;
  }

  protected _get_party_move_holder(move_name: string): [Pokemon | null, string] {
    const target = move_name.toUpperCase();
    const party = this.game_state?.sram?.party?.pokemon ?? [];
    for (let index = 0; index < party.length; index += 1) {
      const pokemon = party[index];
      if (!pokemon) {
        continue;
      }
      for (const move of pokemon.moves ?? []) {
        if (!move) {
          continue;
        }
        const move_id = normalizeMoveName(move.name);
        if (!move_id) {
          continue;
        }
        if (move_id.toUpperCase() !== target) {
          continue;
        }
        const wram = this.game_state?.wram ?? null;
        if (wram) {
          // ASM: CheckPartyMove updates wCurPartyMon for downstream field-move scripts.
          wram.wCurPartyMon = index;
          wram.wCurPartySpecies = String(pokemon.species?.id ?? "");
        }
        let nickname = String(pokemon.nickname ?? "").trim();
        if (!nickname) {
          const species = pokemon.species ?? null;
          nickname = String(species?.id ?? "").trim();
        }
        const clean_name = String(nickname || target).trim();
        return [toPokemon(pokemon), clean_name];
      }
    }
    return [null, ""];
  }

  protected _facing_metatile_coordinates(): [number, number] | null {
    if (!this.get_facing_tile_coords) {
      return null;
    }
    let [tile_x, tile_y] = this.get_facing_tile_coords();
    if (tile_x < 0 || tile_y < 0) {
      return null;
    }
    const adjust = this._counter_adjusted_tile ?? null;
    if (typeof adjust === "function") {
      [tile_x, tile_y] = adjust.call(this, tile_x, tile_y);
    }
    if (!this.map) {
      return null;
    }
    const metatile_x = Math.trunc(tile_x / METATILE_WIDTH);
    const metatile_y = Math.trunc(tile_y / METATILE_WIDTH);
    if (metatile_x < 0 || metatile_x >= this.map.width) {
      return null;
    }
    if (metatile_y < 0 || metatile_y >= this.map.height) {
      return null;
    }
    return [metatile_x, metatile_y];
  }

  protected _resolve_field_move_text(label: string): string {
    let text = "";
    const loader = this.data_loader ?? null;
    if (loader) {
      try {
        text =
          loader.get_text?.(label) ??
          loader.getText?.(label) ??
          loader.getTextByLabel?.(label) ??
          "";
      } catch (error) {
        throw new Error(
          `Failed to resolve ASM field-move text for label '${label}': ${error instanceof Error ? error.message : String(error)}`
        );
      }
    }
    if (!text || !String(text).trim()) {
      text = asmTextLoader.get(label) || asmTextLoader.get(`_${label}`) || STANDARD_TEXT_FALLBACKS[label] || "";
    }
    if (!text || !String(text).trim() || String(text).trim() === label) {
      throw new Error(`Missing ASM field-move text for label '${label}'.`);
    }
    return text;
  }

  protected _format_field_move_text(label: string, actor_name: string | null = null): string {
    const text = this._resolve_field_move_text(label);
    if (!this.game_state) {
      return actor_name ? text.replace(/<STRING_BUFFER_\d+>|@/g, actor_name) : text;
    }
    const formatter = new TextFormatter(this.game_state);
    if (actor_name) {
      formatter.stringBuffers = {
        STRING_BUFFER_1: actor_name,
        STRING_BUFFER_2: actor_name,
        STRING_BUFFER_3: actor_name,
        STRING_BUFFER_4: actor_name,
      };
    }
    return formatter.formatText(text);
  }

  protected _resolve_waterfall_destination(metatile_x: number, metatile_y: number): [number, number, number] {
    void metatile_x;
    void metatile_y;
    if (!this.map || !this.tileset) {
      return [this.player_x, this.player_y, 0];
    }
    if (!this._direction_to_vector) {
      return [this.player_x, this.player_y, 0];
    }
    let dx: number;
    let dy: number;
    try {
      [dx, dy] = this._direction_to_vector(this.player_direction);
    } catch {
      return [this.player_x, this.player_y, 0];
    }

    const stride = this.TILES_PER_COLLISION;
    let current_x = this.player_x;
    let current_y = this.player_y;
    let steps_taken = 0;

    while (true) {
      const next_x = current_x + dx * stride;
      const next_y = current_y + dy * stride;
      let permission: number;
      try {
        permission = getCoordCollision(this.map, this.tileset, next_x, next_y);
      } catch {
        break;
      }
      if (WATERFALL_COLLISIONS.has(permission)) {
        current_x = next_x;
        current_y = next_y;
        steps_taken += 1;
        continue;
      }
      const attributes = describeCollision(permission);
      if (attributes.terrain === Terrain.WATER) {
        current_x = next_x;
        current_y = next_y;
      }
      break;
    }

    return [current_x, current_y, steps_taken];
  }

  protected _show_field_move_text(label: string, actor_name: string | null = null): void {
    if (!this.event_manager) {
      return;
    }
    const text = this._format_field_move_text(label, actor_name);
    openText(this.event_manager);
    showText(this.event_manager, text);
    this._wait_for_dialogue_render();
    waitForInput(this.event_manager);
    this._wait_for_dialogue_ack();
    closeText(this.event_manager);
    this._wait_for_dialogue_closed();
  }

  protected async _show_field_move_text_async(label: string, actor_name: string | null = null): Promise<void> {
    if (!this.event_manager) {
      return;
    }
    const text = this._format_field_move_text(label, actor_name);
    openText(this.event_manager);
    showText(this.event_manager, text);
    await this._wait_for_dialogue_render_async();
    waitForInput(this.event_manager);
    await this._wait_for_dialogue_ack_async();
    closeText(this.event_manager);
    await this._wait_for_dialogue_closed_async();
  }

  protected _teleport_ready(): boolean {
    const environment = getMapEnvironment(this.current_map_name);
    if (!["ROUTE", "TOWN"].includes(String(environment ?? ""))) {
      return false;
    }
    const wram = this._require_wram("Teleport");
    const last_group = wram.wLastSpawnMapGroup;
    const last_number = wram.wLastSpawnMapNumber;
    if (last_group === 0 && last_number === 0) {
      return false;
    }
    return findSpawnForMap(last_group, last_number) !== undefined;
  }

  protected _bug_contest_active(): boolean {
    const wram = this._require_wram("Bug contest");
    const flags = wram.engine_flags;
    if (getBooleanFlag(flags, _BUG_CONTEST_TIMER_FLAG)) {
      return true;
    }
    const contest_state = wram.bug_contest_state ?? null;
    return Boolean(contest_state?.timer_active);
  }

  protected _dig_ready(): boolean {
    const environment = getMapEnvironment(this.current_map_name);
    return _DIG_ALLOWED_ENVIRONMENTS.has(String(environment ?? "")) && Boolean(this._resolve_dig_warp());
  }

  protected _resolve_dig_warp(): [MapMetadata, WarpEvent] | null {
    const wram = this._require_wram("Dig");
    const warp_number = wram.wDigWarpNumber ?? 0;
    const map_group = wram.wDigMapGroup ?? 0;
    const map_number = wram.wDigMapNumber ?? 0;
    if (warp_number === 0 || map_group === 0 || map_number === 0) {
      return null;
    }
    const metadata = getMapMetadataByGroup(map_group, map_number);
    if (!metadata) {
      return null;
    }
    const events = getMapEvents(this.data_loader, metadata.name);
    if (!isMapEvents(events)) {
      return null;
    }
    const warp_candidates = events.warps ?? [];
    const target_warp = warp_candidates.find((warp) => warp.index === warp_number) ?? null;
    if (!target_warp) {
      return null;
    }
    return [metadata, target_warp];
  }

  protected _warp_to_dig_destination(): boolean {
    const destination = this._resolve_dig_warp();
    if (!destination) {
      return false;
    }
    const [metadata, target_warp] = destination;
    const warp_number = target_warp.index;
    const wram = this._require_wram("Dig");
    const prev_group = wram.wMapGroup;
    const prev_number = wram.wMapNumber;
    wram.wBackupMapGroup = prev_group;
    wram.wBackupMapNumber = prev_number;
    wram.wPrevWarp = warp_number;
    wram.wPrevMapGroup = prev_group;
    wram.wPrevMapNumber = prev_number;
    wram.wMapGroup = metadata.groupId;
    wram.wMapNumber = metadata.mapId;
    wram.current_map_group = metadata.groupId;
    wram.current_map_id = metadata.mapId;
    wram.wNextWarp = warp_number;
    wram.wNextMapGroup = metadata.groupId;
    wram.wNextMapNumber = metadata.mapId;

    const [tile_x, tile_y] = warpTilePosition(target_warp, this.TILES_PER_COLLISION);
    this.player_x = tile_x;
    this.player_y = tile_y;
    this.prev_player_x = tile_x;
    this.prev_player_y = tile_y;
    this.target_tile_x = tile_x;
    this.target_tile_y = tile_y;
    this.is_moving = false;
    this._queued_direction = null;
    this.load_map?.(metadata.name);
    return true;
  }

  protected _perform_teleport_sequence(): void {
    this._play_field_move_sound("SFX_WARP_TO");
    const game_state = this._require_game_state("Teleport");
    const overworld = this as unknown as Parameters<typeof warp_to_spawn_point>[1]["overworld"];
    if (!warp_to_spawn_point(game_state, { overworld })) {
      throw new Error("Teleport failed to resolve a spawn warp target.");
    }
    this._play_field_move_sound("SFX_WARP_FROM");
    this._finalize_field_move_transition();
  }

  protected _perform_dig_escape_rope_sequence(): void {
    this._play_field_move_sound("SFX_WARP_TO");
    if (!this._warp_to_dig_destination()) {
      throw new Error("Dig/Escape Rope requested without a valid dig warp destination.");
    }
    this._play_field_move_sound("SFX_WARP_FROM");
    this._finalize_field_move_transition();
  }

  protected _finalize_field_move_transition(): void {
    const wram = this._require_wram("Field move transition");
    wram.surfing = false;
    this.player_state = PlayerState.NORMAL;
    this.target_tile_x = this.player_x;
    this.target_tile_y = this.player_y;
    this.prev_player_x = this.player_x;
    this.prev_player_y = this.player_y;
    this.is_moving = false;
    this._queued_direction = null;
    this.player_direction = "down";
    this.stop_player_movement?.();
    this._sync_player_state?.();
  }

  protected _play_field_move_sound(sound_id: string): void {
    playOverworldSound(this.audio_engine, sound_id);
  }

  protected _field_move_actor_name(pokemon: Pokemon | null): string {
    if (!pokemon) {
      return "POKEMON";
    }
    const nickname = String(pokemon.nickname ?? "").trim();
    if (nickname) {
      return nickname.toUpperCase();
    }
    const species = pokemon.species ?? null;
    const species_name = String(species?.id ?? "").trim();
    return (species_name || "POKEMON").toUpperCase();
  }

  protected _run_field_move_delay(frames: number): void {
    if (frames <= 0) {
      return;
    }
    for (let i = 0; i < frames; i += 1) {
      this._tick_field_move_states();
      this._tick_field_move_animation_queue();
      gameEngine.time.delay(FIELD_MOVE_FRAME_DURATION_MS);
    }
    this._tick_field_move_states();
  }

  protected _run_field_move_animation_frames(frameCount: number): void {
    const frames = Math.max(0, Math.trunc(frameCount));
    if (frames <= 0) {
      return;
    }
    const clock = new gameEngine.time.Clock();
    const fadeUpdater = (this as unknown as { _update_fade?: () => void })._update_fade;
    for (let frame = 0; frame < frames; frame += 1) {
      fadeUpdater?.call(this);
      this._tick_field_move_states();
      this._tick_field_move_animation_queue();
      this.draw?.();
      if (gameEngine.display.get_init()) {
        gameEngine.display.flip();
        clock.tick(GB_FRAME_RATE);
      }
    }
  }

  protected async _run_field_move_animation_frames_async(frameCount: number): Promise<void> {
    const frames = Math.max(0, Math.trunc(frameCount));
    if (frames <= 0) {
      return;
    }
    const fadeUpdater = (this as unknown as { _update_fade?: () => void })._update_fade;
    for (let frame = 0; frame < frames; frame += 1) {
      fadeUpdater?.call(this);
      this._tick_field_move_states();
      this._tick_field_move_animation_queue();
      this.draw?.();
      if (gameEngine.display.get_init()) {
        gameEngine.display.flip();
      }
      await this._await_field_move_frame_async();
    }
  }

  protected async _with_field_move_input_capture_async<T>(run: () => Promise<T>): Promise<T> {
    const input_owner = this as unknown as { input_capture_active?: boolean };
    const had_capture_flag = typeof input_owner.input_capture_active === "boolean";
    const previous_capture = input_owner.input_capture_active;
    if (had_capture_flag) {
      input_owner.input_capture_active = true;
    }
    try {
      return await run();
    } finally {
      if (had_capture_flag) {
        input_owner.input_capture_active = previous_capture;
      }
    }
  }

  protected _tick_field_move_states(): void {
    const wram = this._require_wram("Field move tick");
    if (this._sweet_scent_pending === null) {
      this._sweet_scent_pending = undefined;
    }
    if (this._pending_rock_smash === null) {
      this._pending_rock_smash = undefined;
    }
    this._tick_sweet_scent_state(wram);
    this._tick_rock_smash_state(wram);
  }

  protected _tick_sweet_scent_state(wram: GameState["wram"]): void {
    const pending = this._sweet_scent_pending;
    const timer = Math.max(0, Number(wram.wSweetScentStepTimer ?? 0));
    if (timer > 0) {
      wram.wSweetScentStepTimer = timer - 1;
      return;
    }
    if (wram.wSweetScentState === 0) {
      return;
    }
    wram.wSweetScentState = 0;
    wram.wSweetScentStepTimer = 0;
    wram.wSweetScentTarget = undefined;
    this._sweet_scent_pending = undefined;
    if (!pending) {
      return;
    }
    const encounter = pending.encounter;
    let battle_type = pending.battle_type ?? "BATTLETYPE_NORMAL";
    if (!encounter || encounter.species === undefined || encounter.level === undefined) {
      return;
    }
    if (typeof battle_type !== "string") {
      battle_type = "BATTLETYPE_NORMAL";
    }
    this._start_wild_battle(encounter.species, encounter.level, battle_type);
  }

  protected _tick_rock_smash_state(wram: GameState["wram"]): void {
    const pending = this._pending_rock_smash;
    const timer = Math.max(0, Number(wram.wRockSmashStepTimer ?? 0));
    if (timer > 0) {
      wram.wRockSmashStepTimer = timer - 1;
      return;
    }
    if (wram.wRockSmashState === 0) {
      return;
    }
    wram.wRockSmashState = 0;
    wram.wRockSmashStepTimer = 0;
    wram.wRockSmashTile = undefined;
    const encounter = wram.wRockSmashEncounter;
    wram.wRockSmashEncounter = undefined;
    this._pending_rock_smash = undefined;
    if (pending && pending.object_id != null && this.remove_object) {
      this.remove_object(pending.object_id, { update_event_flag: true });
    }
    if (encounter) {
      const species = String(encounter.species ?? "");
      const level = Number(encounter.level ?? 0);
      if (!species || !Number.isFinite(level) || level <= 0) {
        return;
      }
      this._start_wild_battle(species, level, "BATTLETYPE_NORMAL");
    }
  }

  protected _wait_for_dialogue_render(): void {
    if (typeof globalThis.window !== "undefined") {
      throw new Error("Synchronous _wait_for_dialogue_render is not supported in the browser. Use _wait_for_dialogue_render_async instead.");
    }
    const dialogue = this.dialogue as (OverworldDialogue & FieldDialogueInternals) | null;
    if (!dialogue) {
      return;
    }
    const window = dialogue.window ?? null;
    if (!window) {
      return;
    }
    if (this._field_move_auto_advance) {
      window.complete?.();
      return;
    }
    const clock = new gameEngine.time.Clock();
    while (!window.is_complete?.()) {
      this._process_dialogue_events(dialogue);
      dialogue.update?.();
      this._tick_field_move_animation_queue();
      this.draw?.();
      if (gameEngine.display.get_init()) {
        gameEngine.display.flip();
      }
      clock.tick(GB_FRAME_RATE);
    }
  }

  protected async _wait_for_dialogue_render_async(): Promise<void> {
    const dialogue = this.dialogue as (OverworldDialogue & FieldDialogueInternals) | null;
    if (!dialogue) {
      return;
    }
    const window = dialogue.window ?? null;
    if (!window) {
      return;
    }
    if (this._field_move_auto_advance) {
      window.complete?.();
      return;
    }
    while (!window.is_complete?.()) {
      this._process_dialogue_events(dialogue);
      dialogue.update?.();
      this._tick_field_move_animation_queue();
      this.draw?.();
      if (gameEngine.display.get_init()) {
        gameEngine.display.flip();
      }
      await this._await_field_move_frame_async();
    }
  }

  protected _wait_for_dialogue_ack(): void {
    if (typeof globalThis.window !== "undefined") {
      throw new Error("Synchronous _wait_for_dialogue_ack is not supported in the browser. Use _wait_for_dialogue_ack_async instead.");
    }
    const dialogue = this.dialogue as (OverworldDialogue & FieldDialogueInternals) | null;
    if (!dialogue) {
      return;
    }
    if (this._field_move_auto_advance) {
      dialogue.waiting_for_input = false;
      if (typeof dialogue.pendingWaits === "number") {
        dialogue.pendingWaits = 0;
      }
      const pending_scripts = dialogue.pending_script_waits ?? 0;
      if (pending_scripts) {
        dialogue.pending_script_waits = 0;
        dialogue.script_runner?.resume?.();
      }
      dialogue.script_paused = false;
      return;
    }
    const clock = new gameEngine.time.Clock();
    while (dialogue.waiting_for_input) {
      if (!dialogue.active) {
        dialogue.waiting_for_input = false;
        if (typeof dialogue.pendingWaits === "number") {
          dialogue.pendingWaits = 0;
        }
        dialogue.script_paused = false;
        break;
      }
      this._process_dialogue_events(dialogue);
      dialogue.update?.();
      this._tick_field_move_animation_queue();
      this.draw?.();
      if (gameEngine.display.get_init()) {
        gameEngine.display.flip();
      }
      clock.tick(GB_FRAME_RATE);
    }
  }

  protected async _wait_for_dialogue_ack_async(): Promise<void> {
    const dialogue = this.dialogue as (OverworldDialogue & FieldDialogueInternals) | null;
    if (!dialogue) {
      return;
    }
    if (this._field_move_auto_advance) {
      dialogue.waiting_for_input = false;
      if (typeof dialogue.pendingWaits === "number") {
        dialogue.pendingWaits = 0;
      }
      const pending_scripts = dialogue.pending_script_waits ?? 0;
      if (pending_scripts) {
        dialogue.pending_script_waits = 0;
        dialogue.script_runner?.resume?.();
      }
      dialogue.script_paused = false;
      return;
    }
    while (dialogue.waiting_for_input) {
      if (!dialogue.active) {
        dialogue.waiting_for_input = false;
        if (typeof dialogue.pendingWaits === "number") {
          dialogue.pendingWaits = 0;
        }
        dialogue.script_paused = false;
        break;
      }
      this._process_dialogue_events(dialogue);
      dialogue.update?.();
      this._tick_field_move_animation_queue();
      this.draw?.();
      if (gameEngine.display.get_init()) {
        gameEngine.display.flip();
      }
      await this._await_field_move_frame_async();
    }
  }

  protected _wait_for_dialogue_closed(): void {
    if (typeof globalThis.window !== "undefined") {
      throw new Error("Synchronous _wait_for_dialogue_closed is not supported in the browser. Use _wait_for_dialogue_closed_async instead.");
    }
    const dialogue = this.dialogue ?? null;
    if (!dialogue || this._field_move_auto_advance) {
      return;
    }
    const clock = new gameEngine.time.Clock();
    while (dialogue.active) {
      this._process_dialogue_events(dialogue);
      dialogue.update?.();
      this._tick_field_move_animation_queue();
      this.draw?.();
      if (gameEngine.display.get_init()) {
        gameEngine.display.flip();
      }
      clock.tick(GB_FRAME_RATE);
    }
  }

  protected async _wait_for_dialogue_closed_async(): Promise<void> {
    const dialogue = this.dialogue ?? null;
    if (!dialogue || this._field_move_auto_advance) {
      return;
    }
    while (dialogue.active) {
      this._process_dialogue_events(dialogue);
      dialogue.update?.();
      this._tick_field_move_animation_queue();
      this.draw?.();
      if (gameEngine.display.get_init()) {
        gameEngine.display.flip();
      }
      await this._await_field_move_frame_async();
    }
  }

  protected _auto_close_field_move_dialogue(): void {
    if (!this.event_manager) {
      return;
    }
    const previous_auto_advance = this._field_move_auto_advance;
    this._field_move_auto_advance = true;
    try {
      waitForInput(this.event_manager);
      this._wait_for_dialogue_ack();
      closeText(this.event_manager);
      this._wait_for_dialogue_closed();
    } finally {
      this._field_move_auto_advance = previous_auto_advance;
    }
  }

  protected async _auto_close_field_move_dialogue_async(): Promise<void> {
    if (!this.event_manager) {
      return;
    }
    const previous_auto_advance = this._field_move_auto_advance;
    this._field_move_auto_advance = true;
    try {
      waitForInput(this.event_manager);
      await this._wait_for_dialogue_ack_async();
      closeText(this.event_manager);
      await this._wait_for_dialogue_closed_async();
    } finally {
      this._field_move_auto_advance = previous_auto_advance;
    }
  }

  protected _process_dialogue_events(dialogue: FieldMoveDialogue): void {
    for (const event of gameEngine.event.get(this.ui?.eventQueue)) {
      if (event.type === gameEngine.QUIT) {
        gameEngine.quit();
        throw new Error("Quit requested during field-move dialogue.");
      }
      dialogue.handle_input?.(event);
    }
  }

  protected get _field_move_auto_advance(): boolean {
    return this._field_move_auto_advance_flag;
  }

  protected set _field_move_auto_advance(value: boolean) {
    this._field_move_auto_advance_flag = Boolean(value);
  }

  protected _prompt_field_move_confirmation(move_name: string): boolean {
    const callback = this._field_move_confirm_callback ?? null;
    if (typeof callback === "function") {
      try {
        return Boolean(callback(move_name));
      } catch {
        return false;
      }
    }
    if (this._field_move_auto_advance) {
      return true;
    }
    if (!this.ui) {
      return true;
    }
    if (typeof globalThis.window !== "undefined") {
      throw new Error("Synchronous _prompt_field_move_confirmation is not supported in the browser. Use _prompt_field_move_confirmation_async instead.");
    }
    const prompt = new YesNoPrompt(this.ui, this.audio_engine ?? null);
    const dialogue = this.dialogue ?? null;
    const clock = new gameEngine.time.Clock();
    while (!prompt.finished) {
      for (const event of gameEngine.event.get(this.ui?.eventQueue)) {
        if (event.type === gameEngine.QUIT) {
          gameEngine.quit();
          throw new Error("Quit requested during field-move confirmation.");
        }
        prompt.handle_input(event as KeyboardEvent);
        dialogue?.handle_input?.(event);
      }
      dialogue?.update?.();
      this.draw?.();
      prompt.draw();
      if (gameEngine.display.get_init()) {
        gameEngine.display.flip();
      }
      clock.tick(GB_FRAME_RATE);
    }
    return prompt.result();
  }

  protected async _prompt_field_move_confirmation_async(move_name: string): Promise<boolean> {
    const callback = this._field_move_confirm_callback ?? null;
    if (typeof callback === "function") {
      try {
        return Boolean(callback(move_name));
      } catch {
        return false;
      }
    }
    if (this._field_move_auto_advance) {
      return true;
    }
    if (!this.ui) {
      return true;
    }
    const dialogue = this.dialogue ?? null;
    const event_manager = this.event_manager ?? null;
    const has_prompt_listener =
      typeof event_manager?.hasListener === "function"
        ? event_manager.hasListener("prompt_yes_no")
        : Boolean(event_manager);
    if (dialogue && event_manager && has_prompt_listener) {
      let result: boolean | null = null;
      event_manager.dispatch(new Event("prompt_yes_no", {
        callback: (confirmed: boolean) => {
          result = Boolean(confirmed);
        },
      }));
      while (result === null) {
        this._process_dialogue_events(dialogue);
        dialogue.update?.();
        this._tick_field_move_animation_queue();
        this.draw?.();
        dialogue.draw?.();
        if (gameEngine.display.get_init()) {
          gameEngine.display.flip();
        }
        await this._await_field_move_frame_async();
      }
      return result;
    }
    return this._prompt_field_move_confirmation_direct_async(move_name);
  }

  protected async _prompt_field_move_confirmation_direct_async(_move_name: string): Promise<boolean> {
    if (!this.ui) {
      return true;
    }
    return await this._with_field_move_input_capture_async(async () => {
      const prompt = new YesNoPrompt(this.ui!, this.audio_engine ?? null);
      const dialogue = this.dialogue ?? null;
      while (!prompt.finished) {
        for (const event of gameEngine.event.get(this.ui?.eventQueue)) {
          if (event.type === gameEngine.QUIT) {
            gameEngine.quit();
            throw new Error("Quit requested during field-move confirmation.");
          }
          prompt.handle_input(event as KeyboardEvent);
          dialogue?.handle_input?.(event);
        }
        dialogue?.update?.();
        this.draw?.();
        prompt.draw();
        if (gameEngine.display.get_init()) {
          gameEngine.display.flip();
        }
        await this._await_field_move_frame_async();
      }
      return prompt.result();
    });
  }

  protected _await_field_move_frame_async(): Promise<void> {
    return this._field_move_frame_awaiter();
  }
}

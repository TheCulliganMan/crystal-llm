// ASM mapping: pokecrystal_disassembly/engine/overworld/map_objects.asm (LoadMapAttributes/LoadMapObjects/warps).
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { gameEngine, Rect, Surface } from "@pokecrystal/core/ui/game-engine";
import type { BackgroundEvent, MapAttributes, MapConnection, MapEvents, ObjectEvent, WarpEvent } from "@pokecrystal/core/core/models/map";
import { FacingDirection } from "@pokecrystal/core/core/enums/overworld";
import {
  getMapEnvironment,
  getMapMetadataByConstant,
  getMapMetadataByGroup,
  getMapMetadataByName,
  mapConstantToName,
} from "@pokecrystal/core/engine/world/maps";
import type { MapMetadata } from "@pokecrystal/core/engine/world/maps";
import { build_connection_composite, CompositeSegment } from "@pokecrystal/core/engine/world/overworld/connection-composite";
import {
  build_overworld_map,
  create_map_surface,
  create_priority_surface,
  render_map_onto_surface,
  render_priority_onto_surface,
} from "@pokecrystal/core/engine/world/overworld/map-geometry";
import { sampleCollision } from "@pokecrystal/core/engine/world/overworld/collision-rules";
import { METATILE_SIZE, METATILE_WIDTH } from "@pokecrystal/core/engine/world/tile/constants";
import { updateRoamMons } from "@pokecrystal/core/engine/world/roamers";
import {
  directionalWarpFacing,
  isDirectionalWarp,
  isPitPermission,
  isWarpPermission,
  warpSoundForPermission,
} from "@pokecrystal/core/engine/world/overworld/tile-events";

import { endSafariZone, isSafariMap } from "@pokecrystal/core/engine/world/safari-zone";
import { PendingEventFlagUpdate } from "@pokecrystal/core/engine/world/overworld/pending-event-flag-updates";
import { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import { OverworldObject } from "@pokecrystal/core/engine/world/overworld/overworld-object";
import type { PlayerCharacter } from "@pokecrystal/core/engine/world/overworld/playable-character";
import {
  OverworldNpcManagerMixin,
  resolveNpcDataList,
  setBlueprintIdentifier,
} from "@pokecrystal/core/engine/world/overworld/overworld-npc-manager";

type OverworldObjectLike = OverworldObject | PlayerCharacter;
import { OverworldBase } from "@pokecrystal/core/engine/world/overworld/overworld-base";
import type { GrassRustleTarget } from "@pokecrystal/core/engine/world/overworld/grass-rustle";
import { clearBooleanFlags, getBooleanFlag, setBooleanFlag } from "@pokecrystal/core/engine/world/overworld/flag-collection";
import { playOverworldSound } from "@pokecrystal/core/engine/world/overworld/audio-guards";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { clearTemporaryEventFlags } from "@pokecrystal/core/engine/world/overworld/temporary-event-flags";
import { pushDebugLog } from "@pokecrystal/core/core/debug-log";
import { isDebugEnabled } from "@pokecrystal/core/core/debug-flags";
import type { CompositeResult } from "@pokecrystal/core/engine/world/overworld/connection-composite";
import type {
  OverworldTilesetLike,
  RenderMetatileOptions,
  OverworldMetatile,
} from "@pokecrystal/core/engine/world/overworld/tileset-types";
import type { GameState } from "@pokecrystal/core/core/state";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";
import type { LoggerLike } from "@pokecrystal/core/engine/world/overworld/logger";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import type { WildEncounterManager, OverworldLike } from "@pokecrystal/core/engine/world/overworld/wild-encounters";

const _TIME_OF_DAY_NORMALISATION: Record<string, string> = {
  morning: "morn",
  morn: "morn",
  day: "day",
  nite: "nite",
  night: "nite",
  dark: "dark",
  darkness: "dark",
  indoor: "indoor",
  indoors: "indoor",
};

const _OUTDOOR_ENVIRONMENTS = new Set(["ROUTE", "TOWN"]);
const _INDOOR_ENVIRONMENTS = new Set(["INDOOR", "CAVE", "DUNGEON", "GATE"]);
const _POKECENTER_TILESETS = new Set(["pokecenter", "pokecom_center"]);
const _DIG_PREVIOUS_MAP_BLACKLIST = new Set(["MountMoonSquare", "TinTowerRoof"]);
const _POKECENTER_2F_LINK_ROOM_CONSTANTS = new Set([
  "TRADE_CENTER",
  "COLOSSEUM",
  "TIME_CAPSULE",
  "MOBILE_TRADE_ROOM",
  "MOBILE_BATTLE_ROOM",
]);
const _BIKE_RESET_ENGINE_FLAGS = [
  "ENGINE_STRENGTH_ACTIVE",
  "ENGINE_ALWAYS_ON_BIKE",
  "ENGINE_DOWNHILL",
] as const;

const _EMOTE_IMAGE_MAP: Record<string, string> = {
  EMOTE_BOLT: "bolt",
  EMOTE_FISH: "fish",
  EMOTE_HAPPY: "happy",
  EMOTE_HEART: "heart",
  EMOTE_QUESTION: "question",
  EMOTE_SAD: "sad",
  EMOTE_SHOCK: "shock",
  EMOTE_SLEEP: "sleep",
};

const normalizeMapCallbackKey = (mapName: string): string => {
  const trimmed = String(mapName ?? "").trim();
  if (!trimmed) {
    return trimmed;
  }
  if (trimmed.includes("_") && trimmed.toUpperCase() === trimmed) {
    return mapConstantToName(trimmed);
  }
  return trimmed;
};

const _EMOTE_Z_ORDER: Record<string, number> = { EMOTE_SHOCK: 1 };
const _EMOTE_ASSET_DIR = getAssetPath("gfx", "emotes");

type MapAttributesSource = Map<string, MapAttributes> | Record<string, MapAttributes> | null | undefined;

const resolve_map_attributes = (
  source: MapAttributesSource,
  mapName: string,
): MapAttributes | undefined => {
  if (!source) {
    return undefined;
  }
  if (source instanceof Map) {
    return source.get(mapName) ?? undefined;
  }
  return source[mapName];
};

type GrassRustleController = {
  set_time_of_day?: (time_of_day: string) => void;
  spawn?: (target: GrassRustleTarget, duration_frames: number) => void;
  tick?: () => void;
};

type NpcAutonomousController = {
  add_npc?: (npc: OverworldObject) => void;
  remove_npc?: (npc: OverworldObject) => void;
  rebuild?: (npcs: OverworldObject[]) => void;
  update?: () => void;
};

type TilesetConstructor = new (tileset_name: string, time_of_day: string) => OverworldTilesetLike;

type TilesetAnimator = {
  on_map_loaded?: (payload: {
    map_name: string;
    map_obj: OverworldMap;
    tileset: OverworldTilesetLike;
    surface: Surface | null;
    priority_surface: Surface | null;
  }) => void;
  set_connection_segments?: (segments: CompositeSegment[]) => void;
  set_whirlpool_active?: (active: boolean) => void;
  update?: () => void;
};

type MapSign = {
  on_map_loaded?: (map_name: string) => void;
  update?: () => void;
};

const resolve_tileset_constructor = (data_loader: { Tileset?: TilesetConstructor } | null | undefined): TilesetConstructor => {
  if (data_loader?.Tileset) {
    return data_loader.Tileset;
  }
  const globalTileset = (globalThis as { Tileset?: TilesetConstructor }).Tileset;
  if (globalTileset) {
    return globalTileset;
  }
  class TilesetShim implements OverworldTilesetLike {
    public readonly tilesetName = "TilesetShim";
    public readonly metatiles: OverworldMetatile[] = [];

    constructor(_tileset_name: string, _time_of_day: string) {
      throw new Error("Tileset loader unavailable; supply data_loader.Tileset or global Tileset.");
    }

    public renderMetatile(
      _metatileId: number,
      _target: Surface,
      _x: number,
      _y: number,
      _options?: RenderMetatileOptions,
    ): void {
      throw new Error("Tileset loader unavailable; supply data_loader.Tileset or global Tileset.");
    }

    public renderPriorityMetatile(
      _metatileId: number,
      _target: Surface,
      _x: number,
      _y: number,
    ): void {
      throw new Error("Tileset loader unavailable; supply data_loader.Tileset or global Tileset.");
    }
  }
  return TilesetShim;
};

function _normalise_emote_label(emote_id: string): string {
  const normalized = String(emote_id).trim().toUpperCase();
  if (!normalized) {
    throw new Error("Emote identifier may not be empty.");
  }
  if (normalized.startsWith("EMOTE_")) {
    return normalized;
  }
  const prefixed = `EMOTE_${normalized}`;
  if (_EMOTE_IMAGE_MAP[prefixed]) {
    return prefixed;
  }
  return normalized;
}

function warp_tile_position(warp: WarpEvent, stride: number): [number, number] {
  const offset = Math.max(0, stride - 1);
  return [warp.x * stride + offset, warp.y * stride + offset];
}

const _is_pokecenter_2f_link_room = (metadata: MapMetadata | null | undefined): boolean =>
  Boolean(metadata && _POKECENTER_2F_LINK_ROOM_CONSTANTS.has(metadata.constant));

export class EmoteSurfaceCache {
  private _surfaces: Map<string, Surface> = new Map();

  private static _apply_sprite_transparency(
    surface: Surface
  ): Surface {
    const baseColor = surface.get_at([0, 0]);
    if (baseColor[3] === 0) {
      return surface;
    }
    const [width, height] = surface.get_size();
    const context = surface.getContext();
    const image = context.getImageData(0, 0, width, height);
    const data = image.data;
    for (let index = 0; index < data.length; index += 4) {
      if (
        data[index] === baseColor[0] &&
        data[index + 1] === baseColor[1] &&
        data[index + 2] === baseColor[2] &&
        data[index + 3] === baseColor[3]
      ) {
        data[index + 3] = 0;
      }
    }
    context.putImageData(image, 0, 0);
    return surface;
  }

  public get_surface(emote_id: string): Surface {
    // ASM parity: LoadEmote/GetEmote2bpp is synchronous, so emotes must resolve
    // to a real surface here rather than a placeholder frame.
    const normalized = _normalise_emote_label(emote_id);
    const imageName = _EMOTE_IMAGE_MAP[normalized];
    if (!imageName) {
      throw new Error(`Unsupported emote '${emote_id}' requested.`);
    }
    const cached = this._surfaces.get(imageName);
    if (cached) {
      return cached;
    }
    const loadSync = gameEngine.image.loadSync;
    if (typeof loadSync !== "function") {
      throw new Error(
        `Missing synchronous emote loader for '${emote_id}'. Emote assets must be preloaded before rendering.`
      );
    }
    const path = `${_EMOTE_ASSET_DIR}/${imageName}.png`;
    const surface = loadSync(path);
    if (!surface) {
      throw new Error(`Missing emote surface '${emote_id}' at ${path}.`);
    }
    const transparentSurface = EmoteSurfaceCache._apply_sprite_transparency(
      surface.convert_alpha()
    );
    this._surfaces.set(imageName, transparentSurface);
    return transparentSurface;
  }
}

export class OverworldMapManagerMixin extends OverworldNpcManagerMixin {
  public _follow_leader_id: string | null = null;
  public _follow_follower_id: string | null = null;
  public _logger: LoggerLike | null = null;
  public _map_events!: MapEvents;
  public _active_warp_tile: [string, number, number] | null = null;
  public _active_coord_event: [string, number, number] | null = null;
  public _warp_cooldown: number = 0;
  public _coord_skip_log: Record<string, string> = {};
  public _coord_miss_log: Record<string, [number, number]> = {};
  public _npc_index_lookup: Map<number, OverworldObject> = new Map();
  public _emote_sprite_cache!: EmoteSurfaceCache;
  public _active_emotes: Map<OverworldObject, [string, number]> = new Map();
  public _pending_event_flag_updates: PendingEventFlagUpdate[] = [];
  public _npc_blueprints: Map<string, Map<string, [ObjectEvent, number]>> = new Map();
  public _wild_encounters: WildEncounterManager | null = null;
  public _suppress_initial_map_entry_effects_once: boolean = false;
  public _suppress_initial_map_music_once: boolean = false;
  public current_map_name!: string;
  public data_loader!: DataLoader;
  public game_state!: GameState;
  public event_manager!: EventManager | null;
  public script_runner!: ScriptRunner | null;
  public map!: OverworldMap;
  public tileset!: OverworldTilesetLike;
  public map_surface: Surface | null = null;
  public priority_surface: Surface | null = null;
  public _composite_surface: Surface | null = null;
  public _composite_priority_surface: Surface | null = null;
  public _composite_origin: [number, number] = [0, 0];
  public _composite_segments: CompositeSegment[] = [];
  public _tileset_animator!: TilesetAnimator;
  public follower: OverworldObject | null = null;
  public leader: OverworldObjectLike | null = null;
  public player_object: PlayerCharacter | null = null;
  public audio_engine: AudioEngine | null = null;
  public _grass_rustle: GrassRustleController | null = null;
  public _npc_autonomous_controller: NpcAutonomousController | null = null;
  public fatal_error?: Error;
  public _blocked_coord_events: Set<[string, string, number, number]> | null = null;
  public _map_sign: MapSign | null = null;
  public player_x!: number;
  public player_y!: number;
  public prev_player_x!: number;
  public prev_player_y!: number;
  public target_tile_x!: number;
  public target_tile_y!: number;
  public is_moving!: boolean;
  public _queued_direction: string | null = null;
  public step_progress_px!: number;
  public step_dx_px!: number;
  public step_dy_px!: number;
  protected _walk_frames_override: number | null = null;
  public get WALK_FRAMES(): number {
    return this._walk_frames_override ?? 8;
  }
  public set WALK_FRAMES(value: number) {
    const normalized = this._normalize_walk_frames(value);
    if (normalized === null) {
      return;
    }
    this._walk_frames_override = normalized;
  }
  public TILES_PER_COLLISION!: number;
  public _tileset_cache: Map<string, OverworldTilesetLike> = new Map();
  public player_direction!: string;
  public _warp_tile_lookup: Record<string, WarpEvent[]> | Map<string, WarpEvent[]> | null = {};
  public _warp_permission_cache: Record<string, Array<[WarpEvent, number | null]>> = {};

  public _sync_player_state(): void {}
  protected _refresh_map_environment_flags(_npc_data_list: ObjectEvent[] | null): void {}
  protected _reset_follower_path(): void {}
  public start_map_music(): void {}
  protected _apply_map_entry_player_state(): void {}
  public clear_pending_white_fade(): void {}
  public move_object(_object_id: string | number, _map_x: number, _map_y: number): void {}
  protected _reset_bike_flags_for_new_map(): void {
    const wram = this.game_state?.wram;
    if (wram) {
      // ASM: ResetBikeFlags clears wBikeFlags and companion state on map transitions.
      wram.wBikeFlags = 0;
    }
    const flags = this.game_state?.wram?.engine_flags;
    for (const flag of _BIKE_RESET_ENGINE_FLAGS) {
      setBooleanFlag(flags, flag, false);
    }
  }
  protected _normalize_walk_frames(value: number | null | undefined): number | null {
    if (typeof value !== "number" || !Number.isFinite(value)) {
      return null;
    }
    return Math.max(1, Math.trunc(value));
  }
  public script_tasks_active(): boolean {
    return false;
  }

  public _write_metatile(metatile_x: number, metatile_y: number, block_id: number): void {
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

    if (this.map_surface) {
      render_map_onto_surface(this.map, this.tileset, this.map_surface, { vram: this.game_state?.vram });
    }
    if (this.priority_surface) {
      render_priority_onto_surface(this.map, this.tileset, this.priority_surface);
    }

    const dirty_rect = new Rect(
      metatile_x * METATILE_SIZE,
      metatile_y * METATILE_SIZE,
      METATILE_SIZE,
      METATILE_SIZE
    );
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
    // ASM parity: Script_changeblock + Script_refreshmap updates movement permissions
    // before Script_warpcheck runs (e.g., Ruins of Alph floor-hole sequences).
    this._refresh_warp_permissions();
  }

  private _get_event_flag(flag: string): boolean {
    const flags = this.game_state?.wram?.event_flags;
    return Boolean(getBooleanFlag(flags, flag));
  }

  private _real_object_event_flag(flag: unknown): string | null {
    const normalized = String(flag ?? "").trim();
    return normalized && normalized !== "0" && normalized !== "-1" ? normalized : null;
  }

  private _set_event_flag(flag: string, value: boolean): void {
    const flags = this.game_state?.wram?.event_flags;
    setBooleanFlag(flags, flag, value);
  }

  protected _normalise_map_key(map_name: string): string {
    return OverworldBase._normalizeMapKey(map_name);
  }

  public _normalise_time_of_day_label(label: string | null | undefined): string {
    if (!label) {
      return "day";
    }
    const trimmed = String(label).trim().toLowerCase();
    if (!trimmed) {
      return "day";
    }
    return _TIME_OF_DAY_NORMALISATION[trimmed] || trimmed;
  }

  public _current_map_attributes(): MapAttributes | undefined {
    const map_name = this.current_map_name;
    if (!map_name) {
      return undefined;
    }
    const mapAttributes = this.data_loader?.map_attributes ?? this.data_loader?.mapAttributes;
    return resolve_map_attributes(mapAttributes, map_name);
  }

  public _clear_flash_state(): void {
    const wram = this.game_state.wram;
    const flags = wram.engine_flags;
    if (getBooleanFlag(flags, "STATUSFLAGS_FLASH")) {
      setBooleanFlag(flags, "STATUSFLAGS_FLASH", false);
    }
    clearBooleanFlags(wram.flash_active_maps);
  }

  public _set_map_events(events: MapEvents): void {
    this._map_events = events;
    this._warp_tile_lookup = this._build_warp_lookup(events);
    this._warp_permission_cache = {};
  }

  private _build_warp_lookup(events: MapEvents | null | undefined): Record<string, WarpEvent[]> {
    const lookup: Record<string, WarpEvent[]> = {};
    if (!events) {
      return {};
    }
    const stride = this.TILES_PER_COLLISION;
    for (const warp of events.warps ?? []) {
      const [x, y] = warp_tile_position(warp, stride);
      const key = `${x},${y}`;
      if (!lookup[key]) {
        lookup[key] = [];
      }
      lookup[key].push(warp);
    }
    return lookup;
  }

  public _refresh_warp_permissions(): void {
    const warp_lookup = this._warp_tile_lookup ?? {};
    const map_obj = this.map;
    const tileset = this.tileset;
    if (!map_obj || !tileset) {
      this._warp_permission_cache = {};
      return;
    }

    const cache: Record<string, Array<[WarpEvent, number]>> = {};
    for (const [coordsKey, warps] of Object.entries(warp_lookup)) {
      const [x, y] = coordsKey.split(",").map((value) => Number(value));
      try {
        const permission = sampleCollision(map_obj, tileset, x, y).permission;
        cache[coordsKey] = (warps as WarpEvent[]).map((warp: WarpEvent) => [warp, permission]);
      } catch {
        continue;
      }
    }
    this._warp_permission_cache = cache;
  }

  public _determine_effective_time_of_day(
    map_name: string | null,
    attributes: MapAttributes
  ): string {
    const wram = this.game_state.wram;
    const raw_label = attributes.time_of_day ?? null;
    let normalized = this._normalise_time_of_day_label(raw_label);
    if (raw_label === null || raw_label === undefined) {
      normalized = this._normalise_time_of_day_label(wram.time_of_day ?? "day");
    }
    if (normalized === "dark") {
      const flags = wram.engine_flags;
      const status_flag = Boolean(getBooleanFlag(flags, "STATUSFLAGS_FLASH"));
      const flash_maps = wram.flash_active_maps;
      const has_flash_for_map = Boolean(map_name && getBooleanFlag(flash_maps, map_name));
      if (status_flag || has_flash_for_map) {
        if (map_name) {
          setBooleanFlag(flash_maps, map_name, true);
        }
        return "nite";
      }
      return "dark";
    }

    // ASM reference: `EnvironmentColors` (data/maps/environment_colors.asm) uses the map environment to
    // select the tileset BG palette set. Indoor maps use a distinct palette group that does not follow
    // time-of-day shifts.
    const environment = String(attributes.environment ?? "")
      .trim()
      .toLowerCase();
    if (environment === "indoor" || environment === "gate") {
      return "indoor";
    }
    if (environment === "route" || environment === "town") {
      this._clear_flash_state();
    }
    return normalized;
  }

  public _current_tile_permission(): number | null {
    const map_obj = this.map;
    const tileset = this.tileset;
    if (!map_obj || !tileset) {
      return null;
    }
    try {
      return sampleCollision(map_obj, tileset, this.player_x, this.player_y).permission;
    } catch {
      return null;
    }
  }

  private _ensure_tileset_cache(): Map<string, OverworldTilesetLike> {
    const cache = this._tileset_cache;
    return cache ?? new Map<string, OverworldTilesetLike>();
  }

  private _get_tileset_for_time(tileset_name: string, time_of_day: string): OverworldTilesetLike {
    const key = `${tileset_name}:${String(time_of_day || "day").toLowerCase()}`;
    const cache = this._ensure_tileset_cache();
    const cached = cache.get(key);
    if (cached) {
      return cached;
    }
    const TilesetCtor = resolve_tileset_constructor(this.data_loader);
    const tileset = new TilesetCtor(tileset_name, time_of_day);
    cache.set(key, tileset);
    this._tileset_cache = cache;
    return tileset;
  }

  private _resolve_tileset_for_connection = (
    map_name: string,
    attributes: MapAttributes
  ): OverworldTilesetLike => {
    const time_of_day = this._determine_effective_time_of_day(map_name, attributes);
    return this._get_tileset_for_time(attributes.tileset_name, time_of_day);
  };

  public _refresh_tileset_for_current_map(attributes: MapAttributes | null = null): void {
    const map_name = this.current_map_name;
    if (!map_name || !this.map) {
      return;
    }
    const loader = this.data_loader;
    if (!attributes) {
      if (!loader) {
        return;
      }
      attributes = loader.map_attributes?.get?.(map_name) ?? null;
      if (!attributes) {
        return;
      }
    }
    const tileset_name = attributes.tileset_name;
    if (!tileset_name) {
      throw new Error(`Map '${map_name}' is missing a tileset_name attribute`);
    }
    const time_of_day = this._determine_effective_time_of_day(map_name, attributes);
    this._grass_rustle?.set_time_of_day?.(time_of_day);
    const tileset = this._get_tileset_for_time(tileset_name, time_of_day);
    this.tileset = tileset;
    const animator = this._tileset_animator;
    const renderLoadedTileset = (): void => {
      this.map_surface = create_map_surface(this.map!, tileset, {
        vram: this.game_state?.vram ?? null,
      });
      this.priority_surface = create_priority_surface(this.map!, tileset);
      if (animator?.on_map_loaded) {
        animator.on_map_loaded({
          map_name,
          map_obj: this.map!,
          tileset,
          surface: this.map_surface,
          priority_surface: this.priority_surface,
        });
      }
      this._rebuild_composite_surface();
    };

    const ready = tileset?.ready;
    if (ready && !tileset.loaded) {
      // Do not render with the synthetic fallback tiles/metatiles while the real
      // tileset is still loading; ASM map entry only presents loaded map data.
      this.map_surface = null;
      this.priority_surface = null;
      this._rebuild_composite_surface();
    } else {
      renderLoadedTileset();
    }

    if (ready) {
      Promise.resolve(ready).then(() => {
        if (this.tileset !== tileset || !this.map) {
          return;
        }
        renderLoadedTileset();
        this._refresh_warp_permissions();
        const npc_data_list = this.data_loader
          ? resolveNpcDataList(this.data_loader, map_name)
          : [];
        this._refresh_map_environment_flags(npc_data_list);
      }).catch?.((error: unknown) => {
        const message = `Tileset load failed for map=${map_name} tileset=${tileset_name} tod=${time_of_day}`;
        pushDebugLog(`[fatal] ${message}`, { error: String(error) });
        this._logger?.error?.(message, error);
        this.fatal_error = error instanceof Error ? error : new Error(String(error));
      });
    }
  }

  public _validate_connection_offsets(map_name: string, attributes: MapAttributes): void {
    const loader = this.data_loader;
    if (!loader) {
      return;
    }
    const current_key = this._normalise_map_key(map_name);
    const current_dimensions = loader.map_dimensions?.get?.(current_key);
    if (!current_dimensions) {
      return;
    }
    for (const connection of attributes.connections ?? []) {
      const target_attributes = loader.map_attributes?.get?.(connection.target_map);
      if (!target_attributes) {
        throw new Error(
          `Connection target '${connection.target_map}' missing attributes (referenced by ${this.current_map_name}).`
        );
      }
      const target_key = this._normalise_map_key(connection.target_map);
      const target_dimensions = loader.map_dimensions?.get?.(target_key);
      if (!target_dimensions) {
        continue;
      }

      const direction = String(connection.direction).toLowerCase();
      let axis_name: "width" | "height";
      if (direction === "north" || direction === "south") {
        axis_name = "width";
      } else if (direction === "west" || direction === "east") {
        axis_name = "height";
      } else {
        throw new Error(`Unsupported connection direction '${direction}'.`);
      }

      const current_axis = current_dimensions[axis_name];
      const target_axis = target_dimensions[axis_name];
      if (current_axis === undefined || target_axis === undefined) {
        continue;
      }

      const min_offset = -target_axis;
      const max_offset = current_axis;
      if (!(min_offset <= connection.offset && connection.offset <= max_offset)) {
        throw new Error(
          `Connection offset ${connection.offset} for '${direction}' on ${map_name} lies outside offsets ` +
            `[${min_offset},${max_offset}] derived from ${connection.target_map} (axis=${axis_name}).`
        );
      }
    }
  }

  public _refresh_warp_state(): void {
    if (!this._active_warp_tile) {
      return;
    }
    const [map_name, tile_x, tile_y] = this._active_warp_tile;
    if (map_name !== this.current_map_name) {
      this._active_warp_tile = null;
      return;
    }
    if (this.player_x !== tile_x || this.player_y !== tile_y) {
      if (this._logger?.debug) {
        this._logger.debug(
          `Clearing active warp cache for ${map_name} at (${tile_x},${tile_y}); player now at (${this.player_x},${this.player_y})`
        );
      }
      this._active_warp_tile = null;
    }
  }

  private _prime_active_warp_tile_for_current_position(): void {
    const events = this._map_events;
    if (!events?.warps?.length) {
      return;
    }
    const stride = this.TILES_PER_COLLISION;
    const matchingWarp = events.warps.find((warp) => {
      const [warpX, warpY] = warp_tile_position(warp, stride);
      return warpX === this.player_x && warpY === this.player_y;
    });
    if (!matchingWarp) {
      return;
    }
    this._active_warp_tile = [this.current_map_name, this.player_x, this.player_y];
  }

  private _environment_is_outdoor(environment: string | null | undefined): boolean {
    return _OUTDOOR_ENVIRONMENTS.has(String(environment ?? ""));
  }

  private _environment_is_indoor(environment: string | null | undefined): boolean {
    return _INDOOR_ENVIRONMENTS.has(String(environment ?? ""));
  }

  public check_for_warp_event(): boolean {
    const traceWarp = isDebugEnabled("overworld:warp") || isDebugEnabled("warp");
    const runner = this.script_runner;
    if (runner?._script_stack?.length) {
      if (traceWarp) {
        pushDebugLog("[warp] blocked: script runner active", {
          map: this.current_map_name,
          stackDepth: runner._script_stack.length,
        });
      }
      return false;
    }

    if (this._warp_cooldown > 0) {
      if (traceWarp) {
        pushDebugLog("[warp] blocked: cooldown", {
          map: this.current_map_name,
          cooldown: this._warp_cooldown,
        });
      }
      return false;
    }

    const player_x = this.player_x;
    const player_y = this.player_y;
    if (
      this._active_warp_tile &&
      this._active_warp_tile[0] === this.current_map_name &&
      player_x === this._active_warp_tile[1] &&
      player_y === this._active_warp_tile[2]
    ) {
      if (traceWarp) {
        pushDebugLog("[warp] blocked: active warp tile", {
          map: this.current_map_name,
          tile: [player_x, player_y],
        });
      }
      return false;
    }

    const events = this._map_events;
    if (!events || !(events.warps ?? []).length) {
      if (traceWarp) {
        pushDebugLog("[warp] blocked: no warps on map", { map: this.current_map_name });
      }
      return false;
    }

    const warp_permissions = this._warp_permission_cache ?? {};
    let warps_on_tile = warp_permissions[`${player_x},${player_y}`];
    if (!warps_on_tile) {
      const stride = this.TILES_PER_COLLISION;
      if (events?.warps?.length) {
        const matches = events.warps
          .filter((warp) => {
            const [x, y] = warp_tile_position(warp, stride);
            return x === player_x && y === player_y;
          })
          .map((warp) => [warp, null] as [WarpEvent, number | null]);
        if (matches.length) {
          warps_on_tile = matches;
          this._warp_permission_cache[`${player_x},${player_y}`] = matches;
        }
      }
    }
    if (!warps_on_tile || !warps_on_tile.length) {
      if (traceWarp) {
        pushDebugLog("[warp] blocked: no warp at player tile", {
          map: this.current_map_name,
          tile: [player_x, player_y],
        });
      }
      return false;
    }

    const permission = warps_on_tile[0][1];
    // ASM: Warps are only active if the underlying tile has warp-class collision.
    // This allows map callbacks (e.g., Ruins of Alph puzzles) to hide warps by
    // changing the tile collision from pit/door to floor.
    if (permission !== null && permission !== undefined && !isWarpPermission(permission)) {
      if (traceWarp) {
        pushDebugLog("[warp] blocked: tile is not warp permission", {
          map: this.current_map_name,
          tile: [player_x, player_y],
          permission,
        });
      }
      return false;
    }
    if (permission !== null && permission !== undefined && isDirectionalWarp(permission)) {

      const required_facing = directionalWarpFacing(permission);
      if (required_facing === null) {
        if (traceWarp) {
          pushDebugLog("[warp] blocked: directional warp missing facing", {
            map: this.current_map_name,
            tile: [player_x, player_y],
            permission,
          });
        }
        return false;
      }
      let current_facing: FacingDirection;
      try {
        current_facing = FacingDirection.fromString(this.player_direction);
      } catch {
        if (traceWarp) {
          pushDebugLog("[warp] blocked: invalid player facing", {
            map: this.current_map_name,
            tile: [player_x, player_y],
            player_direction: this.player_direction,
          });
        }
        return false;
      }
      if (current_facing !== required_facing) {
        if (traceWarp) {
          pushDebugLog("[warp] blocked: wrong facing for directional warp", {
            map: this.current_map_name,
            tile: [player_x, player_y],
            facing: current_facing,
            required: required_facing,
          });
        }
        return false;
      }
    }

    for (const [warp, permission_value] of warps_on_tile) {
      const effective_permission = permission_value ?? permission;
      if (traceWarp) {
        pushDebugLog("[warp] triggering", {
          map: this.current_map_name,
          tile: [player_x, player_y],
          permission: effective_permission,
          target_map_constant: warp.target_map_constant,
          target_warp_id: warp.target_warp_id,
        });
      }
      if (this._logger?.debug) {
        this._logger.debug(
          `Player hit warp ${warp.index} on ${this.current_map_name} at (${player_x},${player_y}) targeting ${warp.target_map_constant}#${warp.target_warp_id}`
        );
      }
      this._play_warp_sound(effective_permission ?? null);
      this._activate_warp(warp, effective_permission ?? null);
      break;
    }
    return true;
  }

  private _activate_warp(warp: WarpEvent, permission: number | null = null): void {
    const dynamic_destination = warp.target_warp_id < 0;
    let metadata: MapMetadata | null = null;
    let destination: WarpEvent;
    const current_metadata = getMapMetadataByName(this.current_map_name) ?? null;
    const previous_metadata = getMapMetadataByGroup(
      this.game_state.wram.wPrevMapGroup,
      this.game_state.wram.wPrevMapNumber
    ) ?? null;

    if (dynamic_destination) {
      const leaving_pokecenter_2f_after_link_room =
        current_metadata?.constant === "POKECENTER_2F" &&
        _is_pokecenter_2f_link_room(previous_metadata);
      const leaving_elevator_after_selection =
        current_metadata?.constant.endsWith("_ELEVATOR") ?? false;

      if (!leaving_pokecenter_2f_after_link_room && !leaving_elevator_after_selection) {
        // ASM: GetWarpDestCoords .backup sets the backup warp from the previous warp.
        this.game_state.wram.wBackupWarpNumber = this.game_state.wram.wPrevWarp;
        this.game_state.wram.wBackupMapGroup = this.game_state.wram.wPrevMapGroup;
        this.game_state.wram.wBackupMapNumber = this.game_state.wram.wPrevMapNumber;
      }

      const backup_group = this.game_state.wram.wBackupMapGroup;
      const backup_number = this.game_state.wram.wBackupMapNumber;
      const backup_warp = Math.max(1, Number(this.game_state.wram.wBackupWarpNumber || 1));
      metadata = getMapMetadataByGroup(backup_group, backup_number) ?? null;
      if (!metadata) {
        throw new Error(
          `Unknown backup destination ${backup_group}:${backup_number} for warp ${warp.index} on ${this.current_map_name}`
        );
      }
      let target_events = this.data_loader.map_events?.get?.(metadata.name);
      if (!target_events) {
        this.data_loader.ensure_map_scripts?.(metadata.name);
        target_events = this.data_loader.map_events?.get?.(metadata.name);
      }
      if (!target_events || !(target_events.warps ?? []).length) {
        throw new Error(
          `Missing warp table for backup destination '${metadata.name}' (required by warp ${warp.index} on ${this.current_map_name})`
        );
      }
      const destination_index = backup_warp - 1;
      if (destination_index < 0 || destination_index >= target_events.warps.length) {
        throw new Error(
          `Backup warp id ${backup_warp} for ${metadata.name} exceeds available warps (${target_events.warps.length})`
        );
      }
      destination = target_events.warps[destination_index];
    } else {
      metadata = getMapMetadataByConstant(warp.target_map_constant) ?? null;
      if (!metadata) {
        throw new Error(
          `Unknown target map constant '${warp.target_map_constant}' for warp ${warp.index} on ${this.current_map_name}`
        );
      }
      let target_events = this.data_loader.map_events?.get?.(metadata.name);
      if (!target_events) {
        this.data_loader.ensure_map_scripts?.(metadata.name);
        target_events = this.data_loader.map_events?.get?.(metadata.name);
      }
      if (!target_events || !(target_events.warps ?? []).length) {
        throw new Error(
          `Missing warp table for destination map '${metadata.name}' (required by warp ${warp.index} on ${this.current_map_name})`
        );
      }
      const destination_index = warp.target_warp_id - 1;
      if (destination_index < 0 || destination_index >= target_events.warps.length) {
        throw new Error(
          `Warp id ${warp.target_warp_id} referenced by ${this.current_map_name} exceeds available warps (${target_events.warps.length}) on ${metadata.name}`
        );
      }
      destination = target_events.warps[destination_index];
    }

    if (!metadata) {
      throw new Error("Warp destination metadata was not resolved.");
    }
    const dest_environment = metadata.environment;
    const stride = this.TILES_PER_COLLISION;
    const [dest_x, dest_y] = warp_tile_position(destination, stride);

    const prev_group = this.game_state.wram.wMapGroup;
    const prev_number = this.game_state.wram.wMapNumber;
    const prev_map_name = this.current_map_name;
    const moving_between_pokecenter_2f_and_link_room =
      (prev_map_name === "Pokecenter2F" && _is_pokecenter_2f_link_room(metadata))
      || (_is_pokecenter_2f_link_room(current_metadata) && metadata.constant === "POKECENTER_2F");
    const leaving_elevator_after_selection =
      dynamic_destination && (current_metadata?.constant.endsWith("_ELEVATOR") ?? false);
    if (!moving_between_pokecenter_2f_and_link_room && !leaving_elevator_after_selection) {
      this.game_state.wram.wBackupMapGroup = prev_group;
      this.game_state.wram.wBackupMapNumber = prev_number;
    }
    if (metadata.constant === "POKECENTER_2F" && !_is_pokecenter_2f_link_room(current_metadata)) {
      this.game_state.wram.wBackupWarpNumber = warp.index;
    }
    this.game_state.wram.wPrevWarp = warp.index;
    this.game_state.wram.wPrevMapGroup = prev_group;
    this.game_state.wram.wPrevMapNumber = prev_number;
    this.game_state.wram.wDefaultSpawnpoint = 0;
    const prev_environment = getMapEnvironment(prev_map_name);
    const wram = this.game_state.wram;
    if (
      prev_map_name &&
      this._environment_is_outdoor(prev_environment) &&
      this._environment_is_indoor(dest_environment) &&
      !_DIG_PREVIOUS_MAP_BLACKLIST.has(prev_map_name)
    ) {
      wram.wDigWarpNumber = warp.index;
      wram.wDigMapGroup = prev_group;
      wram.wDigMapNumber = prev_number;
    } else {
      wram.wDigWarpNumber = 0;
      wram.wDigMapGroup = 0;
      wram.wDigMapNumber = 0;
    }

    pushDebugLog(`[warp] ${metadata.name} @ ${dest_x},${dest_y}`, {
      from: prev_map_name,
      to_warp_id: warp.target_warp_id,
    });

    this.game_state.wram.wMapGroup = metadata.groupId;
    this.game_state.wram.wMapNumber = metadata.mapId;
    this.game_state.wram.wNextWarp = warp.index;
    this.game_state.wram.wNextMapGroup = metadata.groupId;
    this.game_state.wram.wNextMapNumber = metadata.mapId;
    this.game_state.wram.current_map_group = metadata.groupId;
    this.game_state.wram.current_map_id = metadata.mapId;

    const map_attrs = this.data_loader.map_attributes?.get?.(metadata.name);
    const tileset_name = map_attrs?.tileset_name;
    if (tileset_name && _POKECENTER_TILESETS.has(tileset_name)) {
      this.game_state.wram.wLastSpawnMapGroup = prev_group;
      this.game_state.wram.wLastSpawnMapNumber = prev_number;
      this.game_state.sram.last_spawn_map_group = prev_group;
      this.game_state.sram.last_spawn_map_number = prev_number;
    }

    this.player_x = dest_x;
    this.player_y = dest_y;
    this.prev_player_x = dest_x;
    this.prev_player_y = dest_y;
    this.target_tile_x = dest_x;
    this.target_tile_y = dest_y;
    this.is_moving = false;
    this._queued_direction = null;
    this.step_progress_px = 0.0;
    this.step_dx_px = 0.0;
    this.step_dy_px = 0.0;
    this.game_state.wram.wXCoord = dest_x;
    this.game_state.wram.wYCoord = dest_y;
    this.game_state.wram.player_x = Math.floor(dest_x / METATILE_WIDTH);
    this.game_state.wram.player_y = Math.floor(dest_y / METATILE_WIDTH);
    this.game_state.wram.player_subtile_x = dest_x % METATILE_WIDTH;
    this.game_state.wram.player_subtile_y = dest_y % METATILE_WIDTH;

    this.load_map(metadata.name);

    this._active_warp_tile = [this.current_map_name, dest_x, dest_y];
    this._sync_player_state();
    this.clear_pending_white_fade();
    this._warp_cooldown = this.WALK_FRAMES;
  }

  private _play_warp_sound(permission: number | null): void {
    if (permission === null || permission === undefined) {
      return;
    }
    if (isPitPermission(permission)) {
      return;
    }
    const audio_engine = this.audio_engine;
    if (!audio_engine) {
      return;
    }
    const sound_id = warpSoundForPermission(permission);
    if (!sound_id) {
      return;
    }
    playOverworldSound(audio_engine, sound_id, {
      logger: this._logger,
      context: "warp sound",
    });
  }

  public check_for_coord_events(): boolean {
    if (this.script_tasks_active?.()) {
      return false;
    }

    const runner = this.script_runner;
    if (runner) {
      const stack_depth = runner._script_stack?.length ?? 0;
      const awaiting_resume = runner._awaiting_resume ?? 0;
      if (stack_depth > 0 || awaiting_resume > 0) {
        return false;
      }
    }

    const events = this._map_events?.coord_events ?? [];
    if (!events.length) {
      this._active_coord_event = null;
      return false;
    }

    const get_script_event_flags = this.data_loader?.get_script_event_flags;
    const wram = this.game_state.wram;
    const event_flags = wram.event_flags;
    const stride = this.TILES_PER_COLLISION;
    const offset = stride - 1;
    let current_scene = wram.scene_name;
    const current_map = this.current_map_name ?? "";
    if (!current_scene && runner?._ensure_map_scene_initialized) {
      const initialized = runner._ensure_map_scene_initialized(current_map);
      if (initialized) {
        const [scene_name] = initialized;
        if (scene_name) {
          this.game_state.wram.scene_name = scene_name;
          current_scene = scene_name;
        }
      }
    }

    const player_x = this.player_x;
    const player_y = this.player_y;
    const player_pos: [number, number] = [player_x, player_y];
    const blocked_events = this._blocked_coord_events;
    if (blocked_events) {
      const to_remove: Set<[string, string, number, number]> = new Set();
      for (const entry of blocked_events) {
        const [map_name, _script, x, y] = entry;
        if (map_name !== current_map || (map_name === current_map && (x !== player_x || y !== player_y))) {
          to_remove.add(entry);
        }
      }
      to_remove.forEach((entry) => blocked_events.delete(entry));
    }

    for (const coord_event of events) {
      const scene_locked = Boolean(coord_event.scene_id);
      if (get_script_event_flags && !scene_locked) {
        const script_flags = get_script_event_flags(coord_event.script_name);
        if (script_flags && script_flags.some((flag: string) => this._get_event_flag(flag))) {
          continue;
        }
      }
      if (scene_locked && coord_event.scene_id !== current_scene) {
        const previous_scene = this._coord_skip_log[coord_event.script_name];
        if (this._logger?.debug && previous_scene !== current_scene) {
          this._coord_skip_log[coord_event.script_name] = current_scene;
        }
        continue;
      }
      const event_x = coord_event.x * stride + offset;
      const event_y = coord_event.y * stride + offset;
      const key: [string, number, number] = [coord_event.script_name, event_x, event_y];
      const blocked_key: [string, string, number, number] = [current_map, coord_event.script_name, event_x, event_y];
      if (blocked_events && blocked_events.has(blocked_key)) {
        continue;
      }
      if (player_x === event_x && player_y === event_y) {
        if (!this._active_coord_event || this._active_coord_event.join(":") !== key.join(":")) {
          this._active_coord_event = key;
          if (this._logger?.info) {
            this._logger.info(`Triggering coord event ${coord_event.script_name} at (${event_x},${event_y})`);
          }
          pushDebugLog(
            `[coord] ${coord_event.script_name} @ ${event_x},${event_y}`,
            { map: current_map }
          );
          delete this._coord_skip_log[coord_event.script_name];
          delete this._coord_miss_log[coord_event.script_name];
          if (blocked_events) {
            blocked_events.add(blocked_key);
          }
          runner?.run?.(coord_event.script_name);
        }
        return true;
      }
      const last_pos = this._coord_miss_log[coord_event.script_name];
      if (this._logger?.debug && (!last_pos || last_pos[0] !== player_x || last_pos[1] !== player_y)) {
        this._logger.debug(
          `Coord miss scene=${coord_event.scene_id} script=${coord_event.script_name} player=(${player_x},${player_y}) target=(${event_x},${event_y})`
        );
        this._coord_miss_log[coord_event.script_name] = player_pos;
      }
    }

    this._active_coord_event = null;
    return false;
  }

  public _run_map_callbacks(map_name: string, callback_type: string | null = null): void {
    const callbacksMap = this.data_loader.map_callbacks;
    if (!callbacksMap) {
      return;
    }
    let resolvedName = map_name;
    let callbacks = callbacksMap.get(map_name) ?? [];
    if (!callbacks.length) {
      const normalized = normalizeMapCallbackKey(map_name);
      if (normalized && normalized !== map_name) {
        resolvedName = normalized;
        callbacks = callbacksMap.get(normalized) ?? [];
      }
    }
    for (const [cb_type, script_name] of callbacks) {
      if (callback_type === null || cb_type === callback_type) {
        this._logger?.debug?.(`Running ${cb_type} callback: ${script_name}`);
        if (isDebugEnabled("script:run") || isDebugEnabled("script")) {
          pushDebugLog(`[script] callback ${cb_type} -> ${script_name}`, { map: resolvedName });
        }
        this.script_runner?.run?.(script_name);
      }
    }
  }

  public _run_map_scene(): void {
    const map_name = this.current_map_name;
    this._repair_story_scene_state(map_name);
    const initialized = this.script_runner?._ensure_map_scene_initialized?.(map_name);
    const scene_name = initialized ? initialized[0] : this.game_state.wram.map_scenes?.[map_name] ?? "";
    this.game_state.wram.scene_name = scene_name;
    const scene_scripts = this.data_loader.map_scene_scripts?.get?.(map_name) ?? {};
    const script_name = scene_scripts?.[scene_name];
    if (script_name) {
      if (isDebugEnabled("script:run") || isDebugEnabled("script")) {
        pushDebugLog(`[script] scene ${scene_name} -> ${script_name}`, { map: map_name });
      }
      const runner = this.script_runner;
      const stackDepth = runner?._script_stack?.length ?? 0;
      const awaitingResume = runner?._awaiting_resume ?? 0;
      if (stackDepth > 0 || awaitingResume > 0 || runner?.is_busy) {
        runner?.defer?.(script_name);
      } else {
        runner?.run?.(script_name);
      }
    }
  }

  public _repair_story_scene_state(map_name: string): void {
    const runner = this.script_runner;
    if (!runner) {
      return;
    }
    const normalized = runner._normalise_map_name?.(map_name) ?? map_name;
    const order = this.data_loader.map_scene_order?.get?.(normalized) ?? [];
    if (!order.length) {
      return;
    }
    const index = this.game_state.wram.map_scene_indices?.[normalized];
    if (index === undefined || index < order.length) {
      return;
    }
    throw new Error(
      `Map scene index ${index} for '${normalized}' exceeds canonical scene order length ${order.length}.`
    );
  }

  public _rebuild_composite_surface(): void {
    const base_surface = this.map_surface;
    const priority_surface = this.priority_surface;
    if (!base_surface || !priority_surface) {
      this._composite_surface = null;
      this._composite_priority_surface = null;
      this._composite_origin = [0, 0];
      this._composite_segments = [];
      return;
    }
    const attributes = this.data_loader.map_attributes?.get?.(this.current_map_name);
    if (!attributes) {
      throw new Error(
        `Missing map attributes for ${this.current_map_name} during composite rebuild`
      );
    }
    const result = build_connection_composite({
      map_name: this.current_map_name,
      map_attributes: attributes,
      base_surface,
      base_priority_surface: priority_surface,
      base_tileset: this.tileset,
      data_loader: this.data_loader,
      resolve_tileset: this._resolve_tileset_for_connection,
      game_state: this.game_state ?? null,
    });

    this._composite_surface = null;
    this._composite_priority_surface = null;
    this._composite_origin = [0, 0];
    this._composite_segments = [];
    result
      .then((payload: CompositeResult) => {
        this._composite_surface = payload.surface;
        this._composite_priority_surface = payload.priority_surface;
        this._composite_origin = payload.origin;
        this._composite_segments = payload.segments;
        const animator = this._tileset_animator;
        if (animator?.set_connection_segments) {
          animator.set_connection_segments(payload.segments);
        }
      })
      .catch((error: unknown) => {
        throw error;
      });
  }

  public refresh_composite_surfaces(
    dirty_rects?: Map<Surface, Rect[]>
  ): void {
    const composite = this._composite_surface;
    if (!composite) {
      return;
    }
    if (!dirty_rects) {
      const [origin_x, origin_y] = this._composite_origin;
      if (this.map_surface) {
        composite.blit(this.map_surface, [origin_x, origin_y]);
      }
      if (this._composite_priority_surface && this.priority_surface) {
        this._composite_priority_surface.blit(this.priority_surface, [origin_x, origin_y]);
      }
      for (const segment of this._composite_segments) {
        composite.blit(segment.surface, segment.dest);
        if (this._composite_priority_surface && segment.priority_surface) {
          this._composite_priority_surface.blit(segment.priority_surface, segment.dest);
        }
      }
      return;
    }

    const [origin_x, origin_y] = this._composite_origin;
    const surface_targets = new Map<
      Surface,
      [Surface, [number, number]]
    >();
    if (this.map_surface) {
      surface_targets.set(this.map_surface, [composite, [origin_x, origin_y]]);
    }
    if (this._composite_priority_surface && this.priority_surface) {
      surface_targets.set(this.priority_surface, [this._composite_priority_surface, [origin_x, origin_y]]);
    }
    for (const segment of this._composite_segments) {
      surface_targets.set(segment.surface, [composite, segment.dest]);
      if (this._composite_priority_surface && segment.priority_surface) {
        surface_targets.set(segment.priority_surface, [this._composite_priority_surface, segment.dest]);
      }
    }

    for (const [src_surface, rects] of dirty_rects.entries()) {
      const target_info = surface_targets.get(src_surface);
      if (!target_info) {
        continue;
      }
      const [target_surface, dest] = target_info;
      const [dest_x, dest_y] = dest;
      for (const rect of rects) {
        target_surface.blit(src_surface, [dest_x + rect.x, dest_y + rect.y], rect);
      }
    }
  }

  public load_map(map_name: string): void {
    clearTemporaryEventFlags(this.game_state);
    this._reset_bike_flags_for_new_map();
    const mapAttributesSource = this.data_loader?.map_attributes;
    const map_attributes = mapAttributesSource?.get?.(map_name);
    if (!map_attributes) {
      throw new Error(`Missing map attributes for ${map_name}`);
    }
    this.data_loader.ensure_map_scripts?.(map_name);
    this._validate_connection_offsets(map_name, map_attributes);
    this._reset_follower_path();
    this.current_map_name = map_name;
    const metadata = getMapMetadataByName(map_name);
    if (metadata) {
      // Direct map loads must keep WRAM's active map ids aligned so later dynamic
      // warps, such as Pokecenter 2F exits, resolve against the real current map.
      this.game_state.wram.wMapGroup = metadata.groupId;
      this.game_state.wram.wMapNumber = metadata.mapId;
      this.game_state.wram.current_map_group = metadata.groupId;
      this.game_state.wram.current_map_id = metadata.mapId;
    }
    this._set_map_events(
      this.data_loader.map_events?.get?.(map_name) ?? { warps: [], coord_events: [], bg_events: [] }
    );

    const provisional_map = build_overworld_map(map_name, map_attributes, {
      data_loader: this.data_loader,
    });
    this.map = provisional_map;
    this._refresh_tileset_for_current_map(map_attributes);
    this._run_map_callbacks(map_name, "MAPCALLBACK_TILES");
    this._refresh_warp_permissions();
    const npc_data_list: ObjectEvent[] = this.data_loader
      ? resolveNpcDataList(this.data_loader, map_name)
      : [];
    this._refresh_map_environment_flags(npc_data_list);
    const blueprint = this._build_blueprint(map_name);
    const entries: Array<[ObjectEvent, number]> = [];
    const map_key = map_name.replace(/\s+/g, "").toUpperCase();
    npc_data_list.forEach((event, index: number) => {
      const obj = new OverworldObject(event);
      obj.objectIndex = index + 1;
      this._apply_variable_sprite(obj);
      this._initialise_object_coordinates(obj);
      this._apply_initial_direction(obj);
      const identifiers = new Set<string>([
        obj.spriteId,
        obj.baseSpriteId,
        ...(obj.objectId ? [obj.objectId] : []),
        `${map_key}_${obj.spriteId}`,
        `${map_key}_${obj.baseSpriteId}`,
        `${map_key}_${obj.spriteId}${obj.objectIndex}`,
        `${map_key}_${obj.baseSpriteId}${obj.objectIndex}`,
        ...(obj.objectId ? [`${map_key}_${obj.objectId}`] : []),
      ]);
      if (obj.constantId) {
        identifiers.add(obj.constantId);
      }
      const script_name = String(event.script ?? "").trim();
      if (script_name) {
        const normalized_script = script_name.toUpperCase();
        identifiers.add(normalized_script);
        if (normalized_script.endsWith("SCRIPT")) {
          identifiers.add(normalized_script.slice(0, -"SCRIPT".length));
        }
      }
      identifiers.add(String(obj.objectIndex));
      if (map_name === "ElmsLab") {
        if (obj.objectIndex === 3) {
          identifiers.add("ELMSLAB_POKE_BALL1");
        } else if (obj.objectIndex === 4) {
          identifiers.add("ELMSLAB_POKE_BALL2");
        } else if (obj.objectIndex === 5) {
          identifiers.add("ELMSLAB_POKE_BALL3");
        }
      }
      identifiers.forEach((identifier) => {
        setBlueprintIdentifier(blueprint, identifier, [event, obj.objectIndex]);
      });
      entries.push([event, obj.objectIndex]);
    });
    this.npcs = this._add_map_sprites(entries);
    this._npc_index_lookup = new Map(this.npcs.map((npc) => [npc.objectIndex, npc]));
    this._reset_follower_path();
    this._resolve_follow_targets();
    const controller = this._npc_autonomous_controller;
    if (controller?.rebuild) {
      controller.rebuild(this.npcs);
    }
    this._active_coord_event = null;
    this._apply_pending_last_talked_position();
    const suppressInitialMapEntryEffects = Boolean(this._suppress_initial_map_entry_effects_once);
    this._suppress_initial_map_entry_effects_once = false;
    if (suppressInitialMapEntryEffects) {
      return;
    }
    this._run_map_callbacks(map_name, "MAPCALLBACK_OBJECTS");

    this._run_map_callbacks(map_name, "MAPCALLBACK_NEWMAP");
    this._apply_map_entry_player_state?.();
    const suppressInitialMapMusic = Boolean(this._suppress_initial_map_music_once);
    this._suppress_initial_map_music_once = false;
    if (!suppressInitialMapMusic) {
      this.start_map_music();
    }
    this._run_map_scene();
    this._wild_encounters?.on_map_loaded?.(map_name);
    this._map_sign?.on_map_loaded?.(map_name);
    if (!isSafariMap(map_name)) {
      endSafariZone(this.game_state);
    }
    this._prime_active_warp_tile_for_current_position();
  }

  public _viewport_origin(viewport_width: number, viewport_height: number): [number, number] {
    const max_x = Math.max(0, this.map.width - viewport_width);
    const max_y = Math.max(0, this.map.height - viewport_height);
    const player_metatile_x = Math.floor(this.player_x / METATILE_WIDTH);
    const player_metatile_y = Math.floor(this.player_y / METATILE_WIDTH);
    const origin_x = Math.min(Math.max(player_metatile_x - Math.floor(viewport_width / 2), 0), max_x);
    const origin_y = Math.min(Math.max(player_metatile_y - Math.floor(viewport_height / 2), 0), max_y);
    return [origin_x, origin_y];
  }

  public get_object_by_id(object_id: string | number): OverworldObject | PlayerCharacter | null {
    if (typeof object_id === "number") {
      if (object_id === 0 || object_id === -2) {
        return this.player_object;
      }
      if (object_id <= 0) {
        return null;
      }
      return this._npc_index_lookup.get(object_id) ?? null;
    }

    const normalized = String(object_id).toUpperCase();
    if (normalized === "PLAYER") {
      return this.player_object;
    }
    if (normalized === "LAST_TALKED") {
      return this.get_object_by_id(this.game_state.wram.last_talked);
    }
    if (/^-?\d+$/.test(normalized)) {
      return this.get_object_by_id(Number(normalized));
    }

    const map_key = this.current_map_name.replace(/\s+/g, "").toUpperCase();
    for (const npc of this.npcs ?? []) {
      const identifiers = new Set<string>([
        npc.name.toUpperCase(),
        npc.spriteId,
        npc.baseSpriteId,
        `${map_key}_${npc.spriteId}`,
        `${map_key}_${npc.baseSpriteId}`,
      ]);
      if (npc.objectId) {
        identifiers.add(npc.objectId);
        identifiers.add(`${map_key}_${npc.objectId}`);
      }
      if (npc.constantId) {
        identifiers.add(npc.constantId);
      }
      const object_index = npc.objectIndex ?? 0;
      identifiers.add(`${map_key}_${npc.spriteId}${object_index}`);
      identifiers.add(`${map_key}_${npc.baseSpriteId}${object_index}`);
      if (identifiers.has(normalized)) {
        return npc;
      }
    }
    return null;
  }

  public resolve_object_index(identifier: string): number | null {
    if (identifier === null || identifier === undefined) {
      return null;
    }
    const normalized = String(identifier).toUpperCase();
    if (normalized === "PLAYER") {
      return 0;
    }
    if (normalized === "LAST_TALKED") {
      return this.game_state.wram.last_talked;
    }
    if (/^-?\d+$/.test(normalized)) {
      const value = Number(normalized);
      return value <= 0 ? 0 : value;
    }
    const [map_name, entry] = this._find_blueprint_entry(normalized);
    if (!entry || map_name !== this.current_map_name) {
      return null;
    }
    return entry[1];
  }

  public _resolve_follow_targets(): void {
    const leader_id = this._follow_leader_id;
    const follower_id = this._follow_follower_id;
    if (!leader_id && !follower_id) {
      return;
    }
    let leader: OverworldObjectLike | null = this.leader ?? null;
    let follower: OverworldObjectLike | null = this.follower ?? null;
    if (leader_id) {
      leader = this.get_object_by_id(leader_id) as OverworldObjectLike | null;
    }
    if (follower_id) {
      follower = this.get_object_by_id(follower_id) as OverworldObjectLike | null;
    }
    if (!leader || !follower || !("event" in follower)) {
      this.stop_following();
      return;
    }
    this.leader = leader;
    this.follower = follower;
  }

  public start_following(
    follower: OverworldObject,
    leader: OverworldObjectLike,
    {
      follower_id = null,
      leader_id = null,
    }: { follower_id?: string | null; leader_id?: string | null } = {}
  ): void {
    this.follower = follower;
    this.leader = leader;
    const inferred_follower_id = follower_id ?? (follower as OverworldObject).objectId ?? null;
    const inferred_leader_id = leader_id ?? (leader as OverworldObject).objectId ?? null;
    this._follow_follower_id = inferred_follower_id;
    this._follow_leader_id = inferred_leader_id;
  }

  public stop_following(): void {
    this.follower = null;
    this.leader = null;
    this._follow_follower_id = null;
    this._follow_leader_id = null;
  }

  public remove_object(
    this: OverworldMapManagerMixin,
    object_id: string | number,
    { update_event_flag = true }: { update_event_flag?: boolean } = {}
  ): void {
    if (!this || !this._npc_blueprints) {
      throw new Error("remove_object() called without a valid overworld manager `this` binding.");
    }
    const normalized = String(object_id).toUpperCase();
    const blueprint_map = this._npc_blueprints.get(this.current_map_name) ?? new Map<string, [ObjectEvent, number]>();
    const controller = this._npc_autonomous_controller;
    if (blueprint_map.has(normalized)) {
      const [event, index] = blueprint_map.get(normalized) as [ObjectEvent, number];
      const target = this._npc_index_lookup.get(index) ?? null;
      if (controller?.remove_npc && target) {
        controller.remove_npc(target);
      }
      this.npcs = (this.npcs ?? []).filter((npc) => npc.objectIndex !== index);
      this._npc_index_lookup = new Map(this.npcs.map((npc) => [npc.objectIndex, npc]));
      const event_flag = this._real_object_event_flag(event.event_flag);
      if (update_event_flag && event_flag) {
        this._set_event_flag(event_flag, true);
      }
      return;
    }

    const remaining: OverworldObject[] = [];
    let removed_flag: string | null = null;
    const removed_npcs: OverworldObject[] = [];
    for (const npc of this.npcs ?? []) {
      const identifiers = new Set<string>([
        npc.name.toUpperCase(),
        npc.spriteId,
        npc.baseSpriteId,
      ]);
      if (npc.objectId) {
        identifiers.add(npc.objectId);
      }
      if (npc.constantId) {
        identifiers.add(npc.constantId);
      }
      const map_key = this.current_map_name.replace(/\s+/g, "").toUpperCase();
      identifiers.add(`${map_key}_${npc.spriteId}`);
      identifiers.add(`${map_key}_${npc.baseSpriteId}`);
      const object_index = npc.objectIndex ?? 0;
      identifiers.add(`${map_key}_${npc.spriteId}${object_index}`);
      identifiers.add(`${map_key}_${npc.baseSpriteId}${object_index}`);
      identifiers.add(String(object_index));
      if (identifiers.has(normalized)) {
        const event_flag = this._real_object_event_flag(npc.event.event_flag);
        if (!removed_flag && event_flag) {
          removed_flag = event_flag;
        }
        removed_npcs.push(npc);
        continue;
      }
      remaining.push(npc);
    }
    this.npcs = remaining;
    this._npc_index_lookup = new Map(this.npcs.map((npc) => [npc.objectIndex, npc]));
    if (update_event_flag && removed_flag) {
      this._set_event_flag(removed_flag, true);
    }
    const removeNpc = controller?.remove_npc;
    if (removeNpc) {
      removed_npcs.forEach((npc) => removeNpc(npc));
    }
  }

  public appear_object(
    object_id: string | number,
    {
      ignore_event_flag = true,
      update_event_flag = true,
      force_spawn = false,
    }: { ignore_event_flag?: boolean; update_event_flag?: boolean; force_spawn?: boolean } = {}
  ): void {
    const blueprint_map = this._npc_blueprints.get(this.current_map_name) ?? new Map<string, [ObjectEvent, number]>();
    const target_id = String(object_id).toUpperCase();
    const controller = this._npc_autonomous_controller;
    const entry = blueprint_map.get(target_id);
    if (!entry) {
      return;
    }
    const [event, index] = entry;
    if (this._npc_index_lookup.has(index)) {
      return;
    }
    const obj = new OverworldObject(event);
    obj.objectIndex = index;
    this._apply_variable_sprite(obj);
    this._initialise_object_coordinates(obj);
    this._apply_initial_direction(obj);
    const event_flag = this._real_object_event_flag(event.event_flag);
    if (update_event_flag && event_flag) {
      this._set_event_flag(event_flag, false);
    }
    if (!force_spawn && !this._object_should_spawn(obj, { ignore_event_flag })) {
      return;
    }
    this._initialise_npc_object(obj);
    this.npcs.push(obj);
    this.npcs.sort((a, b) => (a.y - b.y) || (a.x - b.x));
    this._npc_index_lookup = new Map(this.npcs.map((npc) => [npc.objectIndex, npc]));
    if (controller?.add_npc) {
      controller.add_npc(obj);
    }
  }

  public reload_current_map(): void {
    this.load_map(this.current_map_name);
  }

  private _apply_pending_last_talked_position(): void {
    const wram = this.game_state?.wram;
    if (!wram) {
      return;
    }
    const pending = wram.pending_last_talked_position;
    if (!pending) {
      return;
    }
    const pending_map = wram.pending_last_talked_map;
    if (pending_map !== this.current_map_name) {
      return;
    }
    const pending_index = wram.pending_last_talked_object ?? 0;
    const trainer_index = pending_index || wram.last_talked || 0;
    const move_object = this.move_object;
    wram.pending_last_talked_position = undefined;
    wram.pending_last_talked_map = "";
    wram.pending_last_talked_object = 0;
    if (trainer_index <= 0 || typeof move_object !== "function") {
      return;
    }
    try {
      const [x, y] = pending;
      move_object.call(this, trainer_index, x, y);
    } catch {
      return;
    }
  }

  public refresh_event_flag(event_name: string, { value }: { value?: boolean } = {}): void {
    const resolvedValue = value ?? false;
    const script_runner = this.script_runner;
    const allow_refresh = script_runner ? script_runner.allow_event_flag_refresh ?? true : true;
    const scripts_active = Boolean(
      this.script_tasks_active?.() || (script_runner && script_runner._script_stack?.length)
    );
    if (scripts_active) {
      this._pending_event_flag_updates.push([event_name, resolvedValue, allow_refresh]);
      return;
    }
    if (allow_refresh) {
      this._apply_event_flag_update(event_name, resolvedValue);
    }
  }

  protected _apply_event_flag_update(event_name: string, value: boolean): void {
    const blueprint_map = this._npc_blueprints.get(this.current_map_name) ?? new Map<string, [ObjectEvent, number]>();
    if (!blueprint_map.size) {
      return;
    }
    const processed = new Set<number>();
    for (const [, [event, index]] of blueprint_map.entries()) {
      if (processed.has(index) || event.event_flag !== event_name) {
        continue;
      }
      processed.add(index);
      if (value) {
        this.remove_object(index, { update_event_flag: false });
      } else {
        this.appear_object(index, { ignore_event_flag: false, update_event_flag: false });
      }
    }
  }

  public get_event_flag_for_object_index(index: number): string | null {
    const blueprint_map = this._npc_blueprints.get(this.current_map_name) ?? new Map<string, [ObjectEvent, number]>();
    for (const [, [event, obj_index]] of blueprint_map.entries()) {
      if (obj_index !== index) {
        continue;
      }
      return this._real_object_event_flag(event.event_flag);
    }
    return null;
  }

  protected _bg_event_at(tile_x: number, tile_y: number): BackgroundEvent | null {
    const events = this._map_events?.bg_events ?? [];
    if (!events.length) {
      return null;
    }
    const stride = Math.max(1, Math.trunc(this.TILES_PER_COLLISION ?? 1));
    const offset = Math.max(0, stride - 1);
    for (const event of events) {
      const eventX = event.x * stride + offset;
      const eventY = event.y * stride + offset;
      if (eventX === tile_x && eventY === tile_y) {
        return event;
      }
    }
    return null;
  }

  public remove_background_event(event: BackgroundEvent | null): void {
    if (!event) {
      return;
    }
    const events = this._map_events?.bg_events ?? [];
    if (!events.length) {
      return;
    }
    const filtered = events.filter((candidate) => {
      return !(
        candidate.x === event.x &&
        candidate.y === event.y &&
        candidate.event_type === event.event_type &&
        candidate.script === event.script
      );
    });
    if (filtered.length === events.length) {
      return;
    }
    this._set_map_events({
      warps: this._map_events.warps ?? [],
      coord_events: this._map_events.coord_events ?? [],
      bg_events: filtered,
    } as MapEvents);
  }

  private _transition_to_connection(connection: MapConnection, direction: string, offset_tiles: number): void {
    const metadata = getMapMetadataByName(connection.target_map);
    if (!metadata) {
      throw new Error(
        `Unknown target map '${connection.target_map}' referenced by connection on ${this.current_map_name}`
      );
    }
    const prev_group = this.game_state.wram.wMapGroup;
    const prev_number = this.game_state.wram.wMapNumber;
    this.game_state.wram.wBackupMapGroup = prev_group;
    this.game_state.wram.wBackupMapNumber = prev_number;
    this.game_state.wram.wMapGroup = metadata.groupId;
    this.game_state.wram.wMapNumber = metadata.mapId;
    this.game_state.wram.current_map_group = metadata.groupId;
    this.game_state.wram.current_map_id = metadata.mapId;

    const original_x = this.player_x;
    const original_y = this.player_y;

    const new_width = metadata.width * METATILE_WIDTH;
    const new_height = metadata.height * METATILE_WIDTH;

    direction = direction.toLowerCase();
    let target_x: number;
    let target_y: number;
    if (direction === "north") {
      target_x = original_x - offset_tiles;
      target_y = new_height - 1;
    } else if (direction === "south") {
      target_x = original_x - offset_tiles;
      target_y = 0;
    } else if (direction === "west") {
      target_x = new_width - 1;
      target_y = original_y - offset_tiles;
    } else if (direction === "east") {
      target_x = 0;
      target_y = original_y - offset_tiles;
    } else {
      throw new Error(`Unsupported connection direction '${direction}'`);
    }

    const min_tile = Math.max(this.TILES_PER_COLLISION - 1, 0);
    const max_tile_x = Math.max(min_tile, new_width - 1);
    const max_tile_y = Math.max(min_tile, new_height - 1);
    this.player_x = Math.max(min_tile, Math.min(max_tile_x, target_x));
    this.player_y = Math.max(min_tile, Math.min(max_tile_y, target_y));
    this.prev_player_x = this.player_x;
    this.prev_player_y = this.player_y;
    this.target_tile_x = this.player_x;
    this.target_tile_y = this.player_y;
    this.is_moving = false;
    this._queued_direction = null;
    this.step_progress_px = 0.0;
    this.step_dx_px = 0.0;
    this.step_dy_px = 0.0;
    this._active_warp_tile = null;
    this._sync_player_state();

    this.load_map(connection.target_map);
    updateRoamMons(this.game_state);
    this._sync_player_state();
  }

  public show_emote(emote_id: string, obj: OverworldObject | null, duration: number): void {
    if (!obj || !obj.name) {
      return;
    }
    const normalized_emote = _normalise_emote_label(emote_id);
    if (!_EMOTE_IMAGE_MAP[normalized_emote]) {
      throw new Error(`Unsupported emote '${emote_id}' requested.`);
    }
    const frames = Math.max(1, Math.trunc(duration));
    this._active_emotes.set(obj, [normalized_emote, frames]);
  }

  public _tick_emotes(): void {
    if (!this._active_emotes.size) {
      return;
    }
    const expired: OverworldObject[] = [];
    for (const [obj, [emote_id, remaining]] of this._active_emotes.entries()) {
      const next = remaining - 1;
      if (next <= 0) {
        expired.push(obj);
        continue;
      }
      this._active_emotes.set(obj, [emote_id, next]);
    }
    expired.forEach((obj) => this._active_emotes.delete(obj));
  }

  public check_for_map_transition(): boolean {
    if (!this.data_loader.map_attributes?.has?.(this.current_map_name)) {
      throw new Error(
        `Missing map attributes for ${this.current_map_name} during transition`
      );
    }
    const map_attributes = this.data_loader.map_attributes.get(this.current_map_name);
    const map_tile_width = this.map.width * METATILE_WIDTH;
    const map_tile_height = this.map.height * METATILE_WIDTH;

    for (const connection of (map_attributes?.connections ?? [])) {
      const direction = String(connection.direction).toLowerCase();
      const offset_tiles = connection.offset * METATILE_WIDTH;
      if (direction === "north" && this.player_y < 0) {
        this._transition_to_connection(connection, direction, offset_tiles);
        return true;
      }
      if (direction === "south" && this.player_y >= map_tile_height) {
        this._transition_to_connection(connection, direction, offset_tiles);
        return true;
      }
      if (direction === "west" && this.player_x < 0) {
        this._transition_to_connection(connection, direction, offset_tiles);
        return true;
      }
      if (direction === "east" && this.player_x >= map_tile_width) {
        this._transition_to_connection(connection, direction, offset_tiles);
        return true;
      }
    }
    return false;
  }

  public check_for_wild_encounter(): void {
    if (this._wild_encounters?.maybe_trigger_random_encounter) {
      this._wild_encounters.maybe_trigger_random_encounter(this as OverworldLike);
    }
  }
}

export const OverworldMapManager = OverworldMapManagerMixin;

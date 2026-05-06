
import fs from "fs";
import path from "path";
import { readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { ObjectEvent } from "@pokecrystal/core/core/models/map";
import { getAssetPath, getDataDir } from "@pokecrystal/core/core/paths";
import { FacingDirection, PlayerState } from "@pokecrystal/core/core/enums/overworld";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";
import type { GameState } from "@pokecrystal/core/core/state";
import {
  getCoordCollision,
  isDirectionBlockedLeaving,
  isPermissionPassable,
} from "@pokecrystal/core/engine/world/overworld/collision-rules";
import { collectCollisionSamples } from "@pokecrystal/core/engine/world/overworld/ledge";
import { applyDefaultFacing } from "@pokecrystal/core/engine/world/overworld/npc-movement";
import { OverworldObject } from "@pokecrystal/core/engine/world/overworld/overworld-object";
import { OverworldBase } from "@pokecrystal/core/engine/world/overworld/overworld-base";
import { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import { PlayerCharacter } from "@pokecrystal/core/engine/world/overworld/playable-character";
import type { OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";
import { METATILE_WIDTH, TILE_SIZE } from "@pokecrystal/core/engine/world/tile/constants";
import { SpriteAnimation } from "@pokecrystal/core/engine/systems/animation";
import type { LoggerLike } from "@pokecrystal/core/engine/world/overworld/logger";
import { OverworldWithNpcInteraction } from "@pokecrystal/core/types/overworld";
import { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import { resolveNpcPaletteId } from "@pokecrystal/core/engine/world/overworld/sprite-palettes";

type SpriteAnimationMap = Record<string, SpriteAnimation>;
type BlueprintEntry = [ObjectEvent, number];
type Blueprint = Map<string, BlueprintEntry>;

export const setBlueprintIdentifier = (
  blueprint: Blueprint,
  identifier: string,
  entry: BlueprintEntry
): void => {
  const normalized = identifier.toUpperCase();
  const [event] = entry;
  const explicitIdentifier = event.object_identifier
    ? String(event.object_identifier).trim().toUpperCase()
    : null;
  const existing = blueprint.get(normalized);
  const [existingEvent] = existing ?? [null];
  const existingExplicitIdentifier = existingEvent?.object_identifier
    ? String(existingEvent.object_identifier).trim().toUpperCase()
    : null;

  if (existingExplicitIdentifier === normalized && explicitIdentifier !== normalized) {
    return;
  }

  blueprint.set(normalized, entry);
};

type NpcAutonomousControllerLike = {
  rebuild?: (npcs: OverworldObject[]) => void;
};

type ExtendedObjectEvent = Omit<ObjectEvent, "hram_y"> & {
  hram_y: number | string | null;
};

type OverworldObjectAlias = OverworldObject & Partial<{
  prev_x: number;
  prev_y: number;
  pixel_x: number;
  pixel_y: number;
  target_pixel_x: number;
  target_pixel_y: number;
  sprite_y_offset: number;
  step_frames_remaining: number;
  object_id: string | number | null;
}>;

const debugNpcIdentifier = (npc: OverworldObjectAlias): string => {
  const identifier = npc.object_id ?? npc.objectId ?? npc.constantId ?? npc.name;
  return String(identifier ?? "?");
};

type NpcSpriteCache = {
  instantiate: (
    spriteId: string,
    paletteId: number,
    timeOfDay: string | null | undefined
  ) => SpriteAnimationMap;
};

const TIME_OF_DAY_MASKS: Record<string, number> = {
  morn: 0b001,
  morning: 0b001,
  day: 0b010,
  afternoon: 0b010,
  nite: 0b100,
  night: 0b100,
  dark: 0b100,
  anytime: 0b111,
};

const DEBUG_LEVEL = 10;
const SUBTILE_KEY_MIN = -0x8000;
const SUBTILE_KEY_MAX = 0x7fff;
const FALLBACK_ITEM_BALL_SPRITE_ID = "POKE_BALL";
const MENU_ICONS_JSON_PATH = path.join(getDataDir(), "menu_icons.json");
const BIG_DOLL_MOVEMENT_TYPES = new Set([
  "SPRITEMOVEDATA_BIGDOLL",
  "SPRITEMOVEDATA_BIGDOLLASYM",
  "SPRITEMOVEDATA_BIGDOLLSYM",
]);

let menuIconBySpecies: Record<string, string> | null = null;

const loadMenuIconBySpecies = (): Record<string, string> => {
  if (!menuIconBySpecies) {
    menuIconBySpecies = readJsonAssetSync<Record<string, string>>(MENU_ICONS_JSON_PATH);
  }
  return menuIconBySpecies;
};

const iconAssetStem = (iconToken: string): string =>
  iconToken.trim().toLowerCase().replace(/^icon_/, "");

const pokemonIconAssetExists = (iconToken: string): boolean => {
  const stem = iconAssetStem(iconToken);
  if (!stem) {
    return false;
  }
  return fs.existsSync(getAssetPath("gfx", "icons", `${stem}.png`));
};

const resolvePokemonIconSpriteId = (spriteId: string): string | null => {
  const species = spriteId.replace(/^SPRITE_/, "").trim().toUpperCase();
  if (!species) {
    return null;
  }
  const iconToken = loadMenuIconBySpecies()[species];
  if (!iconToken || !pokemonIconAssetExists(iconToken)) {
    return null;
  }
  return iconToken.trim().toUpperCase();
};

const collisionStrideForObject = (
  npc: OverworldObject,
  defaultStride: number
): number => {
  const movement = String(npc.event?.spritemovedata ?? "").trim().toUpperCase();
  if (BIG_DOLL_MOVEMENT_TYPES.has(movement)) {
    return defaultStride * 2;
  }
  return defaultStride;
};

const isNpcDataRecord = (value: unknown): value is Record<string, ObjectEvent[]> => {
  return typeof value === "object" && value !== null && !Array.isArray(value);
};

const isRealObjectEventFlag = (value: unknown): value is string => {
  const flag = String(value ?? "").trim();
  return Boolean(flag && flag !== "0" && flag !== "-1");
};

const resolveNpcDataSource = (source: unknown, mapName: string): ObjectEvent[] => {
  if (source instanceof Map) {
    return source.get(mapName) ?? [];
  }
  if (isNpcDataRecord(source)) {
    return source[mapName] ?? [];
  }
  throw new Error("Unsupported npc_data structure; expected Map or record.");
};

export const resolveNpcDataList = (
  dataLoader: DataLoader | null | undefined,
  mapName: string
): ObjectEvent[] => {
  if (!dataLoader || dataLoader.npc_data === undefined) {
    throw new Error("NPC blueprint construction requires data_loader.npc_data.");
  }
  return resolveNpcDataSource(dataLoader.npc_data, mapName);
};

const syncNpcAliases = (npc: OverworldObject): void => {
  const aliasTarget = npc as OverworldObjectAlias;
  aliasTarget.prev_x = npc.prevX;
  aliasTarget.prev_y = npc.prevY;
  aliasTarget.pixel_x = npc.pixelX;
  aliasTarget.pixel_y = npc.pixelY;
  aliasTarget.target_pixel_x = npc.targetPixelX;
  aliasTarget.target_pixel_y = npc.targetPixelY;
  aliasTarget.sprite_y_offset = npc.spriteYOffset;
  aliasTarget.object_id = aliasTarget.object_id ?? npc.constantId ?? npc.objectId ?? npc.name;
};

export class OverworldNpcManagerMixin extends OverworldBase {
  public _logger: LoggerLike | null = null;
  protected TILES_PER_COLLISION!: number;
  protected _npc_blueprints!: Map<string, Blueprint>;
  protected _map_prefixes!: Array<[string, string]>;
  protected _sprite_root!: string;
  protected current_map_name!: string;
  protected data_loader!: DataLoader;
  protected _npc_sprite_cache!: NpcSpriteCache;
  protected _npc_autonomous_controller?: NpcAutonomousControllerLike | null;
  protected game_state!: GameState;
  protected map!: OverworldMap;
  protected tileset!: OverworldTilesetLike;
  public npcs: OverworldObject[] = [];
  protected follower?: OverworldObject | null;
  protected _npc_index_lookup!: Map<number, OverworldObject>;
  protected player_x!: number;
  protected player_y!: number;

  protected player_object!: PlayerCharacter | null;

  protected _tile_to_pixels(tileCoordinate: number): number {
    return OverworldBase._tileToPixels(tileCoordinate);
  }

  protected _preserve_animation_frames(
    existing: SpriteAnimation | null | undefined,
    updated: SpriteAnimation,
    options: { reload_standing: boolean; reload_walking: boolean }
  ): SpriteAnimation {
    const frames = [...updated.frames];
    if (!options.reload_standing && existing?.frames?.length) {
      frames[0] = existing.frames[0];
      if (frames.length > 2 && existing.frames.length > 2) {
        frames[2] = existing.frames[2];
      }
    }
    if (!options.reload_walking && existing && existing.frames.length > 1 && frames.length > 1) {
      frames[1] = existing.frames[1];
      if (frames.length > 3 && existing.frames.length > 3) {
        frames[3] = existing.frames[3];
      }
    }
    updated.frames = frames;
    if (existing) {
      updated.currentFrameIndex = Math.min(existing.currentFrameIndex, Math.max(frames.length - 1, 0));
      updated.frameCounter = existing.frameCounter;
      updated.animate = existing.animate;
      updated.facing = existing.facing;
    }
    return updated;
  }

  protected _build_blueprint(mapName: string): Blueprint {
    const existing = this._npc_blueprints.get(mapName);
    if (existing) {
      return existing;
    }

    const npcDataList = resolveNpcDataList(this.data_loader, mapName);
    const mapKey = mapName.replace(/\s+/g, "").toUpperCase();
    const blueprint: Blueprint = new Map();

    npcDataList.forEach((data, indexOffset) => {
      const index = indexOffset + 1;
      const event = data as ObjectEvent;
      const obj = new OverworldObject(event);
      obj.objectIndex = index;
      const identifiers = new Set<string>();
      identifiers.add(obj.spriteId);
      identifiers.add(obj.baseSpriteId);
      if (obj.objectId) {
        identifiers.add(obj.objectId);
      }
      identifiers.add(`${mapKey}_${obj.spriteId}`);
      identifiers.add(`${mapKey}_${obj.baseSpriteId}`);
      identifiers.add(`${mapKey}_${obj.spriteId}${index}`);
      if (obj.objectId) {
        identifiers.add(`${mapKey}_${obj.objectId}`);
      }

      const scriptName = String(event.script ?? "").trim();
      if (scriptName) {
        const normalizedScript = scriptName.toUpperCase();
        identifiers.add(normalizedScript);
        identifiers.add(`${mapKey}_${normalizedScript}`);
        identifiers.add(`${mapKey}_${normalizedScript}${index}`);
        if (normalizedScript.endsWith("SCRIPT")) {
          const trimmedScript = normalizedScript.slice(0, -"SCRIPT".length);
          if (trimmedScript) {
            identifiers.add(trimmedScript);
            identifiers.add(`${mapKey}_${trimmedScript}`);
            identifiers.add(`${mapKey}_${trimmedScript}${index}`);
          }
        }
      }

      if (obj.constantId) {
        identifiers.add(obj.constantId);
      }
      identifiers.add(String(index));

      for (const identifier of identifiers) {
        setBlueprintIdentifier(blueprint, identifier, [event, index]);
      }
    });

    this._npc_blueprints.set(mapName, blueprint);
    return blueprint;
  }

  protected _infer_map_name_from_identifier(identifier: string): string | null {
    const normalized = identifier.toUpperCase();
    for (const [prefix, mapName] of this._map_prefixes ?? []) {
      if (!normalized.startsWith(prefix)) {
        continue;
      }
      const nextCharIndex = prefix.length;
      if (nextCharIndex === normalized.length) {
        return mapName;
      }
      const nextChar = normalized[nextCharIndex];
      if (nextChar === "_" || /[0-9]/.test(nextChar)) {
        return mapName;
      }
    }
    return null;
  }

  protected _find_blueprint_entry(
    identifier: string
  ): [string | null, BlueprintEntry | null] {
    const normalized = identifier.toUpperCase();
    const currentMap = this.current_map_name;
    const currentBlueprint = this._npc_blueprints.get(currentMap);
    const entry = currentBlueprint?.get(normalized) ?? null;
    if (entry) {
      return [currentMap, entry];
    }

    const mapName = this._infer_map_name_from_identifier(normalized);
    if (!mapName) {
      return [null, null];
    }
    const blueprint = this._build_blueprint(mapName);
    const entryFromMap = blueprint.get(normalized) ?? null;
    if (!entryFromMap) {
      return [null, null];
    }
    return [mapName, entryFromMap];
  }

  protected _apply_variable_sprite(npc: OverworldObject): void {
    const replacement = this.game_state?.wram?.variable_sprites?.[npc.baseSprite] ?? null;
    if (replacement) {
      npc.setSprite(replacement);
    } else {
      npc.setSprite(npc.baseSprite);
    }
  }

  protected _initialise_object_coordinates(npc: OverworldObject): void {
    const stride = Math.floor(METATILE_WIDTH / 2);
    if (stride <= 0) {
      throw new Error("METATILE_WIDTH must be at least two for NPC placement.");
    }
    const collisionStride = collisionStrideForObject(npc, this.TILES_PER_COLLISION);
    const footprint = collisionStride - 1;
    npc.setCollisionStride(collisionStride);
    npc.x = npc.event.x * stride + footprint;
    npc.y = npc.event.y * stride + footprint;
    npc.prevX = npc.x;
    npc.prevY = npc.y;
    npc.initialSubtileX = npc.x;
    npc.initialSubtileY = npc.y;
    npc.updatePixelPosition();
    syncNpcAliases(npc);
  }

  protected _apply_initial_direction(npc: OverworldObject): void {
    const movement = String(npc.event.spritemovedata ?? "");
    npc.direction = applyDefaultFacing(movement, { direction: npc.direction });
  }

  protected _initialise_npc_object(
    npc: OverworldObject,
    options: {
      previous?: OverworldObject | null;
      reload_standing?: boolean;
      reload_walking?: boolean;
    } = {}
  ): void {
    const { previous = null, reload_standing = true, reload_walking = true } = options;
    try {
      // ASM: object_event palette 0 means "use sprite default" (macros/scripts/maps.asm).
      const paletteId = resolveNpcPaletteId(npc.spriteConstant, npc.palette);
      const timeOfDay = this.game_state?.wram?.time_of_day ?? null;
      let animations = this._npc_sprite_cache.instantiate(npc.spriteId, paletteId, timeOfDay);
      if (previous) {
        const merged: SpriteAnimationMap = {};
        const existing: SpriteAnimationMap = previous?.animations ?? {};
        for (const [direction, animation] of Object.entries(animations)) {
          merged[direction] = this._preserve_animation_frames(
            existing[direction],
            animation,
            { reload_standing: Boolean(reload_standing), reload_walking: Boolean(reload_walking) }
          );
        }
        animations = merged;
      }
      npc.animations = animations;
    } catch (error: unknown) {
      const message =
        error instanceof Error ? error.message : String(error ?? "");
      if (message.includes("Missing") || message.includes("not found")) {
        throw new Error(
          `Missing overworld sprite for '${npc.spriteId}' on map '${this.current_map_name}'.`
        );
      }
      throw new Error(
        `Invalid sprite data for '${npc.spriteId}' on map '${this.current_map_name}': ${message}`
      );
    }
  }

  protected _initialise_npc_object_safe(
    npc: OverworldObject,
    options: {
      previous?: OverworldObject | null;
      reload_standing?: boolean;
      reload_walking?: boolean;
    } = {}
  ): void {
    this._initialise_npc_object(npc, options);
  }

  protected _sprite_asset_exists(spriteId: string): boolean {
    if (!spriteId) {
      return false;
    }
    if (!this._sprite_root) {
      throw new Error("Sprite root is not configured for overworld NPCs.");
    }
    const spritePath = path.join(this._sprite_root, `${spriteId}.png`);
    if (fs.existsSync(spritePath)) {
      return true;
    }
    return fs.existsSync(path.join(this._sprite_root, `${spriteId.toLowerCase()}.png`));
  }

  protected _resolve_sprite_id_for_render(npc: OverworldObject): string | null {
    const spriteRequest = String(npc.spriteId ?? "").toUpperCase();
    const objectType = String(npc.event?.object_type ?? "").toUpperCase();
    const normalized = spriteRequest.replace(/^SPRITE_/, "").trim();

    const candidates = [
      normalized,
      normalized.replace(/-/, "_"),
      normalized === "POKEBALL" ? "POKE_BALL" : normalized,
      objectType === "OBJECTTYPE_ITEMBALL" ? FALLBACK_ITEM_BALL_SPRITE_ID : "",
      objectType === "OBJECTTYPE_ITEMBALL" || objectType === "OBJECTTYPE_ITEM_BALL" || normalized.includes("POKE")
        ? FALLBACK_ITEM_BALL_SPRITE_ID
        : "",
    ];

    for (const candidate of candidates) {
      if (!candidate) {
        continue;
      }
      if (this._sprite_asset_exists(candidate)) {
        return candidate;
      }
    }
    return resolvePokemonIconSpriteId(normalized);
  }

  protected _object_should_spawn(
    npc: OverworldObject,
    options: { ignore_event_flag?: boolean } = {}
  ): boolean {
    const ignoreEventFlag = options.ignore_event_flag ?? true;
    const timeMask = this._current_time_of_day_mask();
    const eventData = npc.event as ExtendedObjectEvent;
    const scheduledMaskRaw = eventData.hram_y;
    let scheduledMask: number | null = null;
    if (typeof scheduledMaskRaw === "number") {
      scheduledMask = scheduledMaskRaw;
    } else if (typeof scheduledMaskRaw === "string") {
      const trimmed = scheduledMaskRaw.trim();
      if (/^-?\d+$/.test(trimmed)) {
        scheduledMask = Number(trimmed);
      } else if (trimmed) {
        scheduledMask = TIME_OF_DAY_MASKS[trimmed.toLowerCase()] ?? null;
      }
    }
    if (
      typeof scheduledMask === "number" &&
      scheduledMask !== -1 &&
      scheduledMask !== 0 &&
      timeMask !== null &&
      (scheduledMask & timeMask) === 0
    ) {
      return false;
    }

    const eventFlag = isRealObjectEventFlag(npc.event.event_flag) ? npc.event.event_flag : null;
    if (!ignoreEventFlag && eventFlag) {
      const eventFlags = this.game_state?.wram?.event_flags ?? {};
      if (eventFlags[eventFlag]) {
        return false;
      }
    }

    const resolvedSpriteId = this._resolve_sprite_id_for_render(npc);
    if (!resolvedSpriteId) {
      if (this._logger?.isEnabledFor?.(DEBUG_LEVEL)) {
        this._logger.debug?.(
          "Skipping NPC '%s' on map '%s' because no renderable sprite was found for '%s'.",
          debugNpcIdentifier(npc),
          this.current_map_name,
          npc.spriteId
        );
      }
      return false;
    }
    if (resolvedSpriteId !== npc.spriteId) {
      if (this._logger?.isEnabledFor?.(DEBUG_LEVEL)) {
        this._logger.debug?.(
          "Using fallback sprite for NPC '%s' on map '%s': '%s' -> '%s'.",
          debugNpcIdentifier(npc),
          this.current_map_name,
          npc.spriteId,
          resolvedSpriteId
        );
      }
      npc.setSprite(`SPRITE_${resolvedSpriteId}`);
    }
    return true;
  }

  protected _load_and_sort_sprites(objects: OverworldObject[]): OverworldObject[] {
    // ASM: engine/overworld/map_objects.asm::LoadAndSortSprites
    objects.sort((a, b) => {
      if (a.y !== b.y) {
        return a.y - b.y;
      }
      if (a.x !== b.x) {
        return a.x - b.x;
      }
      return (a.objectIndex ?? 0) - (b.objectIndex ?? 0);
    });
    return objects;
  }

  protected _add_map_sprites(
    entries: BlueprintEntry[],
    options: {
      previous_by_index?: Map<number, OverworldObject> | Record<number, OverworldObject>;
      reload_standing?: boolean;
      reload_walking?: boolean;
      reuse_previous?: boolean;
    } = {}
  ): OverworldObject[] {
    // ASM: engine/overworld/map_objects.asm::AddMapSprites
    const {
      previous_by_index = undefined,
      reload_standing = true,
      reload_walking = true,
      reuse_previous = false,
    } = options;
    const processed: OverworldObject[] = [];
    for (const [event, index] of entries) {
      const previous =
        previous_by_index instanceof Map
          ? previous_by_index.get(index)
          : previous_by_index?.[index];
      const usePrevious = Boolean(reuse_previous && previous);
      const npc = usePrevious ? (previous as OverworldObject) : new OverworldObject(event);
      npc.event = event;
      npc.objectIndex = index;
      this._apply_variable_sprite(npc);
      if (!usePrevious) {
        this._initialise_object_coordinates(npc);
        this._apply_initial_direction(npc);
      }
      if (!this._object_should_spawn(npc, { ignore_event_flag: false })) {
        continue;
      }
      this._initialise_npc_object_safe(npc, {
        previous: usePrevious ? npc : previous ?? null,
        reload_standing,
        reload_walking,
      });
      processed.push(npc);
    }
    return this._load_and_sort_sprites(processed);
  }

  public refresh_map_sprites(
    options: { reload_standing?: boolean; reload_walking?: boolean } = {}
  ): void {
    // ASM: engine/overworld/map_objects.asm::RefreshSprites
    const { reload_standing = true, reload_walking = true } = options;
    const mapName = this.current_map_name ?? "";
    if (!mapName) {
      throw new Error("Cannot refresh map sprites without an active map.");
    }

    const blueprint = this._build_blueprint(mapName);
    const previousByIndex = new Map<number, OverworldObject>();
    for (const npc of this.npcs ?? []) {
      previousByIndex.set(npc.objectIndex, npc);
    }

    const eventsByIndex = new Map<number, ObjectEvent>();
    for (const [, [event, index]] of blueprint.entries()) {
      if (!eventsByIndex.has(index)) {
        eventsByIndex.set(index, event);
      }
    }

    const entries = [...eventsByIndex.entries()].sort(([a], [b]) => a - b);
    const mappedEntries: BlueprintEntry[] = entries.map(([index, event]) => [event, index]);

    this.npcs = this._add_map_sprites(mappedEntries, {
      previous_by_index: previousByIndex,
      reload_standing,
      reload_walking,
      reuse_previous: true,
    });

    this._npc_index_lookup = new Map(
      this.npcs.map((npc) => [npc.objectIndex, npc])
    );

    const controller = this._npc_autonomous_controller ?? null;
    controller?.rebuild?.(this.npcs);

    const follower = this.follower ?? null;
    if (follower && "objectIndex" in follower && !this.npcs.includes(follower as OverworldObject)) {
      const npcFollower = follower as OverworldObject;
      this._initialise_npc_object_safe(npcFollower, {
        previous: previousByIndex.get(npcFollower.objectIndex) ?? null,
        reload_standing,
        reload_walking,
      });
    }
  }

  protected _current_time_of_day_mask(): number | null {
    const label = this.game_state?.wram?.time_of_day ?? null;
    if (!label) {
      return TIME_OF_DAY_MASKS.day;
    }
    const key = String(label).trim().toLowerCase();
    return TIME_OF_DAY_MASKS[key] ?? TIME_OF_DAY_MASKS.day;
  }

  protected _npc_footprint_origin(npc: OverworldObject): [number, number] {
    const stride = Math.max(1, npc.collisionStride ?? this.TILES_PER_COLLISION);
    const footprint = stride - 1;
    return [npc.x - footprint, npc.y - footprint];
  }

  protected _npc_is_moving(npc: OverworldObject): boolean {
    const alias = npc as OverworldObjectAlias;
    return Boolean(
      npc.walking ||
      npc.jumping ||
      Number(alias.step_frames_remaining ?? npc.stepFramesRemaining ?? 0) > 0
    );
  }

  protected _npc_frame_size(npc: OverworldObject): [number, number] {
    const animations = npc.animations ?? {};
    const animation = animations[npc.direction] ?? animations.down;
    const frame = animation?.currentFrame as
      | {
          get_width?: () => number;
          get_height?: () => number;
          width?: number;
          height?: number;
        }
      | null
      | undefined;
    const width = typeof frame?.get_width === "function"
      ? frame.get_width()
      : Number(frame?.width ?? 0);
    const height = typeof frame?.get_height === "function"
      ? frame.get_height()
      : Number(frame?.height ?? 0);
    return [
      Number.isFinite(width) && width > 0 ? Math.trunc(width) : 0,
      Number.isFinite(height) && height > 0 ? Math.trunc(height) : 0,
    ];
  }

  protected _npc_collision_subtiles(npc: OverworldObject): Array<[number, number]> {
    const stride = Math.max(1, npc.collisionStride ?? this.TILES_PER_COLLISION);
    const footprint = stride - 1;
    const baseWidthPx = stride * TILE_SIZE;
    const [frameWidth, frameHeight] = this._npc_frame_size(npc);
    const collisionWidthPx = Math.max(baseWidthPx, frameWidth);
    const collisionHeightPx = Math.max(baseWidthPx, frameHeight);
    const coords: Array<[number, number]> = [];
    const seen = new Set<string>();

    const appendRect = (minX: number, minY: number, maxX: number, maxY: number): void => {
      for (let x = minX; x <= maxX; x += 1) {
        for (let y = minY; y <= maxY; y += 1) {
          const key = `${x},${y}`;
          if (seen.has(key)) {
            continue;
          }
          seen.add(key);
          coords.push([x, y]);
        }
      }
    };

    const appendPixelRect = (pixelX: number, pixelY: number): void => {
      const minX = Math.floor(pixelX / TILE_SIZE);
      const minY = Math.floor(pixelY / TILE_SIZE);
      const maxX = Math.floor((pixelX + collisionWidthPx - 1) / TILE_SIZE);
      const maxY = Math.floor((pixelY + collisionHeightPx - 1) / TILE_SIZE);
      appendRect(minX, minY, maxX, maxY);
    };

    if (this._npc_is_moving(npc)) {
      const alias = npc as OverworldObjectAlias;
      const pixelX = alias.pixel_x ?? npc.pixelX;
      const pixelY = alias.pixel_y ?? npc.pixelY;
      if (Number.isFinite(pixelX) && Number.isFinite(pixelY)) {
        appendPixelRect(Number(pixelX), Number(pixelY));
        return coords;
      }

      const prevX = alias.prev_x ?? npc.prevX ?? npc.x;
      const prevY = alias.prev_y ?? npc.prevY ?? npc.y;
      appendRect(prevX - footprint, prevY - footprint, prevX, prevY);
      return coords;
    }

    const alias = npc as OverworldObjectAlias;
    const pixelX = alias.pixel_x ?? npc.pixelX;
    const pixelY = alias.pixel_y ?? npc.pixelY;
    const expectedPixelX = this._tile_to_pixels(npc.x - footprint);
    const expectedPixelY = this._tile_to_pixels(npc.y - footprint);
    if (
      Number.isFinite(pixelX) &&
      Number.isFinite(pixelY) &&
      Number(pixelX) === expectedPixelX &&
      Number(pixelY) === expectedPixelY
    ) {
      appendPixelRect(Number(pixelX), Number(pixelY));
      return coords;
    }

    appendRect(npc.x - footprint, npc.y - footprint, npc.x, npc.y);
    return coords;
  }

  protected _npc_subtiles(npc: OverworldObject): Array<[number, number]> {
    return this._npc_collision_subtiles(npc);
  }

  protected _subtile_in_footprint(
    subtileX: number,
    subtileY: number,
    originX: number,
    originY: number,
    stride: number
  ): boolean {
    return (
      subtileX >= originX &&
      subtileX < originX + stride &&
      subtileY >= originY &&
      subtileY < originY + stride
    );
  }

  protected _subtile_key(subtileX: number, subtileY: number): number {
    const x = Math.trunc(subtileX);
    const y = Math.trunc(subtileY);
    if (x < SUBTILE_KEY_MIN || x > SUBTILE_KEY_MAX || y < SUBTILE_KEY_MIN || y > SUBTILE_KEY_MAX) {
      throw new Error(
        `Subtile coordinate overflow in occupancy lookup (${x},${y}); expected 16-bit signed tile coordinates.`
      );
    }
    return ((x + 0x8000) << 16) | (y + 0x8000);
  }

  protected _npc_covers_subtile(
    npc: OverworldObject,
    subtileX: number,
    subtileY: number
  ): boolean {
    return this._npc_collision_subtiles(npc).some(
      ([x, y]) => x === subtileX && y === subtileY
    );
  }

  protected _npc_occupying_subtile(subtileX: number, subtileY: number): OverworldObject | null {
    for (const npc of this.npcs ?? []) {
      if (this._npc_covers_subtile(npc, subtileX, subtileY)) {
        return npc;
      }
    }
    return null;
  }

  protected _npc_occupancy_lookup(): (x: number, y: number) => OverworldObject | null {
    const lookup = new Map<number, OverworldObject>();
    const appendSubtiles = (
      owner: OverworldObject,
      subtiles: Array<[number, number]>
    ): void => {
      for (const [x, y] of subtiles) {
        const key = this._subtile_key(x, y);
        if (!lookup.has(key)) {
          lookup.set(key, owner);
        }
      }
    };

    for (const npc of this.npcs ?? []) {
      appendSubtiles(npc, this._npc_collision_subtiles(npc));
    }

    return (x: number, y: number) => lookup.get(this._subtile_key(x, y)) ?? null;
  }

  protected _npc_step_blocked(
    npc: OverworldObject,
    direction: string,
    target_tile_x: number,
    target_tile_y: number,
    options: { is_player_target?: boolean; player_only?: boolean; suppress_blocked_log?: boolean } = {}
  ): boolean {
    const isPlayerTarget = options.is_player_target ?? false;
    const playerOnly = options.player_only ?? false;
    const suppressBlockedLog = options.suppress_blocked_log ?? false;
    const stride = Math.max(1, this.TILES_PER_COLLISION);
    const footprint = stride - 1;
    const alias = npc as OverworldObjectAlias;
    const npcLabel = debugNpcIdentifier(alias);

    const appendFootprint = (
      occupied: Set<number>,
      centerX: number,
      centerY: number,
      collisionStride: number
    ): void => {
      const collisionFootprint = collisionStride - 1;
      const originX = centerX - collisionFootprint;
      const originY = centerY - collisionFootprint;
      for (let dx = 0; dx < collisionStride; dx += 1) {
        for (let dy = 0; dy < collisionStride; dy += 1) {
          occupied.add(this._subtile_key(originX + dx, originY + dy));
        }
      }
    };

    let playerPositions: Array<[number, number]> = [];
    if (!isPlayerTarget) {
      playerPositions = [[this.player_x, this.player_y]];
      const moving =
        Boolean((this as unknown as { is_moving?: boolean }).is_moving) ||
        Boolean(this.player_object?.walking) ||
        Boolean(this.player_object?.jumping);
      const movementState = this as unknown as { prev_player_x?: number; prev_player_y?: number };
      if (
        moving &&
        Number.isFinite(movementState.prev_player_x) &&
        Number.isFinite(movementState.prev_player_y)
      ) {
        const prevX = Number(movementState.prev_player_x);
        const prevY = Number(movementState.prev_player_y);
        if (prevX !== this.player_x || prevY !== this.player_y) {
          // ASM: IsNPCAtCoord checks both OBJECT_MAP_* and OBJECT_LAST_MAP_* while objects are moving.
          playerPositions.push([prevX, prevY]);
        }
      }
      const targetTile = this as unknown as { target_tile_x?: number; target_tile_y?: number };
      if (
        moving &&
        Number.isFinite(targetTile.target_tile_x) &&
        Number.isFinite(targetTile.target_tile_y)
      ) {
        const targetX = Number(targetTile.target_tile_x);
        const targetY = Number(targetTile.target_tile_y);
        if (targetX !== this.player_x || targetY !== this.player_y) {
          // ASM: InitStep sets OBJECT_MAP_X/Y to the destination while LAST_MAP_* holds the origin.
          playerPositions.push([targetX, targetY]);
        }
      } else if (!moving) {
        const pendingAutoStep = (this as unknown as {
          _pending_auto_step?: [string, boolean] | null;
        })._pending_auto_step;
        const heldDirections = (this as unknown as {
          _held_directions?: Map<string, unknown>;
        })._held_directions;
        const movementLocked = Boolean(
          (this as unknown as { player_movement_locked?: () => boolean }).player_movement_locked?.(),
        );
        const isDownhillCoast = (directionLabel: string): boolean => {
          const helper = (this as unknown as {
            _is_downhill_coast_direction?: (direction: string) => boolean;
          })._is_downhill_coast_direction;
          if (!helper) {
            return false;
          }
          return Boolean(helper.call(this as unknown as object, directionLabel));
        };
        const [autoDirection, autoForced] = pendingAutoStep ?? [null, false];
        if (
          !movementLocked &&
          autoDirection &&
          (
          autoForced ||
          (typeof heldDirections?.has === "function" && heldDirections.has(autoDirection)) ||
          isDownhillCoast(autoDirection)
        )) {
          const vector = String(autoDirection).toLowerCase();
          const stride = this.TILES_PER_COLLISION;
          if (vector === "up") {
            playerPositions.push([this.player_x, this.player_y - stride]);
          } else if (vector === "down") {
            playerPositions.push([this.player_x, this.player_y + stride]);
          } else if (vector === "left") {
            playerPositions.push([this.player_x - stride, this.player_y]);
          } else if (vector === "right") {
            playerPositions.push([this.player_x + stride, this.player_y]);
          }
        }
      }
    }

    const targetOverlaps = (occupied: Set<number>): boolean => {
      for (let dx = 0; dx < stride; dx += 1) {
        for (let dy = 0; dy < stride; dy += 1) {
          if (occupied.has(this._subtile_key(target_tile_x - dx, target_tile_y - dy))) {
            return true;
          }
        }
      }
      return false;
    };

    if (playerOnly) {
      const playerOccupied = new Set<number>();
      for (const [playerX, playerY] of playerPositions) {
        appendFootprint(playerOccupied, playerX, playerY, stride);
      }
      return targetOverlaps(playerOccupied);
    }

    if (!this.map || !this.tileset) {
      return true;
    }

    const maxTileX = this.map.width * METATILE_WIDTH;
    const maxTileY = this.map.height * METATILE_WIDTH;

    const facing = FacingDirection.fromString(direction);
    const leavingPermission = getCoordCollision(this.map, this.tileset, npc.x, npc.y);
    if (isDirectionBlockedLeaving(leavingPermission, facing)) {
      if (!suppressBlockedLog && this._logger?.isEnabledFor?.(DEBUG_LEVEL)) {
        this._logger.debug?.(
          "Movement blocked by leaving collision %s at (%d,%d) for %s",
          leavingPermission,
          npc.x,
          npc.y,
          npcLabel
        );
      }
      return true;
    }
    const occupied = new Set<number>();
    const appendNpcOccupancy = (other: OverworldObject): void => {
      for (const [x, y] of this._npc_collision_subtiles(other)) {
        occupied.add(this._subtile_key(x, y));
      }
    };
    for (const other of this.npcs ?? []) {
      if (other === npc) {
        continue;
      }
      appendNpcOccupancy(other);
    }
    for (const [playerX, playerY] of playerPositions) {
      appendFootprint(occupied, playerX, playerY, stride);
    }

    const fullyInside =
      target_tile_x - footprint >= 0 &&
      target_tile_y - footprint >= 0 &&
      target_tile_x < maxTileX &&
      target_tile_y < maxTileY;
    if (fullyInside) {
      const samples = collectCollisionSamples(
        this.map,
        this.tileset,
        target_tile_x,
        target_tile_y,
        stride
      );
      if (samples.length !== stride * stride) {
        return true;
      }
      for (const sample of samples) {
        if (occupied.has(this._subtile_key(sample.tileX, sample.tileY))) {
          if (!suppressBlockedLog && this._logger?.isEnabledFor?.(DEBUG_LEVEL)) {
            this._logger.debug?.(
              "Movement blocked by occupant at (%d,%d) for %s",
              sample.tileX,
              sample.tileY,
              npcLabel
            );
          }
          return true;
        }
        if (!isPermissionPassable(sample.permission, facing, PlayerState.NORMAL)) {
          if (!suppressBlockedLog && this._logger?.isEnabledFor?.(DEBUG_LEVEL)) {
            this._logger.debug?.(
              "Movement blocked by collision %s at (%d,%d) for %s",
              sample.permission,
              sample.tileX,
              sample.tileY,
              npcLabel
            );
          }
          return true;
        }
      }
      return false;
    }

    for (let dx = 0; dx < stride; dx += 1) {
      for (let dy = 0; dy < stride; dy += 1) {
        const subtileX = target_tile_x - dx;
        const subtileY = target_tile_y - dy;
        if (subtileX < 0 || subtileY < 0) {
          if (!isPlayerTarget || subtileX < -stride || subtileY < -stride) {
            if (!suppressBlockedLog && this._logger?.isEnabledFor?.(DEBUG_LEVEL)) {
              this._logger.debug?.(
                "Movement blocked at boundary (%d,%d) for %s",
                subtileX,
                subtileY,
                npcLabel
              );
            }
            return true;
          }
          continue;
        }
        if (subtileX >= maxTileX || subtileY >= maxTileY) {
          if (
            !isPlayerTarget ||
            subtileX >= maxTileX + stride ||
            subtileY >= maxTileY + stride
          ) {
            if (!suppressBlockedLog && this._logger?.isEnabledFor?.(DEBUG_LEVEL)) {
              this._logger.debug?.(
                "Movement blocked at boundary (%d,%d) for %s",
                subtileX,
                subtileY,
                npcLabel
              );
            }
            return true;
          }
          continue;
        }
        if (occupied.has(this._subtile_key(subtileX, subtileY))) {
          if (!suppressBlockedLog && this._logger?.isEnabledFor?.(DEBUG_LEVEL)) {
            this._logger.debug?.(
              "Movement blocked by occupant at (%d,%d) for %s",
              subtileX,
              subtileY,
              npcLabel
            );
          }
          return true;
        }
        const permission = getCoordCollision(this.map, this.tileset, subtileX, subtileY);
        if (!isPermissionPassable(permission, facing, PlayerState.NORMAL)) {
          if (!suppressBlockedLog && this._logger?.isEnabledFor?.(DEBUG_LEVEL)) {
            this._logger.debug?.(
              "Movement blocked by collision %s at (%d,%d) for %s",
              permission,
              subtileX,
              subtileY,
              npcLabel
            );
          }
          return true;
        }
      }
    }

    return false;
  }

  protected _npc_pixel_position(npc: OverworldObject): [number, number] {
    const alias = npc as OverworldObjectAlias;
    const pixelX = alias.pixel_x ?? npc.pixelX;
    const pixelY = alias.pixel_y ?? npc.pixelY;
    let baseX: number;
    let baseY: number;
    if (typeof pixelX === "number" && typeof pixelY === "number") {
      baseX = Number(pixelX);
      baseY = Number(pixelY);
    } else {
      const stride = Math.max(1, npc.collisionStride ?? this.TILES_PER_COLLISION);
      const footprint = stride - 1;
      const originX = npc.x - footprint;
      const originY = npc.y - footprint;
      baseX = this._tile_to_pixels(originX);
      baseY = this._tile_to_pixels(originY);
    }
    const offsetY = Number(alias.sprite_y_offset ?? npc.spriteYOffset ?? 0.0);
    return [Math.trunc(baseX), Math.trunc(baseY + offsetY)];
  }

  public _remember_trainer_contact(npc: OverworldObject): void {
    if (!npc || !this.game_state) {
      return;
    }
    const event = npc.event;
    if (!event) {
      return;
    }
    if (String(event.object_type ?? "").toUpperCase() !== "OBJECTTYPE_TRAINER") {
      return;
    }
    this.game_state.wram.seen_trainer_distance = 1;
    this.game_state.wram.seen_trainer_direction = "";
  }

  public check_for_npc_interaction(): boolean {
    const overworld = this as unknown as OverworldWithNpcInteraction;

    const runner = overworld.script_runner;
    if (runner) {
      runner.last_interaction_object_index = null;
    }
    if (runner && runner.is_busy) {
      return false;
    }

    let [tile_x, tile_y] = overworld.get_facing_tile_coords();
    if (tile_x < 0 || tile_y < 0) {
        return false;
    }

    if (!overworld.map) {
        return false;
    }

    const findNpc = (x: number, y: number): OverworldObject | null =>
      this._npc_on_tile(x, y) ?? this._nearest_npc_covering_subtile(x, y);

    let best_npc = findNpc(tile_x, tile_y);
    if (!best_npc) {
      [tile_x, tile_y] = overworld._counter_adjusted_tile(tile_x, tile_y);
      best_npc = findNpc(tile_x, tile_y);
    }

    if (!best_npc) {
        if (this.game_state?.wram) {
            this.game_state.wram.last_talked = 0;
        }
        return false;
    }
    // ASM: CheckFacingObject rejects NPCs that are still walking.
    if (best_npc.walking || best_npc.jumping) {
        if (this.game_state?.wram) {
            this.game_state.wram.last_talked = 0;
        }
        return false;
    }

    if (!this._scripted_interaction_position_allowed(best_npc)) {
        if (this.game_state?.wram) {
            this.game_state.wram.last_talked = 0;
        }
        return false;
    }

    if (this.game_state?.wram) {
      this.game_state.wram.last_talked = best_npc.objectIndex ?? 0;
    }
    if (runner) {
      runner.last_interaction_object_index = best_npc.objectIndex ?? null;
    }
    const player = this.player_object ?? null;
    const playerX = player?.x ?? this.player_x;
    const playerY = player?.y ?? this.player_y;
    if (typeof best_npc.facePlayer === "function") {
      best_npc.facePlayer(playerX, playerY);
    } else if (typeof (best_npc as any).face_player === "function") {
      (best_npc as any).face_player(playerX, playerY);
    }
    overworld._play_interaction_sound();
    const script = best_npc.event?.script;
    if (script && runner) {
      this._remember_trainer_contact(best_npc);
      const objectType = String(best_npc.event?.object_type ?? "").toUpperCase();
      if (objectType === "OBJECTTYPE_TRAINER") {
        runner.run(script, { allow_fallthrough: false });
      } else {
        runner.run(script);
      }
    }
    return true;
  }

  protected _scripted_interaction_position_allowed(npc: OverworldObject): boolean {
    const script = String(npc.event?.script ?? "");
    if (this.current_map_name !== "NewBarkTown" || script !== "NewBarkTownRivalScript") {
      return true;
    }
    const stride = Math.max(1, Math.trunc(this.TILES_PER_COLLISION ?? npc.collisionStride ?? 1));
    return this.player_x === npc.x - stride && this.player_y === npc.y;
  }

  public _npc_on_tile(tile_x: number, tile_y: number): OverworldObject | null {
    let bestNpc: OverworldObject | null = null;
    let bestIndex = Infinity;
    for (const npc of this.npcs ?? []) {
      if (!this._npc_covers_subtile(npc, tile_x, tile_y)) {
        continue;
      }
      const npcIndex = npc.objectIndex ?? 0;
      // ASM ordering uses object index ascending (map object slot order).
      if (npcIndex < bestIndex) {
        bestNpc = npc;
        bestIndex = npcIndex;
      }
    }
    return bestNpc;
  }

  public _nearest_npc_covering_subtile(facing_x: number, facing_y: number): OverworldObject | null {
      let best_npc: OverworldObject | null = null;
      let best_distance = Infinity;
      for (let dx = 0; dx < this.TILES_PER_COLLISION; dx++) {
          for (let dy = 0; dy < this.TILES_PER_COLLISION; dy++) {
              const subtile_x = facing_x - dx;
              const subtile_y = facing_y - dy;
              const npc = this._npc_occupying_subtile(subtile_x, subtile_y);
              if (npc === null) {
                  continue;
              }
              const distance = Math.abs(npc.x - facing_x) + Math.abs(npc.y - facing_y);
              if (best_npc === null || distance < best_distance) {
                  best_npc = npc;
                  best_distance = distance;
              } else if (distance === best_distance) {
                  if (best_npc !== null && npc.objectIndex < best_npc.objectIndex) {
                      best_npc = npc;
                  }
              }
          }
      }
      return best_npc;
  }

  public get_object_by_id(
    object_id: string | number
  ): OverworldObject | PlayerCharacter | null {
    const npcs: OverworldObject[] = this.npcs ?? [];
    const normalized_id = String(object_id).toUpperCase();
    if (["PLAYER", "0"].includes(normalized_id)) {
      return this.player_object ?? null;
    }

    const numeric_id = Number(object_id);
    if (
      !isNaN(numeric_id) &&
      Number.isInteger(numeric_id) &&
      String(numeric_id) === String(object_id)
    ) {
      const by_index = this._npc_index_lookup?.get(numeric_id) ?? null;
      if (by_index) {
        return by_index;
      }
      const by_object_index = npcs.find((npc) => npc.objectIndex === numeric_id) ?? null;
      if (by_object_index) {
        return by_object_index;
      }
    }

    for (const npc of npcs ?? []) {
      const identifiers = [
        String((npc as any).object_id ?? "").toUpperCase(),
        String(npc.constantId ?? "").toUpperCase(),
        String(npc.objectId ?? "").toUpperCase(),
        String(npc.name ?? "").toUpperCase(),
        String(npc.baseSpriteId ?? "").toUpperCase(),
        String(npc.spriteId ?? "").toUpperCase(),
        String(npc.event?.script ?? "").toUpperCase(),
        String(npc.event?.object_type ?? "").toUpperCase(),
      ].filter(Boolean);
      if (identifiers.includes(normalized_id)) {
        return npc;
      }
    }

    return null;
  }
}

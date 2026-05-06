import { beginFlarePlotFrame, finishFlarePlotFrame } from "@pokecrystal/core/ui/flare-plot-renderer";
// ASM reference: engine/overworld/overworld.asm (sprite priority, camera, and overlay passes).
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { FacingDirection, PlayerState } from "@pokecrystal/core/core/enums/overworld";
import type { Pokemon } from "@pokecrystal/core/core/models";
import {
  describeCollision,
  resolveCollisionValue,
  Terrain,
} from "@pokecrystal/core/engine/world/overworld/collision-data";
import type { CollisionAttributes } from "@pokecrystal/core/engine/world/overworld/collision-data";
import {
  CollisionSample,
  isPermissionPassable,
  sampleCollision,
} from "@pokecrystal/core/engine/world/overworld/collision-rules";
import {
  _DIRECTION_VECTORS as BASE_DIRECTION_VECTORS,
  SPRITES_SKIP_WALKING,
} from "@pokecrystal/core/engine/world/overworld/constants";
import { getLedgeJumpOffset } from "@pokecrystal/core/engine/world/overworld/jump-offsets";
import { OverworldBase } from "@pokecrystal/core/engine/world/overworld/overworld-base";
import { scaleTileCoord, unscaleTileCoord } from "@pokecrystal/core/engine/world/overworld/tile-coords";
import { METATILE_SIZE, METATILE_WIDTH, TILE_SIZE } from "@pokecrystal/core/engine/world/tile/constants";
import { buildOverworldMetadata } from "@pokecrystal/core/ui/text-overlays";
import { filterPromptContextLines } from "@pokecrystal/core/ui/text/prompt-context";
import { CompositeUI } from "@pokecrystal/core/ui/composite-ui";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { GameState } from "@pokecrystal/core/core/state";
import { GrassRustleController, type GrassRustleTarget } from "@pokecrystal/core/engine/world/overworld/grass-rustle";
import type { OverworldMap } from "./overworld-map";
import type { OverworldObject } from "./overworld-object";
import type { PlayerCharacter } from "./playable-character";
import type { TownMapOverlayLike } from "@pokecrystal/core/ui/overlays/town-map-overlay";
import type { OverworldTilesetLike } from "./tileset-types";
import type { CompositeSegment } from "./connection-composite";
import type { RemoteOverworldPlayer } from "@pokecrystal/core/types/overworld";

type Surface = InstanceType<typeof gameEngine.Surface>;
type TextTarget = TextUI;

type UiWithChildren = {
  getChildren: () => unknown[];
};

const hasChildren = (candidate: unknown): candidate is UiWithChildren =>
  Boolean(candidate) && typeof (candidate as UiWithChildren).getChildren === "function";

const isTextTarget = (candidate: unknown): candidate is TextTarget => {
  if (!candidate) {
    return false;
  }
  return (
    typeof (candidate as TextTarget).renderOverworldOverlay === "function" ||
    typeof (candidate as TextTarget).renderSnapshot === "function"
  );
};

const formatAxisHeader = (start: number, length: number): string[] => {
  if (length <= 0) {
    return [""];
  }
  const labels = Array.from({ length }, (_, idx) => String(start + idx).padStart(2, "0"));
  return [labels.join(" ")];
};

const ASCII_VIEWPORT_WIDTH = 20;
const ASCII_VIEWPORT_HEIGHT = 18;

const formatRowLabel = (value: number, width: number): string =>
  String(value).padStart(Math.max(width, 2), "0");

const ASCII_LEGEND_DEFINITIONS: Array<{ token: string; label: string }> = [
  { token: "@", label: "Player" },
  { token: "N", label: "Person" },
  { token: "V", label: "Vendor" },
  { token: "+", label: "Healer" },
  { token: "B", label: "Berry tree/Bookshelf" },
  { token: "I", label: "Item ball" },
  { token: "D", label: "Door" },
  { token: "P", label: "PC/Center" },
  { token: "G", label: "Gym/House" },
  { token: "M", label: "Mart" },
  { token: "C", label: "Cut tree" },
  { token: "H", label: "Headbutt tree" },
  { token: "$", label: "Shop/Shelf" },
  { token: "S", label: "Sign" },
  { token: "W", label: "Window" },
  { token: "T", label: "Counter/Trash" },
  { token: "=", label: "Waterfall" },
  { token: "!", label: "Talkable" },
  { token: "~", label: "Water" },
  { token: "\"", label: "Grass" },
  { token: "u", label: "Ledge pass up" },
  { token: "d", label: "Ledge pass down" },
  { token: "l", label: "Ledge pass left" },
  { token: "r", label: "Ledge pass right" },
  { token: "#", label: "Blocked" },
  { token: ".", label: "Floor" },
  { token: "x", label: "Missing" },
  { token: "^", label: "Up" },
  { token: "v", label: "Down" },
  { token: "<", label: "Left" },
  { token: ">", label: "Right" },
];

const MAX_ASCII_LEGEND_LINE = 72;
const PASSABLE_DIRECTIONS: readonly FacingDirection[] = [
  FacingDirection.UP,
  FacingDirection.DOWN,
  FacingDirection.LEFT,
  FacingDirection.RIGHT,
];

const classifyObjectMarker = (
  object: Partial<{
    spriteConstant: string | null;
    baseSprite: string | null;
    sprite: string | null;
    event: Partial<{
      sprite: string | null;
      object_type: string | null;
      script: string | null;
      object_identifier: string | null;
      label: string | null;
    }>;
    objectId: string | null;
    constantId: string | null;
  }>
): { glyph: string; label: "person" | "berry-tree" | "item-ball" | "vendor" | "healer" } => {
  const spriteConstant = String(
    object.spriteConstant ??
      object.baseSprite ??
      object.sprite ??
      object.event?.sprite ??
      ""
  ).toUpperCase();
  const objectType = String(object.event?.object_type ?? "").toUpperCase();
  const source = [
    object.event?.script,
    object.event?.object_identifier,
    object.event?.label,
    object.objectId,
    object.constantId,
    object.spriteConstant,
    object.baseSprite,
    object.sprite,
    object.event?.sprite,
  ]
    .filter(Boolean)
    .map((value) => String(value).toUpperCase())
    .join(" ");

  if (objectType === "OBJECTTYPE_ITEMBALL" || spriteConstant === "SPRITE_POKE_BALL") {
    return { glyph: "I", label: "item-ball" };
  }
  if (spriteConstant === "SPRITE_FRUIT_TREE") {
    return { glyph: "B", label: "berry-tree" };
  }
  if (source.includes("NURSE") || source.includes("POKECENTER")) {
    return { glyph: "+", label: "healer" };
  }
  if (source.includes("MART") || source.includes("CLERK") || source.includes("SHOP")) {
    return { glyph: "V", label: "vendor" };
  }
  return { glyph: "N", label: "person" };
};

const normaliseAsciiEventToken = (value: unknown): string =>
  String(value ?? "")
    .trim()
    .replace(/[\s_]+/g, "")
    .toUpperCase();

const classifyBgEventMarker = (
  event: Partial<{ event_type: string | null; script: string | null }>
): { glyph: string; color: string } | null => {
  const eventType = normaliseAsciiEventToken(event.event_type);
  const script = normaliseAsciiEventToken(event.script);
  if (!script || eventType.includes("ITEM")) {
    return null;
  }
  if (
    script.endsWith("PC") ||
    script.includes("PCSCRIPT") ||
    script.includes("PLAYERSPC") ||
    script.includes("POKECENTERPLAYERSPC") ||
    script.includes("PCTEXT") ||
    script.includes("PCTURNON") ||
    script.includes("PCASKWHATDO")
  ) {
    return { glyph: "P", color: "1;35" };
  }
  if (script.includes("HEALINGMACHINE") || script.includes("POKECENTER")) {
    return { glyph: "+", color: "1;35" };
  }
  if (script.includes("BOOKSHELF")) {
    return { glyph: "B", color: "1;33" };
  }
  if (script.includes("TRASHCAN")) {
    return { glyph: "T", color: "1;33" };
  }
  if (script.includes("WINDOW")) {
    return { glyph: "W", color: "1;36" };
  }
  if (script.includes("TRAVELTIP")) {
    return { glyph: "T", color: "1;33" };
  }
  if (script.includes("GYM")) {
    return { glyph: "G", color: "1;35" };
  }
  if (script.includes("MART")) {
    return { glyph: "$", color: "1;33" };
  }
  if (eventType.startsWith("BGEVENT")) {
    return { glyph: "S", color: "1;33" };
  }
  return null;
};

const asciiDirectionFromMoveData = (value: unknown): string | undefined => {
  const normalized = String(value ?? "").trim().toUpperCase();
  if (!normalized) {
    return undefined;
  }
  if (normalized.includes("UP")) {
    return "up";
  }
  if (normalized.includes("DOWN")) {
    return "down";
  }
  if (normalized.includes("LEFT")) {
    return "left";
  }
  if (normalized.includes("RIGHT")) {
    return "right";
  }
  return undefined;
};

const buildAsciiLegendLines = (tokens: Set<string>): string[] => {
  const entries = ASCII_LEGEND_DEFINITIONS.filter((entry) => tokens.has(entry.token)).map(
    (entry) => `${entry.token}=${entry.label}`
  );
  if (!entries.length) {
    return [];
  }
  const lines: string[] = [];
  let current = `Legend: ${entries[0]}`;
  for (let idx = 1; idx < entries.length; idx += 1) {
    const entry = entries[idx];
    if (current.length + 1 + entry.length > MAX_ASCII_LEGEND_LINE) {
      lines.push(current);
      current = entry;
    } else {
      current = `${current} ${entry}`;
    }
  }
  lines.push(current);
  const combinedHints: string[] = [];
  if (tokens.has("D")) {
    const doorHints: string[] = [];
    if (tokens.has("P")) {
      doorHints.push("DP=Door to Pokecenter");
    }
    if (tokens.has("G")) {
      doorHints.push("DG=Door to Gym/House");
    }
    if (tokens.has("M")) {
      doorHints.push("DM=Door to Mart");
    }
    if (doorHints.length) {
      combinedHints.push(...doorHints);
    }
  }
  if (tokens.has("N")) {
    const facingHints: string[] = [];
    if (tokens.has("^")) {
      facingHints.push("N^=NPC facing up");
    }
    if (tokens.has("v")) {
      facingHints.push("Nv=NPC facing down");
    }
    if (tokens.has("<")) {
      facingHints.push("N<=NPC facing left");
    }
    if (tokens.has(">")) {
      facingHints.push("N>=NPC facing right");
    }
    if (facingHints.length) {
      combinedHints.push(...facingHints);
    }
  }
  if (combinedHints.length) {
    let hintLine = "Combined cells: ";
    for (const hint of combinedHints) {
      const candidate = `${hintLine}${hintLine.endsWith(": ") ? "" : " "} ${hint}`.replace(":  ", ": ");
      if (candidate.length > MAX_ASCII_LEGEND_LINE) {
        lines.push(hintLine.trimEnd());
        hintLine = hint;
      } else {
        hintLine = hintLine.endsWith(": ") ? `${hintLine}${hint}` : `${hintLine} ${hint}`;
      }
    }
    lines.push(hintLine.trimEnd());
  }
  return lines;
};

type FieldMoveAnimationRenderer = {
  draw?: (screen: Surface, cameraX: number, cameraY: number, origin: [number, number]) => void;
};

type DrawableOverlay = {
  draw?: (screen: Surface) => void;
};

type DialogueOverlay = {
  active?: boolean;
  current_text?: string;
  draw?: () => void;
  window?: { visible_text?: string; current_page_text?: string };
  _yes_no_prompt?: { selection: number };
  _selection_prompt?: { selection: number; lines: string[] } | null;
  waiting_for_input?: boolean;
  pending_waits?: number;
  pending_text_count?: number;
  pending_text?: string[];
};

const dialogueTextLinesForSnapshot = (dialogue: DialogueOverlay): string[] => {
  const texts: string[] = [];
  if (typeof dialogue.current_text === "string" && dialogue.current_text.trim()) {
    texts.push(dialogue.current_text);
  }
  if (Array.isArray(dialogue.pending_text)) {
    for (const text of dialogue.pending_text) {
      if (typeof text === "string" && text.trim()) {
        texts.push(text);
      }
    }
  }
  if (!texts.length) {
    const windowText = dialogue.window?.current_page_text ?? dialogue.window?.visible_text ?? "";
    if (windowText.trim()) {
      texts.push(windowText);
    }
  }
  return texts.flatMap((text) =>
    text
      .split(/\r?\n/g)
      .map((line) => line.trimEnd())
      .filter((line) => line.trim().length > 0)
  );
};

type EmoteSpriteCache = {
  get_surface: (emoteId: string) => Surface;
};

type MapWarpEvent = {
  tile_position?: (stride: number) => [number, number];
  x: number;
  y: number;
  index?: number;
  target_map?: string;
  target_warp_id?: number;
};

type MapCoordEvent = {
  x: number;
  y: number;
  scene_id?: string;
  script_name?: string;
};

type MapBgEvent = {
  x: number;
  y: number;
  event_type?: string;
  script?: string;
};

type MapEvents = {
  warps?: MapWarpEvent[];
  coord_events?: MapCoordEvent[];
  bg_events?: MapBgEvent[];
};

type CameraRect = {
  cameraX: number;
  cameraY: number;
  visibleWidth: number;
  visibleHeight: number;
};

function computeCameraRect(
  mapWidthPx: number,
  mapHeightPx: number,
  viewportWidthPx: number,
  viewportHeightPx: number,
  playerCompX: number,
  playerCompY: number
): CameraRect {
  const visibleWidth = Math.min(viewportWidthPx, mapWidthPx);
  const visibleHeight = Math.min(viewportHeightPx, mapHeightPx);
  const maxX = Math.max(0, mapWidthPx - visibleWidth);
  const maxY = Math.max(0, mapHeightPx - visibleHeight);
  const halfWidth = Math.floor(visibleWidth / 2);
  const halfHeight = Math.floor(visibleHeight / 2);
  const cameraX = Math.min(Math.max(playerCompX - halfWidth, 0), maxX);
  const cameraY = Math.min(Math.max(playerCompY - halfHeight, 0), maxY);
  return { cameraX, cameraY, visibleWidth, visibleHeight };
}

type SurfaceWithAlpha = Surface & {
  set_alpha?: (alpha: number) => void;
};

type DrawLayout = [number, number, number, number];
type RenderEntry = [
  number,
  number,
  boolean,
  OverworldObject | PlayerCharacter | null,
  Surface,
  number,
  number,
];
type EmoteLayerEntry = [number, number, number, Surface, number, number];
type GrassRenderableEntry = [number, number, Surface, [number, number]];

const EMPTY_DRAW_LAYOUTS = new Map<GrassRustleTarget, DrawLayout>();

type OverworldRenderingState = {
  _ledge_jump_active?: boolean;
  _ledge_jump_animation_progress_px?: number;
  _ledge_jump_total_distance_px?: number;
  _ledge_shadow_surface?: Surface | null;
  game_state?: GameState | null;
  ui?: unknown;
  _suppress_text_snapshot?: boolean;
};

const getRenderingState = (instance: OverworldBase): OverworldRenderingState =>
  instance as OverworldRenderingState;

type OverworldRenderingInternals = OverworldRenderingState &
  Partial<{
    _composite_surface: Surface | null;
    map_surface: Surface | null;
    _composite_origin: [number, number];
    _composite_segments: CompositeSegment[];
    player_object: OverworldObject | PlayerCharacter | null;
    player_px_x: number;
    target_px_x: number;
    player_px_y: number;
    target_px_y: number;
    player_animations: Record<string, { currentFrame: Surface }>;
    player_direction: string;
    player_x: number;
    player_y: number;
    _composite_priority_surface: Surface | null;
    priority_surface: Surface | null;
    _npc_pixel_position: (npc: OverworldObject) => [number, number];
    npcs: OverworldObject[];
    current_map_name: string;
    _field_move_animation_renderer: FieldMoveAnimationRenderer | null;
    _poison_overlay_alpha: number;
    _poison_overlay: Surface | null;
    _map_sign: DrawableOverlay | null;
    pokepic_overlay: DrawableOverlay | null;
    dialogue: DialogueOverlay | null;
    _town_map_overlay: TownMapOverlayLike | null;
    _egg_hatch_animation: DrawableOverlay | null;
    _fade_alpha: number;
    _fade_overlay: Surface | null;
    _fade_overlay_color: [number, number, number];
    _fade_start_alpha: number;
    _fade_end_alpha: number;
    _fade_steps_total: number;
    _fade_progress: number;
    _fade_active: boolean;
    _white_fade_pending_clear: boolean;
    _poison_flash_remaining: number;
    _earthquake_intensity: number;
    _earthquake_phase: number;
    _earthquake_remaining_frames: number;
    _grass_rustle: GrassRustleController | null;
    _active_emotes: Map<OverworldObject, [string, number]>;
    _emote_sprite_cache: EmoteSpriteCache | null;
    _phone_call_overlay: DrawableOverlay | null;
    _maybe_hide_phone_overlay: (() => void) | null;
    _ascii_overlay_cache_key: string | null;
    _ascii_overlay_cached_viewport: string[] | null;
    _ascii_overlay_cached_info: string[] | null;
    _ascii_overlay_last_npc_positions: number[][];
    _ascii_overlay_last_event_identity: MapEvents | null;
    _ascii_overlay_last_event_counts: [number, number, number] | null;
    _draw_layouts_scratch: Map<GrassRustleTarget, DrawLayout>;
    _render_list_scratch: RenderEntry[];
    _emote_layers_scratch: EmoteLayerEntry[];
    _grass_renderables_scratch: GrassRenderableEntry[];
    _src_rect_scratch: InstanceType<typeof gameEngine.Rect> | null;
    _last_block_feedback: Record<string, unknown> | null;
    _map_events: MapEvents | null;
    map: OverworldMap | null;
    tileset: OverworldTilesetLike | null;
    player_state: PlayerState;
    _debug_sightlines: boolean;
    _npc_is_trainer: ((npc: OverworldObject) => boolean) | null;
    TILES_PER_COLLISION: number;
    _text_ui_color: string | boolean;
    _multiplayer_remote_players: RemoteOverworldPlayer[];
    _multiplayer_remote_render_enabled: boolean;
    _multiplayer_remote_crowd_view: boolean;
    _multiplayer_marker_player: Surface | null;
    _multiplayer_marker_ai: Surface | null;
  }>;

const getRenderingInternals = (instance: OverworldBase): OverworldRenderingInternals =>
  instance as OverworldRenderingInternals;

const POISON_OVERLAY_COLOR: [number, number, number, number] = [230, 173, 255, 255];
const POISON_OVERLAY_ALPHA = 176;
const POISON_FLASH_DURATION = 4;

const GRASS_COLLISION_TOKENS = [
  "CUT_08",
  "TALL_GRASS",
  "TALL_GRASS_10",
  "LONG_GRASS",
  "LONG_GRASS_1C",
  "CUT_28",
  "GRASS_48",
  "GRASS_49",
  "GRASS_4A",
  "GRASS_4B",
  "GRASS_4C",
];
const GRASS_COLLISION_VALUES = new Set(
  GRASS_COLLISION_TOKENS.map((token) => resolveCollisionValue(token))
);
const EMOTE_Z_ORDER: Record<string, number> = { EMOTE_SHOCK: 1 };

const findTextUi = (ui: unknown): TextTarget | null => {
  if (!ui) {
    return null;
  }
  if (ui instanceof CompositeUI) {
    for (const child of ui.getChildren!()) {
      const found = findTextUi(child);
      if (found) {
        return found;
      }
    }
    return null;
  }
  if (isTextTarget(ui)) {
    return ui;
  }
  if (hasChildren(ui)) {
    for (const child of ui.getChildren()) {
      const found = findTextUi(child);
      if (found) {
        return found;
      }
    }
  }
  return null;
};

const buildCrowdMarkerPositions = (
  count: number,
  viewportWidth: number,
  viewportHeight: number,
  markerSize: number
): Array<[number, number]> => {
  const safeCount = Math.max(0, Math.trunc(count));
  const safeMarker = Math.max(1, Math.trunc(markerSize));
  const columns = Math.max(1, Math.floor(viewportWidth / safeMarker));
  const rows = Math.max(1, Math.floor(viewportHeight / safeMarker));
  const capacity = columns * rows;
  const limit = Math.min(safeCount, capacity);
  const positions: Array<[number, number]> = [];
  for (let index = 0; index < limit; index += 1) {
    const x = (index % columns) * safeMarker;
    const y = Math.floor(index / columns) * safeMarker;
    positions.push([x, y]);
  }
  return positions;
};

const blitSurfaceAt = (
  target: Pick<Surface, "blit"> & Partial<Pick<Surface, "blitAt">>,
  source: Surface,
  destX: number,
  destY: number,
  area?: { x: number; y: number; width: number; height: number }
): void => {
  if (typeof target.blitAt === "function") {
    target.blitAt(source, destX, destY, area);
    return;
  }
  target.blit(source, [destX, destY], area);
};

const writeRenderEntry = (
  entries: RenderEntry[],
  index: number,
  tileY: number,
  tileX: number,
  isPlayer: boolean,
  object: OverworldObject | PlayerCharacter | null,
  sprite: Surface,
  screenX: number,
  screenY: number
): number => {
  const existing = entries[index];
  if (existing) {
    existing[0] = tileY;
    existing[1] = tileX;
    existing[2] = isPlayer;
    existing[3] = object;
    existing[4] = sprite;
    existing[5] = screenX;
    existing[6] = screenY;
  } else {
    entries[index] = [tileY, tileX, isPlayer, object, sprite, screenX, screenY];
  }
  return index + 1;
};

const renderEntryObjectIndex = (entry: RenderEntry): number => {
  if (entry[2]) {
    return Number.MAX_SAFE_INTEGER;
  }
  const object = entry[3] as Partial<{ objectIndex: number }> | null;
  return typeof object?.objectIndex === "number" ? object.objectIndex : 0;
};

const rendersAbovePriorityPlane = (
  isPlayer: boolean,
  object: OverworldObject | PlayerCharacter | null
): boolean => {
  if (isPlayer || !object) {
    return false;
  }
  if (classifyObjectMarker(object).label !== "item-ball") {
    return false;
  }
  return !Boolean((object as Partial<{ overhead: boolean }>).overhead);
};

const npcAnchorWithinMapBounds = (
  npc: OverworldObject,
  map: OverworldMap | null | undefined
): boolean => {
  if (!map) {
    return true;
  }
  const maxTileX = Math.max(0, Math.trunc(map.width) * METATILE_WIDTH);
  const maxTileY = Math.max(0, Math.trunc(map.height) * METATILE_WIDTH);
  const x = Math.trunc(Number(npc.x));
  const y = Math.trunc(Number(npc.y));
  return (
    Number.isFinite(x) &&
    Number.isFinite(y) &&
    x >= 0 &&
    y >= 0 &&
    x < maxTileX &&
    y < maxTileY
  );
};

const writeEmoteLayerEntry = (
  entries: EmoteLayerEntry[],
  index: number,
  zIndex: number,
  spriteY: number,
  spriteX: number,
  surface: Surface,
  destX: number,
  destY: number
): number => {
  const existing = entries[index];
  if (existing) {
    existing[0] = zIndex;
    existing[1] = spriteY;
    existing[2] = spriteX;
    existing[3] = surface;
    existing[4] = destX;
    existing[5] = destY;
  } else {
    entries[index] = [zIndex, spriteY, spriteX, surface, destX, destY];
  }
  return index + 1;
};

// Exported for regression tests only.
export const __test__findTextUi = findTextUi;
export const __test__formatAxisHeader = formatAxisHeader;
export const __test__formatRowLabel = formatRowLabel;
export const __test__computeCameraRect = computeCameraRect;
export const __test__buildCrowdMarkerPositions = buildCrowdMarkerPositions;

const isPromiseLike = (value: unknown): value is Promise<unknown> =>
  Boolean(value) && typeof (value as Promise<unknown>).then === "function";

const drawPolyline = (
  screen: Surface,
  color: [number, number, number],
  points: Array<[number, number]>,
  width: number
): void => {
  if (points.length < 2) {
    return;
  }
  for (let idx = 0; idx < points.length - 1; idx += 1) {
    gameEngine.draw.line(screen, color, points[idx], points[idx + 1], width);
  }
};

export class OverworldRenderingMixin extends OverworldBase {
  public screen!: Surface | null;
  protected _fade_overlay!: Surface;
  protected _earthquake_offset: [number, number] = [0, 0];

  protected _tile_to_pixels(tileCoordinate: number): number {
    return OverworldBase._tileToPixels(tileCoordinate);
  }

  protected _player_jump_y_offset(): number {
    const state = getRenderingState(this);
    if (!state._ledge_jump_active) {
      return 0;
    }
    return getLedgeJumpOffset(
      state._ledge_jump_animation_progress_px ?? 0,
      state._ledge_jump_total_distance_px ?? 0
    );
  }

  protected _ledge_shadow_asset_path(): string {
    return getAssetPath("gfx", "overworld", "shadow.png");
  }

  protected async _preload_ledge_shadow_surface(): Promise<void> {
    const shadowPath = this._ledge_shadow_asset_path();
    const preload = gameEngine.image.preload;
    if (typeof preload === "function") {
      await preload(shadowPath);
      return;
    }
    const surface = gameEngine.image.load(shadowPath);
    if (isPromiseLike(surface)) {
      await surface;
    }
  }

  protected _load_ledge_shadow_surface(): Surface {
    const shadowPath = this._ledge_shadow_asset_path();
    const loadSync = gameEngine.image.loadSync;
    const surface =
      typeof loadSync === "function" ? loadSync(shadowPath) : gameEngine.image.load(shadowPath);
    if (!surface || isPromiseLike(surface)) {
      // ASM uses FACING_SHADOW OAM data directly; the shadow graphic is not optional.
      throw new Error(
        `Ledge shadow sprite must be preloaded before overworld rendering: ${shadowPath}`
      );
    }
    let baseShadow = this._transparent_white_surface(surface as Surface);
    baseShadow = gameEngine.transform.flip(baseShadow, true, true);
    const [width, height] = baseShadow.get_size();
    const overlap = Math.floor(width / 2);
    const combinedShadow = new gameEngine.Surface(width + overlap, height);
    const flipped = gameEngine.transform.flip(baseShadow, true, false);
    combinedShadow.blit(flipped, [0, 0]);
    combinedShadow.blit(baseShadow, [overlap, 0]);
    return combinedShadow;
  }

  protected _load_misc_sprite_assets(): void {
    const state = getRenderingState(this);
    const gameState = state.game_state;
    if (!gameState) {
      return;
    }
    const spriteFlags = gameState.wram?.wSpriteFlags ?? 0;
    if (spriteFlags & SPRITES_SKIP_WALKING) {
      return;
    }
    state._ledge_shadow_surface = this._load_ledge_shadow_surface();
  }

  protected _transparent_white_surface(source: Surface): Surface {
    const width = source.get_width();
    const height = source.get_height();
    const srcData = source.getImageData();
    const data = srcData.data;
    const tinted = new gameEngine.Surface(width, height);
    const ctx = tinted.getContext();
    if (!ctx) {
      throw new Error("Failed to get canvas context for transparency pass.");
    }
    const image = ctx.createImageData(width, height);
    if (!image || !image.data) {
      return tinted;
    }
    const out = image.data;
    for (let idx = 0; idx < data.length; idx += 4) {
      const r = data[idx];
      const g = data[idx + 1];
      const b = data[idx + 2];
      const isWhite = r > 240 && g > 240 && b > 240;
      out[idx] = isWhite ? 0 : r;
      out[idx + 1] = isWhite ? 0 : g;
      out[idx + 2] = isWhite ? 0 : b;
      out[idx + 3] = isWhite ? 0 : 255;
    }
    ctx.putImageData(image, 0, 0);
    return tinted;
  }

  private _blit_with_alpha(surface: Surface, alpha: number): void {
    if (!this.screen) {
      return;
    }
    const ctx = this.screen.getContext();
    if (!ctx) {
      throw new Error("Failed to get canvas context for alpha blit.");
    }
    const clamped = Math.max(0, Math.min(255, Math.trunc(alpha)));
    ctx.save();
    ctx.globalAlpha = clamped / 255;
    const canvas = surface.getCanvasImageSource();
    if (canvas) {
      ctx.drawImage(canvas, 0, 0);
    }
    ctx.restore();
  }

  protected _blit_ledge_shadow(baseX: number, baseY: number, sprite: Surface): void {
    const shadow = getRenderingState(this)._ledge_shadow_surface;
    if (!shadow || !this.screen) {
      return;
    }
    const [spriteW, spriteH] = sprite.get_size();
    const [shadowW, shadowH] = shadow.get_size();
    const shadowX = baseX + Math.floor((spriteW - shadowW) / 2);
    const shadowY = baseY + spriteH - Math.floor(shadowH / 2);
    blitSurfaceAt(this.screen, shadow, shadowX, shadowY);
  }

  public draw(): void {
    const state = getRenderingState(this);
    const textTarget = findTextUi(state.ui);
    if (textTarget && !state._suppress_text_snapshot) {
      this._draw_ascii_overworld(textTarget);
      if (textTarget === state.ui) {
        return;
      }
    }
    const flareStart = beginFlarePlotFrame();

    if (!this.screen) {
      throw new Error("No overworld screen available for rendering.");
    }

    const internals = getRenderingInternals(this);
    const surface = internals._composite_surface ?? internals.map_surface;
    if (!surface) {
      const tileset = internals.tileset;
      if (tileset?.ready && !tileset.loaded) {
        this.screen.fill([0, 0, 0, 255]);
        return;
      }
      throw new Error("No overworld surface available for rendering.");
    }

    const viewportWidthPx = this.screen.get_width();
    const viewportHeightPx = this.screen.get_height();

    const [originX, originY] = internals._composite_origin ?? [0, 0];
    const playerSpriteOffset = Math.round(
      (internals.player_object as OverworldObject | PlayerCharacter | null)?.spriteYOffset
        ?? (internals.player_object as any)?.sprite_y_offset
        ?? 0
    );
    const playerIsJumping = Boolean((internals.player_object as OverworldObject | PlayerCharacter | null)?.jumping);
    const playerPxX = internals.player_px_x ?? internals.target_px_x ?? 0;
    const playerPxY = internals.player_px_y ?? internals.target_px_y ?? 0;
    const playerCompX = playerPxX + originX;
    const playerCompY = playerPxY + originY + playerSpriteOffset;

    const [mapWidthPx, mapHeightPx] = surface.get_size();
    const cameraRect = computeCameraRect(
      mapWidthPx,
      mapHeightPx,
      viewportWidthPx,
      viewportHeightPx,
      playerCompX,
      playerCompY
    );
    let cameraX = cameraRect.cameraX;
    let cameraY = cameraRect.cameraY;
    const maxX = Math.max(0, mapWidthPx - cameraRect.visibleWidth);
    const maxY = Math.max(0, mapHeightPx - cameraRect.visibleHeight);
    [cameraX, cameraY] = this._apply_earthquake_offset(cameraX, cameraY, maxX, maxY);

    const srcRect =
      internals._src_rect_scratch ?? (internals._src_rect_scratch = new gameEngine.Rect(0, 0, 0, 0));
    srcRect.x = cameraX;
    srcRect.y = cameraY;
    srcRect.width = cameraRect.visibleWidth;
    srcRect.height = cameraRect.visibleHeight;
    if (cameraRect.visibleWidth !== viewportWidthPx || cameraRect.visibleHeight !== viewportHeightPx) {
      this.screen.fill([0, 0, 0, 255]);
    }
    blitSurfaceAt(this.screen, surface, 0, 0, srcRect);
    this._draw_debug_sightlines(cameraX, cameraY);

    let prioritySurface: Surface | null = null;
    if (surface === internals._composite_surface && internals._composite_priority_surface) {
      prioritySurface = internals._composite_priority_surface;
    } else if (surface === internals.map_surface && internals.priority_surface) {
      prioritySurface = internals.priority_surface;
    }

    const needsDrawLayouts =
      Boolean(internals._grass_rustle) || Boolean(internals._active_emotes?.size);
    const drawLayouts = needsDrawLayouts
      ? (
          internals._draw_layouts_scratch ??
          (internals._draw_layouts_scratch = new Map<GrassRustleTarget, DrawLayout>())
        )
      : EMPTY_DRAW_LAYOUTS;
    if (drawLayouts !== EMPTY_DRAW_LAYOUTS) {
      drawLayouts.clear();
    }

    const playerAnimation =
      internals.player_animations?.[internals.player_direction ?? ""] ?? null;
    if (!playerAnimation) {
      throw new Error("Player animations are missing; cannot render overworld.");
    }
    const playerSprite = playerAnimation.currentFrame as Surface;
    const playerBaseScreenX = playerCompX - cameraX;
    const playerBaseScreenY = playerCompY - cameraY;
    const playerGroundScreenY = playerBaseScreenY - playerSpriteOffset;
    const jumpOffset = this._player_jump_y_offset();
    const playerScreenX = playerBaseScreenX;
    const playerScreenY = playerBaseScreenY + jumpOffset;

    if (drawLayouts !== EMPTY_DRAW_LAYOUTS && internals.player_object != null) {
      drawLayouts.set(internals.player_object, [
        playerScreenX,
        playerScreenY,
        playerSprite.get_width(),
        playerSprite.get_height(),
      ]);
    }

    const renderList = internals._render_list_scratch ?? (internals._render_list_scratch = []);
    let renderCount = 0;
    renderCount = writeRenderEntry(
      renderList,
      renderCount,
      internals.player_y ?? 0,
      internals.player_x ?? 0,
      true,
      internals.player_object ?? null,
      playerSprite,
      playerScreenX,
      playerScreenY
    );

    const shouldRenderRemote = internals._multiplayer_remote_render_enabled ?? true;
    const crowdView = internals._multiplayer_remote_crowd_view ?? false;
    const remotePlayers = internals._multiplayer_remote_players ?? [];
    if (shouldRenderRemote) {
      if (crowdView) {
        const markerSize = 2;
        const columns = Math.max(1, Math.floor(viewportWidthPx / markerSize));
        const rows = Math.max(1, Math.floor(viewportHeightPx / markerSize));
        const limit = Math.min(remotePlayers.length, columns * rows);
        if (!internals._multiplayer_marker_player) {
          const marker = new gameEngine.Surface(markerSize, markerSize);
          marker.fill([37, 182, 255, 255]);
          internals._multiplayer_marker_player = marker;
        }
        if (!internals._multiplayer_marker_ai) {
          const marker = new gameEngine.Surface(markerSize, markerSize);
          marker.fill([255, 111, 0, 255]);
          internals._multiplayer_marker_ai = marker;
        }
        for (let index = 0; index < limit; index += 1) {
          const remotePlayer = remotePlayers[index];
          const marker =
            remotePlayer?.entityType === "ai"
              ? internals._multiplayer_marker_ai
              : internals._multiplayer_marker_player;
          if (!marker) {
            continue;
          }
          const markerX = (index % columns) * markerSize;
          const markerY = Math.floor(index / columns) * markerSize;
          blitSurfaceAt(this.screen, marker, markerX, markerY);
        }
      } else {
        const stride = Math.max(1, Math.trunc(internals.TILES_PER_COLLISION ?? 2));
        const footprint = stride - 1;
        for (const remotePlayer of remotePlayers) {
          if (remotePlayer.mapName !== (internals.current_map_name ?? "")) {
            continue;
          }
          const animation =
            internals.player_animations?.[remotePlayer.direction] ??
            internals.player_animations?.down ??
            null;
          if (!animation) {
            continue;
          }
          const sprite = animation.currentFrame as Surface;
          const spriteW = sprite.get_width();
          const spriteH = sprite.get_height();
          const remoteCompX = (remotePlayer.tileX - footprint) * TILE_SIZE + originX;
          const remoteCompY = (remotePlayer.tileY - footprint) * TILE_SIZE + originY + playerSpriteOffset;

          if (remoteCompX + spriteW <= cameraX || remoteCompX >= cameraX + viewportWidthPx) {
            continue;
          }
          if (remoteCompY + spriteH <= cameraY || remoteCompY >= cameraY + viewportHeightPx) {
            continue;
          }

          const screenX = remoteCompX - cameraX;
          const screenY = remoteCompY - cameraY;
          renderCount = writeRenderEntry(
            renderList,
            renderCount,
            remotePlayer.tileY,
            remotePlayer.tileX,
            false,
            null,
            sprite,
            screenX,
            screenY
          );
        }
      }
    }

    for (const npc of internals.npcs ?? []) {
      if (!npcAnchorWithinMapBounds(npc, internals.map)) {
        continue;
      }
      if (!npc.animations) {
        throw new Error(
          `NPC '${npc.spriteId}' on map '${internals.current_map_name ?? ""}' has no animation data.`
        );
      }
      let animation = npc.animations[npc.direction];
      if (!animation) {
        animation = npc.animations.down;
      }
      if (!animation) {
        throw new Error(
          `NPC '${npc.spriteId}' lacks a usable animation for direction '${npc.direction}'.`
        );
      }
      const frame = animation.currentFrame;
      const spriteW = frame.get_width();
      const spriteH = frame.get_height();
      const [npcPxX, npcPxY] = internals._npc_pixel_position?.(npc) ?? [0, 0];
      const npcCompX = npcPxX + originX;
      const npcCompY = npcPxY + originY;

      if (npcCompX + spriteW <= cameraX || npcCompX >= cameraX + viewportWidthPx) {
        continue;
      }
      if (npcCompY + spriteH <= cameraY || npcCompY >= cameraY + viewportHeightPx) {
        continue;
      }

      const screenX = npcCompX - cameraX;
      const screenY = npcCompY - cameraY;
      const [offsetX, offsetY] = OverworldBase._npcDrawOffsets(spriteW, spriteH);
      const finalX = screenX + offsetX;
      const finalY = screenY + offsetY;

      if (drawLayouts !== EMPTY_DRAW_LAYOUTS) {
        drawLayouts.set(npc, [finalX, finalY, spriteW, spriteH]);
      }
      renderCount = writeRenderEntry(
        renderList,
        renderCount,
        npc.y,
        npc.x,
        false,
        npc,
        frame,
        finalX,
        finalY
      );
    }

    renderList.length = renderCount;

    if (renderCount > 1) {
      renderList.sort((a, b) => {
        if (a[0] !== b[0]) {
          return a[0] - b[0];
        }
        if (a[1] !== b[1]) {
          return a[1] - b[1];
        }
        // ASM OBJ priority favors lower object slots. Draw higher slots first so lower slots land on top.
        return renderEntryObjectIndex(b) - renderEntryObjectIndex(a);
      });
    }

    for (let index = 0; index < renderCount; index += 1) {
      const [, , isPlayer, object, sprite, sx, sy] = renderList[index]!;
      if (prioritySurface && rendersAbovePriorityPlane(isPlayer, object)) {
        continue;
      }
      if (isPlayer && (internals._ledge_jump_active || playerIsJumping)) {
        this._blit_ledge_shadow(playerBaseScreenX, playerGroundScreenY, playerSprite);
      }
      blitSurfaceAt(this.screen, sprite, sx, sy);
    }

    if (prioritySurface) {
      blitSurfaceAt(this.screen, prioritySurface, 0, 0, srcRect);
      for (let index = 0; index < renderCount; index += 1) {
        const [, , isPlayer, object, sprite, sx, sy] = renderList[index]!;
        if (rendersAbovePriorityPlane(isPlayer, object)) {
          blitSurfaceAt(this.screen, sprite, sx, sy);
        }
      }
    }

    this._draw_grass_rustle(drawLayouts);

    const controller = internals._field_move_animation_renderer;
    if (controller?.draw) {
      controller.draw(this.screen, cameraX, cameraY, internals._composite_origin ?? [0, 0]);
    }

    this._draw_emotes(drawLayouts);

    if (internals._poison_overlay_alpha) {
      const overlay = internals._poison_overlay;
      if (overlay) {
        const alphaSurface = overlay as SurfaceWithAlpha;
        if (typeof alphaSurface.set_alpha === "function") {
          alphaSurface.set_alpha(internals._poison_overlay_alpha);
          blitSurfaceAt(this.screen, overlay as Surface, 0, 0);
        } else {
          this._blit_with_alpha(overlay as Surface, internals._poison_overlay_alpha);
        }
      }
    }

    internals._map_sign?.draw?.(this.screen);
    internals.pokepic_overlay?.draw?.(this.screen);
    this._draw_phone_call_overlay();
    internals.dialogue?.draw?.();
    if (internals._town_map_overlay?.visible) {
      internals._town_map_overlay.drawToGameEngine?.(this.screen);
    }
    internals._egg_hatch_animation?.draw?.(this.screen);
    if (internals._fade_alpha) {
      const overlay = internals._fade_overlay;
      if (overlay != null) {
        const alphaSurface = overlay as SurfaceWithAlpha;
        if (typeof alphaSurface.set_alpha === "function") {
          alphaSurface.set_alpha(internals._fade_alpha);
          blitSurfaceAt(this.screen, overlay as Surface, 0, 0);
        } else {
          this._blit_with_alpha(overlay as Surface, internals._fade_alpha);
        }
      }
    }
    finishFlarePlotFrame(flareStart, "overworld", this.screen, 0, 0, 100, 30);
  }

  protected _draw_grass_rustle(drawLayouts: Map<GrassRustleTarget, DrawLayout>): void {
    const internals = getRenderingInternals(this);
    const controller = internals._grass_rustle;
    if (!controller || !this.screen) {
      return;
    }
    const layers =
      controller.renderables(
        drawLayouts,
        internals._grass_renderables_scratch ?? (internals._grass_renderables_scratch = [])
      );
    if (!layers?.length) {
      return;
    }
    for (const layer of layers) {
      const surface = layer[2];
      const dest = layer[3];
      blitSurfaceAt(this.screen, surface, dest[0], dest[1]);
    }
  }

  protected _draw_emotes(drawLayouts: Map<GrassRustleTarget, DrawLayout>): void {
    const internals = getRenderingInternals(this);
    if (!this.screen || !internals._active_emotes?.size || !drawLayouts.size) {
      return;
    }
    const emoteLayers =
      internals._emote_layers_scratch ??
      (internals._emote_layers_scratch = []);
    let emoteLayerCount = 0;
    const emoteCache = internals._emote_sprite_cache;
    if (!emoteCache) {
      return;
    }
    for (const [obj, [emoteId]] of internals._active_emotes.entries()) {
      const layout = drawLayouts.get(obj);
      if (!layout) {
        continue;
      }
      const [spriteX, spriteY, spriteW, spriteH] = layout;
      const emoteSurface = emoteCache.get_surface(emoteId);
      const destX = spriteX + Math.floor((spriteW - emoteSurface.get_width()) / 2);
      const destY = spriteY - emoteSurface.get_height();
      const zIndex = EMOTE_Z_ORDER[emoteId] ?? 0;
      emoteLayerCount = writeEmoteLayerEntry(
        emoteLayers,
        emoteLayerCount,
        zIndex,
        spriteY,
        spriteX,
        emoteSurface,
        destX,
        destY
      );
    }
    emoteLayers.length = emoteLayerCount;
    if (emoteLayerCount > 1) {
      emoteLayers.sort((a, b) => {
        if (a[0] !== b[0]) {
          return a[0] - b[0];
        }
        if (a[1] !== b[1]) {
          return a[1] - b[1];
        }
        return a[2] - b[2];
      });
    }
    for (let index = 0; index < emoteLayerCount; index += 1) {
      const layer = emoteLayers[index]!;
      blitSurfaceAt(this.screen, layer[3], layer[4], layer[5]);
    }
  }

  protected _draw_phone_call_overlay(): void {
    const internals = getRenderingInternals(this);
    const overlay = internals._phone_call_overlay;
    if (!overlay || !this.screen) {
      return;
    }
    internals._maybe_hide_phone_overlay?.();
    if (overlay && typeof (overlay as { draw?: (screen: Surface) => void }).draw === 'function') {
      (overlay as { draw: (screen: Surface) => void }).draw(this.screen);
    }
  }

  protected _invalidate_ascii_overlay_cache(): void {
    const internals = getRenderingInternals(this);
    internals._ascii_overlay_cache_key = null;
    internals._ascii_overlay_cached_viewport = null;
    internals._ascii_overlay_cached_info = null;
  }

  protected _draw_ascii_overworld(text_ui: TextTarget): void {
    const internals = getRenderingInternals(this);
    const state = getRenderingState(this);
    const mapData = internals.map;
    const tileset = internals.tileset;
    const mapEvents = internals._map_events;
    if (!mapData || !tileset || !mapEvents) {
      return;
    }

    const wramCoords = state.game_state?.wram as Partial<{
      wXCoord: number;
      wYCoord: number;
      player_x: number;
      player_y: number;
    }> | null | undefined;
    const readFiniteCoord = (...values: unknown[]): number => {
      for (const value of values) {
        if (typeof value === "number" && Number.isFinite(value) && value >= 0) {
          return value;
        }
      }
      return 0;
    };
    const playerX = readFiniteCoord(wramCoords?.wXCoord, internals.player_x);
    const playerY = readFiniteCoord(wramCoords?.wYCoord, internals.player_y);

    const useColor = Boolean(internals._text_ui_color ?? true);
    const colorize = (char: string, color?: string): string => {
      if (!color || !useColor) {
        return char;
      }
      return `\u001b[${color}m${char}\u001b[0m`;
    };

    const collisionStride = Math.max(1, Math.trunc(internals.TILES_PER_COLLISION ?? 1));
    const warpStride = collisionStride;
    const allWarps = mapEvents.warps ?? [];
    const coordEvents = mapEvents.coord_events ?? [];
    const bgEvents = mapEvents.bg_events ?? [];
    const dataLoader = (this as {
      data_loader?: {
        get_script_event_flags?: (script: string) => string[];
        get_hidden_item_event_flag?: (script: string) => string | null;
      };
      script_runner?: { _ensure_map_scene_initialized?: (mapName: string) => [string] | null };
    }).data_loader ?? null;
    const wram = state.game_state?.wram ?? null;
    let currentScene = String(wram?.scene_name ?? "");
    if (!currentScene && wram) {
      const runner = (this as { script_runner?: { _ensure_map_scene_initialized?: (mapName: string) => [string] | null } })
        .script_runner ?? null;
      if (runner?._ensure_map_scene_initialized) {
        const initialized = runner._ensure_map_scene_initialized(internals.current_map_name ?? "");
        if (initialized) {
          const [sceneName] = initialized;
          if (sceneName) {
            wram.scene_name = sceneName;
            currentScene = String(sceneName);
          }
        }
      }
    }
    const eventFlags = wram?.event_flags ?? null;
    const shouldRenderCoordEvent = (event: MapCoordEvent): boolean => {
      if (!wram) {
        return true;
      }
      const sceneId = String(event.scene_id ?? "").trim();
      const scriptName = String(event.script_name ?? "").trim();
      if (sceneId && sceneId !== currentScene) {
        return false;
      }
      if (!sceneId && dataLoader?.get_script_event_flags && scriptName && eventFlags) {
        const flags = dataLoader.get_script_event_flags(scriptName);
        if (flags && flags.some((flag) => eventFlags[flag])) {
          return false;
        }
      }
      return true;
    };
    const activeCoordEvents = coordEvents.filter(shouldRenderCoordEvent);
    const activeBgEvents = [...bgEvents];
    const activeEventSignature = [
      activeCoordEvents.map((event) => [event.x, event.y, event.scene_id ?? "", event.script_name ?? ""]),
      activeBgEvents.map((event) => [event.x, event.y, event.event_type ?? "", event.script ?? ""]),
    ];
    const eventCounts: [number, number, number] = [
      allWarps.length,
      activeCoordEvents.length,
      activeBgEvents.length,
    ];

    const compositeOrigin = internals._composite_origin ?? [0, 0];
    const compositeSegments = internals._composite_segments ?? [];
    const compositeSignature = compositeSegments.map((segment) => [
      segment.name,
      segment.dest[0],
      segment.dest[1],
      segment.map.width,
      segment.map.height,
      segment.tileset?.tilesetName ?? "",
    ]);

    type SegmentCollisionSource = {
      offsetTileX: number;
      offsetTileY: number;
      widthTiles: number;
      heightTiles: number;
      map: OverworldMap;
      tileset: OverworldTilesetLike;
    };

    const toTileOffset = (value: number): number => Math.floor(value / TILE_SIZE);
    const segmentSources: SegmentCollisionSource[] = compositeSegments.map((segment) => {
      const offsetPxX = segment.dest[0] - compositeOrigin[0];
      const offsetPxY = segment.dest[1] - compositeOrigin[1];
      return {
        offsetTileX: toTileOffset(offsetPxX),
        offsetTileY: toTileOffset(offsetPxY),
        widthTiles: segment.map.width * METATILE_WIDTH,
        heightTiles: segment.map.height * METATILE_WIDTH,
        map: segment.map,
        tileset: segment.tileset,
      };
    });
    const playerTileX = unscaleTileCoord(playerX, collisionStride);
    const playerTileY = unscaleTileCoord(playerY, collisionStride);
    const baseMapWidthTiles = Math.ceil((mapData.width * METATILE_WIDTH) / collisionStride);
    const baseMapHeightTiles = Math.ceil((mapData.height * METATILE_WIDTH) / collisionStride);
    const mapWidthTiles = Math.max(
      baseMapWidthTiles,
      playerTileX + 1,
      ...segmentSources.map((segment) => segment.offsetTileX + Math.ceil(segment.widthTiles / collisionStride))
    );
    const mapHeightTiles = Math.max(
      baseMapHeightTiles,
      playerTileY + 1,
      ...segmentSources.map((segment) => segment.offsetTileY + Math.ceil(segment.heightTiles / collisionStride))
    );
    const viewportWidth = Math.min(ASCII_VIEWPORT_WIDTH, mapWidthTiles);
    const viewportHeight = Math.min(ASCII_VIEWPORT_HEIGHT, mapHeightTiles);
    const maxCollisionTileX = mapWidthTiles * collisionStride - 1;
    const maxCollisionTileY = mapHeightTiles * collisionStride - 1;

    const resolveCompositeCollision = (tileX: number, tileY: number): CollisionSample | null => {
      for (const segment of segmentSources) {
        if (
          tileX >= segment.offsetTileX &&
          tileX < segment.offsetTileX + segment.widthTiles &&
          tileY >= segment.offsetTileY &&
          tileY < segment.offsetTileY + segment.heightTiles
        ) {
          try {
            return sampleCollision(
              segment.map,
              segment.tileset,
              tileX - segment.offsetTileX,
              tileY - segment.offsetTileY
            );
          } catch {
            return null;
          }
        }
      }
      return null;
    };

    const npcSignature = (internals.npcs ?? []).map((npc, idx): [number, number, number, number, number, number] => {
      const npcStride = Math.max(1, Math.trunc((npc as Partial<{ collisionStride: number }>).collisionStride ?? 1));
      return [
        npc.objectIndex ?? idx + 1,
        unscaleTileCoord(npc.x ?? 0, npcStride),
        unscaleTileCoord(npc.y ?? 0, npcStride),
        unscaleTileCoord((npc as Partial<{ prevX: number; prev_x: number }>).prevX
          ?? (npc as Partial<{ prevX: number; prev_x: number }>).prev_x
          ?? npc.x
          ?? 0, npcStride),
        unscaleTileCoord((npc as Partial<{ prevY: number; prev_y: number }>).prevY
          ?? (npc as Partial<{ prevY: number; prev_y: number }>).prev_y
          ?? npc.y
          ?? 0, npcStride),
        npcStride,
      ];
    });
    if (JSON.stringify(npcSignature) !== JSON.stringify(internals._ascii_overlay_last_npc_positions)) {
      internals._ascii_overlay_last_npc_positions = npcSignature;
      this._invalidate_ascii_overlay_cache();
    }

    const eventIdentity = mapEvents;
    const lastEventIdentity = internals._ascii_overlay_last_event_identity;
    const lastEventCounts = internals._ascii_overlay_last_event_counts;
    if (
      eventIdentity !== lastEventIdentity ||
      JSON.stringify(eventCounts) !== JSON.stringify(lastEventCounts)
    ) {
      internals._ascii_overlay_last_event_identity = eventIdentity;
      internals._ascii_overlay_last_event_counts = eventCounts;
      this._invalidate_ascii_overlay_cache();
    }

    const warpTilePosition = (warp: MapWarpEvent): [number, number] => {
      if (typeof warp.tile_position === "function") {
        const [x, y] = warp.tile_position(warpStride);
        return [unscaleTileCoord(x, collisionStride), unscaleTileCoord(y, collisionStride)];
      }
      return [warp.x, warp.y];
    };
    const warpOverlayLabel = (warp: MapWarpEvent): string | null => {
      const rawTarget = String(warp.target_map ?? "");
      if (!rawTarget) {
        return null;
      }
      const normalized = rawTarget.toLowerCase().replace(/[^a-z0-9]/g, "");
      if (!normalized) {
        return null;
      }
      if (normalized.includes("pokecenter") || normalized.includes("pokemoncenter")) {
        return "P";
      }
      if (normalized.includes("gym")) {
        return "G";
      }
      if (normalized.includes("mart")) {
        return "M";
      }
      return null;
    };

    const startX = Math.max(
      0,
      Math.min(playerTileX - Math.floor(viewportWidth / 2), mapWidthTiles - viewportWidth)
    );
    const startY = Math.max(
      0,
      Math.min(playerTileY - Math.floor(viewportHeight / 2), mapHeightTiles - viewportHeight)
    );

    const playerOnWarp = allWarps.some((warp) => {
      const [tx, ty] = warpTilePosition(warp);
      return tx === playerTileX && ty === playerTileY;
    });
    const playerMarker: [string, string] = playerOnWarp ? ["@", "1;92"] : ["@", "1;32"];

    const mapIdentity = mapData.mapName ?? null;
    const tilesetSignature: [string | null, string | null] = [
      tileset.tilesetName ?? null,
      null,
    ];
    const metadataLines = buildOverworldMetadata(state.game_state ?? {}, mapIdentity, {
      x: playerTileX,
      y: playerTileY,
    });
    const lastBlockSignature = internals._last_block_feedback
      ? Object.entries(internals._last_block_feedback)
          .sort(([a], [b]) => a.localeCompare(b))
          .flat()
      : null;

    const gameState = state.game_state;
    const party = gameState?.sram?.party ?? null;
    const cacheKey = JSON.stringify([
      startX,
      startY,
      playerTileX,
      playerTileY,
      playerMarker,
      eventCounts,
      activeEventSignature,
      npcSignature,
      mapIdentity,
      tilesetSignature,
      compositeSignature,
      internals.player_direction,
      internals.player_state,
      lastBlockSignature,
      metadataLines,
      useColor,
      collisionStride,
      viewportWidth,
      viewportHeight,
    ]);

    let viewportLines: string[] | null = null;
    let infoLines: string[] | null = null;
    if (
      internals._ascii_overlay_cache_key === cacheKey &&
      internals._ascii_overlay_cached_viewport &&
      internals._ascii_overlay_cached_info
    ) {
      viewportLines = internals._ascii_overlay_cached_viewport;
      infoLines = internals._ascii_overlay_cached_info;
    } else {
      const grid = Array.from({ length: viewportHeight }, () =>
        Array.from({ length: viewportWidth }, () => ({
          glyph: ".",
          glyphRaw: ".",
          overlay: " ",
          overlayRaw: " ",
        }))
      );

      const within = (tx: number, ty: number): boolean =>
        tx >= startX &&
        tx < startX + viewportWidth &&
        ty >= startY &&
        ty < startY + viewportHeight;

      const setCellGlyph = (cell: { glyph: string; glyphRaw: string }, char: string, color: string): void => {
        cell.glyphRaw = char;
        cell.glyph = colorize(char, color);
      };

      const setCellOverlay = (
        cell: { overlay: string; overlayRaw: string },
        char: string,
        color: string
      ): void => {
        cell.overlayRaw = char;
        cell.overlay = colorize(char, color);
      };

      const markFootprint = (
        tileX: number,
        tileY: number,
        char: string,
        color: string,
        overlayTopRight?: [string, string]
      ): void => {
        if (!within(tileX, tileY)) {
          return;
        }
        const cell = grid[tileY - startY][tileX - startX];
        setCellGlyph(cell, char, color);
        if (overlayTopRight) {
          setCellOverlay(cell, overlayTopRight[0], overlayTopRight[1]);
        }
      };

      for (const event of activeBgEvents) {
        const marker = classifyBgEventMarker(event);
        if (!marker) {
          continue;
        }
        const tx = event.x;
        const ty = event.y;
        if (within(tx, ty)) {
          markFootprint(tx, ty, marker.glyph, marker.color);
        }
      }

      const warpsByTile = new Map<string, MapWarpEvent>();
      for (const warp of allWarps) {
        const [tx, ty] = warpTilePosition(warp);
        if (within(tx, ty)) {
          warpsByTile.set(`${tx},${ty}`, warp);
          const overlay = warpOverlayLabel(warp);
          const overlayTuple = overlay ? ([overlay, "1;35"] as [string, string]) : undefined;
          markFootprint(tx, ty, "D", "1;35", overlayTuple);
        }
      }

      const asciiObjects = [...(internals.npcs ?? [])];

      for (const npc of asciiObjects) {
        const npcStride = Math.max(
          1,
          Math.trunc(
            (npc as Partial<{ collisionStride: number }>).collisionStride ?? 1
          )
        );
        const currentX = unscaleTileCoord(npc.x ?? 0, npcStride);
        const currentY = unscaleTileCoord(npc.y ?? 0, npcStride);
        const marker = classifyObjectMarker(npc);
        const dirToken = String(npc.direction ?? "").toLowerCase();
        const overlay = marker.label === "person" ? { up: "^", down: "v", left: "<", right: ">" }[dirToken] : undefined;
        const overlayTuple = overlay ? ([overlay, "1;36"] as [string, string]) : undefined;
        const color =
          marker.label === "item-ball" ? "1;33" :
          marker.label === "berry-tree" ? "0;32" :
          "1;34";
        if (within(currentX, currentY)) {
          markFootprint(currentX, currentY, marker.glyph, color, overlayTuple);
        }
      }

      const collisionGlyphs = new Map<number, [string, string]>();
      const ledgeFaceGlyphs = new Map<number, { glyph: string; dx: number; dy: number }>();
      const ledgeFaceMarks: Array<{ x: number; y: number; glyph: string }> = [];
      const registerGlyph = (name: string, glyph: string, color: string): void => {
        collisionGlyphs.set(resolveCollisionValue(name), [glyph, color]);
      };
      try {
        registerGlyph("CUT_TREE", "C", "1;33");
        registerGlyph("HEADBUTT_TREE", "H", "1;33");
        registerGlyph("MART_SHELF", "$", "1;33");
        registerGlyph("COUNTER", "!", "1;33");
        registerGlyph("COUNTER_98", "!", "1;33");
        registerGlyph("BOOKSHELF", "!", "1;33");
        registerGlyph("PC", "!", "1;33");
        registerGlyph("RADIO", "!", "1;33");
        registerGlyph("TOWN_MAP", "!", "1;33");
        registerGlyph("TV", "!", "1;33");
        registerGlyph("WINDOW", "!", "1;33");
        registerGlyph("INCENSE_BURNER", "!", "1;33");
        registerGlyph("WATERFALL", "=", "1;34");
        registerGlyph("WATERFALL_LEFT", "=", "1;34");
        registerGlyph("WATERFALL_RIGHT", "=", "1;34");
        registerGlyph("WATERFALL_UP", "=", "1;34");
        registerGlyph("WALK_RIGHT", ">", "1;36");
        registerGlyph("WALK_LEFT", "<", "1;36");
        registerGlyph("WALK_UP", "^", "1;36");
        registerGlyph("WALK_DOWN", "v", "1;36");
        registerGlyph("WALK_RIGHT_ALT", ">", "1;36");
        registerGlyph("WALK_LEFT_ALT", "<", "1;36");
        registerGlyph("WALK_UP_ALT", "^", "1;36");
        registerGlyph("WALK_DOWN_ALT", "v", "1;36");
        registerGlyph("CURRENT_RIGHT", ">", "1;34");
        registerGlyph("CURRENT_LEFT", "<", "1;34");
        registerGlyph("CURRENT_UP", "^", "1;34");
        registerGlyph("CURRENT_DOWN", "v", "1;34");
        registerGlyph("COUNTER", "T", "1;36");
      } catch {
        // Leave glyphs empty if constants cannot be resolved.
      }

      const registerLedgeFace = (
        names: string[],
        glyph: string,
        dx: number,
        dy: number
      ): void => {
        for (const token of names) {
          ledgeFaceGlyphs.set(resolveCollisionValue(token), { glyph, dx, dy });
        }
      };
      registerLedgeFace(["HOP_DOWN", "HOP_DOWN_LEFT", "HOP_DOWN_RIGHT"], "d", 0, 1);
      registerLedgeFace(["HOP_UP", "HOP_UP_LEFT", "HOP_UP_RIGHT"], "u", 0, -1);
      registerLedgeFace(["HOP_LEFT"], "l", -1, 0);
      registerLedgeFace(["HOP_RIGHT"], "r", 1, 0);

      const scriptGlyphs: Record<string, [string, string]> = {
        PCSCRIPT: ["P", "1;35"],
        MERCHANDISESHELFSCRIPT: ["$", "1;33"],
        MARTSHELF: ["$", "1;33"],
      };

      const collisionGlyph = (
        permission: number,
        attrs: CollisionAttributes,
        stdScript: string | null
      ): [string, string] | null => {
        if (collisionGlyphs.has(permission)) {
          return collisionGlyphs.get(permission)!;
        }
        if (stdScript) {
          const glyph = scriptGlyphs[stdScript.trim().toUpperCase()];
          if (glyph) {
            return glyph;
          }
        }
        if (attrs?.talk) {
          return ["!", "1;33"];
        }
        return null;
      };

      for (let y = startY; y < startY + viewportHeight; y += 1) {
        for (let x = startX; x < startX + viewportWidth; x += 1) {
          if (!within(x, y)) {
            continue;
          }
          const collisionX = scaleTileCoord(x, collisionStride);
          const collisionY = scaleTileCoord(y, collisionStride);
          const outOfBounds =
            collisionX < 0 ||
            collisionY < 0 ||
            collisionX > maxCollisionTileX ||
            collisionY > maxCollisionTileY;
          let sample: CollisionSample | null = null;
          if (!outOfBounds) {
            try {
              sample = sampleCollision(mapData, tileset, collisionX, collisionY);
            } catch {
              sample = null;
            }
          }
          if (!sample) {
            sample = resolveCompositeCollision(collisionX, collisionY);
          }
          if (!sample) {
            const cell = grid[y - startY][x - startX];
            if (cell.glyphRaw === ".") {
              setCellGlyph(cell, "x", "1;31");
            }
            continue;
          }
          try {
            const attrs = describeCollision(sample.permission);
            const ledgeFace = ledgeFaceGlyphs.get(sample.permission);
            if (ledgeFace) {
              ledgeFaceMarks.push({
                x: x + ledgeFace.dx,
                y: y + ledgeFace.dy,
                glyph: ledgeFace.glyph,
              });
            }
            const glyph = collisionGlyph(sample.permission, attrs, sample.stdScript);
            const cell = grid[y - startY][x - startX];
            if (glyph && cell.glyphRaw === ".") {
              setCellGlyph(cell, glyph[0], glyph[1]);
              continue;
            }
            if (attrs.terrain === Terrain.WATER && cell.glyphRaw === ".") {
              setCellGlyph(cell, "~", "1;34");
              continue;
            }
            if (
              attrs.terrain === Terrain.LAND &&
              GRASS_COLLISION_VALUES.has(sample.permission) &&
              cell.glyphRaw === "."
            ) {
              setCellGlyph(cell, "\"", "0;32");
            }
            const playerState = internals.player_state ?? PlayerState.NORMAL;
            const passableFromAnyDirection = PASSABLE_DIRECTIONS.some((direction) =>
              isPermissionPassable(sample.permission, direction, playerState)
            );
            if (!passableFromAnyDirection && cell.glyphRaw === ".") {
              setCellGlyph(cell, "#", "1;90");
            }
          } catch {
            continue;
          }
        }
      }

      for (const mark of ledgeFaceMarks) {
        if (!within(mark.x, mark.y)) {
          continue;
        }
        const cell = grid[mark.y - startY][mark.x - startX];
        if (cell.glyphRaw === "#") {
          setCellGlyph(cell, mark.glyph, "1;31");
        }
      }

      if (within(playerTileX, playerTileY)) {
        const overlay = { up: "^", down: "v", left: "<", right: ">" }[
          String(internals.player_direction ?? "").toLowerCase()
        ];
        const overlayTuple = overlay ? ([overlay, "1;36"] as [string, string]) : undefined;
        markFootprint(playerTileX, playerTileY, playerMarker[0], playerMarker[1], overlayTuple);
      }

      const legendTokens = new Set<string>();
      for (const row of grid) {
        for (const cell of row) {
          if (cell.glyphRaw.trim()) {
            legendTokens.add(cell.glyphRaw);
          }
          if (cell.overlayRaw.trim()) {
            legendTokens.add(cell.overlayRaw);
          }
        }
      }
      const legendLines = buildAsciiLegendLines(legendTokens);

      const viewportRows = grid.map((row) =>
        row.map((cell) => `${cell.glyph}${cell.overlay}`).join(" ")
      );
      const rowLabelWidth = Math.max(
        String(startY + viewportHeight - 1).length,
        2
      );
      const colHeaderLines = formatAxisHeader(startX, viewportWidth);
      viewportLines = colHeaderLines.map(
        (line) => `${" ".repeat(rowLabelWidth + 1)}${line}`
      );
      viewportRows.forEach((row, idx) => {
        viewportLines!.push(`${formatRowLabel(startY + idx, rowLabelWidth)} ${row}`);
      });

      infoLines = metadataLines.length ? [...metadataLines] : [];
      if (legendLines.length) {
        infoLines.push(...legendLines);
      }

      internals._ascii_overlay_cache_key = cacheKey;
      internals._ascii_overlay_cached_viewport = viewportLines;
      internals._ascii_overlay_cached_info = infoLines;
    }

    const dialogueLines: string[] = [];
    let promptLines: string[] | null = null;
    let overlayLines: string[] = [];
    const dialogue = internals.dialogue;
    if (dialogue) {
      overlayLines = dialogueTextLinesForSnapshot(dialogue);
      const prompt = dialogue._yes_no_prompt;
      if (prompt) {
        const options = ["YES", "NO"];
        const contextLines = filterPromptContextLines(overlayLines).slice(-2);
        const optionLines = options.map((label, idx) =>
          `${idx === prompt.selection ? ">" : "  "}${label}`
        );
        promptLines = contextLines.length ? [...contextLines, ...optionLines] : optionLines;
      } else if (dialogue._selection_prompt?.lines?.length) {
        promptLines = [...dialogue._selection_prompt.lines];
      }
    }
    if (overlayLines.length) {
      dialogueLines.push(...overlayLines);
    }

    if (typeof text_ui.renderOverworldOverlay === "function") {
      text_ui.renderOverworldOverlay(viewportLines!, infoLines!, {
        promptLines,
        dialogueLines,
      });
    } else {
      text_ui.renderSnapshot(
        viewportLines!,
        infoLines!,
        "Overworld",
        "Info",
        null,
        promptLines,
        dialogueLines
      );
    }
  }

  protected _draw_debug_sightlines(cameraX: number, cameraY: number): void {
    const internals = getRenderingInternals(this);
    if (!internals._debug_sightlines || !this.screen) {
      return;
    }
    const [originX, originY] = internals._composite_origin ?? [0, 0];
    const offsetX = originX - cameraX;
    const offsetY = originY - cameraY;
    const stride = Math.max(1, Math.trunc(internals.TILES_PER_COLLISION ?? 1));
    const footprint = stride - 1;
    const centerOffset = Math.floor(METATILE_SIZE / 2);
    for (const npc of internals.npcs ?? []) {
      if (!internals._npc_is_trainer?.(npc)) {
        continue;
      }
      const direction = String(npc.direction ?? "").toLowerCase();
      const vector = BASE_DIRECTION_VECTORS[direction];
      const radius = Math.max(Number(npc.event?.radius ?? 0), 0);
      const basePx =
        npc.pixelX ??
        (npc as any).pixel_x ??
        npc.pixelX ??
        this._tile_to_pixels((npc.x ?? 0) - footprint);
      const basePy =
        npc.pixelY ??
        (npc as any).pixel_y ??
        npc.pixelY ??
        this._tile_to_pixels((npc.y ?? 0) - footprint);
      const startPoint: [number, number] = [
        basePx + centerOffset + offsetX,
        basePy + centerOffset + offsetY,
      ];
      const points: Array<[number, number]> = [startPoint];
      if (vector && radius > 0) {
        const [dx, dy] = vector;
        for (let step = 1; step <= radius; step += 1) {
          const targetX = (npc.x ?? 0) + dx * step * stride;
          const targetY = (npc.y ?? 0) + dy * step * stride;
          const tilePx = this._tile_to_pixels(targetX - footprint);
          const tilePy = this._tile_to_pixels(targetY - footprint);
          points.push([tilePx + centerOffset + offsetX, tilePy + centerOffset + offsetY]);
        }
      }
      const color: [number, number, number] = radius > 0 ? [255, 100, 0] : [255, 120, 200];
      if (points.length > 1) {
        drawPolyline(this.screen, color, points, 2);
      }
      for (const point of points) {
        gameEngine.draw.circle(this.screen, color, [Math.round(point[0]), Math.round(point[1])], 4);
      }
    }
  }

  public start_earthquake(intensity: number, duration: number): void {
    const internals = getRenderingInternals(this);
    internals._earthquake_intensity = Math.max(1, Math.trunc(intensity));
    internals._earthquake_phase = 0;
    internals._earthquake_remaining_frames = Math.max(1, Math.trunc(duration));
    this._earthquake_offset = [0, 0];
  }

  protected _apply_earthquake_offset(
    cameraX: number,
    cameraY: number,
    maxX: number,
    maxY: number
  ): [number, number] {
    const [offsetX, offsetY] = this._earthquake_offset ?? [0, 0];
    return [
      Math.min(Math.max(cameraX + offsetX, 0), maxX),
      Math.min(Math.max(cameraY + offsetY, 0), maxY),
    ];
  }

  protected _update_earthquake_state(): void {
    const internals = getRenderingInternals(this);
    if ((internals._earthquake_remaining_frames ?? 0) <= 0) {
      this._earthquake_offset = [0, 0];
      return;
    }
    const intensity = internals._earthquake_intensity ?? 1;
    const pattern: Array<[number, number]> = [
      [intensity, 0],
      [-intensity, 0],
      [0, intensity],
      [0, -intensity],
    ];
    const phase = internals._earthquake_phase ?? 0;
    this._earthquake_offset = pattern[phase % pattern.length];
    internals._earthquake_phase = phase + 1;
    internals._earthquake_remaining_frames = Math.max(
      0,
      (internals._earthquake_remaining_frames ?? 0) - 1
    );
    if ((internals._earthquake_remaining_frames ?? 0) <= 0) {
      this._earthquake_offset = [0, 0];
    }
  }

  protected _start_color_fade(color: [number, number, number], targetAlpha: number, steps: number): void {
    const internals = getRenderingInternals(this);
    const clamped = Math.max(0, Math.min(255, Math.trunc(targetAlpha)));
    const totalSteps = steps <= 0 ? 1 : Math.trunc(steps);
    this._fade_overlay.fill([color[0], color[1], color[2], 255]);
    internals._fade_overlay_color = [color[0], color[1], color[2]];
    const currentAlpha = internals._fade_alpha ?? 0;
    internals._fade_start_alpha = currentAlpha;
    internals._fade_end_alpha = clamped;
    internals._fade_steps_total = totalSteps;
    internals._fade_progress = 0;
    if (currentAlpha === clamped) {
      internals._fade_active = false;
      internals._fade_alpha = clamped;
      return;
    }
    internals._fade_active = true;
  }

  protected _update_fade(): void {
    const internals = getRenderingInternals(this);
    const stepsTotal = internals._fade_steps_total ?? 0;
    if (!internals._fade_active || stepsTotal <= 0) {
      return;
    }
    internals._fade_progress = Math.min(
      (internals._fade_progress ?? 0) + 1,
      stepsTotal
    );
    const progressRatio = internals._fade_progress / stepsTotal;
    const delta = (internals._fade_end_alpha ?? 0) - (internals._fade_start_alpha ?? 0);
    internals._fade_alpha = Math.round((internals._fade_start_alpha ?? 0) + delta * progressRatio);
    if (internals._fade_progress >= stepsTotal) {
      internals._fade_active = false;
      internals._fade_alpha = internals._fade_end_alpha ?? 0;
    }
  }

  public fade_to_black(frames = 8): void {
    this._start_color_fade([0, 0, 0], 255, frames);
  }

  public fade_from_black(frames = 8): void {
    this._start_color_fade([0, 0, 0], 0, frames);
  }

  public fade_to_white(frames = 8): void {
    const internals = getRenderingInternals(this);
    internals._white_fade_pending_clear = true;
    this._start_color_fade([255, 255, 255], 255, frames);
  }

  public fade_from_white(frames = 8): void {
    const internals = getRenderingInternals(this);
    internals._white_fade_pending_clear = false;
    this._start_color_fade([255, 255, 255], 0, frames);
  }

  public clear_pending_white_fade(): void {
    const internals = getRenderingInternals(this);
    if (!internals._white_fade_pending_clear) {
      return;
    }
    const color = internals._fade_overlay_color;
    if (!color || color[0] !== 255 || color[1] !== 255 || color[2] !== 255) {
      return;
    }
    this.fade_from_white();
  }

  protected _start_poison_flash(): void {
    const internals = getRenderingInternals(this);
    internals._poison_flash_remaining = POISON_FLASH_DURATION;
    internals._poison_overlay_alpha = POISON_OVERLAY_ALPHA;
    if (!internals._poison_overlay) {
      const overlay = new gameEngine.Surface(
        this.screen?.get_width() ?? 0,
        this.screen?.get_height() ?? 0
      );
      overlay.fill(POISON_OVERLAY_COLOR);
      internals._poison_overlay = overlay;
    }
  }

  protected _update_poison_flash(): void {
    const internals = getRenderingInternals(this);
    if ((internals._poison_flash_remaining ?? 0) <= 0) {
      internals._poison_overlay_alpha = 0;
      return;
    }
    const alpha = Math.trunc(
      (POISON_OVERLAY_ALPHA * (internals._poison_flash_remaining ?? 0)) / POISON_FLASH_DURATION
    );
    internals._poison_overlay_alpha = Math.max(alpha, 0);
    internals._poison_flash_remaining = Math.max(
      0,
      (internals._poison_flash_remaining ?? 0) - 1
    );
  }
}

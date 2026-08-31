// ASM mapping: pokecrystal_disassembly/engine/tilesets/tileset_anims.asm (AnimateFlowerTile, AnimateWaterTile, AnimateFountainTile, tower pillar frames).
import fs from "fs";
import path from "path";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { assetExists } from "@pokecrystal/core/core/asset-manifest";
import { joinPath } from "@pokecrystal/core/core/path-utils";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { GameState } from "@pokecrystal/core/core/state";
import { GB_FRAME_RATE } from "@pokecrystal/core/core/gb-timing";
import { VRAMManager, TileBlockManager } from "@pokecrystal/core/core/memory/vram";
import { METATILE_SIZE, TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { OverworldMap } from "./overworld-map";
import type { OverworldMetatile, OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";

const FLOWER_ASSET_DIR = getAssetPath("gfx", "tilesets", "flower");
const TOWER_PILLAR_ASSET_DIR = getAssetPath("gfx", "tilesets", "tower-pillar");
const WATER_ASSET_PATH = getAssetPath("gfx", "tilesets", "water", "water.2bpp");
const FOUNTAIN_ASSET_DIR = getAssetPath("gfx", "tilesets", "fountain");
const LAVA_ASSET_DIR = getAssetPath("gfx", "tilesets", "lava");
const FOREST_TREE_ASSET_DIR = getAssetPath("gfx", "tilesets", "forest-tree");

const GB_TILE_ANIMATION_TICK_RATE = GB_FRAME_RATE;
const TILE_ANIMATION_TICK_MS = 1000.0 / GB_TILE_ANIMATION_TICK_RATE;
const FLOWER_PHASE_TICKS = Math.max(1, Math.round(GB_TILE_ANIMATION_TICK_RATE));

const FRAME_ORDER = [
  "dmg_1.2bpp",
  "cgb_1.2bpp",
  "dmg_2.2bpp",
  "cgb_2.2bpp",
];

export const WHIRLPOOL_TILE_INDEXES = [0x32, 0x33, 0x42, 0x43];

type Surface = InstanceType<typeof gameEngine.Surface>;

type TileFrame = {
  tileBytes: number[];
  surface: Surface;
};

type CompositeDirtyRects = Map<Surface, Array<InstanceType<typeof gameEngine.Rect>>>;

type TilesetAnimationOwner = {
  refresh_composite_surfaces?: (dirtyRects: CompositeDirtyRects) => void;
};

type AnimatedTarget = {
  name: string;
  surface: Surface;
  prioritySurface: Surface | null;
  map: OverworldMap;
  tileset: TilesetLike;
  renderer: TileRenderer;
  metatileCoords: Array<[number, number]>;
};

type CompositeSegment = {
  name: string;
  map: OverworldMap;
  tileset: TilesetLike;
  surface: Surface;
  priority_surface?: Surface | null;
  prioritySurface?: Surface | null;
};

type MetatileTile = {
  tileIndex?: number;
  tile_index?: number;
  priority?: boolean;
  surface?: Surface;
};

type RenderableMetatile = {
  tiles: Array<Array<MetatileTile | number>>;
};

type MetatileCollision = {
  collision: readonly number[];
};

type TilesetLike = Omit<OverworldTilesetLike, "metatiles"> & {
  tiles?: Surface[];
  tileBytes?: number[][];
  tile_bytes?: number[][];
  metatiles?: readonly RenderableMetatile[] | readonly MetatileCollision[] | readonly OverworldMetatile[];
  tilesetName?: string;
  tileset_name?: string;
};

const nowMs = (): number => {
  if (typeof performance !== "undefined" && typeof performance.now === "function") {
    return performance.now();
  }
  return Date.now();
};

const cloneSurface = (source: Surface): Surface => {
  const surface = new gameEngine.Surface(source.width, source.height);
  surface.blit(source, [0, 0]);
  return surface;
};

const resolveTileBlock = (gameState: GameState, name: string): TileBlockManager => {
  const manager = new VRAMManager(gameState.vram);
  return new TileBlockManager(manager.resolveTileBlock(name));
};

const ensureTilesetName = (tileset: TilesetLike): string => {
  const name = tileset.tilesetName ?? tileset.tileset_name ?? "";
  const normalized = String(name).trim();
  if (!normalized) {
    throw new Error("Tileset animation requires a tileset name.");
  }
  return normalized.toLowerCase();
};

const resolveTileIndex = (entry: MetatileTile | number): number | null => {
  if (typeof entry === "number" && Number.isFinite(entry)) {
    return entry;
  }
  if (entry && typeof entry === "object") {
    const index = (entry.tileIndex ?? entry.tile_index) as number | undefined;
    if (typeof index === "number" && Number.isFinite(index)) {
      return index;
    }
  }
  return null;
};

const resolveTileSurface = (
  tileset: TilesetLike,
  entry: MetatileTile | number,
  tileIndex: number
): Surface => {
  if (entry && typeof entry === "object" && entry.surface) {
    return entry.surface;
  }
  if (Array.isArray(tileset.tiles) && tileIndex < tileset.tiles.length) {
    const surface = tileset.tiles[tileIndex];
    if (surface) {
      return surface;
    }
  }
  throw new Error(`Tileset animation requires a surface for tile ${tileIndex}.`);
};

class TileRenderer {
  public _subtile_cache = new Map<string, Surface>();
  public _priority_subtile_cache = new Map<string, Surface>();
  public _metatile_cache = new Map<number, Surface>();

  constructor(
    private tileset: TilesetLike,
    private surface: Surface,
    private prioritySurface: Surface | null
  ) {}

  _blit_metatile(metatile: RenderableMetatile, baseX: number, baseY: number): void {
    if (!Array.isArray(metatile.tiles)) {
      throw new Error("Tileset animation requires metatile tiles.");
    }
    for (let row = 0; row < metatile.tiles.length; row += 1) {
      const rowTiles = metatile.tiles[row] ?? [];
      for (let col = 0; col < rowTiles.length; col += 1) {
        const entry = rowTiles[col];
        const tileIndex = resolveTileIndex(entry);
        if (tileIndex === null) {
          throw new Error("Tileset animation encountered a metatile without tile indices.");
        }
        const tileSurface = resolveTileSurface(this.tileset, entry, tileIndex);
        const destX = baseX + col * TILE_SIZE;
        const destY = baseY + row * TILE_SIZE;
        this.surface.blit(tileSurface, [destX, destY]);
        const priority = typeof entry === "object" ? Boolean(entry.priority) : false;
        if (priority && this.prioritySurface) {
          this.prioritySurface.blit(tileSurface, [destX, destY]);
        }
      }
    }
  }
}

class FieldMoveVramLoader {
  private static readonly WHIRLPOOL_FILES = ["1.2bpp", "2.2bpp", "3.2bpp", "4.2bpp"];
  private static readonly WHIRLPOOL_DEST_INDEXES = WHIRLPOOL_TILE_INDEXES;
  private static readonly TILE_BYTES = 16;
  private static readonly CUT_GFX: Array<[string, string, number, number]> = [
    ["cut_grass.2bpp", "vTiles0", 0x80, 4],
    ["cut_tree.2bpp", "vTiles0", 0x84, 4],
  ];
  private static readonly HEADBUTT_GFX: Array<[string, string, number, number]> = [
    ["cut_grass.2bpp", "vTiles0", 0x80, 4],
    ["headbutt_tree.2bpp", "vTiles0", 0x84, 8],
  ];

  public _current_frame = -1;
  private readonly whirlpoolFrames = new Map<number, number[][]>();
  private readonly whirlpoolSurfaces = new Map<number, Surface[]>();
  private readonly overworldBase = getAssetPath("gfx", "overworld");

  constructor(private gameState: GameState) {
    this._loadWhirlpoolTiles();
  }

  requestCutTiles(): void {
    for (const [filename, block, index, count] of FieldMoveVramLoader.CUT_GFX) {
      this._request2bppTiles(filename, block, index, count);
    }
  }

  requestHeadbuttTiles(): void {
    for (const [filename, block, index, count] of FieldMoveVramLoader.HEADBUTT_GFX) {
      this._request2bppTiles(filename, block, index, count);
    }
  }

  updateWhirlpoolTiles(timer: number, { force = false }: { force?: boolean } = {}): boolean {
    const frame = timer & 0x03;
    if (!force && frame === this._current_frame) {
      return false;
    }
    this._current_frame = frame;
    const tileBlock = resolveTileBlock(this.gameState, "vTiles2");
    for (const [tileIndex, frames] of this.whirlpoolFrames.entries()) {
      this._assertTileIndex(tileBlock, tileIndex, "whirlpool_update");
      tileBlock.writeTile(tileIndex, frames[frame]);
    }
    return true;
  }

  surfaceForWhirlpoolFrame(tileIndex: number, frameIndex: number): Surface | null {
    const frames = this.whirlpoolSurfaces.get(tileIndex);
    if (!frames || frameIndex < 0 || frameIndex >= frames.length) {
      return null;
    }
    return frames[frameIndex];
  }

  private _request2bppTiles(
    filename: string,
    blockName: string,
    startIndex: number,
    tileCount: number
  ): void {
    const filePath = joinPath(this.overworldBase, filename);
    if (!assetExists(filePath)) {
      throw new Error(`Missing field move tileset ${filePath}`);
    }
    const raw = fs.readFileSync(filePath);
    const expected = tileCount * FieldMoveVramLoader.TILE_BYTES;
    if (raw.length !== expected) {
      throw new Error(
        `${filename} contains ${raw.length} bytes, expected ${expected}`
      );
    }
    const tileBlock = resolveTileBlock(this.gameState, blockName);
    this._validateTileRange(tileBlock, startIndex, tileCount, filename);
    for (let offset = 0; offset < tileCount; offset += 1) {
      const start = offset * FieldMoveVramLoader.TILE_BYTES;
      const chunk = Array.from(
        raw.slice(start, start + FieldMoveVramLoader.TILE_BYTES)
      );
      tileBlock.writeTile(startIndex + offset, chunk);
      this._verifyTileBytes(tileBlock, startIndex + offset, chunk, filename);
    }
  }

  private _validateTileRange(
    tileBlock: TileBlockManager,
    startIndex: number,
    tileCount: number,
    source: string
  ): void {
    const endIndex = startIndex + tileCount;
    if (endIndex > TileBlockManager.TILE_COUNT) {
      throw new Error(
        `${source} would write tiles ${startIndex.toString(16)}-${(
          endIndex - 1
        ).toString(16)}, but ${
          TileBlockManager.TILE_COUNT
        } tiles are available.`
      );
    }
  }

  private _verifyTileBytes(
    tileBlock: TileBlockManager,
    tileIndex: number,
    expected: number[],
    source: string
  ): void {
    const written = tileBlock.readTile(tileIndex);
    if (written.length !== expected.length) {
      throw new Error(`VRAM verification failed for ${source} tile ${tileIndex.toString(16)}`);
    }
    for (let i = 0; i < expected.length; i += 1) {
      if (written[i] !== expected[i]) {
        throw new Error(`VRAM verification failed for ${source} tile ${tileIndex.toString(16)}`);
      }
    }
  }

  private _assertTileIndex(
    tileBlock: TileBlockManager,
    tileIndex: number,
    source: string
  ): void {
    if (tileIndex < 0 || tileIndex >= TileBlockManager.TILE_COUNT) {
      throw new Error(
        `${source} attempted to write tile ${tileIndex.toString(16)}`
      );
    }
  }

  private _loadWhirlpoolTiles(): void {
    const basePath = getAssetPath("gfx", "tilesets", "whirlpool");
    FieldMoveVramLoader.WHIRLPOOL_DEST_INDEXES.forEach((tileIndex, idx) => {
      const filename = FieldMoveVramLoader.WHIRLPOOL_FILES[idx];
      const filePath = joinPath(basePath, filename);
      if (!assetExists(filePath)) {
        throw new Error(`Missing whirlpool assets at ${filePath}`);
      }
      const data = fs.readFileSync(filePath);
      const tiles = decode2bppTiles(data);
      const totalTiles = Math.floor(data.length / FieldMoveVramLoader.TILE_BYTES);
      if (totalTiles < 4) {
        throw new Error(`${filename} contains ${totalTiles} tiles, expected at least 4.`);
      }
      const frames: number[][] = [];
      for (let i = 0; i < totalTiles; i += 1) {
        const start = i * FieldMoveVramLoader.TILE_BYTES;
        frames.push(
          Array.from(data.slice(start, start + FieldMoveVramLoader.TILE_BYTES))
        );
      }
      if (frames.length < 4 || tiles.length < 4) {
        throw new Error(`Whirlpool asset ${filename} lacks 4 frames.`);
      }
      this.whirlpoolFrames.set(tileIndex, frames.slice(0, 4));
      this.whirlpoolSurfaces.set(tileIndex, tiles.slice(0, 4));
    });
    this._writeAllWhirlpoolTiles();
  }

  private _writeAllWhirlpoolTiles(): void {
    const tileBlock = resolveTileBlock(this.gameState, "vTiles2");
    for (const [tileIndex, frames] of this.whirlpoolFrames.entries()) {
      if (!frames.length) {
        continue;
      }
      this._assertTileIndex(tileBlock, tileIndex, "whirlpool_init");
      tileBlock.writeTile(tileIndex, frames[0]);
    }
  }
}

const decode2bppTile = (data: Uint8Array): Surface => {
  const surface = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
  for (let row = 0; row < TILE_SIZE; row += 1) {
    const low = data[row * 2];
    const high = data[row * 2 + 1];
    for (let col = 0; col < TILE_SIZE; col += 1) {
      const bit = 7 - col;
      const level = (((high >> bit) & 1) << 1) | ((low >> bit) & 1);
      const gray = [255, 170, 85, 0][level];
      surface.set_at([col, row], [gray, gray, gray, 255]);
    }
  }
  return surface;
};

export const derivePaletteFromSurface = (source: Surface): Array<[number, number, number]> => {
  const image = source.getImageData();
  const data = image.data;
  const colors = new Map<string, [number, number, number]>();
  for (let i = 0; i < data.length; i += 4) {
    if (data[i + 3] === 0) {
      continue;
    }
    const color: [number, number, number] = [data[i], data[i + 1], data[i + 2]];
    const key = `${color[0]},${color[1]},${color[2]}`;
    if (!colors.has(key)) {
      colors.set(key, color);
    }
  }
  if (!colors.size) {
    // Fall back to a neutral palette when color data is unavailable (e.g., headless tests).
    return [
      [0, 0, 0],
      [0, 0, 0],
      [0, 0, 0],
      [0, 0, 0],
    ];
  }
  const palette = Array.from(colors.values()).sort(
    (a, b) => b[0] + b[1] + b[2] - (a[0] + a[1] + a[2])
  );
  if (palette.length > 4) {
    throw new Error(
      `Animated tile palette extraction found ${palette.length} colors; expected <= 4.`
    );
  }
  while (palette.length < 4) {
    palette.push(palette[palette.length - 1]);
  }
  return palette;
};

const paletteIndexFromGray = (gray: number): number => {
  if (gray >= 213) {
    return 0;
  }
  if (gray >= 128) {
    return 1;
  }
  if (gray >= 43) {
    return 2;
  }
  return 3;
};

const applyPaletteToSurface = (source: Surface, palette: Array<[number, number, number]>): Surface => {
  const [width, height] = source.get_size();
  const target = new gameEngine.Surface(width, height);
  const image = source.getImageData();
  const data = image.data;

  for (let i = 0; i < data.length; i += 4) {
    const alpha = data[i + 3];
    if (alpha === 0) {
      continue;
    }
    const intensity = Math.round((data[i] + data[i + 1] + data[i + 2]) / 3);
    const paletteIndex = paletteIndexFromGray(intensity);
    const [r, g, b] = palette[paletteIndex] ?? palette[0];
    data[i] = r;
    data[i + 1] = g;
    data[i + 2] = b;
  }
  target.getContext()!.putImageData(image, 0, 0);
  return target;
};

const encodeSurfaceTo2bpp = (source: Surface): number[] => {
  const image = source.getImageData();
  const data = image.data;
  const colors = new Map<string, number>();
  for (let i = 0; i < data.length; i += 4) {
    if (data[i + 3] === 0) {
      continue;
    }
    const key = `${data[i]},${data[i + 1]},${data[i + 2]}`;
    if (!colors.has(key)) {
      colors.set(key, data[i] + data[i + 1] + data[i + 2]);
    }
  }
  const levels = Array.from(colors.entries())
    .sort(([, a], [, b]) => b - a)
    .map(([key], index) => [key, Math.min(index, 3)] as const);
  const levelByColor = new Map(levels);
  const bytes: number[] = [];
  for (let row = 0; row < TILE_SIZE; row += 1) {
    let low = 0;
    let high = 0;
    for (let col = 0; col < TILE_SIZE; col += 1) {
      const offset = (row * TILE_SIZE + col) * 4;
      const key = `${data[offset]},${data[offset + 1]},${data[offset + 2]}`;
      const level = data[offset + 3] === 0 ? 0 : levelByColor.get(key) ?? 0;
      const bit = 7 - col;
      low |= (level & 1) << bit;
      high |= ((level >> 1) & 1) << bit;
    }
    bytes.push(low, high);
  }
  return bytes;
};

const rotateByteLeft = (value: number): number => ((value << 1) | (value >> 7)) & 0xff;

const scrollTileRightLeft = (tileBytes: number[]): number[] => {
  const result = tileBytes.slice(0, 16);
  for (let offset = 0; offset < result.length; offset += 2) {
    result[offset] = rotateByteLeft(result[offset] ?? 0);
    result[offset + 1] = rotateByteLeft(result[offset + 1] ?? 0);
  }
  return result;
};

const scrollTileDown = (tileBytes: number[]): number[] => {
  const result = tileBytes.slice(0, 16);
  const lastRow = result.slice(14, 16);
  for (let offset = 14; offset >= 2; offset -= 2) {
    result[offset] = result[offset - 2] ?? 0;
    result[offset + 1] = result[offset - 1] ?? 0;
  }
  result[0] = lastRow[0] ?? 0;
  result[1] = lastRow[1] ?? 0;
  return result;
};

const decode2bppTiles = (data: Uint8Array): Surface[] => {
  if (data.length % 16 !== 0) {
    throw new Error("2bpp payload must be aligned to 16-byte tiles");
  }
  const tiles: Surface[] = [];
  for (let offset = 0; offset < data.length; offset += 16) {
    tiles.push(decode2bppTile(data.slice(offset, offset + 16)));
  }
  return tiles;
};

const loadFramesFrom2bpp = (filePath: string, frameCount: number): TileFrame[] => {
  if (!assetExists(filePath)) {
    throw new Error(`Missing animation asset: ${filePath}`);
  }
  const data = fs.readFileSync(filePath);
  const bytesPerTile = TILE_SIZE * 2;
  if (data.length % bytesPerTile !== 0) {
    throw new Error(
      `Animation payload must be a whole number of tiles, got ${data.length} bytes`
    );
  }
  const totalFrames = Math.floor(data.length / bytesPerTile);
  if (frameCount > 0 && totalFrames !== frameCount) {
    throw new Error(
      `Animation requires ${frameCount} frames, found ${totalFrames} in ${path.basename(
        filePath
      )}`
    );
  }
  const frames: TileFrame[] = [];
  for (let index = 0; index < totalFrames; index += 1) {
    const start = index * bytesPerTile;
    const chunk = Array.from(data.slice(start, start + bytesPerTile));
    const surface = decode2bppTile(data.slice(start, start + bytesPerTile));
    frames.push({ tileBytes: chunk, surface });
  }
  return frames;
};

export class TilesetAnimationController {
  public readonly FLOWER_TILE_INDEX = 0x03;
  public readonly WATER_TILE_INDEX = 0x14;
  public readonly FOUNTAIN_TILE_INDEX = 0x5f;
  public readonly LAVA_BUBBLE_TILE_1_INDEX = 0x5b;
  public readonly LAVA_BUBBLE_TILE_2_INDEX = 0x38;
  public readonly FOREST_TREE_LEFT_TILE_INDEX = 0x0c;
  public readonly FOREST_TREE_RIGHT_TILE_INDEX = 0x0f;
  public readonly CAVE_WATER_TILE_INDEX = 0x14;
  public readonly CAVE_WATERFALL_TILE_INDEX = 0x40;
  public readonly ICE_PATH_WATER_TILE_INDEX = 0x35;
  public readonly ICE_PATH_WATERFALL_TILE_INDEX = 0x31;
  public readonly ICE_PATH_WATER_VISIBLE_TILE_INDEX = 0xb5;
  public readonly ICE_PATH_WATERFALL_VISIBLE_TILE_INDEX = 0xb1;
  public readonly WATER_TIMER_DIVISOR = 11;
  public readonly WHIRLPOOL_TILE_INDICES = WHIRLPOOL_TILE_INDEXES;
  public readonly TOWER_PILLAR_TILE_DESTINATIONS = [
    0x2d,
    0x2f,
    0x3d,
    0x3f,
    0x3c,
    0x2c,
    0x4d,
    0x4f,
    0x5d,
    0x5f,
  ];
  // ASM: TilesetTowerAnim in engine/tilesets/tileset_anims.asm updates one tile
  // per frame in this order, then waits before restarting the command loop.
  public readonly TOWER_PILLAR_UPDATE_SEQUENCE = [
    0x5d,
    0x5f,
    0x4d,
    0x4f,
    0x3c,
    0x2c,
    0x3d,
    0x3f,
    0x2d,
    0x2f,
  ];
  public readonly TOWER_PILLAR_FRAME_OFFSETS = [0, 1, 2, 3, 4, 3, 2, 1];
  public readonly TOWER_PILLAR_FRAME_COUNT = 5;
  public readonly TOWER_PILLAR_FILE_NAMES = Array.from({ length: 10 }, (_, i) =>
    `${i + 1}.2bpp`
  );
  public readonly FLOWER_SUPPORTED_TILESETS = new Set([
    "johto",
    "johto_modern",
    "johto_modern_generated",
    "kanto",
    "park",
    "forest",
  ]);
  public readonly TOWER_SUPPORTED_TILESETS = new Set(["tower"]);
  public readonly FOUNTAIN_SUPPORTED_TILESETS = new Set(["park"]);
  public readonly LAVA_SUPPORTED_TILESETS = new Set(["elite_four_room"]);
  public readonly CAVE_SCROLL_SUPPORTED_TILESETS = new Set(["cave", "dark_cave"]);
  public readonly ICE_PATH_SCROLL_SUPPORTED_TILESETS = new Set(["ice_path"]);
  public readonly WATER_SUPPORTED_TILESETS = new Set([
    "johto",
    "johto_modern",
    "johto_modern_generated",
    "kanto",
    "park",
    "forest",
    "port",
    "cave",
    "dark_cave",
    "ice_path",
  ]);
  public readonly SUPPORTED_TILESETS = new Set([
    ...this.FLOWER_SUPPORTED_TILESETS,
    ...this.TOWER_SUPPORTED_TILESETS,
    ...this.WATER_SUPPORTED_TILESETS,
    ...this.FOUNTAIN_SUPPORTED_TILESETS,
    ...this.LAVA_SUPPORTED_TILESETS,
    ...this.CAVE_SCROLL_SUPPORTED_TILESETS,
    ...this.ICE_PATH_SCROLL_SUPPORTED_TILESETS,
  ]);

  private timer = 0;
  private currentPhase = 0;
  private currentFrameIndex: number | null = null;
  private baseTarget: AnimatedTarget | null = null;
  private segmentTargets: AnimatedTarget[] = [];
  private tilesets: TilesetLike[] = [];
  private active = false;
  private flowerActive = false;
  private towerActive = false;
  private fountainActive = false;
  private lavaActive = false;
  private forestTreeActive = false;
  private caveScrollActive = false;
  private icePathScrollActive = false;
  private waterActive = false;
  private towerCommandFrame = 0;
  private caveScrollCommandFrame = 0;
  private waterFrameIndex: number | null = null;
  private fountainFrameIndex: number | null = null;
  private lavaFrameIndex1: number | null = null;
  private lavaFrameIndex2: number | null = null;
  private forestTreeFrameIndex: number | null = null;
  private isCgb = true;
  private tickAccumulator = 0;
  private lastTickTime = nowMs();
  private fieldMoveLoader: FieldMoveVramLoader;
  private animatedTileIndices: number[] = [...this.WHIRLPOOL_TILE_INDICES];
  private whirlpoolActive = false;
  private whirlpoolBackups = new Map<TilesetLike, Map<number, [number[], Surface]>>();
  private readonly towerFrameSetIndexByDestination = new Map<number, number>(
    this.TOWER_PILLAR_TILE_DESTINATIONS.map((destination, index) => [destination, index])
  );
  private _waterFrameCache?: TileFrame[];
  private _flowerFrameCache?: TileFrame[];
  private _towerFrameCache?: TileFrame[][];
  private _fountainFrameCache?: TileFrame[];
  private _lavaFrameCache?: TileFrame[];
  private _forestTreeFrameCache?: TileFrame[];
  private tileAnimationBuffer: number[] | null = null;

  constructor(private owner: TilesetAnimationOwner, private gameState: GameState) {
    this.fieldMoveLoader = new FieldMoveVramLoader(gameState);
  }

  // Compatibility layer: much of the overworld code still calls snake_case hooks
  // from the original Python engine. Keep these as thin adapters so we don't
  // silently skip ASM-timed tile animations.
  on_map_loaded(options: {
    map_name?: string;
    mapName?: string;
    map_obj?: OverworldMap;
    mapObj?: OverworldMap;
    tileset: TilesetLike;
    surface: Surface | null;
    priority_surface?: Surface | null;
    prioritySurface?: Surface | null;
  }): void {
    const mapName = options.mapName ?? options.map_name;
    const mapObj = options.mapObj ?? options.map_obj;
    if (!mapName || !mapObj) {
      throw new Error("Tileset animation on_map_loaded requires map_name and map_obj.");
    }
    this.onMapLoaded({
      mapName,
      mapObj,
      tileset: options.tileset,
      surface: options.surface,
      prioritySurface: options.prioritySurface ?? options.priority_surface ?? null,
    });
  }

  set_connection_segments(segments: Iterable<CompositeSegment>): void {
    this.setConnectionSegments(segments);
  }

  set_whirlpool_active(active: boolean): void {
    this.setWhirlpoolActive(active);
  }

  onMapLoaded(options: {
    mapName: string;
    mapObj: OverworldMap;
    tileset: TilesetLike;
    surface: Surface | null;
    prioritySurface: Surface | null;
  }): void {
    const { mapName, mapObj, tileset, surface, prioritySurface } = options;
    this.segmentTargets = [];
    this.baseTarget = null;
    this.tilesets = [];
    const tilesetName = ensureTilesetName(tileset);
    this.flowerActive = this.FLOWER_SUPPORTED_TILESETS.has(tilesetName);
    this.towerActive = this.TOWER_SUPPORTED_TILESETS.has(tilesetName);
    this.fountainActive = this.FOUNTAIN_SUPPORTED_TILESETS.has(tilesetName);
    this.lavaActive = this.LAVA_SUPPORTED_TILESETS.has(tilesetName);
    this.forestTreeActive = tilesetName === "forest";
    this.caveScrollActive = this.CAVE_SCROLL_SUPPORTED_TILESETS.has(tilesetName);
    this.icePathScrollActive = this.ICE_PATH_SCROLL_SUPPORTED_TILESETS.has(tilesetName);
    this.waterActive = this.WATER_SUPPORTED_TILESETS.has(tilesetName);
    this.active = this.SUPPORTED_TILESETS.has(tilesetName);
    this.towerCommandFrame = 0;
    this.waterFrameIndex = null;
    this.fountainFrameIndex = null;
    this.lavaFrameIndex1 = null;
    this.lavaFrameIndex2 = null;
    this.forestTreeFrameIndex = null;
    this.caveScrollCommandFrame = 0;
    this.tileAnimationBuffer = null;
    const indices = [...this.WHIRLPOOL_TILE_INDICES];
    if (this.flowerActive) indices.push(this.FLOWER_TILE_INDEX);
    if (this.waterActive) indices.push(this.WATER_TILE_INDEX);
    if (this.fountainActive) indices.push(this.FOUNTAIN_TILE_INDEX);
    if (this.lavaActive) {
      indices.push(this.LAVA_BUBBLE_TILE_1_INDEX, this.LAVA_BUBBLE_TILE_2_INDEX);
    }
    if (this.forestTreeActive) {
      indices.push(this.FOREST_TREE_LEFT_TILE_INDEX, this.FOREST_TREE_RIGHT_TILE_INDEX);
    }
    if (this.caveScrollActive) {
      indices.push(this.CAVE_WATER_TILE_INDEX, this.CAVE_WATERFALL_TILE_INDEX);
    }
    if (this.icePathScrollActive) {
      indices.push(
        this.ICE_PATH_WATER_VISIBLE_TILE_INDEX,
        this.ICE_PATH_WATERFALL_VISIBLE_TILE_INDEX
      );
    }
    if (this.towerActive) indices.push(...this.TOWER_PILLAR_TILE_DESTINATIONS);
    this.animatedTileIndices = indices;
    if (!this.active) {
      return;
    }
    if (!surface) {
      throw new Error("Tileset animation requires a valid overworld surface.");
    }
    const target = this._buildTarget({
      name: mapName,
      mapObj,
      tileset,
      surface,
      prioritySurface,
    });
    this.baseTarget = target;
    this.tilesets = [tileset];
    let dirty = this._applyFlowerFrame({ force: true });
    dirty = this._applyWaterFrame({ force: true }) || dirty;
    dirty = this._applyFountainFrame({ force: true }) || dirty;
    dirty = this._applyLavaFrames({ force: true }) || dirty;
    dirty = this._applyForestTreeFrames({ force: true }) || dirty;
    dirty = this._applyTowerFrames({ force: true }) || dirty;
    this._refreshTargetsIfNeeded(dirty);
  }

  setConnectionSegments(segments: Iterable<CompositeSegment>): void {
    this.segmentTargets = [];
    const baseTileset = this.baseTarget?.tileset ?? null;
    this.tilesets = baseTileset ? [baseTileset] : [];
    if (!this.active) {
      return;
    }
    let dirty = false;
    for (const segment of segments) {
      const tilesetName = ensureTilesetName(segment.tileset);
      if (!this.SUPPORTED_TILESETS.has(tilesetName)) {
        continue;
      }
      const target = this._buildTarget({
        name: segment.name,
        mapObj: segment.map,
        tileset: segment.tileset,
        surface: segment.surface,
        prioritySurface: segment.prioritySurface ?? segment.priority_surface ?? null,
      });
      this.segmentTargets.push(target);
      if (!this.tilesets.includes(segment.tileset)) {
        this.tilesets.push(segment.tileset);
      }
    }
    dirty = this._applyFlowerFrame({ force: true }) || dirty;
    dirty = this._applyWaterFrame({ force: true }) || dirty;
    dirty = this._applyFountainFrame({ force: true }) || dirty;
    dirty = this._applyLavaFrames({ force: true }) || dirty;
    dirty = this._applyForestTreeFrames({ force: true }) || dirty;
    dirty = this._applyTowerFrames({ force: true }) || dirty;
    this._refreshTargetsIfNeeded(dirty);
  }

  update(): void {
    const currentMs = nowMs();
    const deltaMs = Math.max(0, currentMs - this.lastTickTime);
    this.lastTickTime = currentMs;
    if (!this.active || deltaMs <= 0) {
      return;
    }
    const ticks = this.tickAccumulator + deltaMs / TILE_ANIMATION_TICK_MS;
    const wholeTicks = Math.floor(ticks);
    this.tickAccumulator = ticks - wholeTicks;
    let flowerDirty = false;
    if (wholeTicks > 0) {
      this.timer += wholeTicks;
      flowerDirty = this._applyFlowerFrame();
    }
    const waterDirty = this._applyWaterFrame();
    const fountainDirty = this._applyFountainFrame();
    const lavaDirty = this._applyLavaFrames();
    const forestTreeDirty = this._applyForestTreeFrames();
    const caveScrollDirty = this._applyCaveScrollStep();
    const towerDirty = this._applyTowerFrames();
    this._refreshTargetsIfNeeded(
      flowerDirty ||
        waterDirty ||
        fountainDirty ||
        lavaDirty ||
        forestTreeDirty ||
        caveScrollDirty ||
        towerDirty
    );
    this._applyWhirlpoolFrame();
  }

  setWhirlpoolActive(active: boolean): void {
    if (!this.active) {
      return;
    }
    if (active) {
      if (this.whirlpoolActive) {
        return;
      }
      this.whirlpoolActive = true;
      this._captureWhirlpoolBackups();
      this.fieldMoveLoader._current_frame = -1;
      this._applyWhirlpoolFrame({ force: true });
    } else {
      if (!this.whirlpoolActive) {
        return;
      }
      this.whirlpoolActive = false;
      this._restoreWhirlpoolTiles();
    }
  }

  private _applyFlowerFrame(options: { force?: boolean } = {}): boolean {
    const force = Boolean(options.force);
    if (!this.flowerActive) {
      return false;
    }
    const phase = (((this.timer / FLOWER_PHASE_TICKS) | 0) & 1) << 1;
    if (!force && phase === this.currentPhase) {
      return false;
    }
    this.currentPhase = phase;
    const frameIndex = (phase ? 2 : 0) + (this.isCgb ? 1 : 0);
    if (!force && frameIndex === this.currentFrameIndex) {
      return false;
    }
    const frames = this._flowerFrames();
    const frame = frames[frameIndex];
    if (!frame) {
      throw new Error(`Missing flower animation frame ${frameIndex}`);
    }
    this.currentFrameIndex = frameIndex;
    this._uploadTileFrame(frame, this.FLOWER_TILE_INDEX);
    return true;
  }

  private _applyWaterFrame(options: { force?: boolean } = {}): boolean {
    const force = Boolean(options.force);
    if (!this.waterActive) {
      return false;
    }
    const waterTimer = ((this.timer / this.WATER_TIMER_DIVISOR) | 0) & 0xff;
    const frameIndex = (waterTimer >> 1) & 0x03;
    if (!force && frameIndex === this.waterFrameIndex) {
      return false;
    }
    const frames = this._waterFrames();
    const frame = frames[frameIndex];
    if (!frame) {
      throw new Error(`Missing water animation frame ${frameIndex}`);
    }
    this.waterFrameIndex = frameIndex;
    this._uploadTileFrame(frame, this.WATER_TILE_INDEX);
    return true;
  }

  private _applyFountainFrame(options: { force?: boolean } = {}): boolean {
    const force = Boolean(options.force);
    if (!this.fountainActive) {
      return false;
    }
    const fountainTimer = ((this.timer / this.WATER_TIMER_DIVISOR) | 0) & 0xff;
    const sequenceIndex = fountainTimer & 0x07;
    const frameOrder = this._fountainFrameOrder();
    const frameIndex = frameOrder[sequenceIndex];
    if (!force && frameIndex === this.fountainFrameIndex) {
      return false;
    }
    const frames = this._fountainFrames();
    const frame = frames[frameIndex];
    if (!frame) {
      throw new Error(`Missing fountain animation frame ${frameIndex}`);
    }
    this.fountainFrameIndex = frameIndex;
    this._uploadTileFrame(frame, this.FOUNTAIN_TILE_INDEX);
    return true;
  }

  private _applyWhirlpoolFrame(options: { force?: boolean } = {}): void {
    if (!this.whirlpoolActive) {
      return;
    }
    if (!this.fieldMoveLoader) {
      return;
    }
    const force = Boolean(options.force);
    this._captureWhirlpoolBackups();
    const timer = this.gameState.wram.wTileAnimationTimer & 0xff;
    if (!this.fieldMoveLoader.updateWhirlpoolTiles(timer, { force })) {
      return;
    }
    const frameIndex = timer & 0x03;
    this._updateWhirlpoolTilesetSurfaces(frameIndex);
    if (!this.active) {
      return;
    }
    const dirtyRects = this._renderTargets();
    const refreshRects = this.owner.refresh_composite_surfaces;
    if (dirtyRects.size > 0 && typeof refreshRects === "function") {
      refreshRects.call(this.owner, dirtyRects);
    }
  }

  private _applyLavaFrames(options: { force?: boolean } = {}): boolean {
    const force = Boolean(options.force);
    if (!this.lavaActive) {
      return false;
    }
    const timer = this.gameState.wram.wTileAnimationTimer & 0xff;
    const frameIndex2 = (timer & 0x06) >> 1;
    const frameIndex1 = (frameIndex2 + 2) & 0x03;
    const frames = this._lavaFrames();
    let dirty = false;

    if (force || frameIndex2 !== this.lavaFrameIndex2) {
      const frame = frames[frameIndex2];
      if (!frame) {
        throw new Error(`Missing lava animation frame ${frameIndex2}`);
      }
      this.lavaFrameIndex2 = frameIndex2;
      this._uploadTileFrame(frame, this.LAVA_BUBBLE_TILE_2_INDEX);
      dirty = true;
    }

    if (force || frameIndex1 !== this.lavaFrameIndex1) {
      const frame = frames[frameIndex1];
      if (!frame) {
        throw new Error(`Missing lava animation frame ${frameIndex1}`);
      }
      this.lavaFrameIndex1 = frameIndex1;
      this._uploadTileFrame(frame, this.LAVA_BUBBLE_TILE_1_INDEX);
      dirty = true;
    }

    return dirty;
  }

  private _applyForestTreeFrames(options: { force?: boolean } = {}): boolean {
    const force = Boolean(options.force);
    if (!this.forestTreeActive) {
      return false;
    }
    const restless = Boolean(this.gameState.wram.engine_flags?.ENGINE_FOREST_IS_RESTLESS);
    const frameIndex = restless ? this.gameState.wram.wTileAnimationTimer & 1 : 0;
    if (!force && frameIndex === this.forestTreeFrameIndex) {
      return false;
    }
    const frames = this._forestTreeFrames();
    const leftFrame = frames[frameIndex];
    const rightFrame = frames[2 + frameIndex];
    if (!leftFrame || !rightFrame) {
      throw new Error(`Missing forest tree animation frame ${frameIndex}`);
    }
    this.forestTreeFrameIndex = frameIndex;
    this._uploadTileFrame(leftFrame, this.FOREST_TREE_LEFT_TILE_INDEX);
    this._uploadTileFrame(rightFrame, this.FOREST_TREE_RIGHT_TILE_INDEX);
    return true;
  }

  private _applyCaveScrollStep(options: { force?: boolean } = {}): boolean {
    if (!this.caveScrollActive && !this.icePathScrollActive) {
      return false;
    }
    const force = Boolean(options.force);
    const horizontalTile = this.icePathScrollActive
      ? this.ICE_PATH_WATER_VISIBLE_TILE_INDEX
      : this.CAVE_WATER_TILE_INDEX;
    const verticalTile = this.icePathScrollActive
      ? this.ICE_PATH_WATERFALL_VISIBLE_TILE_INDEX
      : this.CAVE_WATERFALL_TILE_INDEX;
    const horizontalVramTile = this.icePathScrollActive
      ? this.ICE_PATH_WATER_TILE_INDEX
      : this.CAVE_WATER_TILE_INDEX;
    const verticalVramTile = this.icePathScrollActive
      ? this.ICE_PATH_WATERFALL_TILE_INDEX
      : this.CAVE_WATERFALL_TILE_INDEX;
    const commandFrame = force ? 0 : this.caveScrollCommandFrame;
    this.caveScrollCommandFrame = force ? 0 : (this.caveScrollCommandFrame + 1) % 19;

    switch (commandFrame) {
      case 0:
        this.tileAnimationBuffer = this._readTileBytes(horizontalTile);
        return false;
      case 2:
        this.tileAnimationBuffer = scrollTileRightLeft(
          this.tileAnimationBuffer ?? this._readTileBytes(horizontalTile)
        );
        return false;
      case 4:
        this._uploadTileBytes(
          this.tileAnimationBuffer ?? this._readTileBytes(horizontalTile),
          horizontalTile,
          horizontalVramTile
        );
        return true;
      case 8:
        this.tileAnimationBuffer = this._readTileBytes(verticalTile);
        return false;
      case 10:
      case 12:
      case 14:
        this.tileAnimationBuffer = scrollTileDown(
          this.tileAnimationBuffer ?? this._readTileBytes(verticalTile)
        );
        return false;
      case 16:
        this._uploadTileBytes(
          this.tileAnimationBuffer ?? this._readTileBytes(verticalTile),
          verticalTile,
          verticalVramTile
        );
        return true;
      case 18:
        this.gameState.wram.wTileAnimationTimer =
          (this.gameState.wram.wTileAnimationTimer + 1) & 0xff;
        return false;
      default:
        return false;
    }
  }

  private _captureWhirlpoolBackups(): void {
    const tileBlock = resolveTileBlock(this.gameState, "vTiles2");
    for (const tileset of this.tilesets) {
      if (!tileset) {
        continue;
      }
      let tileBackup = this.whirlpoolBackups.get(tileset);
      if (!tileBackup) {
        tileBackup = new Map();
        this.whirlpoolBackups.set(tileset, tileBackup);
      }
      for (const tileIndex of this.WHIRLPOOL_TILE_INDICES) {
        if (tileBackup.has(tileIndex)) {
          continue;
        }
        if (tileIndex >= (tileset.tiles?.length ?? 0)) {
          continue;
        }
        let originalBytes: number[];
        try {
          originalBytes = tileBlock.readTile(tileIndex);
        } catch {
          continue;
        }
        const surface = tileset.tiles ? tileset.tiles[tileIndex] : null;
        if (!surface) {
          continue;
        }
        tileBackup.set(tileIndex, [originalBytes, cloneSurface(surface)]);
      }
    }
  }

  private _restoreWhirlpoolTiles(): void {
    const tileBlock = resolveTileBlock(this.gameState, "vTiles2");
    for (const [tileset, backups] of this.whirlpoolBackups.entries()) {
      for (const [tileIndex, [tileBytes, surface]] of backups.entries()) {
        if (tileset.tiles && tileIndex < tileset.tiles.length) {
          tileset.tiles[tileIndex] = cloneSurface(surface);
        }
        tileBlock.writeTile(tileIndex, tileBytes);
      }
    }
    this.whirlpoolBackups.clear();
    this._refreshTargetsIfNeeded(true);
  }

  private _applyTowerFrames(options: { force?: boolean } = {}): boolean {
    const force = Boolean(options.force);
    if (!this.towerActive) {
      return false;
    }
    const timer = this.gameState.wram.wTileAnimationTimer & 0xff;
    const towerTimer = timer >> 4;
    const patternIndex = towerTimer & 0x07;
    const frameIndex = this.TOWER_PILLAR_FRAME_OFFSETS[patternIndex];
    const frameSets = this._towerPillarFrames();
    if (frameSets.length !== this.TOWER_PILLAR_TILE_DESTINATIONS.length) {
      throw new Error("Unexpected number of tower pillar frame sets were loaded.");
    }
    if (force) {
      this.towerCommandFrame = 0;
      this.TOWER_PILLAR_TILE_DESTINATIONS.forEach((destIndex, idx) => {
        const frameSet = frameSets[idx];
        const frame = frameSet[frameIndex];
        if (!frame) {
          throw new Error(
            `Tower pillar frame ${frameIndex} missing for destination ${destIndex.toString(16)}`
          );
        }
        this._uploadTileFrame(frame, destIndex);
      });
      return true;
    }

    const commandFrame = this.towerCommandFrame & 0x0f;
    this.towerCommandFrame = (commandFrame + 1) & 0x0f;

    // ASM waits for command frames 10-15 (StandingTileFrame + waits + done).
    if (commandFrame >= this.TOWER_PILLAR_UPDATE_SEQUENCE.length) {
      return false;
    }
    const destination = this.TOWER_PILLAR_UPDATE_SEQUENCE[commandFrame];
    const frameSetIndex = this.towerFrameSetIndexByDestination.get(destination);
    if (frameSetIndex === undefined) {
      throw new Error(
        `Missing tower pillar frame-set mapping for destination ${destination.toString(16)}`
      );
    }
    const frameSet = frameSets[frameSetIndex];
    const frame = frameSet[frameIndex];
    if (!frame) {
      throw new Error(
        `Tower pillar frame ${frameIndex} missing for destination ${destination.toString(16)}`
      );
    }
    this._uploadTileFrame(frame, destination);
    return true;
  }

  private _updateWhirlpoolTilesetSurfaces(frameIndex: number): void {
    for (const tileset of this.tilesets) {
      if (!tileset.tiles) {
        continue;
      }
      this.WHIRLPOOL_TILE_INDICES.forEach((tileId) => {
        if (tileId >= tileset.tiles!.length) {
          return;
        }
        const sourceSurface = this.fieldMoveLoader.surfaceForWhirlpoolFrame(
          tileId,
          frameIndex
        );
        if (sourceSurface) {
          const reference =
            this.whirlpoolBackups.get(tileset)?.get(tileId)?.[1] ?? tileset.tiles![tileId];
          const palette = derivePaletteFromSurface(reference);
          tileset.tiles![tileId] = applyPaletteToSurface(sourceSurface, palette);
        }
      });
    }
  }

  private _uploadTileFrame(frame: TileFrame, tileIndex: number): void {
    for (const tileset of this.tilesets) {
      if (!tileset.tiles) {
        continue;
      }
      if (tileIndex >= tileset.tiles.length) {
        continue;
      }
      const reference = tileset.tiles[tileIndex];
      const palette = derivePaletteFromSurface(reference);
      tileset.tiles[tileIndex] = applyPaletteToSurface(frame.surface, palette);
      const tileBytes = tileset.tileBytes ?? tileset.tile_bytes;
      if (tileBytes) {
        if (tileIndex < tileBytes.length) {
          tileBytes[tileIndex] = frame.tileBytes;
        } else {
          tileBytes.push(frame.tileBytes);
        }
      }
    }
    const tileBlock = resolveTileBlock(this.gameState, "vTiles2");
    tileBlock.writeTile(tileIndex, frame.tileBytes);
  }

  private _uploadTileBytes(
    tileBytes: number[],
    tileIndex: number,
    vramTileIndex = tileIndex
  ): void {
    const normalizedBytes = tileBytes.slice(0, 16);
    while (normalizedBytes.length < 16) {
      normalizedBytes.push(0);
    }
    const source = decode2bppTile(Uint8Array.from(normalizedBytes));
    for (const tileset of this.tilesets) {
      if (!tileset.tiles || tileIndex >= tileset.tiles.length) {
        continue;
      }
      const reference = tileset.tiles[tileIndex];
      const palette = derivePaletteFromSurface(reference);
      tileset.tiles[tileIndex] = applyPaletteToSurface(source, palette);
      const tileBytesStore = tileset.tileBytes ?? tileset.tile_bytes;
      if (tileBytesStore) {
        tileBytesStore[tileIndex] = normalizedBytes;
      }
    }
    const tileBlock = resolveTileBlock(this.gameState, "vTiles2");
    tileBlock.writeTile(vramTileIndex, normalizedBytes);
  }

  private _readTileBytes(tileIndex: number): number[] {
    for (const tileset of this.tilesets) {
      const tileBytesStore = tileset.tileBytes ?? tileset.tile_bytes;
      const tileBytes = tileBytesStore?.[tileIndex];
      if (tileBytes?.length) {
        return tileBytes.slice(0, 16);
      }
      const surface = tileset.tiles?.[tileIndex];
      if (surface) {
        return encodeSurfaceTo2bpp(surface);
      }
    }
    return new Array(16).fill(0);
  }

  private _renderTargets(): CompositeDirtyRects {
    const targets = [this.baseTarget, ...this.segmentTargets].filter(
      (target): target is AnimatedTarget => Boolean(target)
    );
    const dirtyRegions = new Map<Surface, Array<InstanceType<typeof gameEngine.Rect>>>();
    for (const target of targets) {
      if (!target.metatileCoords.length) {
        continue;
      }
      target.renderer._subtile_cache.clear();
      target.renderer._priority_subtile_cache.clear();
      target.renderer._metatile_cache.clear();
      const rects: Array<InstanceType<typeof gameEngine.Rect>> = [];
      for (const [metatileX, metatileY] of target.metatileCoords) {
        const rect = this._blitMetatile(target, metatileX, metatileY);
        if (rect) {
          rects.push(rect);
        }
      }
      if (!rects.length) {
        continue;
      }
      const existing = dirtyRegions.get(target.surface) ?? [];
      existing.push(...rects);
      dirtyRegions.set(target.surface, existing);
      if (target.prioritySurface) {
        const priorityRects = dirtyRegions.get(target.prioritySurface) ?? [];
        priorityRects.push(...rects);
        dirtyRegions.set(target.prioritySurface, priorityRects);
      }
    }
    return dirtyRegions;
  }

  private _refreshTargetsIfNeeded(dirty: boolean): void {
    if (!dirty || !this.active) {
      return;
    }
    const dirtyRects = this._renderTargets();
    const refreshRects = this.owner.refresh_composite_surfaces;
    if (dirtyRects.size > 0 && typeof refreshRects === "function") {
      refreshRects.call(this.owner, dirtyRects);
    }
  }

  private _blitMetatile(
    target: AnimatedTarget,
    metatileX: number,
    metatileY: number
  ): InstanceType<typeof gameEngine.Rect> | null {
    const width = target.map.width;
    if (width <= 0) {
      throw new Error(`Map '${target.name}' has invalid width ${width}`);
    }
    const index = metatileY * width + metatileX;
    if (index < 0 || index >= target.map.metatileIds.length) {
      return null;
    }
    const metatileId = target.map.metatileIds[index];
    const metatiles = target.tileset.metatiles as
      | readonly RenderableMetatile[]
      | undefined;
    if (!metatiles || metatileId < 0 || metatileId >= metatiles.length) {
      return null;
    }
    const metatile = metatiles[metatileId];
    const baseX = metatileX * METATILE_SIZE;
    const baseY = metatileY * METATILE_SIZE;
    target.renderer._blit_metatile(metatile, baseX, baseY);
    return new gameEngine.Rect(baseX, baseY, METATILE_SIZE, METATILE_SIZE);
  }

  private _buildTarget(options: {
    name: string;
    mapObj: OverworldMap;
    tileset: TilesetLike;
    surface: Surface;
    prioritySurface: Surface | null;
  }): AnimatedTarget {
    const renderer = new TileRenderer(
      options.tileset,
      options.surface,
      options.prioritySurface
    );
    const coords = this._locateAnimatedMetatiles(
      options.mapObj,
      options.tileset
    );
    return {
      name: options.name,
      surface: options.surface,
      prioritySurface: options.prioritySurface,
      map: options.mapObj,
      tileset: options.tileset,
      renderer,
      metatileCoords: coords,
    };
  }

  private _locateAnimatedMetatiles(
    mapObj: OverworldMap,
    tileset: TilesetLike
  ): Array<[number, number]> {
    const width = mapObj.width;
    if (width <= 0) {
      throw new Error(`Map '${mapObj.mapName}' width must be positive.`);
    }
    const metatiles = tileset.metatiles as readonly RenderableMetatile[] | undefined;
    if (!metatiles) {
      throw new Error("Tileset animation requires renderable metatiles.");
    }
    const coords: Array<[number, number]> = [];
    mapObj.metatileIds.forEach((metatileId, index) => {
      if (metatileId < 0 || metatileId >= metatiles.length) {
        return;
      }
      const metatile = metatiles[metatileId];
      if (this._metatileHasAnimatedTile(metatile)) {
        coords.push([index % width, Math.floor(index / width)]);
      }
    });
    return coords;
  }

  private _metatileHasAnimatedTile(metatile: RenderableMetatile): boolean {
    for (const row of metatile.tiles) {
      for (const entry of row) {
        const tileIndex = resolveTileIndex(entry);
        if (tileIndex !== null && this.animatedTileIndices.includes(tileIndex)) {
          return true;
        }
      }
    }
    return false;
  }

  private _waterFrames(): TileFrame[] {
    if (this._waterFrameCache) {
      return this._waterFrameCache;
    }
    const frames = loadFramesFrom2bpp(WATER_ASSET_PATH, 4);
    this._waterFrameCache = frames;
    return frames;
  }

  private _flowerFrames(): TileFrame[] {
    if (this._flowerFrameCache) {
      return this._flowerFrameCache;
    }
    const frames: TileFrame[] = [];
    for (const filename of FRAME_ORDER) {
      const filePath = joinPath(FLOWER_ASSET_DIR, filename);
      const data = fs.readFileSync(filePath);
      if (data.length !== 16) {
        throw new Error(
          `Flower tile '${filename}' must be exactly 16 bytes, got ${data.length}`
        );
      }
      frames.push({
        tileBytes: Array.from(data),
        surface: decode2bppTile(data),
      });
    }
    this._flowerFrameCache = frames;
    return frames;
  }

  private _towerPillarFrames(): TileFrame[][] {
    if (this._towerFrameCache) {
      return this._towerFrameCache;
    }
    const bytesPerTile = TILE_SIZE * 2;
    const expectedLength = this.TOWER_PILLAR_FRAME_COUNT * bytesPerTile;
    const frameSets: TileFrame[][] = [];
    for (const filename of this.TOWER_PILLAR_FILE_NAMES) {
      const filePath = joinPath(TOWER_PILLAR_ASSET_DIR, filename);
      if (!assetExists(filePath)) {
        throw new Error(`Missing tower pillar asset: ${filePath}`);
      }
      const data = fs.readFileSync(filePath);
      if (data.length !== expectedLength) {
        throw new Error(
          `${filename} must contain ${expectedLength} bytes, got ${data.length}`
        );
      }
      const frames: TileFrame[] = [];
      for (let index = 0; index < this.TOWER_PILLAR_FRAME_COUNT; index += 1) {
        const start = index * bytesPerTile;
        const chunk = Array.from(data.slice(start, start + bytesPerTile));
        frames.push({
          tileBytes: chunk,
          surface: decode2bppTile(data.slice(start, start + bytesPerTile)),
        });
      }
      frameSets.push(frames);
    }
    this._towerFrameCache = frameSets;
    return frameSets;
  }

  private _fountainFrames(): TileFrame[] {
    if (this._fountainFrameCache) {
      return this._fountainFrameCache;
    }
    const frames: TileFrame[] = [];
    for (let index = 1; index <= 5; index += 1) {
      const filename = `${index}.2bpp`;
      const filePath = joinPath(FOUNTAIN_ASSET_DIR, filename);
      const data = fs.readFileSync(filePath);
      if (data.length !== TILE_SIZE * 2) {
        throw new Error(
          `Fountain tile '${filename}' must be exactly 16 bytes, got ${data.length}`
        );
      }
      frames.push({
        tileBytes: Array.from(data),
        surface: decode2bppTile(data),
      });
    }
    this._fountainFrameCache = frames;
    return frames;
  }

  private _lavaFrames(): TileFrame[] {
    if (this._lavaFrameCache) {
      return this._lavaFrameCache;
    }
    const frames: TileFrame[] = [];
    for (let index = 1; index <= 4; index += 1) {
      const filename = `${index}.2bpp`;
      const filePath = joinPath(LAVA_ASSET_DIR, filename);
      const data = fs.readFileSync(filePath);
      if (data.length !== TILE_SIZE * 2) {
        throw new Error(
          `Lava tile '${filename}' must be exactly 16 bytes, got ${data.length}`
        );
      }
      frames.push({
        tileBytes: Array.from(data),
        surface: decode2bppTile(data),
      });
    }
    this._lavaFrameCache = frames;
    return frames;
  }

  private _forestTreeFrames(): TileFrame[] {
    if (this._forestTreeFrameCache) {
      return this._forestTreeFrameCache;
    }
    const frames: TileFrame[] = [];
    for (let index = 1; index <= 4; index += 1) {
      const filename = `${index}.2bpp`;
      const filePath = joinPath(FOREST_TREE_ASSET_DIR, filename);
      const data = fs.readFileSync(filePath);
      if (data.length !== TILE_SIZE * 2) {
        throw new Error(
          `Forest tree tile '${filename}' must be exactly 16 bytes, got ${data.length}`
        );
      }
      frames.push({
        tileBytes: Array.from(data),
        surface: decode2bppTile(data),
      });
    }
    this._forestTreeFrameCache = frames;
    return frames;
  }

  private _fountainFrameOrder(): number[] {
    return [0, 1, 2, 3, 2, 3, 4, 0];
  }
}

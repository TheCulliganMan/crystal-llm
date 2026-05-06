import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import {
  getAssetPath,
  getTilesetCollisionPath,
  getTilesetMetatilesPath,
  getTilesetPaletteMapJsonPath,
} from "@pokecrystal/core/core/paths";
import { METATILE_WIDTH, TILE_SIZE } from "@pokecrystal/core/engine/world/tile/constants";
import { resolveCollisionValue } from "@pokecrystal/core/engine/world/overworld/collision-data";
import { assetExists } from "@pokecrystal/core/core/asset-manifest";
import { pushDebugLog } from "@pokecrystal/core/core/debug-log";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";
import type { RenderMetatileOptions } from "./tileset-types";

type MetatileEntry = {
  tiles: Array<Array<{
    tileIndex: number;
    priority?: boolean;
    priorityClipTop?: number;
  }>>;
  collision: number[];
};

export type Palette = [number, number, number][];

const DEFAULT_COLLISION = resolveCollisionValue("FLOOR");
const METATILE_BYTES = METATILE_WIDTH * METATILE_WIDTH;
const PALETTE_LABELS: Record<string, number> = {
  GRAY: 0,
  RED: 1,
  GREEN: 2,
  WATER: 3,
  YELLOW: 4,
  BROWN: 5,
  ROOF: 6,
  TEXT: 7,
};

const PRIORITY_COLLISION_TOKENS = [
  "TALL_GRASS",
  "LONG_GRASS",
  "LONG_GRASS_1C",
  "GRASS_48",
  "GRASS_49",
  "GRASS_4A",
  "GRASS_4B",
  "GRASS_4C",
  "BOOKSHELF",
  "COUNTER",
  "COUNTER_98",
  "INCENSE_BURNER",
  "MART_SHELF",
  "PC",
];
const PRIORITY_COLLISIONS = new Set(
  PRIORITY_COLLISION_TOKENS.map((token) => resolveCollisionValue(token))
);
const WALL_COLLISION = resolveCollisionValue("WALL");

const paletteMapCache = new Map<string, number[]>();
const paletteBankCache = new Map<string, Palette[]>();
const collisionMapCache = new Map<string, Map<number, number[]>>();

const buildTileSurface = (tileIndex: number): InstanceType<typeof gameEngine.Surface> => {
  const surface = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
  const shade = tileIndex & 0xff;
  surface.fill([shade, shade, shade, 255]);
  return surface;
};

const buildFallbackMetatiles = (count: number): MetatileEntry[] => {
  const rows = METATILE_WIDTH;
  const tiles: Array<Array<{ tileIndex: number }>> = [];
  for (let row = 0; row < rows; row += 1) {
    const line: Array<{ tileIndex: number }> = [];
    for (let col = 0; col < rows; col += 1) {
      line.push({ tileIndex: 0 });
    }
    tiles.push(line);
  }
  const collision = [DEFAULT_COLLISION, DEFAULT_COLLISION, DEFAULT_COLLISION, DEFAULT_COLLISION];
  return Array.from({ length: count }, () => ({
    tiles: tiles.map((row) => row.map((entry) => ({ ...entry }))),
    collision: [...collision],
  }));
};

const parseRgbTriples = (line: string): number[] => {
  const cleaned = line.replace(/RGB/gi, "").replace(/,/g, " ").trim();
  if (!cleaned) {
    return [];
  }
  return cleaned
    .split(/\s+/)
    .map((value) => Number(value))
    .filter((value) => !Number.isNaN(value));
};

const gbToRgb = (value: number): number => gbc5To8(value);

const normalizePaletteValue = (value: number): number => {
  if (value >= 0 && value <= 31) {
    return gbToRgb(value);
  }
  return value;
};

export const parsePaletteFile = (content: string): Palette[] => {
  const palettes: Palette[] = [];
  const buffer: number[][] = [];
  for (const rawLine of content.split(/\r?\n/)) {
    const trimmed = rawLine.split(";")[0].trim();
    if (!trimmed.toUpperCase().startsWith("RGB")) {
      continue;
    }
    const values = parseRgbTriples(trimmed);
    if (values.length % 3 !== 0) {
      throw new Error(`Malformed RGB entry '${trimmed}' in palette file.`);
    }
    const triples: Palette = [];
    for (let i = 0; i < values.length; i += 3) {
      triples.push([
        normalizePaletteValue(values[i]),
        normalizePaletteValue(values[i + 1]),
        normalizePaletteValue(values[i + 2]),
      ]);
    }
    if (triples.length === 4) {
      palettes.push(triples);
    } else if (triples.length === 1) {
      buffer.push(triples[0]);
      if (buffer.length === 4) {
        palettes.push(buffer.splice(0, 4) as Palette);
      }
    } else {
      throw new Error(`Unexpected RGB triple count ${triples.length} in palette file.`);
    }
  }
  if (buffer.length) {
    throw new Error("Palette file ended with incomplete RGB entries.");
  }
  return palettes;
};

const parseGroupedPaletteFile = (content: string): Record<string, Palette[]> => {
  const groups: Record<string, Palette[]> = {};
  let currentGroup = "default";
  const buffer: Record<string, number[][]> = {};
  for (const rawLine of content.split(/\r?\n/)) {
    const commentOnly = rawLine.trim().startsWith(";") && !rawLine.toUpperCase().includes("RGB");
    if (commentOnly) {
      const label = rawLine.replace(/^;\s*/, "").trim();
      if (label) {
        currentGroup = label.toLowerCase().replace(/\s+/g, "_");
      }
      continue;
    }
    const trimmed = rawLine.split(";")[0].trim();
    if (!trimmed.toUpperCase().startsWith("RGB")) {
      continue;
    }
    const values = parseRgbTriples(trimmed);
    if (values.length % 3 !== 0) {
      throw new Error(`Malformed RGB entry '${trimmed}' in palette file.`);
    }
    const triples: Palette = [];
    for (let i = 0; i < values.length; i += 3) {
      triples.push([
        normalizePaletteValue(values[i]),
        normalizePaletteValue(values[i + 1]),
        normalizePaletteValue(values[i + 2]),
      ]);
    }
    if (!groups[currentGroup]) {
      groups[currentGroup] = [];
    }
    if (!buffer[currentGroup]) {
      buffer[currentGroup] = [];
    }
    if (triples.length === 4) {
      groups[currentGroup].push(triples);
    } else if (triples.length === 1) {
      buffer[currentGroup].push(triples[0]);
      if (buffer[currentGroup].length === 4) {
        groups[currentGroup].push(buffer[currentGroup].splice(0, 4) as Palette);
      }
    } else {
      throw new Error(`Unexpected RGB triple count ${triples.length} in palette file.`);
    }
  }
  for (const [group, pending] of Object.entries(buffer)) {
    if (pending.length) {
      throw new Error(`Palette group '${group}' ended with incomplete RGB entries.`);
    }
  }
  return groups;
};

const normalizeTimeOfDay = (value: string | null): string => {
  const normalized = String(value || "day").toLowerCase();
  if (normalized === "night") {
    return "nite";
  }
  return normalized;
};

export const getTilesetMetatilesCandidatePaths = (tilesetName: string): string[] => {
  const primaryPath = getTilesetMetatilesPath(tilesetName);
  const runtimePath = getAssetPath("data", "tilesets", `${tilesetName}_metatiles.bin`);
  return Array.from(new Set([primaryPath, runtimePath]));
};

// ASM mapping: gfx/tilesets/*_palette_map.asm encodes palette + VRAM bank in the lower nibble.
export const parseTilesetPaletteMap = (content: string): number[] => {
  const paletteIndices: number[] = [];
  let repeatCount = 0;
  let repeatValues: number[] | null = null;

  const pushValues = (values: number[]) => {
    // Palette map bytes pack two tile nibbles (see gfx/tileset_palette_maps.asm tilepal).
    for (const value of values) {
      const low = value & 0x0f;
      const high = (value >> 4) & 0x0f;
      paletteIndices.push(low, high);
    }
  };

  for (const rawLine of content.split(/\r?\n/)) {
    const trimmed = rawLine.split(";")[0].trim();
    if (!trimmed) {
      continue;
    }
    const lower = trimmed.toLowerCase();
    if (lower.startsWith("rept")) {
      const parts = trimmed.split(/\s+/);
      repeatCount = Number(parts[1] ?? 0);
      repeatValues = null;
      continue;
    }
    if (lower.startsWith("endr")) {
      if (repeatCount > 0 && repeatValues) {
        for (let i = 0; i < repeatCount; i += 1) {
          pushValues(repeatValues);
        }
      }
      repeatCount = 0;
      repeatValues = null;
      continue;
    }
    if (lower.startsWith("db")) {
      const values = trimmed
        .replace(/^db/i, "")
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean)
        .map((value) => Number(value.replace("$", "0x")));
      if (values.some((value) => Number.isNaN(value))) {
        throw new Error(`Invalid palette byte line '${trimmed}'.`);
      }
      if (repeatCount > 0) {
        repeatValues = values;
      } else {
        pushValues(values);
      }
      continue;
    }
    if (lower.startsWith("tilepal")) {
      const tokens = trimmed
        .replace(/^tilepal/i, "")
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean);
      if (tokens.length < 2) {
        continue;
      }
      const bank = Number(tokens[0] ?? 0);
      const paletteTokens = tokens.slice(1);
      const indices = paletteTokens.map((token) => {
        const key = token.toUpperCase();
        const index = PALETTE_LABELS[key];
        if (index === undefined) {
          throw new Error(`Unknown palette token '${token}'.`);
        }
        return ((Number.isNaN(bank) ? 0 : bank) << 3) | index;
      });
      paletteIndices.push(...indices);
    }
  }

  return paletteIndices;
};

export const buildMetatilesFromLayout = (layout: Uint8Array): MetatileEntry[] => {
  if (layout.length % METATILE_BYTES !== 0) {
    throw new Error(`Tileset metatile layout has invalid length ${layout.length}.`);
  }
  const count = layout.length / METATILE_BYTES;
  const metatiles: MetatileEntry[] = [];
  let offset = 0;
  for (let index = 0; index < count; index += 1) {
    const tiles: Array<Array<{ tileIndex: number; priority?: boolean }>> = [];
    for (let row = 0; row < METATILE_WIDTH; row += 1) {
      const line: Array<{ tileIndex: number; priority?: boolean }> = [];
      for (let col = 0; col < METATILE_WIDTH; col += 1) {
        const tileIndex = layout[offset++];
        line.push({ tileIndex });
      }
      tiles.push(line);
    }
    metatiles.push({
      tiles,
      collision: [DEFAULT_COLLISION, DEFAULT_COLLISION, DEFAULT_COLLISION, DEFAULT_COLLISION],
    });
  }
  return metatiles;
};

const parseTilesetCollisionPayload = (payload: unknown): Map<number, number[]> => {
  const collisions = new Map<number, number[]>();
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    throw new Error("Tileset collision payload must be an object.");
  }
  for (const [key, rawValue] of Object.entries(payload as Record<string, unknown>)) {
    const tileIndex = Number.parseInt(key, 16);
    if (Number.isNaN(tileIndex)) {
      continue;
    }
    if (!Array.isArray(rawValue)) {
      continue;
    }
    const tokens = rawValue.map((value) => String(value).trim()).filter(Boolean);
    if (tokens.length !== 4) {
      throw new Error(`Expected 4 collision tokens for ${tileIndex.toString(16)}, got ${tokens.length}.`);
    }
    const values = tokens.map((token) => resolveCollisionValue(token));
    collisions.set(tileIndex, values);
  }
  return collisions;
};

const canFetchAsset = (assetPath: string): boolean => {
  if (typeof window !== "undefined") {
    return true;
  }
  return /^https?:\/\//i.test(assetPath);
};

const loadText = async (assetPath: string): Promise<string> => {
  if (canFetchAsset(assetPath)) {
    if (!assetExists(assetPath)) {
      const message = `Missing asset (manifest): ${assetPath}`;
      pushDebugLog(`[assets] ${message}`);
      throw new Error(message);
    }
    const response = await fetch(assetPath);
    if (!response.ok) {
      const message = `Failed to load tileset text (${response.status} ${response.statusText}): ${assetPath}`;
      pushDebugLog(`[assets] ${message}`);
      throw new Error(message);
    }
    return await response.text();
  }
  const { promises: fs } = await import("fs");
  return await fs.readFile(assetPath, "utf8");
};

const loadBinary = async (assetPath: string): Promise<Uint8Array> => {
  if (canFetchAsset(assetPath)) {
    if (!assetExists(assetPath)) {
      const message = `Missing asset (manifest): ${assetPath}`;
      pushDebugLog(`[assets] ${message}`);
      throw new Error(message);
    }
    const response = await fetch(assetPath);
    if (!response.ok) {
      const message = `Failed to load tileset data (${response.status} ${response.statusText}): ${assetPath}`;
      pushDebugLog(`[assets] ${message}`);
      throw new Error(message);
    }
    const buffer = await response.arrayBuffer();
    return new Uint8Array(buffer);
  }
  const { promises: fs } = await import("fs");
  const raw = await fs.readFile(assetPath);
  return new Uint8Array(raw);
};

const loadBinaryFromCandidates = async (
  candidatePaths: string[]
): Promise<{ bytes: Uint8Array; path: string }> => {
  let lastError: unknown = null;
  for (const candidatePath of candidatePaths) {
    try {
      return {
        bytes: await loadBinary(candidatePath),
        path: candidatePath,
      };
    } catch (error: unknown) {
      lastError = error;
    }
  }
  throw lastError ?? new Error(`Missing asset from candidates: ${candidatePaths.join(", ")}`);
};

const loadTilesetCollisionMap = async (tilesetName: string): Promise<Map<number, number[]>> => {
  const cached = collisionMapCache.get(tilesetName);
  if (cached) {
    return cached;
  }
  const collisionPath = getTilesetCollisionPath(tilesetName);
  try {
    const content = await loadText(collisionPath);
    const parsed = parseTilesetCollisionPayload(JSON.parse(content));
    collisionMapCache.set(tilesetName, parsed);
    return parsed;
  } catch {
    const empty = new Map<number, number[]>();
    collisionMapCache.set(tilesetName, empty);
    return empty;
  }
};

const loadTilesetSurface = async (tilesetName: string): Promise<InstanceType<typeof gameEngine.Surface>> => {
  const tilesetPath = getAssetPath("gfx", "tilesets", `${tilesetName}.png`);
  if (typeof window !== "undefined" && !assetExists(tilesetPath)) {
    const message = `Missing tileset image (manifest): ${tilesetPath}`;
    pushDebugLog(`[assets] ${message}`);
    throw new Error(message);
  }
  return gameEngine.image.load(tilesetPath);
};

const sliceTileset = (source: InstanceType<typeof gameEngine.Surface>): InstanceType<typeof gameEngine.Surface>[] => {
  const width = source.get_width();
  const height = source.get_height();
  if (width % TILE_SIZE || height % TILE_SIZE) {
    throw new Error(`Tileset has unexpected dimensions ${width}x${height}.`);
  }
  const tiles: InstanceType<typeof gameEngine.Surface>[] = [];
  for (let y = 0; y < height; y += TILE_SIZE) {
    for (let x = 0; x < width; x += TILE_SIZE) {
      const tile = source.subsurface(new gameEngine.Rect(x, y, TILE_SIZE, TILE_SIZE));
      tiles.push(tile);
    }
  }
  return tiles;
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

const applyPaletteToSurface = (
  source: InstanceType<typeof gameEngine.Surface>,
  palette: Palette
): InstanceType<typeof gameEngine.Surface> => {
  const [width, height] = source.get_size();
  const target = new gameEngine.Surface(width, height);
  const image = source.getImageData();
  const data = image.data;

  for (let i = 0; i < data.length; i += 4) {
    const alpha = data[i + 3];
    if (alpha === 0) {
      continue;
    }
    const paletteIndex = paletteIndexFromGray(data[i]);
    const [r, g, b] = palette[paletteIndex] ?? palette[0];
    data[i] = r;
    data[i + 1] = g;
    data[i + 2] = b;
    data[i + 3] = 255;
  }
  target.getContext()!.putImageData(image, 0, 0);
  return target;
};

const applyPriorityOverlayToSurface = (
  source: InstanceType<typeof gameEngine.Surface>,
  palette: Palette
): InstanceType<typeof gameEngine.Surface> => {
  const [width, height] = source.get_size();
  const target = new gameEngine.Surface(width, height);
  const image = source.getImageData();
  const data = image.data;

  for (let i = 0; i < data.length; i += 4) {
    const alpha = data[i + 3];
    if (alpha === 0) {
      continue;
    }
    const paletteIndex = paletteIndexFromGray(data[i]);
    const [r, g, b] = palette[paletteIndex] ?? palette[0];
    data[i] = r;
    data[i + 1] = g;
    data[i + 2] = b;
    data[i + 3] = paletteIndex === 0 ? 0 : 255;
  }
  target.getContext()!.putImageData(image, 0, 0);
  return target;
};

const applyPriorityFlags = (metatiles: MetatileEntry[]): void => {
  for (const metatile of metatiles) {
    const collision = metatile.collision ?? [];
    const hasForegroundBottom =
      collision[0] === DEFAULT_COLLISION &&
      collision[1] === DEFAULT_COLLISION &&
      (collision[2] === WALL_COLLISION || PRIORITY_COLLISIONS.has(collision[2])) &&
      (collision[3] === WALL_COLLISION || PRIORITY_COLLISIONS.has(collision[3]));
    for (let rowIndex = 0; rowIndex < metatile.tiles.length; rowIndex += 1) {
      const row = metatile.tiles[rowIndex] ?? [];
      for (let colIndex = 0; colIndex < row.length; colIndex += 1) {
        const entry = row[colIndex];
        if (!entry) {
          continue;
        }
        const collisionIndex = (rowIndex < 2 ? 0 : 2) + (colIndex < 2 ? 0 : 1);
        const foregroundBottomPriority = hasForegroundBottom && rowIndex >= 1;
        entry.priority =
          PRIORITY_COLLISIONS.has(collision[collisionIndex]) ||
          foregroundBottomPriority;
        entry.priorityClipTop = foregroundBottomPriority && rowIndex === 1 ? TILE_SIZE / 2 : 0;
      }
    }
  }
};

export const resolveTilesetTile = (
  tiles: InstanceType<typeof gameEngine.Surface>[],
  tileIndex: number,
  vramBank: number
): InstanceType<typeof gameEngine.Surface> => {
  const base = tiles[tileIndex] ?? tiles[0];
  if (vramBank !== 1) {
    return base;
  }

  if (tiles.length % 2 === 0) {
    const half = tiles.length / 2;
    // ASM parity: engine/tilesets/map_palettes.asm looks up palettes/bank from
    // the raw metatile byte, then clears bit 7 before writing the BG tilemap.
    // PNG exports store the two VRAM banks as equal contiguous halves.
    const candidate = tiles[(tileIndex & 0x7f) + half];
    if (candidate) {
      return candidate;
    }
  }

  const bankOneTile = tiles[(tileIndex & 0x7f) + 0x80];
  if (bankOneTile) {
    return bankOneTile;
  }

  // ASM-faithful fallback for packed tileset exports where bank-1 tiles use
  // the high tile-id range (0x80-0xff) but are stored at lower indices.
  if (tileIndex >= 0x80) {
    const mirrored = tiles[tileIndex - 0x80];
    if (mirrored) {
      return mirrored;
    }
  }

  return base;
};

const loadTilesetPaletteMap = async (tilesetName: string): Promise<number[] | null> => {
  const cached = paletteMapCache.get(tilesetName);
  if (cached) {
    return cached;
  }
  const path = getTilesetPaletteMapJsonPath(tilesetName);
  try {
    const content = await loadText(path);
    const parsed = JSON.parse(content);
    if (!Array.isArray(parsed) || parsed.some((value) => typeof value !== "number")) {
      throw new Error(`Tileset palette map ${path} must be a number array.`);
    }
    paletteMapCache.set(tilesetName, parsed);
    return parsed;
  } catch {
    return null;
  }
};

const loadTilesetPaletteBank = async (
  tilesetName: string,
  timeOfDay: string | null
): Promise<Palette[] | null> => {
  const cacheKey = `${tilesetName}:${normalizeTimeOfDay(timeOfDay)}`;
  const cached = paletteBankCache.get(cacheKey);
  if (cached) {
    return cached;
  }
  const tilesetPalPath = getAssetPath("gfx", "tilesets", `${tilesetName}.pal`);
  if (assetExists(tilesetPalPath)) {
    try {
      const content = await loadText(tilesetPalPath);
      const palettes = parsePaletteFile(content).slice(0, 8);
      if (palettes.length) {
        paletteBankCache.set(cacheKey, palettes);
        return palettes;
      }
    } catch (error) {
      pushDebugLog(`[tileset] failed to load ${tilesetName}.pal; falling back to bg_tiles.pal`, {
        error: String(error),
      });
    }
  }
  const bgTilesPath = getAssetPath("gfx", "tilesets", "bg_tiles.pal");
  try {
    const content = await loadText(bgTilesPath);
    const groups = parseGroupedPaletteFile(content);
    const groupKey = normalizeTimeOfDay(timeOfDay) || "day";
    const palettes = groups[groupKey] ?? groups.day ?? groups.morn ?? null;
    if (palettes?.length) {
      const bank = palettes.slice(0, 8);
      paletteBankCache.set(cacheKey, bank);
      return bank;
    }
  } catch {
    return null;
  }
  return null;
};

export class OverworldTileset {
  public readonly tilesetName: string;
  public metatiles: MetatileEntry[];
  public tiles: InstanceType<typeof gameEngine.Surface>[];
  private _priority_tiles: InstanceType<typeof gameEngine.Surface>[];
  public readonly ready: Promise<void>;
  public loaded: boolean;
  private readonly _timeOfDay: string | null;

  constructor(tilesetName: string, timeOfDay?: string | null, options: { skipLoad?: boolean } = {}) {
    this.tilesetName = String(tilesetName || "unknown").trim() || "unknown";
    this._timeOfDay = timeOfDay ?? null;
    const metatileCount = 0x100;
    this.metatiles = buildFallbackMetatiles(metatileCount);
    this.tiles = Array.from({ length: 0x100 }, (_, index) => buildTileSurface(index));
    this._priority_tiles = [...this.tiles];
    this.loaded = Boolean(options.skipLoad);
    this.ready = options.skipLoad
      ? Promise.resolve()
      : this._load().then(() => {
        this.loaded = true;
      });
  }

  public renderMetatile = (
    metatileId: number,
    target: InstanceType<typeof gameEngine.Surface>,
    x: number,
    y: number,
    _options?: RenderMetatileOptions
  ): void => {
    const metatile = this.metatiles[metatileId];
    if (!metatile) {
      throw new Error(`Metatile ${metatileId} not found in tileset '${this.tilesetName}'.`);
    }
    for (let row = 0; row < metatile.tiles.length; row += 1) {
      const rowTiles = metatile.tiles[row] ?? [];
      for (let col = 0; col < rowTiles.length; col += 1) {
        const tileIndex = rowTiles[col]?.tileIndex ?? 0;
        const tileSurface = this.tiles[tileIndex] ?? this.tiles[0];
        target.blit(tileSurface, [x + col * TILE_SIZE, y + row * TILE_SIZE]);
      }
    }
  };

  public renderPriorityMetatile = (
    metatileId: number,
    target: InstanceType<typeof gameEngine.Surface>,
    x: number,
    y: number
  ): void => {
    const metatile = this.metatiles[metatileId];
    if (!metatile) {
      throw new Error(`Metatile ${metatileId} not found in tileset '${this.tilesetName}'.`);
    }
    for (let row = 0; row < metatile.tiles.length; row += 1) {
      const rowTiles = metatile.tiles[row] ?? [];
      for (let col = 0; col < rowTiles.length; col += 1) {
        const entry = rowTiles[col];
        if (!entry?.priority) {
          continue;
        }
        const tileIndex = entry.tileIndex ?? 0;
        const tileSurface = this._priority_tiles[tileIndex] ?? this.tiles[tileIndex] ?? this.tiles[0];
        const clipTop = Math.max(0, Math.min(TILE_SIZE - 1, Math.trunc(entry.priorityClipTop ?? 0)));
        const destY = y + row * TILE_SIZE + clipTop;
        const area = clipTop > 0
          ? { x: 0, y: clipTop, width: TILE_SIZE, height: TILE_SIZE - clipTop }
          : undefined;
        target.blit(tileSurface, [x + col * TILE_SIZE, destY], area);
      }
    }
  };

  private async _load(): Promise<void> {
    pushDebugLog(`[tileset] load ${this.tilesetName}`, { timeOfDay: this._timeOfDay ?? "default" });
    const layoutPaths = getTilesetMetatilesCandidatePaths(this.tilesetName);
    const layoutPromise = loadBinaryFromCandidates(layoutPaths).catch((error) => {
      const message =
        `Tileset '${this.tilesetName}' missing metatile layout (${layoutPaths.join(" or ")}). ` +
        `In dev, ensure runtime assets are available under \`/assets/data/tilesets\` or rerun \`npm run build\`.`;
      pushDebugLog(`[tileset] ${message}`, { error: String(error) });
      throw new Error(message);
    });
    const collisionPromise = loadTilesetCollisionMap(this.tilesetName);
    const surfacePromise = loadTilesetSurface(this.tilesetName);
    const paletteMapPromise = loadTilesetPaletteMap(this.tilesetName);
    const paletteBankPromise = loadTilesetPaletteBank(this.tilesetName, this._timeOfDay);
    const [layoutResult, collisionMap, surface, paletteMap, paletteBank] = await Promise.all([
      layoutPromise,
      collisionPromise,
      surfacePromise,
      paletteMapPromise,
      paletteBankPromise,
    ]);
    const metatiles = buildMetatilesFromLayout(layoutResult.bytes);
    if (collisionMap.size) {
      for (const [metatileId, collision] of collisionMap.entries()) {
        if (!metatiles[metatileId]) {
          continue;
        }
        const padded = collision.slice(0, 4);
        while (padded.length < 4) {
          padded.push(DEFAULT_COLLISION);
        }
        metatiles[metatileId].collision = padded;
      }
    }
    applyPriorityFlags(metatiles);
    const tiles = sliceTileset(surface);
    let finalTiles = tiles;
    let priorityTiles = tiles;
    if (paletteMap && paletteBank && paletteBank.length) {
      finalTiles = [];
      priorityTiles = [];
      const renderableTileCount = Math.max(tiles.length, paletteMap.length);
      for (let index = 0; index < renderableTileCount; index += 1) {
        const paletteValue = paletteMap[index] ?? 0;
        const paletteIndex = paletteValue & 0x07;
        const vramBank = (paletteValue >> 3) & 0x01;
        const sourceTile = resolveTilesetTile(tiles, index, vramBank);
        const palette = paletteBank[paletteIndex] ?? paletteBank[0];
        finalTiles.push(applyPaletteToSurface(sourceTile, palette));
        priorityTiles.push(applyPriorityOverlayToSurface(sourceTile, palette));
      }
    } else if (paletteBank && paletteBank.length) {
      priorityTiles = [];
      const palette = paletteBank[0];
      for (let index = 0; index < tiles.length; index += 1) {
        const sourceTile = resolveTilesetTile(tiles, index, 0);
        priorityTiles.push(applyPriorityOverlayToSurface(sourceTile, palette));
      }
    }
    this.metatiles = metatiles;
    this.tiles = finalTiles;
    this._priority_tiles = priorityTiles;
  }
}

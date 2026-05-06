// ASM mapping: pokecrystal_disassembly/engine/menus/bills_pc.asm (_CGB_BillsPC) wallpaper assets.
import fs from "fs";
import path from "path";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { Surface } from "@pokecrystal/core/ui/surface";
import { BILLS_PC_ATTR_BANDS, TileRegion } from "./pc-layout";
import { SPACE_TILE, TilemapSurface } from "@pokecrystal/core/ui/tilemap-surface";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";
import { buildDefaultCharMap } from "@pokecrystal/core/ui/text/glyph-map";

const TILE_SIZE = 8;
const CHAR_MAP = buildDefaultCharMap();

type TilePixels = number[];
type Palette = [number, number, number][];

const PC_ASSET_DIR = getAssetPath("gfx", "pc");
const PC_TILESET = path.join(PC_ASSET_DIR, "pc.2bpp");
const PC_MAIL_TILESET = path.join(PC_ASSET_DIR, "pc_mail.2bpp");
const PC_PALETTE = path.join(PC_ASSET_DIR, "orange.pal");
const PC_TILESET_PNG = path.join(PC_ASSET_DIR, "pc.png");
const PC_MAIL_TILESET_PNG = path.join(PC_ASSET_DIR, "pc_mail.png");

const WALLPAPER_ROWS = 7;
const WALLPAPER_COLUMNS = 7;
const WALLPAPER_STRIDE = 7;
const PATTERN: number[][] = Array.from({ length: WALLPAPER_ROWS }, (_unused, row) =>
  Array.from({ length: WALLPAPER_COLUMNS }, (_unusedCol, col) => row + col * WALLPAPER_STRIDE)
);

export const PC_TEXT_PALETTE = 0x07;
export const PC_MAIL_TILE_ID = 0x5c;
export const PC_ITEM_TILE_ID = 0x5d;
export const PC_ARROW_TILE_IDS = { right: 0x5e, left: 0x5f } as const;

const DEFAULT_TEXT_PALETTE: Palette = [
  [255, 255, 255],
  [170, 170, 170],
  [85, 85, 85],
  [0, 0, 0],
];

const scaleGbc = (value: number): number => gbc5To8(value);

const BROWSER_PC_PALETTE: Palette = [
  [scaleGbc(31), scaleGbc(15), scaleGbc(0)],
  [scaleGbc(23), scaleGbc(12), scaleGbc(0)],
  [scaleGbc(15), scaleGbc(7), scaleGbc(0)],
  [scaleGbc(0), scaleGbc(0), scaleGbc(0)],
];

const BG_TEXT_PALETTE: Palette = [
  [scaleGbc(31), scaleGbc(31), scaleGbc(31)],
  [scaleGbc(8), scaleGbc(19), scaleGbc(28)],
  [scaleGbc(5), scaleGbc(5), scaleGbc(16)],
  [scaleGbc(0), scaleGbc(0), scaleGbc(0)],
];

const PALETTE_ORDER: Palette[] = Array.from({ length: PC_TEXT_PALETTE }, () =>
  DEFAULT_TEXT_PALETTE
).concat([BG_TEXT_PALETTE]);

const WALLPAPER_REGION: TileRegion | null = BILLS_PC_ATTR_BANDS.length
  ? BILLS_PC_ATTR_BANDS[0].region
  : null;

const isBrowser = typeof window !== "undefined";

const parsePalette = (palettePath: string): Palette => {
  if (!fs.existsSync(palettePath)) {
    throw new Error(`Missing PC palette ${palettePath}`);
  }
  const lines = fs.readFileSync(palettePath, "utf-8").split(/\r?\n/);
  const colours: [number, number, number][] = [];
  for (const raw of lines) {
    const line = raw.trim();
    if (!line.startsWith("RGB")) {
      continue;
    }
    const components = raw.split("RGB", 2)[1]?.trim().split(",") ?? [];
    if (components.length < 3) {
      continue;
    }
    const [r, g, b] = components.slice(0, 3).map((value) => parseInt(value.trim(), 10));
    colours.push([scaleGbc(r), scaleGbc(g), scaleGbc(b)]);
  }
  if (colours.length !== 4) {
    throw new Error(`PC palette ${palettePath} must expose 4 colours`);
  }
  return colours;
};

const decode2bppTiles = (data: Buffer): TilePixels[] => {
  if (data.length % 16 !== 0) {
    throw new Error("2bpp payload must be aligned to 16-byte tiles");
  }
  const tiles: TilePixels[] = [];
  const tileCount = data.length / 16;
  for (let tileIndex = 0; tileIndex < tileCount; tileIndex++) {
    const base = tileIndex * 16;
    const pixels: number[] = [];
    for (let row = 0; row < TILE_SIZE; row++) {
      const plane0 = data[base + row * 2];
      const plane1 = data[base + row * 2 + 1];
      for (let col = 0; col < TILE_SIZE; col++) {
        const bit = 7 - col;
        const idx = ((plane0 >> bit) & 1) | (((plane1 >> bit) & 1) << 1);
        pixels.push(idx);
      }
    }
    tiles.push(pixels);
  }
  return tiles;
};

const renderTile = (pixels: TilePixels, palette: Palette): Surface => {
  const surface = new Surface(TILE_SIZE, TILE_SIZE);
  for (let row = 0; row < TILE_SIZE; row++) {
    for (let col = 0; col < TILE_SIZE; col++) {
      const idx = pixels[row * TILE_SIZE + col];
      const colour = palette[idx];
      surface.setAt(col, row, [colour[0], colour[1], colour[2], 255]);
    }
  }
  return surface;
};

const decodePngTiles = (source: { getAt: (x: number, y: number) => [number, number, number, number]; get_size: () => [number, number] }): TilePixels[] => {
  const [width, height] = source.get_size();
  if (width % TILE_SIZE !== 0 || height % TILE_SIZE !== 0) {
    throw new Error(`PC PNG tileset must align to ${TILE_SIZE}px tiles, got ${width}x${height}`);
  }
  const tilesWide = width / TILE_SIZE;
  const tilesHigh = height / TILE_SIZE;
  const tiles: TilePixels[] = [];
  for (let tileY = 0; tileY < tilesHigh; tileY += 1) {
    for (let tileX = 0; tileX < tilesWide; tileX += 1) {
      const pixels: number[] = [];
      for (let row = 0; row < TILE_SIZE; row += 1) {
        for (let col = 0; col < TILE_SIZE; col += 1) {
          const [r, g, b, a] = source.getAt(tileX * TILE_SIZE + col, tileY * TILE_SIZE + row);
          if (a === 0) {
            pixels.push(0);
            continue;
          }
          const value = Math.round((r + g + b) / 3);
          const level = Math.max(0, Math.min(3, Math.round(value / 85)));
          pixels.push(level);
        }
      }
      tiles.push(pixels);
    }
  }
  return tiles;
};

const loadCachedPng = (pathName: string): InstanceType<typeof gameEngine.Surface> => {
  const cached = gameEngine.image?.loadSync?.(pathName);
  if (!cached) {
    throw new Error(`PC tileset ${pathName} must be preloaded before use.`);
  }
  return cached;
};

const loadPngTiles = (pathName: string, palette: Palette): Surface[] | null => {
  if (!fs.existsSync(pathName)) {
    return null;
  }
  try {
    const source = loadCachedPng(pathName);
    return decodePngTiles(source).map((tile) => renderTile(tile, palette));
  } catch {
    return null;
  }
};

let cachedPcTiles: Surface[] | null = null;
let cachedMailTiles: Surface[] | null = null;

const pcTileAssets = (): Surface[] => {
  if (cachedPcTiles) {
    return cachedPcTiles;
  }
  if (isBrowser) {
    const palette = BROWSER_PC_PALETTE;
    const source = loadCachedPng(PC_TILESET_PNG);
    const pixels = decodePngTiles(source);
    const tiles = pixels.map((tile) => renderTile(tile, palette));
    cachedPcTiles = tiles;
    return tiles;
  }
  if (!fs.existsSync(PC_TILESET)) {
    const pngTiles = loadPngTiles(PC_TILESET_PNG, BROWSER_PC_PALETTE);
    if (pngTiles) {
      cachedPcTiles = pngTiles;
      return cachedPcTiles;
    }
    throw new Error(`Missing PC tileset ${PC_TILESET}`);
  }
  const palette = parsePalette(PC_PALETTE);
  const pixels = decode2bppTiles(fs.readFileSync(PC_TILESET));
  const tiles = pixels.map((tile) => renderTile(tile, palette));
  cachedPcTiles = tiles;
  return tiles;
};

const backgroundTiles = (): Surface[] => {
  return pcTileAssets();
};

const pcMailTiles = (): Surface[] => {
  if (cachedMailTiles) {
    return cachedMailTiles;
  }
  if (isBrowser) {
    const palette = BROWSER_PC_PALETTE;
    const source = loadCachedPng(PC_MAIL_TILESET_PNG);
    const pixels = decodePngTiles(source);
    cachedMailTiles = pixels.map((tile) => renderTile(tile, palette));
    return cachedMailTiles;
  }
  if (!fs.existsSync(PC_MAIL_TILESET)) {
    const pngTiles = loadPngTiles(PC_MAIL_TILESET_PNG, BROWSER_PC_PALETTE);
    if (pngTiles) {
      cachedMailTiles = pngTiles;
      return cachedMailTiles;
    }
    throw new Error(`Missing PC mail icon tiles ${PC_MAIL_TILESET}`);
  }
  const palette = parsePalette(PC_PALETTE);
  const pixels = decode2bppTiles(fs.readFileSync(PC_MAIL_TILESET));
  cachedMailTiles = pixels.map((tile) => renderTile(tile, palette));
  return cachedMailTiles;
};

export const getPcMailIcon = (): Surface => {
  const tiles = pcMailTiles();
  if (!tiles.length) {
    throw new Error("PC mail icon tiles are unavailable");
  }
  return tiles[0];
};

export const getPcItemIcon = (): Surface => {
  const tiles = pcMailTiles();
  if (tiles.length < 2) {
    throw new Error("PC item icon tiles are unavailable");
  }
  return tiles[1];
};

export const getPcCursorTile = (tileId: number): Surface => {
  const tiles = pcTileAssets();
  const tile = tiles[tileId & 0xff];
  if (!tile) {
    throw new Error(`PC cursor tile ${tileId.toString(16)} is unavailable`);
  }
  return tile;
};

export const seedPcTilemap = (
  tilemap: TilemapSurface,
  { includeIcons = true }: { includeIcons?: boolean } = {}
): Record<string, number> => {
  tilemap.clearTilemap();
  const mailTiles = pcMailTiles();
  const iconIds: Record<string, number> = {};
  if (includeIcons && mailTiles.length > 0) {
    iconIds.mail = PC_MAIL_TILE_ID;
  }
  if (includeIcons && mailTiles.length > 1) {
    iconIds.item = PC_ITEM_TILE_ID;
  }
  return iconIds;
};

export interface BitmapFontLike {
  paletteVariants?: (paletteOrder: ReadonlyArray<Palette>) => Record<number, Record<number, Surface>>;
  fontTiles?: Record<number, Surface>;
  font_tiles?: Record<number, Surface>;
  getCharTile?: (char: string) => Surface | null | undefined;
  get_char_tile?: (char: string) => Surface | null | undefined;
}

const REQUIRED_PC_GLYPH_TILE_IDS = [
  CHAR_MAP["┌"],
  CHAR_MAP["─"],
  CHAR_MAP["┐"],
  CHAR_MAP["│"],
  CHAR_MAP["└"],
  CHAR_MAP["┘"],
  SPACE_TILE,
  CHAR_MAP["▶"],
].filter((tileId): tileId is number => typeof tileId === "number");

const addTileIfMissing = (
  tiles: Record<number, Surface | Record<number, Surface>>,
  tileId: number,
  tile: Surface | Record<number, Surface> | null | undefined,
): void => {
  if (tile && tiles[tileId] === undefined) {
    tiles[tileId] = tile;
  }
};

const mergeFontTiles = (
  tiles: Record<number, Surface | Record<number, Surface>>,
  font: BitmapFontLike,
): void => {
  const directTiles = font.fontTiles ?? font.font_tiles ?? {};
  for (const [tileId, tile] of Object.entries(directTiles)) {
    const numericId = Number(tileId);
    if (!Number.isNaN(numericId)) {
      addTileIfMissing(tiles, numericId, tile);
    }
  }

  if (font.paletteVariants) {
    const paletteVariants = font.paletteVariants(PALETTE_ORDER);
    for (const [tileId, variants] of Object.entries(paletteVariants)) {
      const numericId = Number(tileId);
      if (!Number.isNaN(numericId)) {
        addTileIfMissing(tiles, numericId, variants);
      }
    }
  }

  const getCharTile = font.getCharTile ?? font.get_char_tile;
  if (!getCharTile) {
    return;
  }
  for (const [char, tileId] of Object.entries(CHAR_MAP)) {
    if (typeof tileId !== "number") {
      continue;
    }
    addTileIfMissing(tiles, tileId, getCharTile.call(font, char) ?? null);
  }
};

export const pcTileset = (
  font: BitmapFontLike
): Record<number, Surface | Record<number, Surface>> => {
  const wallpaperTiles = backgroundTiles();
  const maxTile = WALLPAPER_STRIDE * WALLPAPER_COLUMNS - 1;
  const tiles: Record<number, Surface | Record<number, Surface>> = {};
  for (let tileId = 0; tileId <= maxTile; tileId++) {
    tiles[tileId] = wallpaperTiles[tileId % wallpaperTiles.length];
  }
  const mailTiles = pcMailTiles();
  for (let index = 0; index < mailTiles.length; index += 1) {
    tiles[PC_MAIL_TILE_ID + index] = mailTiles[index];
  }

  mergeFontTiles(tiles, font);

  const missingRequired = REQUIRED_PC_GLYPH_TILE_IDS.filter((tileId) => tiles[tileId] === undefined);
  if (missingRequired.length > 0) {
    const formatted = missingRequired
      .map((tileId) => `0x${tileId.toString(16).padStart(2, "0")}`)
      .join(", ");
    throw new Error(`PC tileset is missing required glyph tiles: ${formatted}.`);
  }
  return tiles;
};

export const canRenderPcTilemap = (font: BitmapFontLike | null | undefined): font is BitmapFontLike => {
  if (!font) {
    return false;
  }
  return Boolean(
    font.paletteVariants ||
    font.fontTiles ||
    font.font_tiles ||
    font.getCharTile ||
    font.get_char_tile
  );
};

export const createPcTilemap = (
  options?: { includeIcons?: boolean },
): { tilemap: TilemapSurface; iconIds: Record<string, number> } => {
  const tilemap = new TilemapSurface();
  const iconIds = seedPcTilemap(tilemap, options);
  return { tilemap, iconIds };
};

export const blitPcTilemap = (
  target: Surface,
  font: BitmapFontLike,
  tilemap: TilemapSurface,
): void => {
  if (!canRenderPcTilemap(font)) {
    return;
  }
  tilemap.blit(target, pcTileset(font));
};

export const drawPcWallpaper = (surface: Surface): void => {
  const tiles = backgroundTiles();
  if (!WALLPAPER_REGION) {
    throw new Error("PC wallpaper region is unavailable");
  }
  const rows = WALLPAPER_REGION.height;
  const cols = WALLPAPER_REGION.width;
  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      const idx = PATTERN[row % PATTERN.length][col % PATTERN[0].length];
      const destX = (WALLPAPER_REGION.x + col) * TILE_SIZE;
      const destY = (WALLPAPER_REGION.y + row) * TILE_SIZE;
      surface.blit(tiles[idx % tiles.length], [destX, destY]);
    }
  }
  const mailTiles = pcMailTiles();
  if (mailTiles.length > 0) {
    const icon = mailTiles[0];
    const destX = surface.width - TILE_SIZE * 2;
    const destY = TILE_SIZE;
    surface.blit(icon, [destX, destY]);
    if (mailTiles.length > 1) {
      surface.blit(mailTiles[1], [destX + TILE_SIZE, destY]);
    }
  }
};

export const getPcArrowTiles = (): [Surface, Surface] => {
  const tiles = pcMailTiles();
  if (tiles.length < 4) {
    throw new Error("PC mail icon tiles are missing expected arrow glyphs");
  }
  return [tiles[3], tiles[2]];
};

import fs from "fs";
import { decompress } from "../../core/lz";
import { assetExists } from "../../core/asset-manifest";
import { joinPath } from "../../core/path-utils";
import { getAssetPath } from "../../core/paths";
import { TileRegion } from "../tile-layout";
import { TilemapSurface, SPACE_TILE } from "../tilemap-surface";
import type { TilemapTilesetEntry } from "../tilemap-surface";
import { PlayerGender } from "../../core/enums";
import { Surface } from "../surface";
import { gbc5To8 } from "../../core/gbc-colors";
import type { Palette as FontPalette } from "../font-renderer";

// ASM: engine/items/pack.asm
const SCREEN_WIDTH_TILES = 20;
const SCREEN_HEIGHT_TILES = 18;

const PACK_ASSET_DIR = getAssetPath("gfx", "pack");
const PACK_MENU_STEM = "pack_menu";
const PACK_MENU_TILEMAP = joinPath(PACK_ASSET_DIR, "pack_menu.tilemap");
const PACK_MENU_PALETTE = joinPath(PACK_ASSET_DIR, "pack.pal");
const PACK_ICON_STEM = "pack";
const PACK_MENU_F_PAL = joinPath(PACK_ASSET_DIR, "pack_f.pal");
const PACK_ICON_F_STEM = "pack_f";
const BACKGROUND_TILE = 0x24;
const TOP_BAR_FIRST_TILE = 0x28;
const PACK_ICON_TILE_START = 0x50;
const PACK_ICON_TILES_PER_VARIANT = 15;
const BOTTOM_TEXTBOX_REGION = new TileRegion(0, 12, 20, 6);
const POCKET_LABEL_REGION = new TileRegion(0, 7, 5, 3);
const PACK_ICON_REGION = new TileRegion(0, 3, 5, 3);
const ITEM_LIST_CLEAR_REGION = new TileRegion(5, 1, 15, 11);
const ITEM_TEXT_REGION = new TileRegion(6, 2, 13, 7);
const TEXT_ATTR = 0x07;
const POCKET_TILE_WIDTH = 5;
const POCKET_TILE_HEIGHT = 3;
const POCKET_CHUNKS = 4;
const POINTER_COLUMN = 7;
const LABEL_COLUMN = 8;
const QUANTITY_COLUMN = 16;
const ARROW_COLUMN = 19;
const ACTION_WINDOW_LEFT = 13;
const ACTION_WINDOW_WIDTH = 7;
const TILE_SIZE = 8;

type PackPalette = { colours: [number, number, number][] };
type TilePixels = number[];

const BLANK_TILE: TilePixels = Array(TILE_SIZE * TILE_SIZE).fill(0);
const bagTilesetCache = new WeakMap<object, Map<string, Record<number, TilemapTilesetEntry>>>();

export class BagMenuTilemap extends TilemapSurface {
  constructor() {
    super(SCREEN_WIDTH_TILES, SCREEN_HEIGHT_TILES);
  }
}

const readBytes = (filePath: string): Buffer => {
  if (!assetExists(filePath)) {
    throw new Error(`Missing Pack asset: ${filePath}`);
  }
  const data = fs.readFileSync(filePath);
  if (filePath.endsWith(".lz")) {
    return Buffer.from(decompress(data));
  }
  return data;
};

const loadTileBytes = (stem: string): Buffer => {
  const suffixes = [".2bpp", ".2bpp.lz", ".png"];
  for (const suffix of suffixes) {
    const candidate = joinPath(PACK_ASSET_DIR, `${stem}${suffix}`);
    if (!assetExists(candidate)) {
      continue;
    }
    if (candidate.endsWith(".png")) {
      throw new Error(`PNG decoding not supported for ${candidate}`);
    }
    return readBytes(candidate);
  }
  throw new Error(`Missing Pack graphics for ${stem}`);
};

const parsePalettes = (filePath: string): PackPalette[] => {
  if (!assetExists(filePath)) {
    throw new Error(`Missing palette ${filePath}`);
  }
  const palettes: PackPalette[] = [];
  let colours: [number, number, number][] = [];
  const lines = fs.readFileSync(filePath, "utf-8").split(/\r?\n/);
  for (const raw of lines) {
    const line = raw.trim();
    if (!line.toUpperCase().startsWith("RGB")) {
      if (colours.length) {
        if (colours.length !== 4) {
          throw new Error(`Palette ${filePath} rows must be multiples of four entries`);
        }
        palettes.push({ colours });
        colours = [];
      }
      continue;
    }
    const comps = line.split("RGB", 2)[1].split(",").map((part) => part.trim());
    if (comps.length < 3) {
      continue;
    }
    const [r, g, b] = comps.map((value) => Number(value));
    colours.push([gbc5To8(r), gbc5To8(g), gbc5To8(b)]);
    if (colours.length === 4) {
      palettes.push({ colours });
      colours = [];
    }
  }
  if (colours.length) {
    if (colours.length !== 4) {
      throw new Error(`Palette ${filePath} ended mid-definition.`);
    }
    palettes.push({ colours });
  }
  if (!palettes.length) {
    throw new Error(`Palette file ${filePath} did not contain any RGB entries.`);
  }
  return palettes;
};

const decode2bppTilePixels = (data: Buffer): TilePixels[] => {
  if (data.length % 16 !== 0) {
    throw new Error("2bpp payload must be aligned to 16-byte tiles");
  }
  const tiles: TilePixels[] = [];
  const total = data.length / 16;
  for (let tileIndex = 0; tileIndex < total; tileIndex += 1) {
    const base = tileIndex * 16;
    const pixels: number[] = [];
    for (let row = 0; row < TILE_SIZE; row += 1) {
      const plane0 = data[base + row * 2];
      const plane1 = data[base + row * 2 + 1];
      for (let col = 0; col < TILE_SIZE; col += 1) {
        const bit = 7 - col;
        const idx = ((plane0 >> bit) & 1) | (((plane1 >> bit) & 1) << 1);
        pixels.push(idx);
      }
    }
    tiles.push(pixels);
  }
  return tiles;
};

const renderTile = (pixels: TilePixels, palette: PackPalette): Surface => {
  const surface = new Surface(TILE_SIZE, TILE_SIZE);
  for (let row = 0; row < TILE_SIZE; row += 1) {
    for (let col = 0; col < TILE_SIZE; col += 1) {
      const idx = pixels[row * TILE_SIZE + col];
      const colour = palette.colours[idx] ?? palette.colours[0];
      surface.setAt(col, row, [colour[0], colour[1], colour[2], 255]);
    }
  }
  return surface;
};

let packMenuTilePixelsCache: TilePixels[] | null = null;
const packMenuTilePixels = (): TilePixels[] => {
  if (!packMenuTilePixelsCache) {
    const data = loadTileBytes(PACK_MENU_STEM);
    packMenuTilePixelsCache = decode2bppTilePixels(data);
  }
  return packMenuTilePixelsCache;
};

const packIconChunksCache = new Map<PlayerGender, TilePixels[][]>();
const packIconChunks = (gender: PlayerGender): TilePixels[][] => {
  const cached = packIconChunksCache.get(gender);
  if (cached) {
    return cached;
  }
  const stem = gender === PlayerGender.FEMALE ? PACK_ICON_F_STEM : PACK_ICON_STEM;
  const data = loadTileBytes(stem);
  const tiles = decode2bppTilePixels(data);
  if (tiles.length % PACK_ICON_TILES_PER_VARIANT !== 0) {
    throw new Error("Pack icon tiles must align with 15-tile chunks");
  }
  const chunks: TilePixels[][] = [];
  for (let offset = 0; offset < tiles.length; offset += PACK_ICON_TILES_PER_VARIANT) {
    chunks.push(tiles.slice(offset, offset + PACK_ICON_TILES_PER_VARIANT));
  }
  packIconChunksCache.set(gender, chunks);
  return chunks;
};

const packPalettesCache = new Map<PlayerGender, PackPalette[]>();
const packPalettes = (gender: PlayerGender): PackPalette[] => {
  const cached = packPalettesCache.get(gender);
  if (cached) {
    return cached;
  }
  const filePath = gender === PlayerGender.FEMALE ? PACK_MENU_F_PAL : PACK_MENU_PALETTE;
  const parsed = parsePalettes(filePath);
  packPalettesCache.set(gender, parsed);
  return parsed;
};

const pocketChunkIndex = (pocketIndex: number): number => {
  const mapping = [1, 3, 0, 2];
  if (pocketIndex < 0 || pocketIndex >= mapping.length) {
    throw new Error(`Pocket index ${pocketIndex} is out of range`);
  }
  return mapping[pocketIndex];
};

const packTileVariantsCache = new Map<string, Record<number, TilemapTilesetEntry>>();
const packTileVariants = (pocketIndex: number, gender: PlayerGender): Record<number, TilemapTilesetEntry> => {
  const cacheKey = `${gender}:${pocketIndex}`;
  const cached = packTileVariantsCache.get(cacheKey);
  if (cached) {
    return cached;
  }
  const baseTiles = [...packMenuTilePixels()];
  const tilePixels = [...baseTiles];
  const required = PACK_ICON_TILE_START + PACK_ICON_TILES_PER_VARIANT;
  if (tilePixels.length < required) {
    tilePixels.push(...Array(required - tilePixels.length).fill(BLANK_TILE));
  }
  const chunks = packIconChunks(gender);
  const chunk = chunks[pocketChunkIndex(pocketIndex)];
  chunk.forEach((pixels, offset) => {
    tilePixels[PACK_ICON_TILE_START + offset] = pixels;
  });
  const palettes = packPalettes(gender);
  const variants: Record<number, TilemapTilesetEntry> = {};
  tilePixels.forEach((pixels, tileId) => {
    const paletteVariants: Record<number, Surface> = {};
    palettes.forEach((palette, paletteIndex) => {
      paletteVariants[paletteIndex] = renderTile(pixels, palette);
    });
    variants[tileId] = paletteVariants;
  });
  packTileVariantsCache.set(cacheKey, variants);
  return variants;
};

let pocketLabelTilesCache: number[][][] | null = null;
const pocketLabelTiles = (): number[][][] => {
  if (pocketLabelTilesCache) {
    return pocketLabelTilesCache;
  }
  const data = readBytes(PACK_MENU_TILEMAP);
  const expected = POCKET_TILE_WIDTH * POCKET_TILE_HEIGHT * POCKET_CHUNKS;
  if (data.length !== expected) {
    throw new Error(`pack_menu.tilemap must contain ${expected} bytes, found ${data.length}`);
  }
  const rows: number[][] = [];
  for (let row = 0; row < data.length; row += POCKET_TILE_WIDTH) {
    rows.push(Array.from(data.slice(row, row + POCKET_TILE_WIDTH)));
  }
  const chunks: number[][][] = [];
  for (let offset = 0; offset < rows.length; offset += POCKET_TILE_HEIGHT) {
    chunks.push(rows.slice(offset, offset + POCKET_TILE_HEIGHT));
  }
  if (chunks.length !== POCKET_CHUNKS) {
    throw new Error("pack_menu.tilemap must provide 4 pocket labels");
  }
  pocketLabelTilesCache = chunks;
  return chunks;
};

export function seedTilemapBase(tilemap: BagMenuTilemap, pocketIndex: number): void {
  tilemap.fillRect(0, 0, tilemap.width, tilemap.height, SPACE_TILE);
  tilemap.fillRect(0, 1, tilemap.width, 11, BACKGROUND_TILE);
  tilemap.fillRect(
    ITEM_LIST_CLEAR_REGION.left,
    ITEM_LIST_CLEAR_REGION.top,
    ITEM_LIST_CLEAR_REGION.width,
    ITEM_LIST_CLEAR_REGION.height,
    SPACE_TILE,
  );
  for (let col = 0; col < tilemap.width; col += 1) {
    tilemap.setTile(col, 0, TOP_BAR_FIRST_TILE + col);
  }
  tilemap.drawWindow(
    BOTTOM_TEXTBOX_REGION.left,
    BOTTOM_TEXTBOX_REGION.top,
    BOTTOM_TEXTBOX_REGION.width,
    BOTTOM_TEXTBOX_REGION.height,
    { attr: TEXT_ATTR },
  );
  applyPocketLabel(tilemap, pocketIndex);
  placePackIcon(tilemap);
  applyPackAttrmap(tilemap);
}

const applyPocketLabel = (tilemap: BagMenuTilemap, pocketIndex: number): void => {
  const chunks = pocketLabelTiles();
  if (pocketIndex < 0 || pocketIndex >= chunks.length) {
    throw new Error(`Pocket index ${pocketIndex} is invalid`);
  }
  const chunk = chunks[pocketIndex];
  chunk.forEach((row, rowIdx) => {
    row.forEach((tile, colIdx) => {
      tilemap.setTile(POCKET_LABEL_REGION.left + colIdx, POCKET_LABEL_REGION.top + rowIdx, tile);
    });
  });
};

const placePackIcon = (tilemap: BagMenuTilemap): void => {
  let tileId = PACK_ICON_TILE_START;
  for (let row = 0; row < PACK_ICON_REGION.height; row += 1) {
    for (let col = 0; col < PACK_ICON_REGION.width; col += 1) {
      tilemap.setTile(PACK_ICON_REGION.left + col, PACK_ICON_REGION.top + row, tileId);
      tileId += 1;
    }
  }
};

const applyPackAttrmap = (tilemap: BagMenuTilemap): void => {
  tilemap.fillAttrRect(0, 0, 10, 1, 0x01);
  tilemap.fillAttrRect(10, 0, 10, 1, 0x02);
  tilemap.fillAttrRect(7, 2, 1, 9, 0x03);
  tilemap.fillAttrRect(0, 7, 5, 3, 0x04);
  tilemap.fillAttrRect(0, 3, 5, 3, 0x05);
};

export function bagTileset(
  font: { paletteVariants?: (paletteOrder: ReadonlyArray<FontPalette>) => Record<number, Record<number, Surface>>; fontTiles?: Record<number, Surface>; font_tiles?: Record<number, Surface> } | Record<number, Surface>,
  pocketIndex: number,
  gender: PlayerGender,
): Record<number, TilemapTilesetEntry> {
  const fontObject = font as object;
  const cacheKey = `${gender}:${pocketIndex}`;
  let cachedByFont = bagTilesetCache.get(fontObject);
  if (!cachedByFont) {
    cachedByFont = new Map<string, Record<number, TilemapTilesetEntry>>();
    bagTilesetCache.set(fontObject, cachedByFont);
  }
  const cached = cachedByFont.get(cacheKey);
  if (cached) {
    return cached;
  }

  const tiles: Record<number, TilemapTilesetEntry> = { ...packTileVariants(pocketIndex, gender) };
  const palettes = packPalettes(gender);
  if (!palettes.length) {
    throw new Error("Pack palettes are missing; cannot tint font glyphs.");
  }
  let paletteColours = palettes.map((palette) => palette.colours);
  if (paletteColours.length < 8) {
    const last = paletteColours[paletteColours.length - 1];
    paletteColours = paletteColours.concat(Array(8 - paletteColours.length).fill(last));
  }

  let fontTiles: Record<number, Surface> = {};
  if ("paletteVariants" in font && typeof font.paletteVariants === "function") {
    const variants = font.paletteVariants(paletteColours);
    for (const [tileId, paletteMap] of Object.entries(variants)) {
      if (!(Number(tileId) in tiles)) {
        tiles[Number(tileId)] = paletteMap;
      }
    }
    fontTiles = font.fontTiles ?? font.font_tiles ?? {};
  } else {
    fontTiles = font as Record<number, Surface>;
  }

  for (const [tileId, surface] of Object.entries(fontTiles)) {
    const numericId = Number(tileId);
    if (!(numericId in tiles)) {
      tiles[numericId] = surface;
    }
  }
  cachedByFont.set(cacheKey, tiles);
  return tiles;
}

export function itemTextRegion(): TileRegion {
  return ITEM_TEXT_REGION;
}

export function bottomTextboxRegion(): TileRegion {
  return BOTTOM_TEXTBOX_REGION;
}

export function pointerColumn(): number {
  return POINTER_COLUMN;
}

export function labelColumn(): number {
  return LABEL_COLUMN;
}

export function quantityColumn(): number {
  return QUANTITY_COLUMN;
}

export function scrollColumn(): number {
  return ARROW_COLUMN;
}

export function actionWindowLeft(): number {
  return ACTION_WINDOW_LEFT;
}

export function actionWindowWidth(): number {
  return ACTION_WINDOW_WIDTH;
}

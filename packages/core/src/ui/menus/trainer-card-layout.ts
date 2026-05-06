// ASM mapping: pokecrystal_disassembly/engine/menus/trainer_card.asm (tilemap + palette swaps).
import fs from "fs";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import { assetExists } from "@pokecrystal/core/core/asset-manifest";
import { decompress } from "@pokecrystal/core/core/lz";
import { joinPath } from "@pokecrystal/core/core/path-utils";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { gbc5To8, gbcWordToRgb } from "@pokecrystal/core/core/gbc-colors";
import { TileRegion } from "@pokecrystal/core/ui/tile-layout";
import { Surface } from "@pokecrystal/core/ui/surface";
import { TilemapSurface, SPACE_TILE } from "@pokecrystal/core/ui/tilemap-surface";
import { loadImageSync } from "../game-engine";

const TILE_SIZE = 8;

type TilePixels = number[];
type Palette = [number, number, number][];

const TRAINER_CARD_DIR = getAssetPath("gfx", "trainer_card");
const TRAINER_PAL_DIR = getAssetPath("gfx", "trainers");
const PREDEF_PAL_FILE = getAssetPath("gfx", "sgb", "predef.pal");

const CARD_RIGHT_CORNER_INDEX = 0x1c;
const TRAINER_CARD_TILE_BASE = 0x23;
const CARD_STATUS_TILE_BASE = 0x29;
const SMALL_COLON_TILE = CARD_STATUS_TILE_BASE + 5;

const TOP_BORDER_ROWS = 5;
const BOTTOM_BORDER_ROWS = 6;
const PORTRAIT_X = 14;
const PORTRAIT_Y = 1;
const PORTRAIT_WIDTH = 5;
const PORTRAIT_HEIGHT = 7;

const LEADER_TILE_BASE = 0x29;
const LEADER_SECOND_ROW_TILE_BASE = 0x51;
const LEADER_GRID_ORIGIN: [number, number] = [2, 10];
const LEADER_GRID_WIDTH = 16;
const LEADER_GRID_HEIGHT = 2;
const LEADER_BLOCK_WIDTH = 4;
const LEADER_BLOCK_HEIGHT = 3;
const BADGE_PALETTE_INDEX = 8;

const NAME_VALUE: [number, number] = [7, 2];
const ID_VALUE: [number, number] = [5, 4];
const MONEY_VALUE: [number, number] = [7, 6];
const POKEDEX_VALUE: [number, number] = [15, 10];
const PLAY_TIME_HOURS: [number, number] = [11, 12];
const PLAY_TIME_MINUTES: [number, number] = [16, 12];
const CLOCK_COLON: [number, number] = [15, 12];
const STATUS_CLEAR_REGION = new TileRegion(1, 9, 17, 2);

const TRAINER_PALETTE_FILES: Record<number, string> = {
  0: "cal",
  1: "falkner",
  2: "bugsy",
  3: "whitney",
  4: "morty",
  5: "chuck",
  6: "jasmine",
  7: "pryce",
};

const BLANK_TILE: TilePixels = Array(TILE_SIZE * TILE_SIZE).fill(0);

const BADGES_TILEMAP = [0x79, 0x7a, 0x7b, 0x7c, 0x7d];

export class TrainerCardTilemap extends TilemapSurface {
  constructor() {
    super({ width: 20, height: 18 });
  }
}

export const nameValueOrigin = (): [number, number] => NAME_VALUE;
export const idValueOrigin = (): [number, number] => ID_VALUE;
export const moneyValueOrigin = (): [number, number] => MONEY_VALUE;
export const pokedexValueOrigin = (): [number, number] => POKEDEX_VALUE;
export const playTimeHoursOrigin = (): [number, number] => PLAY_TIME_HOURS;
export const playTimeMinutesOrigin = (): [number, number] => PLAY_TIME_MINUTES;
export const clockColonCoords = (): [number, number] => CLOCK_COLON;
export const statusClearRegion = (): TileRegion => STATUS_CLEAR_REGION;
export const smallColonTile = (): number => SMALL_COLON_TILE;

// ASM: TrainerCard_InitBorder + static label placement.
export const seedTrainerCardPageOne = (tilemap: TrainerCardTilemap, gender: PlayerGender): void => {
  tilemap.fillRect(0, 0, tilemap.width, tilemap.height, { tile: SPACE_TILE });
  drawBorder(tilemap, 0, TOP_BORDER_ROWS);
  drawBorder(tilemap, 8, BOTTOM_BORDER_ROWS);
  placePortrait(tilemap);
  placeLabels(tilemap);
  placeStatusBanner(tilemap);
  applyAttrmap(tilemap, gender);
};

// ASM: TrainerCard_Page2_3_InitObjectsAndStrings + badges header.
export const seedTrainerCardBadgePage = (
  tilemap: TrainerCardTilemap,
  gender: PlayerGender,
  { showLeaders = true, preserveTop = false }: { showLeaders?: boolean; preserveTop?: boolean } = {}
): void => {
  const bottomStart = 8;
  if (preserveTop) {
    tilemap.fillRect(0, bottomStart, tilemap.width, tilemap.height, { tile: SPACE_TILE });
  } else {
    tilemap.fillRect(0, 0, tilemap.width, tilemap.height, { tile: SPACE_TILE });
    drawBorder(tilemap, 0, TOP_BORDER_ROWS);
  }
  drawBorder(tilemap, bottomStart, BOTTOM_BORDER_ROWS);
  tilemap.writeTiles(2, bottomStart, BADGES_TILEMAP);
  if (showLeaders) {
    placeLeaderPortraits(tilemap);
  }
  applyAttrmap(tilemap, gender);
};

export interface BitmapFontLike {
  paletteVariants?: (paletteOrder: ReadonlyArray<Palette>) => Record<number, Record<number, Surface>>;
  fontTiles?: Record<number, Surface>;
}

export const trainerCardTileset = (
  font: BitmapFontLike | Record<number, Surface>,
  gender: PlayerGender,
  {
    includeLeaderTiles = false,
    includeCornerTile = true,
  }: { includeLeaderTiles?: boolean; includeCornerTile?: boolean } = {}
): Record<number, Surface | Record<number, Surface>> => {
  const tiles: Record<number, Surface | Record<number, Surface>> = {
    ...trainerCardTileVariants(gender, includeCornerTile),
  };
  if (includeLeaderTiles) {
    Object.assign(tiles, leaderTileVariants());
  }

  const palettes = trainerCardPalettes();
  const paletteIndices = Object.keys(palettes).map(Number);
  if (!paletteIndices.length) {
    throw new Error("Trainer Card palettes are missing; cannot tint font glyphs.");
  }
  const maxIndex = Math.max(...paletteIndices);
  const fallback = palettes[paletteIndices[0]];
  const paletteOrder = Array.from({ length: maxIndex + 1 }, (_unused, index) => palettes[index] ?? fallback);

  let fontTiles: Record<number, Surface> = {};
  const fontLike = font as BitmapFontLike;
  if (typeof fontLike.paletteVariants === "function") {
    const fontVariants = fontLike.paletteVariants(paletteOrder);
    Object.entries(fontVariants).forEach(([tileId, paletteMap]) => {
      const id = Number(tileId);
      if (tiles[id] === undefined) {
        tiles[id] = paletteMap;
      }
    });
    fontTiles = fontLike.fontTiles ?? {};
  } else {
    fontTiles = font as Record<number, Surface>;
  }

  Object.entries(fontTiles).forEach(([tileId, surface]) => {
    const id = Number(tileId);
    if (tiles[id] === undefined) {
      tiles[id] = surface;
    }
  });
  return tiles;
};

// ---------------------------------------------------------------------------
// Static layout helpers
// ---------------------------------------------------------------------------

const drawBorder = (tilemap: TrainerCardTilemap, top: number, rows: number): void => {
  let row = top;
  for (let col = 0; col < tilemap.width; col++) {
    tilemap.setTile(col, row, TRAINER_CARD_TILE_BASE);
  }
  row += 1;

  tilemap.setTile(0, row, TRAINER_CARD_TILE_BASE);
  for (let col = 1; col < tilemap.width - 2; col++) {
    tilemap.setTile(col, row, SPACE_TILE);
  }
  tilemap.setTile(tilemap.width - 2, row, CARD_RIGHT_CORNER_INDEX);
  tilemap.setTile(tilemap.width - 1, row, TRAINER_CARD_TILE_BASE);
  row += 1;

  for (let i = 0; i < rows; i++) {
    tilemap.setTile(0, row, TRAINER_CARD_TILE_BASE);
    for (let col = 1; col < tilemap.width - 1; col++) {
      tilemap.setTile(col, row, SPACE_TILE);
    }
    tilemap.setTile(tilemap.width - 1, row, TRAINER_CARD_TILE_BASE);
    row += 1;
  }

  tilemap.setTile(0, row, TRAINER_CARD_TILE_BASE);
  tilemap.setTile(1, row, TRAINER_CARD_TILE_BASE + 1);
  for (let col = 2; col < tilemap.width - 1; col++) {
    tilemap.setTile(col, row, SPACE_TILE);
  }
  tilemap.setTile(tilemap.width - 1, row, TRAINER_CARD_TILE_BASE);
  row += 1;

  for (let col = 0; col < tilemap.width; col++) {
    tilemap.setTile(col, row, TRAINER_CARD_TILE_BASE);
  }
};

const placePortrait = (tilemap: TrainerCardTilemap): void => {
  let tileId = 0;
  // ASM: `PlaceGraphic` consumes the 5x7 trainer-card portrait tiles sequentially row-by-row.
  for (let dy = 0; dy < PORTRAIT_HEIGHT; dy++) {
    for (let dx = 0; dx < PORTRAIT_WIDTH; dx++) {
      tilemap.setTile(PORTRAIT_X + dx, PORTRAIT_Y + dy, tileId);
      tileId += 1;
    }
  }
};

const placeLabels = (tilemap: TrainerCardTilemap): void => {
  tilemap.writeText(2, 2, "NAME/", { maxLength: 5, pad: true });
  tilemap.writeTiles(2, 4, [0x27, 0x28]);
  tilemap.writeText(2, 6, "MONEY", { maxLength: 5, pad: true });
  writeHorizontalDivider(tilemap);
  tilemap.writeText(2, 10, "#DEX", { maxLength: 4, pad: true });
  tilemap.writeText(2, 12, "PLAY TIME", { maxLength: 9, pad: true });
  tilemap.writeText(10, 15, "  BADGES▶", { maxLength: 9, pad: true });
  tilemap.setTile(CLOCK_COLON[0], CLOCK_COLON[1], SMALL_COLON_TILE);
};

const writeHorizontalDivider = (tilemap: TrainerCardTilemap): void => {
  const divider = Array(12).fill(0x25).concat([0x26]);
  tilemap.writeTiles(1, 3, divider);
};

const placeStatusBanner = (tilemap: TrainerCardTilemap): void => {
  const tiles = Array.from({ length: 5 }, (_unused, idx) => CARD_STATUS_TILE_BASE + idx);
  tilemap.writeTiles(2, 8, tiles);
};

const placeLeaderFaces = (tilemap: TrainerCardTilemap, startX: number, startY: number, tileId: number): number => {
  const width = tilemap.width;
  let index = startY * width + startX;

  const write = (count: number): void => {
    for (let i = 0; i < count; i++) {
      const x = index % width;
      const y = Math.floor(index / width);
      tilemap.setTile(x, y, tileId);
      index += 1;
      tileId = (tileId + 1) & 0xff;
    }
  };

  write(4);
  index += width - 3;
  write(3);
  index += width - 3;
  write(3);
  return tileId;
};

const placeLeaderPortraits = (tilemap: TrainerCardTilemap): void => {
  const [originX, originY] = LEADER_GRID_ORIGIN;
  const blockCount = LEADER_GRID_WIDTH / LEADER_BLOCK_WIDTH;
  let x = originX;
  let y = originY;
  let tileBase = LEADER_TILE_BASE;
  for (let i = 0; i < blockCount; i++) {
    tileBase = placeLeaderFaces(tilemap, x, y, tileBase);
    x += LEADER_BLOCK_WIDTH;
  }

  x = originX;
  y += LEADER_BLOCK_HEIGHT;
  tileBase = LEADER_SECOND_ROW_TILE_BASE;
  for (let i = 0; i < blockCount; i++) {
    tileBase = placeLeaderFaces(tilemap, x, y, tileBase);
    x += LEADER_BLOCK_WIDTH;
  }
};

const applyAttrmap = (tilemap: TrainerCardTilemap, gender: PlayerGender): void => {
  const backgroundAttr = gender === PlayerGender.MALE ? 1 : 0;
  tilemap.fillAttrRect(0, 0, tilemap.width, tilemap.height, backgroundAttr);

  const portraitAttr = gender === PlayerGender.MALE ? 0 : 1;
  tilemap.fillAttrRect(PORTRAIT_X, PORTRAIT_Y, PORTRAIT_WIDTH, PORTRAIT_HEIGHT, portraitAttr);

  tilemap.setAttr(18, 1, 1);
  tilemap.fillAttrRect(2, 11, 4, 2, 1);
  tilemap.fillAttrRect(6, 11, 4, 2, 2);
  tilemap.fillAttrRect(10, 11, 4, 2, 3);
  tilemap.fillAttrRect(14, 11, 4, 2, 4);
  tilemap.fillAttrRect(2, 14, 4, 2, 5);
  tilemap.fillAttrRect(6, 14, 4, 2, 6);
  tilemap.fillAttrRect(10, 14, 4, 2, 7);
  if (gender === PlayerGender.FEMALE) {
    tilemap.fillAttrRect(14, 14, 4, 2, 1);
  }
  const cornerAttr = gender === PlayerGender.MALE ? 1 : 0;
  tilemap.setAttr(18, 1, cornerAttr);
};

// ---------------------------------------------------------------------------
// Tileset + palette helpers
// ---------------------------------------------------------------------------

const trainerCardTileVariants = (
  gender: PlayerGender,
  includeCornerTile: boolean
): Record<number, Record<number, Surface>> => {
  const tilePixels = composeTilePixels(gender, includeCornerTile);
  const palettes = trainerCardPalettes();
  const variants: Record<number, Record<number, Surface>> = {};
  tilePixels.forEach((pixels, tileId) => {
    const paletteVariants: Record<number, Surface> = {};
    Object.entries(palettes).forEach(([paletteIndex, palette]) => {
      paletteVariants[Number(paletteIndex)] = renderTile(pixels, palette);
    });
    variants[tileId] = paletteVariants;
  });
  return variants;
};

const composeTilePixels = (gender: PlayerGender, includeCornerTile: boolean): TilePixels[] => {
  const pixels = [...heroTilePixels(gender)];
  ensureTileCapacity(pixels, CARD_RIGHT_CORNER_INDEX + 1);
  if (includeCornerTile) {
    pixels[CARD_RIGHT_CORNER_INDEX] = cardRightCornerTile();
  }

  const trainerTiles = trainerCardTiles();
  trainerTiles.forEach((tile, offset) => {
    ensureTileCapacity(pixels, TRAINER_CARD_TILE_BASE + offset + 1);
    pixels[TRAINER_CARD_TILE_BASE + offset] = tile;
  });

  const statusTiles = cardStatusTiles();
  statusTiles.forEach((tile, offset) => {
    ensureTileCapacity(pixels, CARD_STATUS_TILE_BASE + offset + 1);
    pixels[CARD_STATUS_TILE_BASE + offset] = tile;
  });

  return pixels;
};

const ensureTileCapacity = (pixels: TilePixels[], size: number): void => {
  while (pixels.length < size) {
    pixels.push([...BLANK_TILE]);
  }
};

const heroTileCache: Record<string, TilePixels[]> = {};
const heroTilePixels = (gender: PlayerGender): TilePixels[] => {
  const stem = gender === PlayerGender.MALE ? "chris_card" : "kris_card";
  if (!heroTileCache[stem]) {
    heroTileCache[stem] = loadTilePixels(stem, { preferPng: true });
  }
  return heroTileCache[stem];
};

export const trainerCardPortraitTilePixels = (gender: PlayerGender): TilePixels[] =>
  heroTilePixels(gender);

let trainerCardTilesCache: TilePixels[] | null = null;
const trainerCardTiles = (): TilePixels[] => {
  if (!trainerCardTilesCache) {
    trainerCardTilesCache = decode2bppTilePixels(loadTileBytes("trainer_card"));
  }
  return trainerCardTilesCache;
};

let cardStatusTilesCache: TilePixels[] | null = null;
const cardStatusTiles = (): TilePixels[] => {
  if (!cardStatusTilesCache) {
    cardStatusTilesCache = decode2bppTilePixels(loadTileBytes("card_status"));
  }
  return cardStatusTilesCache;
};

let cardRightCornerCache: TilePixels | null = null;
const cardRightCornerTile = (): TilePixels => {
  if (!cardRightCornerCache) {
    const tiles = decode2bppTilePixels(loadTileBytes("card_right_corner"));
    cardRightCornerCache = tiles.length ? tiles[0] : [...BLANK_TILE];
  }
  return cardRightCornerCache;
};

let trainerCardPalettesCache: Record<number, Palette> | null = null;
const trainerCardPalettes = (): Record<number, Palette> => {
  if (trainerCardPalettesCache) {
    return trainerCardPalettesCache;
  }
  const palettes: Record<number, Palette> = {};
  Object.entries(TRAINER_PALETTE_FILES).forEach(([index, stem]) => {
    palettes[Number(index)] = loadGbcpal(stem);
  });
  palettes[8] = loadPredefPalette("PREDEFPAL_CGB_BADGE");
  trainerCardPalettesCache = palettes;
  return palettes;
};

let trainerCardBadgePaletteCache: Palette | null = null;
const trainerCardBadgePalette = (): Palette => {
  if (!trainerCardBadgePaletteCache) {
    trainerCardBadgePaletteCache = loadPredefPalette("PREDEFPAL_CGB_BADGE");
  }
  return trainerCardBadgePaletteCache;
};

let trainerCardBadgeTilesCache: Surface[] | null = null;
export const trainerCardBadgeTiles = (): Surface[] => {
  if (trainerCardBadgeTilesCache) {
    return trainerCardBadgeTilesCache;
  }
  const data = loadTileBytes("badges");
  const palette = trainerCardBadgePalette();
  const tiles = decode2bppTilePixels(data).map((tile) => renderTile(tile, palette));
  trainerCardBadgeTilesCache = tiles;
  return tiles;
};

let leaderTilePixelsCache: TilePixels[] | null = null;
const leaderTilePixels = (): TilePixels[] => {
  if (!leaderTilePixelsCache) {
    leaderTilePixelsCache = decode2bppTilePixels(loadTileBytes("leaders"));
  }
  return leaderTilePixelsCache;
};

let leaderTileVariantsCache: Record<number, Record<number, Surface>> | null = null;
const leaderTileVariants = (): Record<number, Record<number, Surface>> => {
  if (leaderTileVariantsCache) {
    return leaderTileVariantsCache;
  }
  const palettes = trainerCardPalettes();
  const variants: Record<number, Record<number, Surface>> = {};
  leaderTilePixels().forEach((pixels, offset) => {
    const paletteVariants: Record<number, Surface> = {};
    Object.entries(palettes).forEach(([paletteIndex, palette]) => {
      paletteVariants[Number(paletteIndex)] = renderTile(pixels, palette);
    });
    variants[LEADER_TILE_BASE + offset] = paletteVariants;
  });
  leaderTileVariantsCache = variants;
  return variants;
};

const loadGbcpal = (stem: string): Palette => {
  const palettePath = joinPath(TRAINER_PAL_DIR, `${stem}.gbcpal`);
  if (!assetExists(palettePath)) {
    throw new Error(`Missing trainer palette: ${palettePath}`);
  }
  const data = fs.readFileSync(palettePath);
  if (data.length !== 8) {
    throw new Error(`Trainer palette ${palettePath} must be exactly 8 bytes (4 colours), got ${data.length}.`);
  }
  const colours: Palette = [];
  for (let offset = 0; offset < 8; offset += 2) {
    colours.push(gbcWordToRgb(data.readUInt16LE(offset)));
  }
  return colours;
};

const loadPredefPalette = (label: string): Palette => {
  if (!assetExists(PREDEF_PAL_FILE)) {
    throw new Error(`Missing palette file: ${PREDEF_PAL_FILE}`);
  }
  const target = label.trim().toUpperCase();
  const lines = fs.readFileSync(PREDEF_PAL_FILE, "utf-8").split(/\r?\n/);
  for (const raw of lines) {
    if (!raw.includes("RGB") || !raw.includes(";")) {
      continue;
    }
    const [rgbPart, comment] = raw.split(";", 2);
    if (comment.trim().toUpperCase() !== target) {
      continue;
    }
    const comps = rgbPart.split("RGB", 2)[1].split(",").map((value) => Number(value.trim()));
    if (comps.length !== 12) {
      throw new Error(`Palette ${label} must contain four colours`);
    }
    const colours: Array<[number, number, number]> = [];
    for (let idx = 0; idx < comps.length; idx += 3) {
      const [r, g, b] = comps.slice(idx, idx + 3);
      colours.push([gbc5To8(r, `${label} r`), gbc5To8(g, `${label} g`), gbc5To8(b, `${label} b`)]);
    }
    return colours;
  }
  throw new Error(`Palette ${label} not found in ${PREDEF_PAL_FILE}`);
};

// ---------------------------------------------------------------------------
// 2bpp decoding helpers
// ---------------------------------------------------------------------------

const loadTilePixels = (stem: string, { preferPng = false }: { preferPng?: boolean } = {}): TilePixels[] => {
  const suffixes = preferPng ? [".png", ".2bpp", ".2bpp.lz"] : [".2bpp", ".2bpp.lz", ".png"];
  for (const suffix of suffixes) {
    const candidate = joinPath(TRAINER_CARD_DIR, `${stem}${suffix}`);
    if (!assetExists(candidate)) {
      continue;
    }
    if (candidate.endsWith(".png")) {
      return decodePngTilePixels(candidate);
    }
    return decode2bppTilePixels(readTileBytes(candidate));
  }
  throw new Error(`Missing Trainer Card graphics for '${stem}'`);
};

const readTileBytes = (assetPath: string): Buffer => {
  const data = fs.readFileSync(assetPath);
  if (assetPath.endsWith(".lz")) {
    return Buffer.from(decompress(data));
  }
  return data;
};

const loadTileBytes = (stem: string): Buffer => {
  const suffixes = [".2bpp", ".2bpp.lz", ".png"];
  for (const suffix of suffixes) {
    const candidate = joinPath(TRAINER_CARD_DIR, `${stem}${suffix}`);
    if (!assetExists(candidate)) {
      continue;
    }
    if (candidate.endsWith(".png")) {
      throw new Error(`PNG decoding not supported for ${candidate}`);
    }
    return readTileBytes(candidate);
  }
  throw new Error(`Missing Trainer Card graphics for '${stem}'`);
};

const decode2bppTilePixels = (data: Buffer): TilePixels[] => {
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

const decodePngTilePixels = (assetPath: string): TilePixels[] => {
  const surface = loadImageSync(assetPath);
  if (!surface) {
    throw new Error(`PNG decoding not supported for ${assetPath}`);
  }
  const width = surface.get_width();
  const height = surface.get_height();
  if (width % TILE_SIZE !== 0 || height % TILE_SIZE !== 0) {
    throw new Error(`Trainer Card PNG ${assetPath} must align to ${TILE_SIZE}x${TILE_SIZE} tiles.`);
  }
  const tiles: TilePixels[] = [];
  for (let top = 0; top < height; top += TILE_SIZE) {
    for (let left = 0; left < width; left += TILE_SIZE) {
      const pixels: number[] = [];
      for (let row = 0; row < TILE_SIZE; row += 1) {
        for (let col = 0; col < TILE_SIZE; col += 1) {
          const [r, g, b, a] = surface.getAt(left + col, top + row);
          if (a === 0) {
            pixels.push(0);
            continue;
          }
          const luminance = Math.round((r + g + b) / 3);
          const index = Math.max(0, Math.min(3, Math.round((255 - luminance) / 85)));
          pixels.push(index);
        }
      }
      tiles.push(pixels);
    }
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

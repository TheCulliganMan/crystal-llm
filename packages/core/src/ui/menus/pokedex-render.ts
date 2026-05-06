// ASM mapping: pokecrystal_disassembly/engine/pokedex/pokedex.asm (Pokédex rendering helpers).
import fs from "fs";
import path from "path";
import { Rect, Surface } from "@pokecrystal/core/ui/surface";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { gbcWordToRgb } from "@pokecrystal/core/core/gbc-colors";
import { NUM_UNOWN } from "@pokecrystal/core/core/constants";
import { DexMode } from "@pokecrystal/core/core/enums/pokedex";
import { buildDefaultCharMap } from "@pokecrystal/core/ui/text/glyph-map";
import { renderFontText } from "@pokecrystal/core/ui/text/render-font";
import {
  COLOR_1,
  COLOR_2,
  FIRST_UNOWN_CHAR,
  type PokedexFontSource,
  SLOWPOKE_FRAME_COUNT,
  SLOWPOKE_TILE_BASES,
  SLOWPOKE_TILE_OFFSETS,
  cursorTile,
  ensurePokedexCursorTiles,
  ensurePokedexTiles,
  ensureUnownFontTiles,
  getPokedexHardwareState,
  getQuestionMarkPalette,
  prepareTileSurface,
  requirePokedexTile,
  resetPokedexHardwareState,
  slowpokeTiles,
  tintPokedexSprite,
} from "./pokedex-assets";

export { ensurePokedexTiles } from "./pokedex-assets";
import {
  CURSOR_NEW,
  CURSOR_NEW_SEARCH_RESULTS,
  CURSOR_OLD,
  CURSOR_OLD_TOP,
  CursorSprite,
  CURSOR_ROW_HEIGHT,
  getPokedexScrollbarOAMEntry,
  OAM_XFLIP,
  OAM_YFLIP,
  PokedexCursorVariant,
  SPRITE_X_OFFSET,
  SPRITE_Y_OFFSET,
} from "./pokedex-cursor";

const TILE_SIZE = 8;
const CHAR_MAP = buildDefaultCharMap();

const resolveFontTiles = (font: FontLike): Record<number, Surface> => {
  return font.font_tiles ?? font.fontTiles ?? {};
};

const flipSurface = (surface: Surface, flipX: boolean, flipY: boolean): Surface => {
  if (!flipX && !flipY) {
    return surface;
  }
  const [width, height] = surface.get_size();
  const flipped = new Surface(width, height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const sourceX = flipX ? width - 1 - x : x;
      const sourceY = flipY ? height - 1 - y : y;
      flipped.set_at([x, y], surface.get_at([sourceX, sourceY]));
    }
  }
  return flipped;
};

type FontLike = {
  render_text?: (
    text: string,
    x: number,
    y: number,
    surface: Surface | import("@pokecrystal/core/ui/font-renderer").SurfaceLike,
    options?: import("@pokecrystal/core/ui/font-renderer").RenderTextOptions
  ) => void;
  renderText: (
    text: string,
    x: number,
    y: number,
    surface: Surface | import("@pokecrystal/core/ui/font-renderer").SurfaceLike,
    options?: boolean | import("@pokecrystal/core/ui/font-renderer").RenderTextOptions
  ) => void;
  font_tiles?: Record<number, Surface>;
  fontTiles?: Record<number, Surface>;
  getCharTile?: (char: string) => Surface | null | undefined;
};

type UI = PokedexFontSource & {
  font: FontLike;
  getPokemonFrontSurface?: (speciesId: string, frame?: number) => Surface | null;
};

export type DexEntryLike = {
  pokedexNumber: number;
  species: { id: string };
};

const BACKGROUND_TILE_INDEX = 0x32;
const SEPARATOR_TILES = [0x59, 0x5a, 0x53, 0x54, 0x5b] as const;
const FOOTPRINT_CACHE = new Map<string, Surface>();
const SPECIES_PALETTE_CACHE = new Map<string, Array<[number, number, number]>>();
let QUESTION_MARK_TILES: Surface[] | null = null;
let QUESTION_MARK_SURFACE: Surface | null = null;
const SPRITE_SLOT_ORIGIN: [number, number] = [TILE_SIZE, TILE_SIZE];
const SPRITE_SLOT_SIZE = 7 * TILE_SIZE;
const ENTRY_FOOTPRINT_ORIGIN: [number, number] = [17 * TILE_SIZE, TILE_SIZE];
const ENTRY_BORDER_HEIGHT = 15;
const ENTRY_BORDER_WIDTH = 18;
const SPACE_TILE_INDEX = 0x7f;
export const ENTRY_DIVIDER_COLOR: [number, number, number] = [32, 32, 32];
const SCREEN_TILE_WIDTH = 20;
const TEXT_AREA_LEFT_OFFSET = 1;
const TEXT_AREA_LINES = 5;
const TEXT_AREA_WIDTH_TILES = SCREEN_TILE_WIDTH - 2;
const QUESTION_MARK_PALETTE = getQuestionMarkPalette();
const ENTRY_MENU_TEXT = "PAGE AREA CRY PRNT";
const ENTRY_ACTION_CURSOR_TILES = [0, 5, 10, 14] as const;
const SIDEBAR_GREEN_PALETTE: ReadonlyArray<readonly [number, number, number]> = [
  [224, 248, 208],
  [136, 192, 112],
  [52, 104, 86],
  [8, 24, 32],
];
const SELECT_OPTION_TILES = [0x3b, 0x48, 0x49, 0x4a, 0x44, 0x45, 0x46, 0x47];
const START_SEARCH_TILES = [0x3c, 0x3b, 0x41, 0x42, 0x43, 0x4b, 0x4c, 0x4d, 0x4e, 0x3c];
const OPTION_LABELS: Record<DexMode, string> = {
  [DexMode.NEW]: "NEW #DEX MODE",
  [DexMode.OLD]: "OLD #DEX MODE",
  [DexMode.ABC]: "A to Z MODE",
  [DexMode.UNOWN]: "UNOWN MODE",
};
const OPTION_ROW_MAP: Record<DexMode, number> = {
  [DexMode.NEW]: 4,
  [DexMode.OLD]: 6,
  [DexMode.ABC]: 8,
  [DexMode.UNOWN]: 10,
};
const MODE_DESCRIPTIONS: Record<DexMode, [string, string]> = {
  [DexMode.NEW]: ["<PKMN> are listed by", "evolution type."],
  [DexMode.OLD]: ["<PKMN> are listed by", "official type."],
  [DexMode.ABC]: ["<PKMN> are listed", "alphabetically."],
  [DexMode.UNOWN]: ["UNOWN are listed", "in catching order."],
};

export const UNOWN_LETTER_COORDS: Array<[[number, number], [number, number]]> = [
  [[4, 11], [3, 11]],
  [[4, 10], [3, 10]],
  [[4, 9], [3, 9]],
  [[4, 8], [3, 8]],
  [[4, 7], [3, 7]],
  [[4, 6], [3, 6]],
  [[4, 5], [3, 5]],
  [[4, 4], [3, 4]],
  [[4, 3], [3, 2]],
  [[5, 3], [5, 2]],
  [[6, 3], [6, 2]],
  [[7, 3], [7, 2]],
  [[8, 3], [8, 2]],
  [[9, 3], [9, 2]],
  [[10, 3], [10, 2]],
  [[11, 3], [11, 2]],
  [[12, 3], [12, 2]],
  [[13, 3], [13, 2]],
  [[14, 3], [15, 2]],
  [[14, 4], [15, 4]],
  [[14, 5], [15, 5]],
  [[14, 6], [15, 6]],
  [[14, 7], [15, 7]],
  [[14, 8], [15, 8]],
  [[14, 9], [15, 9]],
  [[14, 10], [15, 10]],
];
export const UNOWN_LETTER_WORDS = [
  "",
  "ANGRY",
  "BEAR",
  "CHASE",
  "DIRECT",
  "ENGAGE",
  "FIND",
  "GIVE",
  "HELP",
  "INCREASE",
  "JOIN",
  "KEEP",
  "LAUGH",
  "MAKE",
  "NUZZLE",
  "OBSERVE",
  "PERFORM",
  "QUICKEN",
  "REASSURE",
  "SEARCH",
  "TELL",
  "UNDO",
  "VANISH",
  "WANT",
  "XXXXX",
  "YIELD",
  "ZOOM",
];

export const POKEDEX_TEXT_COLOR: [number, number, number] = [255, 255, 255];

export const formatHeight = (heightDigits: number): string => {
  const feet = Math.floor(heightDigits / 100);
  const inches = heightDigits % 100;
  return `HT  ${feet}'${String(inches).padStart(2, "0")}"`;
};

export const formatWeight = (weightDigits: number): string => {
  const pounds = weightDigits / 10;
  const formatted = pounds.toFixed(1).padStart(5, " ");
  return `WT   ${formatted}lb`;
};

const charTileIndex = (char: string): number => {
  const tileIndex = CHAR_MAP[char];
  if (tileIndex === undefined) {
    throw new Error(`Character ${JSON.stringify(char)} is not supported by the font.`);
  }
  return tileIndex;
};

const blitFontTileAt = (ui: UI, screen: Surface, tileIndex: number, tileX: number, tileY: number): void => {
  const tile = requirePokedexTile(ui, tileIndex);
  screen.blit(tile, [tileX * TILE_SIZE, tileY * TILE_SIZE]);
};

const drawTileString = (ui: UI, screen: Surface, text: string, tileX: number, tileY: number): void => {
  for (let index = 0; index < text.length; index += 1) {
    const char = text[index] ?? " ";
    if (char === " ") {
      continue;
    }
    blitFontTileAt(ui, screen, charTileIndex(char), tileX + index, tileY);
  }
};

const opaqueWhiteGlyphTile = (tile: Surface): Surface => {
  const [width, height] = tile.get_size();
  const opaque = new Surface(width, height);
  opaque.fill([0, 0, 0, 255]);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const [_r, _g, _b, a] = tile.get_at([x, y]);
      if (a > 0) {
        opaque.set_at([x, y], [255, 255, 255, 255]);
      }
    }
  }
  return opaque;
};

const drawOpaqueFooterString = (ui: UI, screen: Surface, text: string, tileX: number, tileY: number): void => {
  for (let index = 0; index < text.length; index += 1) {
    const char = text[index] ?? " ";
    if (char === " ") {
      continue;
    }
    const tile = requirePokedexTile(ui, charTileIndex(char));
    screen.blit(opaqueWhiteGlyphTile(tile), [(tileX + index) * TILE_SIZE, tileY * TILE_SIZE]);
  }
};

const drawEntryActionLabel = (
  ui: UI,
  screen: Surface,
  text: string,
  tileX: number,
  tileY: number
): void => {
  drawOpaqueFooterString(ui, screen, text, tileX, tileY);
};

const drawHeightLine = (
  ui: UI,
  screen: Surface,
  tileX: number,
  tileY: number,
  heightDigits: number,
  isCaught: boolean
): void => {
  renderFontText(ui.font, "HT", tileX * TILE_SIZE, tileY * TILE_SIZE, screen, {
    color: POKEDEX_TEXT_COLOR,
    uppercase: false,
  });
  blitFontTileAt(ui, screen, 0x5e, tileX + 5, tileY);
  blitFontTileAt(ui, screen, 0x5f, tileX + 8, tileY);
  if (!isCaught) {
    renderFontText(ui.font, "?", (tileX + 4) * TILE_SIZE, tileY * TILE_SIZE, screen, {
      color: POKEDEX_TEXT_COLOR,
      uppercase: false,
    });
    renderFontText(ui.font, "??", (tileX + 6) * TILE_SIZE, tileY * TILE_SIZE, screen, {
      color: POKEDEX_TEXT_COLOR,
      uppercase: false,
    });
    return;
  }
  const feet = Math.floor(heightDigits / 100);
  const inches = heightDigits % 100;
  renderFontText(ui.font, `${feet}`.padStart(2, " "), (tileX + 3) * TILE_SIZE, tileY * TILE_SIZE, screen, {
    color: POKEDEX_TEXT_COLOR,
    uppercase: false,
  });
  renderFontText(ui.font, `${String(inches).padStart(2, "0")}`, (tileX + 6) * TILE_SIZE, tileY * TILE_SIZE, screen, {
    color: POKEDEX_TEXT_COLOR,
    uppercase: false,
  });
};

const drawWeightLine = (
  ui: UI,
  screen: Surface,
  tileX: number,
  tileY: number,
  weightDigits: number,
  isCaught: boolean
): void => {
  renderFontText(ui.font, "WT", tileX * TILE_SIZE, tileY * TILE_SIZE, screen, {
    color: POKEDEX_TEXT_COLOR,
    uppercase: false,
  });
  if (!isCaught) {
    renderFontText(ui.font, "???lb", (tileX + 3) * TILE_SIZE, tileY * TILE_SIZE, screen, {
      color: POKEDEX_TEXT_COLOR,
      uppercase: false,
    });
    return;
  }
  renderFontText(ui.font, `${(weightDigits / 10).toFixed(1).padStart(5, " ")}lb`, (tileX + 2) * TILE_SIZE, tileY * TILE_SIZE, screen, {
    color: POKEDEX_TEXT_COLOR,
    uppercase: false,
  });
};

const decode1bppFootprint = (data: Buffer): Surface => {
  if (data.length !== 32) {
    throw new Error(`Footprint 1bpp payload must be 32 bytes, got ${data.length}`);
  }
  const surface = new Surface(16, 16);
  surface.fill([0, 0, 0, 255]);
  for (let tileIndex = 0; tileIndex < 4; tileIndex += 1) {
    const base = tileIndex * 8;
    const tileX = (tileIndex % 2) * TILE_SIZE;
    const tileY = Math.floor(tileIndex / 2) * TILE_SIZE;
    for (let row = 0; row < TILE_SIZE; row += 1) {
      const byte = data[base + row];
      for (let col = 0; col < TILE_SIZE; col += 1) {
        const bit = 7 - col;
        const darkPixel = ((byte >> bit) & 1) === 1;
        surface.set_at([tileX + col, tileY + row], darkPixel ? [0, 0, 0, 255] : [255, 255, 255, 255]);
      }
    }
  }
  return surface;
};

const SIDEBAR_SLOT_FILL_COLOR: [number, number, number, number] = [
  SIDEBAR_GREEN_PALETTE[0]?.[0] ?? 224,
  SIDEBAR_GREEN_PALETTE[0]?.[1] ?? 248,
  SIDEBAR_GREEN_PALETTE[0]?.[2] ?? 208,
  255,
];

const loadPngSurface = (filePath: string): Surface | null =>
  gameEngine.image.loadSync?.(filePath) ?? null;

export const loadFootprintSurface = (ui: UI, speciesId: string): Surface => {
  const cacheKey = speciesId.toUpperCase();
  const cached = FOOTPRINT_CACHE.get(cacheKey);
  if (cached) {
    return cached;
  }
  const footprintPath = path.join(getAssetPath("gfx", "footprints"), `${speciesId.toLowerCase()}.1bpp`);
  let surface: Surface | null = null;
  if (fs.existsSync(footprintPath)) {
    surface = decode1bppFootprint(fs.readFileSync(footprintPath));
  } else {
    const pngPath = footprintPath.replace(/\.1bpp$/, ".png");
    const png = fs.existsSync(pngPath) ? loadPngSurface(pngPath) : null;
    if (png) {
      surface = new Surface(16, 16);
      surface.fill([255, 255, 255, 255]);
      for (let y = 0; y < 16; y += 1) {
        for (let x = 0; x < 16; x += 1) {
          const [r, g, b, a] = png.get_at([x, y]);
          const dark = a > 0 && r + g + b < 384;
          surface.set_at([x, y], dark ? [0, 0, 0, 255] : [255, 255, 255, 255]);
        }
      }
    }
  }
  if (!surface) {
    throw new Error(`Missing Pokédex footprint for ${speciesId} (${footprintPath}).`);
  }
  FOOTPRINT_CACHE.set(cacheKey, surface);
  return surface;
};

const loadQuestionMarkTiles = (): Surface[] => {
  if (QUESTION_MARK_TILES) {
    return QUESTION_MARK_TILES;
  }
  const markPath = path.join(getAssetPath("gfx", "pokedex"), "question_mark.2bpp");
  if (!fs.existsSync(markPath)) {
    const pngPath = markPath.replace(/\.2bpp$/, ".png");
    const png = fs.existsSync(pngPath) ? loadPngSurface(pngPath) : null;
    if (!png) {
      throw new Error(`Missing Pokédex question mark asset (${markPath}).`);
    }
    const tiles: Surface[] = [];
    for (let col = 0; col < 7; col += 1) {
      for (let row = 0; row < 7; row += 1) {
        tiles.push(
          tintPokedexSprite(
            png.subsurface(new Rect(col * TILE_SIZE, row * TILE_SIZE, TILE_SIZE, TILE_SIZE)),
            QUESTION_MARK_PALETTE
          )
        );
      }
    }
    QUESTION_MARK_TILES = tiles;
    return QUESTION_MARK_TILES;
  }
  const data = fs.readFileSync(markPath);
  if (data.length % 16 !== 0) {
    throw new Error(`Question mark tileset must be 2bpp aligned, got ${data.length}`);
  }
  const tileCount = data.length / 16;
  const tilesPerRow = 7;
  if (tileCount !== tilesPerRow * tilesPerRow) {
    throw new Error(`Question mark tileset must contain 49 tiles, got ${tileCount}`);
  }
  const tiles: Surface[] = [];
  for (let tileIndex = 0; tileIndex < tileCount; tileIndex += 1) {
    const tile = new Surface(TILE_SIZE, TILE_SIZE);
    const base = tileIndex * 16;
    for (let row = 0; row < TILE_SIZE; row += 1) {
      const plane0 = data[base + row * 2];
      const plane1 = data[base + row * 2 + 1];
      for (let col = 0; col < TILE_SIZE; col += 1) {
        const bit = 7 - col;
        const idx = ((plane0 >> bit) & 1) | (((plane1 >> bit) & 1) << 1);
        const level = [255, 170, 85, 0][idx] ?? 0;
        tile.set_at([col, row], [level, level, level, 255]);
      }
    }
    tiles.push(tintPokedexSprite(tile, QUESTION_MARK_PALETTE));
  }
  QUESTION_MARK_TILES = tiles;
  return QUESTION_MARK_TILES;
};

const loadQuestionMarkSurface = (): Surface => {
  if (QUESTION_MARK_SURFACE) {
    return QUESTION_MARK_SURFACE;
  }
  const tilesPerAxis = 7;
  const surface = new Surface(tilesPerAxis * TILE_SIZE, tilesPerAxis * TILE_SIZE);
  const tiles = loadQuestionMarkTiles();
  for (let row = 0; row < tilesPerAxis; row += 1) {
    for (let col = 0; col < tilesPerAxis; col += 1) {
      // ASM parity: Pokedex_PlaceFrontpicAtHL writes tile ids in a 7-tile stride,
      // so the decompressed frontpic data is effectively column-major in VRAM.
      const tile = tiles[row + col * tilesPerAxis];
      if (!tile) {
        throw new Error(`Missing Pokédex question mark tile at row ${row}, col ${col}.`);
      }
      surface.blit(tile, [col * TILE_SIZE, row * TILE_SIZE]);
    }
  }
  QUESTION_MARK_SURFACE = surface;
  return QUESTION_MARK_SURFACE;
};

const loadSpeciesNormalPalette = (speciesId: string): Array<[number, number, number]> | null => {
  const normalized = String(speciesId || "").trim().toLowerCase();
  if (!normalized) {
    return null;
  }
  const cached = SPECIES_PALETTE_CACHE.get(normalized);
  if (cached) {
    return cached;
  }
  const baseSpecies = normalized.includes("_") ? normalized.split("_")[0] : normalized;
  const palettePath = [
    getAssetPath("gfx", "pokemon", normalized, "normal.gbcpal"),
    getAssetPath("gfx", "pokemon", normalized, "front.gbcpal"),
    getAssetPath("gfx", "pokemon", baseSpecies, "normal.gbcpal"),
  ].find((candidate) => fs.existsSync(candidate));
  if (!palettePath) {
    return null;
  }
  const data = fs.readFileSync(palettePath);
  if (data.length < 8) {
    throw new Error(`Pokemon palette ${palettePath} must contain at least four GBC colours.`);
  }
  const palette = [
    gbcWordToRgb(data.readUInt16LE(0)),
    gbcWordToRgb(data.readUInt16LE(2)),
    gbcWordToRgb(data.readUInt16LE(4)),
    gbcWordToRgb(data.readUInt16LE(6)),
  ];
  SPECIES_PALETTE_CACHE.set(normalized, palette);
  return palette;
};

const colorDistance = (
  [r, g, b]: readonly [number, number, number],
  [targetR, targetG, targetB]: readonly [number, number, number],
): number => {
  const dr = r - targetR;
  const dg = g - targetG;
  const db = b - targetB;
  return dr * dr + dg * dg + db * db;
};

const nearestPaletteColor = (
  color: readonly [number, number, number],
  palette: ReadonlyArray<readonly [number, number, number]>,
): readonly [number, number, number] => {
  let best = palette[0] ?? color;
  let bestDistance = colorDistance(color, best);
  for (let index = 1; index < palette.length; index += 1) {
    const candidate = palette[index] ?? best;
    const distance = colorDistance(color, candidate);
    if (distance < bestDistance) {
      best = candidate;
      bestDistance = distance;
    }
  }
  return best;
};

const normalizePokedexFrontSprite = (speciesId: string, sprite: Surface): Surface => {
  const prepared = prepareTileSurface(sprite);
  const palette = loadSpeciesNormalPalette(speciesId);
  if (!palette) {
    return prepared;
  }
  const [width, height] = prepared.get_size();
  const normalized = new Surface(width, height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const [r, g, b, a] = prepared.get_at([x, y]);
      if (a === 0) {
        continue;
      }
      const [mappedR, mappedG, mappedB] = nearestPaletteColor([r, g, b], palette);
      normalized.set_at([x, y], [mappedR, mappedG, mappedB, a]);
    }
  }
  return normalized;
};

export const fillBackgroundTiles = (
  ui: UI,
  screen: Surface,
  fillColor: [number, number, number] | null = null
): void => {
  ensurePokedexTiles(ui);
  if (fillColor) {
    screen.fill([fillColor[0], fillColor[1], fillColor[2], 255]);
    return;
  }
  const tile = requirePokedexTile(ui, BACKGROUND_TILE_INDEX);
  const [surfaceWidth, surfaceHeight] = screen.get_size();
  const widthTiles = Math.ceil(surfaceWidth / TILE_SIZE);
  const heightTiles = Math.ceil(surfaceHeight / TILE_SIZE);
  for (let y = 0; y < heightTiles; y += 1) {
    for (let x = 0; x < widthTiles; x += 1) {
      screen.blit(tile, [x * TILE_SIZE, y * TILE_SIZE]);
    }
  }
};

const drawPokedexBorder = (
  ui: UI,
  screen: Surface,
  originX: number,
  originY: number,
  interiorWidth: number,
  interiorHeight: number
): void => {
  const borderTiles = [0x33, 0x34, 0x35, 0x36, 0x7f, 0x37, 0x38, 0x39, 0x3a];
  const [
    topLeft,
    topEdge,
    topRight,
    leftEdge,
    center,
    rightEdge,
    bottomLeft,
    bottomEdge,
    bottomRight,
  ] = borderTiles.map((tileId) => requirePokedexTile(ui, tileId));
  const widthTiles = interiorWidth + 2;
  const heightTiles = interiorHeight + 2;

  screen.blit(topLeft, [originX, originY]);
  for (let col = 1; col < widthTiles - 1; col += 1) {
    screen.blit(topEdge, [originX + col * TILE_SIZE, originY]);
  }
  screen.blit(topRight, [originX + (widthTiles - 1) * TILE_SIZE, originY]);

  for (let row = 1; row < heightTiles - 1; row += 1) {
    const y = originY + row * TILE_SIZE;
    screen.blit(leftEdge, [originX, y]);
    for (let col = 1; col < widthTiles - 1; col += 1) {
      screen.blit(center, [originX + col * TILE_SIZE, y]);
    }
    screen.blit(rightEdge, [originX + (widthTiles - 1) * TILE_SIZE, y]);
  }

  const bottomY = originY + (heightTiles - 1) * TILE_SIZE;
  screen.blit(bottomLeft, [originX, bottomY]);
  for (let col = 1; col < widthTiles - 1; col += 1) {
    screen.blit(bottomEdge, [originX + col * TILE_SIZE, bottomY]);
  }
  screen.blit(bottomRight, [originX + (widthTiles - 1) * TILE_SIZE, bottomY]);
};

const fillTileBlock = (
  ui: UI,
  screen: Surface,
  tileIndex: number,
  originX: number,
  originY: number,
  widthTiles: number,
  heightTiles: number
): void => {
  const tile = requirePokedexTile(ui, tileIndex);
  for (let row = 0; row < heightTiles; row += 1) {
    for (let col = 0; col < widthTiles; col += 1) {
      screen.blit(tile, [originX + col * TILE_SIZE, originY + row * TILE_SIZE]);
    }
  }
};

export const drawSeparatorColumn = (ui: UI, screen: Surface): void => {
  const [top, middle, midTop, midBottom, bottom] = SEPARATOR_TILES;
  const columnX = 8 * TILE_SIZE;
  const fontTiles = resolveFontTiles(ui.font);
  const blit = (tileIndex: number, row: number): void => {
    const tileSurface = fontTiles[tileIndex];
    if (tileSurface) {
      screen.blit(tileSurface, [columnX, row * TILE_SIZE]);
    }
  };
  blit(top, 0);
  for (let row = 1; row < 8; row += 1) {
    blit(middle, row);
  }
  blit(midTop, 8);
  blit(midBottom, 9);
  for (let row = 10; row < 16; row += 1) {
    blit(middle, row);
  }
  blit(bottom, 16);
};

// ASM: engine/pokedex/pokedex.asm::Pokedex_DrawSearchResultsScreenBG separator column.
const drawSearchResultsSeparatorColumn = (ui: UI, screen: Surface): void => {
  const columnX = 8 * TILE_SIZE;
  blitFontTileAt(ui, screen, 0x59, 8, 0);
  const middleTile = requirePokedexTile(ui, 0x5a);
  for (let row = 1; row < 8; row += 1) {
    screen.blit(middleTile, [columnX, row * TILE_SIZE]);
  }
  blitFontTileAt(ui, screen, 0x53, 8, 8);
  blitFontTileAt(ui, screen, 0x69, 8, 9);
  blitFontTileAt(ui, screen, 0x6a, 8, 10);
};

// ASM: engine/pokedex/pokedex.asm::Pokedex_DrawMainScreenBG.
export const drawMainSidebar = (
  ui: UI,
  screen: Surface,
  {
    seenCount,
    caughtCount,
    activeSpeciesId,
    showQuestionMark = false,
  }: {
    seenCount: number;
    caughtCount: number;
    activeSpeciesId: string | null;
    showQuestionMark?: boolean;
  }
): void => {
  ensurePokedexTiles(ui);
  fillBackgroundTiles(ui, screen);
  drawPokedexBorder(ui, screen, 0, 0, 7, 7);
  drawPokedexBorder(ui, screen, 0, 9 * TILE_SIZE, 7, 6);
  renderFontText(ui.font, "SEEN", TILE_SIZE, 11 * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
  renderFontText(ui.font, `${seenCount}`.padStart(3, "0"), 5 * TILE_SIZE, 12 * TILE_SIZE, screen, {
    color: POKEDEX_TEXT_COLOR,
  });
  renderFontText(ui.font, "OWN", TILE_SIZE, 14 * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
  renderFontText(ui.font, `${caughtCount}`.padStart(3, "0"), 5 * TILE_SIZE, 15 * TILE_SIZE, screen, {
    color: POKEDEX_TEXT_COLOR,
  });
  SELECT_OPTION_TILES.forEach((tileId, offset) => {
    blitFontTileAt(ui, screen, tileId, 1 + offset, 17);
  });
  START_SEARCH_TILES.forEach((tileId, offset) => {
    blitFontTileAt(ui, screen, tileId, 1 + SELECT_OPTION_TILES.length + offset, 17);
  });

  if (activeSpeciesId) {
    // ASM: engine/gfx/cgb_layouts.asm::_CGB_Pokedex loads the selected mon's
    // species palette into the 7x7 scrolling preview box. Only unseen entries
    // use PokedexQuestionMarkPalette in this slot.
    drawFrontSprite(ui, screen, activeSpeciesId, SPRITE_SLOT_ORIGIN, false, SIDEBAR_SLOT_FILL_COLOR);
  } else if (showQuestionMark) {
    drawFrontSprite(ui, screen, null, SPRITE_SLOT_ORIGIN, true);
  }
  drawSeparatorColumn(ui, screen);
};

// ASM: engine/pokedex/pokedex.asm::Pokedex_DrawSearchResultsScreenBG.
export const drawSearchResultsBackground = (
  ui: UI,
  screen: Surface,
  {
    resultCount,
    activeSpeciesId,
    showQuestionMark = false,
  }: {
    resultCount: number;
    activeSpeciesId: string | null;
    showQuestionMark?: boolean;
  }
): void => {
  ensurePokedexTiles(ui);
  fillBackgroundTiles(ui, screen);
  drawPokedexBorder(ui, screen, 0, 0, 7, 7);
  drawPokedexBorder(ui, screen, 0, 11 * TILE_SIZE, 18, 5);
  renderFontText(ui.font, "SEARCH RESULTS", TILE_SIZE, 12 * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
  renderFontText(ui.font, "  TYPE", TILE_SIZE, 13 * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
  renderFontText(ui.font, "    FOUND!", TILE_SIZE, 14 * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
  renderFontText(ui.font, `${resultCount}`.padStart(3, "0"), TILE_SIZE, 16 * TILE_SIZE, screen, {
    color: POKEDEX_TEXT_COLOR,
  });
  drawSearchResultsSeparatorColumn(ui, screen);

  if (activeSpeciesId) {
    // ASM: engine/gfx/cgb_layouts.asm::_CGB_Pokedex writes the currently
    // selected species palette to the search-results preview box as well.
    drawFrontSprite(ui, screen, activeSpeciesId, SPRITE_SLOT_ORIGIN, false, SIDEBAR_SLOT_FILL_COLOR);
  } else if (showQuestionMark) {
    drawFrontSprite(ui, screen, null, SPRITE_SLOT_ORIGIN, true);
  }
};

export const drawOptionScreen = (
  ui: UI,
  screen: Surface,
  modes: DexMode[],
  cursorIndex: number,
  modeChangeMessage: [string, string] | null,
  showArrowCursor: boolean = true
): void => {
  if (!screen) {
    return;
  }
  ensurePokedexTiles(ui);
  fillBackgroundTiles(ui, screen);
  drawPokedexBorder(ui, screen, 0, 2 * TILE_SIZE, 18, 8);
  drawPokedexBorder(ui, screen, 0, 12 * TILE_SIZE, 18, 4);
  const title = " OPTION ";
  blitFontTileAt(ui, screen, 0x3b, 0, 1);
  renderFontText(ui.font, title, TILE_SIZE, TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
  blitFontTileAt(ui, screen, 0x3c, 1 + title.length, 1);

  const cursor = modes.length ? Math.min(cursorIndex, modes.length - 1) : 0;
  modes.forEach((mode, index) => {
    const row = OPTION_ROW_MAP[mode];
    const label = OPTION_LABELS[mode];
    renderFontText(ui.font, label, 3 * TILE_SIZE, row * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
    if (index === cursor && showArrowCursor) {
      renderFontText(ui.font, "\u25b6", 2 * TILE_SIZE, row * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
    }
  });

  if (!modes.length) {
    return;
  }
  const description = modeChangeMessage ?? MODE_DESCRIPTIONS[modes[cursor]];
  description.forEach((line, index) => {
    renderFontText(ui.font, line, TILE_SIZE, (14 + index) * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
  });
};

const drawFrontSprite = (
  ui: UI,
  screen: Surface,
  speciesId: string | null,
  origin: [number, number],
  usePlaceholder: boolean = false,
  slotFillColor: [number, number, number, number] | null = null,
  alignShortSpriteOneTileUp = false,
): void => {
  let spriteSurface: Surface | null = null;
  if (usePlaceholder) {
    spriteSurface = loadQuestionMarkSurface();
  } else if (speciesId) {
    const sprite = ui.getPokemonFrontSurface?.(speciesId, 0);
    if (sprite) {
      spriteSurface = normalizePokedexFrontSprite(speciesId, sprite);
    }
  }
  if (!spriteSurface) {
    return;
  }
  if (slotFillColor) {
    const [originX, originY] = origin;
    screen.fill(slotFillColor, { x: originX, y: originY, width: SPRITE_SLOT_SIZE, height: SPRITE_SLOT_SIZE });
  } else {
    fillTileBlock(ui, screen, SPACE_TILE_INDEX, origin[0], origin[1], 7, 7);
  }
  const spriteWidth = spriteSurface.get_width();
  const spriteHeight = spriteSurface.get_height();
  const slotRect = new Rect(origin[0], origin[1], SPRITE_SLOT_SIZE, SPRITE_SLOT_SIZE);
  const spriteX = slotRect.left + Math.floor((slotRect.width - spriteWidth) / 2);
  const spriteYOffset = alignShortSpriteOneTileUp && spriteHeight < slotRect.height ? -TILE_SIZE : 0;
  const spriteY = slotRect.top + Math.floor((slotRect.height - spriteHeight) / 2) + spriteYOffset;
  screen.blit(spriteSurface, [spriteX, spriteY]);
};

const drawCursorSprites = (screen: Surface, sprites: CursorSprite[], row: number): void => {
  const rowOffset = row * CURSOR_ROW_HEIGHT;
  sprites.forEach(([xTile, yTile, xOffset, yOffset, tileId, attr]) => {
    const x = xTile * TILE_SIZE + xOffset - SPRITE_X_OFFSET;
    const y = yTile * TILE_SIZE + yOffset + rowOffset - SPRITE_Y_OFFSET;
    let tile = cursorTile(tileId);
    const flipX = Boolean(attr & OAM_XFLIP);
    const flipY = Boolean(attr & OAM_YFLIP);
    if (flipX || flipY) {
      tile = flipSurface(tile, flipX, flipY);
    }
    screen.blit(tile, [x, y]);
  });
};

const drawScrollbarSprite = (
  screen: Surface,
  cursorIndex: number,
  scrollOffset: number,
  listingEnd: number
): void => {
  if (listingEnd <= 0) {
    return;
  }
  const oam = getPokedexScrollbarOAMEntry(cursorIndex, scrollOffset, listingEnd);
  if (!oam) {
    return;
  }
  screen.blit(cursorTile(oam.tileId), [oam.x - SPRITE_X_OFFSET, oam.y - SPRITE_Y_OFFSET]);
};

export const drawPokedexCursorOverlay = (
  ui: UI,
  screen: Surface,
  dexMode: DexMode,
  cursorIndex: number,
  scrollOffset: number,
  listingHeight: number,
  listingEnd: number,
  variant: PokedexCursorVariant = "main",
): void => {
  if (!screen) {
    return;
  }
  const row = cursorIndex - scrollOffset;
  if (row < 0 || row >= listingHeight) {
    return;
  }
  ensurePokedexCursorTiles(ui);
  const sprites =
    dexMode === DexMode.OLD
      ? cursorIndex === 0
        ? CURSOR_OLD_TOP
        : CURSOR_OLD
      : variant === "search_results"
        ? CURSOR_NEW_SEARCH_RESULTS
        : CURSOR_NEW;
  drawCursorSprites(screen, sprites, row);
  if (dexMode !== DexMode.OLD && variant === "main") {
    drawScrollbarSprite(screen, cursorIndex, scrollOffset, listingEnd);
  }
};

export const drawEntryPage = (
  ui: UI,
  screen: Surface,
  entry: DexEntryLike,
  entryData: {
    classification?: string;
    heightDigits: number;
    weightDigits: number;
    pages: string[];
  },
  pageIndex: number,
  actionIndex: number,
  actionLabels: string[],
  actionCoordinates: Array<[number, number]>,
  {
    isCaught = true,
    showArrowCursor = true,
  }: {
    isCaught?: boolean;
    showArrowCursor?: boolean;
  } = {}
): void => {
  if (!screen) {
    return;
  }
  ensurePokedexTiles(ui);
  drawEntryBackground(ui, screen);

  const nameX = 9 * TILE_SIZE;
  const speciesId = entry.species.id;
  renderFontText(ui.font, speciesId, nameX, 3 * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
  const classification = entryData.classification ?? "";
  if (classification) {
    renderFontText(ui.font, classification, nameX, 5 * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
  }

  blitFontTileAt(ui, screen, 0x5c, 2, 8);
  blitFontTileAt(ui, screen, 0x5d, 3, 8);
  renderFontText(ui.font, String(entry.pokedexNumber).padStart(3, "0"), 4 * TILE_SIZE, 8 * TILE_SIZE, screen, {
    color: POKEDEX_TEXT_COLOR,
  });
  drawPageBadge(ui, screen, pageIndex);

  drawHeightLine(ui, screen, Math.floor(nameX / TILE_SIZE), 7, entryData.heightDigits, isCaught);
  drawWeightLine(ui, screen, Math.floor(nameX / TILE_SIZE), 9, entryData.weightDigits, isCaught);

  renderFontText(ui.font, entryData.pages[pageIndex], TEXT_AREA_LEFT_OFFSET * TILE_SIZE, 11 * TILE_SIZE, screen, {
    color: POKEDEX_TEXT_COLOR,
    textWidth: TEXT_AREA_WIDTH_TILES * TILE_SIZE,
    maxLines: TEXT_AREA_LINES,
    uppercase: false,
  });

  drawEntryActionRow(ui, screen, actionIndex, actionLabels, actionCoordinates, showArrowCursor);
  drawFrontSprite(ui, screen, speciesId, SPRITE_SLOT_ORIGIN, false, [255, 255, 255, 255]);

  const footprintSurface = loadFootprintSurface(ui, speciesId);
  screen.blit(footprintSurface, ENTRY_FOOTPRINT_ORIGIN);
};

const drawEntryBackground = (ui: UI, screen: Surface): void => {
  fillBackgroundTiles(ui, screen);
  drawPokedexBorder(ui, screen, 0, 0, ENTRY_BORDER_WIDTH, ENTRY_BORDER_HEIGHT);
  drawEntryRightColumn(ui, screen);
  // ASM: engine/pokedex/pokedex.asm::Pokedex_DrawDexEntryScreenBG
  // The lower divider crosses through the right strip before the entry body resumes.
  fillTileBlock(ui, screen, 0x61, TILE_SIZE, 10 * TILE_SIZE, SCREEN_TILE_WIDTH - 1, 1);
  fillTileBlock(ui, screen, SPACE_TILE_INDEX, TILE_SIZE, 17 * TILE_SIZE, SCREEN_TILE_WIDTH - 2, 1);
};

const drawEntryRightColumn = (ui: UI, screen: Surface): void => {
  const columnX = SCREEN_TILE_WIDTH - 1;
  // Keep the dedicated right-side strip in place and move the lower-half text
  // left to fit before it.
  blitFontTileAt(ui, screen, 0x66, columnX, 0);
  fillTileBlock(ui, screen, 0x67, columnX * TILE_SIZE, TILE_SIZE, 1, ENTRY_BORDER_HEIGHT);
  blitFontTileAt(ui, screen, 0x68, columnX, ENTRY_BORDER_HEIGHT + 1);
  blitFontTileAt(ui, screen, 0x3c, columnX, 17);
};

const drawPageBadge = (ui: UI, screen: Surface, pageIndex: number): void => {
  blitFontTileAt(ui, screen, 0x55, 1, 9);
  blitFontTileAt(ui, screen, 0x55, 2, 9);
  blitFontTileAt(ui, screen, 0x56, 1, 10);
  const pageTile = pageIndex === 0 ? 0x57 : 0x58;
  blitFontTileAt(ui, screen, pageTile, 2, 10);
};

const drawEntryActionRow = (
  ui: UI,
  screen: Surface,
  actionIndex: number,
  actionLabels: string[],
  actionCoordinates: Array<[number, number]>,
  showArrowCursor: boolean
): void => {
  // ASM: engine/pokedex/pokedex.asm::Pokedex_DrawDexEntryScreenBG .MenuItems
  // draws $3b + " PAGE AREA CRY PRNT". Cursor movement later rewrites the old
  // tile to " " and the new tile to "▶" (Pokedex_MoveArrowCursor).
  blitFontTileAt(ui, screen, SPACE_TILE_INDEX, 0, 17);
  if (actionLabels.length && actionCoordinates.length) {
    actionLabels.forEach((label, index) => {
      const labelX = actionCoordinates[index]?.[0];
      if (labelX !== undefined) {
        drawEntryActionLabel(ui, screen, label, labelX, 17);
      }
    });
  } else {
    drawOpaqueFooterString(ui, screen, ENTRY_MENU_TEXT, 1, 17);
  }
  const cursorTileX = ENTRY_ACTION_CURSOR_TILES[Math.max(0, Math.min(actionIndex, 3))] ?? 0;
  if (showArrowCursor) {
    blitFontTileAt(ui, screen, 0x3b, cursorTileX, 17);
  }
};

export const drawSearchSlowpoke = (
  ui: UI,
  screen: Surface,
  frame: number,
  origin: [number, number] = [11 * TILE_SIZE, 9 * TILE_SIZE]
): void => {
  if (frame < 0 || frame >= SLOWPOKE_FRAME_COUNT) {
    throw new Error(`Invalid slowpoke frame ${frame}`);
  }
  const tiles = slowpokeTiles(ui);
  const [originX, originY] = origin;
  SLOWPOKE_TILE_BASES.forEach((base, index) => {
    const tileIndex = base + frame * 3;
    if (tileIndex >= tiles.length) {
      throw new Error("Missing Pokédex slowpoke animation tile.");
    }
    const [offsetX, offsetY] = SLOWPOKE_TILE_OFFSETS[index];
    screen.blit(tiles[tileIndex], [originX + offsetX, originY + offsetY]);
  });
};

export const drawUnownModeScreen = (
  ui: UI,
  screen: Surface,
  letters: number[],
  cursorIndex: number,
  {
    word = "",
    activeSpeciesId = null,
  }: { word?: string; activeSpeciesId?: string | null } = {}
): void => {
  if (!screen) {
    return;
  }
  ensurePokedexTiles(ui);
  ensureUnownFontTiles(ui);
  fillBackgroundTiles(ui, screen);
  drawPokedexBorder(ui, screen, 2 * TILE_SIZE, 1 * TILE_SIZE, 13, 10);
  drawPokedexBorder(ui, screen, 2 * TILE_SIZE, 14 * TILE_SIZE, 13, 1);

  const leftArrow = requirePokedexTile(ui, 0x3d);
  const rightArrow = requirePokedexTile(ui, 0x3e);
  screen.blit(leftArrow, [2 * TILE_SIZE, 15 * TILE_SIZE]);
  screen.blit(rightArrow, [16 * TILE_SIZE, 15 * TILE_SIZE]);

  const maxSlots = UNOWN_LETTER_COORDS.length;
  letters.slice(0, maxSlots).forEach((letterValue, slotIndex) => {
    if (letterValue <= 0) {
      return;
    }
    const tileIndex = FIRST_UNOWN_CHAR + letterValue - 1;
    const tile = requirePokedexTile(ui, tileIndex);
    const [tileX, tileY] = UNOWN_LETTER_COORDS[slotIndex][0];
    screen.blit(tile, [tileX * TILE_SIZE, tileY * TILE_SIZE]);
  });

  if (UNOWN_LETTER_COORDS.length) {
    const clampedIndex = Math.max(0, Math.min(cursorIndex, UNOWN_LETTER_COORDS.length - 1));
    const tile = requirePokedexTile(ui, FIRST_UNOWN_CHAR + NUM_UNOWN);
    const [cursorX, cursorY] = UNOWN_LETTER_COORDS[clampedIndex][1];
    screen.blit(tile, [cursorX * TILE_SIZE, cursorY * TILE_SIZE]);
  }

  if (word) {
    renderFontText(ui.font, word, 4 * TILE_SIZE, 15 * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
  }
  if (activeSpeciesId) {
    // ASM: engine/pokedex/pokedex.asm::Pokedex_DrawUnownModeBG
    // hlcoord 6, 5 / call Pokedex_PlaceFrontpicAtHL
    // The frontpic occupies a 7x7 white canvas inside the larger black panel.
    drawFrontSprite(ui, screen, activeSpeciesId, [6 * TILE_SIZE, 5 * TILE_SIZE], false, [255, 255, 255, 255], true);
  }
};

export {
  COLOR_1,
  COLOR_2,
  getPokedexHardwareState,
  resetPokedexHardwareState,
  tintPokedexSprite as tint_pokedex_sprite,
};

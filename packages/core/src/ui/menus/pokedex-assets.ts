// ASM mapping: pokecrystal_disassembly/engine/pokedex/pokedex.asm (tileset + palette setup).
import fs from "fs";
import path from "path";
import { NUM_UNOWN } from "@pokecrystal/core/core/constants";
import { decompress } from "@pokecrystal/core/core/lz";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";
import { Surface } from "@pokecrystal/core/ui/surface";
import { Rect, gameEngine } from "@pokecrystal/core/ui/game-engine";

type Palette = ReadonlyArray<ReadonlyArray<number>>;

const TILE_SIZE = 8;

export const COLOR_0: [number, number, number, number] = [255, 255, 255, 255];
export const COLOR_1: [number, number, number, number] = [255, 164, 82, 255];
export const COLOR_2: [number, number, number, number] = [213, 82, 49, 255];
export const COLOR_3: [number, number, number, number] = [0, 0, 0, 255];

const POKEDEX_PALETTE: Palette = [COLOR_0, COLOR_1, COLOR_2, COLOR_3];
const POKEDEX_PALETTE_LEVELS = [0, 85, 170, 255];
const POKEDEX_2BPP_SOURCE_LEVELS = [255, 170, 85, 0];
const POKEDEX_TILE_BASE = 0x31;
const POKEDEX_TILE_CLEAR_COUNT = 0x31;
const POKEDEX_TILESET_SGB = "pokedex_sgb.2bpp";
const POKEDEX_TILESET_DMG = "pokedex.2bpp";
const POKEDEX_CURSOR_TILE_BASE = 0x30;
const SCROLLBAR_TILE_ID = 0x0f;
const CURSOR_TILE_SHEET = "slowpoke.2bpp";
export const SLOWPOKE_TILE_BASES = [0x00, 0x01, 0x02, 0x10, 0x11, 0x12, 0x20, 0x21, 0x22];
export const SLOWPOKE_TILE_OFFSETS: Array<[number, number]> = [
  [0, 0],
  [8, 0],
  [16, 0],
  [0, 8],
  [8, 8],
  [16, 8],
  [0, 16],
  [8, 16],
  [16, 16],
];
export const SLOWPOKE_FRAME_COUNT = 5;
export const FIRST_UNOWN_CHAR = 0x40;
const UNOWN_FONT_TILE_COUNT = NUM_UNOWN + 1;
const UNOWN_FONT_TILESET = "unown_font.2bpp";
const QUESTION_MARK_PAL = "question_mark.pal";
const CURSOR_PAL = "cursor.pal";

export class PokedexHardwareState {
  public lcdEnabled = true;
  public lcdTransitions: string[] = [];

  disableLcd(): void {
    if (!this.lcdEnabled) {
      return;
    }
    this.lcdEnabled = false;
    this.lcdTransitions.push("disable");
  }

  enableLcd(): void {
    if (this.lcdEnabled) {
      return;
    }
    this.lcdEnabled = true;
    this.lcdTransitions.push("enable");
  }
}

const POKEDEX_HARDWARE = new PokedexHardwareState();
export const POKEDEX_FONT_TILES: Record<number, Surface> = {};
export const POKEDEX_CURSOR_TILES: Record<number, Surface> = {};
let POKEDEX_TILES_LOADED = false;
let POKEDEX_CURSOR_TILES_LOADED = false;
let POKEDEX_TILESET_MODE: boolean | null = null;
let POKEDEX_CURSOR_TILESET_MODE: boolean | null = null;
let SLOWPOKE_TILE_SURFACES: Surface[] | null = null;
const UNOWN_FONT_TILES: Record<number, Surface> = {};
let UNOWN_FONT_LOADED = false;
let QUESTION_MARK_PALETTE: Palette | null = null;
let CURSOR_PALETTE: Palette | null = null;

export type PokedexFontSource = {
  font: {
    font_tiles?: Record<number, Surface>;
    fontTiles?: Record<number, Surface>;
    reloadFontExtraTiles?: () => void;
    reload_font_extra_tiles?: () => void;
  };
  useSuperGameBoyTiles?: boolean;
  use_super_game_boy_tiles?: boolean;
};

const ensureFontTiles = (ui: PokedexFontSource): Record<number, Surface> => {
  if (!ui.font.font_tiles) {
    ui.font.font_tiles = ui.font.fontTiles ?? {};
  }
  return ui.font.font_tiles;
};

const scaleComponent = (component: number): number => gbc5To8(component);

const buildPaletteMap = (palette: Palette): Map<number, ReadonlyArray<number>> => {
  if (palette.length !== 4) {
    throw new Error("Palettes must contain exactly four colour entries.");
  }
  const mapping = new Map<number, ReadonlyArray<number>>();
  const levels = [...POKEDEX_PALETTE_LEVELS].reverse();
  levels.forEach((level, index) => {
    mapping.set(level, palette[index] ?? palette[0]);
  });
  return mapping;
};

const remapGrayscaleSurface = (surface: Surface, palette: Palette): Surface => {
  const paletteMap = buildPaletteMap(palette);
  const [width, height] = surface.get_size();
  const remapped = new Surface(width, height);
  for (let x = 0; x < width; x += 1) {
    for (let y = 0; y < height; y += 1) {
      const [r, g, b, a] = surface.get_at([x, y]);
      if (a === 0) {
        continue;
      }
      let nearest = POKEDEX_PALETTE_LEVELS[0];
      let delta = Math.abs(nearest - r);
      for (const level of POKEDEX_PALETTE_LEVELS) {
        const candidate = Math.abs(level - r);
        if (candidate < delta) {
          nearest = level;
          delta = candidate;
        }
      }
      const mapped = paletteMap.get(nearest) ?? [r, g, b, a];
      remapped.set_at([x, y], [mapped[0], mapped[1], mapped[2], a]);
    }
  }
  return remapped;
};

const loadPaletteFile = (filename: string): Palette => {
  const palettePath = path.join(getAssetPath("gfx", "pokedex"), filename);
  if (!fs.existsSync(palettePath)) {
    throw new Error(`Missing palette file for Pokédex rendering: ${palettePath}`);
  }
  const colours: [number, number, number][] = [];
  const lines = fs.readFileSync(palettePath, "utf-8").split(/\r?\n/);
  for (const raw of lines) {
    const line = raw.trim();
    if (!line.toUpperCase().startsWith("RGB")) {
      continue;
    }
    const parts = line.split("RGB", 2)[1]?.split(",") ?? [];
    if (parts.length < 3) {
      throw new Error(`Palette row '${line}' in ${palettePath} is malformed.`);
    }
    const [r, g, b] = parts.slice(0, 3).map((value) => Number(value.trim()));
    colours.push([scaleComponent(r), scaleComponent(g), scaleComponent(b)]);
  }
  if (colours.length !== 4) {
    throw new Error(`Palette ${palettePath} must define exactly four colours.`);
  }
  return colours;
};

const questionMarkPalette = (): Palette => {
  if (!QUESTION_MARK_PALETTE) {
    QUESTION_MARK_PALETTE = loadPaletteFile(QUESTION_MARK_PAL);
  }
  return QUESTION_MARK_PALETTE;
};

const cursorPalette = (): Palette => {
  if (!CURSOR_PALETTE) {
    CURSOR_PALETTE = loadPaletteFile(CURSOR_PAL);
  }
  return CURSOR_PALETTE;
};

export const getQuestionMarkPalette = (): Palette => questionMarkPalette();

export const applyPokedexPalette = (surface: Surface): Surface =>
  remapGrayscaleSurface(surface, POKEDEX_PALETTE);

export const tintPokedexSprite = (
  surface: Surface,
  palette: Palette | null = null
): Surface => remapGrayscaleSurface(surface, palette ?? POKEDEX_PALETTE);

const surfaceHasAlpha = (surface: Surface): boolean => {
  const [width, height] = surface.get_size();
  for (let x = 0; x < width; x += 1) {
    for (let y = 0; y < height; y += 1) {
      if (surface.get_at([x, y])[3] < 255) {
        return true;
      }
    }
  }
  return false;
};

export const prepareTileSurface = (surface: Surface): Surface => {
  const prepared = surface.copy();
  if (!surfaceHasAlpha(prepared)) {
    const [r, g, b] = prepared.get_at([0, 0]);
    prepared.set_colorkey([r, g, b]);
  }
  return prepared;
};

const prepareOpaqueTileSurface = (surface: Surface): Surface => {
  // Pokédex UI tiles are fully opaque in VRAM; avoid introducing colorkey holes.
  return surface;
};

export const cursorTile = (tileId: number): Surface => {
  const tile = POKEDEX_CURSOR_TILES[tileId];
  if (!tile) {
    throw new Error(`Missing Pokédex cursor tile ${tileId.toString(16)}`);
  }
  return tile;
};

const forceOpaque = (surface: Surface): Surface => {
  const [width, height] = surface.get_size();
  const opaque = new Surface(width, height);
  for (let x = 0; x < width; x += 1) {
    for (let y = 0; y < height; y += 1) {
      const [r, g, b] = surface.get_at([x, y]);
      opaque.set_at([x, y], [r, g, b, 255]);
    }
  }
  return opaque;
};

const decode2bppTiles = (data: Uint8Array): Surface[] => {
  if (data.length % 16 !== 0) {
    throw new Error("2bpp payload must be aligned to 16-byte tiles");
  }
  const tiles: Surface[] = [];
  const tileCount = data.length / 16;
  const levels = [255, 170, 85, 0];
  for (let tileIndex = 0; tileIndex < tileCount; tileIndex += 1) {
    const base = tileIndex * 16;
    const surface = new Surface(TILE_SIZE, TILE_SIZE);
    for (let row = 0; row < TILE_SIZE; row += 1) {
      const plane0 = data[base + row * 2];
      const plane1 = data[base + row * 2 + 1];
      for (let col = 0; col < TILE_SIZE; col += 1) {
        const bit = 7 - col;
        const idx = ((plane0 >> bit) & 1) | (((plane1 >> bit) & 1) << 1);
        const level = levels[idx] ?? 0;
        surface.set_at([col, row], [level, level, level, 255]);
      }
    }
    tiles.push(surface);
  }
  return tiles;
};

const loadPngTileSheet = (filePath: string): Surface[] | null => {
  const surface = gameEngine.image.loadSync?.(filePath);
  if (!surface) {
    return null;
  }
  const [width, height] = surface.get_size();
  const columns = Math.floor(width / TILE_SIZE);
  const rows = Math.floor(height / TILE_SIZE);
  if (columns <= 0 || rows <= 0) {
    return [];
  }
  const tiles: Surface[] = [];
  for (let row = 0; row < rows; row += 1) {
    for (let column = 0; column < columns; column += 1) {
      tiles.push(
        surface.subsurface(new Rect(column * TILE_SIZE, row * TILE_SIZE, TILE_SIZE, TILE_SIZE))
      );
    }
  }
  return tiles;
};

export const loadPokedexTilesFromFile = (filePath: string): Surface[] => {
  if (!fs.existsSync(filePath)) {
    if (!filePath.endsWith(".lz")) {
      const pngPath = filePath.replace(/\.2bpp$/, ".png");
      if (pngPath !== filePath && fs.existsSync(pngPath)) {
        const pngTiles = loadPngTileSheet(pngPath);
        if (pngTiles) {
          return pngTiles;
        }
      }
    }
    throw new Error(`Missing Pokédex tileset: ${filePath}`);
  }
  const raw = fs.readFileSync(filePath);
  const decoded = filePath.endsWith(".lz") ? decompress(raw) : raw;
  return decode2bppTiles(decoded);
};

const applyPokedexTilesToUi = (ui: PokedexFontSource): void => {
  const fontTiles = ensureFontTiles(ui);
  Object.entries(POKEDEX_FONT_TILES).forEach(([tileId, tile]) => {
    fontTiles[Number(tileId)] = tile;
  });
};

const applyPokedexCursorTilesToUi = (ui: PokedexFontSource): void => {
  const fontTiles = ensureFontTiles(ui);
  Object.entries(POKEDEX_CURSOR_TILES).forEach(([tileId, tile]) => {
    fontTiles[Number(tileId)] = tile;
  });
};

const clearVTiles2 = (ui: PokedexFontSource): void => {
  const fontTiles = ensureFontTiles(ui);
  const blank = new Surface(TILE_SIZE, TILE_SIZE);
  blank.fill([0, 0, 0, 0]);
  for (let offset = 0; offset < POKEDEX_TILE_CLEAR_COUNT; offset += 1) {
    const tileIndex = POKEDEX_TILE_BASE + offset;
    fontTiles[tileIndex] = prepareTileSurface(blank);
  }
};

const invertTileSurface = (surface: Surface): Surface => {
  const inverted = surface.copy();
  const [width, height] = inverted.get_size();
  for (let x = 0; x < width; x += 1) {
    for (let y = 0; y < height; y += 1) {
      const [r, g, b, a] = inverted.get_at([x, y]);
      inverted.set_at([x, y], [255 - r, 255 - g, 255 - b, a]);
    }
  }
  return prepareTileSurface(inverted);
};

const loadInvertedFontTiles = (ui: PokedexFontSource): void => {
  const fontTiles = ensureFontTiles(ui);
  for (let tileIndex = 0x60; tileIndex < 0x80; tileIndex += 1) {
    const tile = fontTiles[tileIndex];
    if (!tile) {
      continue;
    }
    POKEDEX_FONT_TILES[tileIndex] = invertTileSurface(tile);
  }
};

const solidPokedexTile = (color: [number, number, number, number]): Surface => {
  const tile = new Surface(TILE_SIZE, TILE_SIZE);
  tile.fill(color);
  return prepareOpaqueTileSurface(tile);
};

const overridePokedexSpaceTile = (ui: PokedexFontSource): void => {
  const fontTiles = ensureFontTiles(ui);
  const tile = solidPokedexTile(COLOR_3);
  fontTiles[0x7f] = tile;
  POKEDEX_FONT_TILES[0x7f] = tile;
};

const shouldUseSgbTiles = (ui: PokedexFontSource): boolean =>
  Boolean(ui.useSuperGameBoyTiles ?? ui.use_super_game_boy_tiles);

const loadPokedexTileset = (ui: PokedexFontSource): void => {
  const fontTiles = ensureFontTiles(ui);
  const tilesRoot = getAssetPath("gfx", "pokedex");
  const filename = shouldUseSgbTiles(ui) ? POKEDEX_TILESET_SGB : POKEDEX_TILESET_DMG;
  const tiles = loadPokedexTilesFromFile(path.join(tilesRoot, filename));
  tiles.forEach((surface, index) => {
    const tileIndex = POKEDEX_TILE_BASE + index;
    const opaque = forceOpaque(surface);
    const tinted = applyPokedexPalette(opaque);
    const prepared = prepareOpaqueTileSurface(tinted);
    fontTiles[tileIndex] = prepared;
    POKEDEX_FONT_TILES[tileIndex] = prepared;
  });
};

export const ensurePokedexTiles = (ui: PokedexFontSource): void => {
  ensureFontTiles(ui);
  const reloadRequired = !POKEDEX_TILES_LOADED || POKEDEX_TILESET_MODE !== shouldUseSgbTiles(ui);
  if (!reloadRequired) {
    applyPokedexTilesToUi(ui);
    return;
  }
  POKEDEX_HARDWARE.disableLcd();
  try {
    clearVTiles2(ui);
    ui.font.reloadFontExtraTiles?.();
    ui.font.reload_font_extra_tiles?.();
    Object.keys(POKEDEX_FONT_TILES).forEach((key) => delete POKEDEX_FONT_TILES[Number(key)]);
    POKEDEX_TILESET_MODE = shouldUseSgbTiles(ui);
    loadInvertedFontTiles(ui);
    loadPokedexTileset(ui);
    overridePokedexSpaceTile(ui);
    POKEDEX_TILES_LOADED = true;
    applyPokedexTilesToUi(ui);
  } finally {
    POKEDEX_HARDWARE.enableLcd();
  }
};

const loadCursorTiles = (ui: PokedexFontSource): Record<number, Surface> => {
  const tilesRoot = getAssetPath("gfx", "pokedex");
  const tiles = loadPokedexTilesFromFile(path.join(tilesRoot, CURSOR_TILE_SHEET));
  const requiredIds = [
    SCROLLBAR_TILE_ID,
    ...Array.from({ length: 7 }, (_unused, index) => POKEDEX_CURSOR_TILE_BASE + index),
  ];
  const mapping: Record<number, Surface> = {};
  requiredIds.forEach((tileId) => {
    const tile = tiles[tileId];
    if (!tile) {
      throw new Error(`Missing cursor tile ${tileId.toString(16)} in Pokédex cursor sheet.`);
    }
    mapping[tileId] = remapCursorTileSurface(tile);
  });
  return mapping;
};

const nearestPokedex2bppColorIndex = (value: number): number => {
  let bestIndex = 0;
  let bestDelta = Math.abs(POKEDEX_2BPP_SOURCE_LEVELS[0] - value);
  for (let index = 1; index < POKEDEX_2BPP_SOURCE_LEVELS.length; index += 1) {
    const delta = Math.abs(POKEDEX_2BPP_SOURCE_LEVELS[index] - value);
    if (delta < bestDelta) {
      bestIndex = index;
      bestDelta = delta;
    }
  }
  return bestIndex;
};

const remapCursorTileSurface = (tile: Surface): Surface => {
  const palette = cursorPalette();
  const cursor = new Surface(TILE_SIZE, TILE_SIZE);
  cursor.fill([0, 0, 0, 0]);
  for (let y = 0; y < TILE_SIZE; y += 1) {
    for (let x = 0; x < TILE_SIZE; x += 1) {
      const [r, _g, _b, a] = tile.get_at([x, y]);
      const paletteIndex = nearestPokedex2bppColorIndex(r);
      if (a === 0 || paletteIndex === 0) {
        continue;
      }
      const mapped = palette[paletteIndex] ?? palette[0] ?? [0, 0, 0];
      cursor.set_at([x, y], [
        mapped[0] ?? 0,
        mapped[1] ?? 0,
        mapped[2] ?? 0,
        255,
      ]);
    }
  }
  return cursor;
};

export const ensurePokedexCursorTiles = (ui: PokedexFontSource): void => {
  const fontTiles = ensureFontTiles(ui);
  const reloadRequired =
    !POKEDEX_CURSOR_TILES_LOADED || POKEDEX_CURSOR_TILESET_MODE !== shouldUseSgbTiles(ui);
  if (!reloadRequired) {
    applyPokedexCursorTilesToUi(ui);
    return;
  }
  const tiles = loadCursorTiles(ui);
  Object.keys(POKEDEX_CURSOR_TILES).forEach((key) => delete POKEDEX_CURSOR_TILES[Number(key)]);
  POKEDEX_CURSOR_TILESET_MODE = shouldUseSgbTiles(ui);
  Object.entries(tiles).forEach(([tileId, surface]) => {
    POKEDEX_CURSOR_TILES[Number(tileId)] = surface;
    fontTiles[Number(tileId)] = surface;
  });
  POKEDEX_CURSOR_TILES_LOADED = true;
  applyPokedexCursorTilesToUi(ui);
};

export const slowpokeTiles = (ui: PokedexFontSource): Surface[] => {
  if (SLOWPOKE_TILE_SURFACES) {
    return SLOWPOKE_TILE_SURFACES;
  }
  const tilesRoot = getAssetPath("gfx", "pokedex");
  const tiles = loadPokedexTilesFromFile(path.join(tilesRoot, CURSOR_TILE_SHEET));
  SLOWPOKE_TILE_SURFACES = tiles.map((tile) => prepareTileSurface(tintPokedexSprite(tile)));
  return SLOWPOKE_TILE_SURFACES;
};

export const ensureUnownFontTiles = (ui: PokedexFontSource): void => {
  const fontTiles = ensureFontTiles(ui);
  if (UNOWN_FONT_LOADED) {
    Object.entries(UNOWN_FONT_TILES).forEach(([tileId, tile]) => {
      fontTiles[Number(tileId)] = tile;
    });
    return;
  }
  const tilesRoot = getAssetPath("gfx", "font");
  const tiles = loadPokedexTilesFromFile(path.join(tilesRoot, UNOWN_FONT_TILESET));
  if (tiles.length < UNOWN_FONT_TILE_COUNT) {
    throw new Error("Unown font sheet is missing required tiles for Pokédex rendering.");
  }
  Object.keys(UNOWN_FONT_TILES).forEach((key) => delete UNOWN_FONT_TILES[Number(key)]);
  for (let offset = 0; offset < UNOWN_FONT_TILE_COUNT; offset += 1) {
    const tileIndex = FIRST_UNOWN_CHAR + offset;
    const tileSurface = invertTileSurface(tiles[offset]);
    const prepared = prepareTileSurface(tileSurface);
    UNOWN_FONT_TILES[tileIndex] = prepared;
    fontTiles[tileIndex] = prepared;
    POKEDEX_FONT_TILES[tileIndex] = prepared;
  }
  UNOWN_FONT_LOADED = true;
};

export const getPokedexHardwareState = (): PokedexHardwareState => POKEDEX_HARDWARE;

export const resetPokedexHardwareState = (): void => {
  POKEDEX_HARDWARE.lcdEnabled = true;
  POKEDEX_HARDWARE.lcdTransitions.length = 0;
  POKEDEX_TILES_LOADED = false;
  POKEDEX_TILESET_MODE = null;
  POKEDEX_CURSOR_TILES_LOADED = false;
  POKEDEX_CURSOR_TILESET_MODE = null;
  UNOWN_FONT_LOADED = false;
};

export const requirePokedexTile = (ui: PokedexFontSource, tileIndex: number): Surface => {
  const fontTiles = ensureFontTiles(ui);
  const tile = POKEDEX_FONT_TILES[tileIndex] ?? fontTiles[tileIndex];
  if (!tile) {
    throw new Error(
      `Font tile 0x${tileIndex.toString(16)} required for Pokédex rendering is missing.`
    );
  }
  return tile;
};

import fs from 'fs';
import path from 'path';
import { DEFAULT_TILE_SIZE, BattleUILayoutFactory } from './_battle-layout';
import type { BattleUILayout, BattleTextWindow } from './_battle-layout';
import { Surface } from '../surface';
import { TilemapSurface, _CLEAR_TILE, _SPACE_TILE } from '../tilemap-surface';
import { BattleVRAMAllocator } from './_battle-vram';
import { getAssetPath } from '../../core/paths';
import { gbc5To8 } from '@pokecrystal/core/core/gbc-colors';

type Palette = [number, number, number][];
type PaletteSet = Palette[];

const DMG_SHADES: [number, number, number][] = [
  [255, 255, 255],
  [170, 170, 170],
  [85, 85, 85],
  [0, 0, 0],
];

export const PAL_BG_PLAYER = 0;
export const PAL_BG_ENEMY = 1;
export const PAL_BG_ENEMY_HP = 2;
export const PAL_BG_PLAYER_HP = 3;
export const PAL_BG_EXP = 4;
export const PAL_BG_UNUSED5 = 5;
export const PAL_BG_UNUSED6 = 6;
export const PAL_BG_TEXT = 7;

export const PAL_HP_GREEN = 0;
export const PAL_HP_YELLOW = 1;
export const PAL_HP_RED = 2;
export const PAL_EXP_FILL = PAL_BG_EXP;

export const PAL_BACKGROUND = PAL_BG_ENEMY_HP;
export const PAL_HUD = PAL_BG_ENEMY;
export const PAL_TEXT_WINDOW = PAL_BG_TEXT;
export const PAL_MENU = PAL_BG_PLAYER;

export type { BattleUILayout } from './_battle-layout';
export { BattleUILayoutFactory } from './_battle-layout';

const _gb = (n: number): number => gbc5To8(n);
const _rgb = (r: number, g: number, b: number): [number, number, number] => [_gb(r), _gb(g), _gb(b)];

const readPaletteFile = (
  palettePath: string,
  options: { expectedColours: number; label: string },
): Palette => {
  if (!fs.existsSync(palettePath)) {
    throw new Error(`Missing palette file: ${palettePath}`);
  }
  const entries: Palette = [];
  const lines = fs.readFileSync(palettePath, 'utf-8').split(/\r?\n/);
  for (const line of lines) {
    const stripped = line.split(';', 1)[0].trim();
    if (!stripped || !stripped.startsWith('RGB')) {
      continue;
    }
    const values = stripped
      .replace(/RGB/g, '')
      .replace(/,/g, ' ')
      .split(/\s+/)
      .filter(Boolean)
      .map((value) => Number(value));
    if (values.length !== 3 || values.some((value) => Number.isNaN(value))) {
      throw new Error(`Invalid RGB triple in ${options.label}: ${stripped}`);
    }
    entries.push(_rgb(values[0], values[1], values[2]));
  }
  if (entries.length !== options.expectedColours) {
    throw new Error(
      `${options.label} should define ${options.expectedColours} RGB triples, found ${entries.length}`,
    );
  }
  return entries;
};

const readPredefPalette = (palettePath: string, name: string): Palette => {
  if (!fs.existsSync(palettePath)) {
    throw new Error(`Missing predef palette file: ${palettePath}`);
  }
  const lines = fs.readFileSync(palettePath, 'utf-8').split(/\r?\n/);
  for (const line of lines) {
    if (!line.includes(`PREDEFPAL_${name}`)) {
      continue;
    }
    const prefix = line.split(';', 1)[0];
    const values = prefix
      .replace(/RGB/g, '')
      .replace(/,/g, ' ')
      .split(/\s+/)
      .filter(Boolean)
      .map((value) => Number(value));
    if (values.length !== 12 || values.some((value) => Number.isNaN(value))) {
      throw new Error(
        `Expected 4 colours for PREDEFPAL_${name}, found ${Math.floor(values.length / 3)}`,
      );
    }
    const entries: Palette = [];
    for (let index = 0; index < values.length; index += 3) {
      entries.push(_rgb(values[index], values[index + 1], values[index + 2]));
    }
    return entries;
  }
  throw new Error(`PREDEFPAL_${name} not found in ${palettePath}`);
};

const readHpPalettes = (palettePath: string): PaletteSet => {
  if (!fs.existsSync(palettePath)) {
    throw new Error(`Missing HP bar palette file: ${palettePath}`);
  }
  const entries: Palette = [];
  const lines = fs.readFileSync(palettePath, 'utf-8').split(/\r?\n/);
  for (const line of lines) {
    const stripped = line.split(';', 1)[0].trim();
    if (!stripped || !stripped.startsWith('RGB')) {
      continue;
    }
    const values = stripped
      .replace(/RGB/g, '')
      .replace(/,/g, ' ')
      .split(/\s+/)
      .filter(Boolean)
      .map((value) => Number(value));
    if (values.length !== 3 || values.some((value) => Number.isNaN(value))) {
      throw new Error(`Invalid RGB triple in ${palettePath}: ${stripped}`);
    }
    entries.push(_rgb(values[0], values[1], values[2]));
  }
  if (entries.length !== 6) {
    throw new Error(`hp_bar.pal should define 6 RGB triples, found ${entries.length}`);
  }
  const palettes: PaletteSet = [];
  for (let index = 0; index < entries.length; index += 2) {
    const colour1 = entries[index];
    const colour2 = entries[index + 1];
    palettes.push([
      [255, 255, 255],
      colour1,
      colour2,
      [0, 0, 0],
    ]);
  }
  return palettes;
};

const readExpPalette = (palettePath: string): Palette => {
  if (!fs.existsSync(palettePath)) {
    throw new Error(`Missing EXP bar palette file: ${palettePath}`);
  }
  const entries: Palette = [];
  const lines = fs.readFileSync(palettePath, 'utf-8').split(/\r?\n/);
  for (const line of lines) {
    const stripped = line.split(';', 1)[0].trim();
    if (!stripped || !stripped.startsWith('RGB')) {
      continue;
    }
    const values = stripped
      .replace(/RGB/g, '')
      .replace(/,/g, ' ')
      .split(/\s+/)
      .filter(Boolean)
      .map((value) => Number(value));
    if (values.length !== 3 || values.some((value) => Number.isNaN(value))) {
      throw new Error(`Invalid RGB triple in ${palettePath}: ${stripped}`);
    }
    entries.push(_rgb(values[0], values[1], values[2]));
  }
  if (entries.length !== 2) {
    throw new Error(`exp_bar.pal should define 2 RGB triples, found ${entries.length}`);
  }
  return [
    [255, 255, 255],
    entries[0],
    entries[1],
    [0, 0, 0],
  ];
};

const loadPaletteFiles = (): [PaletteSet, PaletteSet, Palette] => {
  // ASM: gfx/sgb/predef.pal + gfx/battle/*.pal define battle palette tables.
  const predefPath = getAssetPath('gfx', 'sgb', 'predef.pal');
  const hpPath = getAssetPath('gfx', 'battle', 'hp_bar.pal');
  const expPath = getAssetPath('gfx', 'battle', 'exp_bar.pal');
  const textPath = getAssetPath('gfx', 'stats', 'party_menu_bg.pal');
  const blackout = readPredefPalette(predefPath, 'BLACKOUT');
  const hpPalettes = readHpPalettes(hpPath);
  const expPalette = readExpPalette(expPath);
  const textPalette = readPaletteFile(textPath, {
    expectedColours: 4,
    label: 'PartyMenuBGPalette',
  });
  const palettes: PaletteSet = [
    textPalette,
    textPalette,
    hpPalettes[PAL_HP_GREEN],
    hpPalettes[PAL_HP_GREEN],
    expPalette,
    blackout,
    blackout,
    textPalette,
  ];
  return [palettes, hpPalettes, expPalette];
};

const [BATTLE_PALETTES, HP_PALETTE_SET] = loadPaletteFiles();
export const _BATTLE_PALETTES = BATTLE_PALETTES;
export const HP_PALETTES = HP_PALETTE_SET;

const WINDOW_TILES = new Set<number>([0x79, 0x7a, 0x7b, 0x7c, 0x7d, 0x7e, _SPACE_TILE, _CLEAR_TILE]);
const ALL_BATTLE_TILE_IDS = Array.from({ length: 0x20 }, (_, index) => 0x60 + index);
const EXP_TILE_IDS = new Set<number>(Array.from({ length: 0x09 }, (_, index) => 0x55 + index));
const PHONE_TILE_IDS = [0x5e, 0x5f];
const EXP_PARTIAL_TILES = new Set<number>(Array.from({ length: 0x08 }, (_, index) => 0x54 + index));

const ENEMY_HUD_RECT: [number, number, number, number] = [1, 0, 11, 4];
const PLAYER_HUD_RECT: [number, number, number, number] = [9, 7, 11, 5];
const HP_BAR_TILE_WIDTH = 6;
const HP_BAR_TILE_HEIGHT = 1;
const EXP_BAR_TILE_WIDTH = 9;
const EXP_BAR_TILE_HEIGHT = 1;

const ENEMY_HUD_BORDER_TILES = [0x6d, 0x74, 0x78, 0x76];
const PLAYER_HUD_BORDER_TILES = [0x73, 0x77, 0x6f, 0x76];

export const dmg_palette_from_register = (
  registerValue: number,
): [number, number, number][] | null => {
  if (typeof registerValue !== 'number' || Number.isNaN(registerValue)) {
    return null;
  }
  const entries = [0, 1, 2, 3].map((index) => (registerValue >> (index * 2)) & 0x03);
  if (entries.length !== 4) {
    return null;
  }
  return entries.map((index) => DMG_SHADES[index] ?? DMG_SHADES[0]);
};

const decode2bppTiles = (data: Uint8Array): Surface[] => {
  if (data.length % 16 !== 0) {
    throw new Error('2bpp payload must contain 16-byte-aligned tiles');
  }
  const tiles: Surface[] = [];
  for (let tileIndex = 0; tileIndex < data.length / 16; tileIndex += 1) {
    const base = tileIndex * 16;
    const surface = new Surface(DEFAULT_TILE_SIZE, DEFAULT_TILE_SIZE);
    for (let row = 0; row < DEFAULT_TILE_SIZE; row += 1) {
      const plane0 = data[base + row * 2];
      const plane1 = data[base + row * 2 + 1];
      for (let col = 0; col < DEFAULT_TILE_SIZE; col += 1) {
        const bit = 7 - col;
        const colourIndex = ((plane0 >> bit) & 1) | (((plane1 >> bit) & 1) << 1);
        const colour = DMG_SHADES[colourIndex];
        surface.setAt(col, row, [colour[0], colour[1], colour[2], 255]);
      }
    }
    tiles.push(surface);
  }
  return tiles;
};

const decode1bppTiles = (data: Uint8Array): Surface[] => {
  if (data.length % 8 !== 0) {
    throw new Error('1bpp payload must contain 8-byte-aligned tiles');
  }
  const palette: [number, number, number, number][] = [
    [255, 255, 255, 255],
    [0, 0, 0, 255],
  ];
  const tiles: Surface[] = [];
  for (let tileIndex = 0; tileIndex < data.length / 8; tileIndex += 1) {
    const base = tileIndex * 8;
    const surface = new Surface(DEFAULT_TILE_SIZE, DEFAULT_TILE_SIZE);
    for (let row = 0; row < DEFAULT_TILE_SIZE; row += 1) {
      const plane = data[base + row];
      for (let col = 0; col < DEFAULT_TILE_SIZE; col += 1) {
        const bit = (plane >> (7 - col)) & 0x01;
        surface.setAt(col, row, palette[bit]);
      }
    }
    tiles.push(surface);
  }
  return tiles;
};

const loadTileFile = (
  store: Record<number, Surface>,
  assetPath: string,
  options: {
    start_tile: number;
    tile_offset?: number;
    tile_count?: number;
    allocator?: BattleVRAMAllocator | null;
    source_label?: string | null;
  },
): void => {
  if (!fs.existsSync(assetPath)) {
    throw new Error(`Battle tile asset missing: ${assetPath}`);
  }
  const data = fs.readFileSync(assetPath);
  const tiles = assetPath.endsWith('.1bpp') ? decode1bppTiles(data) : decode2bppTiles(data);
  const tileOffset = options.tile_offset ?? 0;
  if (tileOffset > tiles.length) {
    throw new Error(`Tile offset ${tileOffset} beyond ${tiles.length} tiles in ${assetPath}`);
  }
  const end = options.tile_count === undefined ? tiles.length : tileOffset + options.tile_count;
  if (end > tiles.length) {
    throw new Error(
      `Requested ${options.tile_count} tiles from ${assetPath} starting at ${tileOffset}, only ${tiles.length} available`,
    );
  }
  const selection = tiles.slice(tileOffset, end);
  if (options.allocator) {
    options.allocator.record_tiles({
      start_tile: options.start_tile,
      tile_count: selection.length,
      source: options.source_label ?? path.basename(assetPath),
    });
  }
  for (let index = 0; index < selection.length; index += 1) {
    store[options.start_tile + index] = selection[index];
  }
};

const loadFontBattleExtra = (
  store: Record<number, Surface>,
  options?: { allocator?: BattleVRAMAllocator | null },
): void => {
  const fontPath = getAssetPath('gfx', 'font', 'font_battle_extra.2bpp');
  loadTileFile(store, fontPath, {
    start_tile: 0x60,
    tile_count: 12,
    allocator: options?.allocator ?? null,
    source_label: 'FontBattleExtra[$60]',
  });
  loadTileFile(store, fontPath, {
    start_tile: 0x70,
    tile_offset: 16,
    tile_count: 3,
    allocator: options?.allocator ?? null,
    source_label: 'FontBattleExtra[$70]',
  });
};

const loadTextboxFrameTiles = (
  store: Record<number, Surface>,
  options: { textbox_frame: number; allocator?: BattleVRAMAllocator | null },
): void => {
  const frameIndex = options.textbox_frame & 0x07;
  const frameId = frameIndex + 1;
  const framePath = getAssetPath('gfx', 'frames', `${frameId}.1bpp`);
  loadTileFile(store, framePath, {
    start_tile: 0x79,
    tile_count: 6,
    allocator: options.allocator ?? null,
    source_label: `TextboxFrame${frameId}GFX`,
  });
  loadTileFile(store, getAssetPath('gfx', 'font', 'space.2bpp'), {
    start_tile: 0x7f,
    tile_count: 1,
    allocator: options.allocator ?? null,
    source_label: 'TextboxSpaceGFX',
  });
};

const loadBattleGraphics = (
  fontTiles: Record<number, Surface>,
  options?: { allocator?: BattleVRAMAllocator | null; textbox_frame?: number },
): Record<number, Surface> => {
  const tiles: Record<number, Surface> = { ...fontTiles };
  loadFontBattleExtra(tiles, { allocator: options?.allocator ?? null });
  loadTextboxFrameTiles(tiles, {
    textbox_frame: options?.textbox_frame ?? 0,
    allocator: options?.allocator ?? null,
  });
  const battleDir = getAssetPath('gfx', 'battle');
  loadTileFile(tiles, path.join(battleDir, 'enemy_hp_bar_border.1bpp'), {
    start_tile: 0x6c,
    tile_count: 4,
    allocator: options?.allocator ?? null,
    source_label: 'EnemyHPBarBorderGFX',
  });
  loadTileFile(tiles, path.join(battleDir, 'hp_exp_bar_border.1bpp'), {
    start_tile: 0x73,
    tile_count: 6,
    allocator: options?.allocator ?? null,
    source_label: 'HPExpBarBorderGFX',
  });
  loadTileFile(tiles, path.join(battleDir, 'expbar.2bpp'), {
    start_tile: 0x55,
    tile_count: 9,
    allocator: options?.allocator ?? null,
    source_label: 'ExpBarGFX',
  });
  loadTileFile(tiles, getAssetPath('gfx', 'mobile', 'phone_tiles.2bpp'), {
    start_tile: 0x5e,
    tile_offset: 7,
    tile_count: 2,
    allocator: options?.allocator ?? null,
    source_label: 'MobilePhoneTilesGFX',
  });
  return tiles;
};

const greyscaleLevel = (pixel: [number, number, number, number]): number => {
  const value = Math.floor((pixel[0] + pixel[1] + pixel[2]) / 3);
  if (value >= 213) {
    return 0;
  }
  if (value >= 128) {
    return 1;
  }
  if (value >= 43) {
    return 2;
  }
  return 3;
};

const colorizeTile = (source: Surface, palette: Palette): Surface => {
  const tinted = new Surface(source.width, source.height);
  for (let y = 0; y < source.height; y += 1) {
    for (let x = 0; x < source.width; x += 1) {
      const pixel = source.getAt(x, y);
      if (pixel[3] === 0) {
        const colour = palette[0];
        tinted.setAt(x, y, [colour[0], colour[1], colour[2], 255]);
        continue;
      }
      const level = greyscaleLevel(pixel);
      const colour = palette[level];
      tinted.setAt(x, y, [colour[0], colour[1], colour[2], 255]);
    }
  }
  return tinted;
};

const fillTileWithPalette = (colour: [number, number, number]): Surface => {
  const surface = new Surface(DEFAULT_TILE_SIZE, DEFAULT_TILE_SIZE);
  surface.fill([colour[0], colour[1], colour[2], 255]);
  return surface;
};

const fetchTile = (tiles: Record<number, Surface>, tileId: number): Surface => {
  const tile = tiles[tileId];
  if (!tile) {
    throw new Error(`Battle UI tile 0x${tileId.toString(16).padStart(2, '0')} missing from font`);
  }
  return tile;
};

const buildPaletteTiles = (
  tiles: Record<number, Surface>,
  tileIds: number[],
): Record<number, Record<number, Surface>> => {
  const variants: Record<number, Record<number, Surface>> = {};
  const paletteCount = BATTLE_PALETTES.length;
  for (const tileId of tileIds) {
    const base = fetchTile(tiles, tileId);
    const paletteMap: Record<number, Surface> = {};
    for (let palIdx = 0; palIdx < paletteCount; palIdx += 1) {
      paletteMap[palIdx] = colorizeTile(base, BATTLE_PALETTES[palIdx]);
    }
    variants[tileId] = paletteMap;
  }
  return variants;
};

export const retint_tileset_palette = (
  tileset: Record<number, Record<number, Surface>>,
  baseTiles: Record<number, Surface>,
  paletteIndex: number,
  palette: Palette,
): void => {
  if (paletteIndex < 0 || !palette.length) {
    return;
  }
  const targetPalette = paletteIndex & 0x07;
  const recoloured: Record<number, Surface> = {};
  for (const [tileIdRaw, entry] of Object.entries(tileset)) {
    const tileId = Number(tileIdRaw);
    if (Number.isNaN(tileId)) {
      continue;
    }
    let base = baseTiles[tileId];
    if (!base && tileId === _SPACE_TILE) {
      base = fillTileWithPalette(palette[1]);
    }
    if (!base || typeof entry !== 'object' || Array.isArray(entry)) {
      continue;
    }
    if (!recoloured[tileId]) {
      recoloured[tileId] = colorizeTile(base, palette);
    }
    tileset[tileId] = { ...entry, [targetPalette]: recoloured[tileId] };
  }
};

export class BattleBackgroundTilemap extends TilemapSurface {
  constructor(widthTiles: number = 20, heightTiles: number = 18) {
    super(widthTiles, heightTiles);
  }

  static fromDimensions(widthTiles: number, heightTiles: number): BattleBackgroundTilemap {
    if (widthTiles <= 0 || heightTiles <= 0) {
      throw new Error('BattleBackgroundTilemap requires positive tile dimensions');
    }
    return new BattleBackgroundTilemap(widthTiles, heightTiles);
  }
}

export const DEFAULT_TILE_PIXELS = DEFAULT_TILE_SIZE;

const seedAttrmap = (tilemap: BattleBackgroundTilemap): void => {
  tilemap.fill_attr_rect(0, 0, tilemap.width, tilemap.height, { attr: PAL_BG_ENEMY_HP });
  tilemap.fill_attr_rect(0, 4, 8, 10, { attr: PAL_BG_PLAYER });
  tilemap.fill_attr_rect(10, 0, 7, 10, { attr: PAL_BG_ENEMY });
  tilemap.fill_attr_rect(0, 0, 4, 10, { attr: PAL_BG_ENEMY_HP });
  tilemap.fill_attr_rect(10, 7, 5, 10, { attr: PAL_BG_PLAYER_HP });
  tilemap.fill_attr_rect(10, 11, 1, 9, { attr: PAL_BG_EXP });
  tilemap.fill_attr_rect(0, 12, tilemap.width, 6, { attr: PAL_BG_TEXT });
};

const fillWithTile = (tilemap: BattleBackgroundTilemap, tile: number): void => {
  tilemap.clearTilemap({ tile });
};

const tileCoordsFromPixels = (x: number, y: number): [number, number] => [
  Math.floor(x / DEFAULT_TILE_SIZE),
  Math.floor(y / DEFAULT_TILE_SIZE),
];

const fillRectPreserveAttr = (
  tilemap: BattleBackgroundTilemap,
  x: number,
  y: number,
  width: number,
  height: number,
  tile: number,
): void => {
  for (let row = y; row < Math.min(y + height, tilemap.height); row += 1) {
    for (let col = x; col < Math.min(x + width, tilemap.width); col += 1) {
      tilemap.set_tile(col, row, tile);
    }
  }
};

const fillRegion = (
  tilemap: BattleBackgroundTilemap,
  x: number,
  y: number,
  width: number,
  height: number,
  tile: number,
  palette: number,
): void => {
  const innerWidth = Math.max(0, width - 2);
  const innerHeight = Math.max(0, height - 2);
  if (innerWidth > 0 && innerHeight > 0) {
    tilemap.fill_rect(x + 1, y + 1, innerWidth, innerHeight, { tile, attr: palette });
  }
  tilemap.fill_attr_rect(x, y, width, height, { attr: palette });
};

const drawWindow = (
  tilemap: BattleBackgroundTilemap,
  x: number,
  y: number,
  width: number,
  height: number,
  palette: number,
): void => {
  tilemap.draw_window(x, y, width, height, { fill_tile: _SPACE_TILE });
  fillRegion(tilemap, x, y, width, height, _SPACE_TILE, palette);
};

const drawLayoutWindow = (
  tilemap: BattleBackgroundTilemap,
  window: BattleTextWindow,
  palette: number,
): void => {
  drawWindow(tilemap, window.tile_x, window.tile_y, window.width_tiles, window.height_tiles, palette);
};

const placeEnemyHudBorder = (
  tilemap: BattleBackgroundTilemap,
  x: number,
  y: number,
  width: number,
  height: number,
): void => {
  const [tile0, tile1, tile2, tile3] = ENEMY_HUD_BORDER_TILES;
  const tileX = x;
  const topY = y + height - 2;
  const bottomY = y + height - 1;
  tilemap.set_tile(tileX, topY, tile0);
  tilemap.set_tile(tileX, bottomY, tile1);
  for (let offset = 1; offset < width - 2; offset += 1) {
    tilemap.set_tile(tileX + offset, bottomY, tile3);
  }
  tilemap.set_tile(tileX + width - 2, bottomY, tile2);
};

const placePlayerHudBorder = (
  tilemap: BattleBackgroundTilemap,
  x: number,
  y: number,
  width: number,
  height: number,
): void => {
  const [tile0, tile1, tile2, tile3] = PLAYER_HUD_BORDER_TILES;
  const borderX = x + width - 2;
  const bottomY = y + height - 1;
  const verticalTop = y + height - 3;
  const verticalBottom = y + height - 2;
  tilemap.set_tile(borderX, verticalTop, tile0);
  tilemap.set_tile(borderX, verticalBottom, tile0);
  tilemap.set_tile(borderX, bottomY, tile1);
  for (let offset = 1; offset < width - 2; offset += 1) {
    tilemap.set_tile(borderX - offset, bottomY, tile3);
  }
  tilemap.set_tile(borderX - (width - 2), bottomY, tile2);
};

const drawHudBox = (
  tilemap: BattleBackgroundTilemap,
  x: number,
  y: number,
  width: number,
  height: number,
  palette: number,
  border?: 'enemy' | 'player' | null,
): void => {
  fillRectPreserveAttr(tilemap, x, y, width, height, _SPACE_TILE);
  if (border === 'enemy') {
    placeEnemyHudBorder(tilemap, x, y, width, height);
  } else if (border === 'player') {
    placePlayerHudBorder(tilemap, x, y, width, height);
  }
};

const fillHpBarRegion = (
  tilemap: BattleBackgroundTilemap,
  position: [number, number],
): void => {
  if (!position || position.length < 2) {
    return;
  }
  const [xPx, yPx] = position;
  const [tileX, tileY] = tileCoordsFromPixels(xPx, yPx);
  fillRectPreserveAttr(tilemap, tileX, tileY, HP_BAR_TILE_WIDTH, HP_BAR_TILE_HEIGHT, _SPACE_TILE);
};

export const build_battle_tilemap = (layout: BattleUILayout): BattleBackgroundTilemap => {
  const tilemap = new BattleBackgroundTilemap();
  fillWithTile(tilemap, _SPACE_TILE);
  seedAttrmap(tilemap);
  drawHudBox(tilemap, ...ENEMY_HUD_RECT, PAL_HUD, 'enemy');
  drawHudBox(tilemap, ...PLAYER_HUD_RECT, PAL_HUD, 'player');
  fillHpBarRegion(tilemap, layout.enemy_hud.hp_fill_position);
  fillHpBarRegion(tilemap, layout.player_hud.hp_fill_position);
  drawLayoutWindow(tilemap, layout.text_box, PAL_TEXT_WINDOW);
  return tilemap;
};

export const build_battle_tileset = (
  font_tiles: Record<number, Surface>,
  options?: { allocator?: BattleVRAMAllocator; textbox_frame?: number },
): [Record<number, Record<number, Surface>>, Record<number, Surface>] => {
  const tiles = loadBattleGraphics(font_tiles, {
    allocator: options?.allocator ?? null,
    textbox_frame: options?.textbox_frame ?? 0,
  });
  const tileset: Record<number, Record<number, Surface>> = {};
  const coverage = new Set<number>([
    ...ALL_BATTLE_TILE_IDS,
    ...EXP_TILE_IDS,
    ...PHONE_TILE_IDS,
    ...WINDOW_TILES,
    ...ENEMY_HUD_BORDER_TILES,
    ...PLAYER_HUD_BORDER_TILES,
    ...EXP_PARTIAL_TILES,
  ]);
  Object.assign(tileset, buildPaletteTiles(tiles, [...coverage].sort((a, b) => a - b)));
  const spaceShades: Record<number, number> = {
    [PAL_BACKGROUND]: 0,
    [PAL_HUD]: 1,
    [PAL_TEXT_WINDOW]: 0,
    [PAL_MENU]: 0,
  };
  const spacePaletteMap: Record<number, Surface> = {};
  for (let palIdx = 0; palIdx < BATTLE_PALETTES.length; palIdx += 1) {
    const paletteIndex = spaceShades[palIdx] ?? 1;
    spacePaletteMap[palIdx] = fillTileWithPalette(BATTLE_PALETTES[palIdx][paletteIndex]);
  }
  tileset[_SPACE_TILE] = spacePaletteMap;
  return [tileset, tiles];
};

export const build_palette_variants = (
  font_tiles: Record<number, Surface>,
  tile_ids: number[],
  options?: { tiles?: Record<number, Surface>; allocator?: BattleVRAMAllocator; textbox_frame?: number },
): Record<number, Record<number, Surface>> => {
  const tiles =
    options?.tiles ??
    loadBattleGraphics(font_tiles, {
      allocator: options?.allocator ?? null,
      textbox_frame: options?.textbox_frame ?? 0,
    });
  const variants = buildPaletteTiles(tiles, tile_ids);
  const paletteCount = BATTLE_PALETTES.length;
  for (const [tileIdRaw, entries] of Object.entries(variants)) {
    const tileId = Number(tileIdRaw);
    if (!entries || Object.keys(entries).length === paletteCount) {
      continue;
    }
    const base = fetchTile(font_tiles, tileId);
    const paletteMap: Record<number, Surface> = {};
    for (let palIdx = 0; palIdx < paletteCount; palIdx += 1) {
      paletteMap[palIdx] = colorizeTile(base, BATTLE_PALETTES[palIdx]);
    }
    variants[tileId] = paletteMap;
  }
  return variants;
};

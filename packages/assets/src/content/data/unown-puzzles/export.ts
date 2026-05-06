// ASM: engine/games/unown_puzzle.asm constants and graphics manifests.

import fs from 'fs';
import path from 'path';
import { decompress } from '@pokecrystal/core/core/lz';
import { getAssetPath, getDataDir } from '@pokecrystal/core/core/paths';
import { START_POSITIONS, TARGET_LAYOUT } from '@pokecrystal/core/engine/games/unown-puzzle';
import { PUZZLE_IDS, computeCornerTiles } from './unown-puzzle-assets';

type CoordinateEntry = {
  oam_pixel: { x: number; y: number };
  tilemap: { x: number; y: number };
  vacant_tile: number;
};

type OamTemplateEntry = {
  y: number;
  x: number;
  tile: number;
  attributes: number;
};

const DATA_DIR = path.join(getDataDir(), 'unown_puzzles');
const GFX_DIR = getAssetPath('gfx', 'unown_puzzle');
const COORDINATES_PATH = path.join(DATA_DIR, 'coordinates.json');
const LAYOUTS_PATH = path.join(DATA_DIR, 'layouts.json');
const GRAPHICS_PATH = path.join(DATA_DIR, 'graphics.json');

const PUZZLE_BORDER = 0xee;
const PUZZLE_VOID = 0xef;

const COORDINATE_TABLE: CoordinateEntry[] = [
  { oam_pixel: { x: 28, y: 28 }, tilemap: { x: 1, y: 0 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 52, y: 28 }, tilemap: { x: 4, y: 0 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 76, y: 28 }, tilemap: { x: 7, y: 0 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 100, y: 28 }, tilemap: { x: 10, y: 0 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 124, y: 28 }, tilemap: { x: 13, y: 0 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 148, y: 28 }, tilemap: { x: 16, y: 0 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 28, y: 52 }, tilemap: { x: 1, y: 3 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 52, y: 52 }, tilemap: { x: 4, y: 3 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 76, y: 52 }, tilemap: { x: 7, y: 3 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 100, y: 52 }, tilemap: { x: 10, y: 3 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 124, y: 52 }, tilemap: { x: 13, y: 3 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 148, y: 52 }, tilemap: { x: 16, y: 3 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 28, y: 76 }, tilemap: { x: 1, y: 6 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 52, y: 76 }, tilemap: { x: 4, y: 6 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 76, y: 76 }, tilemap: { x: 7, y: 6 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 100, y: 76 }, tilemap: { x: 10, y: 6 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 124, y: 76 }, tilemap: { x: 13, y: 6 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 148, y: 76 }, tilemap: { x: 16, y: 6 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 28, y: 100 }, tilemap: { x: 1, y: 9 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 52, y: 100 }, tilemap: { x: 4, y: 9 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 76, y: 100 }, tilemap: { x: 7, y: 9 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 100, y: 100 }, tilemap: { x: 10, y: 9 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 124, y: 100 }, tilemap: { x: 13, y: 9 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 148, y: 100 }, tilemap: { x: 16, y: 9 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 28, y: 124 }, tilemap: { x: 1, y: 12 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 52, y: 124 }, tilemap: { x: 4, y: 12 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 76, y: 124 }, tilemap: { x: 7, y: 12 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 100, y: 124 }, tilemap: { x: 10, y: 12 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 124, y: 124 }, tilemap: { x: 13, y: 12 }, vacant_tile: PUZZLE_VOID },
  { oam_pixel: { x: 148, y: 124 }, tilemap: { x: 16, y: 12 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 28, y: 148 }, tilemap: { x: 1, y: 15 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 52, y: 148 }, tilemap: { x: 4, y: 15 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 76, y: 148 }, tilemap: { x: 7, y: 15 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 100, y: 148 }, tilemap: { x: 10, y: 15 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 124, y: 148 }, tilemap: { x: 13, y: 15 }, vacant_tile: PUZZLE_BORDER },
  { oam_pixel: { x: 148, y: 148 }, tilemap: { x: 16, y: 15 }, vacant_tile: PUZZLE_BORDER },
];

const OAM_TEMPLATES: Record<string, OamTemplateEntry[]> = {
  holding: [
    { y: 0xf4, x: 0xf4, tile: 0x00, attributes: 0 },
    { y: 0xf4, x: 0xfc, tile: 0x01, attributes: 0 },
    { y: 0xf4, x: 0x04, tile: 0x02, attributes: 0 },
    { y: 0xfc, x: 0xf4, tile: 0x0c, attributes: 0 },
    { y: 0xfc, x: 0xfc, tile: 0x0d, attributes: 0 },
    { y: 0xfc, x: 0x04, tile: 0x0e, attributes: 0 },
    { y: 0x04, x: 0xf4, tile: 0x18, attributes: 0 },
    { y: 0x04, x: 0xfc, tile: 0x19, attributes: 0 },
    { y: 0x04, x: 0x04, tile: 0x1a, attributes: 0 },
  ],
  idle: [
    { y: 0xf4, x: 0xf4, tile: 0x00, attributes: 0 },
    { y: 0xf4, x: 0xfc, tile: 0x01, attributes: 0 },
    { y: 0xf4, x: 0x04, tile: 0x00, attributes: 0x20 },
    { y: 0xfc, x: 0xf4, tile: 0x02, attributes: 0 },
    { y: 0xfc, x: 0xfc, tile: 0x03, attributes: 0 },
    { y: 0xfc, x: 0x04, tile: 0x02, attributes: 0x20 },
    { y: 0x04, x: 0xf4, tile: 0x00, attributes: 0x40 },
    { y: 0x04, x: 0xfc, tile: 0x01, attributes: 0x40 },
    { y: 0x04, x: 0x04, tile: 0x00, attributes: 0x60 },
  ],
};

function assetSummary(filePath: string, compressed: boolean) {
  const raw = fs.readFileSync(filePath);
  let tiles = Math.floor(raw.length / 16);
  if (compressed) {
    tiles = Math.floor(decompress(raw).length / 16);
  }
  return {
    file: path.basename(filePath),
    bytes: raw.length,
    tiles,
    compressed,
  };
}

export function exportGraphicsManifest(): void {
  const puzzles = PUZZLE_IDS.map(puzzleId => ({
    ...assetSummary(path.join(GFX_DIR, `${puzzleId.toLowerCase()}.2bpp.lz`), true),
    id: puzzleId,
  }));

  const payload = {
    assets: {
      border_tiles: assetSummary(path.join(GFX_DIR, 'tile_borders.2bpp'), false),
      cursor: assetSummary(path.join(GFX_DIR, 'cursor.2bpp'), false),
      puzzles,
      start_cancel: assetSummary(path.join(GFX_DIR, 'start_cancel.2bpp.lz'), true),
    },
    constants: {
      PUZZLE_BORDER,
      PUZZLE_VOID,
    },
    corner_tiles: computeCornerTiles(),
  };

  fs.writeFileSync(GRAPHICS_PATH, JSON.stringify(payload, null, 2));
}

export function exportCoordinates(): void {
  const payload = {
    coordinates: COORDINATE_TABLE.map(entry => ({ ...entry, filler: 0 })),
    oam_templates: OAM_TEMPLATES,
  };
  fs.writeFileSync(COORDINATES_PATH, JSON.stringify(payload, null, 2));
}

export function exportLayouts(): void {
  const payload = {
    solved_layout: TARGET_LAYOUT,
    start_positions: START_POSITIONS,
  };
  fs.writeFileSync(LAYOUTS_PATH, JSON.stringify(payload, null, 2));
}

export function exportUnownPuzzleData(): void {
  if (!fs.existsSync(DATA_DIR)) {
    throw new Error(`Expected directory is missing: ${DATA_DIR}`);
  }
  if (!fs.existsSync(GFX_DIR)) {
    throw new Error(`Expected directory is missing: ${GFX_DIR}`);
  }
  exportGraphicsManifest();
  exportCoordinates();
  exportLayouts();
}

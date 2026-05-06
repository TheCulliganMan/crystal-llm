import { Surface } from '@pokecrystal/core/ui/surface';
import { START_POSITIONS, TARGET_LAYOUT } from '@pokecrystal/core/engine/games/unown-puzzle';
import { readJsonAssetSync } from '@pokecrystal/core/core/asset-reader';
import { getDataDir } from '@pokecrystal/core/core/paths';
import { joinPath } from '@pokecrystal/core/core/path-utils';

export const PUZZLE_IDS = ['KABUTO', 'OMANYTE', 'AERODACTYL', 'HOOH'] as const;
const DEFAULT_PALETTE: Array<[number, number, number, number]> = [
  [248, 248, 248, 255],
  [176, 176, 176, 255],
  [96, 96, 96, 255],
  [0, 0, 0, 255],
];
const BORDER_OFFSETS = [0, 1, 2, 12, 14, 24, 25, 26];
const TILE_SIZE = 8;
const RAW_TILE_COUNT = 36;
const ENLARGED_TILE_COUNT = 12 * 12;
const CURSOR_BASE_TILE = 0xe0;
const START_CANCEL_BASE_TILE = 0xed;
const ASSET_BASE_PATH = 'assets/gfx/unown_puzzle';

interface CoordinatesPayloadEntry {
  tilemap: { x: number; y: number };
  oam_pixel: { x: number; y: number };
  vacant_tile: number;
}

interface OamTemplatePayload {
  y: number;
  x: number;
  tile: number;
  attributes: number;
}

interface CoordinatesPayload {
  coordinates: CoordinatesPayloadEntry[];
  oam_templates: Record<string, OamTemplatePayload[]>;
}

interface LayoutsPayload {
  solved_layout: number[][];
  start_positions: Array<[number, number]>;
}

const loadCoordinatesPayload = (() => {
  let cached: CoordinatesPayload | null = null;
  return (): CoordinatesPayload => {
    if (cached) {
      return cached;
    }
    const dataPath = joinPath(getDataDir(), 'unown_puzzles', 'coordinates.json');
    cached = readJsonAssetSync<CoordinatesPayload>(dataPath);
    return cached;
  };
})();

const loadLayoutsPayload = (() => {
  let cached: LayoutsPayload | null = null;
  return (): LayoutsPayload => {
    if (cached) {
      return cached;
    }
    const dataPath = joinPath(getDataDir(), 'unown_puzzles', 'layouts.json');
    cached = readJsonAssetSync<LayoutsPayload>(dataPath);
    return cached;
  };
})();

export class PuzzleCoordinate {
  constructor(
    public readonly tileX: number,
    public readonly tileY: number,
    public readonly oamX: number,
    public readonly oamY: number,
    public readonly vacantTile: number,
  ) {}
}

export class OamTemplate {
  constructor(
    public readonly y: number,
    public readonly x: number,
    public readonly tileOffset: number,
    public readonly attributes: number,
  ) {}

  get signedX(): number {
    return signedByte(this.x);
  }

  get signedY(): number {
    return signedByte(this.y);
  }
}

export type UnownPuzzleAssetLoader = (path: string) => Uint8Array;

let assetLoader: UnownPuzzleAssetLoader | null = null;

export function setUnownPuzzleAssetLoader(loader: UnownPuzzleAssetLoader): void {
  assetLoader = loader;
}

function requireAssetLoader(): UnownPuzzleAssetLoader {
  if (!assetLoader) {
    throw new Error('Unown puzzle asset loader has not been configured.');
  }
  return assetLoader;
}

function assetPath(filename: string): string {
  return `${ASSET_BASE_PATH}/${filename}`;
}

function loadAssetBytes(filename: string): Uint8Array {
  return requireAssetLoader()(assetPath(filename));
}

function load2bppBytes(stem: string): Uint8Array {
  return loadAssetBytes(`${stem}.2bpp`);
}

function signedByte(value: number): number {
  const masked = value & 0xff;
  return masked & 0x80 ? masked - 0x100 : masked;
}

function decode2bppTile(tileBytes: Uint8Array): number[][] {
  if (tileBytes.length !== 16) {
    throw new Error(`2bpp tile must be 16 bytes, received ${tileBytes.length}`);
  }
  const rows: number[][] = [];
  for (let row = 0; row < TILE_SIZE; row += 1) {
    const lo = tileBytes[row * 2];
    const hi = tileBytes[row * 2 + 1];
    const pixels: number[] = [];
    for (let bit = 7; bit >= 0; bit -= 1) {
      const colour = (((hi >> bit) & 1) << 1) | ((lo >> bit) & 1);
      pixels.push(colour);
    }
    rows.push(pixels);
  }
  return rows;
}

function encode2bppTile(pixels: number[][]): Uint8Array {
  if (pixels.length !== TILE_SIZE || pixels.some(row => row.length !== TILE_SIZE)) {
    throw new Error('2bpp tiles must be 8×8 pixels.');
  }
  const out: number[] = [];
  for (const row of pixels) {
    let lo = 0;
    let hi = 0;
    row.forEach((value, bit) => {
      const color = value & 0x3;
      const shift = 7 - bit;
      lo |= (color & 0x1) << shift;
      hi |= ((color >> 1) & 0x1) << shift;
    });
    out.push(lo, hi);
  }
  return Uint8Array.from(out);
}

function tilesToImage(tiles: Uint8Array[], widthTiles: number, heightTiles: number): number[][] {
  if (tiles.length !== widthTiles * heightTiles) {
    throw new Error(`Expected ${widthTiles * heightTiles} tiles, found ${tiles.length} instead.`);
  }
  const canvas: number[][] = Array.from({ length: heightTiles * TILE_SIZE }, () =>
    Array.from({ length: widthTiles * TILE_SIZE }, () => 0),
  );
  tiles.forEach((tileBytes, index) => {
    const tileX = index % widthTiles;
    const tileY = Math.floor(index / widthTiles);
    const pixels = decode2bppTile(tileBytes);
    pixels.forEach((row, py) => {
      row.forEach((colour, px) => {
        canvas[tileY * TILE_SIZE + py][tileX * TILE_SIZE + px] = colour;
      });
    });
  });
  return canvas;
}

function imageToTiles(pixels: number[][], widthTiles: number, heightTiles: number): Uint8Array[] {
  if (pixels.length !== heightTiles * TILE_SIZE) {
    throw new Error('Pixel buffer height does not match the requested tile grid.');
  }
  if (pixels.some(row => row.length !== widthTiles * TILE_SIZE)) {
    throw new Error('Pixel buffer width does not match the requested tile grid.');
  }
  const tiles: Uint8Array[] = [];
  for (let tileY = 0; tileY < heightTiles; tileY += 1) {
    for (let tileX = 0; tileX < widthTiles; tileX += 1) {
      const slicePx = pixels
        .slice(tileY * TILE_SIZE, (tileY + 1) * TILE_SIZE)
        .map(row => row.slice(tileX * TILE_SIZE, (tileX + 1) * TILE_SIZE));
      tiles.push(encode2bppTile(slicePx));
    }
  }
  return tiles;
}

function scalePixels2x(pixels: number[][]): number[][] {
  const scaled: number[][] = [];
  for (const row of pixels) {
    const expandedRow: number[] = [];
    for (const colour of row) {
      expandedRow.push(colour, colour);
    }
    scaled.push([...expandedRow]);
    scaled.push([...expandedRow]);
  }
  return scaled;
}

export function applyPieceBorders(tiles: Uint8Array[], borderTiles: Uint8Array[]): void {
  if (borderTiles.length !== BORDER_OFFSETS.length) {
    throw new Error(
      `Expected ${BORDER_OFFSETS.length} border tiles, received ${borderTiles.length}.`,
    );
  }
  if (tiles.length !== ENLARGED_TILE_COUNT) {
    throw new Error(`Border overlays require ${ENLARGED_TILE_COUNT} tiles, found ${tiles.length}.`);
  }
  const overlay = (base: Uint8Array, overlayBytes: Uint8Array): Uint8Array => {
    if (base.length !== overlayBytes.length) {
      throw new Error('Border overlay tile size mismatch.');
    }
    return Uint8Array.from(base.map((value, index) => (value | overlayBytes[index]) & 0xff));
  };
  for (let pieceRow = 0; pieceRow < 12; pieceRow += 3) {
    for (let pieceCol = 0; pieceCol < 12; pieceCol += 3) {
      const baseIndex = pieceRow * 12 + pieceCol;
      BORDER_OFFSETS.forEach((offset, idx) => {
        tiles[baseIndex + offset] = overlay(tiles[baseIndex + offset], borderTiles[idx]);
      });
    }
  }
}

export function loadPuzzleRawBytes(puzzleId: string): Uint8Array {
  const normalized = puzzleId.trim().toUpperCase();
  return load2bppBytes(normalized.toLowerCase());
}

export function loadBorderTiles(): Uint8Array[] {
  const data = loadAssetBytes('tile_borders.2bpp');
  if (data.length % 16 !== 0) {
    throw new Error('Border tile payload must be 16-byte aligned.');
  }
  const tiles: Uint8Array[] = [];
  for (let i = 0; i < data.length; i += 16) {
    tiles.push(data.slice(i, i + 16));
  }
  return tiles;
}

export function loadCursorTiles(): Uint8Array[] {
  const data = loadAssetBytes('cursor.2bpp');
  if (data.length % 16 !== 0) {
    throw new Error('Cursor tiles must be 16-byte aligned.');
  }
  const tiles: Uint8Array[] = [];
  for (let i = 0; i < data.length; i += 16) {
    tiles.push(data.slice(i, i + 16));
  }
  return tiles;
}

export function loadStartCancelTiles(): Uint8Array[] {
  const data = load2bppBytes('start_cancel');
  if (data.length % 16 !== 0) {
    throw new Error('Start/Cancel payload must be tile-aligned.');
  }
  const tiles: Uint8Array[] = [];
  for (let i = 0; i < data.length; i += 16) {
    tiles.push(data.slice(i, i + 16));
  }
  return tiles;
}

export function convertPuzzleTiles(puzzleId: string): Uint8Array[] {
  const rawBytes = loadPuzzleRawBytes(puzzleId);
  if (rawBytes.length % 16 !== 0) {
    throw new Error('Puzzle art must align to 16-byte tiles.');
  }
  const tiles: Uint8Array[] = [];
  for (let i = 0; i < rawBytes.length; i += 16) {
    tiles.push(rawBytes.slice(i, i + 16));
  }
  if (tiles.length !== RAW_TILE_COUNT) {
    throw new Error(`Puzzle payload should contain ${RAW_TILE_COUNT} tiles, found ${tiles.length}.`);
  }
  const image = tilesToImage(tiles, 6, 6);
  const scaled = scalePixels2x(image);
  const enlarged = imageToTiles(scaled, 12, 12);
  applyPieceBorders(enlarged, loadBorderTiles());
  return enlarged;
}

export function computeCornerTiles(cursorTile: number = CURSOR_BASE_TILE): number[] {
  const corners = [cursorTile];
  for (let row = 0; row < 4; row += 1) {
    for (let col = 0; col < 4; col += 1) {
      corners.push(row * 36 + col * 3);
    }
  }
  return corners;
}

export function loadCoordinates(): [PuzzleCoordinate[], Record<string, OamTemplate[]>] {
  const coords: PuzzleCoordinate[] = [];
  const coordinatesPayload = loadCoordinatesPayload();
  for (const entry of coordinatesPayload.coordinates ?? []) {
    coords.push(
      new PuzzleCoordinate(
        Number(entry.tilemap.x),
        Number(entry.tilemap.y),
        Number(entry.oam_pixel.x),
        Number(entry.oam_pixel.y),
        Number(entry.vacant_tile),
      ),
    );
  }

  const templates: Record<string, OamTemplate[]> = {};
  const sourceTemplates = coordinatesPayload.oam_templates ?? {};
  for (const key of Object.keys(sourceTemplates)) {
    templates[key] = sourceTemplates[key].map(
      template => new OamTemplate(template.y, template.x, template.tile, template.attributes),
    );
  }
  return [coords, templates];
}

export function loadLayouts(): [number[][], Array<[number, number]>] {
  const layoutsPayload = loadLayoutsPayload();
  const solved = layoutsPayload.solved_layout.map(row => row.map(Number));
  const starts = layoutsPayload.start_positions.map(
    ([x, y]) => [Number(x), Number(y)] as [number, number],
  );
  if (JSON.stringify(solved) !== JSON.stringify(TARGET_LAYOUT)) {
    throw new Error('Exported solved layout does not match engine constant.');
  }
  if (JSON.stringify(starts) !== JSON.stringify(START_POSITIONS)) {
    throw new Error('Exported start positions do not match engine constant.');
  }
  return [solved, starts];
}

export function buildTileSurfaces(
  puzzleId: string,
  palette: Array<[number, number, number, number]> = DEFAULT_PALETTE,
): Record<number, Surface> {
  const tiles: Record<number, Surface> = {};
  const renderTile = (tileBytes: Uint8Array): Surface => {
    const pixels = decode2bppTile(tileBytes);
    const surface = new Surface(TILE_SIZE, TILE_SIZE);
    pixels.forEach((row, y) => {
      row.forEach((colour, x) => {
        const [r, g, b, a] = palette[colour];
        surface.setAt(x, y, [r, g, b, a]);
      });
    });
    return surface;
  };

  convertPuzzleTiles(puzzleId).forEach((tile, index) => {
    tiles[index] = renderTile(tile);
  });

  loadCursorTiles().forEach((tile, offset) => {
    tiles[CURSOR_BASE_TILE + offset] = renderTile(tile);
  });

  loadStartCancelTiles().forEach((tile, offset) => {
    tiles[START_CANCEL_BASE_TILE + offset] = renderTile(tile);
  });

  return tiles;
}

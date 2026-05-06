export const DEFAULT_TILE_SHIFT = 0;
export const DEFAULT_PALETTE_INVERSIONS: Readonly<Record<number, boolean>> = {};
export const DEFAULT_TILE_INDEX_MODE = "offset" as const;

const GRAPHIC_REDIRECTS: Record<string, string> = {
  unown_a: "unowns",
  unown_hi: "unowns",
  unowns: "unowns",
  background: "background",
  suicune_jump: "suicune_jump",
  suicune_close: "suicune_close",
  suicune_back: "suicune_back",
  crystal_unowns: "crystal_unowns",
};

export interface TilemapDefaults {
  tile_shift: number;
  tile_index_mode: TileIndexMode;
  palette_inversions: Record<number, boolean>;
}

export type TileIndexMode = "offset" | "signed";

const INTRO_TILEMAP_DEFAULTS: Record<string, TilemapDefaults> = {
  unowns: { tile_shift: 0x80, tile_index_mode: "offset", palette_inversions: {} },
  crystal_unowns: { tile_shift: 0x80, tile_index_mode: "offset", palette_inversions: {} },
  suicune_close: { tile_shift: 0x80, tile_index_mode: "signed", palette_inversions: {} },
};

export function resolveGraphicName(mapName: string): string {
  return GRAPHIC_REDIRECTS[mapName] ?? mapName;
}

function lookupTilemapDefaults(mapName: string): TilemapDefaults | null {
  if (INTRO_TILEMAP_DEFAULTS[mapName]) {
    return INTRO_TILEMAP_DEFAULTS[mapName];
  }
  const resolved = resolveGraphicName(mapName);
  if (resolved !== mapName) {
    return INTRO_TILEMAP_DEFAULTS[resolved] ?? null;
  }
  return null;
}

export function getTileShift(mapName: string): number {
  const defaults = lookupTilemapDefaults(mapName);
  return defaults ? defaults.tile_shift : DEFAULT_TILE_SHIFT;
}

export function getPaletteInversions(mapName: string): Record<number, boolean> {
  const palette: Record<number, boolean> = { ...DEFAULT_PALETTE_INVERSIONS };
  const defaults = lookupTilemapDefaults(mapName);
  if (defaults) {
    Object.assign(palette, defaults.palette_inversions);
  }
  return palette;
}

export function getTileIndexMode(mapName: string): TileIndexMode {
  const defaults = lookupTilemapDefaults(mapName);
  return defaults ? defaults.tile_index_mode : DEFAULT_TILE_INDEX_MODE;
}

export const SLOT_ICON_TILE_SIZE = 8;
export const SLOT_ICON_SPRITE_TILES = 2;
export const SLOT_ICON_SURFACE_SIZE = SLOT_ICON_TILE_SIZE * SLOT_ICON_SPRITE_TILES;
export const SLOT_ICON_TILE_STRIDE = 4;
export const SLOT_ICON_TEXT_START_TILE_X = 2;
export const SLOT_ICON_X_STEP_TILES = 6;
export const SLOT_ICON_Y_START_TILE = 7;
export const SLOT_ICON_Y_STEP_TILES = 2;
export const SLOT_STATUS_LINE_TILE = 13;

export const slotIconTileIndices = (baseTileIndex: number): [number, number, number, number] => [
  baseTileIndex,
  baseTileIndex + 1,
  baseTileIndex + 2,
  baseTileIndex + 3,
];

export const slotIconRowStartTile = (rowIndex: number): number =>
  SLOT_ICON_Y_START_TILE + rowIndex * SLOT_ICON_Y_STEP_TILES;


// ASM mapping: engine/games/slot_machine.asm
// - Slots_InitReelTiles writes REEL_X_COORD as 6, 10, 14 tiles (OAM coordinates).
// - Slots_UpdateReelPositionAndOAM starts reel Y at 10 tiles and steps by 2 tiles.
// OAM coordinates include hardware offsets (x + 8 px, y + 16 px), so convert
// to screen-space tile positions before drawing to the canvas surface.

const OAM_X_OFFSET_TILES = 1;
const OAM_Y_OFFSET_TILES = 2;

export const SLOT_REEL_X_OAM_TILES = [6, 10, 14] as const;
export const SLOT_REEL_Y_OAM_TILES = [6, 8, 10] as const;

export const SLOT_REEL_X_TILES: readonly [number, number, number] = [
  SLOT_REEL_X_OAM_TILES[0] - OAM_X_OFFSET_TILES,
  SLOT_REEL_X_OAM_TILES[1] - OAM_X_OFFSET_TILES,
  SLOT_REEL_X_OAM_TILES[2] - OAM_X_OFFSET_TILES,
];

export const SLOT_REEL_Y_TILES: readonly [number, number, number] = [
  SLOT_REEL_Y_OAM_TILES[0] - OAM_Y_OFFSET_TILES,
  SLOT_REEL_Y_OAM_TILES[1] - OAM_Y_OFFSET_TILES,
  SLOT_REEL_Y_OAM_TILES[2] - OAM_Y_OFFSET_TILES,
];

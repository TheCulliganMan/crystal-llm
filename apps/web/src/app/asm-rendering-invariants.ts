import type { BaseUI } from "@pokecrystal/core/ui/base-ui";

// ASM mapping: constants/hardware_constants.asm and tilemap dimensions used by
// Crystal UI and overworld render paths (20x18 tiles at 8x8 px => 160x144).
export const ASM_SCREEN_WIDTH_PX = 160;
export const ASM_SCREEN_HEIGHT_PX = 144;
export const ASM_TILE_SIZE_PX = 8;
export const ASM_SCREEN_WIDTH_TILES = 20;
export const ASM_SCREEN_HEIGHT_TILES = 18;

export const assertAsmScale = (scale: number, context: string): void => {
  if (!Number.isInteger(scale) || scale <= 0) {
    throw new Error(`[${context}] Scale must be a positive integer, got ${scale}.`);
  }
};

export const assertAsmScreenDimensions = (
  width: number,
  height: number,
  context: string,
): void => {
  if (width !== ASM_SCREEN_WIDTH_PX || height !== ASM_SCREEN_HEIGHT_PX) {
    throw new Error(
      `[${context}] Expected ASM screen ${ASM_SCREEN_WIDTH_PX}x${ASM_SCREEN_HEIGHT_PX}, got ${width}x${height}.`,
    );
  }
};

export const assertAsmTileGeometry = (tileSize: number, context: string): void => {
  if (tileSize !== ASM_TILE_SIZE_PX) {
    throw new Error(`[${context}] Expected ASM tile size ${ASM_TILE_SIZE_PX}, got ${tileSize}.`);
  }
};

export const assertAsmUiInvariants = (ui: BaseUI, context: string): void => {
  assertAsmScreenDimensions(ui.screenWidth, ui.screenHeight, context);
  assertAsmTileGeometry(ui.tile_size, context);
  assertAsmTileGeometry(ui.tileSize, context);
  assertAsmScale(ui.scale, context);
};


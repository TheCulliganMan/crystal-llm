import { Surface } from "../surface";

// ASM: pokecrystal_disassembly/engine/gfx/place_graphic.asm::PlaceGraphic
export const assemble_place_graphic_surface = (
  tiles: Surface[],
  widthTiles: number,
  heightTiles: number
): Surface => {
  if (widthTiles <= 0 || heightTiles <= 0) {
    throw new Error("PlaceGraphic surface requires positive tile dimensions.");
  }
  if (tiles.length < widthTiles * heightTiles) {
    throw new Error(
      `PlaceGraphic requires ${widthTiles * heightTiles} tiles, got ${tiles.length}`
    );
  }
  const baseTile = tiles[0];
  if (!baseTile) {
    throw new Error("PlaceGraphic requires at least one tile.");
  }
  const tileWidth = baseTile.width;
  const tileHeight = baseTile.height;
  const surface = new Surface(widthTiles * tileWidth, heightTiles * tileHeight);
  for (let row = 0; row < heightTiles; row += 1) {
    for (let col = 0; col < widthTiles; col += 1) {
      const tileIndex = row * widthTiles + col;
      const tile = tiles[tileIndex];
      if (!tile) {
        throw new Error(`PlaceGraphic missing tile ${tileIndex}.`);
      }
      surface.blit(tile, [col * tileWidth, row * tileHeight]);
    }
  }
  return surface;
};

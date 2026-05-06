import { Surface } from "@pokecrystal/core/ui/surface";
import { partyMenuTileset } from "./party-menu-layout";
import type { Palette } from "@pokecrystal/core/ui/font-renderer";

const blankTile = (): Surface => {
  const surface = new Surface(8, 8);
  surface.fill([255, 255, 255, 255]);
  return surface;
};

describe("party-menu-layout", () => {
  it("uses the asm PartyMenu palette packet for font variants and HP tiles", () => {
    let captured: ReadonlyArray<Palette> = [];
    const fontTiles: Record<number, Surface> = {};
    for (let tileId = 0x60; tileId < 0x6c; tileId += 1) {
      fontTiles[tileId] = blankTile();
    }

    partyMenuTileset({
      fontTiles,
      paletteVariants: (palettes) => {
        captured = palettes;
        return {};
      },
    });

    expect(captured[0]).toEqual([
      [255, 255, 255],
      [148, 148, 148],
      [82, 82, 82],
      [0, 0, 0],
    ]);
    expect(captured[1]).toEqual([
      [255, 255, 255],
      [247, 214, 123],
      [0, 189, 0],
      [0, 0, 0],
    ]);
    expect(captured[2][2]).toEqual([255, 189, 0]);
    expect(captured[3][2]).toEqual([255, 0, 0]);
    expect(captured[7]).toEqual(captured[0]);
  });
});

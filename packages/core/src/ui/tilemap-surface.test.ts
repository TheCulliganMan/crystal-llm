import { TilemapSurface } from "@pokecrystal/core/ui/tilemap-surface";
import { Surface } from "@pokecrystal/core/ui/surface";

describe("TilemapSurface parity guards", () => {
  it("throws when a tile id has no tileset entry", () => {
    const tilemap = new TilemapSurface(1, 1);
    const screen = new Surface(8, 8);
    tilemap.setTile(0, 0, 1);

    expect(() => tilemap.blit(screen, [])).toThrow(
      "TilemapSurface requires a tile for id 0x01 attr 0x00 at (0,0)",
    );
  });

  it("throws when a palette-specific tile variant is missing", () => {
    const tilemap = new TilemapSurface(1, 1);
    const screen = new Surface(8, 8);
    tilemap.setTile(0, 0, 1, 3);
    const tile = new Surface(8, 8);

    expect(() => tilemap.blit(screen, { 1: { 1: tile } })).toThrow(
      "TilemapSurface requires a tile for id 0x01 attr 0x03 at (0,0)",
    );
  });
});

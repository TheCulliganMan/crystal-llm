import { Tileset } from "./tileset-data";

describe("Tileset data loader", () => {
  it("loads exported tile-index palette maps without treating them as metatile records", async () => {
    const tileset = await Tileset.fromTilesetName("ruins_of_alph");

    expect(tileset.paletteMap.get(0x00)).toBe(0x00);
    expect(tileset.paletteMap.get(0x02)).toBe(0x05);
    expect(tileset.paletteMap.get(0x60)).toBe(0x0f);
    expect(tileset.paletteMap.get(0x80)).toBe(0x0d);
  });
});

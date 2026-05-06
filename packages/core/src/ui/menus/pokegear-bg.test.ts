import { PokegearBackground } from "@pokecrystal/core/ui/menus/pokegear-bg";

const levelsHaveInk = (levels: number[]): boolean => {
  const first = levels[0];
  return levels.some((value) => value !== first);
};

describe("PokegearBackground", () => {
  it("injects font tiles for town map labels", () => {
    const background = new PokegearBackground();
    const tiles = background.tileSurfaces();
    expect(tiles.length).toBeGreaterThan(0xff);
    const fontLevels: number[][] = (background as any).fontTileLevels ?? [];
    const mixedCount = fontLevels.filter((levels) => levelsHaveInk(levels)).length;
    expect(mixedCount).toBeGreaterThan(10);
    expect(tiles[0x80]).toBeDefined();
  });

  it("maps exported pokegear PNG grayscale to Game Boy color indexes", () => {
    const background = new PokegearBackground();
    const tiles = background.tileSurfaces();

    const townMapDarkGrayPixel = tiles[0].getAt(0, 0);
    expect(townMapDarkGrayPixel[2]).toBeGreaterThan(townMapDarkGrayPixel[1]);

    const pokegearWhitePixel = tiles[0x30].getAt(5, 0);
    expect(pokegearWhitePixel[0]).toBeGreaterThan(200);
    expect(pokegearWhitePixel[1]).toBeGreaterThan(200);
    expect(pokegearWhitePixel[2]).toBeGreaterThan(100);
  });
});

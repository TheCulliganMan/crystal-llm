import { DexMode } from "@pokecrystal/core/core/enums/pokedex";
import { drawPokedexList, type PokedexLayoutEntry, type PokedexLayoutUI } from "@pokecrystal/core/ui/menus/pokedex-layout";
import { Surface } from "@pokecrystal/core/ui/surface";
import { ensurePokedexTiles, resetPokedexHardwareState } from "@pokecrystal/core/ui/menus/pokedex-render";
import { buildDefaultCharMap } from "@pokecrystal/core/ui/text/glyph-map";

const buildFontTiles = (): Record<number, Surface> => {
  const charMap = buildDefaultCharMap();
  const tiles: Record<number, Surface> = {};
  for (const char of ["T", "O", "D", "I", "L", "E", "-", "0", "7"]) {
    const tileIndex = charMap[char];
    const tile = new Surface(8, 8);
    tile.fill([0, 0, 0, 255]);
    tiles[tileIndex] = tile;
  }
  return tiles;
};

const createUi = (renderText: jest.Mock): PokedexLayoutUI => ({
  font: {
    renderText: renderText as PokedexLayoutUI["font"]["renderText"],
    font_tiles: buildFontTiles(),
    reloadFontExtraTiles: jest.fn(),
  },
});

describe("drawPokedexList seen/caught matching", () => {
  beforeEach(() => {
    resetPokedexHardwareState();
  });

  it("uses species int_id when deciding if a NEW mode entry is seen", () => {
    const renderText = jest.fn();
    const ui = createUi(renderText);
    const screen = new Surface(160, 144);
    const entries: PokedexLayoutEntry[] = [
      { pokedexNumber: 7, species: { id: "TOTODILE", int_id: 158 } },
    ];

    drawPokedexList(
      ui,
      screen,
      entries,
      0,
      0,
      new Set([158]),
      new Set([158]),
      DexMode.NEW,
      1,
      { drawWindow: false },
    );

    const writtenTexts = renderText.mock.calls.map((call) => String(call[0]));
    expect(writtenTexts).toContain("TOTODILE");
    expect(writtenTexts).not.toContain("-----");
  });

  it("falls back to pokedexNumber when species int_id is unavailable", () => {
    const renderText = jest.fn();
    const ui = createUi(renderText);
    const screen = new Surface(160, 144);
    const entries: PokedexLayoutEntry[] = [
      { pokedexNumber: 7, species: { id: "TOTODILE" } },
    ];

    drawPokedexList(
      ui,
      screen,
      entries,
      0,
      0,
      new Set([7]),
      new Set([7]),
      DexMode.NEW,
      1,
      { drawWindow: false },
    );

    const writtenTexts = renderText.mock.calls.map((call) => String(call[0]));
    expect(writtenTexts).toContain("TOTODILE");
    expect(writtenTexts).not.toContain("-----");
  });

  it("uses the stable pokedex tileset for the main list frame even if font tile slots were overwritten", () => {
    const renderText = jest.fn();
    const ui = createUi(renderText);
    ensurePokedexTiles(ui);
    const poisonedTile = new Surface(8, 8);
    poisonedTile.fill([255, 0, 255, 255]);
    ui.font.font_tiles![0x34] = poisonedTile;
    ui.font.font_tiles![0x39] = poisonedTile;
    ui.font.font_tiles![0x3f] = poisonedTile;

    const screen = new Surface(160, 144);
    const entries: PokedexLayoutEntry[] = [
      { pokedexNumber: 7, species: { id: "TOTODILE", int_id: 158 } },
    ];

    drawPokedexList(
      ui,
      screen,
      entries,
      0,
      0,
      new Set([158]),
      new Set([158]),
      DexMode.NEW,
      1,
    );

    expect(screen.get_at([4, 4])).not.toEqual([255, 0, 255, 255]);
    expect(screen.get_at([4, 16 * 8 + 4])).not.toEqual([255, 0, 255, 255]);
  });
});

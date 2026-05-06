import { DexScreenState, buildPokedexControlLines } from "@pokecrystal/core/ui/menus/pokedex";

describe("buildPokedexControlLines", () => {
  it("returns controls for each screen state", () => {
    expect(buildPokedexControlLines(DexScreenState.MAIN)).toEqual([
      "D-Pad=Move L/R=Page A=Entry",
      "Start=Search Select=Options B=Exit",
    ]);
    expect(buildPokedexControlLines(DexScreenState.ENTRY)).toEqual([
      "Up/Down=Prev/Next L/R=Action",
      "A=Select B=Back",
    ]);
    expect(buildPokedexControlLines(DexScreenState.OPTIONS)).toEqual([
      "Up/Down=Move A/Start=Select",
      "B/Select=Back",
    ]);
    expect(buildPokedexControlLines(DexScreenState.SEARCH)).toEqual([
      "Up/Down=Move L/R=Type A=Confirm Start=Back",
    ]);
    expect(buildPokedexControlLines(DexScreenState.SEARCH_RESULTS)).toEqual([
      "Up/Down=Move L/R=Page A=Entry B=Back",
    ]);
    expect(buildPokedexControlLines(DexScreenState.UNOWN)).toEqual([
      "L/R=Move A/B/Select=Back",
    ]);
  });
});

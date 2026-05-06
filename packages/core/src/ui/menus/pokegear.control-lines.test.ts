import { PokegearCard } from "@pokecrystal/core/ui/menus/pokegear-state";
import { buildPokegearControlLines } from "@pokecrystal/core/ui/menus/pokegear";

describe("buildPokegearControlLines", () => {
  it("returns controls for each pokegear card", () => {
    expect(buildPokegearControlLines(PokegearCard.CLOCK)).toEqual([
      "L/R=Card B=Exit",
    ]);
    expect(buildPokegearControlLines(PokegearCard.MAP)).toEqual([
      "L/R=Card B=Exit",
      "Up/Down=Move",
    ]);
    expect(buildPokegearControlLines(PokegearCard.PHONE)).toEqual([
      "L/R=Card B=Exit",
      "Up/Down=Move A=Call",
    ]);
    expect(buildPokegearControlLines(PokegearCard.RADIO)).toEqual([
      "L/R=Card B=Exit",
      "Up/Down=Tune",
    ]);
  });
});

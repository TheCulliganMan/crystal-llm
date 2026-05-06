import { projectLandmarkToTownMapPixel } from "@pokecrystal/core/ui/overlays/town-map-coords";

describe("projectLandmarkToTownMapPixel", () => {
  it("converts ASM landmark OAM coordinates to town map surface pixels", () => {
    expect(
      projectLandmarkToTownMapPixel({
        id: 12,
        constant: "LANDMARK_AZALEA_TOWN",
        label: "AZALEA_TOWN",
        name: "Azalea Town",
        x: 76,
        y: 140,
        region: "JOHTO",
      })
    ).toEqual([68, 124]);
  });
});

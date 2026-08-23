import { getLandmarkLabel, resolveLandmarkText } from "./pokegear-labels";

describe("pokegear-labels", () => {
  it("resolves landmark labels from bundled pokegear data", () => {
    expect(getLandmarkLabel("VIOLET_CITY")).toBe("VIOLET CITY");
    expect(getLandmarkLabel("MT_MORTAR")).toBe("MT.MORTAR");
    expect(getLandmarkLabel("BATTLE_TOWER")).toBe("BATTLE TOWER");
  });

  it("does not synthesize landmark name aliases", () => {
    expect(getLandmarkLabel("VioletCityName")).toBeUndefined();
  });

  it("falls back to the entry name when a label is unavailable", () => {
    expect(
      resolveLandmarkText({
        id: 999,
        constant: "LANDMARK_TEST",
        label: "MissingLabel",
        name: "TEST PLACE",
        x: 0,
        y: 0,
        region: "JOHTO",
      })
    ).toBe("TEST PLACE");
  });

  it("preserves the ASM town-map line break", () => {
    expect(
      resolveLandmarkText({
        id: 999,
        constant: "LANDMARK_TEST",
        label: "MissingLabel",
        name: "NATIONAL PARK",
        x: 0,
        y: 0,
        region: "JOHTO",
      })
    ).toBe("NATIONAL\nPARK");
  });
});

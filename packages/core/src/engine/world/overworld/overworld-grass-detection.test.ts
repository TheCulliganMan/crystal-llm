import { OverworldEngine } from "./overworld";

describe("OverworldEngine grass detection parity", () => {
  it("returns false until map collision data is available", () => {
    const fakeOverworld = {
      current_map_name: "TEST_MAP",
      map: null,
      tileset: {
        metatiles: [],
      },
    };

    expect(
      (OverworldEngine.prototype as unknown as {
        _detect_map_grass: (this: typeof fakeOverworld) => boolean;
      })._detect_map_grass.call(fakeOverworld),
    ).toBe(false);
  });

  it("throws instead of assuming tall grass when map metatile IDs are invalid", () => {
    const fakeOverworld = {
      current_map_name: "TEST_MAP",
      map: {
        metatileIds: [3],
      },
      tileset: {
        metatiles: [{ collision: [0] }],
      },
    };

    expect(() =>
      (OverworldEngine.prototype as unknown as {
        _detect_map_grass: (this: typeof fakeOverworld) => boolean;
      })._detect_map_grass.call(fakeOverworld),
    ).toThrow("Map 'TEST_MAP' references invalid metatile 3 during grass detection.");
  });
});

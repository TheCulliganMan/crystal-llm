import { defaultMusicTokenForMap } from "./map-music";

describe("defaultMusicTokenForMap", () => {
  it("throws instead of fabricating route music for unknown route labels", () => {
    expect(() => defaultMusicTokenForMap("Route999")).toThrow(
      "No default music mapping for map 'Route999'.",
    );
  });

  it("throws instead of fabricating Pokecenter music for unknown center labels", () => {
    expect(() => defaultMusicTokenForMap("MissingPokecenter1F")).toThrow(
      "No default music mapping for map 'MissingPokecenter1F'.",
    );
  });

  it("uses bundled map attributes and does not require disassembly maps.asm", () => {
    jest.resetModules();
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: (target: string) => {
        if (target !== "/tmp/assets/data/map_attributes.json") {
          throw new Error(`unexpected asset path ${target}`);
        }
        return {
          TestMap: { music: "MUSIC_TEST" },
        };
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => "/tmp/assets/data",
      getDisassemblyRoot: () => {
        throw new Error("map-music should not read the disassembly root");
      },
    }));

    jest.isolateModules(() => {
      const { defaultMusicTokenForMap: resolveMusic } = require("./map-music") as typeof import("./map-music");
      expect(resolveMusic("TestMap")).toBe("MUSIC_TEST");
    });
  });
});

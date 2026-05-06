describe("flee monster runtime assets", () => {
  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
  });

  it("loads flee species tables from the bundled runtime asset", () => {
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: (target: string) => {
        if (target.endsWith("flee_mons.json")) {
          return {
            always: ["RAIKOU", "ENTEI"],
            often: ["DELIBIRD"],
            sometimes: ["MAGNEMITE"],
          };
        }
        throw new Error(`unexpected asset read: ${target}`);
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => "/tmp/assets/data",
    }));
    jest.doMock("@pokecrystal/core/core/path-utils", () => ({
      joinPath: (...parts: string[]) => parts.join("/"),
    }));

    jest.isolateModules(() => {
      const {
        getAlwaysFleeSpecies,
        getOftenFleeSpecies,
        getSometimesFleeSpecies,
      } = require("./flee-constants") as typeof import("./flee-constants");

      expect(getAlwaysFleeSpecies().has("RAIKOU")).toBe(true);
      expect(getOftenFleeSpecies().has("DELIBIRD")).toBe(true);
      expect(getSometimesFleeSpecies().has("MAGNEMITE")).toBe(true);
    });
  });

  it("throws clearly when flee_mons.json is missing", () => {
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: () => {
        throw new Error("ENOENT");
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => "/tmp/assets/data",
    }));
    jest.doMock("@pokecrystal/core/core/path-utils", () => ({
      joinPath: (...parts: string[]) => parts.join("/"),
    }));

    jest.isolateModules(() => {
      const { getAlwaysFleeSpecies } = require("./flee-constants") as typeof import("./flee-constants");

      expect(() => getAlwaysFleeSpecies()).toThrow(
        "Flee monster tables are required for the asset-only runtime: missing or invalid /tmp/assets/data/flee_mons.json."
      );
    });
  });

  it("throws clearly when flee_mons.json has the wrong shape", () => {
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: () => ({ always: ["RAIKOU"] }),
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => "/tmp/assets/data",
    }));
    jest.doMock("@pokecrystal/core/core/path-utils", () => ({
      joinPath: (...parts: string[]) => parts.join("/"),
    }));

    jest.isolateModules(() => {
      const { getAlwaysFleeSpecies } = require("./flee-constants") as typeof import("./flee-constants");

      expect(() => getAlwaysFleeSpecies()).toThrow(
        "Flee monster tables are required for the asset-only runtime: missing or invalid /tmp/assets/data/flee_mons.json."
      );
    });
  });
});

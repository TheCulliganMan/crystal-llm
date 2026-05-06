describe("maps asset runtime", () => {
  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
  });

  it("loads required map metadata and spawn points from bundled assets", () => {
    const bundledMetadata = {
      NEW_BARK_TOWN: {
        constant: "NEW_BARK_TOWN",
        name: "NewBarkTown",
        groupName: "NEW_BARK",
        groupId: 1,
        mapId: 1,
        width: 10,
        height: 9,
        environment: "TOWN",
        phoneService: 0,
      },
    };
    const bundledSpawnPoints = {
      0: {
        identifier: 0,
        mapConstant: "NEW_BARK_TOWN",
        mapName: "NewBarkTown",
        groupId: 1,
        mapId: 1,
        tileX: 6,
        tileY: 6,
        groupName: "NEW_BARK",
        metatileX: 3,
        metatileY: 3,
        subtileX: 0,
        subtileY: 0,
      },
    };

    jest.doMock("path", () => ({
      __esModule: true,
      default: { join: (...parts: string[]) => parts.join("/") },
      join: (...parts: string[]) => parts.join("/"),
    }));
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: (target: string) => {
        if (target.endsWith("runtime_map_metadata.json")) {
          return bundledMetadata;
        }
        if (target.endsWith("runtime_spawn_points.json")) {
          return bundledSpawnPoints;
        }
        throw new Error(`unexpected asset read: ${target}`);
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getAssetsRoot: () => "/tmp/assets",
      getDataDir: () => "/tmp/assets/data",
      getDisassemblyRoot: () => "/tmp/disassembly",
    }));

    jest.isolateModules(() => {
      const { getMapMetadataByConstant, getSpawnPoint, findSpawnForMap } = require("./maps") as typeof import("./maps");

      expect(getMapMetadataByConstant("NEW_BARK_TOWN")).toEqual(bundledMetadata.NEW_BARK_TOWN);
      expect(getSpawnPoint(0)).toEqual(bundledSpawnPoints[0]);
      expect(findSpawnForMap(1, 1)).toEqual([0, bundledSpawnPoints[0]]);
    });
  });

  it("throws when required runtime map metadata is missing", () => {
    jest.doMock("path", () => ({
      __esModule: true,
      default: { join: (...parts: string[]) => parts.join("/") },
      join: (...parts: string[]) => parts.join("/"),
    }));
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: (target: string) => {
        if (target.endsWith("runtime_map_metadata.json")) {
          throw new Error("missing");
        }
        return {};
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getAssetsRoot: () => "/tmp/assets",
      getDataDir: () => "/tmp/assets/data",
      getDisassemblyRoot: () => "/tmp/disassembly",
    }));

    jest.isolateModules(() => {
      const { getMapMetadataByConstant } = require("./maps") as typeof import("./maps");

      expect(() => getMapMetadataByConstant("NEW_BARK_TOWN")).toThrow(
        "Runtime map metadata is required for the asset-only runtime: missing or invalid /tmp/assets/data/runtime_map_metadata.json."
      );
    });
  });
});

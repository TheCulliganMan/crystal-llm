describe("map-blocks asset runtime", () => {
  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
  });

  it("reads required bundled map block bytes", () => {
    const bundledBytes = Buffer.from([1, 2, 3, 4]);

    jest.doMock("./asset-reader", () => ({
      readJsonAssetSync: (target: string) => {
        if (target.endsWith("map_blocks.json")) {
          return { NewBarkTown_Blocks: bundledBytes.toString("base64") };
        }
        throw new Error(`unexpected asset read: ${target}`);
      },
    }));
    jest.doMock("./paths", () => ({
      getAssetsRoot: () => "/tmp/assets",
      getDataDir: () => "/tmp/assets/data",
      getDisassemblyRoot: () => "/tmp/disassembly",
    }));

    jest.isolateModules(() => {
      const { readMapBlockBytes } = require("./map-blocks") as typeof import("./map-blocks");

      expect(Array.from(readMapBlockBytes("NewBarkTown"))).toEqual([1, 2, 3, 4]);
    });
  });

  it("throws when the bundled map block asset is missing", () => {
    jest.doMock("./asset-reader", () => ({
      readJsonAssetSync: () => {
        throw new Error("missing");
      },
    }));
    jest.doMock("./paths", () => ({
      getAssetsRoot: () => "/tmp/assets",
      getDataDir: () => "/tmp/assets/data",
      getDisassemblyRoot: () => "/tmp/disassembly",
    }));

    jest.isolateModules(() => {
      const { readMapBlockBytes } = require("./map-blocks") as typeof import("./map-blocks");

      expect(() => readMapBlockBytes("NewBarkTown")).toThrow(
        "Bundled map block asset is required for the asset-only runtime: missing or invalid /tmp/assets/data/map_blocks.json."
      );
    });
  });
});

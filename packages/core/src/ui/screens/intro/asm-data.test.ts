describe("intro asm-data bundled runtime", () => {
  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
    jest.dontMock("@pokecrystal/core/core/asset-reader");
    jest.dontMock("@pokecrystal/core/core/paths");
  });

  it("loads bundled sprite animation data", () => {
    jest.isolateModules(() => {
      const {
        loadSpriteOamSets,
        loadFramesets,
        loadSpriteObjectDefinitions,
      } = require("./asm-data") as typeof import("./asm-data");

      const oamSets = loadSpriteOamSets();
      const framesets = loadFramesets();
      const objects = loadSpriteObjectDefinitions();

      expect(Object.keys(oamSets).length).toBeGreaterThan(0);
      expect(Object.keys(framesets).length).toBeGreaterThan(0);
      expect(Object.keys(objects).length).toBeGreaterThan(0);
    });
  });

  it("throws an explicit bundled-asset error when framesets are missing", () => {
    const dataDir = "/tmp/assets/data";
    const missingPath = `${dataDir}/sprite_anim_bundle.json`;

    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: (target: string) => {
        if (target === missingPath) {
          throw new Error(`Failed to load asset ${target} (status 404)`);
        }
        return {};
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => dataDir,
      getDisassemblyRoot: () => "/tmp/unused-disassembly",
    }));

    jest.isolateModules(() => {
      const { loadFramesets } = require("./asm-data") as typeof import("./asm-data");
      expect(() => loadFramesets()).toThrow(
        `Missing bundled sprite animation runtime file: ${missingPath}. Failed to load asset ${missingPath} (status 404)`
      );
    });
  });
});

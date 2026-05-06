describe("collision-rules", () => {
  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
  });

  it("throws a clear error when the bundled collision stdscripts asset is unavailable", () => {
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: () => {
        throw new Error("Failed to load asset /tmp/data/collision/collision_stdscripts.json (status 404)");
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => "/tmp/data",
    }));

    jest.isolateModules(() => {
      const { loadCollisionStdScripts } = require("./collision-rules") as typeof import("./collision-rules");

      expect(() => loadCollisionStdScripts()).toThrow(
        "Missing standard collision scripts asset at /tmp/data/collision/collision_stdscripts.json: Failed to load asset /tmp/data/collision/collision_stdscripts.json (status 404)"
      );
    });
  });
});

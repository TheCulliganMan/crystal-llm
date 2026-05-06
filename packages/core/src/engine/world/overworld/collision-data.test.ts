describe("collision-data", () => {
  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
  });

  it("throws a clear error when the bundled collision permissions asset is unavailable", () => {
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: () => {
        throw new Error("Failed to load asset /tmp/data/collision/collision_permissions.json (status 404)");
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => "/tmp/data",
    }));

    jest.isolateModules(() => {
      const { describeCollision } = require("./collision-data") as typeof import("./collision-data");

      expect(() => describeCollision(0)).toThrow(
        "Missing collision permissions asset at /tmp/data/collision/collision_permissions.json: Failed to load asset /tmp/data/collision/collision_permissions.json (status 404)"
      );
    });
  });
});

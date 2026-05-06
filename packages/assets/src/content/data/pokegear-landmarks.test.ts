describe("pokegear landmarks data loading", () => {
  afterEach(() => {
    jest.resetModules();
    jest.clearAllMocks();
    jest.dontMock("@pokecrystal/core/core/asset-reader");
  });

  it("loads landmarks from the canonical runtime asset", () => {
    jest.isolateModules(() => {
      jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
        readJsonAssetSync: () => ({
          landmarks: [{ id: 1, constant: "LANDMARK_HOME", label: "HOME", name: "Home", x: 1, y: 2, region: "JOHTO" }],
          map_to_landmark: { PlayersHouse1F: "LANDMARK_HOME" },
        }),
      }));
      jest.doMock("@pokecrystal/core/core/paths", () => ({
        getAssetsRoot: () => "/var/task/apps/web/assets",
        getDataDir: () => "/var/task/apps/web/assets/data",
        getDisassemblyRoot: () => "/var/task/apps/web/vendor/pokecrystal",
      }));
      const module = require("./pokegear-landmarks") as {
        POKEGEAR_LANDMARKS: Array<{ constant: string }>;
      };
      expect(module.POKEGEAR_LANDMARKS.length).toBeGreaterThan(0);
      expect(module.POKEGEAR_LANDMARKS[0]?.constant).toBeTruthy();
    });
  });
});

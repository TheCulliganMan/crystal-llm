describe("NpcPaletteManager", () => {
  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
  });

  it("throws a clear error when the bundled NPC palette asset is unavailable", () => {
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readTextAssetSync: () => {
        throw new Error("Failed to load asset /tmp/npc_sprites.pal (status 404)");
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getAssetPath: (...parts: string[]) => `/tmp/${parts.join("/")}`,
    }));
    jest.doMock("@pokecrystal/core/ui/game-engine", () => ({
      gameEngine: { Surface: class Surface {} },
    }));

    jest.isolateModules(() => {
      const { NpcPaletteManager } = require("./palette") as typeof import("./palette");

      expect(() => new NpcPaletteManager()).toThrow(
        "Missing overworld palette asset at /tmp/gfx/overworld/npc_sprites.pal: Failed to load asset /tmp/npc_sprites.pal (status 404)"
      );
    });
  });
});

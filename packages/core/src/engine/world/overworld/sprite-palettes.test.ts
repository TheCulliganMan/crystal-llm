import { resolveNpcPaletteId } from "./sprite-palettes";

describe("sprite palette defaults", () => {
  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
  });

  it("uses the sprite default when object palette is zero", () => {
    expect(resolveNpcPaletteId("SPRITE_FRUIT_TREE", 0)).toBe(6);
  });

  it("uses the object palette when provided", () => {
    expect(resolveNpcPaletteId("SPRITE_FRUIT_TREE", 9)).toBe(9);
  });

  it("throws a clear error when bundled sprite palette defaults are unavailable", () => {
    jest.resetModules();
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: () => {
        throw new Error("Failed to load asset /tmp/data/sprite_palette_defaults.json (status 404)");
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => "/tmp/data",
    }));
    jest.doMock("path", () => ({
      __esModule: true,
      default: { join: (...parts: string[]) => parts.join("/") },
      join: (...parts: string[]) => parts.join("/"),
    }));

    jest.isolateModules(() => {
      const { resolveNpcPaletteId: resolveWithMissingAsset } = require("./sprite-palettes") as typeof import("./sprite-palettes");

      expect(() => resolveWithMissingAsset("SPRITE_FRUIT_TREE", 0)).toThrow(
        "Missing bundled sprite palette defaults at /tmp/data/sprite_palette_defaults.json: Failed to load asset /tmp/data/sprite_palette_defaults.json (status 404)"
      );
    });
  });
});

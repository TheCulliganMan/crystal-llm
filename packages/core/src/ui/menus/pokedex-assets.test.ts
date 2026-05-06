const loadPokedexAssetsWithHeadlessCanvas = () => {
  const globalAny = globalThis as Record<string, unknown>;
  const originalOffscreen = globalAny.OffscreenCanvas;
  delete globalAny.OffscreenCanvas;

  let assets: typeof import("./pokedex-assets");
  try {
    jest.isolateModules(() => {
      assets = require("./pokedex-assets");
    });
  } finally {
    if (originalOffscreen === undefined) {
      delete globalAny.OffscreenCanvas;
    } else {
      globalAny.OffscreenCanvas = originalOffscreen;
    }
  }

  return assets!;
};

describe("Pokedex tiles", () => {
  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
    jest.unmock("fs");
    jest.unmock("@pokecrystal/core/core/lz");
    jest.unmock("@pokecrystal/core/core/paths");
  });

  it("keeps background tiles opaque", () => {
    const { ensurePokedexTiles, requirePokedexTile, resetPokedexHardwareState } =
      loadPokedexAssetsWithHeadlessCanvas();
    const ui = {
      font: {
        fontTiles: {},
        reloadFontExtraTiles: jest.fn(),
      },
    };

    resetPokedexHardwareState();
    ensurePokedexTiles(ui);

    const backgroundTile = requirePokedexTile(ui, 0x32);
    const spaceTile = requirePokedexTile(ui, 0x7f);

    expect(backgroundTile.get_at([0, 0])[3]).toBeGreaterThan(0);
    expect(spaceTile.get_at([0, 0])[3]).toBeGreaterThan(0);
  });

  it("throws for a missing compressed tileset instead of stripping .lz and loading an uncompressed sibling", () => {
    const compressedPath = "/tmp/assets/gfx/pokedex/pokedex.2bpp.lz";
    const uncompressedPath = "/tmp/assets/gfx/pokedex/pokedex.2bpp";
    const existsSync = jest.fn((target: string) => target === uncompressedPath);
    const readFileSync = jest.fn();

    jest.doMock("fs", () => ({
      __esModule: true,
      default: { existsSync, readFileSync },
      existsSync,
      readFileSync,
    }));

    jest.isolateModules(() => {
      const { loadPokedexTilesFromFile } =
        require("./pokedex-assets") as typeof import("./pokedex-assets");

      expect(() => loadPokedexTilesFromFile(compressedPath)).toThrow(
        `Missing Pokédex tileset: ${compressedPath}`
      );
      expect(readFileSync).not.toHaveBeenCalled();
    });
  });

  it("decompresses present .lz Pokédex tilesets instead of requiring a decompressed mirror file", () => {
    const compressedPath = "/tmp/assets/gfx/pokedex/pokedex.2bpp.lz";
    const compressedBytes = Uint8Array.from([1, 2, 3, 4]);
    const decompressedBytes = Uint8Array.from(new Array(16).fill(0));
    const existsSync = jest.fn((target: string) => target === compressedPath);
    const readFileSync = jest.fn(() => compressedBytes);
    const decompress = jest.fn(() => decompressedBytes);

    jest.doMock("fs", () => ({
      __esModule: true,
      default: { existsSync, readFileSync },
      existsSync,
      readFileSync,
    }));
    jest.doMock("@pokecrystal/core/core/lz", () => ({
      decompress,
    }));

    jest.isolateModules(() => {
      const { loadPokedexTilesFromFile } =
        require("./pokedex-assets") as typeof import("./pokedex-assets");

      const tiles = loadPokedexTilesFromFile(compressedPath);

      expect(decompress).toHaveBeenCalledWith(compressedBytes);
      expect(tiles).toHaveLength(1);
    });
  });
});

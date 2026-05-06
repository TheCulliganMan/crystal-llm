import fs from "fs";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { BitmapFont } from "./bitmap-font";

const buildSurfaceForPath = (pathName: string) => {
  if (pathName.includes("/gfx/frames/")) {
    return new gameEngine.Surface(24, 16);
  }
  if (pathName.endsWith("/gfx/font/space.png")) {
    return new gameEngine.Surface(8, 8);
  }
  return new gameEngine.Surface(128, 48);
};

describe("BitmapFont async asset loading", () => {
  const buildBattleExtra = () => Buffer.alloc(16 * 20);
  const buildFontExtra = () => Buffer.alloc(16 * 25);
  const buildSingleTile = () => Buffer.alloc(16);
  const encode2bppTiles = (tiles: number[][]): Buffer => {
    const bytes: number[] = [];
    for (const levels of tiles) {
      for (let y = 0; y < 8; y += 1) {
        let lo = 0;
        let hi = 0;
        for (let x = 0; x < 8; x += 1) {
          const level = levels[y * 8 + x] ?? 0;
          const mask = 1 << (7 - x);
          if (level & 1) {
            lo |= mask;
          }
          if (level & 2) {
            hi |= mask;
          }
        }
        bytes.push(lo, hi);
      }
    }
    return Buffer.from(bytes);
  };
  const buildFontExtraWithEllipsis = () => {
    const tiles = Array.from({ length: 25 }, () => Array(64).fill(0));
    const ellipsisSourceTile = tiles[21];
    for (const x of [0, 3, 6]) {
      ellipsisSourceTile[6 * 8 + x] = 3;
      ellipsisSourceTile[7 * 8 + x] = 3;
    }
    return encode2bppTiles(tiles);
  };

  let originalLoad: typeof gameEngine.image.load;
  let originalLoadSync: typeof gameEngine.image.loadSync | undefined;

  beforeEach(() => {
    originalLoad = gameEngine.image.load;
    originalLoadSync = gameEngine.image.loadSync;
  });

  afterEach(() => {
    gameEngine.image.load = originalLoad;
    if (originalLoadSync) {
      gameEngine.image.loadSync = originalLoadSync;
    } else {
      delete gameEngine.image.loadSync;
    }
    jest.restoreAllMocks();
  });

  it("loads font extras via async reads without sync filesystem access", async () => {
    gameEngine.image.load = jest.fn(async (pathName: string) => buildSurfaceForPath(pathName));

    const existsSpy = jest
      .spyOn(fs, "existsSync")
      .mockImplementation((filePath) => {
        const name = String(filePath);
        return (
          name.endsWith("font_extra.2bpp") ||
          name.endsWith("font_battle_extra.2bpp") ||
          name.endsWith("up_arrow.2bpp") ||
          name.endsWith("phone_icon.2bpp")
        );
      });

    const readFileSpy = jest
      .spyOn(fs.promises, "readFile")
      .mockImplementation(async (filePath) => {
        const name = String(filePath);
        if (name.endsWith("font_extra.2bpp")) {
          return buildFontExtra();
        }
        if (name.endsWith("font_battle_extra.2bpp")) {
          return buildBattleExtra();
        }
        if (name.endsWith("up_arrow.2bpp")) {
          return buildSingleTile();
        }
        if (name.endsWith("phone_icon.2bpp")) {
          return buildSingleTile();
        }
        throw new Error(`Unexpected readFile call for ${name}`);
      });

    const readFileSyncSpy = jest
      .spyOn(fs, "readFileSync")
      .mockImplementation(() => {
        throw new Error("readFileSync should not be called during async font load");
      });

    const font = new BitmapFont();
    await font.load();

    expect(readFileSyncSpy).not.toHaveBeenCalled();
    expect(readFileSpy).toHaveBeenCalled();
    expect(existsSpy).toHaveBeenCalled();
    expect(font.fontTiles[0x60]).toBeDefined();
    expect(font.fontTiles[0x61]).toBeDefined();
    expect(font.fontTiles[0x63]).toBeDefined();
    expect(font.fontTiles[0x6e]).toBeDefined();
  });

  it("loads ellipsis font-extra tile with a transparent background", async () => {
    gameEngine.image.load = jest.fn(async (pathName: string) => buildSurfaceForPath(pathName));

    jest.spyOn(fs, "existsSync").mockImplementation((filePath) => {
      const name = String(filePath);
      return name.endsWith("font_extra.2bpp");
    });
    jest.spyOn(fs.promises, "readFile").mockImplementation(async (filePath) => {
      const name = String(filePath);
      if (name.endsWith("font_extra.2bpp")) {
        return buildFontExtraWithEllipsis();
      }
      throw new Error(`Unexpected readFile call for ${name}`);
    });

    const font = new BitmapFont();
    await font.load();

    const ellipsisTile = font.fontTiles[0x75];
    expect(ellipsisTile).toBeDefined();
    expect(ellipsisTile.get_at([0, 0])[3]).toBe(0);
    expect(ellipsisTile.get_at([0, 6])).toEqual([0, 0, 0, 255]);
    expect(ellipsisTile.get_at([3, 6])).toEqual([0, 0, 0, 255]);
    expect(ellipsisTile.get_at([6, 6])).toEqual([0, 0, 0, 255]);
  });

  it("throws when font.png is missing instead of substituting a blank surface", async () => {
    const warn = jest.spyOn(console, "warn").mockImplementation(() => {});
    gameEngine.image.load = jest.fn(async (pathName: string) => {
      if (pathName.endsWith("/gfx/font/font.png")) {
        throw new Error("missing font");
      }
      return buildSurfaceForPath(pathName);
    });

    const font = new BitmapFont();

    await expect(font.load()).rejects.toThrow(/\/gfx\/font\/font\.png$/);
    expect(warn).not.toHaveBeenCalled();
  });

  it("throws when space.png is missing instead of substituting a blank tile", async () => {
    const warn = jest.spyOn(console, "warn").mockImplementation(() => {});
    gameEngine.image.load = jest.fn(async (pathName: string) => {
      if (pathName.endsWith("/gfx/font/space.png")) {
        throw new Error("missing space");
      }
      return buildSurfaceForPath(pathName);
    });

    const font = new BitmapFont();

    await expect(font.load()).rejects.toThrow(/\/gfx\/font\/space\.png$/);
    expect(warn).not.toHaveBeenCalled();
  });

  it("reuses tinted glyph surfaces across repeated draws of the same text color", async () => {
    gameEngine.image.load = jest.fn(async (pathName: string) => buildSurfaceForPath(pathName));
    jest.spyOn(fs, "existsSync").mockReturnValue(false);

    const font = new BitmapFont();
    await font.load();

    const target = new gameEngine.Surface(160, 144);
    const setAtSpy = jest.spyOn(gameEngine.Surface.prototype, "set_at");

    font.renderText("ABAB", 0, 0, target, { color: [0, 0, 0] });
    const firstRenderWrites = setAtSpy.mock.calls.length;

    setAtSpy.mockClear();
    font.renderText("ABAB", 0, 0, target, { color: [0, 0, 0] });

    expect(firstRenderWrites).toBeGreaterThan(0);
    expect(setAtSpy).not.toHaveBeenCalled();
  });
});

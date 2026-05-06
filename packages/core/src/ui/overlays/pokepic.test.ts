import fs from "fs";
import { PokePicOverlay, type UI } from "./pokepic";
import { Surface } from "@pokecrystal/core/ui/surface";

const TILE_SIZE = 8;
const SPRITE_X = 7 * TILE_SIZE;
const SPRITE_Y = 5 * TILE_SIZE;

const encodeTile = (pixels: number[][]): Buffer => {
  const data = Buffer.alloc(16);
  for (let y = 0; y < TILE_SIZE; y += 1) {
    let lo = 0;
    let hi = 0;
    for (let x = 0; x < TILE_SIZE; x += 1) {
      const value = pixels[y]?.[x] ?? 0;
      lo |= (value & 1) << (7 - x);
      hi |= ((value >> 1) & 1) << (7 - x);
    }
    data[y * 2] = lo;
    data[y * 2 + 1] = hi;
  }
  return data;
};

const buildFrontpic2bpp = (
  dimensionTiles: number,
  paint: (tiles: number[][][]) => void
): Buffer => {
  const tiles = Array.from({ length: dimensionTiles * dimensionTiles }, () =>
    Array.from({ length: TILE_SIZE }, () => Array.from({ length: TILE_SIZE }, () => 0))
  );
  paint(tiles);
  return Buffer.concat(tiles.map(encodeTile));
};

const createUi = (): UI & {
  loadSprite: jest.Mock;
  _getPokemonFrameSurface: jest.Mock;
} => {
  const coloredFallback = new Surface(7 * TILE_SIZE, 7 * TILE_SIZE);
  coloredFallback.fill([0, 0, 0, 0]);
  coloredFallback.set_at([16, 16], [255, 0, 0, 255]);
  return {
    tileSize: TILE_SIZE,
    loadSprite: jest.fn(),
    drawWindow: (surface, x, y, widthTiles, heightTiles, options) => {
      const fill = options?.fill ?? [255, 255, 255];
      for (let py = y; py < y + heightTiles * TILE_SIZE; py += 1) {
        for (let px = x; px < x + widthTiles * TILE_SIZE; px += 1) {
          surface.set_at([px, py], [fill[0], fill[1], fill[2], 255]);
        }
      }
    },
    _getPokemonFrameSurface: jest.fn(() => coloredFallback),
  };
};

const mockFrontpicAsset = (data: Buffer): jest.SpyInstance[] => [
  jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
    const path = String(pathLike);
    return path.endsWith("/gfx/pokemon/testmon/front.2bpp");
  }),
  jest.spyOn(fs, "readFileSync").mockImplementation((pathLike) => {
    const path = String(pathLike);
    if (path.endsWith("/gfx/pokemon/testmon/front.2bpp")) {
      return data;
    }
    throw new Error(`Unexpected readFileSync path: ${path}`);
  }),
];

describe("PokePicOverlay", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("renders pokepic front sprites from grayscale 2bpp instead of the full-color fallback", () => {
    const data = buildFrontpic2bpp(5, (tiles) => {
      const centerTile = 1 * 5 + 1;
      tiles[centerTile][1][1] = 1;
      tiles[centerTile][1][2] = 2;
      tiles[centerTile][1][3] = 3;
    });
    mockFrontpicAsset(data);

    const ui = createUi();
    const overlay = new PokePicOverlay(ui);
    const screen = new Surface(160, 144);

    overlay.show("TESTMON");
    overlay.draw(screen);

    expect(ui._getPokemonFrameSurface).not.toHaveBeenCalled();
    expect(screen.get_at([SPRITE_X + 17, SPRITE_Y + 17])).toEqual([170, 170, 170, 255]);
    expect(screen.get_at([SPRITE_X + 18, SPRITE_Y + 17])).toEqual([85, 85, 85, 255]);
    expect(screen.get_at([SPRITE_X + 19, SPRITE_Y + 17])).toEqual([0, 0, 0, 255]);
    expect(screen.get_at([SPRITE_X + 16, SPRITE_Y + 17]).slice(0, 3)).not.toEqual([255, 0, 0]);
  });

  it("keeps border-connected color-0 pixels transparent in the grayscale surface", () => {
    const data = buildFrontpic2bpp(5, (tiles) => {
      tiles[6][1][1] = 3;
    });
    mockFrontpicAsset(data);

    const ui = createUi();
    const overlay = new PokePicOverlay(ui);
    const surface = (overlay as unknown as {
      _loadGrayscaleFrontpic(speciesName: string, tileSize: number): Surface | null;
    })._loadGrayscaleFrontpic("TESTMON", TILE_SIZE);

    expect(surface).not.toBeNull();
    expect(surface!.get_at([0, 0])[3]).toBe(0);
    expect(surface!.get_at([17, 17])).toEqual([0, 0, 0, 255]);
  });

  it("pads inferred 5x5 frontpics with one blank row and one blank column", () => {
    const data = buildFrontpic2bpp(5, (tiles) => {
      tiles[0][1][1] = 3;
    });
    mockFrontpicAsset(data);

    const ui = createUi();
    const overlay = new PokePicOverlay(ui);
    const surface = (overlay as unknown as {
      _loadGrayscaleFrontpic(speciesName: string, tileSize: number): Surface | null;
    })._loadGrayscaleFrontpic("TESTMON", TILE_SIZE);

    expect(surface).not.toBeNull();
    expect(surface!.get_at([8, 8])[3]).toBe(0);
    expect(surface!.get_at([9, 9])).toEqual([0, 0, 0, 255]);
  });

  it("pads inferred 6x6 frontpics with one blank column and no blank row", () => {
    const data = buildFrontpic2bpp(6, (tiles) => {
      tiles[0][1][1] = 3;
    });
    mockFrontpicAsset(data);

    const ui = createUi();
    const overlay = new PokePicOverlay(ui);
    const surface = (overlay as unknown as {
      _loadGrayscaleFrontpic(speciesName: string, tileSize: number): Surface | null;
    })._loadGrayscaleFrontpic("TESTMON", TILE_SIZE);

    expect(surface).not.toBeNull();
    expect(surface!.get_at([8, 0])[3]).toBe(0);
    expect(surface!.get_at([9, 1])).toEqual([0, 0, 0, 255]);
  });
});

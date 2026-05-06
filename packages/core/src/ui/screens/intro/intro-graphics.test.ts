import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { IntroGraphics } from "./intro-graphics";
import { getTileIndexMode, getTileShift } from "./tilemap-defaults";

type IntroGraphicsHarness = IntroGraphics & {
  tileSize: number;
  tiles: Record<string, InstanceType<typeof gameEngine.Surface>[]>;
  palettes: Record<string, [number, number, number][][]>;
  objPalettes: Record<string, [number, number, number][][]>;
  paletteOverrides: Record<string, Record<number, [number, number, number][]>>;
  objPaletteOverrides: Record<string, Record<number, [number, number, number][]>>;
  tileCache: Map<string, InstanceType<typeof gameEngine.Surface>>;
  paletteVersions: Map<string, number>;
  objPaletteVersions: Map<string, number>;
  paletteNameCache: Map<string, string | null>;
};

const makeGraphicsHarness = (): IntroGraphicsHarness => {
  const graphics = Object.create(IntroGraphics.prototype) as IntroGraphicsHarness;
  const tile = new gameEngine.Surface(8, 8);
  tile.fill([255, 255, 255, 255]);
  tile.set_at([0, 0], [170, 170, 170, 255]);
  tile.set_at([1, 0], [85, 85, 85, 255]);
  tile.set_at([2, 0], [0, 0, 0, 255]);

  graphics.tileSize = 8;
  graphics.tiles = { test: [tile] };
  graphics.palettes = {
    test: [
      [
        [10, 20, 30],
        [40, 50, 60],
        [70, 80, 90],
        [100, 110, 120],
      ],
    ],
  };
  graphics.objPalettes = {
    test: [
      [
        [1, 2, 3],
        [4, 5, 6],
        [7, 8, 9],
        [10, 11, 12],
      ],
    ],
  };
  graphics.paletteOverrides = {};
  graphics.objPaletteOverrides = {};
  graphics.tileCache = new Map();
  graphics.paletteVersions = new Map();
  graphics.objPaletteVersions = new Map();
  graphics.paletteNameCache = new Map();
  return graphics;
};

describe("IntroGraphics tile caching", () => {
  it("reuses recolored tiles for repeated lookups", () => {
    const graphics = makeGraphicsHarness();
    const first = graphics.getTile("test", 0, 0, null, false, 0);
    const second = graphics.getTile("test", 0, 0, null, false, 0);

    expect(first).not.toBeNull();
    expect(second).toBe(first);
  });

  it("invalidates cache when palette overrides change", () => {
    const graphics = makeGraphicsHarness();
    const first = graphics.getTile("test", 0, 0, null, false, 0);
    graphics.setPaletteOverride("test", 0, [
      [200, 201, 202],
      [40, 50, 60],
      [70, 80, 90],
      [100, 110, 120],
    ]);
    const second = graphics.getTile("test", 0, 0, null, false, 0);

    expect(first).not.toBeNull();
    expect(second).not.toBeNull();
    expect(second).not.toBe(first);
    expect(second?.get_at([3, 0]).slice(0, 3)).toEqual([200, 201, 202]);
  });

  it("keeps unrelated palette cache entries warm when one palette changes", () => {
    const graphics = makeGraphicsHarness();
    const otherTile = new gameEngine.Surface(8, 8);
    otherTile.fill([255, 255, 255, 255]);
    graphics.tiles.other = [otherTile];
    graphics.palettes.other = [
      [
        [1, 1, 1],
        [2, 2, 2],
        [3, 3, 3],
        [4, 4, 4],
      ],
    ];

    const testFirst = graphics.getTile("test", 0, 0, null, false, 0);
    const otherFirst = graphics.getTile("other", 0, 0, null, false, 0);
    graphics.setPaletteOverride("test", 0, [
      [200, 201, 202],
      [40, 50, 60],
      [70, 80, 90],
      [100, 110, 120],
    ]);

    expect(graphics.getTile("test", 0, 0, null, false, 0)).not.toBe(testFirst);
    expect(graphics.getTile("other", 0, 0, null, false, 0)).toBe(otherFirst);
  });

  it("caches flipped variants independently", () => {
    const graphics = makeGraphicsHarness();
    const plain = graphics.getTile("test", 0, 0, 0x00, false, 0);
    const xflippedA = graphics.getTile("test", 0, 0, 0x20, false, 0);
    const xflippedB = graphics.getTile("test", 0, 0, 0x20, false, 0);

    expect(plain).not.toBeNull();
    expect(xflippedA).not.toBeNull();
    expect(xflippedB).toBe(xflippedA);
    expect(xflippedA).not.toBe(plain);
  });

  it("keeps intro pichu/wooper OBJ palettes sourced from intro background palettes", () => {
    const graphics = new IntroGraphics() as IntroGraphicsHarness;
    const backgroundObjPalette1 = graphics.objPalettes.background?.[1];
    const backgroundObjPalette2 = graphics.objPalettes.background?.[2];
    const pichuWooperPalette1 = graphics.objPalettes.pichu_wooper?.[1];
    const pichuWooperPalette2 = graphics.objPalettes.pichu_wooper?.[2];

    expect(backgroundObjPalette1).toBeDefined();
    expect(backgroundObjPalette2).toBeDefined();
    expect(pichuWooperPalette1).toEqual(backgroundObjPalette1);
    expect(pichuWooperPalette2).toEqual(backgroundObjPalette2);
  });

  it("subtracts the 0x80 tile shift for high intro tile ids in offset mode", () => {
    const graphics = makeGraphicsHarness();
    const tiles = Array.from({ length: 256 }, (_, index) => {
      const tile = new gameEngine.Surface(8, 8);
      tile.fill([index, 0, 0, 255]);
      return tile;
    });
    graphics.tiles = { signed: tiles };

    const zeroTile = graphics.getTile("signed", 0x00, 0, null, false, 0x80);
    const sevenfTile = graphics.getTile("signed", 0x7f, 0, null, false, 0x80);
    const eightyTile = graphics.getTile("signed", 0x80, 0, null, false, 0x80);
    const ffTile = graphics.getTile("signed", 0xff, 0, null, false, 0x80);

    expect(zeroTile?.get_at([0, 0])[0]).toBe(0);
    expect(sevenfTile?.get_at([0, 0])[0]).toBe(127);
    expect(eightyTile?.get_at([0, 0])[0]).toBe(0);
    expect(ffTile?.get_at([0, 0])[0]).toBe(127);
  });

  it("supports signed tile addressing for wide intro atlases like suicune_close", () => {
    const graphics = makeGraphicsHarness();
    const tiles = Array.from({ length: 256 }, (_, index) => {
      const tile = new gameEngine.Surface(8, 8);
      tile.fill([index, 0, 0, 255]);
      return tile;
    });
    graphics.tiles = { signed: tiles };

    const zeroTile = graphics.getTile("signed", 0x00, 0, null, false, 0x80, undefined, "signed");
    const sevenfTile = graphics.getTile("signed", 0x7f, 0, null, false, 0x80, undefined, "signed");
    const eightyTile = graphics.getTile("signed", 0x80, 0, null, false, 0x80, undefined, "signed");
    const ffTile = graphics.getTile("signed", 0xff, 0, null, false, 0x80, undefined, "signed");

    expect(zeroTile?.get_at([0, 0])[0]).toBe(128);
    expect(sevenfTile?.get_at([0, 0])[0]).toBe(255);
    expect(eightyTile?.get_at([0, 0])[0]).toBe(0);
    expect(ffTile?.get_at([0, 0])[0]).toBe(127);
  });

  it("keeps offset addressing for intro unown and crystal word tilemaps, but uses signed mode for suicune_close", () => {
    expect(getTileShift("unown_a")).toBe(0x80);
    expect(getTileShift("unown_hi")).toBe(0x80);
    expect(getTileShift("unowns")).toBe(0x80);
    expect(getTileShift("crystal_unowns")).toBe(0x80);
    expect(getTileIndexMode("unowns")).toBe("offset");
    expect(getTileIndexMode("crystal_unowns")).toBe("offset");
    expect(getTileIndexMode("suicune_close")).toBe("signed");
  });

  it("can render reused unown tiles with the suicune scene palette", () => {
    const graphics = makeGraphicsHarness();
    const tile = new gameEngine.Surface(8, 8);
    tile.fill([255, 255, 255, 255]);
    graphics.tiles = { unowns: [tile] };
    graphics.palettes.unowns = [
      [
        [0, 0, 0],
        [10, 0, 10],
        [19, 0, 19],
        [31, 0, 31],
      ],
    ];
    graphics.palettes.suicune = [
      [
        [24, 12, 9],
        [24, 12, 9],
        [24, 12, 9],
        [24, 12, 9],
      ],
    ];

    const recolored = graphics.getTile("unowns", 0, 0, 0x00, false, 0x80, "suicune");

    expect(recolored).not.toBeNull();
    expect(recolored?.get_at([0, 0]).slice(0, 3)).toEqual([24, 12, 9]);
  });

  it("keeps cache entries separate when the same tile is recolored through different palette overrides", () => {
    const graphics = makeGraphicsHarness();
    const tile = new gameEngine.Surface(8, 8);
    tile.fill([255, 255, 255, 255]);
    graphics.tiles = { unowns: [tile] };
    graphics.palettes.unowns = [
      [
        [0, 0, 0],
        [10, 0, 10],
        [19, 0, 19],
        [31, 0, 31],
      ],
    ];
    graphics.palettes.suicune = [
      [
        [24, 12, 9],
        [24, 12, 9],
        [24, 12, 9],
        [24, 12, 9],
      ],
    ];

    const first = graphics.getTile("unowns", 0, 0, 0x00, false, 0x80, "unowns");
    const second = graphics.getTile("unowns", 0, 0, 0x00, false, 0x80, "suicune");

    expect(first).not.toBeNull();
    expect(second).not.toBeNull();
    expect(second).not.toBe(first);
    expect(first?.get_at([0, 0]).slice(0, 3)).toEqual([0, 0, 0]);
    expect(second?.get_at([0, 0]).slice(0, 3)).toEqual([24, 12, 9]);
  });

  it("keeps crystal_unowns low tiles visible", () => {
    const graphics = new IntroGraphics();
    const visibleTile = graphics.getTile("crystal_unowns", 0x01, 7, 0x0f, false, 0);

    expect(visibleTile).not.toBeNull();

    let visibleOpaquePixels = 0;
    for (let y = 0; y < 8; y += 1) {
      for (let x = 0; x < 8; x += 1) {
        if ((visibleTile?.get_at([x, y])[3] ?? 0) > 0) {
          visibleOpaquePixels += 1;
        }
      }
    }

    expect(visibleOpaquePixels).toBeGreaterThan(0);
  });

  it("keeps crystal_unowns high tile ids visible after the 0x80 shift", () => {
    const graphics = new IntroGraphics();
    const repeatedTile = graphics.getTile("crystal_unowns", 0x81, 7, 0x07, false, 0x80);

    expect(repeatedTile).not.toBeNull();

    let repeatedOpaquePixels = 0;
    for (let y = 0; y < 8; y += 1) {
      for (let x = 0; x < 8; x += 1) {
        if ((repeatedTile?.get_at([x, y])[3] ?? 0) > 0) {
          repeatedOpaquePixels += 1;
        }
      }
    }

    expect(repeatedOpaquePixels).toBeGreaterThan(0);
  });
});

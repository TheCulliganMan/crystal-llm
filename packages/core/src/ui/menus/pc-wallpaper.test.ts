const buildPngSurface = (
  engine: typeof import("@pokecrystal/core/ui/game-engine").gameEngine,
  width: number,
  height: number,
  level: number,
): InstanceType<typeof engine.Surface> => {
  const surface = new engine.Surface(width, height);
  const shade = Math.max(0, Math.min(3, level));
  const value = shade * 85;
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      surface.setAt(x, y, [value, value, value, 255]);
    }
  }
  return surface;
};

const buildGlyphTile = async (
  level: number,
): Promise<InstanceType<(typeof import("@pokecrystal/core/ui/game-engine").gameEngine)["Surface"]>> => {
  const { gameEngine } = await import("@pokecrystal/core/ui/game-engine");
  return buildPngSurface(gameEngine, 8, 8, level);
};

type WindowStub = Record<string, unknown>;

const testGlobal = globalThis as unknown as { window?: Window & WindowStub };
describe("pc-wallpaper browser fallback", () => {
  const originalWindow = testGlobal.window;
  let originalLoadSync: typeof import("@pokecrystal/core/ui/game-engine").gameEngine.image.loadSync | null = null;

  beforeEach(() => {
    testGlobal.window = {} as Window & WindowStub;
    jest.resetModules();
  });

  afterEach(async () => {
    if (originalLoadSync) {
      const { gameEngine } = await import("@pokecrystal/core/ui/game-engine");
      gameEngine.image.loadSync = originalLoadSync;
      originalLoadSync = null;
    }
    testGlobal.window = originalWindow;
  });

  it("builds the tileset from cached PNGs in the browser", async () => {
    const { gameEngine } = await import("@pokecrystal/core/ui/game-engine");
    originalLoadSync = gameEngine.image.loadSync;
    const pcSurface = buildPngSurface(gameEngine, 16, 32, 1);
    const mailSurface = buildPngSurface(gameEngine, 16, 16, 2);
    gameEngine.image.loadSync = (pathName: string) => {
      if (pathName.endsWith("pc.png")) {
        return pcSurface;
      }
      if (pathName.endsWith("pc_mail.png")) {
        return mailSurface;
      }
      return null;
    };

    const { TilemapSurface } = await import("@pokecrystal/core/ui/tilemap-surface");
    const { seedPcTilemap, pcTileset, createPcTilemap, PC_MAIL_TILE_ID, PC_ITEM_TILE_ID } = await import("./pc-wallpaper");

    const tilemap = new TilemapSurface();
    expect(() => seedPcTilemap(tilemap)).not.toThrow();
    const seeded = createPcTilemap();
    expect(seeded.tilemap.attributes[4][1]).toBe(0x00);
    expect(seeded.tilemap.attributes[0][0]).toBe(0x00);
    expect(seeded.iconIds.mail).toBe(PC_MAIL_TILE_ID);
    expect(seeded.iconIds.item).toBe(PC_ITEM_TILE_ID);
    expect(seeded.tilemap.getTile(18, 1)).not.toBe(PC_MAIL_TILE_ID);
    expect(seeded.tilemap.getTile(19, 1)).not.toBe(PC_ITEM_TILE_ID);
    const glyphTiles = {
      0x79: { 0: await buildGlyphTile(0) },
      0x7a: { 0: await buildGlyphTile(1) },
      0x7b: { 0: await buildGlyphTile(2) },
      0x7c: { 0: await buildGlyphTile(3) },
      0x7d: { 0: await buildGlyphTile(0) },
      0x7e: { 0: await buildGlyphTile(1) },
      0x7f: { 0: await buildGlyphTile(0) },
      0xed: { 0: await buildGlyphTile(2) },
    };
    const font = {
      paletteVariants: () => glyphTiles,
    };
    const tileset = pcTileset(font);
    expect(Object.keys(tileset).length).toBeGreaterThan(0);
  });

  it("throws when the PC tileset contract is missing required glyph tiles", async () => {
    const { gameEngine } = await import("@pokecrystal/core/ui/game-engine");
    originalLoadSync = gameEngine.image.loadSync;
    const pcSurface = buildPngSurface(gameEngine, 16, 32, 1);
    const mailSurface = buildPngSurface(gameEngine, 16, 16, 2);
    gameEngine.image.loadSync = (pathName: string) => {
      if (pathName.endsWith("pc.png")) {
        return pcSurface;
      }
      if (pathName.endsWith("pc_mail.png")) {
        return mailSurface;
      }
      return null;
    };

    const { pcTileset } = await import("./pc-wallpaper");

    expect(() => pcTileset({})).toThrow("PC tileset is missing required glyph tiles:");
  });
});

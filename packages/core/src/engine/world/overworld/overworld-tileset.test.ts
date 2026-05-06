import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import fs from "fs";
import path from "path";
import { METATILE_WIDTH, TILE_SIZE } from "@pokecrystal/core/engine/world/tile/constants";
import { resolveCollisionValue } from "./collision-data";
import { getAssetsRoot, getDisassemblyRoot, getTilesetMetatilesPath } from "@pokecrystal/core/core/paths";
import {
  buildMetatilesFromLayout,
  getTilesetMetatilesCandidatePaths,
  parsePaletteFile,
  parseTilesetPaletteMap,
  OverworldTileset,
  resolveTilesetTile,
} from "./overworld-tileset";

const surfaceTiles = (count: number): Array<InstanceType<typeof gameEngine.Surface>> =>
  Array.from({ length: count }, () => new gameEngine.Surface(TILE_SIZE, TILE_SIZE));

describe("OverworldTileset", () => {
  it("builds metatile layouts from raw bytes", () => {
    const bytes = new Uint8Array(METATILE_WIDTH * METATILE_WIDTH);
    bytes.forEach((_, index) => {
      bytes[index] = index;
    });
    const metatiles = buildMetatilesFromLayout(bytes);
    expect(metatiles).toHaveLength(1);
    expect(metatiles[0].tiles[0][0].tileIndex).toBe(0);
    expect(metatiles[0].tiles[0][1].tileIndex).toBe(1);
    expect(metatiles[0].tiles[1][0].tileIndex).toBe(METATILE_WIDTH);
  });

  it("renders metatile tiles using tile indices", () => {
    const tileset = new OverworldTileset("test", null, { skipLoad: true });
    const bytes = new Uint8Array(METATILE_WIDTH * METATILE_WIDTH);
    bytes.forEach((_, index) => {
      bytes[index] = index;
    });
    tileset.metatiles = buildMetatilesFromLayout(bytes);
    tileset.tiles = Array.from({ length: METATILE_WIDTH * METATILE_WIDTH }, () => new gameEngine.Surface(TILE_SIZE, TILE_SIZE));

    const target = new gameEngine.Surface(TILE_SIZE * METATILE_WIDTH, TILE_SIZE * METATILE_WIDTH);
    const blits: Array<[unknown, [number, number]]> = [];
    target.blit = ((surface: unknown, dest: [number, number]) => {
      blits.push([surface, dest]);
    }) as typeof target.blit;

    tileset.renderMetatile(0, target, 0, 0);
    expect(blits).toHaveLength(METATILE_WIDTH * METATILE_WIDTH);
    expect(blits[0][1]).toEqual([0, 0]);
    expect(blits[1][1]).toEqual([TILE_SIZE, 0]);
  });

  it("parses palette map tokens into palette indices", () => {
    const content = `
      tilepal 0, GRAY, RED, GREEN, WATER, YELLOW, BROWN, ROOF, TEXT
      rept 2
        db $ff
      endr
    `;
    const indices = parseTilesetPaletteMap(content);
    expect(indices.slice(0, 8)).toEqual([0, 1, 2, 3, 4, 5, 6, 7]);
    expect(indices.slice(8, 10)).toEqual([0x0f, 0x0f]);
  });

  it("resolves bank-1 high tile IDs through ASM bit-7 clearing into the second tileset half", () => {
    const tiles = surfaceTiles(192);
    const bankOneTile = tiles[0x61];
    const mirroredTile = tiles[0x01];
    const highTile = tiles[0x81];
    expect(bankOneTile).toBeTruthy();
    expect(mirroredTile).toBeTruthy();
    expect(highTile).toBeTruthy();

    const resolved = resolveTilesetTile(tiles, 0x81, 1);
    expect(resolved).toBe(bankOneTile);
    expect(resolved).not.toBe(mirroredTile);
    expect(resolved).not.toBe(highTile);
  });

  it("resolves bank-1 high tile IDs to mirrored packed indices when high tiles are absent", () => {
    const tiles = surfaceTiles(0x41);
    const mirroredTile = tiles[0x01];
    expect(tiles[0x81]).toBeUndefined();
    expect(mirroredTile).toBeTruthy();

    const resolved = resolveTilesetTile(tiles, 0x81, 1);
    expect(resolved).toBe(mirroredTile);
  });

  it("resolves bank-1 low tile IDs from the exported bank-1 tile range", () => {
    const tiles = surfaceTiles(0x180);
    const baseTile = tiles[0x10];
    const bankOneTile = tiles[0x10 + 0xc0];

    const resolved = resolveTilesetTile(tiles, 0x10, 1);
    expect(resolved).toBe(bankOneTile);
    expect(resolved).not.toBe(baseTile);
  });

  it("maps Elm's Lab PC desk metatile through the ASM bank-1 export layout", () => {
    const labDeskMetatile = new Uint8Array([
      0x10, 0x10, 0x85, 0x86,
      0x81, 0x82, 0x83, 0x84,
      0x91, 0x92, 0x93, 0x94,
      0xa1, 0xa2, 0xa3, 0xa4,
    ]);
    const metatile = buildMetatilesFromLayout(labDeskMetatile)[0];
    const tiles = surfaceTiles(192);
    const highTileReferences = metatile.tiles.flat().filter((entry) => entry.tileIndex >= 0x80);
    const expectedSourceTileIndices = [0x65, 0x66, 0x61, 0x62, 0x63, 0x64, 0x71, 0x72, 0x73, 0x74, 0x81, 0x82, 0x83, 0x84];

    expect(highTileReferences.length).toBeGreaterThan(0);
    expect(highTileReferences.map((entry) => tiles.indexOf(resolveTilesetTile(tiles, entry.tileIndex, 1)))).toEqual(
      expectedSourceTileIndices
    );
  });

  it("keeps the bundled Elm's Lab PC desk metatile tied to the traced ASM source tiles", () => {
    const labMetatiles = fs.readFileSync(getTilesetMetatilesPath("lab"));
    const deskBytes = [...labMetatiles.subarray(0x21 * 16, 0x21 * 16 + 16)];
    const tiles = surfaceTiles(192);
    const resolvedSourceTileIds = deskBytes.map((tileId) =>
      tileId >= 0x80 ? tiles.indexOf(resolveTilesetTile(tiles, tileId, 1)) : tileId
    );

    expect(deskBytes).toEqual([
      0x10, 0x10, 0x85, 0x86,
      0x81, 0x82, 0x83, 0x84,
      0x91, 0x92, 0x93, 0x94,
      0xa1, 0xa2, 0xa3, 0xa4,
    ]);
    expect(resolvedSourceTileIds).toEqual([
      0x10, 0x10, 0x65, 0x66,
      0x61, 0x62, 0x63, 0x64,
      0x71, 0x72, 0x73, 0x74,
      0x81, 0x82, 0x83, 0x84,
    ]);
  });

  it("materializes palette-map-only bank-1 tile ids used by ASM link-room tilesets", async () => {
    for (const tilesetName of ["gate", "pokecenter"]) {
      const tileset = new OverworldTileset(tilesetName, "day");
      await tileset.ready;

      expect(tileset.tiles.length).toBeGreaterThanOrEqual(0xe0);
      expect(tileset.tiles[0xc0]).toBeDefined();
      expect(tileset.tiles[0xdf]).toBeDefined();
    }
  });

  it("keeps Cable Club room block layouts byte-for-byte with ASM", () => {
    const mapBlocks = JSON.parse(
      fs.readFileSync(path.join(getAssetsRoot(), "data", "map_blocks.json"), "utf8")
    ) as Record<string, string>;
    const rooms = [
      ["TradeCenter_Blocks", "TradeCenter.blk"],
      ["Colosseum_Blocks", "Colosseum.blk"],
      ["Pokecenter2F_Blocks", "Pokecenter2F.blk"],
    ];
    const disassemblyMapsRoot = path.join(getDisassemblyRoot(), "maps");

    for (const [label, filename] of rooms) {
      const exported = Buffer.from(mapBlocks[label], "base64");
      const asm = fs.readFileSync(path.join(disassemblyMapsRoot, filename));
      expect([...exported]).toEqual([...asm]);
    }
    expect(mapBlocks.TimeCapsule_Blocks).toBe(mapBlocks.TradeCenter_Blocks);
  });

  it("renders Elm's Lab PC desk face high enough to cover standing sprites", async () => {
    const tileset = new OverworldTileset("lab", "day");
    await tileset.ready;
    const target = new gameEngine.Surface(TILE_SIZE * METATILE_WIDTH, TILE_SIZE * METATILE_WIDTH);
    const blits: Array<[
      unknown,
      [number, number],
      { x: number; y: number; width: number; height: number } | undefined
    ]> = [];
    target.blit = ((
      surface: unknown,
      dest: [number, number],
      area?: { x: number; y: number; width: number; height: number }
    ) => {
      blits.push([surface, dest, area]);
    }) as typeof target.blit;

    tileset.renderPriorityMetatile(0x21, target, 0, 0);

    expect(tileset.metatiles[0x21].collision).toEqual([
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("WALL"),
      resolveCollisionValue("WALL"),
    ]);
    expect(blits.map(([, dest]) => dest)).toEqual([
      [0, TILE_SIZE + TILE_SIZE / 2],
      [TILE_SIZE, TILE_SIZE + TILE_SIZE / 2],
      [2 * TILE_SIZE, TILE_SIZE + TILE_SIZE / 2],
      [3 * TILE_SIZE, TILE_SIZE + TILE_SIZE / 2],
      [0, 2 * TILE_SIZE],
      [TILE_SIZE, 2 * TILE_SIZE],
      [2 * TILE_SIZE, 2 * TILE_SIZE],
      [3 * TILE_SIZE, 2 * TILE_SIZE],
      [0, 3 * TILE_SIZE],
      [TILE_SIZE, 3 * TILE_SIZE],
      [2 * TILE_SIZE, 3 * TILE_SIZE],
      [3 * TILE_SIZE, 3 * TILE_SIZE],
    ]);
    expect(blits.slice(0, 4).map(([, , area]) => area)).toEqual([
      { x: 0, y: TILE_SIZE / 2, width: TILE_SIZE, height: TILE_SIZE / 2 },
      { x: 0, y: TILE_SIZE / 2, width: TILE_SIZE, height: TILE_SIZE / 2 },
      { x: 0, y: TILE_SIZE / 2, width: TILE_SIZE, height: TILE_SIZE / 2 },
      { x: 0, y: TILE_SIZE / 2, width: TILE_SIZE, height: TILE_SIZE / 2 },
    ]);
    expect(blits.slice(4).every(([, , area]) => area === undefined)).toBe(true);
  });

  it("renders Game Corner counter faces high enough to cover standing sprites", async () => {
    const tileset = new OverworldTileset("game_corner", "day");
    await tileset.ready;
    const target = new gameEngine.Surface(TILE_SIZE * METATILE_WIDTH, TILE_SIZE * METATILE_WIDTH);
    const blits: Array<[
      unknown,
      [number, number],
      { x: number; y: number; width: number; height: number } | undefined
    ]> = [];
    target.blit = ((
      surface: unknown,
      dest: [number, number],
      area?: { x: number; y: number; width: number; height: number }
    ) => {
      blits.push([surface, dest, area]);
    }) as typeof target.blit;

    tileset.renderPriorityMetatile(0x09, target, 0, 0);

    expect(tileset.metatiles[0x09].collision).toEqual([
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("COUNTER"),
      resolveCollisionValue("COUNTER"),
    ]);
    expect(blits.map(([, dest]) => dest)).toEqual([
      [0, TILE_SIZE + TILE_SIZE / 2],
      [TILE_SIZE, TILE_SIZE + TILE_SIZE / 2],
      [2 * TILE_SIZE, TILE_SIZE + TILE_SIZE / 2],
      [3 * TILE_SIZE, TILE_SIZE + TILE_SIZE / 2],
      [0, 2 * TILE_SIZE],
      [TILE_SIZE, 2 * TILE_SIZE],
      [2 * TILE_SIZE, 2 * TILE_SIZE],
      [3 * TILE_SIZE, 2 * TILE_SIZE],
      [0, 3 * TILE_SIZE],
      [TILE_SIZE, 3 * TILE_SIZE],
      [2 * TILE_SIZE, 3 * TILE_SIZE],
      [3 * TILE_SIZE, 3 * TILE_SIZE],
    ]);
    expect(blits.slice(0, 4).map(([, , area]) => area)).toEqual([
      { x: 0, y: TILE_SIZE / 2, width: TILE_SIZE, height: TILE_SIZE / 2 },
      { x: 0, y: TILE_SIZE / 2, width: TILE_SIZE, height: TILE_SIZE / 2 },
      { x: 0, y: TILE_SIZE / 2, width: TILE_SIZE, height: TILE_SIZE / 2 },
      { x: 0, y: TILE_SIZE / 2, width: TILE_SIZE, height: TILE_SIZE / 2 },
    ]);
    expect(blits.slice(4).every(([, , area]) => area === undefined)).toBe(true);
  });

  it("renders bookshelf desk faces into the priority plane", async () => {
    const tileset = new OverworldTileset("lab", "day");
    await tileset.ready;
    const target = new gameEngine.Surface(TILE_SIZE * METATILE_WIDTH, TILE_SIZE * METATILE_WIDTH);
    const blits: Array<[unknown, [number, number]]> = [];
    target.blit = ((surface: unknown, dest: [number, number]) => {
      blits.push([surface, dest]);
    }) as typeof target.blit;

    tileset.renderPriorityMetatile(0x14, target, 0, 0);

    expect(tileset.metatiles[0x14].collision).toEqual([
      resolveCollisionValue("WALL"),
      resolveCollisionValue("WALL"),
      resolveCollisionValue("BOOKSHELF"),
      resolveCollisionValue("BOOKSHELF"),
    ]);
    expect(blits.map(([, dest]) => dest)).toEqual([
      [0, 2 * TILE_SIZE],
      [TILE_SIZE, 2 * TILE_SIZE],
      [2 * TILE_SIZE, 2 * TILE_SIZE],
      [3 * TILE_SIZE, 2 * TILE_SIZE],
      [0, 3 * TILE_SIZE],
      [TILE_SIZE, 3 * TILE_SIZE],
      [2 * TILE_SIZE, 3 * TILE_SIZE],
      [3 * TILE_SIZE, 3 * TILE_SIZE],
    ]);
  });

  it("renders mart shelf faces into the priority plane", async () => {
    const tileset = new OverworldTileset("mart", "day");
    await tileset.ready;
    const target = new gameEngine.Surface(TILE_SIZE * METATILE_WIDTH, TILE_SIZE * METATILE_WIDTH);
    const blits: Array<[unknown, [number, number]]> = [];
    target.blit = ((surface: unknown, dest: [number, number]) => {
      blits.push([surface, dest]);
    }) as typeof target.blit;

    tileset.renderPriorityMetatile(0x13, target, 0, 0);

    expect(tileset.metatiles[0x13].collision).toEqual([
      resolveCollisionValue("WALL"),
      resolveCollisionValue("WALL"),
      resolveCollisionValue("MART_SHELF"),
      resolveCollisionValue("MART_SHELF"),
    ]);
    expect(blits.map(([, dest]) => dest)).toEqual([
      [0, 2 * TILE_SIZE],
      [TILE_SIZE, 2 * TILE_SIZE],
      [2 * TILE_SIZE, 2 * TILE_SIZE],
      [3 * TILE_SIZE, 2 * TILE_SIZE],
      [0, 3 * TILE_SIZE],
      [TILE_SIZE, 3 * TILE_SIZE],
      [2 * TILE_SIZE, 3 * TILE_SIZE],
      [3 * TILE_SIZE, 3 * TILE_SIZE],
    ]);
  });

  it("parses palette files with 1 or 4 RGB entries per line", () => {
    const multi = `
      ; day
      RGB 28,31,16, 21,21,21, 13,13,13, 07,07,07
    `;
    const multiPalettes = parsePaletteFile(multi);
    expect(multiPalettes).toHaveLength(1);
    expect(multiPalettes[0]).toHaveLength(4);

    const single = `
      RGB 30,28,26
      RGB 19,19,19
      RGB 13,13,13
      RGB 07,07,07
    `;
    const singlePalettes = parsePaletteFile(single);
    expect(singlePalettes).toHaveLength(1);
    expect(singlePalettes[0]).toHaveLength(4);
  });

  it("always includes the runtime asset copy as a metatile layout fallback", () => {
    const paths = getTilesetMetatilesCandidatePaths("johto");
    expect(paths).toContainEqual(expect.stringContaining("johto_metatiles.bin"));
    expect(paths).toContainEqual(expect.stringContaining("/assets/data/tilesets/johto_metatiles.bin"));
  });

  it("seeds safe fallback metatiles and tiles before async load completes", () => {
    const tileset = new OverworldTileset("players_house", "day");

    expect(tileset.metatiles.length).toBeGreaterThan(0);
    expect(tileset.tiles.length).toBeGreaterThan(0);
    expect(tileset.metatiles[0]?.collision).toEqual([
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("FLOOR"),
    ]);
  });

  it("loads tileset data from disk when running in node", async () => {
    const originalFetch = globalThis.fetch;
    const fetchSpy = jest.fn(() => Promise.reject(new Error("fetch should not be called")));
    globalThis.fetch = fetchSpy as unknown as typeof globalThis.fetch;
    try {
      const tileset = new OverworldTileset("johto", "day");
      await tileset.ready;
      expect(tileset.loaded).toBe(true);
      expect(
        tileset.metatiles.some((metatile) =>
          metatile.collision.some((value) => value === resolveCollisionValue("WALL"))
        )
      ).toBe(true);
      expect(fetchSpy).not.toHaveBeenCalled();
    } finally {
      globalThis.fetch = originalFetch;
    }
  });
});

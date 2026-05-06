import { createInitialGameState } from "@pokecrystal/core/core/state";
import { METATILE_SIZE, TILE_SIZE } from "@pokecrystal/core/engine/world/tile/constants";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { create_map_surface, create_priority_surface } from "./map-geometry";
import { OverworldMap } from "./overworld-map";
import { OverworldTileset } from "./overworld-tileset";
import { TilesetAnimationController } from "./tileset-animation";

const findMetatileUsingTile = (
  map: OverworldMap,
  tileset: OverworldTileset,
  tileIndex: number
): [number, number] => {
  for (let index = 0; index < map.metatileIds.length; index += 1) {
    const metatile = tileset.metatiles[map.metatileIds[index]];
    if (!metatile) {
      continue;
    }
    const hasTile = metatile.tiles.some((row) =>
      row.some((entry) => entry.tileIndex === tileIndex)
    );
    if (hasTile) {
      return [index % map.width, Math.floor(index / map.width)];
    }
  }
  throw new Error(`Expected ${map.mapName} to contain tile ${tileIndex.toString(16)}.`);
};

const captureMetatile = (
  surface: InstanceType<typeof gameEngine.Surface>,
  metatileX: number,
  metatileY: number
): number[] => {
  const rect = new gameEngine.Rect(
    metatileX * METATILE_SIZE,
    metatileY * METATILE_SIZE,
    METATILE_SIZE,
    METATILE_SIZE
  );
  return Array.from(surface.subsurface(rect).getImageData().data);
};

const makeSolidTile = (color: [number, number, number, number]): InstanceType<typeof gameEngine.Surface> => {
  const tile = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
  tile.fill(color);
  return tile;
};

const makeGrayTile = (gray: number): InstanceType<typeof gameEngine.Surface> =>
  makeSolidTile([gray, gray, gray, 255]);

const makeFourColorTile = (): InstanceType<typeof gameEngine.Surface> => {
  const tile = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
  const colors: Array<[number, number, number, number]> = [
    [240, 240, 240, 255],
    [160, 160, 160, 255],
    [80, 80, 80, 255],
    [10, 10, 10, 255],
  ];
  for (let y = 0; y < TILE_SIZE; y += 1) {
    for (let x = 0; x < TILE_SIZE; x += 1) {
      const index = (x >= TILE_SIZE / 2 ? 1 : 0) + (y >= TILE_SIZE / 2 ? 2 : 0);
      tile.set_at([x, y], colors[index]);
    }
  }
  return tile;
};

describe("TilesetAnimationController", () => {
  it("applies palettes to tower pillar frames when updating tiles", () => {
    const gameState = createInitialGameState();
    gameState.wram.wTileAnimationTimer = 0;

    const controller = new TilesetAnimationController(
      { refresh_composite_surfaces: jest.fn() },
      gameState
    );

    const tileset = {
      tiles: Array.from({ length: 0x60 }, () => new gameEngine.Surface(TILE_SIZE, TILE_SIZE)),
    } as any;

    const frameSurface = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
    const frameSets = controller.TOWER_PILLAR_TILE_DESTINATIONS.map(() => [
      { tileBytes: new Array(16).fill(0), surface: frameSurface },
    ]);

    (controller as any).tilesets = [tileset];
    (controller as any).towerActive = true;
    (controller as any)._towerFrameCache = frameSets;

    const destIndex = controller.TOWER_PILLAR_TILE_DESTINATIONS[0];
    const originalTile = tileset.tiles[destIndex];

    const updated = (controller as any)._applyTowerFrames({ force: true });

    expect(updated).toBe(true);
    expect(tileset.tiles[destIndex]).not.toBe(originalTile);
    expect(tileset.tiles[destIndex]).not.toBe(frameSurface);
  });

  it("writes tower pillar tiles in ASM command order and waits after ten updates", () => {
    const gameState = createInitialGameState();
    gameState.wram.wTileAnimationTimer = 0;
    const controller = new TilesetAnimationController(
      { refresh_composite_surfaces: jest.fn() },
      gameState
    );
    const tileset = {
      tiles: Array.from({ length: 0x60 }, () => new gameEngine.Surface(TILE_SIZE, TILE_SIZE)),
    } as any;

    const frameSets = controller.TOWER_PILLAR_TILE_DESTINATIONS.map((destination, index) => [
      { tileBytes: new Array(16).fill(index + 1), surface: new gameEngine.Surface(TILE_SIZE, TILE_SIZE) },
    ]);

    (controller as any).tilesets = [tileset];
    (controller as any).towerActive = true;
    (controller as any)._towerFrameCache = frameSets;

    const uploadSpy = jest.spyOn(controller as any, "_uploadTileFrame");
    controller.TOWER_PILLAR_UPDATE_SEQUENCE.forEach((destination, step) => {
      gameState.wram.wTileAnimationTimer = step;
      const dirty = (controller as any)._applyTowerFrames();
      expect(dirty).toBe(true);
      expect(uploadSpy).toHaveBeenLastCalledWith(
        frameSets[controller.TOWER_PILLAR_TILE_DESTINATIONS.indexOf(destination)][0],
        destination
      );
    });

    gameState.wram.wTileAnimationTimer = 10;
    expect((controller as any)._applyTowerFrames()).toBe(false);
  });

  it("updates upper and lower Sprout Tower pillar metatiles when tower frame advances", async () => {
    const gameState = createInitialGameState();
    gameState.wram.wTileAnimationTimer = 0;
    const refreshComposite = jest.fn();
    const controller = new TilesetAnimationController(
      { refresh_composite_surfaces: refreshComposite },
      gameState
    );
    const tileset = new OverworldTileset("tower", "day");
    await tileset.ready;
    const map = new OverworldMap("SproutTower1F", 10, 8);
    const surface = create_map_surface(map, tileset, { vram: gameState.vram });
    const prioritySurface = create_priority_surface(map, tileset);

    controller.onMapLoaded({
      mapName: "SproutTower1F",
      mapObj: map,
      tileset,
      surface,
      prioritySurface,
    });

    const rect = new gameEngine.Rect(4 * METATILE_SIZE, 3 * METATILE_SIZE, METATILE_SIZE, METATILE_SIZE);
    const lowerRect = new gameEngine.Rect(4 * METATILE_SIZE, 4 * METATILE_SIZE, METATILE_SIZE, METATILE_SIZE);
    const capture = (target: InstanceType<typeof gameEngine.Surface>, dirtyRect: InstanceType<typeof gameEngine.Rect>) =>
      Array.from(target.subsurface(dirtyRect).getImageData().data);

    const upperBefore = capture(surface, rect);
    const lowerBefore = capture(surface, lowerRect);

    for (let i = 0; i < controller.TOWER_PILLAR_UPDATE_SEQUENCE.length; i += 1) {
      gameState.wram.wTileAnimationTimer = 16 + i;
      const dirty = (controller as any)._applyTowerFrames();
      (controller as any)._refreshTargetsIfNeeded(dirty);
    }

    const upperAfter = capture(surface, rect);
    const lowerAfter = capture(surface, lowerRect);

    expect(upperAfter).not.toEqual(upperBefore);
    expect(lowerAfter).not.toEqual(lowerBefore);
    expect(refreshComposite).toHaveBeenCalled();
  });

  it("animates Elite Four room lava bubble metatiles from the ASM timer", async () => {
    const gameState = createInitialGameState();
    gameState.wram.wTileAnimationTimer = 0;
    const refreshComposite = jest.fn();
    const controller = new TilesetAnimationController(
      { refresh_composite_surfaces: refreshComposite },
      gameState
    );
    const tileset = new OverworldTileset("elite_four_room", "day");
    await tileset.ready;
    const map = new OverworldMap("BrunosRoom", 5, 9);
    const surface = create_map_surface(map, tileset, { vram: gameState.vram });
    const prioritySurface = create_priority_surface(map, tileset);

    controller.onMapLoaded({
      mapName: "BrunosRoom",
      mapObj: map,
      tileset,
      surface,
      prioritySurface,
    });

    const coords = (controller as any)._locateAnimatedMetatiles(map, tileset) as Array<[number, number]>;
    expect(coords.length).toBeGreaterThan(0);
    const [metatileX, metatileY] = coords[0];
    const rect = new gameEngine.Rect(
      metatileX * METATILE_SIZE,
      metatileY * METATILE_SIZE,
      METATILE_SIZE,
      METATILE_SIZE
    );
    const capture = () => Array.from(surface.subsurface(rect).getImageData().data);

    const before = capture();
    gameState.wram.wTileAnimationTimer = 2;
    const dirty = (controller as any)._applyLavaFrames();
    (controller as any)._refreshTargetsIfNeeded(dirty);
    const after = capture();

    expect(after).not.toEqual(before);
    expect(refreshComposite).toHaveBeenCalled();
  });

  it("keeps lava animation frames on the red tileset palette when a frame omits white", async () => {
    const gameState = createInitialGameState();
    gameState.wram.wTileAnimationTimer = 0;
    const controller = new TilesetAnimationController(
      { refresh_composite_surfaces: jest.fn() },
      gameState
    );
    const tileset = new OverworldTileset("elite_four_room", "day");
    await tileset.ready;
    const map = new OverworldMap("BrunosRoom", 5, 9);
    const surface = create_map_surface(map, tileset, { vram: gameState.vram });
    const prioritySurface = create_priority_surface(map, tileset);

    controller.onMapLoaded({
      mapName: "BrunosRoom",
      mapObj: map,
      tileset,
      surface,
      prioritySurface,
    });

    gameState.wram.wTileAnimationTimer = 2;
    expect((controller as any)._applyLavaFrames()).toBe(true);

    const image = tileset.tiles[controller.LAVA_BUBBLE_TILE_2_INDEX].getImageData();
    const [r, g, b] = Array.from(image.data.slice(0, 3));
    expect([r, g, b]).toEqual([247, 82, 49]);
    expect([r, g, b]).not.toEqual([255, 156, 197]);
  });

  it("animates Ilex Forest tree tiles while the Celebi forest event is restless", async () => {
    const gameState = createInitialGameState();
    gameState.wram.wTileAnimationTimer = 0;
    gameState.wram.engine_flags.ENGINE_FOREST_IS_RESTLESS = true;
    const refreshComposite = jest.fn();
    const controller = new TilesetAnimationController(
      { refresh_composite_surfaces: refreshComposite },
      gameState
    );
    const tileset = new OverworldTileset("forest", "day");
    await tileset.ready;
    const map = new OverworldMap("IlexForest", 15, 27);
    const surface = create_map_surface(map, tileset, { vram: gameState.vram });
    const prioritySurface = create_priority_surface(map, tileset);

    controller.onMapLoaded({
      mapName: "IlexForest",
      mapObj: map,
      tileset,
      surface,
      prioritySurface,
    });

    const [metatileX, metatileY] = findMetatileUsingTile(
      map,
      tileset,
      controller.FOREST_TREE_LEFT_TILE_INDEX
    );
    const before = captureMetatile(surface, metatileX, metatileY);
    gameState.wram.wTileAnimationTimer = 1;
    const dirty = (controller as any)._applyForestTreeFrames();
    (controller as any)._refreshTargetsIfNeeded(dirty);
    const after = captureMetatile(surface, metatileX, metatileY);

    expect(after).not.toEqual(before);
    expect(refreshComposite).toHaveBeenCalled();
  });

  it.each([
    ["cave", "UnionCave1F", 10, 18, "CAVE_WATER_TILE_INDEX"],
    ["ice_path", "IcePath1F", 20, 18, "ICE_PATH_WATER_VISIBLE_TILE_INDEX"],
  ] as const)(
    "applies ASM scroll-buffer tile animation for %s maps",
    async (tilesetName, mapName, width, height, tileIndexKey) => {
      const gameState = createInitialGameState();
      const refreshComposite = jest.fn();
      const controller = new TilesetAnimationController(
        { refresh_composite_surfaces: refreshComposite },
        gameState
      );
      const tileset = new OverworldTileset(tilesetName, "day");
      await tileset.ready;
      const map = new OverworldMap(mapName, width, height);
      const surface = create_map_surface(map, tileset, { vram: gameState.vram });
      const prioritySurface = create_priority_surface(map, tileset);

      controller.onMapLoaded({
        mapName,
        mapObj: map,
        tileset,
        surface,
        prioritySurface,
      });

      const [metatileX, metatileY] = findMetatileUsingTile(
        map,
        tileset,
        controller[tileIndexKey]
      );
      const before = captureMetatile(surface, metatileX, metatileY);
      let dirty = false;
      for (let frame = 0; frame <= 4; frame += 1) {
        dirty = (controller as any)._applyCaveScrollStep() || dirty;
      }
      (controller as any)._refreshTargetsIfNeeded(dirty);
      const after = captureMetatile(surface, metatileX, metatileY);

      expect(after).not.toEqual(before);
      expect(refreshComposite).toHaveBeenCalled();
    }
  );

  it("renders every whirlpool tile frame from the ASM timer and restores the base tiles afterward", async () => {
    const gameState = createInitialGameState();
    const refreshComposite = jest.fn();
    const controller = new TilesetAnimationController(
      { refresh_composite_surfaces: refreshComposite },
      gameState
    );
    const tileset = new OverworldTileset("johto", "day");
    await tileset.ready;
    const map = new OverworldMap("Route41", 1, 1);
    map.metatileIds = [0x07];
    const surface = create_map_surface(map, tileset, { vram: gameState.vram });
    const prioritySurface = create_priority_surface(map, tileset);

    controller.onMapLoaded({
      mapName: "Route41",
      mapObj: map,
      tileset,
      surface,
      prioritySurface,
    });

    const before = captureMetatile(surface, 0, 0);
    controller.setWhirlpoolActive(true);
    const frames: number[][] = [];
    for (let frame = 0; frame < 4; frame += 1) {
      gameState.wram.wTileAnimationTimer = frame;
      (controller as any)._applyWhirlpoolFrame({ force: true });
      frames.push(captureMetatile(surface, 0, 0));
    }

    expect(new Set(frames.map((frame) => frame.join(","))).size).toBe(4);
    expect(refreshComposite).toHaveBeenCalled();

    controller.setWhirlpoolActive(false);
    expect(captureMetatile(surface, 0, 0)).toEqual(before);
    expect((controller as any).whirlpoolBackups.size).toBe(0);
  });

  it("palettes whirlpool frames from the original tile backup instead of the previous frame", () => {
    const gameState = createInitialGameState();
    const controller = new TilesetAnimationController(
      { refresh_composite_surfaces: jest.fn() },
      gameState
    );
    const tileset = {
      tiles: Array.from({ length: 0x50 }, () => makeSolidTile([0, 0, 0, 255])),
    } as any;
    for (const tileIndex of controller.WHIRLPOOL_TILE_INDICES) {
      tileset.tiles[tileIndex] = makeFourColorTile();
    }

    (controller as any).tilesets = [tileset];
    (controller as any).whirlpoolBackups = new Map([
      [
        tileset,
        new Map(
          controller.WHIRLPOOL_TILE_INDICES.map((tileIndex) => [
            tileIndex,
            [new Array(16).fill(0), tileset.tiles[tileIndex]],
          ])
        ),
      ],
    ]);
    (controller as any).fieldMoveLoader = {
      surfaceForWhirlpoolFrame: (_tileIndex: number, frameIndex: number) =>
        frameIndex === 0 ? makeGrayTile(255) : makeGrayTile(85),
    };

    (controller as any)._updateWhirlpoolTilesetSurfaces(0);
    (controller as any)._updateWhirlpoolTilesetSurfaces(1);

    const data = tileset.tiles[controller.WHIRLPOOL_TILE_INDICES[0]].getImageData().data;
    expect(Array.from(data.slice(0, 3))).toEqual([80, 80, 80]);
  });
});

import { OverworldEngine } from "./overworld";
import { resolveCollisionValue } from "./collision-data";

type TestMap = {
  width: number;
  height: number;
  getMetatileAt: (x: number, y: number) => number;
};

type TestTileset = {
  tilesetName: string;
  metatiles: Array<{
    collision: number[];
  }>;
};

const buildContext = (permission: number) => {
  const map: TestMap = {
    width: 4,
    height: 4,
    getMetatileAt: () => 0,
  };
  const tileset: TestTileset = {
    tilesetName: "Test",
    metatiles: [{ collision: [permission, permission, permission, permission] }],
  };
  return {
    map,
    tileset,
    player_x: 6,
    player_y: 6,
    TILES_PER_COLLISION: 2,
  } as unknown as OverworldEngine;
};

describe("OverworldEngine._counter_adjusted_tile", () => {
  it("doubles interaction distance for standard counter tiles", () => {
    const context = buildContext(resolveCollisionValue("COUNTER"));
    const result = OverworldEngine.prototype._counter_adjusted_tile.call(context, 6, 4);
    expect(result).toEqual([6, 2]);
  });

  it("doubles interaction distance for counter_98 tiles", () => {
    const context = buildContext(resolveCollisionValue("COUNTER_98"));
    const result = OverworldEngine.prototype._counter_adjusted_tile.call(context, 8, 6);
    expect(result).toEqual([10, 6]);
  });

  it("leaves non-counter tiles untouched", () => {
    const context = buildContext(resolveCollisionValue("FLOOR"));
    const result = OverworldEngine.prototype._counter_adjusted_tile.call(context, 6, 4);
    expect(result).toEqual([6, 4]);
  });

  it("checks the full front edge for counters before adjusting", () => {
    const floor = resolveCollisionValue("FLOOR");
    const counter = resolveCollisionValue("COUNTER");
    const map: TestMap = {
      width: 4,
      height: 4,
      getMetatileAt: () => 0,
    };
    const tileset: TestTileset = {
      tilesetName: "Test",
      metatiles: [{ collision: [counter, floor, floor, floor] }],
    };
    const context = {
      map,
      tileset,
      player_x: 6,
      player_y: 6,
      TILES_PER_COLLISION: 2,
    } as unknown as OverworldEngine;

    const result = OverworldEngine.prototype._counter_adjusted_tile.call(context, 6, 4);
    expect(result).toEqual([5, 2]);
  });

  it("detects counters across the full aisle width when the player is on the right edge", () => {
    const floor = resolveCollisionValue("FLOOR");
    const counter = resolveCollisionValue("COUNTER");
    const map: TestMap = {
      width: 4,
      height: 4,
      getMetatileAt: () => 0,
    };
    const tileset: TestTileset = {
      tilesetName: "Test",
      metatiles: [{ collision: [floor, floor, counter, floor] }],
    };
    const context = {
      map,
      tileset,
      player_x: 7,
      player_y: 9,
      TILES_PER_COLLISION: 2,
    } as unknown as OverworldEngine;

    const result = OverworldEngine.prototype._counter_adjusted_tile.call(context, 7, 7);
    expect(result).toEqual([5, 5]);
  });
});

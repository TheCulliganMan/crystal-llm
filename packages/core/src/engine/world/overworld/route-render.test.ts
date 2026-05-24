import {
  buildRouteRenderSnapshot,
  renderRouteRenderSurface,
  renderRouteRenderTileSurface,
} from "@pokecrystal/core/engine/world/overworld/route-render";
import { resolveCollisionValue } from "@pokecrystal/core/engine/world/overworld/collision-data";
import type { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import type { OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";

const map = (width: number, height: number, metatileIds: number[]): OverworldMap => ({
  mapName: "TestRoute",
  width,
  height,
  metatileIds,
  getMetatileAt: (x: number, y: number) => metatileIds[y * width + x],
} as unknown as OverworldMap);

const tileset = (collisions: number[][]): OverworldTilesetLike => ({
  tilesetName: "test",
  metatiles: collisions.map((collision) => ({ collision })),
  renderMetatile: jest.fn(),
  renderPriorityMetatile: jest.fn(),
});

describe("route render", () => {
  it("renders the full map instead of the Game Boy viewport", () => {
    const floor = resolveCollisionValue("FLOOR");
    const snapshot = buildRouteRenderSnapshot({
      map: "WideRoute",
      mapId: "1:2",
      coordStride: 2,
      mapData: map(6, 5, Array.from({ length: 30 }, () => 0)),
      tileset: tileset([[floor, floor, floor, floor]]),
      detail: "full",
    });

    expect(snapshot.available).toBe(true);
    expect(snapshot.size).toEqual({ width: 24, height: 20 });
    expect(snapshot.grid?.rows).toHaveLength(20);
    expect(snapshot.grid?.rows[0]).toHaveLength(24);
    expect(snapshot.grid?.cells?.[0]?.[0]).toMatchObject({
      token: ".",
      passable: true,
      terrain: "land",
    });
  });

  it("marks collision, ledges, player, warps, and hotspots on map-info coordinates", () => {
    const floor = resolveCollisionValue("FLOOR");
    const wall = resolveCollisionValue("WALL");
    const water = resolveCollisionValue("WATER");
    const grass = resolveCollisionValue("TALL_GRASS");
    const hopDown = resolveCollisionValue("HOP_DOWN");
    const snapshot = buildRouteRenderSnapshot({
      map: "MixedRoute",
      mapId: "1:3",
      coordStride: 2,
      player: { coords: { x: 1, y: 1 }, facing: "right" },
      mapData: map(2, 2, [1, 2, 3, 0]),
      tileset: tileset([
        [floor, floor, floor, floor],
        [hopDown, floor, wall, floor],
        [water, water, water, water],
        [grass, grass, grass, grass],
      ]),
      warps: [{
        index: 1,
        coords: { x: 5, y: 5 },
        target: { map_constant: "NEXT_MAP", map_name: "NextMap", warp_id: 1 },
      }],
      hotspots: [{
        id: "npc-1",
        type: "npc",
        label: "NPC",
        coords: { x: 6, y: 5 },
        visible: true,
        interactable: true,
        token: "N",
      }],
    });

    expect(snapshot.grid?.rows[1]?.[1]).toBe("@");
    expect(snapshot.grid?.rows[2]?.[0]).toBe("d");
    expect(snapshot.grid?.rows[0]?.[4]).toBe("~");
    expect(snapshot.grid?.rows[4]?.[0]).toBe("\"");
    expect(snapshot.grid?.rows[5]?.[5]).toBe("D");
    expect(snapshot.grid?.rows[5]?.[6]).toBe("N");
    expect(snapshot.legend.map((entry) => entry.token)).toEqual(
      expect.arrayContaining(["@", "d", "~", "\"", "D", "N"])
    );
  });

  it("filters inactive coord events before marking triggers", () => {
    const floor = resolveCollisionValue("FLOOR");
    const snapshot = buildRouteRenderSnapshot({
      map: "ScriptedRoute",
      mapId: "1:4",
      coordStride: 2,
      mapData: map(3, 1, [0, 0, 0]),
      tileset: tileset([[floor, floor, floor, floor]]),
      currentScene: "SCENE_A",
      eventFlags: { EVENT_DONE: true },
      dataLoader: {
        get_script_event_flags: (script) => script === "DoneScript" ? ["EVENT_DONE"] : [],
      },
      mapEvents: {
        coord_events: [
          { x: 0, y: 0, scene_id: "SCENE_A", script_name: "ActiveScript" },
          { x: 1, y: 0, scene_id: "SCENE_B", script_name: "InactiveSceneScript" },
          { x: 2, y: 0, script_name: "DoneScript" },
        ],
      },
    });

    expect(snapshot.grid?.rows[1]?.[1]).toBe("*");
    expect(snapshot.grid?.rows[1]?.[3]).toBe(".");
    expect(snapshot.grid?.rows[1]?.[5]).toBe(".");
  });

  it("renders an annotated schematic surface", () => {
    const floor = resolveCollisionValue("FLOOR");
    const snapshot = buildRouteRenderSnapshot({
      map: "ImageRoute",
      mapId: "1:5",
      mapData: map(1, 1, [0]),
      tileset: tileset([[floor, floor, floor, floor]]),
      player: { coords: { x: 1, y: 1 }, facing: "up" },
    });

    const surface = renderRouteRenderSurface(snapshot, { cellSize: 8 });

    expect(surface.get_width()).toBe(32);
    expect(surface.get_height()).toBe(32);
    expect(surface.get_at([11, 9])).not.toEqual(surface.get_at([0, 0]));
  });

  it("can render the full map with actual metatile artwork", () => {
    const floor = resolveCollisionValue("FLOOR");
    const renderMetatile = jest.fn((
      _metatileId: number,
      target: { fill: (color: [number, number, number, number], rect?: { x: number; y: number; width: number; height: number }) => void },
      x: number,
      y: number
    ) => {
      target.fill([12, 34, 56, 255], { x, y, width: 32, height: 32 });
    });
    const snapshot = buildRouteRenderSnapshot({
      map: "TileRoute",
      mapId: "1:6",
      mapData: map(1, 1, [0]),
      tileset: {
        ...tileset([[floor, floor, floor, floor]]),
        renderMetatile,
      },
      player: { coords: { x: 1, y: 1 }, facing: "down" },
    });

    const surface = renderRouteRenderTileSurface({
      snapshot,
      mapData: map(1, 1, [0]),
      tileset: {
        ...tileset([[floor, floor, floor, floor]]),
        renderMetatile,
      },
    });

    expect(surface.get_width()).toBe(32);
    expect(surface.get_height()).toBe(32);
    expect(renderMetatile).toHaveBeenCalledWith(0, expect.anything(), 0, 0, { vram: null });
    expect(surface.get_at([2, 2])).toEqual([12, 34, 56, 255]);
    expect(surface.get_at([9, 9])).not.toEqual([12, 34, 56, 255]);
  });
});

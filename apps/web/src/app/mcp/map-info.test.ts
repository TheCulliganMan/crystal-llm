import { resolveCollisionValue } from "@pokecrystal/core/engine/world/overworld/collision-data";
import { buildMapInfoSnapshot } from "./map-info";

describe("buildMapInfoSnapshot", () => {
  it("prioritizes the Wise Trio room after Clear Bell before routing to Tin Tower 1F", () => {
    const snapshot = buildMapInfoSnapshot({
      map: "EcruteakCity",
      mapGroup: 4,
      mapNumber: 9,
      playerCoords: { x: 75, y: 19 },
      facing: "down",
      eventFlags: {
        EVENT_GOT_CLEAR_BELL: true,
        EVENT_KOJI_ALLOWS_YOU_PASSAGE_TO_TIN_TOWER: false,
        EVENT_FOUGHT_SUICUNE: false,
      },
      overworld: {
        TILES_PER_COLLISION: 2,
        _map_events: {
          warps: [
            {
              index: 4,
              x: 20,
              y: 2,
              target_map_constant: "WISE_TRIOS_ROOM",
              target_map: "WiseTriosRoom",
              target_warp_id: 1,
            },
            {
              index: 12,
              x: 37,
              y: 7,
              target_map_constant: "TIN_TOWER_1F",
              target_map: "TinTower1F",
              target_warp_id: 1,
            },
          ],
          bg_events: [],
          coord_events: [],
        },
      },
    });

    expect(snapshot.hotspots[0]).toEqual(
      expect.objectContaining({
        type: "objective",
        label: "Wise Trio test",
        coords: { x: 41, y: 5 },
      })
    );
    expect(snapshot.hotspots.find((hotspot) => hotspot.label === "Warp: Tin Tower1 F")).toEqual(
      expect.objectContaining({ type: "warp", coords: { x: 75, y: 15 } })
    );
  });

  it("surfaces the Olivine Pokecenter nurse approach across the real counter lane", () => {
    const floor = resolveCollisionValue("FLOOR");
    const counter = resolveCollisionValue("COUNTER");
    const map = {
      width: 8,
      height: 6,
      getMetatileAt: (x: number, y: number) => (x === 1 && y === 1 ? 1 : 0),
    };
    const tileset = {
      tilesetName: "pokecenter",
      metatiles: [
        { collision: [floor, floor, floor, floor] },
        { collision: [floor, counter, floor, floor] },
      ],
    };

    const snapshot = buildMapInfoSnapshot({
      map: "OlivinePokecenter1F",
      mapGroup: 0,
      mapNumber: 0,
      playerCoords: { x: 7, y: 7 },
      facing: "up",
      overworld: {
        TILES_PER_COLLISION: 2,
        _map_events: { warps: [], bg_events: [], coord_events: [] },
        map,
        tileset,
        npcs: [
          {
            x: 7,
            y: 3,
            objectIndex: 1,
            event: { script: "OlivinePokecenter1FNurseScript" },
          },
        ],
      },
    });

    const healer = snapshot.hotspots.find((hotspot) => hotspot.type === "heal");
    expect(healer).toEqual(expect.objectContaining({ label: "Healer", coords: { x: 7, y: 3 } }));
    expect(healer?.approach_tiles).toContainEqual({ coords: { x: 7, y: 7 }, facing: "up" });
  });
});

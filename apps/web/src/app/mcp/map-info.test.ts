import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { resolveCollisionValue } from "@pokecrystal/core/engine/world/overworld/collision-data";
import { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import { OverworldTileset } from "@pokecrystal/core/engine/world/overworld/overworld-tileset";
import { buildMapInfoSnapshot } from "./map-info";

describe("buildMapInfoSnapshot", () => {
  const pokecenterNurseCases = [
    ["AzaleaPokecenter1F", "AzaleaPokecenter1FNurseScript"],
    ["BlackthornPokecenter1F", "BlackthornPokecenter1FNurseScript"],
    ["CeladonPokecenter1F", "CeladonPokecenter1FNurseScript"],
    ["CeruleanPokecenter1F", "CeruleanPokecenter1FNurseScript"],
    ["CherrygrovePokecenter1F", "CherrygrovePokecenter1FNurseScript"],
    ["CianwoodPokecenter1F", "CianwoodPokecenter1FNurseScript"],
    ["CinnabarPokecenter1F", "CinnabarPokecenter1FNurseScript"],
    ["EcruteakPokecenter1F", "EcruteakPokecenter1FNurseScript"],
    ["FuchsiaPokecenter1F", "FuchsiaPokecenter1FNurseScript"],
    ["GoldenrodPokecenter1F", "GoldenrodPokecenter1FNurseScript"],
    ["IndigoPlateauPokecenter1F", "IndigoPlateauPokecenter1FNurseScript"],
    ["LavenderPokecenter1F", "LavenderPokecenter1FNurseScript"],
    ["MahoganyPokecenter1F", "MahoganyPokecenter1FNurseScript"],
    ["OlivinePokecenter1F", "OlivinePokecenter1FNurseScript"],
    ["PewterPokecenter1F", "PewterPokecenter1FNurseScript"],
    ["Route10Pokecenter1F", "Route10Pokecenter1FNurseScript"],
    ["Route32Pokecenter1F", "Route32Pokecenter1FNurseScript"],
    ["SaffronPokecenter1F", "SaffronPokecenter1FNurseScript"],
    ["SilverCavePokecenter1F", "SilverCavePokecenter1FNurseScript"],
    ["VermilionPokecenter1F", "VermilionPokecenter1FNurseScript"],
    ["VioletPokecenter1F", "VioletPokecenterNurse"],
    ["ViridianPokecenter1F", "ViridianPokecenter1FNurseScript"],
  ] as const;

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

  it.each(pokecenterNurseCases)(
    "surfaces the %s nurse approach across the real counter lane",
    async (mapName, scriptName) => {
      const loader = new DataLoader();
      loader.load_map_attributes();
      loader.load_map_dimensions();
      loader.load_npc_data();

      const attributes = loader.map_attributes.get(mapName);
      const dimensions = attributes?.map_constant
        ? loader.map_dimensions.get(attributes.map_constant)
        : undefined;
      const nurseEvent = loader.npc_data
        .get(mapName)
        ?.find((event) => event.script === scriptName);
      expect(attributes?.tileset_name).toBe("pokecenter");
      expect(dimensions).toBeTruthy();
      expect(nurseEvent).toBeTruthy();

      const stride = 2;
      const offset = stride - 1;
      const nurseX = nurseEvent!.x * stride + offset;
      const nurseY = nurseEvent!.y * stride + offset;
      const map = new OverworldMap(mapName, dimensions!.width, dimensions!.height, attributes!.blocks_label);
      const tileset = new OverworldTileset(attributes!.tileset_name, "day");
      await tileset.ready;

      const snapshot = buildMapInfoSnapshot({
        map: mapName,
        mapGroup: 0,
        mapNumber: 0,
        playerCoords: { x: nurseX, y: nurseY + 4 },
        facing: "up",
        overworld: {
          TILES_PER_COLLISION: stride,
          _map_events: { warps: [], bg_events: [], coord_events: [] },
          map,
          tileset,
          npcs: [
            {
              x: nurseX,
              y: nurseY,
              objectIndex: 1,
              event: nurseEvent,
            },
          ],
        },
      });

      const healer = snapshot.hotspots.find((hotspot) => hotspot.type === "heal");
      expect(healer).toEqual(expect.objectContaining({ label: "Healer", coords: { x: nurseX, y: nurseY } }));
      expect(healer?.approach_tiles).toContainEqual({ coords: { x: nurseX, y: nurseY + 4 }, facing: "up" });
    }
  );

  it("attaches the Mahogany Pokecenter nurse script to the real healer hotspot", async () => {
    const loader = new DataLoader();
    loader.load_map_attributes();
    loader.load_map_dimensions();
    loader.load_npc_data();

    const mapName = "MahoganyPokecenter1F";
    const attributes = loader.map_attributes.get(mapName);
    const dimensions = attributes?.map_constant
      ? loader.map_dimensions.get(attributes.map_constant)
      : undefined;
    expect(attributes?.tileset_name).toBe("pokecenter");
    expect(dimensions).toBeTruthy();

    const map = new OverworldMap(mapName, dimensions!.width, dimensions!.height, attributes!.blocks_label);
    const tileset = new OverworldTileset(attributes!.tileset_name, "day");
    await tileset.ready;

    const snapshot = buildMapInfoSnapshot({
      map: mapName,
      mapGroup: 2,
      mapNumber: 3,
      playerCoords: { x: 7, y: 7 },
      facing: "up",
      dataLoader: loader,
      overworld: {
        TILES_PER_COLLISION: 2,
        _map_events: { warps: [], bg_events: [], coord_events: [] },
        map,
        tileset,
        current_map_name: mapName,
        npcs: [],
      },
    });

    const healer = snapshot.hotspots.find((hotspot) => hotspot.type === "heal");
    expect(healer).toEqual(expect.objectContaining({
      coords: { x: 7, y: 3 },
      script: "MahoganyPokecenter1FNurseScript",
      object_index: 1,
    }));
    expect(healer?.approach_tiles).toContainEqual({ coords: { x: 7, y: 7 }, facing: "up" });
  });

  it("does not classify outdoor Pokecenter signs or entrances as direct heal interactions", () => {
    const snapshot = buildMapInfoSnapshot({
      map: "GoldenrodCity",
      mapGroup: 0,
      mapNumber: 0,
      playerCoords: { x: 33, y: 57 },
      facing: "up",
      overworld: {
        TILES_PER_COLLISION: 2,
        _map_events: {
          warps: [
            {
              index: 1,
              x: 15,
              y: 27,
              target_map_constant: "GOLDENROD_POKECENTER_1F",
              target_map: "GoldenrodPokecenter1F",
              target_warp_id: 1,
            },
          ],
          bg_events: [
            {
              x: 16,
              y: 27,
              event_type: "BGEVENT_READ",
              script: "PokecenterSignScript",
            },
          ],
          coord_events: [],
        },
      },
    });

    expect(snapshot.hotspots.find((hotspot) => hotspot.coords.x === 33 && hotspot.coords.y === 55)).toEqual(
      expect.objectContaining({
        type: "sign",
        label: "Pokecenter sign",
      })
    );
    expect(snapshot.hotspots.find((hotspot) => hotspot.coords.x === 31 && hotspot.coords.y === 55)).toEqual(
      expect.objectContaining({
        type: "warp",
        label: "Warp: Pokecenter",
      })
    );
    expect(snapshot.hotspots.filter((hotspot) => hotspot.type === "heal")).toHaveLength(0);
  });
});

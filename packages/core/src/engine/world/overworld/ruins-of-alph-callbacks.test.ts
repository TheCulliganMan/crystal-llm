import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { build_overworld_map } from "@pokecrystal/core/engine/world/overworld/map-geometry";
import { OverworldMapManagerMixin } from "@pokecrystal/core/engine/world/overworld/overworld-map-manager";
import { OverworldTileset } from "@pokecrystal/core/engine/world/overworld/overworld-tileset";
import { ScriptRunnerImpl } from "@pokecrystal/core/engine/world/story-events/runner";
import { isWarpPermission } from "@pokecrystal/core/engine/world/overworld/tile-events";
import { OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";
import type { MapEvents, WarpEvent } from "@pokecrystal/core/core/models/map";

class TestOverworld extends OverworldMapManagerMixin {}

const stubTileset: OverworldTilesetLike = {
  tilesetName: "test",
  metatiles: [],
  renderMetatile: () => {},
  renderPriorityMetatile: () => {},
};

type ChamberConfig = {
  mapName: string;
  mapConstant: string;
  wallFlag: string;
  solvedFlag: string;
  itemRoomConstant: string;
};

const CHAMBERS: ChamberConfig[] = [
  {
    mapName: "RuinsOfAlphKabutoChamber",
    mapConstant: "RUINS_OF_ALPH_KABUTO_CHAMBER",
    wallFlag: "EVENT_WALL_OPENED_IN_KABUTO_CHAMBER",
    solvedFlag: "EVENT_SOLVED_KABUTO_PUZZLE",
    itemRoomConstant: "RUINS_OF_ALPH_KABUTO_ITEM_ROOM",
  },
  {
    mapName: "RuinsOfAlphOmanyteChamber",
    mapConstant: "RUINS_OF_ALPH_OMANYTE_CHAMBER",
    wallFlag: "EVENT_WALL_OPENED_IN_OMANYTE_CHAMBER",
    solvedFlag: "EVENT_SOLVED_OMANYTE_PUZZLE",
    itemRoomConstant: "RUINS_OF_ALPH_OMANYTE_ITEM_ROOM",
  },
  {
    mapName: "RuinsOfAlphAerodactylChamber",
    mapConstant: "RUINS_OF_ALPH_AERODACTYL_CHAMBER",
    wallFlag: "EVENT_WALL_OPENED_IN_AERODACTYL_CHAMBER",
    solvedFlag: "EVENT_SOLVED_AERODACTYL_PUZZLE",
    itemRoomConstant: "RUINS_OF_ALPH_AERODACTYL_ITEM_ROOM",
  },
  {
    mapName: "RuinsOfAlphHoOhChamber",
    mapConstant: "RUINS_OF_ALPH_HO_OH_CHAMBER",
    wallFlag: "EVENT_WALL_OPENED_IN_HO_OH_CHAMBER",
    solvedFlag: "EVENT_SOLVED_HO_OH_PUZZLE",
    itemRoomConstant: "RUINS_OF_ALPH_HO_OH_ITEM_ROOM",
  },
];

type ChamberContext = {
  overworld: TestOverworld;
  mapEvents: MapEvents;
};

const buildChamberContext = async ({
  config,
  wallOpen,
  puzzleSolved,
}: {
  config: ChamberConfig;
  wallOpen: boolean;
  puzzleSolved: boolean;
}): Promise<ChamberContext> => {
  const dataLoader = new DataLoader();
  dataLoader.ensure_overworld_data({ map_name: config.mapName });
  const attributes = dataLoader.map_attributes.get(config.mapName);
  if (!attributes) {
    throw new Error(`Missing map attributes for ${config.mapName}.`);
  }
  const mapEvents = dataLoader.map_events.get(config.mapName);
  if (!mapEvents) {
    throw new Error(`Missing map events for ${config.mapName}.`);
  }

  const map = build_overworld_map(config.mapName, attributes, { data_loader: dataLoader });
  const tileset = new OverworldTileset(attributes.tileset_name, "day");
  await tileset.ready;

  const gameState = createInitialGameState();
  const eventManager = new EventManager(gameState);
  gameState.wram.event_flags[config.wallFlag] = wallOpen;
  gameState.wram.event_flags[config.solvedFlag] = puzzleSolved;

  const overworld = new TestOverworld();
  overworld.game_state = gameState;
  overworld.data_loader = dataLoader;
  overworld.tileset = tileset;
  overworld.map_surface = null;
  overworld.priority_surface = null;
  overworld.map = map;
  overworld.current_map_name = config.mapName;
  overworld.TILES_PER_COLLISION = 2;
  overworld.player_direction = "down";
  overworld.script_runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
  overworld._set_map_events(mapEvents);

  overworld._run_map_callbacks(config.mapName, "MAPCALLBACK_TILES");
  overworld._refresh_warp_permissions();

  return { overworld, mapEvents };
};

const warpTileCoords = (warp: WarpEvent, stride: number): [number, number] => {
  const offset = Math.max(0, stride - 1);
  return [warp.x * stride + offset, warp.y * stride + offset];
};

const warpPermissionsForTargets = (
  overworld: TestOverworld,
  mapEvents: MapEvents,
  targets: string[],
): Array<[WarpEvent, number | null]> => {
  const stride = overworld.TILES_PER_COLLISION ?? 2;
  const cache = overworld._warp_permission_cache ?? {};
  return (mapEvents.warps ?? [])
    .filter((warp) => targets.includes(warp.target_map_constant))
    .map((warp) => {
      const [x, y] = warpTileCoords(warp, stride);
      const entry = cache[`${x},${y}`]?.[0];
      return [warp, entry?.[1] ?? null];
    });
};

describe("Ruins of Alph chamber callbacks", () => {
  it.each(CHAMBERS)("hides the puzzle hole when unsolved for $mapName", ({ mapName, mapConstant, wallFlag, solvedFlag }) => {
    const dataLoader = new DataLoader();
    dataLoader.ensure_overworld_data({ map_name: mapName });
    const attributes = dataLoader.map_attributes.get(mapName);
    if (!attributes) {
      throw new Error(`Missing map attributes for ${mapName}.`);
    }

    const map = build_overworld_map(mapName, attributes, { data_loader: dataLoader });
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    gameState.wram.event_flags[wallFlag] = false;
    gameState.wram.event_flags[solvedFlag] = false;

    const overworld = new TestOverworld();
    overworld.game_state = gameState;
    overworld.data_loader = dataLoader;
    overworld.tileset = stubTileset;
    overworld.map_surface = null;
    overworld.priority_surface = null;
    overworld.map = map;
    overworld.current_map_name = mapName;
    overworld.script_runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);

    expect(map.getMetatileAt(1, 1)).toBe(0x18);
    expect(map.getMetatileAt(2, 1)).toBe(0x19);

    overworld._run_map_callbacks(mapConstant, "MAPCALLBACK_TILES");

    expect(map.getMetatileAt(1, 1)).toBe(0x01);
    expect(map.getMetatileAt(2, 1)).toBe(0x02);
  });

  it.each(CHAMBERS)("keeps hidden warps inactive on a fresh load for $mapName", async (config) => {
    const { overworld, mapEvents } = await buildChamberContext({
      config,
      wallOpen: false,
      puzzleSolved: false,
    });

    const permissions = warpPermissionsForTargets(overworld, mapEvents, [
      "RUINS_OF_ALPH_INNER_CHAMBER",
      config.itemRoomConstant,
    ]);

    expect(permissions).toHaveLength(3);
    for (const [warp, permission] of permissions) {
      if (permission === null || permission === undefined) {
        throw new Error(`Missing warp permission for ${warp.target_map_constant}.`);
      }
      expect(isWarpPermission(permission)).toBe(false);
    }
  });

  it.each(CHAMBERS)("enables hidden warps after puzzle + wall flags for $mapName", async (config) => {
    const { overworld, mapEvents } = await buildChamberContext({
      config,
      wallOpen: true,
      puzzleSolved: true,
    });

    const permissions = warpPermissionsForTargets(overworld, mapEvents, [
      "RUINS_OF_ALPH_INNER_CHAMBER",
      config.itemRoomConstant,
    ]);

    expect(permissions).toHaveLength(3);
    for (const [warp, permission] of permissions) {
      if (permission === null || permission === undefined) {
        throw new Error(`Missing warp permission for ${warp.target_map_constant}.`);
      }
      expect(isWarpPermission(permission)).toBe(true);
    }
  });

  it("skips warp activation when standing on a non-warp collision", async () => {
    const config = CHAMBERS[0];
    const { overworld, mapEvents } = await buildChamberContext({
      config,
      wallOpen: false,
      puzzleSolved: false,
    });

    const holeWarp = (mapEvents.warps ?? []).find(
      (warp) => warp.target_map_constant === "RUINS_OF_ALPH_INNER_CHAMBER",
    );
    if (!holeWarp) {
      throw new Error("Missing chamber hole warp event.");
    }

    const [tileX, tileY] = warpTileCoords(holeWarp, overworld.TILES_PER_COLLISION ?? 2);
    overworld.player_x = tileX;
    overworld.player_y = tileY;
    overworld._warp_cooldown = 0;
    overworld._active_warp_tile = null;
    const activateSpy = jest.fn();
    (overworld as unknown as { _activate_warp: jest.Mock })._activate_warp = activateSpy;
    (overworld as unknown as { _play_warp_sound: jest.Mock })._play_warp_sound = jest.fn();

    expect(overworld.check_for_warp_event()).toBe(false);
    expect(activateSpy).not.toHaveBeenCalled();
  });

  it("activates the chamber hole warp immediately after hole tiles are written", async () => {
    const config = CHAMBERS[0];
    const { overworld, mapEvents } = await buildChamberContext({
      config,
      wallOpen: true,
      puzzleSolved: false,
    });

    const holeWarp = (mapEvents.warps ?? []).find(
      (warp) => warp.target_map_constant === "RUINS_OF_ALPH_INNER_CHAMBER",
    );
    if (!holeWarp) {
      throw new Error("Missing chamber hole warp event.");
    }

    const [tileX, tileY] = warpTileCoords(holeWarp, overworld.TILES_PER_COLLISION ?? 2);
    overworld.player_x = tileX;
    overworld.player_y = tileY;
    overworld._warp_cooldown = 0;
    overworld._active_warp_tile = null;
    const activateSpy = jest.fn();
    (overworld as unknown as { _activate_warp: jest.Mock })._activate_warp = activateSpy;
    (overworld as unknown as { _play_warp_sound: jest.Mock })._play_warp_sound = jest.fn();

    expect(overworld.check_for_warp_event()).toBe(false);
    expect(activateSpy).not.toHaveBeenCalled();

    // ASM puzzle-complete flow writes the two hole metatiles before warpcheck.
    overworld._write_metatile(1, 1, 0x18);
    overworld._write_metatile(2, 1, 0x19);

    expect(overworld.check_for_warp_event()).toBe(true);
    expect(activateSpy).toHaveBeenCalledTimes(1);
  });
});

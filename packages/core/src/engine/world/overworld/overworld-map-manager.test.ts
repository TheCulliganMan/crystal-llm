import { EmoteSurfaceCache, OverworldMapManagerMixin } from "./overworld-map-manager";
import { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import { OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { OverworldObject } from "./overworld-object";
import type { ObjectEvent } from "@pokecrystal/core/core/models/map";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { getMapMetadataByConstant, getMapMetadataByName } from "@pokecrystal/core/engine/world/maps";
import { resolveCollisionValue } from "@pokecrystal/core/engine/world/overworld/collision-data";
import { isWarpPermission } from "@pokecrystal/core/engine/world/overworld/tile-events";
import { OverworldTileset } from "@pokecrystal/core/engine/world/overworld/overworld-tileset";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { getBooleanFlag, setBooleanFlag } from "@pokecrystal/core/engine/world/overworld/flag-collection";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import type { RenderMetatileOptions } from "@pokecrystal/core/engine/world/overworld/tileset-types";
import { resolveNpcDataList } from "@pokecrystal/core/engine/world/overworld/overworld-npc-manager";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { ScriptRunnerImpl } from "@pokecrystal/core/engine/world/story-events/runner";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import type { BaseFontRenderer } from "@pokecrystal/core/ui/base-ui";
import type { Surface } from "@pokecrystal/core/ui/surface";

jest.mock("@pokecrystal/core/engine/world/overworld/map-geometry", () => {
  const actual = jest.requireActual("@pokecrystal/core/engine/world/overworld/map-geometry");
  return {
    __esModule: true,
    ...actual,
    create_map_surface: jest.fn(() => null),
    create_priority_surface: jest.fn(() => null),
  };
});

const mapGeometry = jest.requireMock("@pokecrystal/core/engine/world/overworld/map-geometry") as {
  create_map_surface: jest.Mock;
  create_priority_surface: jest.Mock;
};

class TestOverworld extends OverworldMapManagerMixin {}

class TestBackgroundEventOverworld extends OverworldMapManagerMixin {
  public bgEventAt(tileX: number, tileY: number) {
    return this._bg_event_at(tileX, tileY);
  }
}

class TestInteractionOverworld extends OverworldMapManagerMixin {
  public facingCoords: [number, number] = [0, 0];
  public script_runner: { is_busy?: boolean; run?: jest.Mock; last_interaction_object_index?: number | null } | null =
    null;

  public get_facing_tile_coords(): [number, number] {
    return this.facingCoords;
  }

  public _counter_adjusted_tile(x: number, y: number): [number, number] {
    return [x, y];
  }

  public _play_interaction_sound(): void {
    // Test stub.
  }
}

class TestCounterInteractionOverworld extends OverworldMapManagerMixin {
  public facingCoords: [number, number] = [0, 0];
  public script_runner: { is_busy?: boolean; run?: jest.Mock; last_interaction_object_index?: number | null } | null =
    null;

  public get_facing_tile_coords(): [number, number] {
    return this.facingCoords;
  }

  public _counter_adjusted_tile(x: number, y: number): [number, number] {
    return OverworldEngine.prototype._counter_adjusted_tile.call(
      this as unknown as OverworldEngine,
      x,
      y
    );
  }

  public _play_interaction_sound(): void {
    // Test stub.
  }
}

const createTestMap = (width: number, height: number): OverworldMap => {
  const metatileIds = Array(width * height).fill(0);
  return {
    mapName: `test-map-${width}x${height}`,
    width,
    height,
    dataLoader: null,
    metatileIds,
    getMetatileAt(x: number, y: number): number {
      if (x < 0 || x >= width || y < 0 || y >= height) {
        throw new Error("Metatile lookup out of range.");
      }
      return metatileIds[y * width + x];
    },
  } as OverworldMap;
};

const stubTileset: OverworldTilesetLike = {
  tilesetName: "test",
  metatiles: [],
  renderMetatile: () => {},
  renderPriorityMetatile: () => {},
};

const buildEvent = (label: string): ObjectEvent => ({
  sprite: "SPRITE_TEST",
  x: 0,
  y: 0,
  spritemovedata: "",
  move_range_x: 0,
  move_range_y: 0,
  hram_x: 0,
  hram_y: 0,
  pal: 0,
  object_type: "OBJECTTYPE_SCRIPT",
  radius: 0,
  script: label,
  label,
  event_flag: "",
  object_identifier: null,
  sightline_direction_override: null,
});

const buildTextUi = (): TextUI & {
  tile_size?: number;
  font: BaseFontRenderer;
} => {
  const ui = new TextUI(160, 144, 1, null, false, 0) as TextUI & {
    tile_size?: number;
    font: BaseFontRenderer;
  };
  const fontTiles: Record<number, Surface> = {};
  for (let i = 0; i < 256; i += 1) {
    fontTiles[i] = new gameEngine.Surface(8, 8) as unknown as Surface;
  }
  ui.tile_size = 8;
  ui.font.font_tiles = fontTiles;
  const noopRender: (..._args: Parameters<NonNullable<BaseFontRenderer["renderText"]>>) => void = () => {};
  ui.font.render_text = noopRender;
  ui.font.renderText = noopRender;
  return ui;
};

describe("OverworldMapManagerMixin map scenes", () => {
  it("defers a new map scene while a warp script is still unwinding", () => {
    const overworld = new TestOverworld();
    const gameState = createInitialGameState();
    const defer = jest.fn();
    const run = jest.fn();

    overworld.current_map_name = "FastShip1F";
    overworld.game_state = gameState;
    overworld.data_loader = {
      map_scene_scripts: new Map([
        [
          "FastShip1F",
          {
            SCENE_FASTSHIP1F_ENTER_SHIP: "FastShip1FEnterShipScene",
          },
        ],
      ]),
    } as unknown as DataLoader;
    overworld.script_runner = {
      _script_stack: [{ name: "OlivinePortSailorAtGangwayScript" }],
      _awaiting_resume: 0,
      _ensure_map_scene_initialized: jest.fn(() => [
        "SCENE_FASTSHIP1F_ENTER_SHIP",
        1,
      ]),
      defer,
      run,
    } as unknown as ScriptRunnerImpl;

    overworld._run_map_scene();

    expect(defer).toHaveBeenCalledWith("FastShip1FEnterShipScene");
    expect(run).not.toHaveBeenCalled();
  });

  it("runs a map scene immediately when no script is active", () => {
    const overworld = new TestOverworld();
    const gameState = createInitialGameState();
    const defer = jest.fn();
    const run = jest.fn();

    overworld.current_map_name = "FastShip1F";
    overworld.game_state = gameState;
    overworld.data_loader = {
      map_scene_scripts: new Map([
        [
          "FastShip1F",
          {
            SCENE_FASTSHIP1F_ENTER_SHIP: "FastShip1FEnterShipScene",
          },
        ],
      ]),
    } as unknown as DataLoader;
    overworld.script_runner = {
      _script_stack: [],
      _awaiting_resume: 0,
      _ensure_map_scene_initialized: jest.fn(() => [
        "SCENE_FASTSHIP1F_ENTER_SHIP",
        1,
      ]),
      defer,
      run,
    } as unknown as ScriptRunnerImpl;

    overworld._run_map_scene();

    expect(run).toHaveBeenCalledWith("FastShip1FEnterShipScene");
    expect(defer).not.toHaveBeenCalled();
  });
});

class SpyAudioEngine extends AudioEngine {
  constructor(private readonly spy: (name: string) => void) {
    super({ masterVolume: 0, muted: true });
  }

  playSound(name: string): void {
    this.spy(name);
  }
}

describe("OverworldMapManagerMixin._write_metatile", () => {
  it("writes metatile IDs without requiring field move mixins", () => {
    const overworld = new TestOverworld();
    overworld.map = createTestMap(2, 2);
    overworld.tileset = stubTileset;
    overworld.map_surface = null;
    overworld.priority_surface = null;

    overworld._write_metatile(1, 0, 0x12);

    expect(overworld.map.metatileIds[1]).toBe(0x12);
  });
});

describe("OverworldMapManagerMixin bike flag reset", () => {
  it("clears bike, downhill, and strength flags before map callbacks", () => {
    const overworld = new TestOverworld();
    overworld.game_state = createInitialGameState();
    setBooleanFlag(overworld.game_state.wram.engine_flags, "ENGINE_STRENGTH_ACTIVE", true);
    setBooleanFlag(overworld.game_state.wram.engine_flags, "ENGINE_ALWAYS_ON_BIKE", true);
    setBooleanFlag(overworld.game_state.wram.engine_flags, "ENGINE_DOWNHILL", true);
    overworld.game_state.wram.wBikeFlags = 0x07;

    (overworld as unknown as { _reset_bike_flags_for_new_map: () => void })._reset_bike_flags_for_new_map();

    expect(getBooleanFlag(overworld.game_state.wram.engine_flags, "ENGINE_STRENGTH_ACTIVE")).toBe(false);
    expect(getBooleanFlag(overworld.game_state.wram.engine_flags, "ENGINE_ALWAYS_ON_BIKE")).toBe(false);
    expect(getBooleanFlag(overworld.game_state.wram.engine_flags, "ENGINE_DOWNHILL")).toBe(false);
    expect(overworld.game_state.wram.wBikeFlags).toBe(0);
  });

  it("installs the destination tileset before running tile callbacks", () => {
    const dataLoader = new DataLoader();
    dataLoader.ensure_overworld_data({ map_name: "Route19FuchsiaGate" });
    dataLoader.ensure_overworld_data({ map_name: "Route19" });

    const overworld = new TestOverworld();
    overworld.data_loader = dataLoader;
    overworld.game_state = createInitialGameState();
    overworld.event_manager = null;
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => true;
    (overworld as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };

    const callbackTilesets: string[] = [];
    overworld.script_runner = {
      run: jest.fn((scriptName: string) => {
        if (scriptName === "Route19ClearRocksCallback") {
          callbackTilesets.push(overworld.tileset?.tilesetName ?? "missing");
        }
      }),
    } as any;

    overworld.load_map("Route19FuchsiaGate");
    expect(overworld.tileset?.tilesetName).toBe("gate");

    overworld.load_map("Route19");

    expect(callbackTilesets).toEqual(["kanto"]);
  });
});

describe("OverworldMapManagerMixin audio guards", () => {
  it("does not play emote sounds without explicit ASM cues", () => {
    const overworld = new TestOverworld();
    const playSound = jest.fn();
    overworld.audio_engine = new SpyAudioEngine(playSound);
    const npc = new OverworldObject(buildEvent("NPC"));

    expect(() => overworld.show_emote("shock", npc, 5)).not.toThrow();
    expect(playSound).not.toHaveBeenCalled();
  });

  it("plays warp sounds via playSound when play_sound is missing", () => {
    const overworld = new TestOverworld();
    const playSound = jest.fn();
    overworld.audio_engine = new SpyAudioEngine(playSound);

    const warp = overworld as unknown as { _play_warp_sound: (permission: number | null) => void };
    expect(() => warp._play_warp_sound(0x01)).not.toThrow();
    expect(playSound).toHaveBeenCalledWith("SFX_EXIT_BUILDING");
  });
});

describe("EmoteSurfaceCache", () => {
  let originalLoadSync: typeof gameEngine.image.loadSync | undefined;

  beforeEach(() => {
    originalLoadSync = gameEngine.image.loadSync;
  });

  afterEach(() => {
    gameEngine.image.loadSync = originalLoadSync;
  });

  it("loads and caches a real emote surface instead of returning a placeholder", () => {
    const source = new gameEngine.Surface(16, 16);
    source.fill([255, 255, 255, 255]);
    source.set_at([1, 1], [170, 170, 170, 255]);
    source.set_at([2, 1], [0, 0, 0, 255]);
    const loadSync = jest.fn((path: string) => {
      expect(path).toBe(getAssetPath("gfx", "emotes", "shock.png"));
      return source;
    });
    gameEngine.image.loadSync = loadSync;

    const cache = new EmoteSurfaceCache();
    const surface = cache.get_surface("shock");

    expect(surface).toBe(source);
    expect(surface.get_size()).toEqual([16, 16]);
    expect(surface.get_at([0, 0])).toEqual([255, 255, 255, 0]);
    expect(surface.get_at([1, 1])).toEqual([170, 170, 170, 255]);
    expect(surface.get_at([2, 1])).toEqual([0, 0, 0, 255]);
    expect(loadSync).toHaveBeenCalledTimes(1);
    expect(cache.get_surface("EMOTE_SHOCK")).toBe(surface);
    expect(loadSync).toHaveBeenCalledTimes(1);
  });

  it("throws when an emote surface is unavailable", () => {
    gameEngine.image.loadSync = jest.fn(() => null);

    const cache = new EmoteSurfaceCache();

    expect(() => cache.get_surface("shock")).toThrow("Missing emote surface");
  });
});

describe("OverworldMapManagerMixin.check_for_coord_events", () => {
  it("disables event flag refresh while running coord event scripts", () => {
    const overworld = new TestOverworld();
    overworld.game_state = createInitialGameState();
    overworld.TILES_PER_COLLISION = 2;
    overworld.current_map_name = "PLAYERS_HOUSE_1F";
    overworld.player_x = 3;
    overworld.player_y = 5;
    overworld.script_tasks_active = jest.fn().mockReturnValue(false);
    overworld._map_events = {
      warps: [],
      coord_events: [
        {
          x: 1,
          y: 2,
          scene_id: "",
          script_name: "MeetMomRightScript",
        },
      ],
      bg_events: [],
    };

    const runner = {
      allow_event_flag_refresh: true,
      _script_stack: [],
      _awaiting_resume: 0,
      run: jest.fn(),
    };
    runner.run.mockImplementation(() => {
      runner.allow_event_flag_refresh = false;
      expect(runner.allow_event_flag_refresh).toBe(false);
    });
    overworld.script_runner = runner as any;

    expect(overworld.check_for_coord_events()).toBe(true);
    expect(runner.run).toHaveBeenCalledWith("MeetMomRightScript");
    expect(runner.allow_event_flag_refresh).toBe(false);
  });
});

describe("OverworldMapManagerMixin._repair_story_scene_state", () => {
  it("throws on out-of-range scene indices instead of repairing them", () => {
    const overworld = new TestOverworld();
    overworld.game_state = createInitialGameState();
    overworld.data_loader = {
      map_scene_order: new Map([
        ["TestMap", ["SCENE_TESTMAP_DEFAULT", "SCENE_TESTMAP_EVENT", "SCENE_TESTMAP_NOOP"]],
      ]),
    } as unknown as DataLoader;
    overworld.script_runner = {
      _normalise_map_name: (map: string) => map,
      _set_map_scene: jest.fn((map: string, scene: string) => {
        overworld.game_state.wram.map_scenes[map] = scene;
      }),
    } as any;

    overworld.game_state.wram.map_scene_indices.TestMap = 99;

    expect(() => overworld._repair_story_scene_state("TestMap")).toThrow(
      "Map scene index 99 for 'TestMap' exceeds canonical scene order length 3."
    );
  });
});

describe("OverworldMapManagerMixin._activate_warp", () => {
  it("uses the previous warp id when resolving dynamic pokecenter 2F exits", () => {
    const overworld = new TestOverworld();
    const gameState = createInitialGameState();
    const center2f = getMapMetadataByConstant("POKECENTER_2F");
    const goldenrod1f = getMapMetadataByConstant("GOLDENROD_POKECENTER_1F");
    if (!center2f || !goldenrod1f) {
      throw new Error("Missing map metadata required for dynamic warp test.");
    }

    overworld.game_state = gameState;
    overworld.data_loader = {
      map_events: new Map([
        [
          goldenrod1f.name,
          {
            warps: [
              {
                index: 1,
                x: 7,
                y: 7,
                target_map_constant: "GOLDENROD_CITY",
                target_map: "GoldenrodCity",
                target_warp_id: 1,
              },
              {
                index: 2,
                x: 0,
                y: 7,
                target_map_constant: "POKECENTER_2F",
                target_map: "Pokecenter2F",
                target_warp_id: 1,
              },
            ],
            coord_events: [],
            bg_events: [],
          },
        ],
      ]),
      ensure_map_scripts: jest.fn(),
    } as any;
    overworld.TILES_PER_COLLISION = 2;
    overworld.current_map_name = center2f.name;
    overworld.load_map = jest.fn((mapName: string) => {
      overworld.current_map_name = mapName;
    });

    gameState.wram.wMapGroup = center2f.groupId;
    gameState.wram.wMapNumber = center2f.mapId;
    gameState.wram.wPrevMapGroup = goldenrod1f.groupId;
    gameState.wram.wPrevMapNumber = goldenrod1f.mapId;
    gameState.wram.wPrevWarp = 2;
    gameState.wram.wBackupMapGroup = goldenrod1f.groupId;
    gameState.wram.wBackupMapNumber = goldenrod1f.mapId;
    gameState.wram.wBackupWarpNumber = 0;

    const warp = {
      index: 1,
      x: 0,
      y: 7,
      target_map_constant: "POKECENTER_2F",
      target_map: "Pokecenter2F",
      target_warp_id: -1,
    };

    (overworld as any)._activate_warp(warp, null);

    expect(gameState.wram.wBackupWarpNumber).toBe(2);
    expect(overworld.player_x).toBe(1);
    expect(overworld.player_y).toBe(15);
  });

  it("preserves the selected elevator destination until the player exits", () => {
    const overworld = new TestOverworld();
    const gameState = createInitialGameState();
    const elevator = getMapMetadataByConstant("GOLDENROD_DEPT_STORE_ELEVATOR");
    const origin = getMapMetadataByConstant("GOLDENROD_DEPT_STORE_1F");
    const destination = getMapMetadataByConstant("GOLDENROD_DEPT_STORE_2F");
    if (!elevator || !origin || !destination) {
      throw new Error("Missing Goldenrod Dept. Store metadata required for elevator dynamic warp test.");
    }

    overworld.game_state = gameState;
    overworld.data_loader = {
      map_events: new Map([
        [
          destination.name,
          {
            warps: [
              {
                index: 1,
                x: 9,
                y: 1,
                target_map_constant: "GOLDENROD_DEPT_STORE_ELEVATOR",
                target_map: elevator.name,
                target_warp_id: 1,
              },
              {
                index: 2,
                x: 8,
                y: 1,
                target_map_constant: "GOLDENROD_DEPT_STORE_ELEVATOR",
                target_map: elevator.name,
                target_warp_id: 1,
              },
              {
                index: 3,
                x: 2,
                y: 0,
                target_map_constant: "GOLDENROD_DEPT_STORE_ELEVATOR",
                target_map: elevator.name,
                target_warp_id: 1,
              },
            ],
            coord_events: [],
            bg_events: [],
          },
        ],
      ]),
      ensure_map_scripts: jest.fn(),
    } as any;
    overworld.TILES_PER_COLLISION = 2;
    overworld.current_map_name = elevator.name;
    overworld.load_map = jest.fn((mapName: string) => {
      overworld.current_map_name = mapName;
    });

    gameState.wram.wMapGroup = elevator.groupId;
    gameState.wram.wMapNumber = elevator.mapId;
    gameState.wram.wPrevMapGroup = origin.groupId;
    gameState.wram.wPrevMapNumber = origin.mapId;
    gameState.wram.wPrevWarp = 4;
    gameState.wram.wBackupMapGroup = destination.groupId;
    gameState.wram.wBackupMapNumber = destination.mapId;
    gameState.wram.wBackupWarpNumber = 3;

    const exitWarp = {
      index: 1,
      x: 7,
      y: 3,
      target_map_constant: "GOLDENROD_DEPT_STORE_1F",
      target_map: origin.name,
      target_warp_id: -1,
    };

    (overworld as any)._activate_warp(exitWarp, null);

    expect(overworld.load_map).toHaveBeenCalledWith(destination.name);
    expect(gameState.wram.wBackupMapGroup).toBe(destination.groupId);
    expect(gameState.wram.wBackupMapNumber).toBe(destination.mapId);
    expect(gameState.wram.wBackupWarpNumber).toBe(3);
    expect(overworld.player_x).toBe(5);
    expect(overworld.player_y).toBe(1);
  });
});

describe("OverworldMapManagerMixin live story map interaction parity", () => {
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

  it.each(pokecenterNurseCases)("can interact with the %s nurse through the real counter tile", async (mapName, scriptName) => {
    const dataLoader = new DataLoader();
    dataLoader.ensure_overworld_data({ map_name: mapName });

    const overworld = new TestCounterInteractionOverworld();
    overworld.data_loader = dataLoader;
    overworld.game_state = createInitialGameState();
    overworld.event_manager = null;
    overworld.script_runner = { is_busy: false, run: jest.fn(), last_interaction_object_index: null } as any;
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => true;
    (overworld as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };

    overworld.load_map(mapName);
    await overworld.tileset?.ready;
    await Promise.resolve();

    const nurse = overworld.npcs.find((npc) => npc.event.script === scriptName);
    if (!nurse) {
      throw new Error(`Missing nurse ${scriptName} on ${mapName}`);
    }
    overworld.player_x = nurse.x;
    overworld.player_y = nurse.y + 4;
    overworld.player_direction = "up";
    overworld.facingCoords = [nurse.x, nurse.y + 2];

    expect(
      overworld.npcs.map((npc) => ({
        objectIndex: npc.objectIndex,
        x: npc.x,
        y: npc.y,
        script: npc.event.script,
      }))
    ).toContainEqual({
      objectIndex: nurse.objectIndex,
      x: nurse.x,
      y: nurse.y,
      script: scriptName,
    });
    expect(overworld.check_for_npc_interaction()).toBe(true);
    expect(overworld.game_state.wram.last_talked).toBe(nurse.objectIndex);
    expect(overworld.script_runner?.run).toHaveBeenCalledWith(scriptName);
  });

  it("can interact with the Goldenrod Game Corner coin vendor through the real counter tile", async () => {
    const dataLoader = new DataLoader();
    const mapName = "GoldenrodGameCorner";
    dataLoader.ensure_overworld_data({ map_name: mapName });

    const overworld = new TestCounterInteractionOverworld();
    overworld.data_loader = dataLoader;
    overworld.game_state = createInitialGameState();
    overworld.event_manager = null;
    overworld.script_runner = { is_busy: false, run: jest.fn(), last_interaction_object_index: null } as any;
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => true;
    (overworld as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };

    overworld.load_map(mapName);
    await overworld.tileset?.ready;
    await Promise.resolve();

    const vendor = overworld.npcs.find((npc) => npc.event.script === "GoldenrodGameCornerCoinVendorScript");
    if (!vendor) {
      throw new Error("Missing Goldenrod Game Corner coin vendor");
    }
    overworld.player_x = vendor.x - 1;
    overworld.player_y = vendor.y + 4;
    overworld.player_direction = "up";
    overworld.facingCoords = [vendor.x - 1, vendor.y + 2];

    expect(overworld.check_for_npc_interaction()).toBe(true);
    expect(overworld.game_state.wram.last_talked).toBe(vendor.objectIndex);
    expect(overworld.script_runner?.run).toHaveBeenCalledWith("GoldenrodGameCornerCoinVendorScript");
  });

  it("can interact with the Goldenrod Game Corner pharmacist through the side counter", async () => {
    const dataLoader = new DataLoader();
    const mapName = "GoldenrodGameCorner";
    dataLoader.ensure_overworld_data({ map_name: mapName });

    const overworld = new TestCounterInteractionOverworld();
    overworld.data_loader = dataLoader;
    overworld.game_state = createInitialGameState();
    overworld.event_manager = null;
    overworld.script_runner = { is_busy: false, run: jest.fn(), last_interaction_object_index: null } as any;
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => true;
    (overworld as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };

    overworld.load_map(mapName);
    await overworld.tileset?.ready;
    await Promise.resolve();

    const pharmacist = overworld.npcs.find((npc) => npc.event.script === "GoldenrodGameCornerPharmacistScript");
    if (!pharmacist) {
      throw new Error("Missing Goldenrod Game Corner pharmacist");
    }
    overworld.player_x = pharmacist.x - 4;
    overworld.player_y = pharmacist.y;
    overworld.player_direction = "right";
    overworld.facingCoords = [pharmacist.x - 2, pharmacist.y];

    expect(overworld.check_for_npc_interaction()).toBe(true);
    expect(overworld.game_state.wram.last_talked).toBe(pharmacist.objectIndex);
    expect(overworld.script_runner?.run).toHaveBeenCalledWith("GoldenrodGameCornerPharmacistScript");
  });

  it("resolves Goldenrod Game Corner slot machines at their live interaction tile", async () => {
    const dataLoader = new DataLoader();
    const mapName = "GoldenrodGameCorner";
    dataLoader.ensure_overworld_data({ map_name: mapName });

    const overworld = new TestBackgroundEventOverworld();
    overworld.data_loader = dataLoader;
    overworld.game_state = createInitialGameState();
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => true;
    (overworld as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };

    overworld.load_map(mapName);
    await overworld.tileset?.ready;
    await Promise.resolve();

    expect(overworld.bgEventAt(25, 13)).toEqual(
      expect.objectContaining({
        event_type: "BGEVENT_READ",
        script: "GoldenrodGameCornerSlotsMachineScript",
      })
    );
    expect(overworld.bgEventAt(15, 15)).toEqual(
      expect.objectContaining({
        event_type: "BGEVENT_READ",
        script: "GoldenrodGameCornerLuckySlotsMachineScript",
      })
    );
    expect(overworld.bgEventAt(37, 17)).toEqual(
      expect.objectContaining({
        event_type: "BGEVENT_READ",
        script: "GoldenrodGameCornerCardFlipMachineScript",
      })
    );
  });

  it("starts the Goldenrod Game Corner slot machine from the live facing tile", async () => {
    const dataLoader = new DataLoader();
    dataLoader.Tileset = OverworldTileset;
    const gameState = createInitialGameState();
    gameState.sram.key_items.COIN_CASE = 1;
    gameState.sram.coins = 10;
    const metadata = getMapMetadataByName("GoldenrodGameCorner");
    if (!metadata) {
      throw new Error("Missing GoldenrodGameCorner metadata.");
    }
    gameState.wram.wMapGroup = metadata.groupId;
    gameState.wram.wMapNumber = metadata.mapId;

    const engine = new OverworldEngine(
      gameState,
      dataLoader,
      new EventManager(gameState),
      new OverworldTileset("johto", "day"),
      new AudioEngine({ masterVolume: 0, muted: true }),
      buildTextUi(),
      { suppressInitialMapEntryEffects: true, suppressInitialMapMusic: true }
    );
    (engine as any)._sprite_asset_exists = () => true;
    (engine as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };
    (engine as any)._tileset_animator = null;
    engine.load_map("GoldenrodGameCorner");
    await engine.tileset?.ready;
    await Promise.resolve();

    engine.player_x = 25;
    engine.player_y = 9;
    engine.player_direction = "down";

    expect(() => engine.handle_a_button()).not.toThrow();
    expect(engine.script_runner.last_value).toEqual(
      expect.objectContaining({
        played: true,
        bet: expect.any(Number),
        coins: expect.any(Number),
      })
    );
    expect(gameState.sram.coins).toBe((engine.script_runner.last_value as { coins: number }).coins);
  });

  it("can interact with Mr. Pokemon from the real house anchor tile", async () => {
    const dataLoader = new DataLoader();
    const mapName = "MrPokemonsHouse";
    dataLoader.ensure_overworld_data({ map_name: mapName });
    const npcEvents = resolveNpcDataList(dataLoader, mapName);
    expect(npcEvents.length).toBeGreaterThan(0);

    const overworld = new TestInteractionOverworld();
    overworld.data_loader = dataLoader;
    overworld.game_state = createInitialGameState();
    overworld.event_manager = null;
    overworld.script_runner = { is_busy: false, run: jest.fn(), last_interaction_object_index: null } as any;
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => true;
    (overworld as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };

    const npc = new OverworldObject(npcEvents[0]);
    npc.objectIndex = 1;
    (overworld as any)._apply_variable_sprite(npc);
    (overworld as any)._initialise_object_coordinates(npc);
    expect((overworld as any)._object_should_spawn(npc, { ignore_event_flag: false })).toBe(true);

    overworld.load_map(mapName);
    await overworld.tileset?.ready;
    await Promise.resolve();

    overworld.player_x = 7;
    overworld.player_y = 13;
    overworld.player_direction = "up";
    overworld.facingCoords = [7, 11];

    expect(
      overworld.npcs.map((npc) => ({
        objectIndex: npc.objectIndex,
        x: npc.x,
        y: npc.y,
        prevX: npc.prevX,
        prevY: npc.prevY,
        script: npc.event.script,
      }))
    ).toContainEqual({
      objectIndex: 1,
      x: 7,
      y: 11,
      prevX: 7,
      prevY: 11,
      script: "MrPokemonsHouse_MrPokemonScript",
    });
    expect(overworld.check_for_npc_interaction()).toBe(true);
    expect(overworld.game_state.wram.last_talked).toBe(1);
    expect(overworld.script_runner?.run).toHaveBeenCalledWith("MrPokemonsHouse_MrPokemonScript");
  });

  it("starts the real Mr. Pokemon script from the real house anchor tile", async () => {
    const dataLoader = new DataLoader();
    const mapName = "MrPokemonsHouse";
    dataLoader.ensure_overworld_data({ map_name: mapName });

    const overworld = new TestInteractionOverworld();
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const observedEvents: string[] = [];
    gameState.wram.map_scenes[mapName] = "SCENE_MRPOKEMONSHOUSE_NOOP";
    gameState.wram.map_scene_indices[mapName] = 1;

    for (const name of ["open_text", "show_text", "wait_for_input"]) {
      eventManager.on(name, (event) => observedEvents.push(event.name));
    }

    overworld.data_loader = dataLoader;
    overworld.game_state = gameState;
    overworld.event_manager = eventManager;
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => true;
    (overworld as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };

    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld as any);
    overworld.script_runner = runner as any;

    overworld.load_map(mapName);
    await overworld.tileset?.ready;
    await Promise.resolve();

    overworld.player_x = 7;
    overworld.player_y = 13;
    overworld.player_direction = "up";
    (overworld as any).player_object = {
      x: 7,
      y: 13,
      prevX: 7,
      prevY: 13,
      direction: "up",
      updatePixelPosition: jest.fn(),
    };
    overworld.facingCoords = [7, 11];

    expect(overworld.check_for_npc_interaction()).toBe(true);
    expect(gameState.wram.last_talked).toBe(1);
    expect(observedEvents).toContain("open_text");
    expect(observedEvents).toContain("show_text");
    expect(observedEvents).toContain("wait_for_input");
  });

  it("starts the real Cyndaquil starter script from Elm's Lab counter lane", async () => {
    const dataLoader = new DataLoader();
    const mapName = "ElmsLab";
    dataLoader.ensure_overworld_data({ map_name: mapName });

    const overworld = new TestInteractionOverworld();
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const observedEvents: string[] = [];
    gameState.wram.map_scenes[mapName] = "SCENE_ELMSLAB_NOOP";
    gameState.wram.map_scene_indices[mapName] = 3;

    for (const name of ["show_pokepic", "open_text", "show_text", "wait_for_input"]) {
      eventManager.on(name, (event) => observedEvents.push(event.name));
    }

    overworld.data_loader = dataLoader;
    overworld.game_state = gameState;
    overworld.event_manager = eventManager;
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => true;
    (overworld as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };

    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld as any);
    overworld.script_runner = runner as any;

    overworld.load_map(mapName);
    await overworld.tileset?.ready;
    await Promise.resolve();
    (overworld as any).pokepic_overlay = { show: jest.fn(), hide: jest.fn(), isVisible: false };

    overworld.player_x = 13;
    overworld.player_y = 9;
    overworld.player_direction = "up";
    (overworld as any).player_object = {
      x: 13,
      y: 9,
      prevX: 13,
      prevY: 9,
      direction: "up",
      updatePixelPosition: jest.fn(),
    };
    overworld.facingCoords = [13, 7];

    expect(() => overworld.check_for_npc_interaction()).toThrow(
      "YesOrNoCommand requires an overworld dialogue controller."
    );
    expect(gameState.wram.last_talked).toBe(3);
    expect((overworld as any).pokepic_overlay.show).toHaveBeenCalledWith("CYNDAQUIL");
    expect(observedEvents).toContain("open_text");
    expect(observedEvents).toContain("show_text");
    expect(observedEvents).toContain("wait_for_input");
  });

  it("does not write the ASM -1 event flag sentinel when removing unflagged objects", () => {
    const overworld = new TestOverworld();
    const gameState = createInitialGameState();
    const event = {
      ...buildEvent("CianwoodCityChucksWife"),
      event_flag: "-1",
      object_identifier: "CIANWOODCITY_POKEFAN_F",
    } as ObjectEvent;
    const npc = new OverworldObject(event);
    npc.objectIndex = 10;

    overworld.current_map_name = "CianwoodCity";
    overworld.game_state = gameState;
    overworld.npcs = [npc];
    overworld._npc_index_lookup = new Map([[10, npc]]);
    overworld._npc_blueprints = new Map([
      ["CianwoodCity", new Map([["CIANWOODCITY_POKEFAN_F", [event, 10]]])],
    ]);

    overworld.remove_object("CIANWOODCITY_POKEFAN_F");

    expect(overworld.npcs).toEqual([]);
    expect(gameState.wram.event_flags["-1"]).toBeUndefined();
  });

  it("loads real CianwoodCity NPCs when state already contains the -1 sentinel flag", async () => {
    const dataLoader = new DataLoader();
    const mapName = "CianwoodCity";
    dataLoader.ensure_overworld_data({ map_name: mapName });

    const overworld = new TestOverworld();
    const gameState = createInitialGameState();
    gameState.wram.event_flags["-1"] = true;
    overworld.data_loader = dataLoader;
    overworld.game_state = gameState;
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => true;
    (overworld as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };

    overworld.load_map(mapName);
    await overworld.tileset?.ready;
    await Promise.resolve();

    expect(overworld.npcs.length).toBeGreaterThanOrEqual(10);
    expect(overworld.npcs.map((npc) => npc.event.script)).toContain("CianwoodCityChucksWife");
    expect(overworld.npcs.map((npc) => npc.constantId)).toContain("CIANWOODCITY_POKEFAN_F");
  });

  it("keeps Vermilion City's Big Snorlax blocking Diglett's Cave until it is fought", async () => {
    const dataLoader = new DataLoader();
    dataLoader.Tileset = OverworldTileset;
    const gameState = createInitialGameState();
    gameState.wram.event_flags.EVENT_INITIALIZED_EVENTS = true;
    gameState.wram.event_flags.EVENT_FOUGHT_SNORLAX = false;
    gameState.wram.event_flags.EVENT_VERMILION_CITY_SNORLAX = false;
    const metadata = getMapMetadataByName("VermilionCity");
    if (!metadata) {
      throw new Error("Missing VermilionCity metadata.");
    }
    gameState.wram.wMapGroup = metadata.groupId;
    gameState.wram.wMapNumber = metadata.mapId;

    const engine = new OverworldEngine(
      gameState,
      dataLoader,
      new EventManager(gameState),
      new OverworldTileset("johto", "day"),
      new AudioEngine({ masterVolume: 0, muted: true }),
      buildTextUi(),
      { suppressInitialMapEntryEffects: true, suppressInitialMapMusic: true }
    );
    (engine as any)._sprite_asset_exists = () => true;
    (engine as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };
    (engine as any)._tileset_animator = null;
    engine.load_map("VermilionCity");
    await engine.tileset?.ready;
    await Promise.resolve();

    const snorlax = engine.npcs.find((npc) => npc.constantId === "VERMILIONCITY_BIG_SNORLAX");
    if (!snorlax) {
      throw new Error("Missing VermilionCity Big Snorlax.");
    }
    expect(snorlax.spriteConstant).toBe("SPRITE_BIG_SNORLAX");
    expect(snorlax.event.spritemovedata).toBe("SPRITEMOVEDATA_BIGDOLLSYM");
    expect(snorlax.collisionStride).toBe(4);
    expect([snorlax.x, snorlax.y]).toEqual([71, 19]);
    for (let x = 68; x <= 71; x += 1) {
      for (let y = 16; y <= 19; y += 1) {
        expect((engine as any)._npc_occupying_subtile(x, y)).toBe(snorlax);
      }
    }

    engine.player_x = 69;
    engine.player_y = 20;
    engine.player_direction = "up";
    expect(engine.is_colliding(69, 19, "up")).toBe(true);

    gameState.wram.event_flags.EVENT_VERMILION_CITY_SNORLAX = true;
    engine.load_map("VermilionCity");
    await engine.tileset?.ready;
    await Promise.resolve();

    expect(engine.npcs.some((npc) => npc.constantId === "VERMILIONCITY_BIG_SNORLAX")).toBe(false);
    engine.player_x = 69;
    engine.player_y = 20;
    engine.player_direction = "up";
    expect(engine.is_colliding(69, 19, "up")).toBe(false);
  });
});

describe("OverworldMapManagerMixin._refresh_tileset_for_current_map", () => {
  it("does not render synthetic fallback map surfaces before tileset.ready resolves", async () => {
    let resolveReady: (() => void) | null = null;
    const ready = new Promise<void>((resolve) => {
      resolveReady = resolve;
    });

    class DeferredTileset implements OverworldTilesetLike {
      public readonly tilesetName = "deferred";
      public readonly metatiles = [];
      public loaded = false;
      public readonly ready = ready;

      public renderMetatile(
        _metatileId: number,
        _target: Surface,
        _x: number,
        _y: number,
        _options?: RenderMetatileOptions,
      ): void {}

      public renderPriorityMetatile(
        _metatileId: number,
        _target: Surface,
        _x: number,
        _y: number,
      ): void {}
    }

    const overworld = new TestOverworld() as TestOverworld & {
      _tileset_cache_by_time?: Map<string, OverworldTilesetLike>;
      _rebuild_composite_surface?: jest.Mock;
    };
    overworld.game_state = createInitialGameState();
    overworld.current_map_name = "NEW_BARK_TOWN";
    overworld.map = createTestMap(2, 2);
    overworld._rebuild_composite_surface = jest.fn();
    overworld._tileset_cache_by_time = new Map();
    overworld._tileset_animator = null;
    overworld._grass_rustle = null;
    overworld.data_loader = {
      map_attributes: new Map([
        ["NEW_BARK_TOWN", { tileset_name: "johto", connections: [] }],
      ]),
      Tileset: DeferredTileset,
    } as unknown as DataLoader;

    mapGeometry.create_map_surface.mockClear();
    mapGeometry.create_priority_surface.mockClear();

    overworld._refresh_tileset_for_current_map();

    expect(overworld.map_surface).toBeNull();
    expect(overworld.priority_surface).toBeNull();
    expect(mapGeometry.create_map_surface).not.toHaveBeenCalled();
    expect(mapGeometry.create_priority_surface).not.toHaveBeenCalled();

    resolveReady?.();
    await Promise.resolve();
    await Promise.resolve();

    expect(mapGeometry.create_map_surface).toHaveBeenCalledTimes(1);
    expect(mapGeometry.create_priority_surface).toHaveBeenCalledTimes(1);
  });
});

describe("OverworldMapManagerMixin._determine_effective_time_of_day", () => {
  it("clears flash state only when exiting to routes or towns", () => {
    const overworld = new TestOverworld();
    overworld.game_state = createInitialGameState();
    const wram = overworld.game_state.wram;
    setBooleanFlag(wram.engine_flags, "STATUSFLAGS_FLASH", true);
    wram.flash_active_maps = { TEST_CAVE: true };

    const indoor = {
      time_of_day: "day",
      environment: "INDOOR",
      tileset_name: "test",
    } as any;
    overworld._determine_effective_time_of_day("TEST_CAVE", indoor);
    expect(getBooleanFlag(wram.engine_flags, "STATUSFLAGS_FLASH")).toBe(true);
    expect(wram.flash_active_maps.TEST_CAVE).toBe(true);

    const route = {
      time_of_day: "day",
      environment: "ROUTE",
      tileset_name: "test",
    } as any;
    overworld._determine_effective_time_of_day("TEST_CAVE", route);
    expect(getBooleanFlag(wram.engine_flags, "STATUSFLAGS_FLASH")).toBe(false);
    expect(Object.keys(wram.flash_active_maps).length).toBe(0);
  });
});

describe("OverworldMapManagerMixin._refresh_tileset_for_current_map", () => {
  it("rebuilds warp permissions after the tileset finishes loading", async () => {
    const mapName = "TestWarpMap";
    const warpPermission = resolveCollisionValue("WARP_CARPET_DOWN");
    const floorPermission = resolveCollisionValue("FLOOR");
    let resolveReady: (() => void) | null = null;
    let lastTileset: FakeTileset | null = null;

    class FakeTileset {
      public tilesetName: string;
      public metatiles: Array<{ tiles: Array<Array<{ tileIndex: number }>>; collision: number[] }>;
      public ready: Promise<void>;

      constructor(tilesetName: string) {
        this.tilesetName = tilesetName;
        this.metatiles = [
          {
            tiles: [[{ tileIndex: 0 }]],
            collision: [floorPermission, floorPermission, floorPermission, floorPermission],
          },
        ];
        this.ready = new Promise<void>((resolve) => {
          resolveReady = resolve;
        });
        lastTileset = this;
      }

      public renderMetatile(): void {}
      public renderPriorityMetatile(): void {}
    }

    const overworld = new TestOverworld();
    overworld.game_state = createInitialGameState();
    overworld.data_loader = {
      map_attributes: new Map([
        [
          mapName,
          {
            tileset_name: "test",
            border_block: 0,
            width: 1,
            height: 1,
            connections: [],
            time_of_day: null,
            phone_service: 0,
            phone_flag: false,
            environment: null,
            location: null,
            music: null,
            palette: null,
            fishing_group: null,
            map_constant: null,
            map_group_constant: null,
            blocks_label: null,
            map_scripts_label: null,
            map_events_label: null,
            connection_flags: null,
          },
        ],
      ]),
      npc_data: new Map([[mapName, []]]),
      Tileset: FakeTileset as any,
    } as any;
    overworld.map = createTestMap(1, 1);
    overworld.current_map_name = mapName;
    overworld.TILES_PER_COLLISION = 2;
    overworld._set_map_events({
      warps: [
        {
          index: 1,
          x: 0,
          y: 0,
          target_map_constant: "TEST_TARGET",
          target_map: "TestTarget",
          target_warp_id: 1,
        },
      ],
      coord_events: [],
      bg_events: [],
    });

    overworld._refresh_tileset_for_current_map();
    overworld._refresh_warp_permissions();

    const cacheKey = "1,1";
    const initialPermission = overworld._warp_permission_cache?.[cacheKey]?.[0]?.[1];
    expect(initialPermission).toBe(floorPermission);
    if (initialPermission !== null && initialPermission !== undefined) {
      expect(isWarpPermission(initialPermission)).toBe(false);
    }

    if (!lastTileset || !resolveReady) {
      throw new Error("Tileset did not initialize for warp permission test.");
    }
    lastTileset.metatiles[0].collision = [
      warpPermission,
      warpPermission,
      warpPermission,
      warpPermission,
    ];
    resolveReady();
    await Promise.resolve();

    const updatedPermission = overworld._warp_permission_cache?.[cacheKey]?.[0]?.[1];
    expect(updatedPermission).toBe(warpPermission);
    if (updatedPermission !== null && updatedPermission !== undefined) {
      expect(isWarpPermission(updatedPermission)).toBe(true);
    }
  });
});

describe("OverworldMapManagerMixin.check_for_warp_event", () => {
  it("does not spam cooldown logs every frame", () => {
    const overworld = new TestOverworld();
    const cooldownLogger = jest.fn();
    overworld._logger = { debug: cooldownLogger };
    overworld.script_runner = null;
    overworld.current_map_name = "NewBarkTown";
    overworld.player_x = 0;
    overworld.player_y = 0;
    overworld._active_warp_tile = null;
    overworld._warp_cooldown = 5;

    expect(overworld.check_for_warp_event()).toBe(false);
    expect(overworld.check_for_warp_event()).toBe(false);
    expect(overworld.check_for_warp_event()).toBe(false);
    expect(cooldownLogger).not.toHaveBeenCalled();
  });

  it("warps when standing on a real-world warp tile", async () => {
    const dataLoader = new DataLoader();
    const mapName = "NewBarkTown";
    dataLoader.ensure_overworld_data({ map_name: mapName });

    const mapEvents = dataLoader.map_events.get(mapName);
    if (!mapEvents || !mapEvents.warps.length) {
      throw new Error("Missing warps for NewBarkTown; expected map events to be loaded.");
    }
    const warp = mapEvents.warps.find((entry) => entry.target_map_constant === "ELMS_LAB") ?? mapEvents.warps[0];
    const targetMetadata = getMapMetadataByConstant(warp.target_map_constant);
    if (!targetMetadata) {
      throw new Error(`Missing map metadata for ${warp.target_map_constant}.`);
    }

    const overworld = new TestOverworld();
    overworld.data_loader = dataLoader;
    overworld.game_state = createInitialGameState();
    overworld.event_manager = null;
    overworld.script_runner = null;
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => true;
    (overworld as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };

    const firstNpc = new OverworldObject(resolveNpcDataList(dataLoader, mapName)[0]);
    (overworld as any)._apply_variable_sprite(firstNpc);
    (overworld as any)._initialise_object_coordinates(firstNpc);
    expect((overworld as any)._object_should_spawn(firstNpc, { ignore_event_flag: false })).toBe(true);

    overworld.load_map(mapName);
    await overworld.tileset?.ready;
    await Promise.resolve();

    const stride = overworld.TILES_PER_COLLISION;
    const offset = Math.max(0, stride - 1);
    overworld.player_x = warp.x * stride + offset;
    overworld.player_y = warp.y * stride + offset;
    overworld._warp_cooldown = 0;
    overworld._active_warp_tile = null;

    const warped = overworld.check_for_warp_event();

    expect(warped).toBe(true);
    expect(overworld.current_map_name).toBe(targetMetadata.name);
  });

  it("returns downstairs from Pokecenter2F when stepping back onto the ladder warp tile", async () => {
    const dataLoader = new DataLoader();
    const mapName = "Pokecenter2F";
    dataLoader.ensure_overworld_data({ map_name: mapName });

    const mapEvents = dataLoader.map_events.get(mapName);
    if (!mapEvents || !mapEvents.warps.length) {
      throw new Error("Missing warps for Pokecenter2F; expected map events to be loaded.");
    }
    const downstairsWarp = mapEvents.warps.find(
      (entry) => entry.target_map_constant === "POKECENTER_2F" && entry.target_warp_id === -1
    );
    if (!downstairsWarp) {
      throw new Error("Missing downstairs warp on Pokecenter2F.");
    }

    const overworld = new TestOverworld();
    overworld.data_loader = dataLoader;
    overworld.game_state = createInitialGameState();
    overworld.event_manager = null;
    overworld.script_runner = null;
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => false;

    const goldenrod1f = getMapMetadataByConstant("GOLDENROD_POKECENTER_1F");
    if (!goldenrod1f) {
      throw new Error("Missing map metadata for GoldenrodPokecenter1F.");
    }
    overworld.game_state.wram.wPrevMapGroup = goldenrod1f.groupId;
    overworld.game_state.wram.wPrevMapNumber = goldenrod1f.mapId;
    overworld.game_state.wram.wPrevWarp = 2;
    overworld.game_state.wram.wBackupMapGroup = goldenrod1f.groupId;
    overworld.game_state.wram.wBackupMapNumber = goldenrod1f.mapId;

    const stride = overworld.TILES_PER_COLLISION;
    const offset = Math.max(0, stride - 1);
    overworld.player_x = downstairsWarp.x * stride + offset;
    overworld.player_y = downstairsWarp.y * stride + offset;
    overworld.load_map(mapName);
    await overworld.tileset?.ready;
    await Promise.resolve();

    expect(overworld._active_warp_tile).toEqual([
      mapName,
      downstairsWarp.x * stride + offset,
      downstairsWarp.y * stride + offset,
    ]);

    overworld.player_y -= stride;
    overworld._refresh_warp_state();
    expect(overworld._active_warp_tile).toBeNull();

    overworld.player_y += stride;
    overworld._warp_cooldown = 0;
    overworld._active_warp_tile = null;

    expect(overworld.check_for_warp_event()).toBe(true);
    expect(overworld.current_map_name).toBe(goldenrod1f.name);
  });

  it("returns to the originating Pokemon Center 1F after a live 1F to 2F round trip", async () => {
    const dataLoader = new DataLoader();
    const center1f = "GoldenrodPokecenter1F";
    const center2f = "Pokecenter2F";
    dataLoader.ensure_overworld_data({ map_name: center1f });
    dataLoader.ensure_overworld_data({ map_name: center2f });

    const center1fEvents = dataLoader.map_events.get(center1f);
    if (!center1fEvents || !center1fEvents.warps.length) {
      throw new Error("Missing warps for GoldenrodPokecenter1F; expected map events to be loaded.");
    }
    const upstairsWarp = center1fEvents.warps.find(
      (entry) => entry.target_map_constant === "POKECENTER_2F"
    );
    if (!upstairsWarp) {
      throw new Error("Missing upstairs warp from GoldenrodPokecenter1F to Pokecenter2F.");
    }

    const center2fEvents = dataLoader.map_events.get(center2f);
    if (!center2fEvents || !center2fEvents.warps.length) {
      throw new Error("Missing warps for Pokecenter2F; expected map events to be loaded.");
    }
    const downstairsWarp = center2fEvents.warps.find(
      (entry) => entry.target_map_constant === "POKECENTER_2F" && entry.target_warp_id === -1
    );
    if (!downstairsWarp) {
      throw new Error("Missing downstairs warp on Pokecenter2F.");
    }

    const overworld = new TestOverworld();
    overworld.data_loader = dataLoader;
    overworld.game_state = createInitialGameState();
    overworld.event_manager = null;
    overworld.script_runner = null;
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => true;
    (overworld as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };

    const stride = overworld.TILES_PER_COLLISION;
    const offset = Math.max(0, stride - 1);

    overworld.load_map(center1f);
    await overworld.tileset?.ready;
    await Promise.resolve();

    overworld.player_x = upstairsWarp.x * stride + offset;
    overworld.player_y = upstairsWarp.y * stride + offset;
    overworld._warp_cooldown = 0;
    overworld._active_warp_tile = null;

    expect(overworld.check_for_warp_event()).toBe(true);
    expect(overworld.current_map_name).toBe(center2f);
    expect(overworld.game_state.wram.wPrevWarp).toBe(upstairsWarp.index);

    overworld.player_y -= stride;
    overworld._refresh_warp_state();
    expect(overworld._active_warp_tile).toBeNull();

    overworld.player_y += stride;
    overworld._warp_cooldown = 0;
    overworld._active_warp_tile = null;

    expect(overworld.player_x).toBe(downstairsWarp.x * stride + offset);
    expect(overworld.player_y).toBe(downstairsWarp.y * stride + offset);
    expect(overworld.check_for_warp_event()).toBe(true);
    expect(overworld.current_map_name).toBe(center1f);
  });

  it("returns to the originating Pokemon Center 1F after coming back from Trade Center", async () => {
    const dataLoader = new DataLoader();
    const center1f = "GoldenrodPokecenter1F";
    const center2f = "Pokecenter2F";
    const tradeCenter = "TradeCenter";
    dataLoader.ensure_overworld_data({ map_name: center1f });
    dataLoader.ensure_overworld_data({ map_name: center2f });
    const tradeCenterMetadata = getMapMetadataByName(tradeCenter);
    if (!tradeCenterMetadata) {
      throw new Error("Missing map metadata for TradeCenter.");
    }

    const center1fEvents = dataLoader.map_events.get(center1f);
    const center2fEvents = dataLoader.map_events.get(center2f);
    if (!center1fEvents?.warps.length || !center2fEvents?.warps.length) {
      throw new Error("Missing warp tables required for Pokecenter2F link-room regression.");
    }

    const upstairsWarp = center1fEvents.warps.find(
      (entry) => entry.target_map_constant === "POKECENTER_2F"
    );
    const downstairsWarp = center2fEvents.warps.find(
      (entry) => entry.target_map_constant === "POKECENTER_2F" && entry.target_warp_id === -1
    );
    if (!upstairsWarp || !downstairsWarp) {
      throw new Error("Missing one or more expected Pokecenter2F warps.");
    }

    const overworld = new TestOverworld();
    overworld.data_loader = dataLoader;
    overworld.game_state = createInitialGameState();
    overworld.event_manager = null;
    overworld.script_runner = null;
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => true;
    (overworld as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };

    const stride = overworld.TILES_PER_COLLISION;
    const offset = Math.max(0, stride - 1);

    overworld.load_map(center1f);
    await overworld.tileset?.ready;
    await Promise.resolve();

    overworld.player_x = upstairsWarp.x * stride + offset;
    overworld.player_y = upstairsWarp.y * stride + offset;
    overworld._warp_cooldown = 0;
    overworld._active_warp_tile = null;
    expect(overworld.check_for_warp_event()).toBe(true);
    expect(overworld.current_map_name).toBe(center2f);
    expect(overworld.game_state.wram.wBackupWarpNumber).toBe(upstairsWarp.index);

    overworld.game_state.wram.wPrevMapGroup = tradeCenterMetadata.groupId;
    overworld.game_state.wram.wPrevMapNumber = tradeCenterMetadata.mapId;
    overworld.game_state.wram.wPrevWarp = 1;

    overworld.player_x = downstairsWarp.x * stride + offset;
    overworld.player_y = downstairsWarp.y * stride + offset;
    overworld._warp_cooldown = 0;
    overworld._active_warp_tile = null;
    expect(overworld.check_for_warp_event()).toBe(true);
    expect(overworld.current_map_name).toBe(center1f);
  });

  it("spawns the Pokecenter2F trade receptionist on the authored blocking tile", async () => {
    const dataLoader = new DataLoader();
    const mapName = "Pokecenter2F";
    dataLoader.ensure_overworld_data({ map_name: mapName });

    const overworld = new TestOverworld();
    overworld.data_loader = dataLoader;
    overworld.game_state = createInitialGameState();
    overworld.event_manager = null;
    overworld.script_runner = null;
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => true;
    (overworld as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };

    overworld.load_map(mapName);
    await overworld.tileset?.ready;
    await Promise.resolve();

    expect(
      overworld.npcs.map((npc) => ({
        script: npc.event?.script ?? null,
        x: npc.x,
        y: npc.y,
      }))
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          script: "LinkReceptionistScript_Trade",
          x: 11,
          y: 5,
        }),
      ])
    );
  });

  it("does not immediately re-trigger a warp after loading onto the destination warp tile", async () => {
    const dataLoader = new DataLoader();
    const mapName = "PlayersHouse1F";
    dataLoader.ensure_overworld_data({ map_name: mapName });

    const mapEvents = dataLoader.map_events.get(mapName);
    if (!mapEvents || !mapEvents.warps.length) {
      throw new Error("Missing warps for PlayersHouse1F; expected map events to be loaded.");
    }
    const warp = mapEvents.warps.find((entry) => entry.target_map_constant === "PLAYERS_HOUSE_2F");
    if (!warp) {
      throw new Error("Missing stair warp from PlayersHouse1F to PlayersHouse2F.");
    }

    const overworld = new TestOverworld();
    overworld.data_loader = dataLoader;
    overworld.game_state = createInitialGameState();
    overworld.event_manager = null;
    overworld.script_runner = null;
    overworld.TILES_PER_COLLISION = 2;
    overworld.data_loader.Tileset = OverworldTileset;
    (overworld as any)._sprite_asset_exists = () => false;

    const stride = overworld.TILES_PER_COLLISION;
    const offset = Math.max(0, stride - 1);
    overworld.player_x = warp.x * stride + offset;
    overworld.player_y = warp.y * stride + offset;

    overworld.load_map(mapName);
    await overworld.tileset?.ready;
    await Promise.resolve();

    overworld._warp_cooldown = 0;
    expect(overworld.check_for_warp_event()).toBe(false);
    expect(overworld.current_map_name).toBe(mapName);

    overworld.player_y += overworld.TILES_PER_COLLISION;
    overworld._refresh_warp_state();
    expect(overworld._active_warp_tile).toBeNull();
  });

  it.each([
    [47, 13],
    [48, 13],
  ])("walks south from Route36 through the north gate into RuinsOfAlphOutside at (%d,%d)", async (warpX, warpY) => {
    const dataLoader = new DataLoader();
    dataLoader.Tileset = OverworldTileset;
    const gameState = createInitialGameState();
    gameState.wram.event_flags.EVENT_INITIALIZED_EVENTS = true;
    const route36Metadata = getMapMetadataByConstant("ROUTE_36");
    if (!route36Metadata) {
      throw new Error("Missing ROUTE_36 metadata.");
    }
    gameState.wram.wMapGroup = route36Metadata.groupId;
    gameState.wram.wMapNumber = route36Metadata.mapId;

    dataLoader.ensure_overworld_data({ map_name: "Route36" });
    const route36Events = dataLoader.map_events.get("Route36");
    if (!route36Events) {
      throw new Error("Missing Route36 events.");
    }
    const northGateWarp = route36Events.warps.find(
      (entry) =>
        entry.x === warpX &&
        entry.y === warpY &&
        entry.target_map_constant === "ROUTE_36_RUINS_OF_ALPH_GATE"
    );
    if (!northGateWarp) {
      throw new Error("Missing Route36 -> Route36RuinsOfAlphGate warp.");
    }

    const engine = new OverworldEngine(
      gameState,
      dataLoader,
      new EventManager(gameState),
      new OverworldTileset("johto", "day"),
      new AudioEngine({ masterVolume: 0, muted: true }),
      buildTextUi(),
      { suppressInitialMapEntryEffects: true, suppressInitialMapMusic: true }
    );
    (engine as any)._sprite_asset_exists = () => true;
    (engine as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };
    (engine as any)._tileset_animator = null;
    engine.load_map("Route36");
    await engine.tileset?.ready;
    await Promise.resolve();

    const stride = engine.TILES_PER_COLLISION;
    const offset = Math.max(0, stride - 1);
    engine.player_x = northGateWarp.x * stride + offset;
    engine.player_y = northGateWarp.y * stride + offset;
    engine.player_direction = "down";
    engine._active_warp_tile = null;
    engine._warp_cooldown = 0;

    expect(engine.check_for_warp_event()).toBe(true);
    expect(engine.current_map_name).toBe("Route36RuinsOfAlphGate");
    await engine.tileset?.ready;
    await Promise.resolve();

    for (let step = 0; step < 8 && engine.current_map_name === "Route36RuinsOfAlphGate"; step += 1) {
      engine.move_player("down", true);
      for (let frame = 0; frame < engine.WALK_FRAMES + 2; frame += 1) {
        engine.update();
      }
    }

    expect(engine.current_map_name).toBe("RuinsOfAlphOutside");
  });

  it("pushes the CianwoodGym middle boulder without mutating sorted trainer objects", async () => {
    const dataLoader = new DataLoader();
    dataLoader.Tileset = OverworldTileset;
    const gameState = createInitialGameState();
    gameState.wram.event_flags.EVENT_INITIALIZED_EVENTS = true;
    const metadata = getMapMetadataByName("CianwoodGym");
    if (!metadata) {
      throw new Error("Missing CianwoodGym metadata.");
    }
    gameState.wram.wMapGroup = metadata.groupId;
    gameState.wram.wMapNumber = metadata.mapId;

    const engine = new OverworldEngine(
      gameState,
      dataLoader,
      new EventManager(gameState),
      new OverworldTileset("johto", "day"),
      new AudioEngine({ masterVolume: 0, muted: true }),
      buildTextUi(),
      { suppressInitialMapEntryEffects: true, suppressInitialMapMusic: true }
    );
    (engine as any)._sprite_asset_exists = () => true;
    (engine as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };
    (engine as any)._tileset_animator = null;
    engine.load_map("CianwoodGym");
    await engine.tileset?.ready;
    await Promise.resolve();
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_STRENGTH_ACTIVE", true);

    const middleBoulder = engine.npcs.find(
      (npc) => npc.constantId === "CIANWOODGYM_BOULDER3"
    );
    const blackBeltPositions = new Map(
      engine.npcs
        .filter((npc) => npc.spriteConstant === "SPRITE_BLACK_BELT")
        .map((npc) => [npc.constantId, [npc.event.x, npc.event.y, npc.x, npc.y]])
    );
    if (!middleBoulder) {
      throw new Error("Missing CianwoodGym middle boulder.");
    }

    engine.player_x = middleBoulder.x;
    engine.player_y = middleBoulder.y + engine.TILES_PER_COLLISION;
    engine.player_direction = "up";

    expect(engine.is_colliding(middleBoulder.x, middleBoulder.y, "up")).toBe(false);
    expect(middleBoulder.event.y).toBe(6);
    expect(middleBoulder.y).toBe(13);
    expect(middleBoulder.spriteConstant).toBe("SPRITE_BOULDER");
    expect(middleBoulder.constantId).toBe("CIANWOODGYM_BOULDER3");
    for (const blackBelt of engine.npcs.filter((npc) => npc.spriteConstant === "SPRITE_BLACK_BELT")) {
      expect([blackBelt.event.x, blackBelt.event.y, blackBelt.x, blackBelt.y]).toEqual(
        blackBeltPositions.get(blackBelt.constantId)
      );
    }
  });

  it("resets undropped Ice Path strength boulder positions after reloading the map", async () => {
    const dataLoader = new DataLoader();
    dataLoader.Tileset = OverworldTileset;
    const gameState = createInitialGameState();
    gameState.wram.event_flags.EVENT_INITIALIZED_EVENTS = true;
    const metadata = getMapMetadataByName("IcePathB1F");
    if (!metadata) {
      throw new Error("Missing IcePathB1F metadata.");
    }
    gameState.wram.wMapGroup = metadata.groupId;
    gameState.wram.wMapNumber = metadata.mapId;

    const engine = new OverworldEngine(
      gameState,
      dataLoader,
      new EventManager(gameState),
      new OverworldTileset("ice_path", "day"),
      new AudioEngine({ masterVolume: 0, muted: true }),
      buildTextUi(),
      { suppressInitialMapEntryEffects: true, suppressInitialMapMusic: true }
    );
    (engine as any)._sprite_asset_exists = () => true;
    (engine as any)._npc_sprite_cache = { instantiate: jest.fn(() => ({})) };
    (engine as any)._tileset_animator = null;
    engine.load_map("IcePathB1F");
    await engine.tileset?.ready;
    await Promise.resolve();
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_STRENGTH_ACTIVE", true);

    const boulder = engine.npcs.find((npc) => npc.constantId === "ICEPATHB1F_BOULDER1");
    if (!boulder) {
      throw new Error("Missing IcePathB1F boulder 1.");
    }
    const original = [boulder.event.x, boulder.event.y, boulder.x, boulder.y];
    engine.player_x = boulder.x;
    engine.player_y = boulder.y + engine.TILES_PER_COLLISION;
    engine.player_direction = "up";

    expect(engine.is_colliding(boulder.x, boulder.y, "up")).toBe(false);
    expect([boulder.event.x, boulder.event.y, boulder.x, boulder.y]).not.toEqual(original);

    engine.load_map("IcePath1F");
    await engine.tileset?.ready;
    await Promise.resolve();
    engine.load_map("IcePathB1F");
    await engine.tileset?.ready;
    await Promise.resolve();

    const reloaded = engine.npcs.find((npc) => npc.constantId === "ICEPATHB1F_BOULDER1");
    expect(reloaded).toBeTruthy();
    expect([reloaded?.event.x, reloaded?.event.y, reloaded?.x, reloaded?.y]).toEqual(original);
  });
});

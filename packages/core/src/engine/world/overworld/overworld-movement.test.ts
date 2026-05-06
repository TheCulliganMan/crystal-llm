import { PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { OverworldMovement } from "./overworld-movement";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { OverworldObject } from "./overworld-object";
import type { LoggerLike } from "./logger";
import type { WarpEvent } from "@pokecrystal/core/core/models/map";
import { createInitialGameState, type GameState } from "@pokecrystal/core/core/state";
import type { NPCStepBlocker } from "./npc-autonomous-controller";
import type { BlockFeedbackDetails } from "@pokecrystal/core/types/overworld";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";
import type { OverworldTilesetLike } from "./tileset-types";
import type { OverworldMap } from "./overworld-map";
import { resolveCollisionValue } from "@pokecrystal/core/engine/world/overworld/collision-data";
import type { MapAttributes } from "@pokecrystal/core/core/models/map";
import { setBooleanFlag } from "@pokecrystal/core/engine/world/overworld/flag-collection";

const createTestMap = (): OverworldMap => {
  const width = 1;
  const height = 1;
  const metatileIds = [0];
  return {
    mapName: "Test",
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

class TestMovement extends OverworldMovement {
  protected readonly TILES_PER_COLLISION = 2;
  protected readonly TURN_FRAMES = 1;
  protected readonly STEP_PIXELS = 4;
  protected readonly STEP_SPEED_PX = 1;

  protected player_x = 0;
  protected player_y = 0;
  protected prev_player_x = 0;
  protected prev_player_y = 0;
  protected target_tile_x = 0;
  protected target_tile_y = 0;
  protected target_px_x = 0;
  protected target_px_y = 0;

  protected is_moving = false;
  protected _turn_frames_remaining = 0;
  protected _turning_direction: string | null = null;
  protected _turn_should_force_step = false;
  protected _pending_auto_step: [string, boolean] | null = null;
  protected _ledge_jump_active = false;
  protected _ledge_jump_total_distance_px = 0;
  protected _ledge_jump_animation_progress_px = 0;

  protected player_direction = "down";
  protected player_state = PlayerState.NORMAL;

  protected _last_step_direction: string | null = null;
  protected _queued_direction: string | null = null;
  protected _pending_ledge_landing: {
    tile_x: number;
    tile_y: number;
    dx: number;
    dy: number;
    direction: string;
  } | null = null;
  protected _last_block_feedback: BlockFeedbackDetails | null = null;
  protected step_progress_px = 0;
  protected _current_step_speed_px = 0;
  protected _current_step_distance_px = 0;
  protected step_dx_px = 0;
  protected step_dy_px = 0;
  protected _block_feedback_tracking = false;
  protected _npc_step_blocked?: NPCStepBlocker;

  protected map: OverworldMap = createTestMap();
  protected tileset: OverworldTilesetLike = stubTileset;
  protected data_loader: DataLoader = {} as DataLoader;
  protected current_map_name = "Test";
  protected game_state: GameState = createInitialGameState();
  protected player_object: OverworldObject | null = null;
  protected audio_engine: AudioEngine | null = null;
  protected _warp_tile_lookup: Record<string, WarpEvent[]> | Map<string, WarpEvent[]> | null = null;
  public _logger: LoggerLike | null = { debug: jest.fn() };

  public player_movement_locked(): boolean {
    return false;
  }

  protected _npc_occupying_subtile(_x: number, _y: number): OverworldObject | null {
    return null;
  }

  protected _prime_player_walk_cycle(): void {}

  protected _maybe_spawn_grass_rustle(_target: OverworldObject | null, _x: number, _y: number): void {}

  protected move_object(_objectId: string | number, _mapX: number, _mapY: number): void {}

  public set_audio_engine(engine: AudioEngine | null): void {
    this.audio_engine = engine;
  }
}

type MovementTestHost = TestMovement & {
  _play_bump_sound(): void;
  _play_strength_sound(): void;
  _play_ledge_jump_sound(): void;
};

describe("OverworldMovement audio guards", () => {
  it("plays bump sounds via playSound when play_sound is missing", () => {
    const movement = new TestMovement() as MovementTestHost;
    const playSound = jest.fn();
    movement.set_audio_engine({ playSound } as unknown as AudioEngine);

    expect(() => movement._play_bump_sound()).not.toThrow();
    expect(playSound).toHaveBeenCalledWith("SFX_BUMP");
  });

  it("plays strength sounds via playSound when play_sound is missing", () => {
    const movement = new TestMovement() as MovementTestHost;
    const playSound = jest.fn();
    movement.set_audio_engine({ playSound } as unknown as AudioEngine);

    expect(() => movement._play_strength_sound()).not.toThrow();
    expect(playSound).toHaveBeenCalledWith("SFX_STRENGTH");
  });

  it("plays ledge jump sounds via playSound when play_sound is missing", () => {
    const movement = new TestMovement() as MovementTestHost;
    const playSound = jest.fn();
    movement.set_audio_engine({ playSound } as unknown as AudioEngine);

    expect(() => movement._play_ledge_jump_sound()).not.toThrow();
    expect(playSound).toHaveBeenCalledWith("SFX_JUMP_OVER_LEDGE");
  });
});

describe("OverworldMovement surf exit", () => {
  const createSurfExitMap = (): OverworldMap => {
    const width = 2;
    const height = 1;
    const metatileIds = [0, 1];
    return {
      mapName: "Test",
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

  const createSurfExitTileset = (): OverworldTilesetLike => {
    const water = resolveCollisionValue("WATER");
    const land = resolveCollisionValue("FLOOR");
    const waterMetatile = { collision: [water, water, water, water] };
    const landMetatile = { collision: [land, land, land, land] };
    return {
      tilesetName: "test",
      metatiles: [waterMetatile, landMetatile],
      renderMetatile: () => {},
      renderPriorityMetatile: () => {},
    } as OverworldTilesetLike;
  };

  it("exits surfing when stepping onto land", () => {
    class SurfExitMovement extends TestMovement {
      public start_map_music = jest.fn();
      public _create_player_animations = jest.fn(() => ({}));
    }

    const movement = new SurfExitMovement();
    (movement as any).map = createSurfExitMap();
    (movement as any).tileset = createSurfExitTileset();
    (movement as any).player_state = PlayerState.SURF;
    (movement as any).game_state.wram.surfing = true;
    (movement as any).player_x = 2;
    (movement as any).player_y = 1;
    (movement as any).player_direction = "right";
    (movement as any).data_loader = {
      map_attributes: new Map([
        ["Test", { connections: [] } as MapAttributes],
      ]),
    } as unknown as DataLoader;

    (movement as any)._handle_surf_movement("right");

    expect((movement as any).player_state).toBe(PlayerState.NORMAL);
    expect((movement as any).game_state.wram.surfing).toBe(false);
    expect(movement.start_map_music).toHaveBeenCalled();
  });
});

describe("OverworldMovement ice sliding", () => {
  const createIceMap = (): OverworldMap => {
    const width = 3;
    const height = 1;
    const metatileIds = [0, 1, 2];
    return {
      mapName: "IceTest",
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

  const createIceTileset = (iceCollision: number): OverworldTilesetLike => {
    const floor = resolveCollisionValue("FLOOR");
    return {
      tilesetName: "ice-test",
      metatiles: [
        { collision: [floor, floor, floor, floor] },
        { collision: [iceCollision, iceCollision, iceCollision, iceCollision] },
        { collision: [floor, floor, floor, floor] },
      ],
      renderMetatile: () => {},
      renderPriorityMetatile: () => {},
    } as OverworldTilesetLike;
  };

  it.each(["ICE", "ICE_2B"])("queues a forced %s slide in the last step direction", (collisionName) => {
    const movement = new TestMovement();
    (movement as any).map = createIceMap();
    (movement as any).tileset = createIceTileset(resolveCollisionValue(collisionName));
    (movement as any).player_x = 5;
    (movement as any).player_y = 1;
    (movement as any)._last_step_direction = "right";
    (movement as any)._queued_direction = "down";

    expect((movement as any)._queue_ice_slide_step()).toBe(true);
    expect((movement as any)._pending_auto_step).toEqual(["right", true]);
  });

  it("does not queue a slide on normal floor", () => {
    const movement = new TestMovement();
    (movement as any).map = createIceMap();
    (movement as any).tileset = createIceTileset(resolveCollisionValue("FLOOR"));
    (movement as any).player_x = 1;
    (movement as any).player_y = 1;
    (movement as any)._last_step_direction = "right";

    expect((movement as any)._queue_ice_slide_step()).toBe(false);
    expect((movement as any)._pending_auto_step).toBeNull();
  });
});

describe("OverworldMovement surf map connections", () => {
  const createWaterMap = (mapName: string, width: number, height: number): OverworldMap => {
    const metatileIds = new Array(width * height).fill(0);
    return {
      mapName,
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

  const createWaterTileset = (): OverworldTilesetLike => {
    const water = resolveCollisionValue("WATER");
    return {
      tilesetName: "johto",
      metatiles: [{ collision: [water, water, water, water] }],
      renderMetatile: () => {},
      renderPriorityMetatile: () => {},
    } as OverworldTilesetLike;
  };

  it.each([
    {
      label: "east from Route40 into OlivineCity",
      mapName: "Route40",
      width: 10,
      height: 18,
      x: 38,
      y: 18,
      direction: "right",
      target: [40, 18],
      connections: [
        { direction: "south", target_map: "Route41", offset: -15 },
        { direction: "east", target_map: "OlivineCity", offset: -9 },
      ],
    },
    {
      label: "west from OlivineCity into Route40",
      mapName: "OlivineCity",
      width: 20,
      height: 18,
      x: 1,
      y: 18,
      direction: "left",
      target: [-1, 18],
      connections: [
        { direction: "north", target_map: "Route39", offset: 5 },
        { direction: "west", target_map: "Route40", offset: 9 },
      ],
    },
  ])("starts a surf step across the connected map edge $label", (scenario) => {
    const movement = new TestMovement();
    (movement as any).map = createWaterMap(scenario.mapName, scenario.width, scenario.height);
    (movement as any).tileset = createWaterTileset();
    (movement as any).player_state = PlayerState.SURF;
    (movement as any).game_state.wram.surfing = true;
    (movement as any).player_x = scenario.x;
    (movement as any).player_y = scenario.y;
    (movement as any).player_direction = scenario.direction;
    (movement as any).current_map_name = scenario.mapName;
    (movement as any).data_loader = {
      map_attributes: new Map([
        [
          scenario.mapName,
          { connections: scenario.connections } as MapAttributes,
        ],
      ]),
    } as unknown as DataLoader;

    (movement as any)._handle_surf_movement(scenario.direction);

    expect((movement as any).is_moving).toBe(true);
    expect([(movement as any).target_tile_x, (movement as any).target_tile_y]).toEqual(scenario.target);
    expect((movement as any).player_state).toBe(PlayerState.SURF);
    expect((movement as any).game_state.wram.surfing).toBe(true);
  });
});

describe("OverworldMovement surf field-move blockers", () => {
  const createTwoMetatileMap = (
    mapName: string,
    width: number,
    height: number,
    metatileIds: number[],
  ): OverworldMap => {
    return {
      mapName,
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

  const createFieldMoveTileset = (fieldMoveCollision: number): OverworldTilesetLike => {
    const water = resolveCollisionValue("WATER");
    return {
      tilesetName: "field-move-blocker",
      metatiles: [
        { collision: [water, water, water, water] },
        {
          collision: [
            fieldMoveCollision,
            fieldMoveCollision,
            fieldMoveCollision,
            fieldMoveCollision,
          ],
        },
      ],
      renderMetatile: () => {},
      renderPriorityMetatile: () => {},
    } as OverworldTilesetLike;
  };

  const assignAttributes = (movement: TestMovement, mapName: string): void => {
    (movement as any).data_loader = {
      map_attributes: new Map([[mapName, { connections: [] } as MapAttributes]]),
    } as unknown as DataLoader;
    (movement as any).current_map_name = mapName;
  };

  it("blocks surfing onto a whirlpool until Whirlpool removes it", () => {
    const movement = new TestMovement();
    (movement as any).map = createTwoMetatileMap("WhirlpoolTest", 2, 1, [0, 1]);
    (movement as any).tileset = createFieldMoveTileset(resolveCollisionValue("WHIRLPOOL"));
    (movement as any).player_state = PlayerState.SURF;
    (movement as any).game_state.wram.surfing = true;
    (movement as any).player_x = 2;
    (movement as any).player_y = 1;
    (movement as any).player_direction = "right";
    assignAttributes(movement, "WhirlpoolTest");

    (movement as any)._handle_surf_movement("right");

    expect((movement as any).is_moving).toBe(false);
  });

  it("allows surfing onto a water quadrant of an uncleared whirlpool metatile", () => {
    const movement = new TestMovement();
    const water = resolveCollisionValue("WATER");
    const buoy = resolveCollisionValue("BUOY");
    (movement as any).map = createTwoMetatileMap("WhirlpoolQuadrantTest", 2, 1, [0, 1]);
    (movement as any).tileset = {
      tilesetName: "johto",
      metatiles: [
        { collision: [water, water, water, water] },
        {
          collision: [
            resolveCollisionValue("WHIRLPOOL"),
            buoy,
            water,
            buoy,
          ],
        },
      ],
      renderMetatile: () => {},
      renderPriorityMetatile: () => {},
    } as OverworldTilesetLike;
    (movement as any).player_state = PlayerState.SURF;
    (movement as any).game_state.wram.surfing = true;
    (movement as any).player_x = 2;
    (movement as any).player_y = 2;
    (movement as any).player_direction = "right";
    assignAttributes(movement, "WhirlpoolQuadrantTest");

    (movement as any)._handle_surf_movement("right");

    expect((movement as any).is_moving).toBe(true);
  });

  it("blocks surfing onto a waterfall without Waterfall active", () => {
    const movement = new TestMovement();
    (movement as any).map = createTwoMetatileMap("WaterfallBlockTest", 1, 2, [1, 0]);
    (movement as any).tileset = createFieldMoveTileset(resolveCollisionValue("WATERFALL"));
    (movement as any).player_state = PlayerState.SURF;
    (movement as any).game_state.wram.surfing = true;
    (movement as any).player_x = 1;
    (movement as any).player_y = 5;
    (movement as any).player_direction = "up";
    assignAttributes(movement, "WaterfallBlockTest");

    (movement as any)._handle_surf_movement("up");

    expect((movement as any).is_moving).toBe(false);
  });

  it("allows Waterfall-driven surf movement onto waterfall collisions", () => {
    const movement = new TestMovement();
    (movement as any).map = createTwoMetatileMap("WaterfallMoveTest", 1, 2, [1, 0]);
    (movement as any).tileset = createFieldMoveTileset(resolveCollisionValue("WATERFALL"));
    (movement as any).player_state = PlayerState.SURF;
    (movement as any).game_state.wram.surfing = true;
    (movement as any).player_x = 1;
    (movement as any).player_y = 5;
    (movement as any).player_direction = "up";
    (movement as any)._waterfall_movement_active = true;
    assignAttributes(movement, "WaterfallMoveTest");

    (movement as any)._handle_surf_movement("up");

    expect((movement as any).is_moving).toBe(true);
  });

  it("allows surfing down onto a waterfall without Waterfall active", () => {
    const movement = new TestMovement();
    (movement as any).map = createTwoMetatileMap("WaterfallDescendTest", 1, 2, [0, 1]);
    (movement as any).tileset = createFieldMoveTileset(resolveCollisionValue("WATERFALL"));
    (movement as any).player_state = PlayerState.SURF;
    (movement as any).game_state.wram.surfing = true;
    (movement as any).player_x = 1;
    (movement as any).player_y = 3;
    (movement as any).player_direction = "down";
    assignAttributes(movement, "WaterfallDescendTest");

    (movement as any)._handle_surf_movement("down");

    expect((movement as any).is_moving).toBe(true);
  });

  it.each([
    ["WATERFALL_RIGHT", "right"],
    ["WATERFALL_LEFT", "left"],
    ["WATERFALL_UP", "up"],
    ["WATERFALL", "down"],
    ["CURRENT_RIGHT", "right"],
    ["CURRENT_LEFT", "left"],
    ["CURRENT_UP", "up"],
    ["CURRENT_DOWN", "down"],
  ])("queues ASM forced %s current movement", (collisionName, direction) => {
    const movement = new TestMovement();
    (movement as any).map = createTwoMetatileMap("WaterfallForceTest", 1, 2, [1, 0]);
    (movement as any).tileset = createFieldMoveTileset(resolveCollisionValue(collisionName));
    (movement as any).player_state = PlayerState.SURF;
    (movement as any).game_state.wram.surfing = true;
    (movement as any).player_x = 1;
    (movement as any).player_y = 1;
    (movement as any).player_direction = "down";
    assignAttributes(movement, "WaterfallForceTest");

    expect((movement as any)._queue_forced_waterfall_step()).toBe(true);
    expect((movement as any)._pending_auto_step).toEqual([direction, true]);
    expect((movement as any).player_direction).toBe(direction);
  });

  it("blocks surfing upward onto a waterfall current without Waterfall active", () => {
    const movement = new TestMovement();
    (movement as any).map = createTwoMetatileMap("CurrentDownBlockTest", 1, 2, [1, 0]);
    (movement as any).tileset = createFieldMoveTileset(resolveCollisionValue("CURRENT_DOWN"));
    (movement as any).player_state = PlayerState.SURF;
    (movement as any).game_state.wram.surfing = true;
    (movement as any).player_x = 1;
    (movement as any).player_y = 5;
    (movement as any).player_direction = "up";
    assignAttributes(movement, "CurrentDownBlockTest");

    (movement as any)._handle_surf_movement("up");

    expect((movement as any).is_moving).toBe(false);
  });
});

describe("OverworldMovement ledge safety", () => {
  it("treats invalid metatile collision samples as a non-ledge instead of throwing", () => {
    const movement = new TestMovement() as TestMovement & {
      _try_ledge_jump(direction: string): boolean;
    };
    (movement as any).player_x = 8;
    (movement as any).player_y = 0;
    (movement as any).player_direction = "up";
    (movement as any).map = {
      width: 5,
      height: 5,
      getMetatileAt: () => 10,
    };
    (movement as any).tileset = {
      tilesetName: "players_house",
      metatiles: [{ collision: [0, 0, 0, 0] }],
      renderMetatile: jest.fn(),
      renderPriorityMetatile: jest.fn(),
    };

    expect(() => movement._try_ledge_jump("up")).not.toThrow();
    expect(movement._try_ledge_jump("up")).toBe(false);
  });
});

describe("OverworldMovement bike speed parity", () => {
  type MovementHost = TestMovement & {
    _handle_ground_movement(direction: string): void;
    is_colliding: jest.Mock;
  };

  const createHost = (): MovementHost => {
    const movement = new TestMovement() as MovementHost;
    movement.is_colliding = jest.fn(() => false);
    return movement;
  };

  it("uses bike speed on bike states", () => {
    const movement = createHost();
    (movement as any).player_state = PlayerState.BIKE;

    movement._handle_ground_movement("right");

    expect((movement as any)._current_step_speed_px).toBe(2);
  });

  it("falls back to walking speed when downhill and not moving down", () => {
    const movement = createHost();
    (movement as any).player_state = PlayerState.BIKE;
    (movement as any).game_state.wram.engine_flags.ENGINE_DOWNHILL = true;

    movement._handle_ground_movement("left");

    expect((movement as any)._current_step_speed_px).toBe(1);
  });

  it("keeps bike speed while moving down on downhill", () => {
    const movement = createHost();
    (movement as any).player_state = PlayerState.BIKE;
    (movement as any).game_state.wram.engine_flags.ENGINE_DOWNHILL = true;

    movement._handle_ground_movement("down");

    expect((movement as any)._current_step_speed_px).toBe(2);
  });
});

describe("OverworldMovement forced down movement", () => {
  const createDoorMap = (): OverworldMap => {
    const width = 1;
    const height = 1;
    const metatileIds = [0];
    return {
      mapName: "DoorTest",
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

  const createDoorTileset = (): OverworldTilesetLike => {
    const doorCollision = resolveCollisionValue("DOOR");
    return {
      tilesetName: "door",
      metatiles: [
        {
          collision: [doorCollision, doorCollision, doorCollision, doorCollision],
        },
      ],
      renderMetatile: () => {},
      renderPriorityMetatile: () => {},
    } as OverworldTilesetLike;
  };

  const createWarpPanelTileset = (): OverworldTilesetLike => {
    const panelCollision = resolveCollisionValue("WARP_PANEL");
    return {
      tilesetName: "warp-panel",
      metatiles: [
        {
          collision: [panelCollision, panelCollision, panelCollision, panelCollision],
        },
      ],
      renderMetatile: () => {},
      renderPriorityMetatile: () => {},
    } as OverworldTilesetLike;
  };

  it("forces walking down when a warp-facing-down tile is occupied", () => {
    type ForcedMovementHost = TestMovement & {
      _handle_ground_movement(direction: string): void;
      _begin_step: jest.Mock;
      is_colliding: jest.Mock;
    };

    const movement = new TestMovement() as ForcedMovementHost;
    (movement as any).map = createDoorMap();
    (movement as any).tileset = createDoorTileset();
    (movement as any).is_colliding = jest.fn(() => false);
    const beginStep = jest.spyOn(movement as any, "_begin_step").mockImplementation(() => {});

    movement._handle_ground_movement("up");

    expect((movement as any).is_colliding).toHaveBeenCalledWith(0, 2, "down");
    expect(beginStep).toHaveBeenCalledWith(0, 1, 0, 2, 1, 1);
    expect((movement as any)._last_step_direction).toBe("down");
  });

  it("does not force walking down from warp panels", () => {
    type ForcedMovementHost = TestMovement & {
      _handle_ground_movement(direction: string): void;
      _begin_step: jest.Mock;
      is_colliding: jest.Mock;
    };

    const movement = new TestMovement() as ForcedMovementHost;
    (movement as any).map = createDoorMap();
    (movement as any).tileset = createWarpPanelTileset();
    (movement as any).is_colliding = jest.fn(() => false);
    const beginStep = jest.spyOn(movement as any, "_begin_step").mockImplementation(() => {});

    movement._handle_ground_movement("left");

    expect((movement as any).is_colliding).toHaveBeenCalledWith(-2, 0, "left");
    expect(beginStep).toHaveBeenCalledWith(-1, 0, -2, 0, 1, 1);
    expect((movement as any)._last_step_direction).toBe("left");
  });
});

describe("OverworldMovement warp collision parity", () => {
  const createBoundaryWarpMap = (): OverworldMap => {
    const width = 1;
    const height = 2;
    const metatileIds = [1, 0];
    return {
      mapName: "Test",
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

  const createBoundaryWarpTileset = (topCollision: number): OverworldTilesetLike => {
    const floorCollision = resolveCollisionValue("FLOOR");
    return {
      tilesetName: "warp-test",
      metatiles: [
        {
          collision: [floorCollision, floorCollision, floorCollision, floorCollision],
        },
        {
          collision: [topCollision, topCollision, topCollision, topCollision],
        },
      ],
      renderMetatile: () => {},
      renderPriorityMetatile: () => {},
    } as OverworldTilesetLike;
  };

  const createWarpLookup = (): Record<string, WarpEvent[]> => ({
    "1,3": [
      {
        index: 0,
        x: 0,
        y: 0,
        target_map_constant: "TEST_TARGET",
        target_map: "TestTarget",
        target_warp_id: 1,
      },
    ],
  });

  const assignTestAttributes = (movement: TestMovement): void => {
    (movement as any).data_loader = {
      map_attributes: new Map([["Test", { connections: [] } as MapAttributes]]),
    } as unknown as DataLoader;
  };

  it("blocks movement when a warp event sits on a wall tile", () => {
    const movement = new TestMovement();
    (movement as any).map = createBoundaryWarpMap();
    (movement as any).tileset = createBoundaryWarpTileset(resolveCollisionValue("WALL"));
    (movement as any)._warp_tile_lookup = createWarpLookup();
    (movement as any).player_x = 1;
    (movement as any).player_y = 5;
    (movement as any).player_direction = "up";
    assignTestAttributes(movement);

    expect(movement.is_colliding(1, 3, "up")).toBe(true);
  });

  it("keeps movement passable when the warp tile collision itself is a warp permission", () => {
    const movement = new TestMovement();
    (movement as any).map = createBoundaryWarpMap();
    (movement as any).tileset = createBoundaryWarpTileset(resolveCollisionValue("DOOR"));
    (movement as any)._warp_tile_lookup = createWarpLookup();
    (movement as any).player_x = 1;
    (movement as any).player_y = 5;
    (movement as any).player_direction = "up";
    assignTestAttributes(movement);

    expect(movement.is_colliding(1, 3, "up")).toBe(false);
  });

  it("allows stepping down out of bounds when standing on a downward doorway warp", () => {
    const movement = new TestMovement();
    (movement as any).map = createTestMap();
    (movement as any).tileset = createBoundaryWarpTileset(resolveCollisionValue("DOOR"));
    (movement as any)._warp_tile_lookup = {
      "1,1": [
        {
          index: 0,
          x: 0,
          y: 0,
          target_map_constant: "TEST_TARGET",
          target_map: "TestTarget",
          target_warp_id: 1,
        },
      ],
    };
    (movement as any).player_x = 1;
    (movement as any).player_y = 1;
    (movement as any).player_direction = "down";
    assignTestAttributes(movement);

    expect(movement.is_colliding(1, 3, "down")).toBe(false);
  });
});

describe("OverworldMovement ASM collision sampling", () => {
  const createParityMap = (): OverworldMap => {
    const width = 1;
    const height = 1;
    const metatileIds = [0];
    return {
      mapName: "ParityTest",
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

  const createParityTileset = (collision: number[]): OverworldTilesetLike => {
    return {
      tilesetName: "parity",
      metatiles: [{ collision }],
      renderMetatile: () => {},
      renderPriorityMetatile: () => {},
    } as OverworldTilesetLike;
  };

  const assignAttributes = (movement: TestMovement): void => {
    movement.data_loader = {
      map_attributes: new Map([["ParityTest", { connections: [] } as MapAttributes]]),
    } as unknown as DataLoader;
    movement.current_map_name = "ParityTest";
  };

  it("blocks only when the ASM-facing tile is blocked", () => {
    const movement = new TestMovement();
    movement.map = createParityMap();
    movement.tileset = createParityTileset([
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("WALL"),
      resolveCollisionValue("FLOOR"),
    ]);
    movement.player_direction = "right";
    assignAttributes(movement);

    expect(movement.is_colliding(1, 2, "right")).toBe(true);
  });

  it("does not block when a non-leading subtile is blocked but the ASM-facing tile is clear", () => {
    const movement = new TestMovement();
    movement.map = createParityMap();
    movement.tileset = createParityTileset([
      resolveCollisionValue("WALL"),
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("FLOOR"),
    ]);
    movement.player_direction = "right";
    assignAttributes(movement);

    expect(movement.is_colliding(1, 2, "right")).toBe(false);
  });

  it("blocks when a non-leading destination footprint subtile is PC collision", () => {
    const movement = new TestMovement();
    movement.map = createParityMap();
    movement.tileset = createParityTileset([
      resolveCollisionValue("PC"),
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("FLOOR"),
    ]);
    movement.player_direction = "right";
    assignAttributes(movement);

    expect(movement.is_colliding(1, 2, "right")).toBe(true);
  });

  it("checks only the current anchor tile when forcing downward movement on warp tiles", () => {
    const movement = new TestMovement();
    movement.map = createParityMap();
    movement.tileset = createParityTileset([
      resolveCollisionValue("DOOR"),
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("FLOOR"),
      resolveCollisionValue("FLOOR"),
    ]);
    movement.player_x = 1;
    movement.player_y = 2;

    expect(
      (movement as unknown as { _current_tile_has_downward_warp: () => boolean })._current_tile_has_downward_warp()
    ).toBe(false);
  });
});

describe("OverworldMovement NPC occupancy lookup", () => {
  class OccupancyLookupMovement extends TestMovement {
    public snapshotLookups = 0;
    public liveLookups = 0;
    public snapshotNpc: OverworldObject | null = null;
    public liveNpc: OverworldObject | null = null;

    protected _npc_occupying_subtile(_x: number, _y: number): OverworldObject | null {
      this.liveLookups += 1;
      return this.liveNpc;
    }

    protected _npc_occupancy_lookup(): (x: number, y: number) => OverworldObject | null {
      const npc = this.snapshotNpc;
      return (_x: number, _y: number) => {
        this.snapshotLookups += 1;
        return npc;
      };
    }
  }

  const createFlatMap = (): OverworldMap => {
    const width = 3;
    const height = 3;
    const metatileIds = new Array(width * height).fill(0);
    return {
      mapName: "Test",
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

  const createFlatMapWithSize = (width: number, height: number): OverworldMap => {
    const metatileIds = new Array(width * height).fill(0);
    return {
      mapName: "Test",
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

  const createFlatTileset = (): OverworldTilesetLike => {
    const floor = resolveCollisionValue("FLOOR");
    return {
      tilesetName: "flat",
      metatiles: [{ collision: [floor, floor, floor, floor] }],
      renderMetatile: () => {},
      renderPriorityMetatile: () => {},
    } as OverworldTilesetLike;
  };

  const assignAttributes = (movement: TestMovement): void => {
    (movement as any).data_loader = {
      map_attributes: new Map([["Test", { connections: [] } as MapAttributes]]),
    } as unknown as DataLoader;
    (movement as any).current_map_name = "Test";
  };

  it("uses snapshot occupancy lookup for collision checks when no object moves", () => {
    const movement = new OccupancyLookupMovement();
    (movement as any).map = createFlatMap();
    (movement as any).tileset = createFlatTileset();
    (movement as any).player_direction = "right";
    assignAttributes(movement);

    expect(movement.is_colliding(3, 3, "right")).toBe(false);
    expect(movement.snapshotLookups).toBeGreaterThan(0);
    expect(movement.liveLookups).toBe(0);
  });

  it("falls back to live occupancy after pushing a strength boulder", () => {
    const movement = new OccupancyLookupMovement();
    (movement as any).map = createFlatMap();
    (movement as any).tileset = createFlatTileset();
    (movement as any).player_direction = "right";
    assignAttributes(movement);

    const occupant = { objectIndex: 1 } as OverworldObject;
    movement.snapshotNpc = occupant;
    movement.liveNpc = null;
    const pushSpy = jest.spyOn(movement as any, "_push_strength_boulder").mockReturnValue(true);

    expect(movement.is_colliding(3, 3, "right")).toBe(false);
    expect(pushSpy).toHaveBeenCalledTimes(1);
    expect(movement.liveLookups).toBeGreaterThan(0);
  });

  it("pushes a live Cianwood-style strength boulder once Strength is active", () => {
    class StrengthPushMovement extends TestMovement {
      public npcs: OverworldObject[] = [];
      public moved: Array<[string | number, number, number]> = [];

      protected override _npc_occupying_subtile(x: number, y: number): OverworldObject | null {
        for (const npc of this.npcs) {
          const footprint = Math.max(1, npc.collisionStride) - 1;
          if (x >= npc.x - footprint && x <= npc.x && y >= npc.y - footprint && y <= npc.y) {
            return npc;
          }
        }
        return null;
      }

      protected override move_object(objectId: string | number, mapX: number, mapY: number): void {
        this.moved.push([objectId, mapX, mapY]);
        const npc = this.npcs.find((candidate) => candidate.objectIndex === objectId);
        if (!npc) {
          throw new Error(`Missing moved object ${objectId}`);
        }
        npc.event.x = mapX;
        npc.event.y = mapY;
        npc.x = mapX * 2 + 1;
        npc.y = mapY * 2 + 1;
      }
    }

    const movement = new StrengthPushMovement();
    (movement as any).map = createFlatMapWithSize(5, 9);
    (movement as any).tileset = createFlatTileset();
    (movement as any).player_direction = "up";
    assignAttributes(movement);
    setBooleanFlag((movement as any).game_state.wram.engine_flags, "ENGINE_STRENGTH_ACTIVE", true);
    (movement as any)._npc_step_blocked = jest.fn(() => false);

    const boulder = new OverworldObject({
      x: 5,
      y: 7,
      sprite: "SPRITE_BOULDER",
      spritemovedata: "SPRITEMOVEDATA_STRENGTH_BOULDER",
      move_range_x: 0,
      move_range_y: 0,
      hram_x: -1,
      hram_y: -1,
      pal: 0,
      object_type: "OBJECTTYPE_SCRIPT",
      radius: 0,
      script: "CianwoodGymBoulder",
      event_flag: "-1",
      object_identifier: "CIANWOODGYM_BOULDER4",
    });
    boulder.objectIndex = 9;
    boulder.setCollisionStride(2);
    boulder.x = 11;
    boulder.y = 15;
    boulder.prevX = 11;
    boulder.prevY = 15;
    movement.npcs = [boulder];

    expect(movement.is_colliding(11, 15, "up")).toBe(false);
    expect(movement.moved).toEqual([[9, 5, 6]]);
    expect(boulder.x).toBe(11);
    expect(boulder.y).toBe(13);
  });

  it("blocks pushing a strength boulder past the map edge", () => {
    class EdgeBoulderMovement extends TestMovement {
      public npcs: OverworldObject[] = [];
      public moved: Array<[string | number, number, number]> = [];

      protected override _npc_occupying_subtile(x: number, y: number): OverworldObject | null {
        for (const npc of this.npcs) {
          const footprint = Math.max(1, npc.collisionStride) - 1;
          if (x >= npc.x - footprint && x <= npc.x && y >= npc.y - footprint && y <= npc.y) {
            return npc;
          }
        }
        return null;
      }

      protected override move_object(objectId: string | number, mapX: number, mapY: number): void {
        this.moved.push([objectId, mapX, mapY]);
      }
    }

    const movement = new EdgeBoulderMovement();
    (movement as any).map = createFlatMapWithSize(3, 3);
    (movement as any).tileset = createFlatTileset();
    (movement as any).player_direction = "right";
    assignAttributes(movement);
    setBooleanFlag((movement as any).game_state.wram.engine_flags, "ENGINE_STRENGTH_ACTIVE", true);
    (movement as any)._npc_step_blocked = jest.fn(() => false);

    const boulder = new OverworldObject({
      x: 2,
      y: 1,
      sprite: "SPRITE_BOULDER",
      spritemovedata: "SPRITEMOVEDATA_STRENGTH_BOULDER",
      move_range_x: 0,
      move_range_y: 0,
      hram_x: -1,
      hram_y: -1,
      pal: 0,
      object_type: "OBJECTTYPE_SCRIPT",
      radius: 0,
      script: "BlackthornGymBoulder",
      event_flag: "-1",
      object_identifier: "BLACKTHORNGYM_BOULDER",
    });
    boulder.objectIndex = 10;
    boulder.setCollisionStride(2);
    boulder.x = 11;
    boulder.y = 3;
    boulder.prevX = 11;
    boulder.prevY = 3;
    movement.npcs = [boulder];

    expect(movement.is_colliding(11, 3, "right")).toBe(true);
    expect(movement.moved).toEqual([]);
    expect(boulder.x).toBe(11);
    expect(boulder.y).toBe(3);
  });

  it("runs the map stonetable script when a strength boulder lands on a pit warp", () => {
    class PitBoulderMovement extends TestMovement {
      public npcs: OverworldObject[] = [];
      public moved: Array<[string | number, number, number]> = [];
      public script_runner = { call: jest.fn() };

      protected override _npc_occupying_subtile(x: number, y: number): OverworldObject | null {
        return this.npcs.find((npc) => npc.x === x && npc.y === y) ?? null;
      }

      protected override move_object(objectId: string | number, mapX: number, mapY: number): void {
        this.moved.push([objectId, mapX, mapY]);
        const npc = this.npcs.find((candidate) => candidate.objectIndex === objectId);
        if (!npc) {
          throw new Error(`Missing moved object ${objectId}`);
        }
        npc.event.x = mapX;
        npc.event.y = mapY;
        npc.x = mapX * 2 + 1;
        npc.y = mapY * 2 + 1;
      }
    }

    const movement = new PitBoulderMovement();
    const map = createFlatMapWithSize(8, 8);
    map.metatileIds[1 * map.width + 2] = 1;
    const floor = resolveCollisionValue("FLOOR");
    const pit = resolveCollisionValue("PIT");
    (movement as any).map = map;
    (movement as any).tileset = {
      tilesetName: "pit-test",
      metatiles: [
        { collision: [floor, floor, floor, floor] },
        { collision: [pit, pit, pit, pit] },
      ],
      renderMetatile: () => {},
      renderPriorityMetatile: () => {},
    } as OverworldTilesetLike;
    (movement as any).player_direction = "up";
    (movement as any).data_loader = {
      map_attributes: new Map([["Test", { connections: [] } as MapAttributes]]),
      map_events: new Map([
        [
          "Test",
          {
            warps: [
              {
                index: 3,
                x: 5,
                y: 2,
                target_map_constant: "TEST_B2F",
                target_map: "TestB2F",
                target_warp_id: 1,
              },
            ],
            coord_events: [],
            bg_events: [],
          },
        ],
      ]),
      map_callbacks: new Map([["Test", [["MAPCALLBACK_CMDQUEUE", "TestSetUpStoneTableCallback"]]]]),
      get_script: jest.fn((scriptName: string) =>
        scriptName === "TestSetUpStoneTableCallback"
          ? [
              { command: "stonetable", args: ["3", "TEST_BOULDER", ".Boulder"] },
              { command: "db", args: ["-1"] },
            ]
          : null
      ),
    } as unknown as DataLoader;
    (movement as any).current_map_name = "Test";
    setBooleanFlag((movement as any).game_state.wram.engine_flags, "ENGINE_STRENGTH_ACTIVE", true);
    (movement as any)._npc_step_blocked = jest.fn(() => false);

    const boulder = new OverworldObject({
      x: 5,
      y: 3,
      sprite: "SPRITE_BOULDER",
      spritemovedata: "SPRITEMOVEDATA_STRENGTH_BOULDER",
      move_range_x: 0,
      move_range_y: 0,
      hram_x: -1,
      hram_y: -1,
      pal: 0,
      object_type: "OBJECTTYPE_SCRIPT",
      radius: 0,
      script: "TestBoulder",
      event_flag: "EVENT_TEST_BOULDER",
      object_identifier: "TEST_BOULDER",
    });
    boulder.objectIndex = 7;
    boulder.setCollisionStride(2);
    boulder.x = 11;
    boulder.y = 7;
    movement.npcs = [boulder];

    expect(movement.is_colliding(11, 7, "up")).toBe(false);
    expect(movement.moved).toEqual([[7, 5, 2]]);
    expect(movement.script_runner.call).toHaveBeenCalledWith(".Boulder", "TestSetUpStoneTableCallback");
  });

  it("blocks when an NPC occupies any subtile in the destination footprint", () => {
    class FootprintOccupancyMovement extends OccupancyLookupMovement {
      private readonly occupant = { objectIndex: 7 } as OverworldObject;

      protected override _npc_occupying_subtile(x: number, y: number): OverworldObject | null {
        this.liveLookups += 1;
        return x === 2 && y === 2 ? this.occupant : null;
      }

      protected override _npc_occupancy_lookup(): (x: number, y: number) => OverworldObject | null {
        return (x: number, y: number) => {
          this.snapshotLookups += 1;
          return x === 2 && y === 2 ? this.occupant : null;
        };
      }
    }

    const movement = new FootprintOccupancyMovement();
    (movement as any).map = createFlatMap();
    (movement as any).tileset = createFlatTileset();
    (movement as any).player_direction = "right";
    assignAttributes(movement);

    expect(movement.is_colliding(3, 3, "right")).toBe(true);
  });
});

import { MovementTask, type MovementOverworldContext, type MovementTarget } from "./movement-task";
import * as debugLog from "@pokecrystal/core/core/debug-log";
import { PlayerCharacter } from "../playable-character";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile/constants";

describe("MovementTask debug logging", () => {
  const originalDebug = process.env.POKE_DEBUG;

  afterEach(() => {
    process.env.POKE_DEBUG = originalDebug;
    jest.restoreAllMocks();
  });

  it("skips movement logs when debug flags are disabled", () => {
    process.env.POKE_DEBUG = "";
    const spy = jest.spyOn(debugLog, "pushDebugLog").mockImplementation(() => {});
    const task = new MovementTask({ x: 1, y: 1, name: "NPC" }, ["step_end"]);
    const overworld = {} as MovementOverworldContext;

    task.start(overworld);
    task.finish(overworld);

    expect(spy).not.toHaveBeenCalled();
  });

  it("emits movement logs when task debugging is enabled", () => {
    process.env.POKE_DEBUG = "tasks";
    const spy = jest.spyOn(debugLog, "pushDebugLog").mockImplementation(() => {});
    const task = new MovementTask({ x: 1, y: 1, name: "NPC" }, ["step_end"]);
    const overworld = {} as MovementOverworldContext;

    task.start(overworld);
    task.finish(overworld);

    expect(spy).toHaveBeenCalledTimes(2);
    expect(spy.mock.calls[0][0]).toContain("[task] movement start");
    expect(spy.mock.calls[1][0]).toContain("[task] movement done");
  });
});

describe("MovementTask last-coord syncing", () => {
  it("mirrors ASM last-map updates across a scripted step", () => {
    const target: MovementTarget = {
      x: 2,
      y: 3,
      name: "NPC",
      pixelX: 0,
      pixelY: 0,
      targetPixelX: 0,
      targetPixelY: 0,
      walking: false,
      jumping: false,
      direction: "down",
    };
    const task = new MovementTask(target, ["step RIGHT", "step_end"]);
    const overworld = { WALK_FRAMES: 1, TILES_PER_COLLISION: 2 } as MovementOverworldContext;

    task.start(overworld);

    expect(target.prev_x).toBe(2);
    expect(target.prev_y).toBe(3);
    expect(target.prevX).toBe(2);
    expect(target.prevY).toBe(3);
    expect(target.x).toBe(4);
    expect(target.y).toBe(3);

    task.update(overworld);

    expect(target.walking).toBe(false);
    expect(target.prev_x).toBe(4);
    expect(target.prev_y).toBe(3);
    expect(target.prevX).toBe(4);
    expect(target.prevY).toBe(3);
  });

  it("still lets independently scripted actors claim the same tile", () => {
    const first: MovementTarget = {
      x: 1,
      y: 1,
      name: "FIRST",
      pixelX: 0,
      pixelY: 0,
      targetPixelX: 0,
      targetPixelY: 0,
      walking: false,
      jumping: false,
      direction: "right",
    };
    const second: MovementTarget = {
      x: 5,
      y: 1,
      name: "SECOND",
      pixelX: 0,
      pixelY: 0,
      targetPixelX: 0,
      targetPixelY: 0,
      walking: false,
      jumping: false,
      direction: "left",
    };
    const overworld = {
      WALK_FRAMES: 1,
      TILES_PER_COLLISION: 2,
      _npc_step_blocked: jest.fn(() => true),
    } as unknown as MovementOverworldContext;

    new MovementTask(first, ["step RIGHT", "step_end"], { blocking: false }).start(overworld);
    new MovementTask(second, ["step LEFT", "step_end"], { blocking: false }).start(overworld);

    expect(first.x).toBe(3);
    expect(second.x).toBe(3);
    expect((overworld as any)._npc_step_blocked).toHaveBeenCalledTimes(2);
  });
});

describe("MovementTask player pixel proxies", () => {
  it("keeps snake_case pixel access in sync with overworld state", () => {
    const overworld = {
      player_x: 10,
      player_y: 11,
      prev_player_x: 10,
      prev_player_y: 11,
      player_px_x: 64,
      player_px_y: 72,
      target_px_x: 64,
      target_px_y: 72,
      step_dx_px: 0,
      step_dy_px: 0,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      WALK_FRAMES: 8,
    } as unknown as MovementOverworldContext;
    const player = new PlayerCharacter(overworld as any);

    expect(player.pixel_x).toBe(64);
    expect(player.pixel_y).toBe(72);

    player.pixel_x = 80;
    player.pixel_y = 88;

    expect(overworld.player_px_x).toBe(80);
    expect(overworld.player_px_y).toBe(88);
  });

  it("advances scripted player steps from overworld pixel coordinates", () => {
    const overworld = {
      player_x: 10,
      player_y: 11,
      prev_player_x: 10,
      prev_player_y: 11,
      player_px_x: 64,
      player_px_y: 72,
      target_px_x: 64,
      target_px_y: 72,
      step_dx_px: 0,
      step_dy_px: 0,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      WALK_FRAMES: 8,
    } as unknown as MovementOverworldContext;
    const player = new PlayerCharacter(overworld as any);
    (player as any)._pixelX = 0;
    (player as any)._pixelY = 0;

    const task = new MovementTask(player as any, ["step right", "step_end"]);
    task.start(overworld);

    let guard = 0;
    while (!task.completed && guard < 32) {
      task.update(overworld);
      guard += 1;
    }

    const stride = overworld.TILES_PER_COLLISION ?? 1;
    const expectedPixelX = 64 + stride * TILE_SIZE;

    expect(guard).toBeLessThan(32);
    expect(overworld.player_px_x).toBe(expectedPixelX);
    expect(overworld.player_x).toBe(10 + stride);
  });
});

describe("MovementTask jump steps", () => {
  it("advances two tiles and mirrors the ASM jump offsets for jump_step", () => {
    const target: MovementTarget = {
      x: 0,
      y: 0,
      name: "NPC",
      pixelX: 0,
      pixelY: 0,
      targetPixelX: 0,
      targetPixelY: 0,
      walking: false,
      jumping: false,
      direction: "down",
      sprite_y_offset: 0,
    };
    const task = new MovementTask(target, ["jump_step DOWN", "step_end"]);
    const overworld = { WALK_FRAMES: 8, TILES_PER_COLLISION: 2 } as MovementOverworldContext;

    task.start(overworld);

    const offsets: number[] = [];
    let guard = 0;
    while (!task.completed && guard < 64) {
      task.update(overworld);
      offsets.push(target.sprite_y_offset ?? 0);
      guard += 1;
    }

    expect(guard).toBeLessThan(64);
    expect(offsets).toEqual([
      -4, -6, -8, -10, -11, -12, -12, -12,
      -11, -10, -9, -8, -6, -4, 0, 0,
    ]);
    expect(target.y).toBe(4);
    expect(target.pixelY).toBe(32);
  });

  it("uses the fast jump arc for fast_jump_step", () => {
    const target: MovementTarget = {
      x: 0,
      y: 0,
      name: "NPC",
      pixelX: 0,
      pixelY: 0,
      targetPixelX: 0,
      targetPixelY: 0,
      walking: false,
      jumping: false,
      direction: "down",
      sprite_y_offset: 0,
    };
    const task = new MovementTask(target, ["fast_jump_step DOWN", "step_end"]);
    const overworld = { WALK_FRAMES: 8, TILES_PER_COLLISION: 2 } as MovementOverworldContext;

    task.start(overworld);

    const offsets: number[] = [];
    let guard = 0;
    while (!task.completed && guard < 32) {
      task.update(overworld);
      offsets.push(target.sprite_y_offset ?? 0);
      guard += 1;
    }

    expect(guard).toBeLessThan(32);
    expect(offsets).toEqual([-4, -8, -11, -12, -11, -9, -6, 0]);
    expect(target.y).toBe(4);
    expect(target.pixelY).toBe(32);
  });
});

describe("MovementTask scripted effect commands", () => {
  it("runs skyfall_top as the ASM stationary fall-top animation", () => {
    const target: MovementTarget = {
      x: 21,
      y: 19,
      name: "PLAYER",
      pixelX: 64,
      pixelY: 72,
      targetPixelX: 64,
      targetPixelY: 72,
      stepFramesRemaining: 0,
      stepTotalFrames: 0,
      walking: true,
      jumping: true,
      direction: "left",
      sprite_y_offset: 0,
    };
    const task = new MovementTask(target, ["skyfall_top", "step_end"]);
    const overworld = { WALK_FRAMES: 8, TILES_PER_COLLISION: 2 } as MovementOverworldContext;

    task.start(overworld);

    expect(task.completed).toBe(false);
    expect(target.stepFramesRemaining).toBe(16);
    expect(target.walking).toBe(false);
    expect(target.jumping).toBe(false);

    let guard = 0;
    while (!task.completed && guard < 32) {
      task.update(overworld);
      guard += 1;
    }
    task.finish(overworld);

    expect(guard).toBe(16);
    expect(target.x).toBe(21);
    expect(target.y).toBe(19);
    expect(target.sprite_y_offset).toBe(0x60);
  });

  it("accepts ASM sliding and teleport effect commands without displacement", () => {
    const target: MovementTarget = {
      x: 5,
      y: 6,
      name: "NPC",
      pixelX: 16,
      pixelY: 24,
      targetPixelX: 16,
      targetPixelY: 24,
      stepFramesRemaining: 0,
      stepTotalFrames: 0,
      walking: false,
      jumping: false,
      sliding: false,
      _sliding: false,
      direction: "down",
      sprite_y_offset: 0,
    };
    const task = new MovementTask(target, ["set_sliding", "teleport_from", "remove_sliding", "teleport_to", "step_end"]);
    const overworld = { WALK_FRAMES: 8, TILES_PER_COLLISION: 2 } as MovementOverworldContext;

    task.start(overworld);

    expect(target.sliding).toBe(true);
    expect(target._sliding).toBe(true);
    expect(target.stepFramesRemaining).toBe(24);

    for (let i = 0; i < 24; i += 1) {
      task.update(overworld);
    }

    expect(task.completed).toBe(false);
    expect(target.sliding).toBe(false);
    expect(target._sliding).toBe(false);
    expect(target.stepFramesRemaining).toBe(64);

    let guard = 0;
    while (!task.completed && guard < 96) {
      task.update(overworld);
      guard += 1;
    }
    task.finish(overworld);

    expect(guard).toBe(64);
    expect(target.x).toBe(5);
    expect(target.y).toBe(6);
    expect(target.sprite_y_offset).toBe(0);
  });
});

describe("MovementTask scripted collision bypass", () => {
  it("probes collision with quiet scripted-movement options while preserving the cutscene step", () => {
    const target: MovementTarget = {
      object_id: "ELMSLAB_ELM",
      x: 7,
      y: 7,
      pixelX: 0,
      pixelY: 0,
      targetPixelX: 0,
      targetPixelY: 0,
      walking: false,
      jumping: false,
      direction: "down",
    };
    const overworld = {
      WALK_FRAMES: 1,
      TILES_PER_COLLISION: 2,
      _npc_step_blocked: jest.fn(() => true),
    } as unknown as MovementOverworldContext;
    const task = new MovementTask(target, ["step DOWN", "step_end"]);

    task.start(overworld);

    expect((overworld as any)._npc_step_blocked).toHaveBeenCalledWith(target, "down", 7, 9, {
      suppress_blocked_log: true,
    });
    expect(target.y).toBe(9);
  });
});

describe("PlayerCharacter apply_movement", () => {
  it("moves jump_step by two tiles", () => {
    const overworld = {
      player_x: 5,
      player_y: 5,
      prev_player_x: 5,
      prev_player_y: 5,
      player_px_x: 0,
      player_px_y: 0,
      target_px_x: 0,
      target_px_y: 0,
      step_dx_px: 0,
      step_dy_px: 0,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      WALK_FRAMES: 8,
    } as unknown as MovementOverworldContext;

    const player = new PlayerCharacter(overworld as any);

    player.apply_movement(["jump_step DOWN", "step_end"]);

    expect(overworld.player_y).toBe(9);
  });
});

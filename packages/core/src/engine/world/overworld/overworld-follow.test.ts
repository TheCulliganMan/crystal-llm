import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";

describe("Overworld follower queue", () => {
  type Leader = { x: number; y: number };
  type Follower = { x: number; y: number; collision_stride: number };
  type FollowerQueueOverworld = Record<string, unknown> & {
    leader: Leader;
    follower: Follower;
    npcs: Leader[];
    _compute_initial_follow_step: (...args: unknown[]) => unknown;
    _normalize_follower_step: (...args: unknown[]) => unknown;
    _cancel_follow_if_leader_missing: (...args: unknown[]) => unknown;
    _follower_step_queue: string[];
    _follower_queue_length: number;
    _follower_movement_task: unknown;
    _active_follower_target: unknown;
    _pending_follower_origin: unknown;
    _last_step_direction: string | null;
    _try_start_follower_step: jest.Mock;
  };

  const queueFirstStep = (overworld: FollowerQueueOverworld, leader: Leader, follower: Follower) =>
    (OverworldEngine.prototype as any)._queue_follower_first_step.call(overworld, leader, follower);
  const enqueueFollowerStep = (overworld: FollowerQueueOverworld, direction: string, originX: number, originY: number) =>
    (OverworldEngine.prototype as any)._enqueue_follower_step.call(overworld, direction, originX, originY);
  const nextFollowerStep = (overworld: FollowerQueueOverworld) =>
    (OverworldEngine.prototype as any)._get_next_follower_step.call(overworld);

  it("buffers the initial step until a leader movement is queued (ASM QueueFollowerFirstStep)", () => {
    const leader: Leader = { x: 5, y: 0 };
    const follower: Follower = { x: 0, y: 0, collision_stride: 1 };
    const overworld: FollowerQueueOverworld = {
      leader,
      follower,
      npcs: [leader],
      _compute_initial_follow_step: (OverworldEngine.prototype as any)._compute_initial_follow_step,
      _normalize_follower_step: (OverworldEngine.prototype as any)._normalize_follower_step,
      _cancel_follow_if_leader_missing: (OverworldEngine.prototype as any)._cancel_follow_if_leader_missing,
      _follower_step_queue: [],
      _follower_queue_length: 0,
      _follower_movement_task: null,
      _active_follower_target: null,
      _pending_follower_origin: null,
      _last_step_direction: null,
      _try_start_follower_step: jest.fn(),
    };

    queueFirstStep(overworld, leader, follower);
    expect(overworld._follower_queue_length).toBe(0);
    expect(overworld._follower_step_queue).toEqual(["step right"]);
    expect(nextFollowerStep(overworld)).toBeNull();

    enqueueFollowerStep(overworld, "right", 5, 0);
    expect(overworld._follower_queue_length).toBe(1);
    expect(overworld._follower_step_queue[1]).toBe("step right");
    expect(nextFollowerStep(overworld)).toBe("step right");
    expect(overworld._follower_queue_length).toBe(0);
    expect(overworld._follower_step_queue[0]).toBe("step right");
  });

  it("accepts scripted step commands when the leader uses applymovement (ASM ApplyMovementToFollower)", () => {
    const leader: Leader = { x: 5, y: 0 };
    const follower: Follower = { x: 0, y: 0, collision_stride: 1 };
    const overworld: FollowerQueueOverworld = {
      leader,
      follower,
      npcs: [leader],
      _compute_initial_follow_step: (OverworldEngine.prototype as any)._compute_initial_follow_step,
      _normalize_follower_step: (OverworldEngine.prototype as any)._normalize_follower_step,
      _cancel_follow_if_leader_missing: (OverworldEngine.prototype as any)._cancel_follow_if_leader_missing,
      _follower_step_queue: [],
      _follower_queue_length: 0,
      _follower_movement_task: null,
      _active_follower_target: null,
      _pending_follower_origin: null,
      _last_step_direction: null,
      _try_start_follower_step: jest.fn(),
    };

    queueFirstStep(overworld, leader, follower);
    enqueueFollowerStep(overworld, "step right", 5, 0);

    expect(overworld._follower_queue_length).toBe(1);
    expect(overworld._follower_step_queue[1]).toBe("step right");
  });

  it("stays one step behind when starting on the same tile (ASM QueueFollowerFirstStep)", () => {
    const leader: Leader = { x: 2, y: 2 };
    const follower: Follower = { x: 2, y: 2, collision_stride: 1 };
    const overworld: FollowerQueueOverworld = {
      leader,
      follower,
      npcs: [leader],
      _compute_initial_follow_step: (OverworldEngine.prototype as any)._compute_initial_follow_step,
      _normalize_follower_step: (OverworldEngine.prototype as any)._normalize_follower_step,
      _cancel_follow_if_leader_missing: (OverworldEngine.prototype as any)._cancel_follow_if_leader_missing,
      _follower_step_queue: [],
      _follower_queue_length: 0,
      _follower_movement_task: null,
      _active_follower_target: null,
      _pending_follower_origin: null,
      _last_step_direction: null,
      _try_start_follower_step: jest.fn(),
    };

    queueFirstStep(overworld, leader, follower);
    expect(overworld._follower_queue_length).toBe(-1);
    expect(overworld._follower_step_queue).toEqual([]);

    enqueueFollowerStep(overworld, "up", 2, 2);
    expect(overworld._follower_queue_length).toBe(0);
    expect(nextFollowerStep(overworld)).toBeNull();

    enqueueFollowerStep(overworld, "left", 2, 1);
    expect(overworld._follower_queue_length).toBe(1);
    expect(nextFollowerStep(overworld)).toBe("step up");
    expect(overworld._follower_queue_length).toBe(0);
  });

  it("does not snap the follower to a queued leader origin when resetting (ASM ApplyMovementToFollower)", () => {
    type Follower = {
      x: number;
      y: number;
      updatePixelPosition: jest.Mock;
      walking?: boolean;
      jumping?: boolean;
      sprite_y_offset?: number;
    };
    type ResetFollowerOverworld = {
      follower: Follower | null;
      _follower_movement_task: null;
      _active_follower_target: [number, number] | null;
      _pending_follower_origin: [number, number] | null;
      _follower_step_queue: string[];
      _follower_queue_length: number;
      _last_step_direction: string | null;
      _finalise_follower_position: () => void;
    };
    const follower: Follower = {
      x: 3,
      y: 4,
      updatePixelPosition: jest.fn(),
      walking: true,
      jumping: true,
      sprite_y_offset: 5,
    };
    const overworld: ResetFollowerOverworld = {
      follower,
      _follower_movement_task: null,
      _active_follower_target: null,
      _pending_follower_origin: [9, 9],
      _follower_step_queue: ["step up"],
      _follower_queue_length: 1,
      _last_step_direction: "up",
      _finalise_follower_position: (OverworldEngine.prototype as any)._finalise_follower_position,
    };

    (OverworldEngine.prototype as any)._reset_follower_path.call(overworld);

    expect(follower.x).toBe(3);
    expect(follower.y).toBe(4);
    expect(follower.updatePixelPosition).toHaveBeenCalledTimes(1);
    expect(overworld._pending_follower_origin).toBeNull();
    expect(overworld._follower_step_queue).toEqual([]);
    expect(overworld._follower_queue_length).toBe(0);
  });

  it("finalises follower steps using the active target instead of queued leader origins", () => {
    type Follower = {
      x: number;
      y: number;
      updatePixelPosition: jest.Mock;
      walking?: boolean;
      jumping?: boolean;
      sprite_y_offset?: number;
    };
    type FinaliseFollowerOverworld = {
      follower: Follower | null;
      _follower_movement_task: null;
      _active_follower_target: [number, number] | null;
      _pending_follower_origin: [number, number] | null;
      _follower_step_queue: string[];
      _follower_queue_length: number;
    };
    const follower: Follower = {
      x: 0,
      y: 0,
      updatePixelPosition: jest.fn(),
      walking: true,
      jumping: true,
      sprite_y_offset: 4,
    };
    const overworld: FinaliseFollowerOverworld = {
      follower,
      _follower_movement_task: null,
      _active_follower_target: [6, 7],
      _pending_follower_origin: [1, 1],
      _follower_step_queue: ["step right"],
      _follower_queue_length: 1,
    };

    (OverworldEngine.prototype as any)._finalise_follower_position.call(overworld);

    expect(follower.x).toBe(6);
    expect(follower.y).toBe(7);
    expect(follower.updatePixelPosition).toHaveBeenCalledTimes(1);
    expect(overworld._active_follower_target).toBeNull();
    expect(overworld._pending_follower_origin).toBeNull();
  });
});

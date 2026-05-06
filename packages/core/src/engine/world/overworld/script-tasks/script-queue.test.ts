import { OverworldScriptQueueMixin } from "./script-queue";
import { ScriptTask } from "./script-task";

class TestTask extends ScriptTask {
  update(): void {
    return;
  }
}

class TestOverworld extends OverworldScriptQueueMixin {
  public _script_task_queue: ScriptTask[] = [];
  public _active_script_task: ScriptTask | null = null;
  public _blocking_task_count = 0;
  public _blocking_movement_lock_active = false;
  public lock_player_movement = jest.fn();
  public unlock_player_movement = jest.fn();
  public _logger = null;
  public _maybe_process_idle_phone_calls = jest.fn();
  public _clear_stale_blocking_tasks = jest.fn();
  public _movement_lock_count = 0;
  public script_runner: { _script_stack?: unknown[]; _awaiting_resume?: number; is_busy?: boolean } | null = null;
}

class CollisionOverworld extends TestOverworld {
  public TILES_PER_COLLISION = 2;
  public WALK_FRAMES = 1;
  public _npc_step_blocked = jest.fn(
    (
      _npc: unknown,
      _direction: string,
      targetX: number,
      targetY: number,
      options?: { player_only?: boolean; suppress_blocked_log?: boolean },
    ) => Boolean(options?.player_only && targetX === 3 && targetY === 1)
  );
}

class PhoneCallOverworld extends OverworldScriptQueueMixin {
  public game_state = { wram: { scheduled_phone_calls: ["ELM"] } };
  public script_runner = null;
  public is_moving = false;
  public observed_game_state: unknown = null;

  public _process_special_phone_call(): boolean {
    this.observed_game_state = this.game_state;
    return true;
  }
}

describe("OverworldScriptQueueMixin", () => {
  it("aliases player_movement_locked to playerMovementLocked", () => {
    const mixin = new TestOverworld();
    mixin._movement_lock_count = 0;
    expect(mixin.player_movement_locked()).toBe(false);
    mixin._movement_lock_count = 2;
    expect(mixin.player_movement_locked()).toBe(true);
  });

  it("locks player movement while a script is running", () => {
    const mixin = new TestOverworld();
    mixin.script_runner = { _script_stack: [{}] };
    expect(mixin.player_movement_locked()).toBe(true);
  });

  it("keeps the active task when the queue is empty", () => {
    const overworld = new TestOverworld();
    const task = new TestTask();

    overworld._enqueue_script_task(task);

    expect(overworld._active_script_task).toBe(task);
    expect(overworld._blocking_task_count).toBe(1);
    expect(overworld.lock_player_movement).toHaveBeenCalledTimes(1);

    overworld._process_script_tasks();
    expect(overworld._active_script_task).toBe(task);
    expect(overworld._blocking_task_count).toBe(1);
  });

  it("binds special phone call processing to the overworld context", () => {
    const overworld = new PhoneCallOverworld();

    expect(() => overworld._maybe_process_idle_phone_calls()).not.toThrow();
    expect(overworld.observed_game_state).toBe(overworld.game_state);
  });

  it("skips queueDelay gracefully when the task queue is unavailable", () => {
    const overworld = new TestOverworld();
    overworld._enqueue_script_task = undefined as unknown as typeof overworld._enqueue_script_task;

    const scheduled = overworld.queueDelay(4, { onComplete: jest.fn() });

    expect(scheduled).toBe(false);
    expect(overworld._script_task_queue).toHaveLength(0);
    expect(overworld.lock_player_movement).not.toHaveBeenCalled();
  });

  it("returns false when queueDelay is invoked without a bound overworld", () => {
    const queueDelay = (TestOverworld.prototype as unknown as { queueDelay: typeof TestOverworld.prototype.queueDelay }).queueDelay;
    expect(queueDelay.call(undefined as unknown as TestOverworld, 4, { onComplete: jest.fn() })).toBe(false);
  });

  it("blocks queued NPC movement from stepping onto the player", () => {
    const overworld = new CollisionOverworld();
    const npc = {
      x: 1,
      y: 1,
      prevX: 1,
      prevY: 1,
      pixelX: 0,
      pixelY: 0,
      targetPixelX: 0,
      targetPixelY: 0,
      walking: false,
      jumping: false,
      direction: "right",
      name: "NPC",
    };
    const onComplete = jest.fn();

    overworld.queueMovementTask(npc as any, ["step RIGHT", "step_end"], { onComplete });

    expect(overworld._npc_step_blocked).toHaveBeenCalledWith(npc, "right", 3, 1, {
      player_only: true,
      suppress_blocked_log: true,
    });
    expect(npc.x).toBe(1);
    expect(npc.y).toBe(1);
    expect(npc.walking).toBe(false);
    expect(onComplete).toHaveBeenCalledTimes(1);
  });
});

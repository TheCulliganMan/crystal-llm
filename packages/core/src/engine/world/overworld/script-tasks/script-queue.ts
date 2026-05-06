import { PendingEventFlagUpdate, normalizePendingEventFlagUpdate } from "../pending-event-flag-updates";
import { ScriptRunnerState } from "@pokecrystal/core/engine/world/story-events/runner";
import type { OverworldObject } from "../overworld-object";
import { DelayTask } from "./delay-task";
import { FollowTask } from "./follow-task";
import { MovementTask } from "./movement-task";
import { ScriptTask } from "./script-task";
import { pushDebugLog } from "@pokecrystal/core/core/debug-log";
import { isDebugEnabled } from "@pokecrystal/core/core/debug-flags";
import type { OverworldContext } from "@pokecrystal/core/engine/world/story-events/commands/base";

const isInfoEnabled = (): boolean => {
  const level = process.env.POKECRYSTAL_LOG_LEVEL?.toLowerCase();
  return level === "info" || level === "debug";
};

const isWarnEnabled = (): boolean => {
  const level = process.env.POKECRYSTAL_LOG_LEVEL?.toLowerCase();
  return level === "warn" || level === "warning" || level === "info" || level === "debug";
};

type ScriptRunnerQueueState = {
  _script_stack?: unknown[];
  _awaiting_resume?: unknown;
  state?: ScriptRunnerState;
  is_busy?: boolean;
};

type OverworldScriptQueueContext = OverworldContext & {
  _movement_lock_count?: number;
  _enqueue_script_task: (task: ScriptTask<OverworldScriptQueueContext>) => void;
  _try_start_next_script_task: () => void;
  _clear_stale_blocking_tasks: () => void;
  _maybe_process_idle_phone_calls: () => void;
  _finalise_script_task: (task: ScriptTask<OverworldScriptQueueContext>) => void;
  lock_player_movement?: () => void;
  unlock_player_movement?: () => void;
  queue_movement_task?: (
    actor: OverworldObject,
    movementCommands: string[],
    options: { onComplete: () => void },
  ) => void;
  start_following?: (
    follower: OverworldObject,
    leader: OverworldObject,
    options: { follower_id: string | null; leader_id: string | null },
  ) => void;
  queue_follow_task?: (
    follower: OverworldObject,
    leader: OverworldObject,
    options: { onComplete?: (() => void) | null },
  ) => void;
  scriptTasksActive: () => boolean;
  _script_task_queue?: Array<ScriptTask<OverworldScriptQueueContext>>;
  _active_script_task?: ScriptTask<OverworldScriptQueueContext> | null;
  _blocking_task_count?: number;
  _blocking_movement_lock_active?: boolean;
  _pending_event_flag_updates?: PendingEventFlagUpdate[];
  script_runner?: ScriptRunnerQueueState | null;
  game_state?: { wram?: { scheduled_phone_calls?: unknown[] } };
  _apply_event_flag_update?: (eventName: string, value: boolean) => void;
  _logger?: { warn?: (message: string) => void };
  _process_special_phone_call?: () => boolean;
  is_moving?: boolean;
};

const asScriptQueueContext = (value: unknown): OverworldScriptQueueContext =>
  value as OverworldScriptQueueContext;

export class OverworldScriptQueueMixin {
  playerMovementLocked(): boolean {
    const overworld = asScriptQueueContext(this);
    if ((overworld._movement_lock_count ?? 0) > 0) {
      return true;
    }
    const runner = overworld.script_runner;
    if (!runner) {
      return false;
    }
    // ASM: engine/overworld/events.asm::PlayerEvents blocks OWPlayerInput when wScriptRunning is set.
    const runnerAny = runner as { is_busy?: unknown; _script_stack?: unknown[]; _awaiting_resume?: unknown; state?: ScriptRunnerState };
    const isBusy = runnerAny.is_busy;
    if (typeof isBusy === "boolean") {
      return isBusy;
    }
    if (typeof isBusy === "function") {
      return Boolean(isBusy.call(runner));
    }
    const stackDepth = Array.isArray(runnerAny._script_stack) ? runnerAny._script_stack.length : 0;
    const awaitingResume = typeof runnerAny._awaiting_resume === "number"
      ? runnerAny._awaiting_resume
      : Number(runnerAny._awaiting_resume ?? 0);
    return (
      stackDepth > 0
      || awaitingResume > 0
      || runnerAny.state === ScriptRunnerState.RUNNING
      || runnerAny.state === ScriptRunnerState.PAUSED
    );
  }

  player_movement_locked(): boolean {
    return this.playerMovementLocked();
  }

  queueDelay(frames: number, options: { onComplete: () => void; blocking?: boolean }): boolean {
    if (frames <= 0) {
      return false;
    }
    const overworld = asScriptQueueContext(this);
    if (!overworld) {
      return false;
    }
    if (typeof overworld._enqueue_script_task !== "function") {
      if (isWarnEnabled()) {
        overworld._logger?.warn?.("queueDelay skipped: script task queue unavailable.");
      }
      return false;
    }
    const task = new DelayTask(frames, {
      onComplete: options.onComplete,
      blocking: options.blocking,
    }) as ScriptTask<OverworldScriptQueueContext>;
    overworld._enqueue_script_task(task);
    return true;
  }

  queueMovement(
    actor: OverworldObject,
    movementCommands: string[],
    options: { onComplete: () => void },
  ): boolean {
    if (!movementCommands?.length) {
      return false;
    }
    this.queueMovementTask(actor, movementCommands, { onComplete: options.onComplete });
    return true;
  }

  queueFollow(
    follower: OverworldObject,
    leader: OverworldObject,
    options: { followerId?: string | null; leaderId?: string | null; onComplete: () => void },
  ): boolean {
    const overworld = asScriptQueueContext(this);
    overworld.start_following?.(follower, leader, {
      follower_id: options.followerId ?? null,
      leader_id: options.leaderId ?? null,
    });
    this.queueFollowTask(follower, leader, { onComplete: options.onComplete });
    return true;
  }

  queueMovementTask(
    obj: OverworldObject,
    movementCommands: string[],
    options: { onComplete?: (() => void) | null } = {},
  ): void {
    const overworld = asScriptQueueContext(this);
    const task = new MovementTask(
      obj as unknown as ConstructorParameters<typeof MovementTask>[0],
      movementCommands,
      {
        onComplete: options.onComplete ?? null,
        respectPlayerCollision: true,
      },
    ) as ScriptTask<OverworldScriptQueueContext>;
    overworld._enqueue_script_task(task);
  }

  queueFollowTask(
    follower: OverworldObject,
    leader: OverworldObject,
    options: { onComplete?: (() => void) | null } = {},
  ): void {
    const overworld = asScriptQueueContext(this);
    const task = new FollowTask(follower, leader, {
      onComplete: options.onComplete ?? null,
    }) as ScriptTask<OverworldScriptQueueContext>;
    overworld._enqueue_script_task(task);
  }

  scriptTasksActive(): boolean {
    const overworld = asScriptQueueContext(this);
    if (overworld._active_script_task) {
      return true;
    }
    return Boolean(overworld._script_task_queue?.length);
  }

  script_tasks_active(): boolean {
    return this.scriptTasksActive();
  }

  _enqueue_script_task(task: ScriptTask): void {
    const overworld = asScriptQueueContext(this);
    const trace = isDebugEnabled("script:tasks") || isDebugEnabled("tasks") || isDebugEnabled("script");
    if (task.blocking) {
      if ((overworld._blocking_task_count ?? 0) === 0) {
        overworld.lock_player_movement?.();
        overworld._blocking_movement_lock_active = true;
      }
      overworld._blocking_task_count = (overworld._blocking_task_count ?? 0) + 1;
    }
    if (!overworld._script_task_queue) {
      overworld._script_task_queue = [];
    }
    overworld._script_task_queue.push(task);
    if (trace) {
      pushDebugLog(`[task] enqueue ${task.constructor?.name ?? "ScriptTask"}`, {
        blocking: task.blocking,
        queueDepth: overworld._script_task_queue.length,
        blockingCount: overworld._blocking_task_count ?? 0,
      });
    }
    overworld._try_start_next_script_task();
  }

  _try_start_next_script_task(): void {
    const overworld = asScriptQueueContext(this);
    const trace = isDebugEnabled("script:tasks") || isDebugEnabled("tasks") || isDebugEnabled("script");
    if (overworld._active_script_task) {
      return;
    }
    while (overworld._script_task_queue?.length) {
      const task = overworld._script_task_queue.shift();
      if (!task) {
        continue;
      }
      overworld._active_script_task = task;
      if (trace) {
        pushDebugLog(`[task] start ${task.constructor?.name ?? "ScriptTask"}`, {
          blocking: task.blocking,
          queueDepth: overworld._script_task_queue.length,
        });
      }
      task.start(overworld);
      if (task.completed) {
        task.finish(overworld);
        const callback = task.onComplete;
        if (trace) {
          pushDebugLog(`[task] finish ${task.constructor?.name ?? "ScriptTask"} (immediate)`, {
            blocking: task.blocking,
            hadCallback: Boolean(callback),
          });
        }
        overworld._finalise_script_task(task);
        if (callback) {
          callback();
        }
        overworld._active_script_task = null;
        continue;
      }
      break;
    }
    if (!overworld._script_task_queue?.length && !overworld._active_script_task) {
      overworld._active_script_task = null;
    }
  }

  _process_script_tasks(): void {
    const overworld = asScriptQueueContext(this);
    const trace = isDebugEnabled("script:tasks") || isDebugEnabled("tasks") || isDebugEnabled("script");
    if (!overworld._active_script_task) {
      overworld._try_start_next_script_task();
    }
    const task = overworld._active_script_task;
    if (!task) {
      overworld._clear_stale_blocking_tasks();
      overworld._maybe_process_idle_phone_calls();
      return;
    }
    task.update(overworld);
    if (task.completed) {
      task.finish(overworld);
      overworld._active_script_task = null;
      const callback = task.onComplete;
      overworld._finalise_script_task(task);
      if (trace) {
        pushDebugLog(`[task] finish ${task.constructor?.name ?? "ScriptTask"}`, {
          blocking: task.blocking,
          hadCallback: Boolean(callback),
        });
      }
      if (callback) {
        callback();
      }
    }
    overworld._try_start_next_script_task();
    overworld._clear_stale_blocking_tasks();
    overworld._maybe_process_idle_phone_calls();
  }

  _finalise_script_task(task: ScriptTask): void {
    const overworld = asScriptQueueContext(this);
    const trace = isDebugEnabled("script:tasks") || isDebugEnabled("tasks") || isDebugEnabled("script");
    if (task.blocking && (overworld._blocking_task_count ?? 0) > 0) {
      const nextCount = Math.max(0, (overworld._blocking_task_count ?? 0) - 1);
      overworld._blocking_task_count = nextCount;
      if (nextCount === 0 && overworld._blocking_movement_lock_active) {
        overworld.unlock_player_movement?.();
        overworld._blocking_movement_lock_active = false;
      }
    }
    if (trace) {
      pushDebugLog(`[task] finalise ${task.constructor?.name ?? "ScriptTask"}`, {
        blocking: task.blocking,
        blockingCount: overworld._blocking_task_count ?? 0,
      });
    }
  }

  _process_pending_event_flag_updates(): void {
    const overworld = asScriptQueueContext(this);
    if (!overworld._pending_event_flag_updates?.length) {
      return;
    }
    const runner = overworld.script_runner;
    if (overworld.scriptTasksActive() || (runner && runner._script_stack?.length)) {
      return;
    }
    const pending: PendingEventFlagUpdate[] = overworld._pending_event_flag_updates;
    overworld._pending_event_flag_updates = [];
    for (const update of pending) {
      const [eventName, value, allowRefresh] = normalizePendingEventFlagUpdate(update);
      if (!allowRefresh) {
        continue;
      }
      overworld._apply_event_flag_update?.(eventName, value);
    }
  }

  _clear_stale_blocking_tasks(): void {
    const overworld = asScriptQueueContext(this);
    if (overworld.scriptTasksActive()) {
      return;
    }
    if ((overworld._blocking_task_count ?? 0) === 0) {
      return;
    }
    if (isDebugEnabled("script:tasks") || isDebugEnabled("tasks") || isDebugEnabled("script")) {
      pushDebugLog("[task] clearing stale blocking tasks", {
        blockingCount: overworld._blocking_task_count ?? 0,
      });
    }
    if (isWarnEnabled()) {
      overworld._logger?.warn?.(
        `Clearing stale blocking tasks without matches (count=${overworld._blocking_task_count})`,
      );
    }
    overworld._blocking_task_count = 0;
    if (overworld._blocking_movement_lock_active) {
      overworld.unlock_player_movement?.();
      overworld._blocking_movement_lock_active = false;
    }
  }

  _maybe_process_idle_phone_calls(): void {
    const overworld = asScriptQueueContext(this);
    const processCall = overworld._process_special_phone_call;
    if (typeof processCall !== "function") {
      return;
    }
    const runner = overworld.script_runner;
    if (runner) {
      if (runner._script_stack?.length) {
        if (isInfoEnabled()) {
          console.info(
            `Skipping phone processing: runner stack active (depth=${runner._script_stack.length})`,
          );
        }
        return;
      }
      if (runner._awaiting_resume) {
        if (isInfoEnabled()) {
          console.info(
            `Skipping phone processing: runner awaiting resume (${runner._awaiting_resume})`,
          );
        }
        return;
      }
    }
    const queue = overworld.game_state?.wram?.scheduled_phone_calls ?? [];
    if (!queue.length) {
      return;
    }
    if (isInfoEnabled()) {
      console.info(
        "Processing pending phone call queue=%s runner_state=%s is_moving=%s",
        [...queue],
        runner ? runner.state ?? ScriptRunnerState.IDLE : "none",
        overworld.is_moving ?? false,
      );
    }
    processCall.call(overworld);
  }
}

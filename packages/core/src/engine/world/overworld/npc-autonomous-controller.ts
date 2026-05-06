// ASM mapping: pokecrystal_disassembly/data/sprites/sprite_movement.asm (SpriteMovementData behaviors).
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { _DIRECTION_VECTORS } from "./constants";
import { spinCycleForMovement, stepPatternForObject } from "./npc-movement";
import { OverworldObject } from "./overworld-object";
import { MovementTask, type MovementOverworldContext } from "./script-tasks/movement-task";
import type { GameState } from "@pokecrystal/core/core/state";
import type { OverworldEngine } from "./overworld";
import type { ScriptTask } from "./script-tasks/script-task";

const BEHAVIOUR_MAP: Record<string, string> = {
  SPRITEMOVEDATA_WALK_UP_DOWN: "vertical_walk",
  SPRITEMOVEDATA_WALK_LEFT_RIGHT: "horizontal_walk",
  SPRITEMOVEDATA_WANDER: "wander",
  SPRITEMOVEDATA_SWIM_WANDER: "wander",
  SPRITEMOVEDATA_SPINRANDOM_SLOW: "spin_slow",
  SPRITEMOVEDATA_SPINRANDOM_FAST: "spin_fast",
  SPRITEMOVEDATA_SPINCLOCKWISE: "spin_clockwise",
  SPRITEMOVEDATA_SPINCOUNTERCLOCKWISE: "spin_counterclockwise",
};

const SPIN_DIRECTIONS = ["down", "up", "left", "right"] as const;
const ASM_FIXED_SPIN_STEP_DURATION = 0x10;
const STEP_COMMANDS: Record<string, string[]> = {
  up: ["step UP", "step_end"],
  down: ["step DOWN", "step_end"],
  left: ["step LEFT", "step_end"],
  right: ["step RIGHT", "step_end"],
};

type ScriptRunnerSnapshot = {
  _script_stack?: unknown[];
  _awaiting_resume?: number;
};

export type NPCStepBlocker = (
  npc: OverworldObject,
  direction: string,
  target_tile_x: number,
  target_tile_y: number,
  options?: { is_player_target?: boolean; player_only?: boolean; suppress_blocked_log?: boolean }
) => boolean;

type OverworldWithRuntime = OverworldEngine & {
  game_state: GameState | null;
  npcs?: OverworldObject[];
  _npc_step_blocked?: NPCStepBlocker;
  _active_script_task?: ScriptTask | null;
  _script_task_queue?: ScriptTask[] | null;
  _follower_movement_task?: ScriptTask | null;
  script_runner?: ScriptRunnerSnapshot | null;
};

type SpinDirection = (typeof SPIN_DIRECTIONS)[number];

class NpcMovementState {
  public npc: OverworldObject;
  public behaviour: string;
  public bounds: [number, number, number, number] | null;
  public cooldown: number;
  public active_task: MovementTask | null;
  public pattern: string[] | null;
  public pattern_index: number;
  public spin_cycle: string[] | null;
  public paused_for_script: boolean;

  constructor(
    npc: OverworldObject,
    behaviour: string,
    bounds: [number, number, number, number] | null,
    cooldown: number,
    pattern: string[] | null,
    spin_cycle: string[] | null
  ) {
    this.npc = npc;
    this.behaviour = behaviour;
    this.bounds = bounds;
    this.cooldown = cooldown;
    this.active_task = null;
    this.pattern = pattern;
    this.pattern_index = 0;
    this.spin_cycle = spin_cycle;
    this.paused_for_script = false;
  }
}

class ScriptControlSnapshot {
  public targets: Set<OverworldObject>;
  public last_talked: number;
  public script_active: boolean;

  constructor(targets: Set<OverworldObject>, last_talked: number, script_active: boolean) {
    this.targets = targets;
    this.last_talked = last_talked;
    this.script_active = script_active;
  }
}

export class NpcAutonomousController {
  private overworld: OverworldWithRuntime;
  private rng_factory: (state: GameState) => HardwareRNG;
  private rng: HardwareRNG;
  private states: Map<OverworldObject, NpcMovementState> = new Map();
  private logger: Console = console;

  constructor(
    overworld: OverworldWithRuntime,
    { rng_factory = null }: { rng_factory?: ((state: GameState) => HardwareRNG) | null } = {}
  ) {
    this.overworld = overworld;
    this.rng_factory = rng_factory ?? ((state: GameState) => new HardwareRNG(state));
    if (!overworld.game_state) {
      throw new Error("NpcAutonomousController requires an active game state.");
    }
    this.rng = this.rng_factory(overworld.game_state);
  }

  public rebuild(npcs: Iterable<OverworldObject>): void {
    for (const state of this.states.values()) {
      this.finalise_task(state);
    }
    this.states.clear();
    for (const npc of npcs) {
      this.add_npc(npc);
    }
  }

  public add_npc(npc: OverworldObject): void {
    const movement_label = npc.event.spritemovedata ?? "";
    const behaviour = BEHAVIOUR_MAP[movement_label.toUpperCase()] ?? null;
    const pattern = stepPatternForObject(movement_label, {
      moveRangeX: npc.event.move_range_x ?? 0,
      moveRangeY: npc.event.move_range_y ?? 0,
    });
    let resolved_behaviour = behaviour;
    if (pattern) {
      resolved_behaviour = resolved_behaviour ?? "pattern_walk";
    }
    const spin_cycle = spinCycleForMovement(movement_label);
    if (!resolved_behaviour && spin_cycle) {
      resolved_behaviour = "pattern_spin";
    }
    if (!resolved_behaviour) {
      return;
    }
    const bounds = this.compute_bounds(npc, resolved_behaviour);
    // ASM: map_objects.asm::_MovementSpinRepeat sleeps for a fixed $10 frames
    // before each SPINCLOCKWISE/SPINCOUNTERCLOCKWISE facing update.
    const initial_cooldown = spin_cycle
      ? ASM_FIXED_SPIN_STEP_DURATION
      : pattern
        ? 0
        : this.roll_idle_frames(resolved_behaviour);
    const state = new NpcMovementState(npc, resolved_behaviour, bounds, initial_cooldown, pattern, spin_cycle);
    this.states.set(npc, state);
  }

  public remove_npc(npc: OverworldObject): void {
    const state = this.states.get(npc);
    if (state) {
      this.finalise_task(state);
      this.states.delete(npc);
    }
  }

  public update(): void {
    if (!this.states.size) {
      return;
    }
    const live_npcs = new Set<OverworldObject>(this.overworld.npcs ?? []);
    const script_control = this.build_script_control_snapshot();
    for (const [npc_key, state] of Array.from(this.states.entries())) {
      if (!state) {
        continue;
      }
      const npc = state.npc;
      if (!live_npcs.has(npc_key)) {
        this.finalise_task(state);
        this.states.delete(npc_key);
        continue;
      }
      if (this.npc_under_script_control(npc, script_control)) {
        state.paused_for_script = true;
        this.finalise_task(state, { reset_cooldown: false });
        continue;
      }
      if (state.paused_for_script) {
        state.paused_for_script = false;
        this.finalise_task(state, { reset_cooldown: true });
        state.cooldown = 0;
        const direction = this.choose_direction(state);
        if (direction) {
          const [dx, dy] = _DIRECTION_VECTORS[direction] ?? [0, 0];
          const stride = Math.max(1, this.overworld.TILES_PER_COLLISION ?? 2);
          const target_x = state.npc.x + dx * stride;
          const target_y = state.npc.y + dy * stride;
          if (
            this.target_within_bounds(state, target_x, target_y) &&
            !this.overworld._npc_step_blocked?.(state.npc, direction, target_x, target_y)
          ) {
            state.npc.x = target_x;
            state.npc.y = target_y;
            state.cooldown = this.roll_idle_frames(state.behaviour);
            continue;
          }
        }
      }
      if (state.behaviour.startsWith("spin") || state.spin_cycle) {
        this.update_spin(state);
      } else {
        this.update_walk(state);
      }
    }
  }

  private compute_bounds(npc: OverworldObject, behaviour: string): [number, number, number, number] | null {
    const stride = Math.max(1, this.overworld.TILES_PER_COLLISION ?? 2);
    const origin_x = npc.initialSubtileX ?? npc.x;
    const origin_y = npc.initialSubtileY ?? npc.y;
    const range_x = Math.max(0, npc.event.move_range_x ?? 0) * stride;
    const range_y = Math.max(0, npc.event.move_range_y ?? 0) * stride;
    return [origin_x - range_x, origin_x + range_x, origin_y - range_y, origin_y + range_y];
  }

  private target_within_bounds(state: NpcMovementState, target_x: number, target_y: number): boolean {
    const bounds = state.bounds;
    if (!bounds) {
      return true;
    }
    const [min_x, max_x, min_y, max_y] = bounds;
    return min_x <= target_x && target_x <= max_x && min_y <= target_y && target_y <= max_y;
  }

  private npc_under_script_control(npc: OverworldObject, snapshot: ScriptControlSnapshot): boolean {
    const npc_index = npc.objectIndex ?? 0;
    if (snapshot.targets.has(npc)) {
      return true;
    }
    if (snapshot.last_talked > 0 && npc_index === snapshot.last_talked) {
      return true;
    }
    if (snapshot.script_active) {
      return true;
    }
    return false;
  }

  private build_script_control_snapshot(): ScriptControlSnapshot {
    const targets = new Set<OverworldObject>();
    const active = this.overworld._active_script_task ?? null;
    this.record_task_target(active, targets);
    const queue = this.overworld._script_task_queue ?? [];
    for (const task of queue) {
      this.record_task_target(task, targets);
    }
    const follower_task = this.overworld._follower_movement_task ?? null;
    this.record_task_target(follower_task, targets);

    const script_runner = this.overworld.script_runner ?? null;
    const script_stack = script_runner?._script_stack ?? null;
    const awaiting_resume = script_runner?._awaiting_resume ?? 0;
    const script_active = Boolean(script_stack && script_stack.length) || awaiting_resume > 0;
    const last_talked = Number(this.overworld.game_state?.wram.last_talked ?? 0);

    return new ScriptControlSnapshot(targets, last_talked, script_active);
  }

  private record_task_target(task: ScriptTask | null, targets: Set<OverworldObject>): void {
    if (task instanceof MovementTask) {
      const target = task.getTarget ? task.getTarget() : null;
      if (target) {
        targets.add(target as unknown as OverworldObject);
      }
    }
  }

  private roll_idle_frames(behaviour: string): number {
    const mask = behaviour === "spin_fast" ? 0x1f : 0x7f;
    this.rng.nextByte();
    const add_value = this.rng.peekHRandomAdd();
    return add_value & mask;
  }

  private update_spin(state: NpcMovementState): void {
    if (state.spin_cycle) {
      this.update_spin_pattern(state);
      return;
    }
    if (state.active_task) {
      this.finalise_task(state);
    }
    if (state.cooldown > 0) {
      state.cooldown -= 1;
      return;
    }
    let direction = this.random_spin_direction();
    const current = state.npc.direction ?? null;
    if (direction === current) {
      const idx = SPIN_DIRECTIONS.indexOf(direction);
      const next = idx < 0 ? 0 : (idx + 1) % SPIN_DIRECTIONS.length;
      direction = SPIN_DIRECTIONS[next];
    }
    state.npc.direction = direction;
    state.cooldown = this.roll_idle_frames(state.behaviour);
  }

  private update_walk(state: NpcMovementState): void {
    const overworld = this.overworld;
    const movementContext = overworld as unknown as MovementOverworldContext;
    if (state.active_task) {
      state.active_task.update(movementContext);
      if (state.active_task.completed) {
        state.active_task.finish(movementContext);
        state.active_task = null;
        state.cooldown = this.roll_idle_frames(state.behaviour);
      }
      return;
    }
    if (state.cooldown > 0) {
      state.cooldown -= 1;
      return;
    }
    let direction: string | null = null;
    if (state.pattern) {
      direction = this.next_pattern_direction(state);
    } else {
      direction = this.choose_direction(state);
    }
    if (!direction) {
      state.cooldown = this.roll_idle_frames(state.behaviour);
      return;
    }
    const commands = STEP_COMMANDS[direction] ?? [`step ${direction.toUpperCase()}`, "step_end"];
    STEP_COMMANDS[direction] = commands;
    const task = new MovementTask(state.npc, commands, {
      blocking: false,
      onComplete: null,
      respectCollision: true,
    });
    task.start(movementContext);
    if (task.completed) {
      task.finish(movementContext);
      state.cooldown = this.roll_idle_frames(state.behaviour);
    } else {
      state.active_task = task;
    }
  }

  private choose_direction(state: NpcMovementState): string | null {
    const stride = Math.max(1, this.overworld.TILES_PER_COLLISION ?? 2);
    const npc = state.npc;
    const step_blocked = this.overworld._npc_step_blocked?.bind(this.overworld);
    const candidates: string[] = [];

    const direction_allowed = (direction: string): boolean => {
      const [dx, dy] = _DIRECTION_VECTORS[direction] ?? [0, 0];
      if (dx === 0 && dy === 0) {
        return false;
      }
      const target_x = npc.x + dx * stride;
      const target_y = npc.y + dy * stride;
      if (!this.target_within_bounds(state, target_x, target_y)) {
        return false;
      }
      if (step_blocked?.(npc, direction, target_x, target_y)) {
        return false;
      }
      return true;
    };

    if (state.behaviour === "vertical_walk") {
      ["up", "down"].forEach((option) => {
        if (direction_allowed(option)) {
          candidates.push(option);
        }
      });
    } else if (state.behaviour === "horizontal_walk") {
      ["left", "right"].forEach((option) => {
        if (direction_allowed(option)) {
          candidates.push(option);
        }
      });
    } else if (state.behaviour === "wander") {
      ["up", "down", "left", "right"].forEach((option) => {
        if (direction_allowed(option)) {
          candidates.push(option);
        }
      });
    }

    if (!candidates.length) {
      return null;
    }
    return candidates[this.rng.randrange(candidates.length)];
  }

  private next_pattern_direction(state: NpcMovementState): string | null {
    const pattern = state.pattern ?? [];
    if (!pattern.length) {
      return null;
    }
    const stride = Math.max(1, this.overworld.TILES_PER_COLLISION ?? 2);
    const npc = state.npc;
    const step_blocked = this.overworld._npc_step_blocked?.bind(this.overworld);
    for (let attempts = 0; attempts < pattern.length; attempts += 1) {
      const direction = pattern[state.pattern_index % pattern.length];
      state.pattern_index = (state.pattern_index + 1) % pattern.length;
      const [dx, dy] = _DIRECTION_VECTORS[direction] ?? [0, 0];
      if (dx === 0 && dy === 0) {
        continue;
      }
      const target_x = npc.x + dx * stride;
      const target_y = npc.y + dy * stride;
      if (!this.target_within_bounds(state, target_x, target_y)) {
        continue;
      }
      if (step_blocked?.(npc, direction, target_x, target_y)) {
        continue;
      }
      return direction;
    }
    return null;
  }

  private random_spin_direction(): SpinDirection {
    const index = this.rng.randrange(SPIN_DIRECTIONS.length);
    return SPIN_DIRECTIONS[index];
  }

  private update_spin_pattern(state: NpcMovementState): void {
    if (state.active_task) {
      this.finalise_task(state);
    }
    if (state.cooldown > 0) {
      state.cooldown -= 1;
      if (state.cooldown > 0) {
        return;
      }
    }
    const cycle = state.spin_cycle ?? [];
    const npc_direction = state.npc.direction ?? "down";
    if (!cycle.length) {
      return;
    }
    const idx = cycle.indexOf(npc_direction);
    const next_direction = cycle[(idx + 1 + cycle.length) % cycle.length];
    state.npc.direction = next_direction;
    state.cooldown = ASM_FIXED_SPIN_STEP_DURATION;
  }

  private finalise_task(state: NpcMovementState, { reset_cooldown = false }: { reset_cooldown?: boolean } = {}): void {
    if (state.active_task) {
      state.active_task.finish(this.overworld as unknown as MovementOverworldContext);
      state.active_task = null;
    }
    if (reset_cooldown) {
      state.cooldown = this.roll_idle_frames(state.behaviour);
    }
  }
}

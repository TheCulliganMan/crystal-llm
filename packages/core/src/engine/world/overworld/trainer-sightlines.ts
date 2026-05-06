// ASM mapping: pokecrystal_disassembly/home/trainers.asm (CheckTrainerBattle),
// pokecrystal_disassembly/engine/events/trainer_scripts.asm (SeenByTrainerScript),
// pokecrystal_disassembly/home/movement.asm (ComputePathToWalkToPlayer).
import logger from "@pokecrystal/core/core/logger";
import { GameState } from "@pokecrystal/core/core/state";
import { OverworldObject } from "./overworld-object";

export type TrainerSighting = {
  npc: OverworldObject;
  direction: string;
  distanceTiles: number;
  deltaX: number;
  deltaY: number;
};

type ScriptRunnerLike = {
  run: (
    scriptName: string,
    options?: { allow_fallthrough?: boolean; allowFallthrough?: boolean }
  ) => void;
  _script_stack?: unknown[];
  last_interaction_object_index?: number | null;
};

type DialogueLike = {
  active?: boolean;
  waiting_for_input?: boolean;
};

type LegacyFacingNPC = OverworldObject & {
  face_player?: (playerX: number, playerY: number) => void;
};

type TrainerSightlineContext = {
  npcs?: OverworldObject[];
  script_runner?: ScriptRunnerLike | null;
  game_state: GameState;
  dialogue?: DialogueLike | null;
  player_x: number;
  player_y: number;
  player_direction: string;
  is_moving: boolean;
  TILES_PER_COLLISION: number;
  _trainer_cutscene_active?: boolean;
  _active_trainer_sighting?: TrainerSighting | null;
  data_loader?: {
    trainer_event_flags?: Record<string, string>;
    get_script?: (scriptName: string) => unknown[] | null;
    getScript?: (scriptName: string) => unknown[] | null;
  } | null;
  queue_delay?: (frames: number, options: { on_complete: () => void; blocking?: boolean }) => boolean;
  queueDelay?: (frames: number, options: { onComplete: () => void; blocking?: boolean }) => boolean;
  queue_movement_task?: (
    obj: OverworldObject,
    movement_commands: string[],
    options?: { on_complete?: () => void }
  ) => void;
  queueMovementTask?: (
    obj: OverworldObject,
    movement_commands: string[],
    options?: { onComplete?: () => void }
  ) => void;
  script_tasks_active?: () => boolean;
  scriptTasksActive?: () => boolean;
  player_movement_locked?: () => boolean;
  playerMovementLocked?: () => boolean;
  show_emote?: (emote_id: string, obj: OverworldObject, duration: number) => void;
};

type ScriptEntryLike = { command?: unknown; args?: unknown[] };

export class TrainerSightlineMixin {
  public static readonly TRAINER_EMOTE_DURATION = 30;

  check_for_trainer_sightlines(): boolean {
    const overworld = this as unknown as TrainerSightlineContext;
    const npcs = overworld.npcs ?? [];
    const script_runner = overworld.script_runner ?? null;
    const dialogue = overworld.dialogue ?? null;
    const is_moving = Boolean(overworld.is_moving);
    const player_movement_locked = overworld.player_movement_locked ?? overworld.playerMovementLocked;
    const script_tasks_active = overworld.script_tasks_active ?? overworld.scriptTasksActive;

    if (!npcs || npcs.length === 0) {
      return false;
    }
    if (overworld._trainer_cutscene_active) {
      return false;
    }
    if (is_moving || (player_movement_locked ? player_movement_locked.call(overworld) : false)) {
      return false;
    }
    if (script_tasks_active ? script_tasks_active.call(overworld) : false) {
      return false;
    }
    if (script_runner && Array.isArray(script_runner._script_stack)) {
      if (script_runner._script_stack.length > 0) {
        return false;
      }
    }
    if (dialogue && (dialogue.active || dialogue.waiting_for_input)) {
      return false;
    }

    for (const npc of [...npcs]) {
      if (!TrainerSightlineMixin.prototype._npc_is_trainer.call(this, npc)) {
        continue;
      }
      if (TrainerSightlineMixin.prototype._trainer_event_flag_is_set.call(this, npc)) {
        continue;
      }
      const result = TrainerSightlineMixin.prototype._trainer_distance_and_direction.call(
        this,
        npc
      );
      if (!result) {
        continue;
      }
      const [distanceTiles, direction, deltaX, deltaY] = result;
      const sightRange = Math.max(npc.event.radius ?? 0, 0);
      if (sightRange === 0 || distanceTiles > sightRange) {
        continue;
      }
      TrainerSightlineMixin.prototype._engage_trainer_via_sightline.call(
        this,
        npc,
        distanceTiles,
        direction,
        deltaX,
        deltaY
      );
      return true;
    }
    return false;
  }

  private _npc_is_trainer(npc: OverworldObject): boolean {
    const objectType = String(npc.event.object_type ?? "").toUpperCase();
    if (objectType !== "OBJECTTYPE_TRAINER") {
      return false;
    }
    const script = String(npc.event.script ?? "").trim();
    if (!script) {
      return false;
    }
    if (script.toUpperCase() === "OBJECTEVENT") {
      return false;
    }
    if (npc.walking || npc.jumping) {
      return false;
    }
    return true;
  }

  private _trainer_event_flag_is_set(npc: OverworldObject): boolean {
    const overworld = this as unknown as TrainerSightlineContext;
    const flag = TrainerSightlineMixin.prototype._lookup_trainer_event_flag.call(this, npc);
    if (!flag) {
      return false;
    }
    return Boolean(overworld.game_state.wram.event_flags?.[flag]);
  }

  private _lookup_trainer_event_flag(npc: OverworldObject): string | null {
    const overworld = this as unknown as TrainerSightlineContext;
    const scriptName = String(npc.event.script ?? "").trim();
    if (!scriptName) {
      return null;
    }
    const data_loader = overworld.data_loader ?? null;
    const trainerFlags = data_loader?.trainer_event_flags;
    if (!trainerFlags || typeof trainerFlags !== "object") {
      return TrainerSightlineMixin.prototype._lookup_trainer_event_flag_from_script.call(
        this,
        scriptName,
        data_loader
      );
    }
    return (
      trainerFlags[scriptName] ??
      TrainerSightlineMixin.prototype._lookup_trainer_event_flag_from_script.call(
        this,
        scriptName,
        data_loader
      )
    );
  }

  private _lookup_trainer_event_flag_from_script(
    scriptName: string,
    data_loader: TrainerSightlineContext["data_loader"]
  ): string | null {
    if (!data_loader) {
      return null;
    }
    const get_script = data_loader.get_script ?? data_loader.getScript;
    if (typeof get_script !== "function") {
      return null;
    }
    const scriptData = get_script.call(data_loader, scriptName);
    if (!scriptData) {
      throw new Error(`Trainer sightline script '${scriptName}' is missing from story data.`);
    }
    // ASM: _CheckTrainerBattle reads the first word in the trainer script (dw EVENT_BEAT_*).
    for (const entry of scriptData) {
      if (!entry || typeof entry !== "object") {
        continue;
      }
      const command = String((entry as ScriptEntryLike).command ?? "").trim().toLowerCase();
      if (command !== "trainer") {
        continue;
      }
      if (!Array.isArray((entry as ScriptEntryLike).args)) {
        throw new Error(
          `Trainer sightline script '${scriptName}' has malformed trainer arguments.`
        );
      }
      const args = (entry as ScriptEntryLike).args as unknown[];
      const flag = String(args[2] ?? "").trim();
      if (!flag || flag === "0" || flag === "-1") {
        return null;
      }
      if (data_loader.trainer_event_flags) {
        data_loader.trainer_event_flags[scriptName] = flag;
      }
      return flag;
    }
    throw new Error(`Trainer sightline script '${scriptName}' missing trainer data.`);
  }

  private _trainer_distance_and_direction(
    npc: OverworldObject
  ): [number, string, number, number] | null {
    const overworld = this as unknown as TrainerSightlineContext;
    const stride = Math.max(1, overworld.TILES_PER_COLLISION ?? 2);
    const npcTileX = Math.floor((npc.x ?? 0) / stride);
    const npcTileY = Math.floor((npc.y ?? 0) / stride);
    const playerTileX = Math.floor(overworld.player_x / stride);
    const playerTileY = Math.floor(overworld.player_y / stride);
    const dx = playerTileX - npcTileX;
    const dy = playerTileY - npcTileY;
    const deltaX = dx;
    const deltaY = dy;

    let direction: string | null = null;
    let distanceTiles = 0;
    if (deltaX === 0 && deltaY !== 0) {
      direction = deltaY > 0 ? "down" : "up";
      distanceTiles = Math.abs(deltaY);
    } else if (deltaY === 0 && deltaX !== 0) {
      direction = deltaX > 0 ? "right" : "left";
      distanceTiles = Math.abs(deltaX);
    } else {
      return null;
    }

    if (distanceTiles <= 0 || !direction) {
      return null;
    }

    const currentDirection = TrainerSightlineMixin.prototype._trainer_facing_direction.call(
      this,
      npc
    );
    if (currentDirection !== direction) {
      return null;
    }

    return [distanceTiles, direction, deltaX, deltaY];
  }

  private _engage_trainer_via_sightline(
    npc: OverworldObject,
    distanceTiles: number,
    direction: string,
    deltaX: number,
    deltaY: number
  ): void {
    const overworld = this as unknown as TrainerSightlineContext;
    const script = String(npc.event.script ?? "").trim();
    if (!script) {
      throw new Error(
        `Trainer '${npc.objectId ?? npc.objectIndex}' is missing a script.`
      );
    }
    const { game_state } = overworld;
    game_state.wram.last_talked = npc.objectIndex ?? 0;
    game_state.wram.seen_trainer_distance = distanceTiles;
    game_state.wram.seen_trainer_direction = direction;
    if (overworld.script_runner) {
      overworld.script_runner.last_interaction_object_index = npc.objectIndex ?? null;
    }
    const sighting: TrainerSighting = {
      npc,
      direction,
      distanceTiles,
      deltaX,
      deltaY,
    };
    overworld._active_trainer_sighting = sighting;
    overworld._trainer_cutscene_active = true;

    const emoteDuration = TrainerSightlineMixin.TRAINER_EMOTE_DURATION;
    if (overworld.show_emote) {
      overworld.show_emote("EMOTE_SHOCK", npc, emoteDuration);
    }

    const afterEmote = () => {
      TrainerSightlineMixin.prototype._queue_trainer_walk.call(this, sighting);
    };

    const queue_delay = overworld.queue_delay;
    const queueDelay = overworld.queueDelay;
    const scheduled = queue_delay
      ? queue_delay.call(overworld, emoteDuration, { on_complete: afterEmote, blocking: true })
      : queueDelay
        ? queueDelay.call(overworld, emoteDuration, { onComplete: afterEmote, blocking: true })
        : false;
    if (!scheduled) {
      TrainerSightlineMixin.prototype._queue_trainer_walk.call(this, sighting);
    }
  }

  private _queue_trainer_walk(sighting: TrainerSighting | null): void {
    const overworld = this as unknown as TrainerSightlineContext;
    if (!sighting) {
      overworld._trainer_cutscene_active = false;
      return;
    }

    const movementCommands = TrainerSightlineMixin.prototype._build_trainer_path_commands.call(
      this,
      sighting.deltaX,
      sighting.deltaY
    );
    movementCommands.pop();

    const commands = ["step_sleep 1"];
    commands.push(...movementCommands);
    commands.push("step_end");

    const onComplete = () => {
      TrainerSightlineMixin.prototype._complete_trainer_walk.call(this, sighting);
    };
    const queue_movement_task = overworld.queue_movement_task;
    const queueMovementTask = overworld.queueMovementTask;
    if (queue_movement_task) {
      queue_movement_task.call(overworld, sighting.npc, commands, { on_complete: onComplete });
      return;
    }
    if (queueMovementTask) {
      queueMovementTask.call(overworld, sighting.npc, commands, { onComplete: onComplete });
      return;
    }
    TrainerSightlineMixin.prototype._complete_trainer_walk.call(this, sighting);
  }

  private _build_trainer_path_commands(deltaX: number, deltaY: number): string[] {
    const xDistance = Math.abs(Math.trunc(deltaX));
    const yDistance = Math.abs(Math.trunc(deltaY));

    let firstDirection = deltaX >= 0 ? "right" : "left";
    let secondDirection = deltaY >= 0 ? "down" : "up";
    let firstDistance = xDistance;
    let secondDistance = yDistance;

    // ASM: home/movement.asm::ComputePathToWalkToPlayer swaps axis order when
    // the Y distance is shorter than the X distance, then appends one axis then the other.
    if (yDistance < xDistance) {
      [firstDirection, secondDirection] = [secondDirection, firstDirection];
      [firstDistance, secondDistance] = [secondDistance, firstDistance];
    }

    const commands: string[] = [];
    for (let i = 0; i < firstDistance; i += 1) {
      commands.push(`slow_step ${firstDirection}`);
    }
    for (let i = 0; i < secondDistance; i += 1) {
      commands.push(`slow_step ${secondDirection}`);
    }
    return commands;
  }

  private _complete_trainer_walk(sighting: TrainerSighting | null): void {
    const overworld = this as unknown as TrainerSightlineContext;
    if (!sighting) {
      overworld._trainer_cutscene_active = false;
      return;
    }
    const npc = sighting.npc;
    if (typeof npc.facePlayer === "function") {
      npc.facePlayer(overworld.player_x, overworld.player_y);
    } else {
      const legacyNpc = npc as LegacyFacingNPC;
      if (typeof legacyNpc.face_player === "function") {
        legacyNpc.face_player(overworld.player_x, overworld.player_y);
      }
    }
    TrainerSightlineMixin.prototype._face_player_toward_trainer.call(
      this,
      sighting.direction
    );
    TrainerSightlineMixin.prototype._start_trainer_script.call(this, npc);
  }

  private _face_player_toward_trainer(trainerDirection: string): void {
    const opposite: Record<string, string> = {
      up: "down",
      down: "up",
      left: "right",
      right: "left",
    };
    const newDirection = opposite[trainerDirection.toLowerCase()];
    if (newDirection) {
      (this as unknown as TrainerSightlineContext).player_direction = newDirection;
    }
  }

  private _start_trainer_script(npc: OverworldObject): void {
    const overworld = this as unknown as TrainerSightlineContext;
    const script = String(npc.event.script ?? "").trim();
    if (!script) {
      throw new Error(
        `Trainer '${npc.objectId ?? npc.objectIndex}' script missing during sightline.`
      );
    }
    try {
      const runner = overworld.script_runner;
      if (!runner) {
        throw new Error("Trainer sightline handling requires a ScriptRunner.");
      }
      runner.run(script, { allow_fallthrough: false });
    } catch (error) {
      logger.error("Trainer sightline script failed.", error);
      throw error;
    } finally {
      overworld._active_trainer_sighting = null;
      overworld._trainer_cutscene_active = false;
    }
  }

  private _trainer_facing_direction(npc: OverworldObject): string {
    const overrideRaw = npc.event.sightline_direction_override;
    if (typeof overrideRaw === "string") {
      const override = overrideRaw.trim().toLowerCase();
      if (override) {
        return override;
      }
    }
    return String(npc.direction ?? "").toLowerCase();
  }
}

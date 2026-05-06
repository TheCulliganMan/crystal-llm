import { TrainerSightlineMixin } from "@pokecrystal/core/engine/world/overworld/trainer-sightlines";
import type { GameState } from "@pokecrystal/core/core/state";
import type { OverworldObject } from "@pokecrystal/core/engine/world/overworld/overworld-object";

describe("TrainerSightlineMixin", () => {
  it("returns false without crashing when NPC list is missing", () => {
    const game_state = {
      wram: { event_flags: {} },
    } as GameState;

    const context = {
      npcs: undefined,
      script_runner: null,
      game_state,
      dialogue: null,
      player_x: 0,
      player_y: 0,
      player_direction: "down",
      is_moving: false,
      TILES_PER_COLLISION: 2,
      playerMovementLocked: () => false,
      scriptTasksActive: () => false,
      queueDelay: () => false,
      queueMovementTask: () => undefined,
    };

    const result = TrainerSightlineMixin.prototype.check_for_trainer_sightlines.call(context);

    expect(result).toBe(false);
  });

  it("uses camelCase queue helpers to trigger trainer scripts", () => {
    const runner = { run: jest.fn(), _script_stack: [] as unknown[] };
    const game_state = {
      wram: { event_flags: {}, last_talked: 0, seen_trainer_distance: 0, seen_trainer_direction: "" },
    } as GameState;

    const npc = {
      event: {
        object_type: "OBJECTTYPE_TRAINER",
        script: "TestTrainerScript",
        radius: 3,
      },
      x: 0,
      y: 0,
      direction: "down",
      walking: false,
      jumping: false,
      objectIndex: 7,
    } as OverworldObject;

    const queueMovementTask = jest.fn((_, __, options: { onComplete?: () => void }) => {
      options.onComplete?.();
    });

    const context = {
      npcs: [npc],
      script_runner: runner,
      game_state,
      dialogue: null,
      player_x: 0,
      player_y: 4,
      player_direction: "up",
      is_moving: false,
      TILES_PER_COLLISION: 2,
      playerMovementLocked: () => false,
      scriptTasksActive: () => false,
      queueDelay: () => false,
      queueMovementTask,
    };

    const result = TrainerSightlineMixin.prototype.check_for_trainer_sightlines.call(context);

    expect(result).toBe(true);
    expect(queueMovementTask).toHaveBeenCalled();
    expect(runner.run).toHaveBeenCalledWith("TestTrainerScript", { allow_fallthrough: false });
    expect(game_state.wram.last_talked).toBe(7);
  });
});

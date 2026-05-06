import { OverworldEngine } from "./overworld";
import { TrainerSightlineMixin } from "./trainer-sightlines";

type DummyOverworldObject = {
  event: { object_type: string; script: string; radius?: number };
  objectIndex?: number;
  walking?: boolean;
  jumping?: boolean;
  facePlayer?: (x: number, y: number) => void;
  x?: number;
  y?: number;
  direction?: string;
};

class TrainerSightlineOverworld extends TrainerSightlineMixin {
  public npcs: DummyOverworldObject[] = [];
  public script_runner = null;
  public dialogue = null;
  public is_moving = false;
  public _trainer_cutscene_active = false;
  public player_x = 0;
  public player_y = 0;
  public player_direction = "down";
  public game_state = {
    wram: {
      last_talked: 0,
      seen_trainer_distance: 0,
      seen_trainer_direction: "",
    },
  };
  public player_movement_locked_context: unknown = null;
  public script_tasks_active_context: unknown = null;

  public player_movement_locked(): boolean {
    this.player_movement_locked_context = this;
    return false;
  }

  public script_tasks_active(): boolean {
    this.script_tasks_active_context = this;
    return true;
  }
}

class TrainerSightlineQueueOverworld extends TrainerSightlineMixin {
  public script_runner = { run: jest.fn(), last_interaction_object_index: null as number | null };
  public game_state = {
    wram: {
      last_talked: 0,
      seen_trainer_distance: 0,
      seen_trainer_direction: "",
    },
  };
  public player_x = 0;
  public player_y = 0;
  public player_direction = "down";
  public queue_delay_context: unknown = null;
  public queue_movement_context: unknown = null;

  public queue_delay(frames: number, options: { on_complete: () => void; blocking?: boolean }): boolean {
    this.queue_delay_context = this;
    void frames;
    void options;
    return true;
  }

  public queue_movement_task(
    _npc: DummyOverworldObject,
    _commands: string[],
    options: { on_complete?: () => void }
  ): void {
    this.queue_movement_context = this;
    options.on_complete?.();
  }
}

class TrainerSightlineFlagOverworld extends TrainerSightlineMixin {
  public npcs: DummyOverworldObject[] = [];
  public script_runner = null;
  public dialogue = null;
  public is_moving = false;
  public _trainer_cutscene_active = false;
  public player_x = 0;
  public player_y = 1;
  public player_direction = "down";
  public TILES_PER_COLLISION = 1;
  public data_loader = {
    get_script(scriptName: string) {
      if (scriptName !== "TrainerTest") {
        return null;
      }
      return [
        {
          command: "trainer",
          args: [
            "BUG_CATCHER",
            "BUG_CATCHER_TEST",
            "EVENT_BEAT_TRAINER_TEST",
            "SeenText",
            "WinText",
            "0",
            "AfterScript",
          ],
        },
      ];
    },
  };
  public game_state = {
    wram: {
      last_talked: 0,
      seen_trainer_distance: 0,
      seen_trainer_direction: "",
      event_flags: {
        EVENT_BEAT_TRAINER_TEST: true,
      },
    },
  };

  public player_movement_locked(): boolean {
    return false;
  }

  public script_tasks_active(): boolean {
    return false;
  }
}

class TrainerSightlineMalformedScriptOverworld extends TrainerSightlineMixin {
  public npcs: DummyOverworldObject[] = [];
  public script_runner = null;
  public dialogue = null;
  public is_moving = false;
  public _trainer_cutscene_active = false;
  public player_x = 0;
  public player_y = 1;
  public player_direction = "down";
  public TILES_PER_COLLISION = 1;
  public data_loader = {
    get_script(scriptName: string) {
      if (scriptName !== "TrainerBadArgs") {
        return null;
      }
      return [
        {
          command: "trainer",
          args: "EVENT_BEAT_TRAINER_BAD_ARGS",
        },
      ];
    },
  };
  public game_state = {
    wram: {
      last_talked: 0,
      seen_trainer_distance: 0,
      seen_trainer_direction: "",
      event_flags: {},
    },
  };

  public player_movement_locked(): boolean {
    return false;
  }

  public script_tasks_active(): boolean {
    return false;
  }
}

describe("TrainerSightlineMixin", () => {
  it("falls back to the trainer sightline mixin from player events when the instance method is missing", () => {
    const discardPendingWildEncounter = jest.fn();
    const context = {
      _map_has_trainer_sightlines: true,
      check_for_trainer_sightlines: undefined,
      npcs: [],
      script_runner: null,
      dialogue: null,
      is_moving: false,
      _trainer_cutscene_active: false,
      player_x: 0,
      player_y: 0,
      player_direction: "down",
      TILES_PER_COLLISION: 1,
      game_state: { wram: { wEnabledPlayerEvents: 0 } },
      player_movement_locked: () => false,
      script_tasks_active: () => false,
      _player_events_blocked: () => false,
      _skip_wild_encounter_for_step: false,
      _discard_pending_wild_encounter: discardPendingWildEncounter,
      _player_events_enabled: () => false,
      check_for_map_transition: jest.fn(),
      check_for_warp_event: jest.fn(),
      check_for_coord_events: jest.fn(),
      check_for_wild_encounter: jest.fn(),
    } as unknown as OverworldEngine;

    expect(() => OverworldEngine.prototype._process_player_events.call(context)).not.toThrow();
    expect(discardPendingWildEncounter).toHaveBeenCalledTimes(1);
  });

  it("binds movement and script task checks to the overworld context", () => {
    const overworld = new TrainerSightlineOverworld();
    overworld.npcs = [
      {
        event: { object_type: "OBJECTTYPE_TRAINER", script: "TEST_SCRIPT" },
        objectIndex: 1,
      },
    ];

    expect(() => overworld.check_for_trainer_sightlines()).not.toThrow();
    expect(overworld.player_movement_locked_context).toBe(overworld);
    expect(overworld.script_tasks_active_context).toBe(overworld);
  });

  it("binds queued delay callbacks to the overworld context", () => {
    const overworld = new TrainerSightlineQueueOverworld();
    const npc: DummyOverworldObject = {
      event: { object_type: "OBJECTTYPE_TRAINER", script: "TEST_SCRIPT" },
      objectIndex: 3,
    };

    expect(() =>
      (overworld as unknown as { _engage_trainer_via_sightline: Function })._engage_trainer_via_sightline(
        npc,
        2,
        "left"
      )
    ).not.toThrow();
    expect(overworld.queue_delay_context).toBe(overworld);
  });

  it("binds queued movement tasks to the overworld context", () => {
    const overworld = new TrainerSightlineQueueOverworld();
    const npc: DummyOverworldObject = {
      event: { object_type: "OBJECTTYPE_TRAINER", script: "TEST_SCRIPT" },
      objectIndex: 2,
    };

    expect(() =>
      (overworld as unknown as { _queue_trainer_walk: Function })._queue_trainer_walk({
        npc,
        direction: "up",
        distanceTiles: 2,
      })
    ).not.toThrow();
    expect(overworld.queue_movement_context).toBe(overworld);
  });

  it("starts sightline trainer scripts as object interactions without script fallthrough", () => {
    const overworld = new TrainerSightlineQueueOverworld();
    overworld.queue_delay = jest.fn(() => false);
    const npc: DummyOverworldObject = {
      event: { object_type: "OBJECTTYPE_TRAINER", script: "TEST_SCRIPT" },
      objectIndex: 4,
    };

    (overworld as unknown as { _engage_trainer_via_sightline: Function })._engage_trainer_via_sightline(
      npc,
      2,
      "down",
      0,
      2
    );

    expect(overworld.game_state.wram.last_talked).toBe(4);
    expect(overworld.script_runner.last_interaction_object_index).toBe(4);
    expect(overworld.script_runner.run).toHaveBeenCalledWith("TEST_SCRIPT", {
      allow_fallthrough: false,
    });
  });

  it("skips sightlines for trainers with a beaten flag", () => {
    const overworld = new TrainerSightlineFlagOverworld();
    overworld.npcs = [
      {
        event: { object_type: "OBJECTTYPE_TRAINER", script: "TrainerTest", radius: 3 },
        objectIndex: 7,
        x: 0,
        y: 0,
        direction: "down",
      },
    ];

    const engaged = overworld.check_for_trainer_sightlines();
    expect(engaged).toBe(false);
    expect(overworld._trainer_cutscene_active).toBe(false);
    expect(overworld.game_state.wram.seen_trainer_distance).toBe(0);
  });

  it("skips ObjectEvent trainer placeholders that are owned by coord-event scripts", () => {
    const overworld = new TrainerSightlineFlagOverworld();
    overworld.game_state.wram.event_flags = {};
    overworld.npcs = [
      {
        event: { object_type: "OBJECTTYPE_TRAINER", script: "ObjectEvent", radius: 3 },
        objectIndex: 8,
        x: 0,
        y: 0,
        direction: "down",
      },
    ];

    expect(() => overworld.check_for_trainer_sightlines()).not.toThrow();
    expect(overworld.check_for_trainer_sightlines()).toBe(false);
    expect(overworld._trainer_cutscene_active).toBe(false);
    expect(overworld.game_state.wram.seen_trainer_distance).toBe(0);
  });

  it("throws when a trainer script has malformed trainer args", () => {
    const overworld = new TrainerSightlineMalformedScriptOverworld();
    overworld.npcs = [
      {
        event: { object_type: "OBJECTTYPE_TRAINER", script: "TrainerBadArgs", radius: 3 },
        objectIndex: 9,
        x: 0,
        y: 0,
        direction: "down",
      },
    ];

    expect(() => overworld.check_for_trainer_sightlines()).toThrow(
      "Trainer sightline script 'TrainerBadArgs' has malformed trainer arguments."
    );
  });
});

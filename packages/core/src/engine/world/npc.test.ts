import { createInitialGameState, type GameState } from "@pokecrystal/core/core/state";
import type { Trainer } from "@pokecrystal/core/core/models";
import type { ObjectEvent } from "@pokecrystal/core/core/models/map";
import { checkNpcInteractions, getTrainer } from "./npc";

type GameStateWithLoader = GameState & {
  data_loader?: DataLoaderLike | null;
};

type DataLoaderLike = {
  npc_data: Record<string, ObjectEvent[]>;
  get_script?: (scriptName: string) => unknown[] | null;
  get_trainer?: (trainerId: string) => Trainer | undefined;
  getTrainer?: (trainerId: string) => Trainer | undefined;
  load_npc_data?: jest.Mock;
  load_trainer_data?: jest.Mock;
};

const createTrainer = (trainerId: string): Trainer =>
  ({
    name: `Trainer ${trainerId}`,
    trainer_id: trainerId,
    trainer_class: "TRAINER_CLASS",
    party: [],
    win_quote: "",
    lose_quote: "",
    items: [],
    base_reward: 0,
    ai_move_flags: 0,
    ai_item_switch_flags: 0,
    encounter_music: "",
    ai_layers: [],
  }) as Trainer;

const createObjectEvent = (overrides: Partial<ObjectEvent> = {}): ObjectEvent =>
  ({
    sprite: "SPRITE_NPC",
    sprite_has_facings: true,
    x: 0,
    y: 0,
    spritemovedata: "SPRITEMOVEDATA_STANDING_LEFT",
    move_range_x: 0,
    move_range_y: 0,
    hram_x: 0,
    hram_y: 0,
    pal: 0,
    object_type: "OBJECTTYPE_SCRIPT",
    radius: 0,
    script: "",
    event_flag: "",
    object_identifier: null,
    sightline_direction_override: null,
    ...overrides,
  }) as ObjectEvent;

const createGameState = (setup?: (state: GameState) => void): GameStateWithLoader => {
  const state = createInitialGameState();
  state.wram.current_map_group = -1;
  state.wram.current_map_id = -1;
  state.wram.wMapGroup = -1;
  state.wram.wMapNumber = -1;
  state.wram.pending_last_talked_map = "Town_Square";
  setup?.(state);
  return state as GameStateWithLoader;
};

describe("checkNpcInteractions", () => {
  it("prefers pending_last_talked_object when present", () => {
    const npcA = createObjectEvent({ x: 1, y: 1, script: "A" });
    const npcB = createObjectEvent({ x: 2, y: 2, script: "B" });
    const state = createGameState((gameState) => {
      gameState.wram.pending_last_talked_object = 2;
      gameState.wram.last_talked = 1;
      gameState.wram.pending_last_talked_position = [9, 9];
      gameState.data_loader = {
        npc_data: {
          Town_Square: [npcA, npcB],
        },
        load_npc_data: jest.fn(),
      };
    });

    expect(checkNpcInteractions(state)).toEqual(npcB);
  });

  it("falls back to pending_last_talked_position for coordinate matches", () => {
    const npcA = createObjectEvent({ x: 4, y: 5, script: "A" });
    const state = createGameState((gameState) => {
      gameState.wram.pending_last_talked_object = 99;
      gameState.wram.pending_last_talked_position = [4, 5];
      gameState.data_loader = {
        npc_data: {
          Town_Square: [npcA],
        },
        load_npc_data: jest.fn(),
      };
    });

    expect(checkNpcInteractions(state)).toEqual(npcA);
  });

  it("throws when the selected NPC object event is malformed", () => {
    const invalidNpc = {
      x: 4,
      y: 5,
      script: "A",
    } as ObjectEvent;
    const state = createGameState((gameState) => {
      gameState.wram.pending_last_talked_object = 1;
      gameState.data_loader = {
        npc_data: {
          Town_Square: [invalidNpc],
        },
        load_npc_data: jest.fn(),
      };
    });

    expect(() => checkNpcInteractions(state)).toThrow(
      "ASM-backed NPC object event is invalid for object index 1."
    );
  });

  it("falls back to last_talked when no pending object is set", () => {
    const npcA = createObjectEvent({ x: 1, y: 1 });
    const state = createGameState((gameState) => {
      gameState.wram.pending_last_talked_position = undefined;
      gameState.wram.last_talked = 1;
      gameState.data_loader = {
        npc_data: {
          Town_Square: [npcA],
        },
        load_npc_data: jest.fn(),
      };
    });

    expect(checkNpcInteractions(state)).toEqual(npcA);
  });

  it("throws when coordinate lookup encounters a malformed NPC object event", () => {
    const invalidNpc = {
      x: 4,
      y: 5,
      script: "A",
    } as ObjectEvent;
    const state = createGameState((gameState) => {
      gameState.wram.pending_last_talked_object = 99;
      gameState.wram.pending_last_talked_position = [4, 5];
      gameState.data_loader = {
        npc_data: {
          Town_Square: [invalidNpc],
        },
        load_npc_data: jest.fn(),
      };
    });

    expect(() => checkNpcInteractions(state)).toThrow(
      "ASM-backed NPC object event is invalid for object index 1."
    );
  });
});

describe("getTrainer", () => {
  it("resolves trainer IDs from script loadtrainer commands", () => {
    const target = createTrainer("BROCK");
    const state = createGameState((gameState) => {
      gameState.data_loader = {
        npc_data: {
          Town_Square: [],
        },
        get_script: (scriptName: string) =>
          scriptName === "TrainerScript"
            ? [{ command: "loadtrainer", args: ["bug", "brock"] }]
            : null,
        load_npc_data: jest.fn(),
        load_trainer_data: jest.fn(),
        get_trainer: (trainerId) => (trainerId === "BROCK" ? target : undefined),
      };
    });

    const trainer = getTrainer(state, createObjectEvent({ script: "TrainerScript" }));
    expect(state.data_loader?.load_trainer_data).toHaveBeenCalledTimes(1);
    expect(trainer).toBe(target);
  });

  it("falls back to object_identifier when script does not provide one", () => {
    const target = createTrainer("ELSA");
    const state = createGameState((gameState) => {
      gameState.data_loader = {
        npc_data: {
          Town_Square: [],
        },
        load_npc_data: jest.fn(),
        load_trainer_data: jest.fn(),
        get_trainer: (trainerId) => (trainerId === "ELSA" ? target : undefined),
      };
    });

    const trainer = getTrainer(
      state,
      createObjectEvent({ script: "NoMatchScript", object_identifier: "Elsa" })
    );
    expect(trainer).toBe(target);
  });
});

import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { Ability, EggGroup, GenderRatio, GrowthRate, PokemonType } from "@pokecrystal/core/core/enums";
import { BoxSchema, PokemonSchema, PokemonSpeciesSchema, toPokemon, type Box, type Pokemon, type PokemonSpecies } from "@pokecrystal/core/core/models";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/special-events/utils";
import { createScriptRunnerStub } from "@pokecrystal/core/engine/world/story-events/test-utils";
import { check_for_lucky_number_winners, check_lucky_number_show_flag, reset_lucky_number_show_flag } from "./lucky-number";

const BASE_STATS = {
  hp: 20,
  attack: 10,
  defense: 10,
  speed: 10,
  special_attack: 10,
  special_defense: 10,
};

const speciesCache = new Map<string, PokemonSpecies>();

const ensureSpecies = (id: string): PokemonSpecies => {
  const normalized = id.toUpperCase();
  const cached = speciesCache.get(normalized);
  if (cached) {
    return cached;
  }
  const species = PokemonSpeciesSchema.parse({
    id: normalized,
    int_id: 0,
    base_stats: BASE_STATS,
    type1: PokemonType.NORMAL,
    type2: PokemonType.NONE,
    catch_rate: 45,
    base_exp: 64,
    item1: null,
    item2: null,
    gender_ratio: GenderRatio.GENDER_F50,
    unknown1: 0,
    step_cycles_to_hatch: 5120,
    unknown2: 0,
    growth_rate: GrowthRate.GROWTH_MEDIUM_FAST,
    egg_group1: EggGroup.EGG_MONSTER,
    egg_group2: EggGroup.EGG_MONSTER,
    tmhm_learnset: [],
    ability: Ability.NONE,
    pic_size: 0,
    front_pic: 0,
    back_pic: 0,
  });
  speciesCache.set(normalized, species);
  return species;
};

const makePartyPokemon = (speciesId: string, overrides: Partial<Pokemon> = {}): Pokemon =>
  toPokemon(
    PokemonSchema.parse({
      species: ensureSpecies(speciesId),
      nickname: speciesId,
      level: 5,
      hp: 20,
      max_hp: 20,
      experience: 0,
      original_trainer_name: "PLAYER",
      original_trainer_id: 0,
      happiness: 70,
      ...overrides,
    })
  );

const makeRunner = (): ScriptRunner =>
  createScriptRunnerStub({
    variables: {},
    string_buffers: {},
  });

const makePcBox = (name: string): Box => BoxSchema.parse({ name });

const seedPcBoxes = (gameState: ReturnType<typeof createInitialGameState>, count = 2): void => {
  gameState.sram.pc_boxes = Array.from({ length: count }, (_value, index) =>
    makePcBox(`BOX ${String(index + 1).padStart(2, "0")}`)
  );
};

describe("Lucky number specials", () => {
  it("reports a perfect match and stores the winning species", () => {
    const gameState = createInitialGameState();
    gameState.sram.lucky_id_number = 12345;
    gameState.sram.lucky_number_day = 1;
    gameState.wram.wCurDay = 1;
    gameState.wram.wPartyCount = 1;

    gameState.sram.party.pokemon[0] = makePartyPokemon("PIKACHU", {
      original_trainer_id: 12345,
      original_trainer_name: "PLAYER",
    });

    const runner = makeRunner();
    const eventManager = new EventManager(gameState);

    const result = check_for_lucky_number_winners(gameState, {
      runner,
      event_manager: eventManager,
    });

    expect(result).toBe(1);
    expect(runner.last_value).toBe(1);
    expect(gameState.wram.wCurPartySpecies).toBe("PIKACHU");
  });

  it("does not scan PC winners when the party is empty", () => {
    const gameState = createInitialGameState();
    gameState.sram.lucky_id_number = 12345;
    gameState.sram.lucky_number_day = 1;
    gameState.wram.wCurDay = 1;
    gameState.wram.wPartyCount = 0;
    gameState.sram.party.pokemon = [];
    seedPcBoxes(gameState, 1);
    gameState.sram.pc_boxes[0].pokemon[0] = makePartyPokemon("PIKACHU", {
      original_trainer_id: 12345,
      original_trainer_name: "PLAYER",
    });

    const runner = makeRunner();
    const eventManager = new EventManager(gameState);

    const result = check_for_lucky_number_winners(gameState, {
      runner,
      event_manager: eventManager,
    });

    expect(result).toBe(0);
    expect(runner.last_value).toBe(0);
    expect(gameState.wram.wCurPartySpecies).toBe("");
  });

  it("does not fall back to stale party array contents when wPartyCount is zero", () => {
    const gameState = createInitialGameState();
    gameState.sram.lucky_id_number = 12345;
    gameState.sram.lucky_number_day = 1;
    gameState.wram.wCurDay = 1;
    gameState.wram.wPartyCount = 0;
    gameState.sram.party.pokemon[0] = makePartyPokemon("PIKACHU", {
      original_trainer_id: 12345,
      original_trainer_name: "PLAYER",
    });

    const result = check_for_lucky_number_winners(gameState, {
      runner: makeRunner(),
      event_manager: new EventManager(gameState),
    });

    expect(result).toBe(0);
    expect(gameState.wram.wCurPartySpecies).toBe("");
  });

  it("throws when the ASM-backed party count is invalid", () => {
    const gameState = createInitialGameState();
    gameState.wram.wPartyCount = "not_a_number" as unknown as number;

    expect(() =>
      check_for_lucky_number_winners(gameState, {
        runner: makeRunner(),
        event_manager: new EventManager(gameState),
      })
    ).toThrow("ASM-backed party count is invalid: not_a_number.");
  });

  it("throws when the ASM-backed current PC box index is missing", () => {
    const gameState = createInitialGameState();
    gameState.sram.lucky_id_number = 12345;
    gameState.sram.lucky_number_day = 1;
    gameState.wram.wCurDay = 1;
    gameState.wram.wPartyCount = 1;
    gameState.sram.party.pokemon[0] = makePartyPokemon("CATERPIE", {
      original_trainer_id: 10000,
      original_trainer_name: "PLAYER",
    });
    seedPcBoxes(gameState, 1);
    gameState.sram.current_pc_box = undefined as unknown as number;

    expect(() =>
      check_for_lucky_number_winners(gameState, {
        runner: makeRunner(),
        event_manager: new EventManager(gameState),
      })
    ).toThrow("ASM-backed current PC box index is invalid: undefined.");
  });

  it("throws when a mon has an invalid ASM-backed trainer ID", () => {
    const gameState = createInitialGameState();
    gameState.sram.lucky_id_number = 12345;
    gameState.sram.lucky_number_day = 1;
    gameState.wram.wCurDay = 1;
    gameState.wram.wPartyCount = 1;
    gameState.sram.party.pokemon[0] = makePartyPokemon("PIKACHU", {
      original_trainer_name: "PLAYER",
    });
    gameState.sram.party.pokemon[0]!.original_trainer_id = "oops" as unknown as number;

    expect(() =>
      check_for_lucky_number_winners(gameState, {
        runner: makeRunner(),
        event_manager: new EventManager(gameState),
      })
    ).toThrow("ASM-backed trainer ID is invalid: oops.");
  });

  it("prefers the current PC box over other boxes for equal-tier PC winners", () => {
    const gameState = createInitialGameState();
    gameState.sram.lucky_id_number = 12345;
    gameState.sram.lucky_number_day = 1;
    gameState.wram.wCurDay = 1;
    gameState.wram.wPartyCount = 1;
    gameState.sram.party.pokemon[0] = makePartyPokemon("CATERPIE", {
      original_trainer_id: 10000,
      original_trainer_name: "PLAYER",
    });
    seedPcBoxes(gameState, 2);
    gameState.sram.current_pc_box = 1;
    gameState.sram.pc_boxes[0].count = 1;
    gameState.sram.pc_boxes[1].count = 1;
    gameState.sram.pc_boxes[0].pokemon[0] = makePartyPokemon("RATTATA", {
      original_trainer_id: 12345,
      original_trainer_name: "BOX0",
    });
    gameState.sram.pc_boxes[1].pokemon[0] = makePartyPokemon("PIKACHU", {
      original_trainer_id: 12345,
      original_trainer_name: "BOX1",
    });

    const result = check_for_lucky_number_winners(gameState, {
      runner: makeRunner(),
      event_manager: new EventManager(gameState),
    });

    expect(result).toBe(1);
    expect(gameState.wram.wCurPartySpecies).toBe("PIKACHU");
  });

  it("lets a later equal-tier PC winner override an earlier party winner", () => {
    const gameState = createInitialGameState();
    gameState.sram.lucky_id_number = 12345;
    gameState.sram.lucky_number_day = 1;
    gameState.wram.wCurDay = 1;
    gameState.wram.wPartyCount = 1;
    gameState.sram.party.pokemon[0] = makePartyPokemon("CATERPIE", {
      original_trainer_id: 12345,
      original_trainer_name: "PLAYER",
    });
    seedPcBoxes(gameState, 1);
    gameState.sram.current_pc_box = 0;
    gameState.sram.pc_boxes[0].count = 1;
    gameState.sram.pc_boxes[0].pokemon[0] = makePartyPokemon("PIKACHU", {
      original_trainer_id: 12345,
      original_trainer_name: "BOX0",
    });

    const result = check_for_lucky_number_winners(gameState, {
      runner: makeRunner(),
      event_manager: new EventManager(gameState),
    });

    expect(result).toBe(1);
    expect(gameState.wram.wCurPartySpecies).toBe("PIKACHU");
  });

  it("masks high bits from the ASM-backed current box value before ordering PC scans", () => {
    const gameState = createInitialGameState();
    gameState.sram.lucky_id_number = 12345;
    gameState.sram.lucky_number_day = 1;
    gameState.wram.wCurDay = 1;
    gameState.wram.wPartyCount = 1;
    gameState.sram.party.pokemon[0] = makePartyPokemon("CATERPIE", {
      original_trainer_id: 10000,
      original_trainer_name: "PLAYER",
    });
    seedPcBoxes(gameState, 2);
    gameState.sram.current_pc_box = 0x11;
    gameState.sram.pc_boxes[0].count = 1;
    gameState.sram.pc_boxes[1].count = 1;
    gameState.sram.pc_boxes[0].pokemon[0] = makePartyPokemon("RATTATA", {
      original_trainer_id: 12345,
      original_trainer_name: "BOX0",
    });
    gameState.sram.pc_boxes[1].pokemon[0] = makePartyPokemon("PIKACHU", {
      original_trainer_id: 12345,
      original_trainer_name: "BOX1",
    });

    const result = check_for_lucky_number_winners(gameState, {
      runner: makeRunner(),
      event_manager: new EventManager(gameState),
    });

    expect(result).toBe(1);
    expect(gameState.wram.wCurPartySpecies).toBe("PIKACHU");
  });

  it("does not scan PC mons beyond each box count", () => {
    const gameState = createInitialGameState();
    gameState.sram.lucky_id_number = 12345;
    gameState.sram.lucky_number_day = 1;
    gameState.wram.wCurDay = 1;
    gameState.wram.wPartyCount = 1;
    gameState.sram.party.pokemon[0] = makePartyPokemon("CATERPIE", {
      original_trainer_id: 10000,
      original_trainer_name: "PLAYER",
    });

    seedPcBoxes(gameState, 1);
    gameState.sram.current_pc_box = 0;
    gameState.sram.pc_boxes[0].count = 0;
    gameState.sram.pc_boxes[0].pokemon[0] = makePartyPokemon("PIKACHU", {
      original_trainer_id: 12345,
      original_trainer_name: "BOX0",
    });

    const result = check_for_lucky_number_winners(gameState, {
      runner: makeRunner(),
      event_manager: new EventManager(gameState),
    });

    expect(result).toBe(0);
    expect(gameState.wram.wCurPartySpecies).toBe("");
  });

  it("throws when a PC box count is missing instead of falling back to populated slots", () => {
    const gameState = createInitialGameState();
    gameState.sram.lucky_id_number = 12345;
    gameState.sram.lucky_number_day = 1;
    gameState.wram.wCurDay = 1;
    gameState.wram.wPartyCount = 1;
    gameState.sram.party.pokemon[0] = makePartyPokemon("CATERPIE", {
      original_trainer_id: 10000,
      original_trainer_name: "PLAYER",
    });
    seedPcBoxes(gameState, 1);
    gameState.sram.current_pc_box = 0;
    gameState.sram.pc_boxes[0].count = undefined as unknown as number;
    gameState.sram.pc_boxes[0].pokemon[0] = makePartyPokemon("PIKACHU", {
      original_trainer_id: 12345,
      original_trainer_name: "BOX0",
    });

    expect(() =>
      check_for_lucky_number_winners(gameState, {
        runner: makeRunner(),
        event_manager: new EventManager(gameState),
      })
    ).toThrow("ASM-backed PC box count is invalid: undefined.");
  });

  it("throws when lucky number match text resolves to its own label token", () => {
    const gameState = createInitialGameState();
    gameState.sram.lucky_id_number = 12345;
    gameState.sram.lucky_number_day = 1;
    gameState.wram.wCurDay = 1;
    gameState.wram.wPartyCount = 1;

    gameState.sram.party.pokemon[0] = makePartyPokemon("PIKACHU", {
      original_trainer_id: 12345,
      original_trainer_name: "PLAYER",
    });

    const runner = makeRunner();
    runner.dataLoader = {
      getText: (label: string) => label,
    };

    expect(() =>
      check_for_lucky_number_winners(gameState, {
        runner,
        event_manager: new EventManager(gameState),
      })
    ).toThrow("Missing ASM text for label 'LuckyNumberMatchPartyText'.");
  });

  it("returns false for show flag checks when cleared", () => {
    const gameState = createInitialGameState();
    const runner = makeRunner();

    expect(check_lucky_number_show_flag(gameState, { runner })).toBe(false);
    expect(runner.last_value).toBe(0);
    expect(reset_lucky_number_show_flag(gameState, { runner })).toBe(true);
    expect(gameState.wram.lucky_number_show_flag).toBe(false);
  });
});

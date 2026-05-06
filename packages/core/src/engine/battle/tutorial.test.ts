import { createInitialGameState } from "@pokecrystal/core/core/state";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import { CatchTutorialRunner } from "./tutorial";
import { PokemonSpeciesSchema, createPokemon } from "@pokecrystal/core/core/models/pokemon";
import * as dataLoader from "@pokecrystal/core/core/data-loader";

const buildSpecies = () =>
  PokemonSpeciesSchema.parse({
    id: "RATTATA",
    int_id: 19,
    base_stats: {
      hp: 30,
      attack: 56,
      defense: 35,
      speed: 72,
      special_attack: 25,
      special_defense: 35,
    },
    type1: "NORMAL",
    type2: "NORMAL",
    catch_rate: 255,
    base_exp: 57,
    gender_ratio: 31,
    unknown1: 0,
    step_cycles_to_hatch: 15,
    unknown2: 0,
    growth_rate: "GROWTH_MEDIUM_FAST",
    egg_group1: "EGG_GROUND",
    egg_group2: "EGG_GROUND",
  });

describe("CatchTutorialRunner", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("clears stale trainer context before starting the tutorial battle", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const species = buildSpecies();
    const playerMon = createPokemon(gameState, species, 5);
    gameState.sram.party.pokemon = [playerMon, null, null, null, null, null];
    gameState.wram.other_trainer_class = "RIVAL1";
    gameState.wram.other_trainer_id = "RIVAL1_1";
    gameState.wram.other_trainer = { name: "RIVAL" } as never;
    gameState.wram.other_trainer_party = [createPokemon(gameState, species, 5)];

    const onComplete = jest.fn();
    let trainerSnapshot: {
      trainerClass: string;
      trainerId: string;
      trainerPartySize: number;
    } | null = null;

    eventManager.on("start_battle", () => {
      trainerSnapshot = {
        trainerClass: gameState.wram.other_trainer_class,
        trainerId: gameState.wram.other_trainer_id,
        trainerPartySize: gameState.wram.other_trainer_party.length,
      };
      eventManager.dispatch(new Event("battle_complete", { result: 0 }));
    });

    const runner = new CatchTutorialRunner(gameState, eventManager, {
      get_pokemon_species: (name: string) => (name === "RATTATA" ? species : undefined),
    });

    runner.run({
      wild_species: "RATTATA",
      wild_level: 5,
      battle_type: "BATTLETYPE_TUTORIAL",
      on_complete: onComplete,
    });

    expect(trainerSnapshot).toEqual({
      trainerClass: "",
      trainerId: "",
      trainerPartySize: 0,
    });
    expect(gameState.wram.other_trainer_class).toBe("");
    expect(gameState.wram.other_trainer_id).toBe("");
    expect(gameState.wram.other_trainer).toBeUndefined();
    expect(gameState.wram.other_trainer_party).toEqual([]);
    expect(onComplete).toHaveBeenCalledTimes(1);
  });

  it("falls back to the canonical species table when the injected loader cannot resolve the tutorial species", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const species = buildSpecies();
    const playerMon = createPokemon(gameState, species, 5);
    gameState.sram.party.pokemon = [playerMon, null, null, null, null, null];

    jest.spyOn(dataLoader, "getSpecies").mockReturnValue(species);

    const onComplete = jest.fn();
    eventManager.on("start_battle", () => {
      eventManager.dispatch(new Event("battle_complete", { result: 0 }));
    });

    const runner = new CatchTutorialRunner(gameState, eventManager, {});

    expect(() =>
      runner.run({
        wild_species: "RATTATA",
        wild_level: 5,
        battle_type: "BATTLETYPE_TUTORIAL",
        on_complete: onComplete,
      }),
    ).not.toThrow();

    expect(onComplete).toHaveBeenCalledTimes(1);
    expect(dataLoader.getSpecies).toHaveBeenCalledWith("RATTATA");
  });
});

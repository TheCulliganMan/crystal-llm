import { createInitialGameState, type GameState } from "@pokecrystal/core/core/state";
import { getPokedexFlag, recordPokedexCaught } from "@pokecrystal/core/core/pokedex";
import { Ability, EggGroup, GenderRatio, GrowthRate, PokemonType } from "@pokecrystal/core/core/enums";
import { Pokemon, PokemonSpecies } from "@pokecrystal/core/core/models";
import { ItemSystem } from "@pokecrystal/core/engine/systems/items";
import { createPokemon } from "@pokecrystal/core/engine/systems/pokemon";
import { toPokemon } from "@pokecrystal/core/core/models/pokemon";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import { createScriptRunnerStub } from "@pokecrystal/core/engine/world/story-events/test-utils";
import {
  beasts_check,
  find_party_mon_that_species,
  find_party_mon_that_species_your_trainer_id,
  game_corner_prize_mon_check_dex,
} from "./pokemon";

const DEFAULT_BASE_STATS = {
  hp: 20,
  attack: 10,
  defense: 10,
  speed: 10,
  special_attack: 10,
  special_defense: 10,
};

type PokemonDataLoader = DataLoader & {
  get_pokemon_species?: (id: string) => PokemonSpecies | null;
  getPokemonSpecies?: (id: string) => PokemonSpecies | null;
  getSpecies?: (id: string) => PokemonSpecies | null;
};

type PokemonTestRunner = ScriptRunner & {
  data_loader: PokemonDataLoader;
  variables: { _value: string | number };
  last_value?: number;
  last_condition_result?: boolean;
};

const speciesCache = new Map<string, PokemonSpecies>();

const ensureSpecies = (id: string): PokemonSpecies => {
  const normalized = id.toUpperCase();
  const cached = speciesCache.get(normalized);
  if (cached) {
    return cached;
  }
  const species: PokemonSpecies = {
    id: normalized,
    int_id: 0,
    base_stats: { ...DEFAULT_BASE_STATS },
    type1: PokemonType.NORMAL,
    type2: PokemonType.NONE,
    catch_rate: 45,
    base_exp: 64,
    item1: undefined,
    item2: undefined,
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
    evolutions: null,
    weight: 0,
  };
  speciesCache.set(normalized, species);
  return species;
};

const makePokemon = (gameState: GameState, speciesId: string, level = 5): Pokemon =>
  toPokemon(createPokemon(gameState, ensureSpecies(speciesId), level));

const makePlayerOwnedPokemon = (gameState: GameState, speciesId: string, level = 40): Pokemon => {
  const mon = makePokemon(gameState, speciesId, level);
  mon.original_trainer_name = String(gameState.sram.player_name ?? "");
  mon.original_trainer_id = Number(gameState.sram.player_id ?? 0);
  return mon;
};

const createTestRunner = (gameState: GameState, value: string | number): PokemonTestRunner => {
  const dataLoader = new DataLoader();
  dataLoader.get_script = () => null;
  dataLoader.get_text = () => null;
  return createScriptRunnerStub({
    data_loader: dataLoader,
    item_system: new ItemSystem(gameState),
    run: jest.fn(),
    variables: { _value: value },
    string_buffers: {},
    stop_execution: false,
  }) as PokemonTestRunner;
};

describe("Pokemon specials", () => {
  it("finds a party mon by species", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon[0] = makePokemon(gameState, "ABRA");
    const runner = createTestRunner(gameState, "ABRA");

    const result = find_party_mon_that_species(gameState, { runner });

    expect(result).toBe(true);
    expect(runner.last_value).toBe(1);
  });

  it("finds a party mon by species and trainer id", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "PLAYER";
    gameState.sram.player_id = 999;
    const partyMon = makePokemon(gameState, "EEVEE");
    partyMon.original_trainer_name = "PLAYER";
    partyMon.original_trainer_id = 999;
    gameState.sram.party.pokemon[0] = partyMon;
    const runner = createTestRunner(gameState, "EEVEE");

    const result = find_party_mon_that_species_your_trainer_id(gameState, { runner });

    expect(result).toBe(true);
    expect(runner.last_value).toBe(1);
  });

  it("marks prize mons as caught in the pokedex", () => {
    const gameState = createInitialGameState();
    const runner = createTestRunner(gameState, "ABRA");
    const seenEvents: Array<{ species_id: string; pokedex_number: number }> = [];
    runner.event_manager?.on("show_pokedex_entry", (event) => {
      const data = event.data as { species_id?: string; pokedex_number?: number };
      if (data.species_id && data.pokedex_number) {
        seenEvents.push({ species_id: data.species_id, pokedex_number: data.pokedex_number });
      }
    });
    runner.data_loader.get_pokemon_species = (id: string) => ({
      ...ensureSpecies(id),
      int_id: 63,
    });

    const result = game_corner_prize_mon_check_dex(gameState, { runner });

    expect(result).toBe(true);
    expect(gameState.wram.wNamedObjectIndex).toBe(63);
    expect(getPokedexFlag(gameState, 63, "seen")).toBe(true);
    expect(getPokedexFlag(gameState, 63, "owned")).toBe(true);
    const caught = gameState.sram.pokedex_caught as Set<number>;
    expect(caught.has(63)).toBe(true);
    expect(seenEvents).toEqual([{ species_id: "ABRA", pokedex_number: 63 }]);
  });

  it("skips dex entry when the prize mon is already caught", () => {
    const gameState = createInitialGameState();
    recordPokedexCaught(gameState, { int_id: 133 });
    const runner = createTestRunner(gameState, "EEVEE");
    const seenEvents: Array<{ species_id: string; pokedex_number: number }> = [];
    runner.event_manager?.on("show_pokedex_entry", (event) => {
      const data = event.data as { species_id?: string; pokedex_number?: number };
      if (data.species_id && data.pokedex_number) {
        seenEvents.push({ species_id: data.species_id, pokedex_number: data.pokedex_number });
      }
    });
    runner.data_loader.get_pokemon_species = (id: string) => ({
      ...ensureSpecies(id),
      int_id: 133,
    });
    gameState.wram.wNamedObjectIndex = 7;

    const result = game_corner_prize_mon_check_dex(gameState, { runner });

    expect(result).toBe(false);
    expect(gameState.wram.wNamedObjectIndex).toBe(7);
    expect(seenEvents).toEqual([]);
  });

  it("passes BeastsCheck only when the player owns Raikou, Entei, and Suicune with their OT", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "CRIS";
    gameState.sram.player_id = 12345;
    const runner = createTestRunner(gameState, 0);

    gameState.sram.party.pokemon = [
      makePlayerOwnedPokemon(gameState, "RAIKOU"),
      makePlayerOwnedPokemon(gameState, "ENTEI"),
      makePlayerOwnedPokemon(gameState, "SUICUNE"),
      null,
      null,
      null,
    ];

    expect(beasts_check(gameState, { runner })).toBe(true);
    expect(runner.variables._value).toBe(1);
    expect(runner.last_value).toBe(1);
    expect(runner.last_condition_result).toBe(true);
  });

  it("fails BeastsCheck when Suicune is missing or owned by another trainer", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "CRIS";
    gameState.sram.player_id = 12345;
    const runner = createTestRunner(gameState, 0);

    const tradedSuicune = makePlayerOwnedPokemon(gameState, "SUICUNE");
    tradedSuicune.original_trainer_name = "EUSINE";
    gameState.sram.party.pokemon = [
      makePlayerOwnedPokemon(gameState, "RAIKOU"),
      makePlayerOwnedPokemon(gameState, "ENTEI"),
      tradedSuicune,
      null,
      null,
      null,
    ];

    expect(beasts_check(gameState, { runner })).toBe(false);
    expect(runner.variables._value).toBe(0);
    expect(runner.last_value).toBe(0);
    expect(runner.last_condition_result).toBe(false);

    tradedSuicune.original_trainer_name = "CRIS";
    gameState.sram.party.pokemon[2] = null;
    expect(beasts_check(gameState, { runner })).toBe(false);
  });
});

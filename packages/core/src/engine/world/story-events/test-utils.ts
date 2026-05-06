import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import type { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import {
  Ability,
  EggGroup,
  GenderRatio,
  GrowthRate,
  PokemonType,
} from "@pokecrystal/core/core/enums";
import {
  PokemonSchema,
  type Pokemon,
  type PokemonSpecies,
  pokemonSpeciesDisplayName,
} from "@pokecrystal/core/core/models";
import { toPokemon } from "@pokecrystal/core/core/models/pokemon";

const buildBaseStats = (): PokemonSpecies["base_stats"] => ({
  hp: 10,
  attack: 5,
  defense: 5,
  speed: 5,
  special_attack: 5,
  special_defense: 5,
});

export const createTestSpecies = (
  id: string,
  intId: number,
): PokemonSpecies => ({
  id,
  int_id: intId,
  base_stats: buildBaseStats(),
  type1: PokemonType.NORMAL,
  type2: PokemonType.NONE,
  catch_rate: 255,
  base_exp: 100,
  gender_ratio: GenderRatio.GENDER_UNKNOWN,
  unknown1: 0,
  step_cycles_to_hatch: 256,
  unknown2: 0,
  growth_rate: GrowthRate.GROWTH_MEDIUM_FAST,
  egg_group1: EggGroup.EGG_HUMANSHAPE,
  egg_group2: EggGroup.EGG_NONE,
  tmhm_learnset: [],
  ability: Ability.NONE,
  pic_size: 0,
  front_pic: 0,
  back_pic: 0,
  evolutions: null,
  weight: 0,
});

type PokemonOverrides = Partial<Omit<Pokemon, "species">> & {
  species?: PokemonSpecies;
};

export const createTestPokemon = (
  speciesId: string,
  intId: number,
  overrides: PokemonOverrides = {},
): Pokemon => {
  const { species: overridesSpecies, ...rest } = overrides;
  return toPokemon(PokemonSchema.parse({
    species: overridesSpecies ?? createTestSpecies(speciesId, intId),
    nickname: pokemonSpeciesDisplayName(overridesSpecies ?? speciesId),
    level: 5,
    hp: 10,
    max_hp: 10,
    original_trainer_name: "PLAYER",
    original_trainer_id: 0,
    experience: 0,
    happiness: 0,
    ...rest,
  }));
};

const noop = (): void => {};

export const createOverworldEngineStub = <T extends Record<string, unknown> = Record<string, never>>(
  overrides: T = {} as T,
): OverworldEngine & T =>
  createOverworldStub(overrides) as unknown as OverworldEngine & T;

export const createScriptRunnerStub = <TOverworld extends OverworldEngine = OverworldEngine>(
  overrides: Partial<ScriptRunner> & { overworld?: TOverworld } = {},
): ScriptRunner & { overworld: TOverworld } => {
  const gameState = overrides.game_state ?? createInitialGameState();
  const eventManager = overrides.event_manager ?? new EventManager(gameState);
  const overworld = overrides.overworld ?? createOverworldEngineStub();
  return {
    game_state: gameState,
    event_manager: eventManager,
    overworld,
    variables: {},
    string_buffers: {},
    run: noop,
    jump: noop,
    call: noop,
    defer: noop,
    pause: noop,
    resume: noop,
    ...overrides,
  } as ScriptRunner & { overworld: TOverworld };
};

export const createOverworldStub = <T extends Record<string, unknown> = Record<string, never>>(
  overrides?: T,
): OverworldMap & T => ({
  mapName: "test_map",
  width: 1,
  height: 1,
  metatileIds: [],
  getMetatileAt: () => 0,
  ...overrides,
} as unknown as OverworldMap & T);

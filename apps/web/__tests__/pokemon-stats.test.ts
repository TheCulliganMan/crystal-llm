import { createInitialGameState } from "@/core/state";
import { PokemonSchema, PokemonSpeciesSchema, toPokemon } from "@/core/models";
import { PokemonType } from "@/core/enums";
import { PokemonStatsScreen } from "@/ui/menus/pokemon-stats";

const buildSpecies = () =>
  PokemonSpeciesSchema.parse({
    id: "TOTODILE",
    int_id: 158,
    base_stats: {
      hp: 50,
      attack: 65,
      defense: 64,
      speed: 43,
      special_attack: 44,
      special_defense: 48,
    },
    type1: PokemonType.WATER,
    type2: PokemonType.WATER,
    catch_rate: 45,
    base_exp: 63,
    gender_ratio: 31,
    unknown1: 0,
    step_cycles_to_hatch: 21,
    unknown2: 0,
    growth_rate: "GROWTH_MEDIUM_SLOW",
    egg_group1: "EGG_MONSTER",
    egg_group2: "EGG_WATER_1",
  });

const buildPokemon = () =>
  toPokemon(
    PokemonSchema.parse({
      species: buildSpecies(),
      nickname: "TOTODILE",
      level: 5,
      hp: 20,
      max_hp: 20,
      dvs: {
        attack: 9,
        defense: 8,
        speed: 1,
        special: 7,
        hp: 8,
      },
      original_trainer_name: "PLAYER",
      original_trainer_id: 0,
      experience: 125,
      happiness: 70,
    })
  );

describe("PokemonStatsScreen input", () => {
  it("exits on B button events", () => {
    const gameState = createInitialGameState();
    const pokemon = buildPokemon();
    gameState.sram.party.pokemon[0] = pokemon;
    const stats = new PokemonStatsScreen({ screen: null, font: {} }, gameState);

    stats.showPokemon(pokemon);
    const result = stats.handleInput({ type: "keydown", button: "b" });

    expect(result).toBe("exit");
  });

  it("exits on A from the final page when using button events", () => {
    const gameState = createInitialGameState();
    const pokemon = buildPokemon();
    gameState.sram.party.pokemon[0] = pokemon;
    const stats = new PokemonStatsScreen({ screen: null, font: {} }, gameState);

    stats.showPokemon(pokemon);
    stats.handleInput({ type: "keydown", button: "a" });
    stats.handleInput({ type: "keyup", button: "a" });
    stats.handleInput({ type: "keydown", button: "a" });
    stats.handleInput({ type: "keyup", button: "a" });
    const result = stats.handleInput({ type: "keydown", button: "a" });

    expect(result).toBe("exit");
  });
});

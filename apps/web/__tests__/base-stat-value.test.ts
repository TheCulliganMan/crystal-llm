import { PokemonSchema, PokemonSpeciesSchema, toPokemon } from '@/core/models';
import { PokemonType, Stat } from '@/core/enums';

describe('Pokemon._calculateStat', () => {
  it('calculates the correct stat values', () => {
    const species = PokemonSpeciesSchema.parse({
      id: 'BULBASAUR',
      int_id: 1,
      base_stats: {
        hp: 45,
        attack: 49,
        defense: 49,
        speed: 45,
        special_attack: 65,
        special_defense: 65,
      },
      type1: PokemonType.GRASS,
      type2: PokemonType.POISON,
      catch_rate: 45,
      base_exp: 64,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 21,
      unknown2: 0,
      growth_rate: "GROWTH_MEDIUM_SLOW",
      egg_group1: "EGG_MONSTER",
      egg_group2: "EGG_PLANT",
    });

    const pokemon = toPokemon(PokemonSchema.parse({
      species,
      nickname: 'BULBASAUR',
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
      original_trainer_name: 'JULES',
      original_trainer_id: 12345,
      experience: 125,
      happiness: 70,
    }));

    expect(pokemon._calculateStat(Stat.ATTACK)).toBe(10);
    expect(pokemon._calculateStat(Stat.DEFENSE)).toBe(10);
    expect(pokemon._calculateStat(Stat.SPEED)).toBe(9);
    expect(pokemon._calculateStat(Stat.SPECIAL_ATTACK)).toBe(12);
    expect(pokemon._calculateStat(Stat.SPECIAL_DEFENSE)).toBe(12);
  });
});

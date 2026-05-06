import { z } from 'zod';
import { PokemonSpeciesSchema, MoveSchema, PokemonSchema } from '../models';
import { toPokemon } from '../models/pokemon';
import { Stat, GenderRatio, GrowthRate, EggGroup, Ability, PokemonType, MoveName } from '../enums';

type PokemonSpecies = z.infer<typeof PokemonSpeciesSchema>;
type Move = z.infer<typeof MoveSchema>;

describe('PokemonSpecies', () => {
  it('should parse valid pokemon species data', () => {
    const data = {
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
      gender_ratio: GenderRatio.GENDER_F12_5,
      unknown1: 0,
      step_cycles_to_hatch: 21,
      unknown2: 0,
      growth_rate: GrowthRate.GROWTH_MEDIUM_SLOW,
      egg_group1: EggGroup.EGG_MONSTER,
      egg_group2: EggGroup.EGG_PLANT,
      tmhm_learnset: [],
      ability: Ability.NONE,
      pic_size: 0,
      front_pic: 0,
      back_pic: 0,
      evolutions: null,
      weight: 0,
    };
    const species = PokemonSpeciesSchema.parse(data);
    expect(species.id).toBe('BULBASAUR');
    expect(species.base_stats.hp).toBe(45);
  });
});

describe('Move', () => {
  it('should parse valid move data', () => {
    const data = {
      name: MoveName.TACKLE,
      type: PokemonType.NORMAL,
      power: 35,
      accuracy: 95,
      pp: 35,
    };
    const move = MoveSchema.parse(data);
    expect(move.name).toBe(MoveName.TACKLE);
    expect(move.power).toBe(35);
  });

  it('should parse moves with newly added effects (FOCUS_ENERGY, FORESIGHT, FRUSTRATION)', () => {
    const focusEnergy = {
      name: MoveName.FOCUS_ENERGY,
      type: PokemonType.NORMAL,
      power: 0,
      accuracy: 100,
      pp: 30,
      effect: 'FOCUS_ENERGY',
    };
    const foresight = {
      name: MoveName.FORESIGHT,
      type: PokemonType.NORMAL,
      power: 0,
      accuracy: 100,
      pp: 40,
      effect: 'FORESIGHT',
    };
    const frustration = {
      name: MoveName.FRUSTRATION,
      type: PokemonType.NORMAL,
      power: 0,
      accuracy: 100,
      pp: 20,
      effect: 'FRUSTRATION',
    };

    expect(MoveSchema.parse(focusEnergy).effect).toBe('FOCUS_ENERGY');
    expect(MoveSchema.parse(foresight).effect).toBe('FORESIGHT');
    expect(MoveSchema.parse(frustration).effect).toBe('FRUSTRATION');
  });
});

describe('Pokemon', () => {
  it('should calculate stats correctly, including stat experience', () => {
    const species: PokemonSpecies = {
      id: 'CHARMANDER',
      int_id: 4,
      base_stats: {
        hp: 39,
        attack: 52,
        defense: 43,
        speed: 65,
        special_attack: 60,
        special_defense: 50,
      },
      type1: PokemonType.FIRE,
      type2: PokemonType.FIRE,
      catch_rate: 45,
      base_exp: 62,
      gender_ratio: GenderRatio.GENDER_F12_5,
      unknown1: 0,
      step_cycles_to_hatch: 20,
      unknown2: 0,
      growth_rate: GrowthRate.GROWTH_MEDIUM_SLOW,
      egg_group1: EggGroup.EGG_MONSTER,
      egg_group2: EggGroup.EGG_DRAGON,
      tmhm_learnset: [],
      ability: Ability.NONE,
      pic_size: 0,
      front_pic: 0,
      back_pic: 0,
      evolutions: null,
      weight: 0,
    };

    const partialPokemon = {
        species: species,
        nickname: "CHARMANDER",
        level: 5,
        hp: 20,
        max_hp: 20,
        original_trainer_name: "Jules",
        original_trainer_id: 12345,
        experience: 125,
        happiness: 70,
    };
    const pokemon = toPokemon(PokemonSchema.parse(partialPokemon));

    pokemon.attack_exp = 2500;

    // Expected value is calculated using the correct formula, including stat experience
    // See: https://bulbapedia.bulbagarden.net/wiki/Stat#Generation_I_and_II
    // floor((min(255, floor(sqrt(2500 - 1)) + 1) / 4) * 5 / 100) -> floor((min(255, 49+1)/4) * 5/100) -> floor(12.5*0.05) -> 0
    // floor(((52 + 0) * 2 * 5) / 100) + 5 -> floor(5.2) + 5 -> 10
    // Total = 10
    expect(pokemon._calculateStat(Stat.ATTACK)).toBe(10);
  });
});

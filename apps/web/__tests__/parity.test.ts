import { MAX_MONEY } from '@pokecrystal/core/core/constants';
import { Item } from '@pokecrystal/core/core/enums/item';

import { getBallBonus } from '@pokecrystal/core/engine/battle/battle/item-effects';
import { BattleContext } from '@pokecrystal/core/engine/battle/battle/battle-context';
import { Pokemon, PokemonSpecies as Species } from '@pokecrystal/core/core/models';
import { PlayerGender } from '@pokecrystal/core/core/enums';
import { speciesMap } from '@pokecrystal/core/core/data-loader';

describe('Python/TypeScript Parity', () => {
  beforeAll(async () => {
    // Mock species data
    (speciesMap as Map<string, any>).set('CHIKORITA', {
      id: 'CHIKORITA',
      int_id: 152,
      base_stats: { hp: 45, attack: 49, defense: 49, speed: 45, special_attack: 49, special_defense: 65 },
      type1: 'GRASS',
      type2: 'GRASS',
      catch_rate: 45,
      base_exp: 64,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 20,
      unknown2: 0,
      growth_rate: 'GROWTH_MEDIUM_SLOW',
      egg_group1: 'EGG_MONSTER',
      egg_group2: 'EGG_PLANT',
      tmhm_learnset: [],
      ability: 'OVERGROW',
      pic_size: 0,
      front_pic: 0,
      back_pic: 0,
      evolutions: [],
    });
    (speciesMap as Map<string, any>).set('NIDORINO', {
      id: 'NIDORINO',
      int_id: 111,
      base_stats: { hp: 61, attack: 72, defense: 57, speed: 65, special_attack: 55, special_defense: 55 },
      type1: 'POISON',
      type2: 'POISON',
      catch_rate: 120,
      base_exp: 128,
      gender_ratio: 0,
      unknown1: 0,
      step_cycles_to_hatch: 20,
      unknown2: 0,
      growth_rate: 'GROWTH_MEDIUM_SLOW',
      egg_group1: 'EGG_MONSTER',
      egg_group2: 'EGG_GROUND',
      tmhm_learnset: [],
      ability: 'POISON_POINT',
      pic_size: 0,
      front_pic: 0,
      back_pic: 0,
      evolutions: [],
    });
  });
  it('should have MAX_MONEY defined', () => {
    expect(MAX_MONEY).toBe(999_999);
  });

  it('should have the correct value for BITTER_BERRY', () => {
    expect(Item.BITTER_BERRY).toBe('BITTER_BERRY');
  });

  it('Love Ball bonus applies to same-gender Pokémon', () => {
    const chikorita = speciesMap.get('CHIKORITA')!;
    const playerMon: Pokemon = {
      species: chikorita,
      gender: PlayerGender.MALE,
    } as Pokemon;
    const enemyMon: Pokemon = {
      species: chikorita,
      gender: PlayerGender.MALE,
    } as Pokemon;
    const context = new BattleContext([playerMon], [enemyMon], playerMon, enemyMon, undefined, false, undefined, 0);
    const bonus = getBallBonus('LOVE_BALL', enemyMon, context, null);
    expect(bonus).toBe(8.0);
  });

  it('Moon Ball bonus checks for Burn Heal evolution', () => {
    const nidorinoSpecies = speciesMap.get('NIDORINO')!;
    const enemyMon: Pokemon = { species: nidorinoSpecies } as Pokemon;
    const context = new BattleContext([], [enemyMon], {} as Pokemon, enemyMon, undefined, false, undefined, 0);
    let bonus = getBallBonus('MOON_BALL', enemyMon, context, null);
    expect(bonus).toBe(1.0);

    const mockSpecies: Species = {
      ...nidorinoSpecies,
      evolutions: [{ item: 'BURN_HEAL', level: 0, type: '', target_species: '' }],
      id: 'FAKEMON',
    } as Species;
    enemyMon.species = mockSpecies;
    bonus = getBallBonus('MOON_BALL', enemyMon, context, null);
    expect(bonus).toBe(1.0);
  });
});

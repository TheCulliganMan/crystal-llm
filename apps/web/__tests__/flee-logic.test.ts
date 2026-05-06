import {
  attemptRun
} from '@/engine/battle/battle/flee-logic';
import {
  calculateBattleStat
} from '@/engine/battle/battle/stats';
import {
  Battle
} from '@/engine/battle/battle/battle-logic';
import {
  GenderRatio
} from '@/core/enums/pokemon';
import {
  Pokemon,
  PokemonSchema
} from '@/core/models';
import {
  toPokemon
} from '@/core/models/pokemon';
import {
  BattleContext
} from '@/engine/battle/battle/battle-context';

jest.mock('@/engine/battle/battle/stats', () => ({
  ...jest.requireActual('@/engine/battle/battle/turn-order'),
  calculateBattleStat: jest.fn(),
}));

describe('attemptRun', () => {
  let battle: Battle;
  let player: Pokemon;
  let enemy: Pokemon;

  beforeEach(() => {
    player = toPokemon(
      PokemonSchema.parse({
        species: {
          id: "MEWTWO",
          int_id: 150,
          base_stats: {
            hp: 106,
            attack: 110,
            defense: 90,
            speed: 130,
            special_attack: 154,
            special_defense: 90,
          },
          type1: "PSYCHIC_TYPE",
          type2: "PSYCHIC_TYPE",
          catch_rate: 3,
          base_exp: 220,
          growth_rate: "GROWTH_SLOW",
          gender_ratio: GenderRatio.GENDER_UNKNOWN,
          egg_group1: "EGG_NONE",
          egg_group2: "EGG_NONE",
          unknown1: 0,
          step_cycles_to_hatch: 0,
          unknown2: 0,
        },
        nickname: "PLAYER",
        level: 100,
        hp: 416,
        max_hp: 416,
        original_trainer_name: "PLAYER",
        original_trainer_id: 0,
        experience: 0,
        happiness: 0,
      })
    );
    enemy = toPokemon(
      PokemonSchema.parse({
        species: {
          id: "PIDGEY",
          int_id: 16,
          base_stats: {
            hp: 40,
            attack: 45,
            defense: 40,
            speed: 56,
            special_attack: 35,
            special_defense: 35,
          },
          type1: "NORMAL",
          type2: "FLYING",
          catch_rate: 255,
          base_exp: 55,
          growth_rate: "GROWTH_MEDIUM_SLOW",
          gender_ratio: GenderRatio.GENDER_F50,
          egg_group1: "EGG_FLYING",
          egg_group2: "EGG_FLYING",
          unknown1: 0,
          step_cycles_to_hatch: 0,
          unknown2: 0,
        },
        nickname: "ENEMY",
        level: 100,
        hp: 100,
        max_hp: 100,
        original_trainer_name: "ENEMY",
        original_trainer_id: 0,
        experience: 0,
        happiness: 0,
      })
    );
    const context = new BattleContext([player], [enemy], player, enemy, undefined, false, undefined, 0);
    context.badgeBoostActive = jest.fn().mockReturnValue(false);
    battle = {
      context,
      gameState: {
        hram: {
          hRandomAdd: 0,
          hRandomSub: 0,
        },
      },
      eventManager: {
        dispatch: jest.fn()
      },
    } as unknown as Battle;
    (calculateBattleStat as jest.Mock).mockClear();
  });

  test('should call calculateBattleStat for speed check', () => {
    (calculateBattleStat as jest.Mock).mockReturnValue(100);
    attemptRun(battle);
    expect(calculateBattleStat).toHaveBeenCalledTimes(2);
  });
});

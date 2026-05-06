import { Pokemon, Move as MoveData, PokemonSpecies } from '@pokecrystal/core/core/models';
import { BattleContext } from './battle-context';
import { attackerCannotMove, moveIsDisabled, resolveConfusion, tickDisable } from './status-effects';
import { MoveEffect, MoveName, PokemonType, GrowthRate, EggGroup, GenderRatio, Ability, StatusCondition, Stat } from '@pokecrystal/core/core/enums';
import { EventManager } from '@pokecrystal/core/engine/events/events';
import { GameState } from '@pokecrystal/core/core/state';
import { Battle } from './battle-logic';
import { HardwareRNG } from '@pokecrystal/core/engine/games/rng';
import { movesMap } from '@pokecrystal/core/core/data-loader';

jest.mock('@pokecrystal/core/engine/games/rng');

const mockSpecies: PokemonSpecies = {
    id: "MOCK",
    int_id: 1,
    base_stats: { hp: 100, attack: 100, defense: 100, speed: 100, special_attack: 100, special_defense: 100 },
    type1: PokemonType.NORMAL,
    type2: PokemonType.NONE,
    catch_rate: 255,
    base_exp: 100,
    item1: null,
    item2: null,
    gender_ratio: GenderRatio.GENDER_F50,
    unknown1: 0,
    step_cycles_to_hatch: 20,
    unknown2: 0,
    growth_rate: GrowthRate.GROWTH_MEDIUM_FAST,
    egg_group1: EggGroup.EGG_MONSTER,
    egg_group2: EggGroup.EGG_MONSTER,
    tmhm_learnset: [],
    ability: Ability.NONE,
    pic_size: 0,
    front_pic: 0,
    back_pic: 0,
    evolutions: [],
    weight: 0,
};

const createMockPokemon = (): Pokemon => {
    const pokemon = {
        species: mockSpecies,
        nickname: 'MOCK',
        level: 50,
        hp: 100,
        max_hp: 100,
        stat_boosts: {},
        moves: [],
        dvs: {},
        status: null,
        original_trainer_name: 'TRAINER',
        original_trainer_id: 12345,
        experience: 125000,
        happiness: 70,
        confusion_turns: 0,
    };
    return {
        ...pokemon,
        _calculateStat: (stat: Stat) => 100,
    } as unknown as Pokemon;
};

describe('resolveConfusion', () => {
  let attacker: Pokemon;
  let context: BattleContext;
  let eventManager: EventManager;
  let gameState: GameState;
  let battle: Battle;

  beforeEach(() => {
    attacker = createMockPokemon();
    gameState = {} as GameState;
    context = new BattleContext(
      [attacker],
      [],
      attacker,
      {} as Pokemon,
      undefined,
      false,
      undefined,
      0
    );
    eventManager = new EventManager(gameState);
    battle = { context, eventManager, gameState } as Battle;
    movesMap.set(MoveName.POUND, {
        name: MoveName.POUND,
        power: 40,
        type: PokemonType.NORMAL,
        accuracy: 100,
        pp: 35,
        effect: MoveEffect.NORMAL_HIT,
        effect_chance: 0,
    } as MoveData);
    jest.spyOn(eventManager, 'dispatch');
  });

  it('should clear volatile confusion on the final turn', () => {
    attacker.status = undefined;
    attacker.confusion_turns = 1;
    (HardwareRNG as jest.Mock).mockImplementation(() => ({
        coinFlip: () => false, // Don't self-hit
        nextByte: () => 255,
    }));

    resolveConfusion(battle, attacker);
    expect(attacker.status).toBeUndefined();
    expect(attacker.confusion_turns).toBe(0);
  });

  it('blocks sleeping attackers until their sleep counter reaches zero', () => {
    attacker.status = StatusCondition.SLEEP;
    attacker.sleep_turns = 2;

    expect(attackerCannotMove(battle, attacker)).toBe(true);
    expect(attacker.status).toBe(StatusCondition.SLEEP);
    expect(attacker.sleep_turns).toBe(1);

    expect(attackerCannotMove(battle, attacker)).toBe(false);
    expect(attacker.status).toBeUndefined();
    expect(attacker.sleep_turns).toBe(0);
  });

  it('ticks disabled moves down and clears them when the counter expires', () => {
    attacker.disabled_move = MoveName.BITE;
    attacker.disable_turns = 1;

    expect(moveIsDisabled(attacker, MoveName.BITE)).toBe(true);

    tickDisable(battle, attacker);

    expect(attacker.disable_turns).toBe(0);
    expect(attacker.disabled_move).toBeUndefined();
    expect(moveIsDisabled(attacker, MoveName.BITE)).toBe(false);
    expect(eventManager.dispatch).toHaveBeenCalledWith(
      expect.objectContaining({
        name: "show_text",
        data: { text: `${attacker.nickname}'s disabled no more!` },
      }),
    );
  });

  it('ticks disabled moves before paralysis can block the turn', () => {
    attacker.disabled_move = MoveName.BITE;
    attacker.disable_turns = 1;
    attacker.status = StatusCondition.PARALYSIS;
    (HardwareRNG as jest.Mock).mockImplementation(() => ({
        coinFlip: () => true,
        nextByte: () => 255,
    }));

    expect(attackerCannotMove(battle, attacker)).toBe(true);
    expect(attacker.disabled_move).toBeUndefined();
    expect(attacker.disable_turns).toBe(0);
  });
});

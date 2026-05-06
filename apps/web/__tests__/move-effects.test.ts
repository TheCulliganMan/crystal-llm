import { Pokemon, PokemonSpecies, Move as MoveData, Trainer } from '@/core/models';
import { BattleContext } from '@/engine/battle/battle/battle-context';
import { applyMoveEffect } from '@/engine/battle/battle/move-effects';
import { BattleTurn, MoveEffect, PokemonType, StatusCondition } from '@/core/enums';
import { EventManager } from '@/engine/events/events';
import { createInitialGameState, GameState } from '@/core/state';

describe('applyMoveEffect', () => {
  let attacker: Pokemon;
  let defender: Pokemon;
  let context: BattleContext;
  let eventManager: EventManager;
  let gameState: GameState;
  let move: MoveData;

  beforeEach(() => {
    const species: PokemonSpecies = {
        id: 'TESTMON',
        int_id: 1,
        base_stats: {
            hp: 50,
            attack: 50,
            defense: 50,
            speed: 50,
            special_attack: 50,
            special_defense: 50,
        },
        type1: PokemonType.NORMAL,
        type2: PokemonType.NONE,
        catch_rate: 255,
        base_exp: 100,
        gender_ratio: 0.5,
        unknown1: 0,
        step_cycles_to_hatch: 20,
        unknown2: 0,
        growth_rate: 0,
        egg_group1: 0,
        egg_group2: 0,
    };
    attacker = {
        species: species,
        nickname: 'ATTACKER',
        level: 50,
        hp: 50,
        max_hp: 50,
    } as Pokemon;
    defender = {
        species: species,
        nickname: 'DEFENDER',
        level: 50,
        hp: 50,
        max_hp: 50,
    } as Pokemon;
    gameState = createInitialGameState();
    context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    eventManager = new EventManager(gameState);
    move = {
        effect: MoveEffect.POISON_HIT,
        power: 0,
        type: PokemonType.POISON,
        accuracy: 100,
        pp: 10,
        effect_chance: 100,
        name: 'POISON_GAS',
    } as MoveData;
  });

  it('should not poison a poison-type pokemon', () => {
    defender.species.type1 = PokemonType.POISON;
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState);
    expect(defender.status).toBeUndefined();
  });

  it('should not poison a pokemon protected by safeguard', () => {
    context.setBarrier(BattleTurn.ENEMY, 'safeguard', 5);
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState);
    expect(defender.status).toBeUndefined();
  });
});

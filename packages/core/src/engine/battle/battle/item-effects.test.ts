import { __test__computeFinalCatchRate, applyItemEffect, catchPokemon } from './item-effects';
import { Item, Pokemon, PokemonSpecies, toPokemon, PokemonSchema } from '../../../core/models';
import { BattleContext } from './battle-context';
import { EventManager } from '../../events/events';
import { GameState, createInitialGameState } from '../../../core/state';
import { MAX_BOX_MONS, MAX_PC_BOXES, PARTY_SIZE } from '../../../core/constants';
import { addPokemon as addBoxPokemon, BoxSchema, formatDefaultBoxName } from '../../../core/models/box';
import { Stat, PokemonType, GrowthRate, EggGroup, GenderRatio, ItemEffect, Ability, StatusCondition, ItemPocket, BattleScene } from '../../../core/enums';

const captureBattleTexts = (eventManager: EventManager): string[] => {
    const texts: string[] = [];
    eventManager.on('show_text', (event) => {
        const payload = event.data as { text?: string };
        if (typeof payload.text === 'string') {
            texts.push(payload.text);
        }
    });
    return texts;
};

describe('applyItemEffect', () => {
    let pokemon: Pokemon;
    let context: BattleContext;
    let eventManager: EventManager;
    let gameState: GameState;

    beforeEach(() => {
        const species: PokemonSpecies = {
            id: 'CHARMANDER',
            int_id: 4,
            base_stats: { hp: 39, attack: 52, defense: 43, speed: 65, special_attack: 60, special_defense: 50 },
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
            evolutions: null,
            ability: Ability.NONE,
            pic_size: 0,
            front_pic: 0,
            back_pic: 0,
        } as any;
        pokemon = toPokemon(PokemonSchema.parse({
            species,
            nickname: 'CHARMANDER',
            level: 5,
            hp: 20,
            max_hp: 20,
            original_trainer_name: 'PLAYER',
            original_trainer_id: 1,
            experience: 125,
            happiness: 70,
            moves: [],
            stat_boosts: {
                HP: 0,
                ATTACK: 0,
                DEFENSE: 0,
                SPEED: 0,
                SPECIAL_ATTACK: 0,
                SPECIAL_DEFENSE: 0,
                ACCURACY: 0,
                EVASION: 0,
            },
        }));
        context = new BattleContext(
            [pokemon],
            [pokemon],
            pokemon,
            pokemon,
            undefined,
            false,
            undefined,
            0
        );
        gameState = createInitialGameState();
        eventManager = new EventManager(gameState);
    });

  it('should boost Special Attack for X SPECIAL', () => {
    const xSpecial: Item = {
      script_name: 'X_SPECIAL',
      name: 'X SPECIAL',
      description: 'Boosts Special stats during battle.',
      effect: ItemEffect.X_ITEM,
      parameter: 0,
      price: 0,
      pocket: 0,
    } as any;

    applyItemEffect(xSpecial, pokemon, eventManager, context, gameState, null as any);

    expect(pokemon.stat_boosts[Stat.SPECIAL_ATTACK]).toBe(1);
    expect(pokemon.stat_boosts[Stat.SPECIAL_DEFENSE]).toBe(0);
  });

  it('should boost Accuracy for X ACCURACY instead of falling back to unsupported-effect handling', () => {
    const warn = jest.spyOn(console, 'warn').mockImplementation(() => {});
    const xAccuracy: Item = {
      script_name: 'X_ACCURACY',
      name: 'X ACCURACY',
      description: 'Raises accuracy during one battle.',
      effect: ItemEffect.X_ACCURACY,
      parameter: 0,
      price: 0,
      pocket: ItemPocket.ITEM,
    } as any;

    const texts = captureBattleTexts(eventManager);

    applyItemEffect(xAccuracy, pokemon, eventManager, context, gameState, null as any);

    expect(pokemon.stat_boosts[Stat.ACCURACY]).toBe(1);
    expect(texts).toContain("CHARMANDER's ACCURACY rose!");
    expect(warn).not.toHaveBeenCalled();
  });

  it('heals confusion for BITTER BERRY instead of falling back to unsupported-effect handling', () => {
    const warn = jest.spyOn(console, 'warn').mockImplementation(() => {});
    const bitterBerry: Item = {
      script_name: 'BITTER_BERRY',
      name: 'BITTER BERRY',
      description: 'A self-restore item that heals confusion.',
      effect: ItemEffect.BITTER_BERRY,
      parameter: 0,
      price: 0,
      pocket: ItemPocket.ITEM,
    } as any;

    pokemon.confusion_turns = 3;
    const texts = captureBattleTexts(eventManager);

    applyItemEffect(bitterBerry, pokemon, eventManager, context, gameState, null as any);

    expect(pokemon.confusion_turns).toBe(0);
    expect(texts).not.toContain("It won't have any effect.");
    expect(warn).not.toHaveBeenCalled();
  });

  it('should not boost Special Attack if it is already maxed out', () => {
    const xSpecial: Item = {
      script_name: 'X_SPECIAL',
      name: 'X SPECIAL',
      description: 'Boosts Special Attack during battle.',
      effect: ItemEffect.X_ITEM,
      parameter: 0,
      price: 0,
      pocket: 0,
    } as any;
    pokemon.stat_boosts[Stat.SPECIAL_ATTACK] = 6;

    applyItemEffect(xSpecial, pokemon, eventManager, context, gameState, null as any);

    expect(pokemon.stat_boosts[Stat.SPECIAL_ATTACK]).toBe(6);
    expect(pokemon.stat_boosts[Stat.SPECIAL_DEFENSE]).toBe(0);
  });

  it("shows the no-effect text when a status heal item is used on the wrong status", () => {
    const antidote: Item = {
      script_name: 'ANTIDOTE',
      name: 'Antidote',
      description: 'Heals poison.',
      effect: ItemEffect.STATUS_HEAL,
      parameter: 0,
      price: 0,
      pocket: 0,
    } as any;
    pokemon.status = StatusCondition.BURN;

    const texts = captureBattleTexts(eventManager);

    applyItemEffect(antidote, pokemon, eventManager, context, gameState, null as any);

    expect(texts).toContain("It won't have any effect.");
    expect(pokemon.status).toBe(StatusCondition.BURN);
  });

  it("shows the no-effect text when a healing item is used at full HP", () => {
    const potion: Item = {
      script_name: 'POTION',
      name: 'Potion',
      description: 'Restores 20 HP.',
      effect: ItemEffect.RESTORE_HP,
      parameter: 20,
      price: 0,
      pocket: 0,
    } as any;
    pokemon.hp = pokemon.max_hp;

    const texts = captureBattleTexts(eventManager);

    applyItemEffect(potion, pokemon, eventManager, context, gameState, null as any);

    expect(texts).toContain("It won't have any effect.");
    expect(pokemon.hp).toBe(pokemon.max_hp);
  });

  it("shows the no-effect text when a revive item is used on a healthy Pokemon", () => {
    const revive: Item = {
      script_name: 'REVIVE',
      name: 'Revive',
      description: 'Revives a fainted Pokemon.',
      effect: ItemEffect.REVIVE,
      parameter: 0,
      price: 0,
      pocket: 0,
    } as any;
    pokemon.hp = Math.max(1, pokemon.hp);

    const texts = captureBattleTexts(eventManager);

    applyItemEffect(revive, pokemon, eventManager, context, gameState, null as any);

    expect(texts).toContain("It won't have any effect.");
    expect(pokemon.hp).toBeGreaterThan(0);
  });

  it("throws instead of silently degrading unsupported battle item effects", () => {
    const mysteryBattleItem: Item = {
      script_name: 'MYSTERY_BATTLE_ITEM',
      name: 'MYSTERY_BATTLE_ITEM',
      description: 'Unimplemented battle effect.',
      effect: ItemEffect.REPEL,
      parameter: 0,
      price: 0,
      pocket: ItemPocket.ITEM,
    } as any;

    expect(() =>
      applyItemEffect(mysteryBattleItem, pokemon, eventManager, context, gameState, null as any),
    ).toThrow('Unsupported battle item effect REPEL for MYSTERY_BATTLE_ITEM.');
  });

  it("shows the no-effect text when FULL RESTORE is used with nothing to heal", () => {
    const fullRestore: Item = {
      script_name: 'FULL_RESTORE',
      name: 'Full Restore',
      description: 'Fully restores HP and status.',
      effect: ItemEffect.FULL_RESTORE,
      parameter: 0,
      price: 0,
      pocket: 0,
    } as any;
    pokemon.hp = pokemon.max_hp;
    pokemon.status = undefined;
    pokemon.sleep_turns = 0;
    pokemon.confusion_turns = 0;

    const texts = captureBattleTexts(eventManager);

    applyItemEffect(fullRestore, pokemon, eventManager, context, gameState, null as any);

    expect(texts).toContain("It won't have any effect.");
    expect(pokemon.hp).toBe(pokemon.max_hp);
  });

  it('preserves captured HP and status on a successful catch', () => {
    const masterBall: Item = {
      script_name: 'MASTER_BALL',
      name: 'MASTER BALL',
      description: 'The best BALL with the ultimate level of performance.',
      effect: ItemEffect.POKE_BALL,
      parameter: 0,
      price: 0,
      pocket: ItemPocket.BALL,
    } as any;
    const enemy = toPokemon(PokemonSchema.parse({
      species: pokemon.species,
      nickname: 'WILD',
      level: 5,
      hp: 7,
      max_hp: 20,
      original_trainer_name: 'WILD',
      original_trainer_id: 2,
      experience: 125,
      happiness: 70,
      moves: [],
      stat_boosts: {
        HP: 0,
        ATTACK: 0,
        DEFENSE: 0,
        SPEED: 0,
        SPECIAL_ATTACK: 0,
        SPECIAL_DEFENSE: 0,
        ACCURACY: 0,
        EVASION: 0,
      },
      status: StatusCondition.POISON,
    }));
    const localContext = new BattleContext(
      [pokemon],
      [enemy],
      pokemon,
      enemy,
      undefined,
      false,
      undefined,
      0,
    );
    const expectedHp = enemy.hp;
    const expectedStatus = enemy.status;

    const result = catchPokemon(masterBall, eventManager, localContext, gameState);

    expect(result).toBe(true);
    expect(enemy.hp).toBe(expectedHp);
    const captured = gameState.sram.party?.pokemon[0];
    expect(captured?.hp).toBe(expectedHp);
    expect(captured?.status).toBe(expectedStatus);
  });

  it('blocks ball use instead of throwing when party and PC storage are full', () => {
    const masterBall: Item = {
      script_name: 'MASTER_BALL',
      name: 'MASTER BALL',
      description: 'The best BALL with the ultimate level of performance.',
      effect: ItemEffect.POKE_BALL,
      parameter: 0,
      price: 0,
      pocket: ItemPocket.BALL,
    } as any;
    gameState.sram.party.pokemon = Array(PARTY_SIZE).fill(pokemon);
    gameState.sram.pc_boxes = Array.from({ length: MAX_PC_BOXES }, (_, index) => {
      const box = BoxSchema.parse({ name: formatDefaultBoxName(index) });
      for (let slot = 0; slot < MAX_BOX_MONS; slot += 1) {
        addBoxPokemon(box, pokemon);
      }
      return box;
    });
    const texts = captureBattleTexts(eventManager);

    let result = true;
    expect(() => {
      result = catchPokemon(masterBall, eventManager, context, gameState);
    }).not.toThrow();
    expect(result).toBe(false);
    expect(texts).toContain('The BOX is full!');
  });

  it('omits the throw text during tutorial catches', () => {
    const pokeBall: Item = {
      script_name: 'POKE_BALL',
      name: 'POKE BALL',
      description: 'A BALL for catching wild POKEMON.',
      effect: ItemEffect.POKE_BALL,
      parameter: 0,
      price: 200,
      pocket: ItemPocket.BALL,
    } as any;
    gameState.wram.battle_type = 'BATTLETYPE_TUTORIAL';

    const texts = captureBattleTexts(eventManager);

    const result = catchPokemon(pokeBall, eventManager, context, gameState);

    expect(result).toBe(true);
    expect(texts.some((text) => text.toLowerCase().includes('threw'))).toBe(false);
    expect(texts.some((text) => text.includes('Gotcha!'))).toBe(true);
  });

  it('does not queue Poke Ball throw animation in instant mode even when Battle Scene is on', () => {
    const pokeBall: Item = {
      script_name: 'POKE_BALL',
      name: 'POKE BALL',
      description: 'A BALL for catching wild POKEMON.',
      effect: ItemEffect.POKE_BALL,
      parameter: 0,
      price: 200,
      pocket: ItemPocket.BALL,
    } as any;
    gameState.wram.instant_mode = true;
    gameState.sram.options.battle_scene = BattleScene.ON;
    const animationSpy = jest.fn();
    eventManager.on('play_animation', animationSpy);

    catchPokemon(pokeBall, eventManager, context, gameState);

    expect(animationSpy).not.toHaveBeenCalled();
  });

  it('calculates Gen 2 catch rate with HP and sleep bonus', () => {
    const species: PokemonSpecies = {
      ...pokemon.species,
      catch_rate: 100,
    };
    const enemy = toPokemon(PokemonSchema.parse({
      species,
      nickname: 'WILD',
      level: 5,
      hp: 10,
      max_hp: 20,
      original_trainer_name: 'WILD',
      original_trainer_id: 2,
      experience: 125,
      happiness: 70,
      moves: [],
      stat_boosts: {
        HP: 0,
        ATTACK: 0,
        DEFENSE: 0,
        SPEED: 0,
        SPECIAL_ATTACK: 0,
        SPECIAL_DEFENSE: 0,
        ACCURACY: 0,
        EVASION: 0,
      },
      status: StatusCondition.SLEEP,
    }));
    const localContext = new BattleContext(
      [pokemon],
      [enemy],
      pokemon,
      enemy,
      undefined,
      false,
      undefined,
      0,
    );

    const rate = __test__computeFinalCatchRate('POKE_BALL', enemy, localContext, gameState);

    expect(rate).toBe(76);
  });

  it('level ball ignores HP and status when computing catch rate', () => {
    const species: PokemonSpecies = {
      ...pokemon.species,
      catch_rate: 100,
    };
    const enemy = toPokemon(PokemonSchema.parse({
      species,
      nickname: 'WILD',
      level: 10,
      hp: 1,
      max_hp: 20,
      original_trainer_name: 'WILD',
      original_trainer_id: 2,
      experience: 125,
      happiness: 70,
      moves: [],
      stat_boosts: {
        HP: 0,
        ATTACK: 0,
        DEFENSE: 0,
        SPEED: 0,
        SPECIAL_ATTACK: 0,
        SPECIAL_DEFENSE: 0,
        ACCURACY: 0,
        EVASION: 0,
      },
      status: StatusCondition.SLEEP,
    }));
    const player = toPokemon(PokemonSchema.parse({
      species,
      nickname: 'PLAYER',
      level: 21,
      hp: 20,
      max_hp: 20,
      original_trainer_name: 'PLAYER',
      original_trainer_id: 1,
      experience: 125,
      happiness: 70,
      moves: [],
      stat_boosts: {
        HP: 0,
        ATTACK: 0,
        DEFENSE: 0,
        SPEED: 0,
        SPECIAL_ATTACK: 0,
        SPECIAL_DEFENSE: 0,
        ACCURACY: 0,
        EVASION: 0,
      },
    }));
    const localContext = new BattleContext(
      [player],
      [enemy],
      player,
      enemy,
      undefined,
      false,
      undefined,
      0,
    );

    const rateLowHp = __test__computeFinalCatchRate('LEVEL_BALL', enemy, localContext, gameState);
    enemy.hp = 20;
    const rateHighHp = __test__computeFinalCatchRate('LEVEL_BALL', enemy, localContext, gameState);

    expect(rateLowHp).toBe(200);
    expect(rateHighHp).toBe(200);
  });

  it('moon ball keeps the Crystal no-boost bug for pokemon that evolve via moon stone', () => {
    const species: PokemonSpecies = {
      ...pokemon.species,
      catch_rate: 45,
      evolutions: [
        { item: 'MOON_STONE', target: 'NIDOKING' }
      ]
    };
    const enemy = toPokemon(PokemonSchema.parse({
      species,
      nickname: 'WILD',
      level: 10,
      hp: 1,
      max_hp: 20,
      original_trainer_name: 'WILD',
      original_trainer_id: 2,
      experience: 125,
      happiness: 70,
      moves: [],
      stat_boosts: {
        HP: 0,
        ATTACK: 0,
        DEFENSE: 0,
        SPEED: 0,
        SPECIAL_ATTACK: 0,
        SPECIAL_DEFENSE: 0,
        ACCURACY: 0,
        EVASION: 0,
      },
    }));
    const player = toPokemon(PokemonSchema.parse({
      species,
      nickname: 'PLAYER',
      level: 21,
      hp: 20,
      max_hp: 20,
      original_trainer_name: 'PLAYER',
      original_trainer_id: 1,
      experience: 125,
      happiness: 70,
      moves: [],
      stat_boosts: {
        HP: 0,
        ATTACK: 0,
        DEFENSE: 0,
        SPEED: 0,
        SPECIAL_ATTACK: 0,
        SPECIAL_DEFENSE: 0,
        ACCURACY: 0,
        EVASION: 0,
      },
    }));

    const localContext = new BattleContext(
      [player],
      [enemy],
      player,
      enemy,
      undefined,
      false,
      undefined,
      0,
    );

    const normalRate = __test__computeFinalCatchRate('POKE_BALL', enemy, localContext, gameState);
    const moonBallRate = __test__computeFinalCatchRate('MOON_BALL', enemy, localContext, gameState);

    expect(normalRate).toBe(43);
    expect(moonBallRate).toBe(normalRate);
  });

  it('fast ball only boosts Magnemite, Grimer, and Tangela like Crystal', () => {
    const makeEnemy = (id: string) => toPokemon(PokemonSchema.parse({
      species: { ...pokemon.species, id, catch_rate: 45 },
      nickname: id,
      level: 10,
      hp: 1,
      max_hp: 20,
      original_trainer_name: 'WILD',
      original_trainer_id: 2,
      experience: 125,
      happiness: 70,
      moves: [],
      stat_boosts: {
        HP: 0,
        ATTACK: 0,
        DEFENSE: 0,
        SPEED: 0,
        SPECIAL_ATTACK: 0,
        SPECIAL_DEFENSE: 0,
        ACCURACY: 0,
        EVASION: 0,
      },
    }));

    const magnemite = makeEnemy('MAGNEMITE');
    const raikou = makeEnemy('RAIKOU');
    const magnemiteContext = new BattleContext([pokemon], [magnemite], pokemon, magnemite, undefined, false, undefined, 0);
    const raikouContext = new BattleContext([pokemon], [raikou], pokemon, raikou, undefined, false, undefined, 0);

    expect(__test__computeFinalCatchRate('FAST_BALL', magnemite, magnemiteContext, gameState)).toBe(174);
    expect(__test__computeFinalCatchRate('FAST_BALL', raikou, raikouContext, gameState)).toBe(43);
  });

  it('heavy ball applies Crystal junk-weight bonus to Kadabra, Tauros, and Sunflora', () => {
    const kadabra = toPokemon(PokemonSchema.parse({
      species: { ...pokemon.species, id: 'KADABRA', int_id: 64, catch_rate: 100, weight: 1250 },
      nickname: 'KADABRA',
      level: 10,
      hp: 20,
      max_hp: 20,
      original_trainer_name: 'WILD',
      original_trainer_id: 2,
      experience: 125,
      happiness: 70,
      moves: [],
      stat_boosts: {
        HP: 0,
        ATTACK: 0,
        DEFENSE: 0,
        SPEED: 0,
        SPECIAL_ATTACK: 0,
        SPECIAL_DEFENSE: 0,
        ACCURACY: 0,
        EVASION: 0,
      },
    }));
    const localContext = new BattleContext([pokemon], [kadabra], pokemon, kadabra, undefined, false, undefined, 0);

    expect(__test__computeFinalCatchRate('HEAVY_BALL', kadabra, localContext, gameState)).toBe(46);
  });
});

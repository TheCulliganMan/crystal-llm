import { Pokemon, Move as MoveData, PokemonSpecies, toPokemon, PokemonSchema } from '@pokecrystal/core/core/models';
import { BattleContext } from './battle-context';
import { applyMoveEffect } from './move-effects';
import { BattleTurn, MoveEffect, MoveName, Stat, PokemonType, GrowthRate, EggGroup, GenderRatio, Ability, StatusCondition, PlayerGender } from '@pokecrystal/core/core/enums';
import { EventManager } from '@pokecrystal/core/engine/events/events';
import { GameState } from '@pokecrystal/core/core/state';
import { Battle } from './battle-logic';

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
    evolutions: null,
} as any;


const createMockPokemon = (hp: number, max_hp: number): Pokemon => {
    return toPokemon(PokemonSchema.parse({
        species: mockSpecies,
        nickname: 'MOCK',
        level: 50,
        hp,
        max_hp,
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
        moves: [],
        dvs: { attack: 0, defense: 0, speed: 0, special: 0, hp: 0 },
        status: undefined,
        original_trainer_name: 'TRAINER',
        original_trainer_id: 12345,
        experience: 125000,
        happiness: 70,
    }));
};


describe('applyMoveEffect', () => {
  let attacker: Pokemon;
  let defender: Pokemon;
  let context: BattleContext;
  let eventManager: EventManager;
  let gameState: GameState;
  let battle: Battle;
  let move: MoveData;

  beforeEach(() => {
    attacker = createMockPokemon(100, 100);
    defender = createMockPokemon(100, 100);
    gameState = { hram: { hRandomAdd: 0, hRandomSub: 0 } } as GameState;
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
    battle = {} as Battle;

    jest.spyOn(eventManager, 'dispatch');
  });

  const captureBattleTexts = (): string[] => {
    const texts: string[] = [];
    eventManager.on('show_text', (event) => {
      const payload = event.data as { text?: string };
      if (typeof payload?.text === 'string') {
        texts.push(payload.text);
      }
    });
    return texts;
  };

  it('should cut HP and maximize attack when Belly Drum is used with sufficient HP', () => {
    move = {
        name: 'BELLY_DRUM' as any,
        effect: MoveEffect.BELLY_DRUM,
        power: 0,
        type: PokemonType.NORMAL,
        accuracy: 100,
        pp: 10,
        effect_chance: 0,
    };
    const damage = 0;
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(attacker.hp).toBe(50);
    expect(attacker.stat_boosts[Stat.ATTACK]).toBe(6);
  });

  it('should maximize attack when Belly Drum is used with exactly half HP', () => {
    attacker.hp = 50;
    move = {
        name: 'BELLY_DRUM' as any,
        effect: MoveEffect.BELLY_DRUM,
        power: 0,
        type: PokemonType.NORMAL,
        accuracy: 100,
        pp: 10,
        effect_chance: 0,
    };
    const damage = 0;
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(attacker.hp).toBe(0);
    expect(attacker.stat_boosts[Stat.ATTACK]).toBe(6);
  });

  it('should only sharply boost attack before failing when Belly Drum is used below half HP', () => {
    attacker.hp = 49;
    move = {
        name: 'BELLY_DRUM' as any,
        effect: MoveEffect.BELLY_DRUM,
        power: 0,
        type: PokemonType.NORMAL,
        accuracy: 100,
        pp: 10,
        effect_chance: 0,
    };
    const texts = captureBattleTexts();
    const damage = 0;
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(attacker.hp).toBe(49);
    expect(attacker.stat_boosts[Stat.ATTACK]).toBe(2);
    expect(texts).toEqual(
        expect.arrayContaining(["MOCK's attack rose sharply!", "But it failed!"])
    );
  });

  it('uses one shared consecutive-use counter for Protect and Endure', () => {
    move = {
      name: MoveName.PROTECT,
      effect: MoveEffect.PROTECT,
      power: 0,
      type: PokemonType.NORMAL,
      accuracy: 100,
      pp: 10,
      effect_chance: 0,
    };
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);
    expect(attacker.protect_active).toBe(true);
    expect(attacker.protect_counter).toBe(1);

    move = {
      name: MoveName.ENDURE,
      effect: MoveEffect.ENDURE,
      power: 0,
      type: PokemonType.NORMAL,
      accuracy: 100,
      pp: 10,
      effect_chance: 0,
    };
    const texts = captureBattleTexts();
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);

    expect(attacker.endure_active).toBe(false);
    expect(attacker.protect_counter).toBe(0);
    expect(attacker.endure_counter).toBe(0);
    expect(texts).toContain("But it failed!");
  });

  it('fails Protect if the opponent already moved or the user has a substitute', () => {
    move = {
      name: MoveName.PROTECT,
      effect: MoveEffect.PROTECT,
      power: 0,
      type: PokemonType.NORMAL,
      accuracy: 100,
      pp: 10,
      effect_chance: 0,
    };
    context.turnOrder = [BattleTurn.ENEMY, BattleTurn.PLAYER];
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);
    expect(attacker.protect_active).toBe(false);

    context.turnOrder = [];
    attacker.substitute_hp = 1;
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);
    expect(attacker.protect_active).toBe(false);
  });

  it('allows Ghost Curse at half HP or less and can reduce the user to 0 HP', () => {
    attacker.species = { ...attacker.species, type1: PokemonType.GHOST };
    attacker.hp = 25;
    move = {
      name: MoveName.CURSE,
      effect: MoveEffect.CURSE,
      power: 0,
      type: PokemonType.GHOST,
      accuracy: 100,
      pp: 10,
      effect_chance: 0,
    };
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);

    expect(attacker.hp).toBe(0);
    expect(defender.cursed).toBe(true);
  });

  it('fails non-Ghost Curse without lowering Speed when Attack and Defense cannot rise', () => {
    attacker.stat_boosts[Stat.ATTACK] = 6;
    attacker.stat_boosts[Stat.DEFENSE] = 6;
    attacker.stat_boosts[Stat.SPEED] = 0;
    move = {
      name: MoveName.CURSE,
      effect: MoveEffect.CURSE,
      power: 0,
      type: PokemonType.GHOST,
      accuracy: 100,
      pp: 10,
      effect_chance: 0,
    };
    const texts = captureBattleTexts();
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);

    expect(attacker.stat_boosts[Stat.SPEED]).toBe(0);
    expect(texts).toContain("But it failed!");
  });

  it('should increase attack by 1 when ATTACK_UP is applied', () => {
    move = {
        name: 'HONE_CLAWS' as any,
        effect: MoveEffect.ATTACK_UP,
        power: 0,
        type: PokemonType.NORMAL,
        accuracy: 100,
        pp: 15,
        effect_chance: 0,
    };
    const damage = 0;
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(attacker.stat_boosts[Stat.ATTACK]).toBe(1);
    });

  it('should increase attack by 2 when ATTACK_UP_2 is applied', () => {
    move = {
        name: 'SWORDS_DANCE' as any,
        effect: MoveEffect.ATTACK_UP_2,
        power: 0,
        type: PokemonType.NORMAL,
        accuracy: 100,
        pp: 30,
        effect_chance: 0,
    };
    const damage = 0;
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(attacker.stat_boosts[Stat.ATTACK]).toBe(2);
    });

  it('should decrease attack by 1 when ATTACK_DOWN is applied', () => {
    move = {
        name: 'GROWL' as any,
        effect: MoveEffect.ATTACK_DOWN,
        power: 0,
        type: PokemonType.NORMAL,
        accuracy: 100,
        pp: 40,
        effect_chance: 0,
    };
    const damage = 0;
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(defender.stat_boosts[Stat.ATTACK]).toBe(-1);
    });

  it('should decrease attack by 2 when ATTACK_DOWN_2 is applied', () => {
    move = {
        name: 'SCREECH' as any,
        effect: MoveEffect.ATTACK_DOWN_2,
        power: 0,
        type: PokemonType.NORMAL,
        accuracy: 85,
        pp: 40,
        effect_chance: 0,
    };
    const damage = 0;
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(defender.stat_boosts[Stat.ATTACK]).toBe(-2);
    });

  it('should set attacker hp to 0 when SELFDESTRUCT is used', () => {
    move = {
        name: 'SELFDESTRUCT' as any,
        effect: MoveEffect.SELFDESTRUCT,
        power: 200,
        type: PokemonType.NORMAL,
        accuracy: 100,
        pp: 5,
        effect_chance: 0,
    };
    const damage = 0;
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(attacker.hp).toBe(0);
    });

  it('should transform into the defender and copy ASM battle fields', () => {
    const dittoSpecies = { ...mockSpecies, id: "DITTO" } as PokemonSpecies;
    const mewSpecies = {
      ...mockSpecies,
      id: "MEW",
      type1: PokemonType.PSYCHIC_TYPE,
      base_stats: {
        hp: 100,
        attack: 100,
        defense: 100,
        speed: 100,
        special_attack: 100,
        special_defense: 100,
      },
    } as PokemonSpecies;
    attacker = toPokemon(PokemonSchema.parse({
      ...attacker,
      species: dittoSpecies,
      nickname: "DITTO",
      moves: [{ name: MoveName.TRANSFORM, current_pp: 10 }],
    }));
    defender = toPokemon(PokemonSchema.parse({
      ...defender,
      species: mewSpecies,
      nickname: "MEW",
      dvs: { attack: 15, defense: 14, speed: 13, special: 12, hp: 11 },
      attack: 120,
      defense: 121,
      speed: 122,
      special_attack: 123,
      special_defense: 124,
      stat_boosts: {
        HP: 0,
        ATTACK: 2,
        DEFENSE: 1,
        SPEED: -1,
        SPECIAL_ATTACK: 3,
        SPECIAL_DEFENSE: -2,
        ACCURACY: 1,
        EVASION: -1,
      },
      moves: [
        { name: MoveName.PSYCHIC_M, current_pp: 16 },
        { name: MoveName.SKETCH, current_pp: 1 },
      ],
    }));
    context = new BattleContext([attacker], [defender], attacker, defender, undefined, false, undefined, 0);
    move = {
      name: MoveName.TRANSFORM,
      effect: MoveEffect.TRANSFORM,
      power: 0,
      type: PokemonType.NORMAL,
      accuracy: 100,
      pp: 10,
      effect_chance: 0,
    };

    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);

    expect(attacker.transformed).toBe(true);
    expect(attacker.original_species?.id).toBe("DITTO");
    expect(attacker.species.id).toBe("MEW");
    expect(attacker.moves).toEqual([
      { name: MoveName.PSYCHIC_M, current_pp: 5 },
      { name: MoveName.SKETCH, current_pp: 1 },
    ]);
    expect(attacker.dvs).toEqual(defender.dvs);
    expect(attacker.attack).toBe(120);
    expect(attacker.special_defense).toBe(124);
    expect(attacker.stat_boosts).toEqual(defender.stat_boosts);
    expect(attacker.transform_backup_moves).toEqual([{ name: MoveName.TRANSFORM, current_pp: 10 }]);
  });

  it('should fail when attempting to transform into an already transformed target', () => {
    defender.transformed = true;
    move = {
      name: MoveName.TRANSFORM,
      effect: MoveEffect.TRANSFORM,
      power: 0,
      type: PokemonType.NORMAL,
      accuracy: 100,
      pp: 10,
      effect_chance: 0,
    };
    const texts = captureBattleTexts();

    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);

    expect(attacker.transformed).toBe(false);
    expect(texts).toContain("But it failed!");
  });

  it('should set weather for Rain Dance and weather-heal with Morning Sun variants', () => {
    move = {
      name: 'RAIN_DANCE' as any,
      effect: MoveEffect.RAIN_DANCE,
      power: 0,
      type: PokemonType.WATER,
      accuracy: 100,
      pp: 5,
      effect_chance: 0,
    };
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);
    expect(context.weather).toBeDefined();
    attacker.hp = 40;
    move = {
      name: 'MORNING_SUN' as any,
      effect: MoveEffect.MORNING_SUN,
      power: 0,
      type: PokemonType.NORMAL,
      accuracy: 100,
      pp: 5,
      effect_chance: 0,
    };
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);
    expect(attacker.hp).toBe(65);
  });

  it('should place spikes and rapid spin them away', () => {
    move = {
      name: 'SPIKES' as any,
      effect: MoveEffect.SPIKES,
      power: 0,
      type: PokemonType.GROUND,
      accuracy: 100,
      pp: 20,
      effect_chance: 0,
    };
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);
    expect(context.enemySpikesLayers).toBe(1);

    attacker.leech_seeded = true;
    attacker.trapped_turns = 3;
    context.playerSpikesLayers = 1;
    move = {
      name: 'RAPID_SPIN' as any,
      effect: MoveEffect.RAPID_SPIN,
      power: 20,
      type: PokemonType.NORMAL,
      accuracy: 100,
      pp: 40,
      effect_chance: 0,
    };
    applyMoveEffect(attacker, defender, move, 10, context, eventManager, gameState, battle);
    expect(attacker.leech_seeded).toBe(false);
    expect(attacker.trapped_turns).toBe(0);
    expect(context.playerSpikesLayers).toBe(0);
  });

  it('should apply Future Sight and Perish Song counters', () => {
    battle = { _actionCounter: 0 } as Battle;
    move = {
      name: 'FUTURE_SIGHT' as any,
      effect: MoveEffect.FUTURE_SIGHT,
      power: 80,
      type: PokemonType.PSYCHIC_TYPE,
      accuracy: 100,
      pp: 10,
      effect_chance: 0,
    };
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);
    expect(context.enemyFutureSightCounter).toBe(3);
    expect(context.enemyFutureSightDamage).toBeGreaterThan(0);

    move = {
      name: 'PERISH_SONG' as any,
      effect: MoveEffect.PERISH_SONG,
      power: 0,
      type: PokemonType.NORMAL,
      accuracy: 100,
      pp: 5,
      effect_chance: 0,
    };
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);
    expect(attacker.perish_song_turns).toBe(4);
    expect(defender.perish_song_turns).toBe(4);
  });

  it('should apply attract and heal bell party cleansing', () => {
    defender.status = StatusCondition.BURN;
    defender.confusion_turns = 3;
    attacker.gender = PlayerGender.MALE;
    defender.gender = PlayerGender.FEMALE;
    move = {
      name: 'ATTRACT' as any,
      effect: MoveEffect.ATTRACT,
      power: 0,
      type: PokemonType.NORMAL,
      accuracy: 100,
      pp: 15,
      effect_chance: 0,
    };
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);
    expect(defender.attract_source_side).toBe(BattleTurn.PLAYER);

    move = {
      name: 'HEAL_BELL' as any,
      effect: MoveEffect.HEAL_BELL,
      power: 0,
      type: PokemonType.NORMAL,
      accuracy: 100,
      pp: 5,
      effect_chance: 0,
    };
    applyMoveEffect(defender, attacker, move, 0, context, eventManager, gameState, battle);
    expect(defender.status).toBeUndefined();
    expect(defender.confusion_turns).toBe(0);
  });

  it('should fail Attract against same-gender or genderless targets', () => {
    move = {
      name: 'ATTRACT' as any,
      effect: MoveEffect.ATTRACT,
      power: 0,
      type: PokemonType.NORMAL,
      accuracy: 100,
      pp: 15,
      effect_chance: 0,
    };

    attacker.gender = PlayerGender.MALE;
    defender.gender = PlayerGender.MALE;
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);
    expect(defender.attract_source_side).toBeUndefined();

    defender.gender = undefined;
    defender.species = { ...defender.species, gender_ratio: GenderRatio.GENDER_UNKNOWN };
    applyMoveEffect(attacker, defender, move, 0, context, eventManager, gameState, battle);
    expect(defender.attract_source_side).toBeUndefined();
  });

  it('should make the defender flinch when FLINCH_HIT is applied', () => {
    move = {
        name: 'STOMP' as any,
        effect: MoveEffect.FLINCH_HIT,
        power: 65,
        type: PokemonType.NORMAL,
        accuracy: 100,
        pp: 20,
        effect_chance: 30,
    };
    const damage = 20;
    context.predefinedRandomValue = 0.2; // Ensure the effect triggers
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(defender.flinching).toBe(true);
  });

  it('should apply recoil damage to the attacker when RECOIL_HIT is used', () => {
    attacker.hp = 100;
    move = {
        name: 'TAKE_DOWN' as any,
        effect: MoveEffect.RECOIL_HIT,
        power: 90,
        type: PokemonType.NORMAL,
        accuracy: 85,
        pp: 20,
        effect_chance: 0,
    };
    const damage = 40;
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(attacker.hp).toBe(90); // 100 - (40 / 4)
  });

  it('should seed the defender when LEECH_SEED is used', () => {
    move = {
        name: 'LEECH_SEED' as any,
        effect: MoveEffect.LEECH_SEED,
        power: 0,
        type: PokemonType.GRASS,
        accuracy: 90,
        pp: 10,
        effect_chance: 0,
    };
    const damage = 0;
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(defender.leech_seeded).toBe(true);
    expect(defender.leech_seed_source_side).toBe(context.sideFor(attacker));
    expect(eventManager.dispatch).toHaveBeenCalledWith(expect.objectContaining({
      data: { text: `${defender.nickname} was seeded!` }
    }));
  });

  it('should fail to seed a Grass-type defender', () => {
    defender.species.type1 = PokemonType.GRASS;
    move = {
        name: 'LEECH_SEED' as any,
        effect: MoveEffect.LEECH_SEED,
        power: 0,
        type: PokemonType.GRASS,
        accuracy: 90,
        pp: 10,
        effect_chance: 0,
    };
    const damage = 0;
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(defender.leech_seeded).toBe(false);
    expect(eventManager.dispatch).toHaveBeenCalledWith(expect.objectContaining({
      data: { text: `It doesn't affect ${defender.nickname}...` }
    }));
  });

  it('should fail to seed an already seeded defender', () => {
    defender.leech_seeded = true;
    move = {
        name: 'LEECH_SEED' as any,
        effect: MoveEffect.LEECH_SEED,
        power: 0,
        type: PokemonType.GRASS,
        accuracy: 90,
        pp: 10,
        effect_chance: 0,
    };
    const damage = 0;
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(eventManager.dispatch).toHaveBeenCalledWith(expect.objectContaining({
      data: { text: `But it failed!` }
    }));
  });

  it('should decrease attack by 1 when ATTACK_DOWN_HIT is applied', () => {
    move = {
        name: 'AURORA_BEAM' as any,
        effect: MoveEffect.ATTACK_DOWN_HIT,
        power: 65,
        type: PokemonType.ICE,
        accuracy: 100,
        pp: 20,
        effect_chance: 10,
    };
    const damage = 20;
    context.predefinedRandomValue = 0.05; // Ensure the effect triggers
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(defender.stat_boosts[Stat.ATTACK]).toBe(-1);
  });

  it('should decrease defense by 1 when DEFENSE_DOWN_HIT is applied', () => {
    move = {
        name: 'ACID' as any,
        effect: MoveEffect.DEFENSE_DOWN_HIT,
        power: 40,
        type: PokemonType.POISON,
        accuracy: 100,
        pp: 30,
        effect_chance: 10,
    };
    const damage = 20;
    context.predefinedRandomValue = 0.05; // Ensure the effect triggers
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(defender.stat_boosts[Stat.DEFENSE]).toBe(-1);
  });

  it('should decrease speed by 1 when SPEED_DOWN_HIT is applied', () => {
    move = {
        name: 'BUBBLE' as any,
        effect: MoveEffect.SPEED_DOWN_HIT,
        power: 20,
        type: PokemonType.WATER,
        accuracy: 100,
        pp: 30,
        effect_chance: 10,
    };
    const damage = 20;
    context.predefinedRandomValue = 0.05; // Ensure the effect triggers
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(defender.stat_boosts[Stat.SPEED]).toBe(-1);
  });

  it('should decrease special defense by 1 when SPECIAL_DEFENSE_DOWN_HIT is applied', () => {
    move = {
        name: 'SHADOW_BALL' as any,
        effect: MoveEffect.SPECIAL_DEFENSE_DOWN_HIT,
        power: 80,
        type: PokemonType.GHOST,
        accuracy: 100,
        pp: 15,
        effect_chance: 20,
    };
    const damage = 20;
    context.predefinedRandomValue = 0.1; // Ensure the effect triggers
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(defender.stat_boosts[Stat.SPECIAL_DEFENSE]).toBe(-1);
  });

  it('should decrease accuracy by 1 when ACCURACY_DOWN_HIT is applied', () => {
    move = {
        name: 'OCTAZOOKA' as any,
        effect: MoveEffect.ACCURACY_DOWN_HIT,
        power: 65,
        type: PokemonType.WATER,
        accuracy: 85,
        pp: 10,
        effect_chance: 50,
    };
    const damage = 20;
    context.predefinedRandomValue = 0.4; // Ensure the effect triggers
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(defender.stat_boosts[Stat.ACCURACY]).toBe(-1);
  });

  it('should decrease evasion by 1 when EVASION_DOWN_HIT is applied', () => {
    move = {
        name: 'SWEET_SCENT' as any,
        effect: MoveEffect.EVASION_DOWN_HIT,
        power: 0,
        type: PokemonType.NORMAL,
        accuracy: 100,
        pp: 20,
        effect_chance: 0,
    };
    const damage = 0;
    applyMoveEffect(attacker, defender, move, damage, context, eventManager, gameState, battle);

    expect(defender.stat_boosts[Stat.EVASION]).toBe(-1);
  });

  it.each([
    {
      name: 'PSNCUREBERRY',
      itemKey: 'PSNCUREBERRY',
      effect: MoveEffect.POISON_HIT,
      status: StatusCondition.POISON,
      expectedText: 'MOCK recovered using a PSNCUREBERRY!',
    },
    {
      name: 'PRZCUREBERRY',
      itemKey: 'PRZCUREBERRY',
      effect: MoveEffect.PARALYZE_HIT,
      status: StatusCondition.PARALYSIS,
      expectedText: 'MOCK recovered using a PRZCUREBERRY!',
    },
    {
      name: 'BURNT BERRY',
      itemKey: 'BURNT_BERRY',
      effect: MoveEffect.FREEZE_HIT,
      status: StatusCondition.FREEZE,
      expectedText: 'MOCK recovered using a BURNT BERRY!',
    },
    {
      name: 'ICE BERRY',
      itemKey: 'ICE_BERRY',
      effect: MoveEffect.BURN_HIT,
      status: StatusCondition.BURN,
      expectedText: 'MOCK recovered using a ICE BERRY!',
    },
    {
      name: 'MINT BERRY',
      itemKey: 'MINT_BERRY',
      effect: MoveEffect.SLEEP,
      status: StatusCondition.SLEEP,
      expectedText: 'MOCK recovered using a MINT BERRY!',
    },
    {
      name: 'MIRACLEBERRY',
      itemKey: 'MIRACLEBERRY',
      effect: MoveEffect.POISON_HIT,
      status: StatusCondition.POISON,
      expectedText: 'MOCK recovered using a MIRACLEBERRY!',
    },
  ])('consumes $name on status effect and recovers $status', ({ name, itemKey, effect, expectedText }) => {
    defender.item = itemKey;
    move = {
      name: `${name} MOVE` as any,
      effect,
      power: 0,
      type: PokemonType.NORMAL,
      accuracy: 100,
      pp: 10,
      effect_chance: 0,
    };

    const texts = captureBattleTexts();

    applyMoveEffect(attacker, defender, move, 10, context, eventManager, gameState, battle);

    expect(defender.item).toBeUndefined();
    expect(defender.status).toBeUndefined();
    expect(defender.sleep_turns).toBe(0);
    expect(texts).toContain(expectedText);
  });

  it('consumes BITTER BERRY on confusion and shows the confusion heal text', () => {
    defender.item = 'BITTER_BERRY';
    move = {
      name: 'CONFUSE MOVE' as any,
      effect: MoveEffect.CONFUSE,
      power: 0,
      type: PokemonType.NORMAL,
      accuracy: 100,
      pp: 10,
      effect_chance: 0,
    };

    const texts = captureBattleTexts();

    applyMoveEffect(attacker, defender, move, 10, context, eventManager, gameState, battle);

    expect(defender.item).toBeUndefined();
    expect(defender.confusion_turns).toBe(0);
    expect(texts).toContain('A BITTER BERRY rid MOCK of its confusion.');
  });
});

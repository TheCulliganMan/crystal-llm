
import { calculateDamage } from './damage-calculation';
import { Pokemon, PokemonSpecies, Move, PokemonSchema, PokemonSpeciesSchema, MoveSchema, LearnedMove, toPokemon } from '@pokecrystal/core/core/models';
import { PokemonType, Stat, GrowthRate, GenderRatio, EggGroup, MoveName, BattleTurn, ItemEnum, MoveEffect } from '@pokecrystal/core/core/enums';
import { BattleContext } from './battle-context';
import Fraction from 'fraction.js';
import { loadItems } from '@pokecrystal/core/core/data-loader';

describe('calculateDamage', () => {
    let charizard: Pokemon;
    let flamethrower: Move;
    let attacker: Pokemon;
    let defender: Pokemon;
    let move: Move;
    let context: BattleContext;

  beforeAll(async () => {
    await loadItems();
  });

  beforeEach(() => {
    const attackerSpecies: PokemonSpecies = PokemonSpeciesSchema.parse({
      id: "ATTACKER",
      int_id: 1,
      base_stats: { hp: 100, attack: 100, defense: 100, speed: 100, special_attack: 100, special_defense: 100 },
      type1: PokemonType.NORMAL,
      type2: PokemonType.NONE,
      catch_rate: 255,
      base_exp: 100,
      gender_ratio: GenderRatio.GENDER_F50,
      unknown1: 0,
      step_cycles_to_hatch: 0,
      unknown2: 0,
      growth_rate: GrowthRate.GROWTH_MEDIUM_FAST,
      egg_group1: EggGroup.EGG_MONSTER,
      egg_group2: EggGroup.EGG_MONSTER,
    });

    const defenderSpecies: PokemonSpecies = PokemonSpeciesSchema.parse({
        id: "DEFENDER",
        int_id: 2,
        base_stats: { hp: 100, attack: 100, defense: 100, speed: 100, special_attack: 100, special_defense: 100 },
        type1: PokemonType.NORMAL,
        type2: PokemonType.NONE,
        catch_rate: 255,
        base_exp: 100,
        gender_ratio: GenderRatio.GENDER_F50,
        unknown1: 0,
        step_cycles_to_hatch: 0,
        unknown2: 0,
        growth_rate: GrowthRate.GROWTH_MEDIUM_FAST,
        egg_group1: EggGroup.EGG_MONSTER,
        egg_group2: EggGroup.EGG_MONSTER,
      });

    attacker = toPokemon(PokemonSchema.parse({
        species: attackerSpecies,
        nickname: "ATTACKER",
        level: 50,
        hp: 100,
        max_hp: 100,
        item: undefined,
        moves: [],
        original_trainer_name: "Player",
        original_trainer_id: 1,
        experience: 0,
        happiness: 0,
      }));

    defender = toPokemon(PokemonSchema.parse({
        species: defenderSpecies,
        nickname: "DEFENDER",
        level: 50,
        hp: 100,
        max_hp: 100,
        item: undefined,
        moves: [],
        original_trainer_name: "Enemy",
        original_trainer_id: 2,
        experience: 0,
        happiness: 0,
      }));

    move = MoveSchema.parse({
        name: MoveName.TACKLE,
        type: PokemonType.NORMAL,
        power: 50,
        accuracy: 100,
        pp: 35,
      });

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
  });

  test('Thick Club should double the attack of Cubone/Marowak', () => {
    attacker.species.id = 'CUBONE';
    attacker.item = ItemEnum.THICK_CLUB;
    context.predefined_random_value = 1.0;
    const result = calculateDamage(attacker, defender, move, context);
    // Base attack is 100, doubled to 200.
    // damage = floor(floor((22 * 50 * 200) / 100) / 50) = 44
    // damage = damage + 2 = 46
    // STAB: damage = floor(46 * 1.5) = 69
    // damage = floor((damage * 255) / 255) = 69
    expect(result.damage).toBe(69);
  });

  test('Berserk Gene should raise attack by 2 stages', () => {
    attacker.item = ItemEnum.BERSERK_GENE;
    context.predefined_random_value = 1.0;
    const result = calculateDamage(attacker, defender, move, context);
    // Base attack is 100, with +2 stage it's 200.
    // damage = floor(floor((22 * 50 * 200) / 100) / 50) = 44
    // damage = damage + 2 = 46
    // STAB: damage = floor(46 * 1.5) = 69
    // damage = floor((damage * 255) / 255) = 69
    expect(result.damage).toBe(69);
  });

  test('badge boost should not be applied', () => {
    context.predefined_random_value = 1.0;

    // Calculation without badge boost:
    // level_factor = floor((2 * 50) / 5) + 2 = 22
    // damage = floor(floor((22 * 50 * 100) / 100) / 50) = 22
    // damage = damage + 2 = 24
    // STAB: damage = floor(24 * 1.5) = 36
    // damage = floor((damage * 255) / 255) = 36
    const result = calculateDamage(attacker, defender, move, context);
    expect(result.damage).toBe(36);
  });

  test('critical hit should double the damage', () => {
    context.predefined_random_value = 1.0;

    const normalResult = calculateDamage(attacker, defender, move, context, false);
    const critResult = calculateDamage(attacker, defender, move, context, true);

    expect(critResult.damage).toBe(69);
  });

  test('critical hit keeps boosted stats when defender stage is not higher', () => {
    attacker.stat_boosts[Stat.ATTACK] = 2;
    context.predefined_random_value = 1.0;

    const result = calculateDamage(attacker, defender, move, context, true);

    expect(result.damage).toBe(135);
  });

  test('critical hit ignores boosts when defender stage is higher than attacker stage', () => {
    attacker.stat_boosts[Stat.ATTACK] = 0;
    defender.stat_boosts[Stat.DEFENSE] = 1;
    context.predefined_random_value = 1.0;

    const result = calculateDamage(attacker, defender, move, context, true);

    expect(result.damage).toBe(69);
  });

  test('dragon scale should boost dragon moves by Gen 2 type-boost bug', () => {
    attacker.item = ItemEnum.DRAGON_SCALE;
    move.type = PokemonType.DRAGON;
    context.predefined_random_value = 1.0;

    const result = calculateDamage(attacker, defender, move, context);

    // Gen 2 bug: Type-boosting items increase move power by 1/16 (6.25%).
    // We intentionally preserve this behavior for faithful reproduction.
    // movePower = floor(50 * 1.0625) = 53
    // The calculation is complex due to integer truncation at multiple steps.
    // The empirically correct value is 25 with the 1/16th held-item modifier.
    expect(result.damage).toBe(25);
  });

  test('held type-boosting item should not affect non-matching move types', () => {
    attacker.item = ItemEnum.CHARCOAL;
    move.type = PokemonType.WATER;
    context.predefined_random_value = 1.0;

    const result = calculateDamage(attacker, defender, move, context);

    // No type match means Charcoal should be ignored.
    expect(result.damage).toBe(24);
  });

  test('ghost vs steel effectiveness', () => {
    defender.species.type1 = PokemonType.STEEL;
    move.type = PokemonType.GHOST;
    context.predefined_random_value = 1.0;

    const result = calculateDamage(attacker, defender, move, context);

    // Ghost against Steel should be not very effective (0.5x)
    // Base damage is 24, with 0.5x it's 12
    expect(result.damage).toBe(12);
  });

  test('dark vs steel effectiveness', () => {
    defender.species.type1 = PokemonType.STEEL;
    move.type = PokemonType.DARK;
    context.predefined_random_value = 1.0;

    const result = calculateDamage(attacker, defender, move, context);

    expect(result.damage).toBe(12);
  });

  test('explosion should halve defense', () => {
    attacker.species.base_stats.attack = 130;
    defender.species.base_stats.defense = 100;
    defender.species.type1 = PokemonType.ROCK;
    move.name = MoveName.EXPLOSION;
    move.power = 250;
    context.predefined_random_value = 1.0;

    const result = calculateDamage(attacker, defender, move, context);
    expect(result.damage).toBe(215);
  });

  test('plus two modifier order', () => {
    attacker.item = "CHARCOAL";
    move.type = PokemonType.FIRE;
    move.power = 50;
    attacker.level = 50;
    defender.level = 50;

    // Calculation breakdown with Gen 2 bug (1.0625x boost):
    // 1. Base damage (as above) = 22
    // 2. Apply item boost: damage = int(22 * 1.0625) = 23
    // 3. Apply critical hit: damage = 23 * 2 = 46
    // 4. Apply +2 modifier: damage = 46 + 2 = 48
    // 5. Apply random roll (max damage): (48 * 255) // 255 = 48

    context.predefined_random_value = 1.0; // Max damage roll
    const result = calculateDamage(
        attacker, defender, move, context, true
    );

    expect(result.damage).toBe(48);
  });

  test('intermediate damage cap', () => {
    attacker.level = 255;
    attacker.species.base_stats.attack = 255;
    defender.species.base_stats.defense = 1;
    move.power = 255;
    context.predefinedRandomValue = 1.0;

    // Calculation:
    // 1. Base Damage: A very large number that will exceed 999.
    // 2. Intermediate Cap: Damage is capped at 999.
    // 3. STAB (1.5x): 999 * 1.5 = 1498
    // 4. Final damage should be 1498, not a much larger number.

    const result = calculateDamage(
        attacker, defender, move, context, true
    );

    expect(result.damage).toBe(1498);
  });

  test('confusion damage should be typeless', () => {
    attacker.item = "BLACK_BELT";
    move.type = PokemonType.FIGHTING;
    move.power = 50;
    context.predefined_random_value = 1.0;

    const result = calculateDamage(attacker, defender, move, context, false, true);

    // Base damage is 22, +2 modifier is 24.
    // STAB and Black belt should not apply.
    expect(result.damage).toBe(24);
    });

    test('charcoal should boost fire moves by 1/16th (Gen 2 bug)', () => {
      attacker.item = "CHARCOAL";
      attacker.species.type1 = PokemonType.FIRE;
      move.type = PokemonType.FIRE;
      context.predefined_random_value = 1.0;

      const result = calculateDamage(attacker, defender, move, context);

      // Base damage calculation with bug:
      // We intentionally preserve this 1/16 (6.25%) boost for faithful reproduction.
      // movePower = floor(50 * 1.0625) = 53
      // damage = floor(floor(((2 * 50 / 5 + 2) * 53 * 100) / 100) / 50) = 23
      // +2 modifier: 23 + 2 = 25
      // STAB: floor(25 * 1.5) = 37
      expect(result.damage).toBe(37);
    });

    test('screens should apply on critical hits', () => {
        context.predefined_random_value = 1.0;
        context.enemyLightScreenTurns = 5; // Set the underlying turn counter
        move.type = PokemonType.FIRE; // A special move

        const result = calculateDamage(
            attacker,
            defender,
            move,
            context,
            true
        );

        // For a detailed breakdown of this calculation, see the Python
        // test case `test_screens_apply_on_critical_hits`.
        expect(result.damage).toBe(24);
    });

    test('Berserk Gene should stack with existing boosts', () => {
        attacker.item = ItemEnum.BERSERK_GENE;
        attacker.stat_boosts[Stat.ATTACK] = 2; // Attacker already has +2 attack
        context.predefined_random_value = 1.0;
        const result = calculateDamage(attacker, defender, move, context);
        // Base attack is 100.
        // With +2 from Swords Dance and +2 from Berserk Gene, total is +4 (3x).
        // damage = floor(floor((22 * 50 * 300) / 100) / 50) = 66
        // damage = damage + 2 = 68
        // STAB: damage = floor(68 * 1.5) = 102
        // damage = floor((damage * 255) / 255) = 102
        expect(result.damage).toBe(102);
    });

    test('struggle should hit ghost type', () => {
        defender.species.type1 = PokemonType.GHOST;
        move.name = MoveName.STRUGGLE;
        move.type = PokemonType.NORMAL;
        context.predefined_random_value = 1.0;

        const result = calculateDamage(attacker, defender, move, context);

        // Struggle should ignore type effectiveness and hit Ghost-types.
        // It should do neutral damage.
        // Base damage is 22, +2 is 24, STAB is 36.
        // But Struggle has no STAB.
        expect(result.damage).toBe(24);
    });

    test('reflect wraparound bug', () => {
        // In Gen 2, if Reflect doubles a Pokemon's defense to a value over
        // 1023, the defense stat wraps around.
        // e.g., Defense 600 * 2 = 1200. 1200 & 0x3FF = 176.
        defender._calculateStat = (stat: Stat) => {
          if (stat === Stat.DEFENSE) return 600;
          return 100;
        };

        // Attacker setup for consistent damage
        attacker._calculateStat = (stat: Stat) => {
            if (stat === Stat.ATTACK) return 100;
            return 100;
        };
        move.power = 50;
        context.predefined_random_value = 1.0; // Max damage roll

        // Enable Reflect for the defender's side
        context.enemyReflectTurns = 5;

        const resultWithBug = calculateDamage(
          attacker,
          defender,
          move,
          context
        );

        // Now, calculate what the damage *should* be with a doubled, capped defense
        defender._calculateStat = (stat: Stat) => {
            if (stat === Stat.DEFENSE) return 999;
            return 100;
        };
        context.enemyReflectTurns = 0; // Don't double it again
        const resultCorrectReflect = calculateDamage(
          attacker,
          defender,
          move,
          context
        );

        // The damage with the bug (low wrapped defense) should be much higher than
        // the damage with a properly doubled and capped defense.
        expect(resultWithBug.damage).toBeGreaterThan(resultCorrectReflect.damage);
      });

      test('thick club wraparound bug', () => {
        // In Gen 2, if Thick Club doubles an attack stat over 511, the
        // resulting value wraps around (e.g., 600 * 2 = 1200 -> 176).
        attacker.species.id = 'MAROWAK';
        attacker.item = ItemEnum.THICK_CLUB;
        attacker._calculateStat = (stat: Stat) => {
          if (stat === Stat.ATTACK) return 600;
          return 100;
        };

        // Attacker setup for consistent damage
        move.power = 50;
        context.predefined_random_value = 1.0; // Max damage roll

        const resultWithBug = calculateDamage(
          attacker,
          defender,
          move,
          context
        );

        // Manually calculate damage with a non-bugged, capped attack stat.
        attacker.item = undefined; // Remove Thick Club
        attacker._calculateStat = (stat: Stat) => {
            if (stat === Stat.ATTACK) return 999;
            return 100;
        };
        const resultWithoutBug = calculateDamage(
          attacker,
          defender,
          move,
          context
        );

        // With wraparound math (600 * 2 -> 176), the expected attack is low.
        expect(resultWithBug.damage).toBe(57);
        // Without wraparound, a capped high attack would be far higher.
        expect(resultWithoutBug.damage).toBe(316);
      });

      test('metal powder preserves boosted-defense asm quirk', () => {
        defender.species.id = 'DITTO';
        defender.item = ItemEnum.METAL_POWDER;
        defender.stat_boosts[Stat.DEFENSE] = 2;
        context.predefined_random_value = 1.0;

        const result = calculateDamage(
          attacker,
          defender,
          move,
          context
        );

        expect(result.damage).toBe(13);
      });

      test('item boosts should not apply to confusion damage', () => {
        attacker.item = "BLACK_BELT";
        move.type = PokemonType.FIGHTING;
        context.predefined_random_value = 1.0;

        const result = calculateDamage(
          attacker,
          defender,
          move,
          context,
          false,
          true
        );

        // Per Gen 2 mechanics, item boosts should not apply to confusion damage.
        // The base damage is 24. With the item boost, it would be 25.
        // This test will fail if the item boost is incorrectly applied.
        expect(result.damage).toBe(24);
      });

      test('type-boosting held item should not apply to confusion damage', () => {
        attacker.item = ItemEnum.CHARCOAL;
        move.type = PokemonType.FIRE;
        context.predefined_random_value = 1.0;

        const result = calculateDamage(
          attacker,
          defender,
          move,
          context,
          false,
          true
        );

        // Confusion damage is typeless and should ignore held item modifiers.
        expect(result.damage).toBe(24);
      });

      test('berserk gene should not apply to confusion damage', () => {
        attacker.item = ItemEnum.BERSERK_GENE;
        context.predefined_random_value = 1.0;

        const result = calculateDamage(
          attacker,
          defender,
          move,
          context,
          false,
          true
        );

        // Per Gen 2 mechanics, Berserk Gene should not apply to confusion damage.
        // The base damage is 24. With the boost, it would be 69.
        expect(result.damage).toBe(24);
      });

      test('berserk gene should only affect physical moves', () => {
        attacker.item = ItemEnum.BERSERK_GENE;
        move.type = PokemonType.ELECTRIC;
        context.predefined_random_value = 1.0;

        const result = calculateDamage(attacker, defender, move, context);

        // Special moves should ignore the Berserk Gene +2 attack-stage effect.
        expect(result.damage).toBe(24);
      });

      test('rage damage scales before the random roll', () => {
        const rageMove = MoveSchema.parse({
          name: MoveName.RAGE,
          type: PokemonType.NORMAL,
          power: 20,
          accuracy: 100,
          pp: 20,
          effect: MoveEffect.RAGE,
          effect_chance: 0,
        });

        context.predefined_random_value = 1.0;
        attacker.rage_counter = 0;
        const maxRoll = calculateDamage(attacker, defender, rageMove, context).damage;

        context.predefined_random_value = 0.0;
        attacker.rage_counter = 2;
        const rageResult = calculateDamage(attacker, defender, rageMove, context).damage;

        const expected = Math.trunc((maxRoll * 3 * 217) / 255);
        expect(rageResult).toBe(expected);
      });
});


import { calculateDamage, _normalizeItemName } from '@pokecrystal/core/engine/battle/battle/damage-calculation';
import { Pokemon, PokemonData, PokemonSpecies, PokemonSchema, toPokemon, Move } from '@pokecrystal/core/core/models';
import { BattleContext } from '@pokecrystal/core/engine/battle/battle/battle-context';
import {
    PokemonType,
    Stat,
    Ability,
    MoveEffect,
    MoveName,
    StatusCondition,
    BattleTurn,
    ItemEnum,
    GrowthRate,
    GenderRatio,
    EggGroup,
} from '@pokecrystal/core/core/enums';
import Fraction from 'fraction.js';

const DEFAULT_SPECIES_BASE_STATS = {
    hp: 50,
    attack: 50,
    defense: 50,
    speed: 50,
    special_attack: 50,
    special_defense: 50,
};

const defaultSpecies: PokemonSpecies = {
    id: 'TEST',
    int_id: 1,
    base_stats: { ...DEFAULT_SPECIES_BASE_STATS },
    type1: PokemonType.NORMAL,
    type2: PokemonType.NONE,
    catch_rate: 255,
    base_exp: 1,
    gender_ratio: GenderRatio.GENDER_F50,
    unknown1: 0,
    step_cycles_to_hatch: 0,
    unknown2: 0,
    growth_rate: GrowthRate.GROWTH_MEDIUM_FAST,
    egg_group1: EggGroup.EGG_MONSTER,
    egg_group2: EggGroup.EGG_MONSTER,
    tmhm_learnset: [],
    ability: Ability.NONE,
    pic_size: 0,
    front_pic: 0,
    back_pic: 0,
};

const defaultPokemonData: Partial<PokemonData> = {
    nickname: 'TEST',
    level: 50,
    hp: 100,
    max_hp: 100,
    original_trainer_name: 'PLAYER',
    original_trainer_id: 1,
    experience: 0,
    happiness: 70,
};

const defaultMove: Move = {
    name: MoveName.TACKLE,
    type: PokemonType.NORMAL,
    power: 50,
    accuracy: 100,
    pp: 35,
    effect: MoveEffect.NORMAL_HIT,
    effect_chance: 0,
};

const makeSpecies = (overrides: Partial<PokemonSpecies> = {}): PokemonSpecies => ({
    ...defaultSpecies,
    ...overrides,
    base_stats: {
        ...DEFAULT_SPECIES_BASE_STATS,
        ...(overrides.base_stats ?? {}),
    },
});

const createPokemon = ({
    species = defaultSpecies,
    overrides = {},
}: {
    species?: PokemonSpecies;
    overrides?: Partial<PokemonData>;
} = {}): Pokemon => {
    return toPokemon(
        PokemonSchema.parse({
            ...defaultPokemonData,
            ...overrides,
            species,
        })
    );
};

const buildBattleContext = (attacker: Pokemon, defender: Pokemon): BattleContext =>
    new BattleContext([attacker], [defender], attacker, defender, undefined, false, undefined, 0);

const staticStatCalculator = (
    overrides: Partial<Record<Stat, number>>,
    fallback = 100
): Pokemon['_calculateStat'] =>
    ((stat: Stat) => overrides[stat] ?? fallback) as Pokemon['_calculateStat'];

describe('damage-calculation', () => {
    describe('_normalizeItemName', () => {
        it('should normalize item names', () => {
            expect(_normalizeItemName('test-item')).toBe('TEST ITEM');
            expect(_normalizeItemName('test_item')).toBe('TEST ITEM');
            expect(_normalizeItemName('TeSt ItEm')).toBe('TEST ITEM');
            expect(_normalizeItemName('test--item')).toBe('TEST ITEM');
        });
    });

    describe('calculateDamage', () => {
        it('should calculate damage correctly', () => {
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
                type2: PokemonType.NONE,
                catch_rate: 45,
                base_exp: 62,
                gender_ratio: 31,
                unknown1: 0,
                step_cycles_to_hatch: 20,
                unknown2: 0,
                growth_rate: "GROWTH_MEDIUM_SLOW",
                egg_group1: "EGG_MONSTER",
                egg_group2: "EGG_DRAGON",
                tmhm_learnset: [],
                ability: Ability.NONE,
                pic_size: 0,
                front_pic: 0,
                back_pic: 0,
            };

            const attacker: Pokemon = toPokemon(
                PokemonSchema.parse({
                    species,
                    nickname: 'CHARMANDER',
                    level: 50,
                    hp: 100,
                    max_hp: 100,
                    dvs: { attack: 0, defense: 0, speed: 0, special: 0, hp: 0 },
                    status: StatusCondition.NONE,
                    moves: [],
                    original_trainer_name: 'RED',
                    original_trainer_id: 12345,
                    experience: 125000,
                    happiness: 70,
                    stat_boosts: {
                        [Stat.HP]: 0,
                        [Stat.ATTACK]: 0,
                        [Stat.DEFENSE]: 0,
                        [Stat.SPEED]: 0,
                        [Stat.SPECIAL_ATTACK]: 0,
                        [Stat.SPECIAL_DEFENSE]: 0,
                        [Stat.ACCURACY]: 0,
                        [Stat.EVASION]: 0,
                    },
                    attack: 0, // These will be ignored by the stat calculation, which is correct
                    defense: 0,
                    speed: 0,
                    special_attack: 0,
                    special_defense: 0,
                    hp_exp: 0,
                    attack_exp: 0,
                    defense_exp: 0,
                    speed_exp: 0,
                    special_exp: 0,
                    sleep_turns: 0,
                    cant_run: false,
                    flinching: false,
                    rampage_turns: 0,
                    confusion_turns: 0,
                    perish_song_turns: 0,
                    turns_in_battle: 0,
                    locked_turns_remaining: 0,
                    trapped_turns: 0,
                    leech_seeded: false,
                    nightmare: false,
                    cursed: false,
                    disable_turns: 0,
                    encore_turns_remaining: 0,
                    destiny_bond_active: false,
                    focus_energy: false,
                    rage_active: false,
                    rage_counter: 0,
                    fury_cutter_count: 0,
                    rollout_step: 0,
                    rollout_active: false,
                    defense_curled: false,
                    bide_active: false,
                    bide_turns_remaining: 0,
                    bide_damage: 0,
                    protect_active: false,
                    protect_counter: 0,
                    endure_active: false,
                    endure_counter: 0,
                    foresight_active: false,
                    lock_on_active: false,
                    substitute_hp: 0,
                    transformed: false,
                    last_damage_taken: 0,
                })
            );

            const defender: Pokemon = toPokemon(
                PokemonSchema.parse({
                    ...attacker,
                    species: {
                        ...species,
                        type1: PokemonType.GRASS,
                    },
                })
            );

            const move: Move = {
                name: MoveName.FLAMETHROWER,
                type: PokemonType.FIRE,
                power: 95,
                accuracy: 100,
                pp: 15,
                effect: 0,
                effect_chance: 0,
            };

            const context: Partial<BattleContext> = {
                sideFor: () => BattleTurn.PLAYER,
                badgeBoostActive: () => false,
                weather: 0,
                predefined_random_value: 1.0,
                playerReflect: false,
                enemyReflect: false,
                playerLightScreen: false,
                enemyLightScreen: false,
            };

            const result = calculateDamage(attacker, defender, move, context as BattleContext);

            expect(result.damage).toBe(152);
            expect(result.type_multiplier).toEqual(new Fraction(2));
        });
    });

    describe('Gen 2 wraparound quirks', () => {
        it('keeps Reflect defense wraparound intact', () => {
            const attacker = createPokemon();
            const defender = createPokemon();
            attacker._calculateStat = staticStatCalculator({ [Stat.ATTACK]: 100 });
            defender._calculateStat = staticStatCalculator({ [Stat.DEFENSE]: 600 });

            const context = buildBattleContext(attacker, defender);
            context.predefinedRandomValue = 1.0;
            context.enemyReflectTurns = 5;

            const resultWithBug = calculateDamage(attacker, defender, { ...defaultMove }, context);

            defender._calculateStat = staticStatCalculator({ [Stat.DEFENSE]: 999 });
            context.enemyReflectTurns = 0;

            const resultWithoutWrap = calculateDamage(attacker, defender, { ...defaultMove }, context);

            expect(resultWithBug.damage).toBeGreaterThan(resultWithoutWrap.damage);
        });

        it('reproduces the Thick Club attack wraparound', () => {
            const attacker = createPokemon({
                species: makeSpecies({ id: 'MAROWAK', type1: PokemonType.GROUND }),
            });
            const defender = createPokemon();
            attacker.item = ItemEnum.THICK_CLUB;
            attacker._calculateStat = staticStatCalculator({ [Stat.ATTACK]: 600 });
            defender._calculateStat = staticStatCalculator({ [Stat.DEFENSE]: 100 });

            const context = buildBattleContext(attacker, defender);
            context.predefinedRandomValue = 1.0;

            const resultWithBug = calculateDamage(attacker, defender, { ...defaultMove }, context);

            attacker.item = null;
            attacker._calculateStat = staticStatCalculator({ [Stat.ATTACK]: 999 });

            const resultWithoutBug = calculateDamage(attacker, defender, { ...defaultMove }, context);

            expect(resultWithBug.damage).toBeLessThan(resultWithoutBug.damage);
        });
    });
});

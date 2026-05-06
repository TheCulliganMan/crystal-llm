import { awardStatExp, grantPlayerExperience } from './experience';
import { Pokemon, PokemonSchema, PokemonSpecies, toPokemon } from '../../../core/models';
import { GrowthRate, PokemonType, EggGroup, GenderRatio, MoveName, Ability } from '../../../core/enums';
import { Battle } from './battle-logic';

describe('grantPlayerExperience', () => {
    let participant: Pokemon;
    let bench_warmer: Pokemon;
    let fainted: Pokemon;

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

        const createPokemon = (nickname: string): Pokemon => {
            return toPokemon(PokemonSchema.parse({
                species,
                nickname,
                level: 5,
                hp: 20,
                max_hp: 20,
                original_trainer_name: 'PLAYER',
                original_trainer_id: 1,
                experience: 125,
                hp_exp: 0,
                attack_exp: 0,
                defense_exp: 0,
                speed_exp: 0,
                special_exp: 0,
                happiness: 70,
                moves: [{ name: MoveName.TACKLE, current_pp: 35 }],
            }));
        };

        participant = createPokemon('participant');
        bench_warmer = createPokemon('bench_warmer');
        fainted = createPokemon('fainted');
    });

    it('should award stat experience only to participants', () => {
        const mockBattle = {
            context: {
            playerParty: [participant, bench_warmer],
            playerParticipantsNotFainted: [0],
        },
        gameState: {
            sram: { player_id: 1 },
            auto_exp_share_enabled: false,
        },
            battleUiCall: () => null,
            eventManager: { dispatch: () => { } },
        } as unknown as Battle;

        grantPlayerExperience(mockBattle, fainted);

        const statsYield = fainted.species.base_stats;
        expect(participant.hp_exp).toBe(statsYield.hp);
        expect(participant.attack_exp).toBe(statsYield.attack);
        expect(participant.defense_exp).toBe(statsYield.defense);
        expect(participant.speed_exp).toBe(statsYield.speed);
        expect(participant.special_exp).toBe(statsYield.special_attack);

        expect(bench_warmer.hp_exp).toBe(0);
        expect(bench_warmer.attack_exp).toBe(0);
        expect(bench_warmer.defense_exp).toBe(0);
        expect(bench_warmer.speed_exp).toBe(0);
        expect(bench_warmer.special_exp).toBe(0);
    });

    it('should not halve stat experience when an Exp. Share is held or auto-share is enabled', () => {
        const mockBattle = {
            context: {
            playerParty: [participant, bench_warmer],
            playerParticipantsNotFainted: [0],
        },
        gameState: {
            sram: { player_id: 1 },
            auto_exp_share_enabled: true,
        },
            battleUiCall: () => null,
            eventManager: { dispatch: () => { } },
        } as unknown as Battle;

        grantPlayerExperience(mockBattle, fainted);

        const statsYield = fainted.species.base_stats;
        expect(participant.hp_exp).toBe(statsYield.hp);
        expect(participant.attack_exp).toBe(statsYield.attack);
        expect(participant.defense_exp).toBe(statsYield.defense);
        expect(participant.speed_exp).toBe(statsYield.speed);
        expect(participant.special_exp).toBe(statsYield.special_attack);

        expect(bench_warmer.hp_exp).toBe(0);
        expect(bench_warmer.attack_exp).toBe(0);
        expect(bench_warmer.defense_exp).toBe(0);
        expect(bench_warmer.speed_exp).toBe(0);
        expect(bench_warmer.special_exp).toBe(0);
    });

});

describe('awardStatExp', () => {
    let pokemon: Pokemon;
    let fainted: Pokemon;

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
            hp_exp: 0,
            attack_exp: 0,
            defense_exp: 0,
            speed_exp: 0,
            special_exp: 0,
            happiness: 70,
            moves: [],
        }));
        fainted = { ...pokemon };
    });

    it('should award stat experience correctly', () => {
        const statsYield = {
            hp: fainted.species.base_stats.hp,
            attack: fainted.species.base_stats.attack,
            defense: fainted.species.base_stats.defense,
            speed: fainted.species.base_stats.speed,
            special: fainted.species.base_stats.special_attack,
        };
        awardStatExp(pokemon, statsYield);
        expect(pokemon.hp_exp).toBe(39);
        expect(pokemon.attack_exp).toBe(52);
        expect(pokemon.defense_exp).toBe(43);
        expect(pokemon.speed_exp).toBe(65);
        expect(pokemon.special_exp).toBe(60);
    });

    it('should double stat experience for pokemon with pokerus', () => {
        pokemon.pokerus = true;
        const statsYield = {
            hp: fainted.species.base_stats.hp,
            attack: fainted.species.base_stats.attack,
            defense: fainted.species.base_stats.defense,
            speed: fainted.species.base_stats.speed,
            special: fainted.species.base_stats.special_attack,
        };
        awardStatExp(pokemon, statsYield);
        expect(pokemon.hp_exp).toBe(78);
        expect(pokemon.attack_exp).toBe(104);
        expect(pokemon.defense_exp).toBe(86);
        expect(pokemon.speed_exp).toBe(130);
        expect(pokemon.special_exp).toBe(120);
    });
});

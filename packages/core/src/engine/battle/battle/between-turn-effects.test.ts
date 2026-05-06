import { BattleContext } from './battle-context';
import { activateBerserkGene } from './between-turn-effects';
import { Pokemon, PokemonSchema, PokemonSpecies, PokemonSpeciesSchema, toPokemon } from '@pokecrystal/core/core/models';
import { EventManager } from '@pokecrystal/core/engine/events/events';
import { StatusCondition, PokemonType, GrowthRate, GenderRatio, EggGroup, Ability } from '@pokecrystal/core/core/enums';

const createSpecies = (): PokemonSpecies =>
    PokemonSpeciesSchema.parse({
        id: "MOCK_SPECIES",
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
        tmhm_learnset: [],
        evolutions: null,
        ability: Ability.NONE,
        pic_size: 0,
        front_pic: 0,
        back_pic: 0,
    });

const createBerserkPokemon = (species: PokemonSpecies, options: {
    nickname: string,
    status?: StatusCondition | null,
    item?: string | undefined,
    hp?: number,
    confusionTurns?: number,
}) => toPokemon(PokemonSchema.parse({
    species,
    nickname: options.nickname,
    level: 50,
    hp: options.hp ?? 100,
    max_hp: options.hp ?? 100,
    item: options.item,
    moves: [],
    original_trainer_name: options.nickname,
    original_trainer_id: 1,
    experience: 0,
    happiness: 0,
    status: options.status,
    confusion_turns: options.confusionTurns ?? 0,
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

describe('activateBerserkGene', () => {
    const createEventManager = (): EventManager =>
        new EventManager({ hram: { hardware_divider: 0, hRandomAdd: 0, hRandomSub: 0 } } as any);

    it('sets volatile confusion without replacing primary status', () => {
        const species = createSpecies();
        const playerPokemon: Pokemon = createBerserkPokemon(species, { nickname: "MOCK", item: 'BERSERK_GENE' });

        const enemyPokemon: Pokemon = createBerserkPokemon(species, { nickname: "ENEMY", hp: 0, item: undefined });


        const context = new BattleContext(
            [playerPokemon],
            [enemyPokemon],
            playerPokemon,
            enemyPokemon,
            undefined,
            false,
            undefined,
            0
        );

        const eventManager = createEventManager();
        playerPokemon.confusion_turns = 0;

        activateBerserkGene(context, eventManager);

        expect(playerPokemon.status).toBeUndefined();
        expect(playerPokemon.stat_boosts.ATTACK).toBe(2);
        expect(playerPokemon.item).toBeUndefined();
        expect(playerPokemon.confusion_turns).toBeGreaterThanOrEqual(2);
        expect(playerPokemon.confusion_turns).toBeLessThanOrEqual(5);
    });
    it('should not overwrite status', () => {
        const species = createSpecies();
        const playerPokemon: Pokemon = createBerserkPokemon(species, {
            nickname: "MOCK",
            item: 'BERSERK_GENE',
            status: StatusCondition.POISON,
        });

        const enemyPokemon: Pokemon = createBerserkPokemon(species, { nickname: "ENEMY", hp: 0, item: undefined });


        const context = new BattleContext(
            [playerPokemon],
            [enemyPokemon],
            playerPokemon,
            enemyPokemon,
            undefined,
            false,
            undefined,
            0
        );

        const eventManager = createEventManager();

        activateBerserkGene(context, eventManager);

        expect(playerPokemon.status).toBe(StatusCondition.POISON);
        expect(playerPokemon.confusion_turns).toBeGreaterThanOrEqual(2);
        expect(playerPokemon.stat_boosts.ATTACK).toBe(2);
        expect(playerPokemon.item).toBeUndefined();
    });

    it('activates berserk gene for both sides', () => {
        const species = createSpecies();
        const playerPokemon: Pokemon = createBerserkPokemon(species, {
            nickname: "MOCK",
            item: 'BERSERK_GENE',
            status: undefined,
        });
        const enemyPokemon: Pokemon = createBerserkPokemon(species, {
            nickname: "ENEMY",
            item: 'BERSERK_GENE',
            status: StatusCondition.POISON,
        });

        const context = new BattleContext(
            [playerPokemon],
            [enemyPokemon],
            playerPokemon,
            enemyPokemon,
            undefined,
            false,
            undefined,
            0
        );

        const eventManager = createEventManager();

        activateBerserkGene(context, eventManager);

        expect(playerPokemon.item).toBeUndefined();
        expect(enemyPokemon.item).toBeUndefined();
        expect(playerPokemon.stat_boosts.ATTACK).toBe(2);
        expect(enemyPokemon.stat_boosts.ATTACK).toBe(2);
        expect(playerPokemon.status).toBeUndefined();
        expect(enemyPokemon.status).toBe(StatusCondition.POISON);
        expect(playerPokemon.confusion_turns).toBeGreaterThanOrEqual(2);
        expect(enemyPokemon.confusion_turns).toBeGreaterThanOrEqual(2);
    });
});

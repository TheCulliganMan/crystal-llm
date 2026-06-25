import { createPokemon, pokemonSpeciesDisplayName, Pokemon, PokemonSpecies, toPokemon, PokemonSchema, PokemonSpeciesSchema } from '../models/pokemon';
import { GameState, WRAMSchema } from '@pokecrystal/core/core/state';
import { HardwareRNG } from '@pokecrystal/core/engine/games/rng';
import { GrowthRate, Stat, PokemonType, GenderRatio, EggGroup, Ability } from '@pokecrystal/core/core/enums';
import { MoveName } from "@pokecrystal/core/core/enums/move";

// Mock the HardwareRNG class
jest.mock('@pokecrystal/core/engine/games/rng');

describe('Pokemon', () => {
    let mockGameState: GameState;
    let sampleSpecies: PokemonSpecies;

    beforeEach(() => {
        // Reset mocks before each test
        (HardwareRNG as jest.Mock).mockClear();

        // Mock GameState
        mockGameState = {
            wram: WRAMSchema.parse({}),
            hram: {
                hRandomAdd: 0,
                hRandomSub: 0,
            }
        } as GameState;

        // Sample species for testing
        sampleSpecies = {
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
            type2: PokemonType.FIRE,
            catch_rate: 45,
            base_exp: 62,
            item1: undefined,
            item2: undefined,
            gender_ratio: GenderRatio.GENDER_F12_5,
            growth_rate: GrowthRate.GROWTH_MEDIUM_SLOW,
            egg_group1: EggGroup.EGG_MONSTER,
            egg_group2: EggGroup.EGG_DRAGON,
            step_cycles_to_hatch: 20,
            unknown1: 0,
            unknown2: 0,
            tmhm_learnset: [],
            ability: Ability.NONE,
            pic_size: 0,
            front_pic: 0,
            back_pic: 0,
            evolutions: null,
            weight: 0,
        };
    });

    describe('pokemonSpeciesDisplayName', () => {
        it('uses ASM species names for identifier-only species constants', () => {
            expect(pokemonSpeciesDisplayName('FARFETCH_D')).toBe("FARFETCH'D");
            expect(pokemonSpeciesDisplayName('HO_OH')).toBe('HO-OH');
            expect(pokemonSpeciesDisplayName('MR__MIME')).toBe('MR.MIME');
            expect(pokemonSpeciesDisplayName('NIDORAN_F')).toBe('NIDORAN\u2640');
            expect(pokemonSpeciesDisplayName('NIDORAN_M')).toBe('NIDORAN\u2642');
        });
    });

    describe('PokemonSpeciesSchema', () => {
        it('preserves exact modpack growth-rate ids', () => {
            const species = PokemonSpeciesSchema.parse({
                ...sampleSpecies,
                growth_rate: 'GROWTH_CUSTOM_PACK_CURVE',
            });

            expect(species.growth_rate).toBe('GROWTH_CUSTOM_PACK_CURVE');
        });
    });

    describe('createPokemon', () => {
        it('should create a new Pokemon with correct stats and DVs', () => {
            // Configure the mock to return specific DV values
            const mockRandRange = jest.fn()
                .mockReturnValueOnce(9)  // Attack DV
                .mockReturnValueOnce(7)  // Defense DV
                .mockReturnValueOnce(5) // Speed DV
                .mockReturnValueOnce(3); // Special DV

            (HardwareRNG as jest.Mock).mockImplementation(() => {
                return {
                    randrange: mockRandRange,
                };
            });

            const level = 5;

            // Adjust the mock to make it more interesting for HP DV calculation
            mockRandRange.mockReset()
                .mockReturnValueOnce(9)  // Attack DV (odd)
                .mockReturnValueOnce(7)  // Defense DV (odd)
                .mockReturnValueOnce(5)  // Speed DV (odd)
                .mockReturnValueOnce(3); // Special DV (odd)

            const pokemon = createPokemon(mockGameState, sampleSpecies, level);
            expect(pokemon.dvs.hp).toBe(8 + 4 + 2 + 1);

            // Verify level and experience
            expect(pokemon.level).toBe(level);
            expect(pokemon.experience).toBe(135); // Based on GROWTH_MEDIUM_SLOW at level 5

            // Verify stats (cross-referenced with an online calculator for Gen 2)
            // DVs: HP=15, Atk=9, Def=7, Spe=5, Spc=3
            // Level 5, Charmander
            expect(pokemon.max_hp).toBe(20);
            expect(pokemon.attack).toBe(11);
            expect(pokemon.defense).toBe(10);
            expect(pokemon.speed).toBe(12);
            expect(pokemon.special_attack).toBe(11);
            expect(pokemon.special_defense).toBe(10);
        });

        it('should create a Pokemon with the correct default moves', () => {
            const level = 10;

            const pokemon = createPokemon(mockGameState, sampleSpecies, level);

            expect(pokemon.moves.map((move) => move.name)).toEqual([
              MoveName.SCRATCH,
              MoveName.GROWL,
              MoveName.EMBER,
            ]);
        });

        it('uses the ASM species name for Ho-Oh as the default nickname', () => {
            const species = {
                ...sampleSpecies,
                id: 'HO_OH',
                int_id: 250,
            };

            const pokemon = createPokemon(mockGameState, species, 60);

            expect(pokemon.nickname).toBe('HO-OH');
        });
    });

    describe('_calculateStat', () => {
        it('should calculate the correct stat values', () => {
            const species = PokemonSpeciesSchema.parse({
                ...sampleSpecies,
            });
            const pokemonSchema = {
                species: species,
                level: 50,
                dvs: { attack: 15, defense: 15, speed: 15, special: 15, hp: 15 },
                hp_exp: 25000,
                attack_exp: 30000,
                defense_exp: 20000,
                speed_exp: 40000,
                special_exp: 35000,
                nickname: 'CHARMANDER',
                original_trainer_name: 'RED',
                original_trainer_id: 123,
                experience: 117360,
                happiness: 70,
                hp: 0,
                max_hp: 0
            };
            const pokemon = toPokemon(PokemonSchema.parse(pokemonSchema));

            // Values cross-referenced with an online Gen 2 stat calculator
            expect(pokemon._calculateStat(Stat.HP)).toBe(133);
            expect(pokemon._calculateStat(Stat.ATTACK)).toBe(93);
            expect(pokemon._calculateStat(Stat.DEFENSE)).toBe(80);
            expect(pokemon._calculateStat(Stat.SPEED)).toBe(110);
            expect(pokemon._calculateStat(Stat.SPECIAL_ATTACK)).toBe(103);
            expect(pokemon._calculateStat(Stat.SPECIAL_DEFENSE)).toBe(93);
        });
    });
});

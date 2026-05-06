
import { Pokemon, PokemonSpecies, DV } from "@/core/models/pokemon";
import { GenderRatio, PlayerGender, Stat, EggGroup } from "@/core/enums";
import { _compatibility_threshold, _choose_gender, checkBreedingCompatibility } from "@/engine/systems/breeding";

const createMockPokemon = (species_id: string, egg_group1: EggGroup, egg_group2: EggGroup, ot_id: number, gender: PlayerGender | null, defense_dv: number, special_dv: number): Pokemon => {
    return {
        species: {
            id: species_id,
            egg_group1: egg_group1,
            egg_group2: egg_group2,
        } as PokemonSpecies,
        original_trainer_id: ot_id,
        gender: gender,
        dvs: {
            defense: defense_dv,
            special: special_dv,
        } as DV,
    } as Pokemon;
};


describe("Breeding", () => {
    describe("_compatibility_threshold", () => {
        it("should return the correct compatibility threshold", () => {
        // "appears to care for" -> ~31.25% chance
        expect(_compatibility_threshold(254)).toBe(80);
        expect(_compatibility_threshold(230)).toBe(80);

        // "friendly" -> ~16% chance
        expect(_compatibility_threshold(229)).toBe(40);
        expect(_compatibility_threshold(170)).toBe(40);

        // "shows interest" -> ~12% chance
        expect(_compatibility_threshold(110)).toBe(30);
        expect(_compatibility_threshold(109)).toBe(10);
        expect(_compatibility_threshold(70)).toBe(10);
        expect(_compatibility_threshold(1)).toBe(10);
        expect(_compatibility_threshold(0)).toBe(0);
        expect(_compatibility_threshold(255)).toBe(0);
        });
    });

    describe("checkBreedingCompatibility", () => {
        it("should return the correct compatibility score", () => {
            // Same species, same OT
            let pokemon1 = createMockPokemon("BULBASAUR", EggGroup.EGG_MONSTER, EggGroup.EGG_PLANT, 1, PlayerGender.MALE, 5, 5);
            let pokemon2 = createMockPokemon("BULBASAUR", EggGroup.EGG_MONSTER, EggGroup.EGG_PLANT, 1, PlayerGender.FEMALE, 10, 10);
            expect(checkBreedingCompatibility(pokemon1, pokemon2)).toBe(177);

            // Same species, different OT
            pokemon1 = createMockPokemon("BULBASAUR", EggGroup.EGG_MONSTER, EggGroup.EGG_PLANT, 1, PlayerGender.MALE, 5, 5);
            pokemon2 = createMockPokemon("BULBASAUR", EggGroup.EGG_MONSTER, EggGroup.EGG_PLANT, 2, PlayerGender.FEMALE, 10, 10);
            expect(checkBreedingCompatibility(pokemon1, pokemon2)).toBe(254);

            // Different species, same egg group, same OT
            pokemon1 = createMockPokemon("BULBASAUR", EggGroup.EGG_MONSTER, EggGroup.EGG_PLANT, 1, PlayerGender.MALE, 5, 5);
            pokemon2 = createMockPokemon("CHARMANDER", EggGroup.EGG_MONSTER, EggGroup.EGG_DRAGON, 1, PlayerGender.FEMALE, 10, 10);
            expect(checkBreedingCompatibility(pokemon1, pokemon2)).toBe(51);

            // Different species, same egg group, different OT
            pokemon1 = createMockPokemon("BULBASAUR", EggGroup.EGG_MONSTER, EggGroup.EGG_PLANT, 1, PlayerGender.MALE, 5, 5);
            pokemon2 = createMockPokemon("CHARMANDER", EggGroup.EGG_MONSTER, EggGroup.EGG_DRAGON, 2, PlayerGender.FEMALE, 10, 10);
            expect(checkBreedingCompatibility(pokemon1, pokemon2)).toBe(128);

            // Incompatible egg group
            pokemon1 = createMockPokemon("BULBASAUR", EggGroup.EGG_MONSTER, EggGroup.EGG_PLANT, 1, PlayerGender.MALE, 5, 5);
            pokemon2 = createMockPokemon("GEODUDE", EggGroup.EGG_MINERAL, EggGroup.EGG_MINERAL, 1, PlayerGender.FEMALE, 10, 10);
            expect(checkBreedingCompatibility(pokemon1, pokemon2)).toBe(0);

            // No eggs species
            pokemon1 = createMockPokemon("MEWTWO", EggGroup.EGG_NONE, EggGroup.EGG_NONE, 1, null, 5, 5);
            pokemon2 = createMockPokemon("BULBASAUR", EggGroup.EGG_MONSTER, EggGroup.EGG_PLANT, 1, PlayerGender.FEMALE, 10, 10);
            expect(checkBreedingCompatibility(pokemon1, pokemon2)).toBe(0);

            // Two dittos
            pokemon1 = createMockPokemon("DITTO", EggGroup.EGG_DITTO, EggGroup.EGG_NONE, 1, null, 5, 5);
            pokemon2 = createMockPokemon("DITTO", EggGroup.EGG_DITTO, EggGroup.EGG_NONE, 2, null, 10, 10);
            expect(checkBreedingCompatibility(pokemon1, pokemon2)).toBe(0);

            // Same gender
            pokemon1 = createMockPokemon("BULBASAUR", EggGroup.EGG_MONSTER, EggGroup.EGG_PLANT, 1, PlayerGender.MALE, 5, 5);
            pokemon2 = createMockPokemon("BULBASAUR", EggGroup.EGG_MONSTER, EggGroup.EGG_PLANT, 1, PlayerGender.MALE, 10, 10);
            expect(checkBreedingCompatibility(pokemon1, pokemon2)).toBe(0);

            // Genderless (non-ditto)
            pokemon1 = createMockPokemon("MAGNEMITE", EggGroup.EGG_MINERAL, EggGroup.EGG_NONE, 1, null, 5, 5);
            pokemon2 = createMockPokemon("BULBASAUR", EggGroup.EGG_MONSTER, EggGroup.EGG_PLANT, 1, PlayerGender.FEMALE, 10, 10);
            expect(checkBreedingCompatibility(pokemon1, pokemon2)).toBe(0);

            // Matching DVs
            pokemon1 = createMockPokemon("BULBASAUR", EggGroup.EGG_MONSTER, EggGroup.EGG_PLANT, 1, PlayerGender.MALE, 10, 15); // special is 1111
            pokemon2 = createMockPokemon("BULBASAUR", EggGroup.EGG_MONSTER, EggGroup.EGG_PLANT, 1, PlayerGender.FEMALE, 10, 7); // special is 0111
            expect(checkBreedingCompatibility(pokemon1, pokemon2)).toBe(255);

            // Ditto + another pokemon
            pokemon1 = createMockPokemon("DITTO", EggGroup.EGG_DITTO, EggGroup.EGG_NONE, 1, null, 5, 5);
            pokemon2 = createMockPokemon("BULBASAUR", EggGroup.EGG_MONSTER, EggGroup.EGG_PLANT, 1, PlayerGender.MALE, 10, 10);
            expect(checkBreedingCompatibility(pokemon1, pokemon2)).toBe(51);
        });
    });

    describe("_choose_gender", () => {
        const species: PokemonSpecies = {
            id: "TEST",
            int_id: 1,
            growth_rate: "GROWTH_MEDIUM_FAST",
            gender_ratio: GenderRatio.GENDER_F12_5,
            base_stats: {
                hp: 45,
                attack: 49,
                defense: 49,
                speed: 45,
                special_attack: 65,
                special_defense: 65
            },
            egg_group1: EggGroup.EGG_MONSTER,
            egg_group2: EggGroup.EGG_MONSTER,
            tmhm_learnset: [],
            step_cycles_to_hatch: 0,
            type1: "GRASS",
            type2: "POISON",
            catch_rate: 45,
            base_exp: 64,
            unknown1: 0,
            unknown2: 0,
        } as PokemonSpecies;
        const dvs: DV = { attack: 0, defense: 0, speed: 0, special: 0, hp: 0 };

        it("should return the correct gender based on attack DV", () => {
            dvs.attack = 1;
            expect(_choose_gender(species, dvs)).toBe(PlayerGender.FEMALE);

            dvs.attack = 15;
            expect(_choose_gender(species, dvs)).toBe(PlayerGender.MALE);
        });
    });
});

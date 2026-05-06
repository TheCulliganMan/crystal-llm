import { StatusCondition } from "@pokecrystal/core/core/enums/battle";
import { Ability, EggGroup, GenderRatio, GrowthRate, PokemonType } from "@pokecrystal/core/core/enums";
import { Pokemon, PokemonSpecies } from "@pokecrystal/core/core/models";
import { PokemonData, PokemonSchema, toPokemon } from "@pokecrystal/core/core/models/pokemon";
import { applyPoisonToParty } from "./poison";

const DEFAULT_BASE_STATS = {
    hp: 30,
    attack: 15,
    defense: 10,
    speed: 10,
    special_attack: 10,
    special_defense: 10,
};

const ensureSpecies = (id: string): PokemonSpecies => {
    const normalized = id.toUpperCase();
    return {
        id: normalized,
        int_id: 1,
        base_stats: DEFAULT_BASE_STATS,
        type1: PokemonType.NORMAL,
        type2: PokemonType.NONE,
        catch_rate: 45,
        base_exp: 64,
        item1: undefined,
        item2: undefined,
        gender_ratio: GenderRatio.GENDER_F50,
        unknown1: 0,
        step_cycles_to_hatch: 5120,
        unknown2: 0,
        growth_rate: GrowthRate.GROWTH_MEDIUM_FAST,
        egg_group1: EggGroup.EGG_MONSTER,
        egg_group2: EggGroup.EGG_MONSTER,
        tmhm_learnset: [],
        ability: Ability.NONE,
        pic_size: 0,
        front_pic: 0,
        back_pic: 0,
        weight: 0,
        evolutions: null,
    };
};

const createTestPokemon = ({
    speciesId,
    nickname,
    hp = 10,
    maxHp,
    level = 5,
}: {
    speciesId: string;
    nickname?: string;
    hp?: number;
    maxHp?: number;
    level?: number;
}): Pokemon => {
    const species = ensureSpecies(speciesId);
    const pokemonData: PokemonData = PokemonSchema.parse({
        species,
        nickname: nickname ?? species.id,
        level,
        hp,
        max_hp: maxHp ?? hp,
        original_trainer_name: "PLAYER",
        original_trainer_id: 1,
        experience: 0,
        happiness: 50,
    });
    return toPokemon(pokemonData);
};

describe("applyPoisonToParty", () => {
    it("damages poisoned members by one hp and ignores safe slots", () => {
        const poisoned = createTestPokemon({ speciesId: "BULBASAUR", nickname: "Bulby", hp: 5 });
        poisoned.status = StatusCondition.POISON;
        const healthy = createTestPokemon({ speciesId: "PIDGEY", hp: 3 });

        const result = applyPoisonToParty([null, poisoned, healthy]);

        expect(poisoned.hp).toBe(4);
        expect(healthy.hp).toBe(3);
        expect(result.damagedNames).toEqual(["BULBY"]);
        expect(result.faintedNames).toEqual([]);
        expect(poisoned.status).toBe(StatusCondition.POISON);
    });

    it("captures fainted names, clears status, and does not emit damaged entries", () => {
        const fainting = createTestPokemon({ speciesId: "CHARMANDER", nickname: "Char", hp: 1 });
        fainting.status = StatusCondition.POISON;

        const result = applyPoisonToParty([fainting]);

        expect(fainting.hp).toBe(0);
        expect(fainting.status).toBeUndefined();
        expect(result.damagedNames).toEqual([]);
        expect(result.faintedNames).toEqual(["CHAR"]);
    });

    it("falls back to uppercase species ids and replaces underscores when no nickname is provided", () => {
        const mrMime = createTestPokemon({ speciesId: "mr_mime", nickname: "", hp: 3 });
        mrMime.status = StatusCondition.POISON;

        const result = applyPoisonToParty([mrMime]);

        expect(result.damagedNames).toEqual(["MR MIME"]);
    });

    it("supports legacy status payloads stored as lowercase strings", () => {
        const poisoned = createTestPokemon({ speciesId: "GENGAR", hp: 5 });
        (poisoned as { status: unknown }).status = "poison";

        const result = applyPoisonToParty([poisoned]);

        expect(poisoned.hp).toBe(4);
        expect(result.damagedNames).toEqual(["GENGAR"]);
    });

    it("supports legacy status payloads stored as named objects", () => {
        const poisoned = createTestPokemon({ speciesId: "EVOLUTION", hp: 5 });
        (poisoned as { status: unknown }).status = { name: "poison" };

        const result = applyPoisonToParty([poisoned]);

        expect(poisoned.hp).toBe(4);
        expect(result.damagedNames).toEqual(["EVOLUTION"]);
    });
});

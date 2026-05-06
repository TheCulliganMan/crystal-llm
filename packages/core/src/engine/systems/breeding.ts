import { Pokemon, PokemonData, PokemonSpecies, DV, LearnedMove, PokemonSchema, toPokemon } from "../../core/models";
import { EggGroup, GenderRatio, Stat, PlayerGender, MoveName, GrowthRate } from "../../core/enums";
import { GameState } from "../../core/state";
import { HardwareRNG } from "../games/rng";
import { calculateExperience } from "../experience";
import {
    loadMergedEggMovesSync,
    loadMergedEvolutionsSync,
    loadMergedLevelUpMovesSync,
    loadMergedMovesDataSync,
    loadMergedPokemonDataSync,
} from "../../core/content-packs";

export const EGG_LEVEL = 5;
const HATCHED_HAPPINESS = 0x78;

const memoize = <T>(fn: () => T): () => T => {
    let initialized = false;
    let result: T;
    return () => {
        if (!initialized) {
            result = fn();
            initialized = true;
        }
        return result;
    };
};

type EvolutionEntry = {
    species: string;
    evolutions: { species: string }[];
};

type LevelUpMoveEntry = { level: number; move: string };
type LevelUpMoves = Record<string, LevelUpMoveEntry[]>;
type EggMoves = Record<string, string[]>;
type MovesData = Record<MoveName, { pp: number }>;

const loadEvolutions = memoize(() => {
    return loadMergedEvolutionsSync() as EvolutionEntry[];
});

const loadPokemonData = memoize(() => {
    return Object.values(loadMergedPokemonDataSync()) as PokemonSpecies[];
});

const loadLevelUpMoves = memoize(() => {
    return loadMergedLevelUpMovesSync() as LevelUpMoves;
});

const loadEggMoves = memoize(() => {
    return loadMergedEggMovesSync() as EggMoves;
});

const loadMovesData = memoize(() => {
    return loadMergedMovesDataSync() as MovesData;
});
export class DayCareBreedingState {
    parent1: Pokemon;
    parent2: Pokemon;
    compatibility: number;
    steps_to_next_check: number;

    constructor(parent1: Pokemon, parent2: Pokemon, compatibility: number, steps_to_next_check: number) {
        this.parent1 = parent1;
        this.parent2 = parent2;
        this.compatibility = compatibility;
        this.steps_to_next_check = steps_to_next_check;
    }

    public static initialize(game_state: GameState, parent1: Pokemon, parent2: Pokemon): DayCareBreedingState {
        const compatibility = checkBreedingCompatibility(parent1, parent2);
        const stepsToNextCheck = _generate_initial_step_counter(game_state);
        return new DayCareBreedingState(
            parent1,
            parent2,
            _is_egg_roll_compatible_score(compatibility) ? compatibility : 0,
            stepsToNextCheck
        );
    }

    public advance_step(game_state: GameState): boolean {
        game_state.sram.day_care.steps_since_last_egg = (game_state.sram.day_care.steps_since_last_egg + 1) % 256;
        this.steps_to_next_check = (this.steps_to_next_check - 1) & 0xff;
        if (this.steps_to_next_check !== 0) {
            return false;
        }

        const rng = new HardwareRNG(game_state);
        this.steps_to_next_check = rng.randrange(256);
        const compatibility = this.compatibility;
        if (!_is_egg_roll_compatible_score(compatibility)) {
            return false;
        }
        const threshold = _compatibility_threshold(compatibility);
        if (threshold === 0) {
            return false;
        }

        return rng.randrange(256) < threshold;
    }
}

export function checkBreedingCompatibility(pokemon1: Pokemon, pokemon2: Pokemon): number {
    if (_is_no_eggs_species(pokemon1) || _is_no_eggs_species(pokemon2)) {
        return 0;
    }

    if (_is_ditto(pokemon1) && _is_ditto(pokemon2)) {
        return 0;
    }

    const egg_group_compatible = _have_compatible_egg_groups(pokemon1, pokemon2);

    if (!_is_ditto(pokemon1) && !_is_ditto(pokemon2)) {
        if (pokemon1.gender === null || pokemon2.gender === null) {
            return 0;
        }
        if (pokemon1.gender === pokemon2.gender) {
            return 0;
        }
        if (!egg_group_compatible) {
            return 0;
        }
    }

    if (_defense_and_special_match(pokemon1, pokemon2)) {
        return 255;
    }

    let base_compatibility: number;
    if (pokemon1.species.id === pokemon2.species.id) {
        base_compatibility = 254;
    } else if (egg_group_compatible || _is_ditto(pokemon1) || _is_ditto(pokemon2)) {
        base_compatibility = 128;
    } else {
        return 0;
    }

    if ((pokemon1.original_trainer_id & 0xffff) === (pokemon2.original_trainer_id & 0xffff)) {
        base_compatibility -= 77;
    }

    return base_compatibility;
}

export function createEgg(game_state: GameState, parent1: Pokemon, parent2: Pokemon, player_name: string, player_id: number): Pokemon {
    const mother = _determine_mother(parent1, parent2);
    const father = _determine_move_donor(parent1, parent2);

    const egg_species = _resolve_egg_species(game_state, mother.species);
    const dv_donor = _determine_dv_donor(parent1, parent2);
    const dvs = _generate_egg_dvs(game_state, dv_donor);

    const move_names = _default_moves_for_species(egg_species.id, EGG_LEVEL);
    const inherited = _compute_inherited_moves(
        egg_species,
        father,
        mother,
        move_names
    );
    const learned_moves = _to_learned_moves(inherited);

    const experience = _experience_at_level(egg_species, EGG_LEVEL);
    const max_hp = _calculate_hp_stat(egg_species.base_stats.hp, dvs.hp, EGG_LEVEL);

    return toPokemon(PokemonSchema.parse({
        species: egg_species,
        nickname: "EGG",
        gender: undefined,
        item: undefined,
        moves: learned_moves,
        level: EGG_LEVEL,
        hp: max_hp,
        max_hp,
        dvs,
        status: undefined,
        original_trainer_name: player_name,
        original_trainer_id: player_id,
        experience,
        happiness: egg_species.step_cycles_to_hatch,
        stat_boosts: {
            [Stat.ATTACK]: 0,
            [Stat.DEFENSE]: 0,
            [Stat.SPEED]: 0,
            [Stat.SPECIAL_ATTACK]: 0,
            [Stat.SPECIAL_DEFENSE]: 0,
            [Stat.ACCURACY]: 0,
            [Stat.EVASION]: 0,
        },
    }));
}

export function hatchEgg(game_state: GameState, egg: PokemonData): PokemonData {
    egg.nickname = egg.species.id;
    egg.gender = _choose_gender(egg.species, egg.dvs) ?? undefined;
    const max_hp = _calculate_hp_stat(egg.species.base_stats.hp, egg.dvs.hp, egg.level);
    egg.max_hp = max_hp;
    egg.hp = max_hp;
    egg.happiness = HATCHED_HAPPINESS;
    return egg;
}

function _is_no_eggs_species(pokemon: Pokemon): boolean {
    return (
        pokemon.species.egg_group1 === EggGroup.EGG_NONE &&
        pokemon.species.egg_group2 === EggGroup.EGG_NONE
    );
}

function _is_ditto(pokemon: Pokemon): boolean {
    return pokemon.species.id === "DITTO";
}

function _have_compatible_egg_groups(pokemon1: Pokemon, pokemon2: Pokemon): boolean {
    const groups1 = new Set([pokemon1.species.egg_group1, pokemon1.species.egg_group2]);
    const groups2 = new Set([pokemon2.species.egg_group1, pokemon2.species.egg_group2]);
    groups1.delete(EggGroup.EGG_NONE);
    groups2.delete(EggGroup.EGG_NONE);
    for (const group of groups1) {
        if (groups2.has(group)) {
            return true;
        }
    }
    return false;
}

function _defense_and_special_match(pokemon1: Pokemon, pokemon2: Pokemon): boolean {
    return (
        pokemon1.dvs.defense === pokemon2.dvs.defense &&
        (pokemon1.dvs.special & 0b111) === (pokemon2.dvs.special & 0b111)
    );
}

function _generate_initial_step_counter(game_state: GameState): number {
    const rng = new HardwareRNG(game_state);
    while (true) {
        const value = rng.randrange(256);
        if (value >= 150) {
            return value;
        }
    }
}

// ASM mapping:
// - pokecrystal_disassembly/engine/pokemon/breeding.asm: DayCareStep
// - pokecrystal_disassembly/engine/events/daycare.asm: DayCare_InitBreeding
const _EGG_CHANCE_TABLE: [number, number][] = [
    [230, 80], // 31 percent + 1
    [170, 40], // 16 percent
    [110, 30], // 12 percent
    [0, 10], // 4 percent
];

export function _compatibility_threshold(score: number): number {
    if (!_is_egg_roll_compatible_score(score)) {
        return 0;
    }
    for (const [threshold, value] of _EGG_CHANCE_TABLE) {
        if (score >= threshold) {
            return value;
        }
    }
    return 0;
}

function _is_egg_roll_compatible_score(score: number): boolean {
    return score > 0 && score < 255;
}

function _determine_mother(parent1: Pokemon, parent2: Pokemon): Pokemon {
    if (_is_ditto(parent1)) {
        return parent2;
    }
    if (_is_ditto(parent2)) {
        return parent1;
    }
    if (parent1.gender === PlayerGender.FEMALE) {
        return parent1;
    }
    return parent2;
}

function _determine_move_donor(parent1: Pokemon, parent2: Pokemon): Pokemon {
    if (_is_ditto(parent1)) {
        if (parent2.gender === PlayerGender.MALE || parent2.gender === null) {
            return parent2;
        }
        return parent1;
    }
    if (_is_ditto(parent2)) {
        if (parent1.gender === PlayerGender.MALE || parent1.gender === null) {
            return parent1;
        }
        return parent2;
    }
    if (parent1.gender === PlayerGender.MALE) {
        return parent1;
    }
    return parent2;
}

function _determine_dv_donor(parent1: Pokemon, parent2: Pokemon): Pokemon {
    if (_is_ditto(parent1) && !_is_ditto(parent2)) {
        if (parent2.gender === PlayerGender.MALE || parent2.gender === null) {
            return parent2;
        }
        return parent1;
    }
    if (_is_ditto(parent2) && !_is_ditto(parent1)) {
        if (parent1.gender === PlayerGender.MALE || parent1.gender === null) {
            return parent1;
        }
        return parent2;
    }
    if (parent1.gender === PlayerGender.MALE) {
        return parent1;
    }
    return parent2;
}

function _resolve_egg_species(game_state: GameState, species: PokemonSpecies): PokemonSpecies {
    let baby_id = species.id;
    const pre_map = _pre_evolution_map();
    while (pre_map.has(baby_id)) {
        baby_id = pre_map.get(baby_id)!;
    }
    const rng = new HardwareRNG(game_state);
    if (baby_id === "NIDORAN_F") {
        if (rng.randrange(100) >= 50) {
            baby_id = "NIDORAN_M";
        }
    }
    const species_map = _species_data();
    return species_map.get(baby_id)!;
}

function _generate_egg_dvs(game_state: GameState, parent: Pokemon): DV {
    const rng = new HardwareRNG(game_state);
    let first_byte = rng.randrange(256);
    let second_byte = rng.randrange(256);

    const defense = parent.dvs.defense & 0xf;
    const special_low = parent.dvs.special & 0x7;

    first_byte = (first_byte & 0xf0) | defense;
    second_byte = (second_byte & 0xf8) | special_low;

    const attack = (first_byte >> 4) & 0xf;
    const defense_dv = first_byte & 0xf;
    const speed = (second_byte >> 4) & 0xf;
    const special = second_byte & 0xf;

    let hp = 0;
    if (attack & 1) {
        hp |= 0x8;
    }
    if (defense_dv & 1) {
        hp |= 0x4;
    }
    if (speed & 1) {
        hp |= 0x2;
    }
    if (special & 1) {
        hp |= 0x1;
    }

    return { attack, defense: defense_dv, speed, special, hp };
}

function _experience_at_level(species: PokemonSpecies, level: number): number {
    return calculateExperience(species.growth_rate as GrowthRate, level);
}

function _calculate_hp_stat(base_hp: number, hp_dv: number, level: number): number {
    return Math.floor(((base_hp + hp_dv) * 2 * level) / 100) + level + 10;
}

const _pre_evolution_map = memoize((): Map<string, string> => {
    const mapping = new Map<string, string>();
    for (const entry of loadEvolutions()) {
        const species = entry.species;
        for (const evo of entry.evolutions) {
            mapping.set(evo.species, species);
        }
    }
    return mapping;
});

const _species_data = memoize((): Map<string, PokemonSpecies> => {
    const mapping = new Map<string, PokemonSpecies>();
    for (const entry of loadPokemonData()) {
        mapping.set(entry.id, entry as PokemonSpecies);
    }
    return mapping;
});

function _default_moves_for_species(species_id: string, level: number): MoveName[] {
    const moves: MoveName[] = [];
    for (const entry of loadLevelUpMoves()[species_id]) {
        if (entry.level > level) {
            break;
        }
        const move = entry.move as MoveName;
        if (moves.includes(move)) {
            continue;
        }
        moves.push(move);
        if (moves.length > 4) {
            moves.shift();
        }
    }
    return moves;
}

function _compute_inherited_moves(
    baby_species: PokemonSpecies,
    father: Pokemon,
    mother: Pokemon,
    existing: MoveName[]
): MoveName[] {
    const moves = [...existing];
    const eggMoves = new Set(loadEggMoves()[baby_species.id]);
    const mother_moves = new Set(
        mother.moves
            .filter((move): move is LearnedMove => move !== null)
            .map((move) => move.name)
    );
    const level_up_moves_set = new Set(loadLevelUpMoves()[baby_species.id].map((entry) => entry.move));
    const tm_moves = new Set(baby_species.tmhm_learnset);

    for (const learned of father.moves) {
        if (!learned) {
            continue;
        }
        const move = learned.name;
        if (moves.includes(move)) {
            continue;
        }
        if (eggMoves.has(move)) {
            _add_move(moves, move);
            continue;
        }
        if (mother_moves.has(move) && level_up_moves_set.has(move)) {
            _add_move(moves, move);
            continue;
        }
        if (tm_moves.has(move)) {
            _add_move(moves, move);
        }
    }

    return moves;
}

function _add_move(moves: MoveName[], move: MoveName): void {
    moves.push(move);
    if (moves.length > 4) {
        moves.shift();
    }
}

function _to_learned_moves(moves: MoveName[]): LearnedMove[] {
    const pp_map = _move_pp();
    return moves.map((move) => ({
        name: move,
        current_pp: pp_map.get(move)!,
    }));
}

const _move_pp = memoize((): Map<MoveName, number> => {
    type MovePPEntry = { pp: number };
    const movesLookup = loadMovesData();
    const mapping = new Map<MoveName, number>();
    for (const name of Object.keys(movesLookup) as MoveName[]) {
        mapping.set(name, movesLookup[name].pp);
    }
    return mapping;
});

const GENDER_THRESHOLDS: { [key in GenderRatio]?: number } = {
    [GenderRatio.GENDER_F12_5]: 2,
    [GenderRatio.GENDER_F25]: 4,
    [GenderRatio.GENDER_F50]: 8,
    [GenderRatio.GENDER_F75]: 12,
};

export function _choose_gender(species: PokemonSpecies, dvs: DV): PlayerGender | null {
    if (species.gender_ratio === GenderRatio.GENDER_UNKNOWN) {
        return null;
    }
    if (species.gender_ratio === GenderRatio.GENDER_F100) {
        return PlayerGender.FEMALE;
    }
    if (species.gender_ratio === GenderRatio.GENDER_F0) {
        return PlayerGender.MALE;
    }

    const threshold = GENDER_THRESHOLDS[species.gender_ratio];
    if (threshold === undefined) {
        return PlayerGender.MALE;
    }

    const attack_dv = dvs.attack & 0xf;
    return attack_dv < threshold ? PlayerGender.FEMALE : PlayerGender.MALE;
}

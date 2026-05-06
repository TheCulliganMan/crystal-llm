
import { GameState } from "./state";
import { NUM_POKEMON, NUM_UNOWN, Pokemon as PokemonId } from "./constants";
import type { Pokemon } from "./models";

const validateSpeciesId = (speciesId: number): number => {
    if (!Number.isInteger(speciesId) || speciesId < 1 || speciesId > NUM_POKEMON) {
        throw new Error(`Invalid Pok\u00e9dex species id ${speciesId}`);
    }
    return speciesId;
};

export function setPokedexFlag(gameState: GameState, speciesId: number, flag: 'seen' | 'owned'): void {
    validateSpeciesId(speciesId);
    const byte = Math.floor((speciesId - 1) / 8);
    const bit = (speciesId - 1) % 8;

    if (flag === 'seen') {
        gameState.sram.pokedex_seen[byte] |= 1 << bit;
    } else {
        gameState.sram.pokedex_owned[byte] |= 1 << bit;
    }
}

export function getPokedexFlag(gameState: GameState, speciesId: number, flag: 'seen' | 'owned'): boolean {
    validateSpeciesId(speciesId);
    const byte = Math.floor((speciesId - 1) / 8);
    const bit = (speciesId - 1) % 8;

    if (flag === 'seen') {
        return (gameState.sram.pokedex_seen[byte] & (1 << bit)) !== 0;
    } else {
        return (gameState.sram.pokedex_owned[byte] & (1 << bit)) !== 0;
    }
}

export function recordPokedexSeen(gameState: GameState, species: { int_id: number }): void {
    setPokedexFlag(gameState, species.int_id, 'seen');
}

export const pokedexFlagSet = (flags: number[]): Set<number> => {
    const set = new Set<number>();
    for (let speciesId = 1; speciesId <= NUM_POKEMON; speciesId += 1) {
        const byte = Math.floor((speciesId - 1) / 8);
        const bit = (speciesId - 1) % 8;
        if (((flags[byte] ?? 0) & (1 << bit)) !== 0) {
            set.add(speciesId);
        }
    }
    return set;
};

export const countPokedexEntries = (flags: number[]): number => {
    return pokedexFlagSet(flags).size;
};

type SpeciesRef = number | { int_id?: number } | Pokemon;

const isRecord = (value: unknown): value is Record<string, unknown> => {
    return typeof value === "object" && value !== null;
};

const extractSpeciesId = (value: unknown): number | null => {
    if (!isRecord(value)) {
        return null;
    }
    if (isRecord(value.species) && typeof value.species.int_id === "number") {
        return value.species.int_id;
    }
    if (typeof value.int_id === "number") {
        return value.int_id;
    }
    return null;
};

const resolveSpeciesId = (species: SpeciesRef): number => {
    if (typeof species === "number") {
        return validateSpeciesId(species);
    }
    const resolved = extractSpeciesId(species);
    if (resolved !== null) {
        const value = Number(resolved);
        if (!Number.isFinite(value)) {
            throw new Error("Resolved species id must be a number");
        }
        return validateSpeciesId(value);
    }
    throw new Error("Species reference must include an int_id");
};

const unownLetterFromDvs = (dvs: { attack?: number; defense?: number; speed?: number; special?: number }): number => {
    const attack = Number(dvs.attack ?? 0) & 0x3;
    const defense = Number(dvs.defense ?? 0) & 0x3;
    const speed = Number(dvs.speed ?? 0) & 0x3;
    const special = Number(dvs.special ?? 0) & 0x3;
    const value = (attack << 6) | (defense << 4) | (speed << 2) | special;
    return Math.floor(value / 10) + 1;
};

const updateUnownDex = (gameState: GameState, species: SpeciesRef, speciesId: number): void => {
    if (speciesId !== PokemonId.UNOWN) {
        return;
    }
    const dvs = isRecord(species) && "dvs" in species ? (species as { dvs?: Record<string, unknown> }).dvs : undefined;
    if (!isRecord(dvs)) {
        return;
    }
    const letterValue = unownLetterFromDvs(dvs as { attack?: number; defense?: number; speed?: number; special?: number });
    if (letterValue < 1 || letterValue > NUM_UNOWN) {
        return;
    }
    const unownDex = gameState.wram.wUnownDex;
    if (unownDex.includes(letterValue)) {
        return;
    }
    for (let index = 0; index < unownDex.length; index += 1) {
        if (unownDex[index] === 0) {
            unownDex[index] = letterValue;
            break;
        }
    }
    gameState.wram.wUnownLetter = letterValue;
};

export function recordPokedexCaught(gameState: GameState, species: SpeciesRef): void {
    const speciesId = resolveSpeciesId(species);
    recordPokedexSeen(gameState, { int_id: speciesId });
    setPokedexFlag(gameState, speciesId, 'owned');
    const caught = gameState.sram.pokedex_caught as Set<number>;
    caught.add(speciesId);
    updateUnownDex(gameState, species, speciesId);
}

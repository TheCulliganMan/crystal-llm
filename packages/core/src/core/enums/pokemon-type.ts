import { z } from 'zod';

export enum PokemonType {
    NORMAL = "NORMAL",
    FIGHTING = "FIGHTING",
    FLYING = "FLYING",
    POISON = "POISON",
    GROUND = "GROUND",
    ROCK = "ROCK",
    BUG = "BUG",
    GHOST = "GHOST",
    STEEL = "STEEL",
    FIRE = "FIRE",
    WATER = "WATER",
    GRASS = "GRASS",
    ELECTRIC = "ELECTRIC",
    PSYCHIC_TYPE = "PSYCHIC_TYPE",
    ICE = "ICE",
    DRAGON = "DRAGON",
    DARK = "DARK",
    CURSE_TYPE = "CURSE_TYPE",
    NONE = "NONE",
    UNKNOWN = "UNKNOWN",
}

export const PokemonTypeSchema = z.nativeEnum(PokemonType);

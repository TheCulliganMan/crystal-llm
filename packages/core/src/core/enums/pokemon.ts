import { z } from 'zod';

export const PokemonType = {
    NORMAL: "NORMAL",
    FIGHTING: "FIGHTING",
    FLYING: "FLYING",
    POISON: "POISON",
    GROUND: "GROUND",
    ROCK: "ROCK",
    BUG: "BUG",
    GHOST: "GHOST",
    STEEL: "STEEL",
    FIRE: "FIRE",
    WATER: "WATER",
    GRASS: "GRASS",
    ELECTRIC: "ELECTRIC",
    PSYCHIC_TYPE: "PSYCHIC_TYPE",
    ICE: "ICE",
    DRAGON: "DRAGON",
    DARK: "DARK",
    CURSE_TYPE: "CURSE_TYPE",
    NONE: "NONE",
    UNKNOWN: "UNKNOWN"
} as const;
export type PokemonType = keyof typeof PokemonType;
export const PokemonTypeSchema = z.nativeEnum(PokemonType);

export const Stat = {
    HP: "HP",
    ATTACK: "ATTACK",
    DEFENSE: "DEFENSE",
    SPEED: "SPEED",
    SPECIAL_ATTACK: "SPECIAL_ATTACK",
    SPECIAL_DEFENSE: "SPECIAL_DEFENSE",
    ACCURACY: "ACCURACY",
    EVASION: "EVASION"
} as const;
export type Stat = keyof typeof Stat;
export const StatSchema = z.nativeEnum(Stat);

export const GrowthRate = {
    GROWTH_MEDIUM_FAST: "GROWTH_MEDIUM_FAST",
    GROWTH_SLIGHTLY_FAST: "GROWTH_SLIGHTLY_FAST",
    GROWTH_SLIGHTLY_SLOW: "GROWTH_SLIGHTLY_SLOW",
    GROWTH_MEDIUM_SLOW: "GROWTH_MEDIUM_SLOW",
    GROWTH_FAST: "GROWTH_FAST",
    GROWTH_SLOW: "GROWTH_SLOW",
    GROWTH_ERRATIC: "GROWTH_ERRATIC",
    GROWTH_FLUCTUATING: "GROWTH_FLUCTUATING"
} as const;
export type GrowthRate = keyof typeof GrowthRate;
export const GrowthRateSchema = z.nativeEnum(GrowthRate);

export const EggGroup = {
    EGG_NONE: "EGG_NONE",
    EGG_MONSTER: "EGG_MONSTER",
    EGG_WATER_1: "EGG_WATER_1",
    EGG_BUG: "EGG_BUG",
    EGG_FLYING: "EGG_FLYING",
    EGG_GROUND: "EGG_GROUND",
    EGG_FAIRY: "EGG_FAIRY",
    EGG_PLANT: "EGG_PLANT",
    EGG_HUMANSHAPE: "EGG_HUMANSHAPE",
    EGG_WATER_3: "EGG_WATER_3",
    EGG_MINERAL: "EGG_MINERAL",
    EGG_INDETERMINATE: "EGG_INDETERMINATE",
    EGG_WATER_2: "EGG_WATER_2",
    EGG_DITTO: "EGG_DITTO",
    EGG_DRAGON: "EGG_DRAGON",
    EGG_UNDISCOVERED: "EGG_UNDISCOVERED"
} as const;
export type EggGroup = keyof typeof EggGroup;
export const EggGroupSchema = z.nativeEnum(EggGroup);

export const MonType = {
    PARTYMON: 0,
    OTPARTYMON: 1,
    BOXMON: 2,
    TEMPMON: 3,
    WILDMON: 4,
    MONTYPE_NORMAL: 5, // Placeholder for the other values I saw earlier
    MONTYPE_GRASS: 6,
    MONTYPE_WATER: 7,
    MONTYPE_FIRE: 8,
    MONTYPE_ELECTRIC: 9,
    MONTYPE_ICE: 10,
    MONTYPE_FLYING: 11,
    MONTYPE_BUG: 12,
    MONTYPE_POISON: 13,
    MONTYPE_GROUND: 14,
    MONTYPE_ROCK: 15,
    MONTYPE_FIGHTING: 16,
    MONTYPE_PSYCHIC: 17,
    MONTYPE_GHOST: 18,
    MONTYPE_DRAGON: 19
} as const;
export type MonType = typeof MonType[keyof typeof MonType];
export const MonTypeSchema = z.nativeEnum(MonType);

export const GenderRatio = {
    GENDER_F0: 0,
    GENDER_F12_5: 31,
    GENDER_F25: 63,
    GENDER_F50: 127,
    GENDER_F75: 191,
    GENDER_F100: 254,
    GENDER_UNKNOWN: 255
} as const;
export type GenderRatio = typeof GenderRatio[keyof typeof GenderRatio];
export const GenderRatioSchema = z.nativeEnum(GenderRatio);

export const Ability = {
    NONE: "NONE",
    GUTS: "GUTS",
    LIGHT_BALL: "LIGHT_BALL",
    THICK_CLUB: "THICK_CLUB",
} as const;
export type Ability = keyof typeof Ability;
export const AbilitySchema = z.nativeEnum(Ability);

export const EvolutionMethod = {
    LEVEL: "LEVEL",
    ITEM: "ITEM",
    HAPPINESS: "HAPPINESS",
    TRADE: "TRADE",
    STAT: "STAT",
    EVOLVE_LEVEL: "EVOLVE_LEVEL",
    EVOLVE_ITEM: "EVOLVE_ITEM",
    EVOLVE_TRADE: "EVOLVE_TRADE",
    EVOLVE_HAPPINESS: "EVOLVE_HAPPINESS"
} as const;
export type EvolutionMethod = keyof typeof EvolutionMethod;
export const EvolutionMethodSchema = z.nativeEnum(EvolutionMethod);

export const PlayerGender = {
    MALE: 0,
    FEMALE: 1
} as const;
export type PlayerGender = typeof PlayerGender[keyof typeof PlayerGender];
export const PlayerGenderSchema = z.nativeEnum(PlayerGender);
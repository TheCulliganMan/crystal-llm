import { z } from 'zod';
export declare const evolutionMethodSchema: z.ZodEnum<{
    LEVEL: "LEVEL";
    ITEM: "ITEM";
    HAPPINESS: "HAPPINESS";
    TRADE: "TRADE";
    STAT: "STAT";
}>;
export type EvolutionMethod = z.infer<typeof evolutionMethodSchema>;
export declare const evolutionDataSchema: z.ZodObject<{
    method: z.ZodEnum<{
        LEVEL: "LEVEL";
        ITEM: "ITEM";
        HAPPINESS: "HAPPINESS";
        TRADE: "TRADE";
        STAT: "STAT";
    }>;
    level: z.ZodOptional<z.ZodNumber>;
    item: z.ZodOptional<z.ZodString>;
    held_item: z.ZodOptional<z.ZodString>;
    happiness: z.ZodOptional<z.ZodString>;
    stat_ratio: z.ZodOptional<z.ZodString>;
    species: z.ZodString;
}, z.core.$strip>;
export type EvolutionData = z.infer<typeof evolutionDataSchema>;
export declare const pokemonEvolutionDataSchema: z.ZodObject<{
    species: z.ZodString;
    evolutions: z.ZodArray<z.ZodObject<{
        method: z.ZodEnum<{
            LEVEL: "LEVEL";
            ITEM: "ITEM";
            HAPPINESS: "HAPPINESS";
            TRADE: "TRADE";
            STAT: "STAT";
        }>;
        level: z.ZodOptional<z.ZodNumber>;
        item: z.ZodOptional<z.ZodString>;
        held_item: z.ZodOptional<z.ZodString>;
        happiness: z.ZodOptional<z.ZodString>;
        stat_ratio: z.ZodOptional<z.ZodString>;
        species: z.ZodString;
    }, z.core.$strip>>;
}, z.core.$strip>;
export type PokemonEvolutionData = z.infer<typeof pokemonEvolutionDataSchema>;
export declare const evolutionData: PokemonEvolutionData[];

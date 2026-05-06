import { z } from 'zod';
export declare const pokedexDataSchema: z.ZodObject<{
    species: z.ZodString;
    classification: z.ZodString;
    height: z.ZodNumber;
    weight: z.ZodNumber;
    text: z.ZodString;
}, z.core.$strip>;
export type PokedexData = z.infer<typeof pokedexDataSchema>;
export declare const pokedexData: PokedexData[];

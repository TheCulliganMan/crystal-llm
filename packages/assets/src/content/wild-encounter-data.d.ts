import { z } from 'zod';
export declare const WildEncounterSchema: z.ZodObject<{
    level: z.ZodNumber;
    species: z.ZodString;
}, z.core.$strip>;
export type WildEncounter = z.infer<typeof WildEncounterSchema>;
export declare const WildEncounterTableSchema: z.ZodObject<{
    morning: z.ZodArray<z.ZodObject<{
        level: z.ZodNumber;
        species: z.ZodString;
    }, z.core.$strip>>;
    day: z.ZodArray<z.ZodObject<{
        level: z.ZodNumber;
        species: z.ZodString;
    }, z.core.$strip>>;
    night: z.ZodArray<z.ZodObject<{
        level: z.ZodNumber;
        species: z.ZodString;
    }, z.core.$strip>>;
}, z.core.$strip>;
export type WildEncounterTable = z.infer<typeof WildEncounterTableSchema>;
export declare const WildEncounterDataSchema: z.ZodObject<{
    map_name: z.ZodString;
    grass_rates: z.ZodNullable<z.ZodOptional<z.ZodRecord<z.ZodString, z.ZodNumber>>>;
    water_rate: z.ZodNullable<z.ZodOptional<z.ZodNumber>>;
    grass: z.ZodNullable<z.ZodOptional<z.ZodObject<{
        morning: z.ZodArray<z.ZodObject<{
            level: z.ZodNumber;
            species: z.ZodString;
        }, z.core.$strip>>;
        day: z.ZodArray<z.ZodObject<{
            level: z.ZodNumber;
            species: z.ZodString;
        }, z.core.$strip>>;
        night: z.ZodArray<z.ZodObject<{
            level: z.ZodNumber;
            species: z.ZodString;
        }, z.core.$strip>>;
    }, z.core.$strip>>>;
    water: z.ZodNullable<z.ZodOptional<z.ZodObject<{
        morning: z.ZodArray<z.ZodObject<{
            level: z.ZodNumber;
            species: z.ZodString;
        }, z.core.$strip>>;
        day: z.ZodArray<z.ZodObject<{
            level: z.ZodNumber;
            species: z.ZodString;
        }, z.core.$strip>>;
        night: z.ZodArray<z.ZodObject<{
            level: z.ZodNumber;
            species: z.ZodString;
        }, z.core.$strip>>;
    }, z.core.$strip>>>;
}, z.core.$strip>;
export type WildEncounterData = z.infer<typeof WildEncounterDataSchema>;
export declare const wildEncounterData: WildEncounterData[];

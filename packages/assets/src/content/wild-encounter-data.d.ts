import { z } from 'zod';
export declare const WildEncounterSchema: z.ZodObject<{
    level: z.ZodNumber;
    species: z.ZodString;
}, z.core.$strict>;
export type WildEncounter = z.infer<typeof WildEncounterSchema>;
export declare const WildEncounterTableSchema: z.ZodObject<{
    morning: z.ZodArray<z.ZodObject<{
        level: z.ZodNumber;
        species: z.ZodString;
    }, z.core.$strict>>;
    day: z.ZodArray<z.ZodObject<{
        level: z.ZodNumber;
        species: z.ZodString;
    }, z.core.$strict>>;
    night: z.ZodArray<z.ZodObject<{
        level: z.ZodNumber;
        species: z.ZodString;
    }, z.core.$strict>>;
}, z.core.$strict>;
export type WildEncounterTable = z.infer<typeof WildEncounterTableSchema>;
export declare const WildEncounterDataSchema: z.ZodObject<{
    map_name: z.ZodString;
    grass_rates: z.ZodNullable<z.ZodRecord<z.ZodString, z.ZodNumber>>;
    water_rate: z.ZodNullable<z.ZodNumber>;
    grass: z.ZodNullable<z.ZodObject<{
        morning: z.ZodArray<z.ZodObject<{
            level: z.ZodNumber;
            species: z.ZodString;
        }, z.core.$strict>>;
        day: z.ZodArray<z.ZodObject<{
            level: z.ZodNumber;
            species: z.ZodString;
        }, z.core.$strict>>;
        night: z.ZodArray<z.ZodObject<{
            level: z.ZodNumber;
            species: z.ZodString;
        }, z.core.$strict>>;
    }, z.core.$strict>>;
    water: z.ZodNullable<z.ZodObject<{
        morning: z.ZodArray<z.ZodObject<{
            level: z.ZodNumber;
            species: z.ZodString;
        }, z.core.$strict>>;
        day: z.ZodArray<z.ZodObject<{
            level: z.ZodNumber;
            species: z.ZodString;
        }, z.core.$strict>>;
        night: z.ZodArray<z.ZodObject<{
            level: z.ZodNumber;
            species: z.ZodString;
        }, z.core.$strict>>;
    }, z.core.$strict>>;
}, z.core.$strict>;
export type WildEncounterData = z.infer<typeof WildEncounterDataSchema>;
export declare const wildEncounterData: WildEncounterData[];

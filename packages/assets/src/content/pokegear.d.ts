import { z } from 'zod';
export declare const LandmarkEntrySchema: z.ZodObject<{
    id: z.ZodNumber;
    constant: z.ZodString;
    label: z.ZodString;
    name: z.ZodString;
    x: z.ZodNumber;
    y: z.ZodNumber;
    region: z.ZodString;
}, z.core.$strip>;
export declare const PokegearLandmarkPayloadSchema: z.ZodObject<{
    landmarks: z.ZodArray<z.ZodObject<{
        id: z.ZodNumber;
        constant: z.ZodString;
        label: z.ZodString;
        name: z.ZodString;
        x: z.ZodNumber;
        y: z.ZodNumber;
        region: z.ZodString;
    }, z.core.$strip>>;
    map_to_landmark: z.ZodRecord<z.ZodString, z.ZodString>;
}, z.core.$strip>;
export type LandmarkEntry = z.infer<typeof LandmarkEntrySchema>;
export declare function loadPokegearPayloadSync(): {
    landmarks: {
        id: number;
        constant: string;
        label: string;
        name: string;
        x: number;
        y: number;
        region: string;
    }[];
    map_to_landmark: Record<string, string>;
};
export declare function getPokegearLandmarks(): Promise<LandmarkEntry[]>;
export declare function getPokegearLandmarksSync(): LandmarkEntry[];
export declare function getMapToLandmark(): Promise<Record<string, string>>;
export declare function getMapToLandmarkSync(): Record<string, string>;

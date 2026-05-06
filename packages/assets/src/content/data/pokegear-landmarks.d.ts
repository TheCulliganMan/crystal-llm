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
export type LandmarkEntry = z.infer<typeof LandmarkEntrySchema>;
export declare const POKEGEAR_LANDMARKS: LandmarkEntry[];
export declare const MAP_TO_LANDMARK: Record<string, string>;

import { z } from 'zod';
export declare const RadioChannelFrequencySchema: z.ZodObject<{
    raw: z.ZodNumber;
    frequency: z.ZodNumber;
    handler: z.ZodString;
}, z.core.$strip>;
export type RadioChannelFrequency = z.infer<typeof RadioChannelFrequencySchema>;
export declare const RADIO_CHANNEL_FREQUENCIES: RadioChannelFrequency[];
export declare const RadioChannelConstantSchema: z.ZodObject<{
    constant: z.ZodString;
    id: z.ZodNumber;
    song: z.ZodString;
}, z.core.$strip>;
export type RadioChannelConstant = z.infer<typeof RadioChannelConstantSchema>;
export declare const RADIO_CHANNEL_CONSTANTS: RadioChannelConstant[];
export declare const RADIO_STATION_NAMES: Record<string, string>;

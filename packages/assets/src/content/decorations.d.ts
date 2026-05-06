import { z } from 'zod';
declare const DecorationAttributeSchema: z.ZodObject<{
    index: z.ZodNumber;
    deco_type: z.ZodString;
    name_token: z.ZodString;
    action_token: z.ZodString;
    event_flag: z.ZodString;
    sprite_token: z.ZodString;
    sprite_value: z.ZodNullable<z.ZodNumber>;
}, z.core.$strip>;
export type DecorationAttribute = z.infer<typeof DecorationAttributeSchema>;
export declare const decorations: DecorationAttribute[];
export declare const decorationIdMap: {
    [key: number]: string;
};
export declare function getDecorationConstant(decoId: number): string | undefined;
export {};

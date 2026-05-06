import { z } from 'zod';

export const DateSchema = z.object({
  year: z.number().int().default(2000),
  month: z.number().int().default(1),
  day: z.number().int().default(1),
});

export type GameDate = z.infer<typeof DateSchema>;

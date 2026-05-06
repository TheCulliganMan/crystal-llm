import { z } from 'zod';

export const TimeSchema = z.object({
  day: z.number().int().default(0),
  hour: z.number().int().default(0),
  minute: z.number().int().default(0),
  second: z.number().int().default(0),
});

export type Time = z.infer<typeof TimeSchema>;

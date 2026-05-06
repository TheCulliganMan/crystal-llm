import { z } from 'zod';

export const PokemonSchema = z.object({
  id: z.number(),
  name: z.string(),
  // ... more fields later
});
export type Pokemon = z.infer<typeof PokemonSchema>;

export const MoveSchema = z.object({
  id: z.number(),
  name: z.string(),
  // ... more fields later
});
export type Move = z.infer<typeof MoveSchema>;

export const ItemSchema = z.object({
  id: z.number(),
  name: z.string(),
  // ... more fields later
});
export type Item = z.infer<typeof ItemSchema>;

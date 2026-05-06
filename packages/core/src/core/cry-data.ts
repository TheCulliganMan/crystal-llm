import { z } from "zod";

export const PokemonCrySchema = z.object({
  species: z.string(),
  cry: z.string(),
  pitch: z.number(),
  length: z.number(),
});

export type PokemonCry = z.infer<typeof PokemonCrySchema>;

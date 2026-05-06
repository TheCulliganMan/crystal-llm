import { z } from "zod";
import { PokemonSchema } from "../models";
import { AILayer } from "../enums";

export const TrainerSchema = z.object({
  name: z.string(),
  trainer_id: z.string().default(""),
  trainer_class: z.string().default(""),
  party: z.array(PokemonSchema),
  win_quote: z.string(),
  lose_quote: z.string(),
  items: z.array(z.string().nullable()).default([]),
  base_reward: z.number().default(0),
  ai_move_flags: z.number().default(0),
  ai_item_switch_flags: z.number().default(0),
  encounter_music: z.string().default(""),
  ai_layers: z.array(z.nativeEnum(AILayer)).default([]),
});
export type Trainer = z.infer<typeof TrainerSchema>;

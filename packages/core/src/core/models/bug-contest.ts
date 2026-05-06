import { z } from 'zod';
import { Pokemon, PokemonSchema } from "./pokemon";

export const BugContestWinner = z.object({
  winner_id: z.number().default(0),
  mon_species: z.string().default(''),
  score: z.number().default(0),
});
export type BugContestWinner = z.infer<typeof BugContestWinner>;

const createDefaultWinner = () => ({ winner_id: 0, mon_species: '', score: 0 });

export const BugContestResults = z.object({
  first_place: BugContestWinner.default(createDefaultWinner),
  second_place: BugContestWinner.default(createDefaultWinner),
  third_place: BugContestWinner.default(createDefaultWinner),
  temp_winner: BugContestWinner.default(createDefaultWinner),
  winner_name: z.string().default(''),
});
export type BugContestResults = z.infer<typeof BugContestResults>;

export const BugContestTimer = z.object({
  mins_remaining: z.number().default(0),
  secs_remaining: z.number().default(0),
  start_time: z.array(z.number()).default([0, 0, 0, 0]),
  started_at: z.number().nullable().default(null),
});
export type BugContestTimer = z.infer<typeof BugContestTimer>;

export const BugContestState = z.object({
  timer_active: z.boolean().default(false),
  park_balls_remaining: z.number().default(20),
  caught_species: z.string().nullable().default(null),
  caught_level: z.number().nullable().default(null),
  pending_caught_mon: PokemonSchema.nullable().default(null),
  party_backup: z.array(PokemonSchema.nullable()).default([]),
});
export type BugContestState = z.infer<typeof BugContestState>;

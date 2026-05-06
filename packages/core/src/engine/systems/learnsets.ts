import { z } from "zod";
import { MoveName, MoveNameSchema } from "@pokecrystal/core/core/enums/move";
import { loadMergedLearnsetsSync } from "@pokecrystal/core/core/content-packs";

const LearnsetSchema = z.tuple([z.number(), MoveNameSchema]);
export type Learnset = z.infer<typeof LearnsetSchema>;

const SpeciesLearnsetsSchema = z.record(z.string(), z.array(LearnsetSchema));
export type SpeciesLearnsets = z.infer<typeof SpeciesLearnsetsSchema>;

let _learnsets: SpeciesLearnsets | null = null;
function loadLevelUpLearnsets(): SpeciesLearnsets {
  if (_learnsets) {
    return _learnsets;
  }
  _learnsets = SpeciesLearnsetsSchema.parse(loadMergedLearnsetsSync());
  return _learnsets;
}

export function levelUpMovesForSpecies(speciesId: string): Learnset[] {
  const lookup = loadLevelUpLearnsets();
  return lookup[speciesId.toUpperCase()] || [];
}

export function defaultMovesForLevel(
  speciesId: string,
  level: number,
  maxMoves: number = 4
): MoveName[] {
  if (level <= 0 || maxMoves <= 0) {
    return [];
  }

  let slots: MoveName[] = [];
  for (const [learnLevel, move] of levelUpMovesForSpecies(speciesId)) {
    if (learnLevel > level) {
      continue;
    }
    const index = slots.indexOf(move);
    if (index >= 0) {
      slots.splice(index, 1);
    }
    slots.push(move);
    if (slots.length > maxMoves) {
      slots.shift();
    }
  }
  return slots;
}

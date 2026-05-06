import { MoveName } from "@pokecrystal/core/core/enums/move";
import { PokemonType } from "@pokecrystal/core/core/enums/pokemon";
import { MoveEffect } from "@pokecrystal/core/core/enums";
import { Stat } from "@pokecrystal/core/core/enums/pokemon";
import { loadMergedMovesDataSync } from "@pokecrystal/core/core/content-packs";

export interface Move {
  name: MoveName;
  type: PokemonType;
  power: number;
  accuracy: number;
  pp: number;
  effect: MoveEffect;
  effect_chance: number;
  stat?: Stat | null;
  amount?: number | null;
}

type MovesData = Record<string, Move>;

let _moves: MovesData | null = null;
export function loadAllMoves(): MovesData {
  if (_moves) {
    return _moves;
  }
  _moves = loadMergedMovesDataSync() as MovesData;
  return _moves;
}

import type { Pokemon } from "@pokecrystal/core/core/models";
import { Stat } from "@pokecrystal/core/core/enums";

const BATTLE_STAGE_STATS = Object.values(Stat) as Stat[];

export const resetBattleStatStages = (pokemon: Pokemon): void => {
  for (const stat of BATTLE_STAGE_STATS) {
    pokemon.stat_boosts[stat] = 0;
  }
};

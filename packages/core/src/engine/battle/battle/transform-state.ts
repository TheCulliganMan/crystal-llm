import { type LearnedMove, type Pokemon } from "@pokecrystal/core/core/models";
import { MoveName, Stat } from "@pokecrystal/core/core/enums";

const cloneLearnedMoves = (moves: LearnedMove[] = []): LearnedMove[] =>
  moves.map((move) => ({ ...move }));

const cloneStatBoosts = (pokemon: Pokemon): Record<Stat, number> => ({
  HP: pokemon.stat_boosts[Stat.HP] ?? 0,
  ATTACK: pokemon.stat_boosts[Stat.ATTACK] ?? 0,
  DEFENSE: pokemon.stat_boosts[Stat.DEFENSE] ?? 0,
  SPEED: pokemon.stat_boosts[Stat.SPEED] ?? 0,
  SPECIAL_ATTACK: pokemon.stat_boosts[Stat.SPECIAL_ATTACK] ?? 0,
  SPECIAL_DEFENSE: pokemon.stat_boosts[Stat.SPECIAL_DEFENSE] ?? 0,
  ACCURACY: pokemon.stat_boosts[Stat.ACCURACY] ?? 0,
  EVASION: pokemon.stat_boosts[Stat.EVASION] ?? 0,
});

const snapshotDirectStats = (pokemon: Pokemon): Record<string, number> => ({
  attack: pokemon.attack ?? 0,
  defense: pokemon.defense ?? 0,
  speed: pokemon.speed ?? 0,
  special_attack: pokemon.special_attack ?? 0,
  special_defense: pokemon.special_defense ?? 0,
});

// ASM mapping: pokecrystal_disassembly/engine/battle/move_effects/transform.asm
export const applyTransformState = (attacker: Pokemon, defender: Pokemon): boolean => {
  if (defender.transformed) {
    return false;
  }

  if (!attacker.original_species) {
    attacker.original_species = attacker.species;
  }
  if (!attacker.transform_backup_dvs) {
    attacker.transform_backup_dvs = { ...attacker.dvs };
  }
  if (!attacker.transform_backup_moves) {
    attacker.transform_backup_moves = cloneLearnedMoves(attacker.moves);
  }
  if (!attacker.transform_backup_stat_boosts) {
    attacker.transform_backup_stat_boosts = cloneStatBoosts(attacker);
  }
  if (!attacker.transform_backup_stats) {
    attacker.transform_backup_stats = snapshotDirectStats(attacker);
  }

  attacker.transformed = true;
  attacker.species = defender.species;
  attacker.moves = defender.moves.map((move) => ({
    name: move.name,
    current_pp: move.name === MoveName.SKETCH ? 1 : 5,
  }));
  attacker.dvs = { ...defender.dvs };
  attacker.attack = defender.attack ?? 0;
  attacker.defense = defender.defense ?? 0;
  attacker.speed = defender.speed ?? 0;
  attacker.special_attack = defender.special_attack ?? 0;
  attacker.special_defense = defender.special_defense ?? 0;
  attacker.stat_boosts = cloneStatBoosts(defender);
  return true;
};

export const restoreTransformState = (pokemon: Pokemon): void => {
  if (!pokemon.transformed && !pokemon.original_species) {
    return;
  }

  if (pokemon.original_species) {
    pokemon.species = pokemon.original_species;
  }
  if (pokemon.transform_backup_dvs) {
    pokemon.dvs = { ...pokemon.transform_backup_dvs };
  }
  if (pokemon.transform_backup_moves) {
    pokemon.moves = cloneLearnedMoves(pokemon.transform_backup_moves);
  }
  if (pokemon.transform_backup_stat_boosts) {
    pokemon.stat_boosts = {
      ...pokemon.stat_boosts,
      ...pokemon.transform_backup_stat_boosts,
    };
  }
  if (pokemon.transform_backup_stats) {
    pokemon.attack = pokemon.transform_backup_stats.attack ?? 0;
    pokemon.defense = pokemon.transform_backup_stats.defense ?? 0;
    pokemon.speed = pokemon.transform_backup_stats.speed ?? 0;
    pokemon.special_attack = pokemon.transform_backup_stats.special_attack ?? 0;
    pokemon.special_defense = pokemon.transform_backup_stats.special_defense ?? 0;
  }

  pokemon.transformed = false;
  pokemon.original_species = undefined;
  pokemon.transform_backup_dvs = undefined;
  pokemon.transform_backup_moves = undefined;
  pokemon.transform_backup_stat_boosts = undefined;
  pokemon.transform_backup_stats = undefined;
};

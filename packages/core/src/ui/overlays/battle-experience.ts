import { z } from 'zod';
import { MoveSchema, LearnedMove, Move, Pokemon } from '../../core/models';
import { MoveName } from '../../core/enums';
import { calculateExperience } from '../../engine/experience';
import { levelUpMovesForSpecies } from '../../engine/systems/learnsets';
import { loadMergedMovesDataSync } from '../../core/content-packs';

let moveCache: Map<MoveName, Move> | null = null;
let moveDataCache: Record<string, unknown> | null = null;

const isRecord = (value: unknown): value is Record<string, unknown> =>
  Boolean(value && typeof value === 'object');

const loadMoveData = (): Record<string, unknown> => {
  if (!moveDataCache) {
    moveDataCache = loadMergedMovesDataSync() as Record<string, unknown>;
  }
  return moveDataCache;
};

export const loadMoveMetadata = (): Map<MoveName, Move> => {
  if (!moveCache) {
    const parsed: Map<MoveName, Move> = new Map();
    const rawData = loadMoveData();
    const entries = isRecord(rawData) ? rawData : {};
    for (const [name, raw] of Object.entries(entries)) {
      if (!isRecord(raw)) {
        continue;
      }
      const sanitized = {
        ...raw,
        stat: raw['stat'] ?? undefined,
        amount: raw['amount'] ?? undefined,
      };
      const move = MoveSchema.parse(sanitized);
      parsed.set(name as MoveName, move);
    }
    moveCache = parsed;
  }
  return moveCache;
};

export type LevelUpStats = {
  max_hp: number;
  attack: number;
  defense: number;
  speed: number;
  special_attack: number;
  special_defense: number;
};

export type LevelUpInfo = {
  level: number;
  expThreshold: number;
  stats: LevelUpStats;
  hpDelta: number;
  learnedMoves: LearnedMove[];
};

export type ExpBarAnimationState = {
  pokemon: Pokemon;
  targetExp: number;
  pendingLevels: LevelUpInfo[];
  speed: number;
};

export const buildLevelQueue = (pokemon: Pokemon, targetExp: number): LevelUpInfo[] => {
  const pending: LevelUpInfo[] = [];
  const growth = pokemon.species.growth_rate;
  if (!growth) {
    return pending;
  }
  let currentLevel = pokemon.level;
  let previousMaxHp = pokemon.max_hp;
  const learnset = levelUpMovesForSpecies(pokemon.species.id);
  const learnMap = new Map<number, LearnedMove[]>();
  const moveMetadata = loadMoveMetadata();
  for (const [level, moveName] of learnset) {
    const moveData = moveMetadata.get(moveName);
    const pp = moveData ? moveData.pp : 0;
    const learned: LearnedMove = { name: moveName, current_pp: pp };
    if (!learnMap.has(level)) {
      learnMap.set(level, []);
    }
    learnMap.get(level)?.push(learned);
  }
  while (currentLevel < 100) {
    const nextThreshold = calculateExperience(growth, currentLevel + 1);
    if (targetExp < nextThreshold) {
      break;
    }
    const stats = statsForLevel(pokemon, currentLevel + 1);
    const hpDelta = stats.max_hp - previousMaxHp;
    previousMaxHp = stats.max_hp;
    pending.push({
      level: currentLevel + 1,
      expThreshold: nextThreshold,
      stats,
      hpDelta,
      learnedMoves: learnMap.get(currentLevel + 1) ?? [],
    });
    currentLevel += 1;
  }
  return pending;
};

export const statsForLevel = (pokemon: Pokemon, level: number): LevelUpStats => {
  const base = pokemon.species.base_stats;
  const dvs = pokemon.dvs;
  const maxHp = Math.floor(((base.hp + dvs.hp) * 2 * level) / 100 + level + 10);
  const attack = Math.floor(((base.attack + dvs.attack) * 2 * level) / 100 + 5);
  const defense = Math.floor(((base.defense + dvs.defense) * 2 * level) / 100 + 5);
  const speed = Math.floor(((base.speed + dvs.speed) * 2 * level) / 100 + 5);
  const specialAttack = Math.floor(
    ((base.special_attack + dvs.special) * 2 * level) / 100 + 5
  );
  const specialDefense = Math.floor(
    ((base.special_defense + dvs.special) * 2 * level) / 100 + 5
  );
  return {
    max_hp: maxHp,
    attack,
    defense,
    speed,
    special_attack: specialAttack,
    special_defense: specialDefense,
  };
};

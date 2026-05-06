import { Pokemon } from '@pokecrystal/core/core/models';
import { Stat, StatusCondition } from '@pokecrystal/core/core/enums';
import { Battle } from './battle-logic';
import Fraction from 'fraction.js';

const STAT_STAGE_MULTIPLIERS: Map<number, Fraction> = new Map([
    [-6, new Fraction(25, 100)],
    [-5, new Fraction(28, 100)],
    [-4, new Fraction(33, 100)],
    [-3, new Fraction(40, 100)],
    [-2, new Fraction(50, 100)],
    [-1, new Fraction(66, 100)],
    [0, new Fraction(1, 1)],
    [1, new Fraction(15, 10)],
    [2, new Fraction(2, 1)],
    [3, new Fraction(25, 10)],
    [4, new Fraction(3, 1)],
    [5, new Fraction(35, 10)],
    [6, new Fraction(4, 1)],
]);

export function clampStage(stage: number): number {
    return Math.max(-6, Math.min(6, stage));
}

export function stageMultiplier(stage: number): Fraction {
    return STAT_STAGE_MULTIPLIERS.get(clampStage(stage))!;
}

export function accuracyStageMultiplier(stage: number): Fraction {
    stage = clampStage(stage);
    if (stage >= 0) {
        return new Fraction(3 + stage, 3);
    }
    return new Fraction(3, 3 - stage);
}

export function applyStage(value: number, stage: number): number {
    const modifier = stageMultiplier(stage);
    const n = Number(modifier.n);
    const d = Number(modifier.d);
    const modified = Math.floor(value * n / d);
    return Math.max(1, Math.min(modified, 999));
}

export function calculateBattleStat(battle: Battle, pokemon: Pokemon, stat: Stat): number {
  let baseValue = pokemon._calculateStat(stat);
  const side = battle.context.sideFor(pokemon);
  if (side !== undefined && battle.context.badgeBoostActive(side, stat)) {
    baseValue = Math.min(999, baseValue + Math.floor(baseValue / 8));
  }

  let value = applyStage(baseValue, pokemon.stat_boosts[stat] ?? 0);

  if (stat === Stat.ATTACK && pokemon.status === StatusCondition.BURN) {
    value = Math.max(1, Math.floor(value / 2));
  }
  if (stat === Stat.SPEED && pokemon.status === StatusCondition.PARALYSIS) {
    value = Math.max(1, Math.floor(value / 4));
  }

  return value;
}
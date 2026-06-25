import type { LearnedMove, Move } from "@pokecrystal/core/core/models";
import {
  AILayer,
  MoveEffect,
  MoveName,
  PokemonType,
  Stat,
  StatusCondition,
} from "@pokecrystal/core/core/enums";
import { movesMap } from "@pokecrystal/core/core/data-loader";
import type { GameState } from "@pokecrystal/core/core/state";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { BattleContext } from "@pokecrystal/core/engine/battle/battle/battle-context";
import { calculateDamage, TYPE_CHART } from "@pokecrystal/core/engine/battle/battle/damage-calculation";

// ASM mapping: engine/battle/ai/move.asm (AI_Move) and engine/battle/ai/scoring.asm (AI_* layers).

const NUM_MOVES = 4;
const DEFAULT_SCORE = 20;
const UNUSABLE_SCORE = 80;
const STATUS_ONLY_EFFECTS = new Set<string>([
  MoveEffect.SLEEP,
  MoveEffect.TOXIC,
  MoveEffect.POISON,
  MoveEffect.PARALYZE,
]);
const RISKY_EFFECTS = new Set<string>([
  MoveEffect.SELFDESTRUCT,
  MoveEffect.OHKO,
]);
const RECKLESS_EFFECTS = new Set<string>([
  MoveEffect.SELFDESTRUCT,
  MoveEffect.RAMPAGE,
  MoveEffect.MULTI_HIT,
  MoveEffect.DOUBLE_HIT,
]);
const CONSTANT_DAMAGE_EFFECTS = new Set<string>([
  MoveEffect.STATIC_DAMAGE,
  MoveEffect.LEVEL_DAMAGE,
  MoveEffect.PSYWAVE,
]);
const STALL_MOVES = new Set<MoveName>([
  MoveName.SWORDS_DANCE,
  MoveName.TAIL_WHIP,
  MoveName.LEER,
  MoveName.GROWL,
  MoveName.DISABLE,
  MoveName.MIST,
  MoveName.COUNTER,
  MoveName.LEECH_SEED,
  MoveName.GROWTH,
  MoveName.STRING_SHOT,
  MoveName.MEDITATE,
  MoveName.AGILITY,
  MoveName.RAGE,
  MoveName.MIMIC,
  MoveName.SCREECH,
  MoveName.HARDEN,
  MoveName.WITHDRAW,
  MoveName.DEFENSE_CURL,
  MoveName.BARRIER,
  MoveName.LIGHT_SCREEN,
  MoveName.REFLECT,
  MoveName.FOCUS_ENERGY,
  MoveName.BIDE,
  MoveName.AMNESIA,
  MoveName.KINESIS,
  MoveName.MIRROR_MOVE,
  MoveName.SUBSTITUTE,
  MoveName.SMOKESCREEN,
]);
const RESIDUAL_MOVES = new Set<MoveName>([
  MoveName.MIST,
  MoveName.LEECH_SEED,
  MoveName.POISONPOWDER,
  MoveName.STUN_SPORE,
  MoveName.THUNDER_WAVE,
  MoveName.FOCUS_ENERGY,
  MoveName.BIDE,
  MoveName.POISON_GAS,
  MoveName.TRANSFORM,
  MoveName.CONVERSION,
  MoveName.SUBSTITUTE,
  MoveName.SPIKES,
]);

class ContextualRNG {
  private predefined: number | null;
  private hardware: HardwareRNG | null;

  constructor(context: BattleContext, gameState?: GameState | null) {
    this.predefined =
      context.predefinedRandomValue !== undefined
        ? Number(context.predefinedRandomValue)
        : null;
    this.hardware = gameState ? new HardwareRNG(gameState) : null;
  }

  private consumePredefined(): number | null {
    if (this.predefined === null) {
      return null;
    }
    const value = this.predefined;
    this.predefined = null;
    return value;
  }

  public randrange(upperBound: number): number {
    const predefined = this.consumePredefined();
    if (predefined !== null) {
      return Math.trunc(predefined * upperBound) % Math.max(1, upperBound);
    }
    if (this.hardware) {
      return this.hardware.randrange(upperBound);
    }
    if (upperBound <= 0) {
      return 0;
    }
    return Math.floor(Math.random() * upperBound);
  }

  public coinFlip(probability: number): boolean {
    const predefined = this.consumePredefined();
    if (predefined !== null) {
      return predefined < probability;
    }
    if (this.hardware) {
      return this.hardware.coinFlip(probability);
    }
    return Math.random() < probability;
  }
}

type MoveSlot = {
  index: number;
  learned: LearnedMove;
  move: Move;
};

class AIScoringState {
  constructor(
    public context: BattleContext,
    public slots: Array<MoveSlot | null>,
    public scores: number[],
    public rng: ContextualRNG,
  ) {}

  public *iterSlots(): Iterable<MoveSlot> {
    for (const slot of this.slots) {
      if (slot && slot.learned.current_pp > 0) {
        yield slot;
      }
    }
  }
}

export function getBestMove(
  context: BattleContext,
  gameState?: GameState | null,
  moveData: Map<MoveName, Move> = movesMap,
): Move | null {
  const trainer = context.enemyTrainer;
  if (!trainer) {
    return null;
  }

  const aiLayers =
    trainer.ai_layers && trainer.ai_layers.length > 0
      ? trainer.ai_layers
      : [AILayer.AI_BASIC];

  const slots: Array<MoveSlot | null> = [];
  const scores: number[] = [];
  for (let index = 0; index < NUM_MOVES; index += 1) {
    if (index >= context.enemyPokemon.moves.length) {
      slots.push(null);
      scores.push(0);
      continue;
    }

    const learned = context.enemyPokemon.moves[index];
    if (!learned) {
      slots.push(null);
      scores.push(0);
      continue;
    }

    const move = moveData.get(learned.name);
    if (!move) {
      slots.push(null);
      scores.push(0);
      continue;
    }

    slots.push({ index, learned, move });
    const score = learned.current_pp <= 0 ? UNUSABLE_SCORE : DEFAULT_SCORE;
    scores.push(score);
  }

  const rng = new ContextualRNG(context, gameState ?? undefined);
  const state = new AIScoringState(context, slots, scores, rng);

  for (const layer of aiLayers) {
    const scorer = AI_SCORING_FUNCTIONS.get(layer);
    if (scorer) {
      scorer(state);
    }
  }

  let candidates: MoveSlot[] = [];
  let minScore: number | null = null;
  for (let i = 0; i < state.slots.length; i += 1) {
    const slot = state.slots[i];
    const score = state.scores[i];
    if (!slot || slot.learned.current_pp <= 0) {
      continue;
    }
    if (score >= UNUSABLE_SCORE) {
      continue;
    }
    if (minScore === null || score < minScore) {
      minScore = score;
      candidates = [slot];
    } else if (score === minScore) {
      candidates.push(slot);
    }
  }

  if (!candidates.length) {
    for (const slot of state.slots) {
      if (slot && slot.learned.current_pp > 0) {
        return slot.move;
      }
    }
    return null;
  }

  if (candidates.length === 1) {
    return candidates[0].move;
  }

  return candidates[rng.randrange(candidates.length)].move;
}

const clampScore = (value: number): number => {
  return Math.max(0, Math.min(255, value));
};

const encourage = (state: AIScoringState, slotIndex: number, steps = 1): void => {
  state.scores[slotIndex] = clampScore(state.scores[slotIndex] - steps);
};

const discourage = (state: AIScoringState, slotIndex: number, steps = 1): void => {
  state.scores[slotIndex] = clampScore(state.scores[slotIndex] + steps);
};

const aiDiscourage = (state: AIScoringState, slotIndex: number): void => {
  state.scores[slotIndex] = clampScore(state.scores[slotIndex] + 10);
};

const hpRatio = (hp: number, maxHp: number): number => {
  if (maxHp <= 0) {
    return 1;
  }
  return hp / maxHp;
};

const typeEffectiveness = (move: Move, defenderTypes: PokemonType[]): number => {
  let effectiveness = 1;
  for (const defenderType of defenderTypes) {
    if (!defenderType || defenderType === PokemonType.NONE) {
      continue;
    }
    effectiveness *= TYPE_CHART.get(move.type)?.get(defenderType) ?? 1;
  }
  return effectiveness;
};

const enemyHasEffect = (state: AIScoringState, effects: Iterable<string>): boolean => {
  const effectSet = new Set(effects);
  for (const slot of state.iterSlots()) {
    if (effectSet.has(slot.move.effect)) {
      return true;
    }
  }
  return false;
};

const scoreBasic = (state: AIScoringState): void => {
  const player = state.context.playerPokemon;
  const enemy = state.context.enemyPokemon;
  for (const slot of state.iterSlots()) {
    if (STATUS_ONLY_EFFECTS.has(slot.move.effect)) {
      if (player.status != null) {
        state.scores[slot.index] = UNUSABLE_SCORE;
        continue;
      }
      if (state.context.playerSafeguard) {
        state.scores[slot.index] = UNUSABLE_SCORE;
        continue;
      }
    }
    if (
      slot.move.effect === MoveEffect.HEAL ||
      slot.move.effect === MoveEffect.MORNING_SUN ||
      slot.move.effect === MoveEffect.MOONLIGHT ||
      slot.move.effect === MoveEffect.SYNTHESIS
    ) {
      const ratio = hpRatio(enemy.hp, enemy.max_hp);
      if (ratio <= 0.5) {
        encourage(state, slot.index, 3);
      } else if (ratio >= 0.75) {
        discourage(state, slot.index, 3);
        continue;
      }
    }
  }
};

const scoreSetup = (state: AIScoringState): void => {
  const enemy = state.context.enemyPokemon;
  const player = state.context.playerPokemon;

  const statUpEffects = new Set<string>([
    MoveEffect.ATTACK_UP,
    MoveEffect.DEFENSE_UP,
    MoveEffect.SPEED_UP,
    MoveEffect.SPECIAL_ATTACK_UP,
    MoveEffect.SPECIAL_DEFENSE_UP,
    MoveEffect.ACCURACY_UP,
    MoveEffect.EVASION_UP,
    MoveEffect.ATTACK_UP_2,
    MoveEffect.DEFENSE_UP_2,
    MoveEffect.SPEED_UP_2,
    MoveEffect.SPECIAL_ATTACK_UP_2,
    MoveEffect.SPECIAL_DEFENSE_UP_2,
    MoveEffect.ACCURACY_UP_2,
    MoveEffect.EVASION_UP_2,
  ]);
  const statDownEffects = new Set<string>([
    MoveEffect.ATTACK_DOWN,
    MoveEffect.DEFENSE_DOWN,
    MoveEffect.SPEED_DOWN,
    MoveEffect.SPECIAL_ATTACK_DOWN,
    MoveEffect.SPECIAL_DEFENSE_DOWN,
    MoveEffect.ACCURACY_DOWN,
    MoveEffect.EVASION_DOWN,
    MoveEffect.ATTACK_DOWN_2,
    MoveEffect.DEFENSE_DOWN_2,
    MoveEffect.SPEED_DOWN_2,
    MoveEffect.SPECIAL_ATTACK_DOWN_2,
    MoveEffect.SPECIAL_DEFENSE_DOWN_2,
    MoveEffect.ACCURACY_DOWN_2,
    MoveEffect.EVASION_DOWN_2,
  ]);

  for (const slot of state.iterSlots()) {
    if (statUpEffects.has(slot.move.effect)) {
      if (enemy.turns_in_battle === 0) {
        if (state.rng.coinFlip(0.5)) {
          encourage(state, slot.index, 2);
        }
      } else if (state.rng.coinFlip(0.12)) {
        discourage(state, slot.index, 2);
      }
    } else if (statDownEffects.has(slot.move.effect)) {
      if (player.turns_in_battle === 0) {
        if (state.rng.coinFlip(0.5)) {
          encourage(state, slot.index, 2);
        }
      } else if (state.rng.coinFlip(0.12)) {
        discourage(state, slot.index, 2);
      }
    }
  }
};

const scoreTypes = (state: AIScoringState): void => {
  const player = state.context.playerPokemon;
  const defenderTypes = [player.species.type1, player.species.type2].filter(
    (type): type is PokemonType => Boolean(type && type !== PokemonType.NONE),
  );

  for (const slot of state.iterSlots()) {
    const effectiveness = typeEffectiveness(slot.move, defenderTypes);
    if (effectiveness === 0) {
      aiDiscourage(state, slot.index);
      continue;
    }
    if (effectiveness > 1 && slot.move.power > 0) {
      encourage(state, slot.index);
    } else if (effectiveness < 1) {
      if (hasAlternativeType(state, slot.move.type)) {
        discourage(state, slot.index);
      }
    }
  }
};

const hasAlternativeType = (state: AIScoringState, moveType: PokemonType): boolean => {
  for (const slot of state.iterSlots()) {
    if (slot.move.power <= 0) {
      continue;
    }
    if (slot.move.type !== moveType) {
      return true;
    }
  }
  return false;
};

const hasSaferAlternative = (
  state: AIScoringState,
  slotIndex: number,
  accuracy: number,
): boolean => {
  for (const slot of state.iterSlots()) {
    if (slot.index === slotIndex) {
      continue;
    }
    if (slot.move.power <= 0) {
      continue;
    }
    if (slot.move.accuracy >= accuracy) {
      return true;
    }
  }
  return false;
};

const scoreOffensive = (state: AIScoringState): void => {
  for (const slot of state.iterSlots()) {
    if (slot.move.power <= 0) {
      discourage(state, slot.index, 2);
    }
    if (
      slot.move.accuracy < 60 &&
      hasSaferAlternative(state, slot.index, slot.move.accuracy)
    ) {
      discourage(state, slot.index, 2);
    }
  }
};

const scoreSmart = (state: AIScoringState): void => {
  for (const slot of state.iterSlots()) {
    const handler = SMART_HANDLERS.get(slot.move.effect);
    if (handler) {
      handler(state, slot);
    }
  }
};

const smartSleep = (state: AIScoringState, slot: MoveSlot): void => {
  const player = state.context.playerPokemon;
  if (player.status === StatusCondition.SLEEP) {
    return;
  }
  const encourageMove =
    enemyHasEffect(state, [MoveEffect.DREAM_EATER, MoveEffect.NIGHTMARE]) ||
    state.rng.coinFlip(0.5);
  if (encourageMove) {
    encourage(state, slot.index, 2);
  }
};

const smartDreamEater = (state: AIScoringState, slot: MoveSlot): void => {
  if (state.context.playerPokemon.status !== StatusCondition.SLEEP) {
    aiDiscourage(state, slot.index);
  }
};

const smartHeal = (state: AIScoringState, slot: MoveSlot): void => {
  const enemy = state.context.enemyPokemon;
  const ratio = hpRatio(enemy.hp, enemy.max_hp);
  if (ratio < 0.25) {
    if (state.rng.coinFlip(0.1)) {
      encourage(state, slot.index, 2);
    }
  } else if (ratio > 0.5) {
    discourage(state, slot.index);
  }
};

const smartToxicOrLeechSeed = (state: AIScoringState, slot: MoveSlot): void => {
  const player = state.context.playerPokemon;
  if (player.status != null) {
    aiDiscourage(state, slot.index);
    return;
  }

  const isToxic = new Set<string>([
    MoveEffect.TOXIC,
    MoveEffect.POISON,
    MoveEffect.POISON_HIT,
  ]).has(slot.move.effect);
  if (isToxic) {
    if (slot.move.effect === MoveEffect.TOXIC) {
      const playerTypes = [player.species.type1, player.species.type2];
      if (playerTypes.includes(PokemonType.POISON)) {
        aiDiscourage(state, slot.index);
        return;
      }
    }

    const playerRatio = hpRatio(player.hp, player.max_hp);
    if (playerRatio < 0.5 && slot.move.effect === MoveEffect.TOXIC) {
      encourage(state, slot.index, 2);
    }
  }
};

const smartLightScreenReflect = (state: AIScoringState, slot: MoveSlot): void => {
  const enemy = state.context.enemyPokemon;
  if (enemy.hp >= enemy.max_hp) {
    return;
  }
  if (state.rng.coinFlip(0.08)) {
    discourage(state, slot.index);
  }
};

const smartSafeguard = (state: AIScoringState, slot: MoveSlot): void => {
  if (state.context.enemySafeguard) {
    discourage(state, slot.index);
  }
};

const smartStatus = (state: AIScoringState, slot: MoveSlot): void => {
  if (state.context.playerPokemon.status != null) {
    aiDiscourage(state, slot.index);
  }
};

const smartStatUp = (state: AIScoringState, slot: MoveSlot): void => {
  const enemy = state.context.enemyPokemon;
  let stat: Stat | null = null;
  if (
    slot.move.effect === MoveEffect.ATTACK_UP ||
    slot.move.effect === MoveEffect.ATTACK_UP_2
  ) {
    stat = Stat.ATTACK;
  } else if (
    slot.move.effect === MoveEffect.DEFENSE_UP ||
    slot.move.effect === MoveEffect.DEFENSE_UP_2
  ) {
    stat = Stat.DEFENSE;
  } else if (
    slot.move.effect === MoveEffect.SPEED_UP ||
    slot.move.effect === MoveEffect.SPEED_UP_2
  ) {
    stat = Stat.SPEED;
  } else if (
    slot.move.effect === MoveEffect.SPECIAL_ATTACK_UP ||
    slot.move.effect === MoveEffect.SPECIAL_ATTACK_UP_2
  ) {
    stat = Stat.SPECIAL_ATTACK;
  } else if (
    slot.move.effect === MoveEffect.SPECIAL_DEFENSE_UP ||
    slot.move.effect === MoveEffect.SPECIAL_DEFENSE_UP_2
  ) {
    stat = Stat.SPECIAL_DEFENSE;
  } else if (
    slot.move.effect === MoveEffect.ACCURACY_UP ||
    slot.move.effect === MoveEffect.ACCURACY_UP_2
  ) {
    stat = Stat.ACCURACY;
  } else if (
    slot.move.effect === MoveEffect.EVASION_UP ||
    slot.move.effect === MoveEffect.EVASION_UP_2
  ) {
    stat = Stat.EVASION;
  }

  if (stat && (enemy.stat_boosts[stat] ?? 0) >= 6) {
    aiDiscourage(state, slot.index);
  }
};

const smartStatDown = (state: AIScoringState, slot: MoveSlot): void => {
  const player = state.context.playerPokemon;
  let stat: Stat | null = null;
  if (
    slot.move.effect === MoveEffect.ATTACK_DOWN ||
    slot.move.effect === MoveEffect.ATTACK_DOWN_2
  ) {
    stat = Stat.ATTACK;
  } else if (
    slot.move.effect === MoveEffect.DEFENSE_DOWN ||
    slot.move.effect === MoveEffect.DEFENSE_DOWN_2
  ) {
    stat = Stat.DEFENSE;
  } else if (
    slot.move.effect === MoveEffect.SPEED_DOWN ||
    slot.move.effect === MoveEffect.SPEED_DOWN_2
  ) {
    stat = Stat.SPEED;
  } else if (
    slot.move.effect === MoveEffect.SPECIAL_ATTACK_DOWN ||
    slot.move.effect === MoveEffect.SPECIAL_ATTACK_DOWN_2
  ) {
    stat = Stat.SPECIAL_ATTACK;
  } else if (
    slot.move.effect === MoveEffect.SPECIAL_DEFENSE_DOWN ||
    slot.move.effect === MoveEffect.SPECIAL_DEFENSE_DOWN_2
  ) {
    stat = Stat.SPECIAL_DEFENSE;
  } else if (
    slot.move.effect === MoveEffect.ACCURACY_DOWN ||
    slot.move.effect === MoveEffect.ACCURACY_DOWN_2
  ) {
    stat = Stat.ACCURACY;
  } else if (
    slot.move.effect === MoveEffect.EVASION_DOWN ||
    slot.move.effect === MoveEffect.EVASION_DOWN_2
  ) {
    stat = Stat.EVASION;
  }

  if (stat && (player.stat_boosts[stat] ?? 0) <= -6) {
    aiDiscourage(state, slot.index);
  }
};

const smartBide = (state: AIScoringState, slot: MoveSlot): void => {
  const enemy = state.context.enemyPokemon;
  if (hpRatio(enemy.hp, enemy.max_hp) < 0.5) {
    aiDiscourage(state, slot.index);
  }
};

const smartSubstitute = (state: AIScoringState, slot: MoveSlot): void => {
  const enemy = state.context.enemyPokemon;
  if (enemy.hp <= Math.floor(enemy.max_hp / 4)) {
    aiDiscourage(state, slot.index);
  }
};

const smartHyperBeam = (state: AIScoringState, slot: MoveSlot): void => {
  const player = state.context.playerPokemon;
  const estimated = estimateDamage(state, slot);
  if (estimated >= player.hp) {
    return;
  }
  if (hpRatio(player.hp, player.max_hp) > 0.3) {
    if (state.rng.coinFlip(0.5)) {
      discourage(state, slot.index);
    }
  }
};

const SMART_HANDLERS = new Map<string, (state: AIScoringState, slot: MoveSlot) => void>([
  [MoveEffect.SLEEP, smartSleep],
  [MoveEffect.DREAM_EATER, smartDreamEater],
  [MoveEffect.HEAL, smartHeal],
  [MoveEffect.MORNING_SUN, smartHeal],
  [MoveEffect.SYNTHESIS, smartHeal],
  [MoveEffect.MOONLIGHT, smartHeal],
  [MoveEffect.TOXIC, smartToxicOrLeechSeed],
  [MoveEffect.LEECH_SEED, smartToxicOrLeechSeed],
  [MoveEffect.LIGHT_SCREEN, smartLightScreenReflect],
  [MoveEffect.REFLECT, smartLightScreenReflect],
  [MoveEffect.SAFEGUARD, smartSafeguard],
  [MoveEffect.PARALYZE, smartStatus],
  [MoveEffect.BURN_HIT, smartStatus],
  [MoveEffect.POISON, smartStatus],
  [MoveEffect.ATTACK_UP, smartStatUp],
  [MoveEffect.DEFENSE_UP, smartStatUp],
  [MoveEffect.SPEED_UP, smartStatUp],
  [MoveEffect.SPECIAL_ATTACK_UP, smartStatUp],
  [MoveEffect.SPECIAL_DEFENSE_UP, smartStatUp],
  [MoveEffect.ACCURACY_UP, smartStatUp],
  [MoveEffect.EVASION_UP, smartStatUp],
  [MoveEffect.ATTACK_UP_2, smartStatUp],
  [MoveEffect.DEFENSE_UP_2, smartStatUp],
  [MoveEffect.SPEED_UP_2, smartStatUp],
  [MoveEffect.SPECIAL_ATTACK_UP_2, smartStatUp],
  [MoveEffect.SPECIAL_DEFENSE_UP_2, smartStatUp],
  [MoveEffect.ACCURACY_UP_2, smartStatUp],
  [MoveEffect.EVASION_UP_2, smartStatUp],
  [MoveEffect.ATTACK_DOWN, smartStatDown],
  [MoveEffect.DEFENSE_DOWN, smartStatDown],
  [MoveEffect.SPEED_DOWN, smartStatDown],
  [MoveEffect.SPECIAL_ATTACK_DOWN, smartStatDown],
  [MoveEffect.SPECIAL_DEFENSE_DOWN, smartStatDown],
  [MoveEffect.ACCURACY_DOWN, smartStatDown],
  [MoveEffect.EVASION_DOWN, smartStatDown],
  [MoveEffect.ATTACK_DOWN_2, smartStatDown],
  [MoveEffect.DEFENSE_DOWN_2, smartStatDown],
  [MoveEffect.SPEED_DOWN_2, smartStatDown],
  [MoveEffect.SPECIAL_ATTACK_DOWN_2, smartStatDown],
  [MoveEffect.SPECIAL_DEFENSE_DOWN_2, smartStatDown],
  [MoveEffect.ACCURACY_DOWN_2, smartStatDown],
  [MoveEffect.EVASION_DOWN_2, smartStatDown],
  [MoveEffect.BIDE, smartBide],
  [MoveEffect.SUBSTITUTE, smartSubstitute],
  [MoveEffect.HYPER_BEAM, smartHyperBeam],
]);

const scoreOpportunist = (state: AIScoringState): void => {
  const enemy = state.context.enemyPokemon;
  if (enemy.max_hp <= 0) {
    return;
  }
  const ratio = hpRatio(enemy.hp, enemy.max_hp);
  if (ratio > 0.5) {
    return;
  }
  if (ratio >= 0.25 && state.rng.coinFlip(0.5)) {
    return;
  }
  for (const slot of state.iterSlots()) {
    if (STALL_MOVES.has(slot.move.name)) {
      discourage(state, slot.index);
    }
  }
};

const scoreAggressive = (state: AIScoringState): void => {
  const damages = new Map<number, number>();
  let bestDamage = -1;
  for (const slot of state.iterSlots()) {
    if (slot.move.power <= 0) {
      continue;
    }
    const damage = estimateDamage(state, slot);
    if (damage <= 0) {
      continue;
    }
    damages.set(slot.index, damage);
    if (damage > bestDamage) {
      bestDamage = damage;
    }
  }

  if (bestDamage <= 0) {
    return;
  }

  for (const slot of state.iterSlots()) {
    if (!damages.has(slot.index)) {
      continue;
    }
    if (damages.get(slot.index) === bestDamage) {
      continue;
    }
    if (slot.move.power < 2) {
      continue;
    }
    if (RECKLESS_EFFECTS.has(slot.move.effect)) {
      continue;
    }
    discourage(state, slot.index);
  }
};

const estimateDamage = (state: AIScoringState, slot: MoveSlot): number => {
  const move = slot.move;
  const attacker = state.context.enemyPokemon;
  const defender = state.context.playerPokemon;

  if (CONSTANT_DAMAGE_EFFECTS.has(move.effect)) {
    if (move.effect === MoveEffect.SUPER_FANG) {
      return Math.max(1, Math.floor(defender.hp / 2));
    }
    if (
      move.effect === MoveEffect.STATIC_DAMAGE ||
      move.effect === MoveEffect.LEVEL_DAMAGE
    ) {
      return attacker.level;
    }
    if (move.effect === MoveEffect.PSYWAVE) {
      return state.rng.randrange(Math.trunc(attacker.level * 1.5)) + 1;
    }
  }

  return calculateDamage(attacker, defender, move, state.context, false).damage;
};

const scoreCautious = (state: AIScoringState): void => {
  if (state.context.enemyPokemon.turns_in_battle === 0) {
    return;
  }
  for (const slot of state.iterSlots()) {
    if (!RESIDUAL_MOVES.has(slot.move.name)) {
      continue;
    }
    if (state.rng.coinFlip(0.9)) {
      discourage(state, slot.index);
    }
  }
};

const scoreStatus = (state: AIScoringState): void => {
  const player = state.context.playerPokemon;
  const playerTypes = [player.species.type1, player.species.type2];
  for (const slot of state.iterSlots()) {
    const move = slot.move;
    if (move.effect === MoveEffect.TOXIC || move.effect === MoveEffect.POISON) {
      if (
        playerTypes.includes(PokemonType.POISON) ||
        playerTypes.includes(PokemonType.STEEL)
      ) {
        aiDiscourage(state, slot.index);
      }
      continue;
    }
    if (move.effect === MoveEffect.SLEEP || move.effect === MoveEffect.PARALYZE) {
      const effectiveness = typeEffectiveness(move, playerTypes.filter(Boolean) as PokemonType[]);
      if (effectiveness === 0) {
        aiDiscourage(state, slot.index);
      }
      continue;
    }
    if (move.power > 0) {
      const effectiveness = typeEffectiveness(move, playerTypes.filter(Boolean) as PokemonType[]);
      if (effectiveness === 0) {
        aiDiscourage(state, slot.index);
      }
    }
  }
};

const scoreRisky = (state: AIScoringState): void => {
  const enemy = state.context.enemyPokemon;
  const player = state.context.playerPokemon;
  if (enemy.max_hp <= 0 || player.hp <= 0) {
    return;
  }

  for (const slot of state.iterSlots()) {
    const move = slot.move;
    if (move.power <= 0) {
      continue;
    }
    const damage = estimateDamage(state, slot);
    if (damage > enemy.hp) {
      encourage(state, slot.index, 8);
    }
    if (damage > player.hp) {
      discourage(state, slot.index, 10);
    }
  }
};

const scoreNone = (_state: AIScoringState): void => undefined;

const AI_SCORING_FUNCTIONS = new Map<AILayer, (state: AIScoringState) => void>([
  [AILayer.AI_BASIC, scoreBasic],
  [AILayer.AI_SETUP, scoreSetup],
  [AILayer.AI_TYPES, scoreTypes],
  [AILayer.AI_OFFENSIVE, scoreOffensive],
  [AILayer.AI_SMART, scoreSmart],
  [AILayer.AI_OPPORTUNIST, scoreOpportunist],
  [AILayer.AI_AGGRESSIVE, scoreAggressive],
  [AILayer.AI_CAUTIOUS, scoreCautious],
  [AILayer.AI_STATUS, scoreStatus],
  [AILayer.AI_RISKY, scoreRisky],
  [AILayer.AI_NONE, scoreNone],
]);

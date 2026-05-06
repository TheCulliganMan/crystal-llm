
import type { Item, Pokemon } from '../../../core/models';
import { BattleScene, ItemEffect, ItemPocket, StatusCondition, Stat } from '../../../core/enums';
import { recordPokedexCaught } from '../../../core/pokedex';
import { GameState } from '../../../core/state';
import { Event, EventManager } from '../../events/events';
import { bug_contest_set_caught_contest_mon } from '../../world/special-events/bug-contest';
import { movesMap } from '../../../core/data-loader';
import { HardwareRNG } from '../../games/rng';
import { addPokemon as addPartyPokemon } from '../../../core/models/party';
import { addPokemon as addBoxPokemon, BoxSchema, formatDefaultBoxName } from '../../../core/models/box';
import { MAX_PC_BOXES } from '../../../core/constants';
import { BattleContext } from './battle-context';

// ASM: engine/items/item_effects.asm::PokeBallEffect
const STATUS_KEYWORDS = [
  'cure',
  'heals',
  'heal ',
  'status',
  'awakens',
  'antidote',
  'defrost',
  'burned',
  'paralyzed',
  'poison',
  'sleep',
  'frozen',
];
const X_ITEM_NAMES = new Set([
  'X_ATTACK',
  'X_DEFEND',
  'X_SPEED',
  'X_SPECIAL',
  'X_ACCURACY',
]);
const REVIVE_ITEMS = new Set(['REVIVE', 'MAX_REVIVE', 'REVIVAL_HERB']);
const STATUS_CLEAR_ALL = new Set(['FULL_HEAL', 'FULL_RESTORE', 'HEAL_POWDER', 'MIRACLEBERRY']);
const CONFUSION_ITEMS = new Set(['BITTER_BERRY']);
const STATUS_ITEM_MAP: Record<string, StatusCondition> = {
  ANTIDOTE: StatusCondition.POISON,
  BURN_HEAL: StatusCondition.BURN,
  ICE_HEAL: StatusCondition.FREEZE,
  AWAKENING: StatusCondition.SLEEP,
  PARLYZ_HEAL: StatusCondition.PARALYSIS,
  PSNCUREBERRY: StatusCondition.POISON,
  PRZCUREBERRY: StatusCondition.PARALYSIS,
  BURNT_BERRY: StatusCondition.FREEZE,
  ICE_BERRY: StatusCondition.BURN,
  MINT_BERRY: StatusCondition.SLEEP,
};
const NO_EFFECT_TEXT = "It won't have any effect.";
const CRYSTAL_FAST_BALL_SPECIES = new Set(['MAGNEMITE', 'GRIMER', 'TANGELA']);
const CRYSTAL_HEAVY_BALL_JUNK_WEIGHT_SPECIES = new Set(['KADABRA', 'TAUROS', 'SUNFLORA']);

const dispatchCatchText = (eventManager: EventManager, text: string): void => {
  eventManager.dispatch(new Event('show_text', { text, wait_for_animation: true }));
};

const dispatchNoEffect = (eventManager: EventManager | null | undefined): void => {
  eventManager?.dispatch(new Event('show_text', { text: NO_EFFECT_TEXT }));
};

export function effectiveItemEffect(item: Item): ItemEffect {
  if (item.effect !== ItemEffect.NONE) {
    return item.effect;
  }
  if (item.pocket === ItemPocket.BALL) {
    return ItemEffect.POKE_BALL;
  }

  const name = item.script_name;
  const description = (item.description || '').toLowerCase();

  if (X_ITEM_NAMES.has(name)) {
    return ItemEffect.X_ITEM;
  }
  if (description.includes('full restore') || name === 'FULL_RESTORE') {
    return ItemEffect.FULL_RESTORE;
  }
  if (REVIVE_ITEMS.has(name) || description.includes('revive')) {
    return ItemEffect.REVIVE;
  }
  if (description.includes('restores') && description.includes('pp')) {
    return ItemEffect.RESTORE_PP;
  }
  if (
    (description.includes('restore') && description.includes('hp')) ||
    description.includes('self-restore')
  ) {
    return ItemEffect.RESTORE_HP;
  }
  if (STATUS_KEYWORDS.some((keyword) => description.includes(keyword))) {
    return ItemEffect.STATUS_HEAL;
  }
  if (name === 'GUARD_SPEC') {
    return ItemEffect.GUARD_SPEC;
  }
  if (name === 'DIRE_HIT') {
    return ItemEffect.DIRE_HIT;
  }
  if (name === 'POKE_DOLL') {
    return ItemEffect.POKE_DOLL;
  }
  return ItemEffect.NONE;
}

export function applyItemEffect(
  item: Item,
  targetPokemon: Pokemon,
  eventManager: EventManager,
  context: BattleContext,
  gameState: GameState | null,
  moveIndex: number | null,
): boolean {
  const effect = effectiveItemEffect(item);
  switch (effect) {
    case ItemEffect.POKE_BALL:
      return applyPokeBallEffect(item, eventManager, context, gameState);
    case ItemEffect.STATUS_HEAL:
    case ItemEffect.BITTER_BERRY:
      return applyStatusHealEffect(item, targetPokemon, eventManager);
    case ItemEffect.FULL_RESTORE:
      return applyFullRestoreEffect(targetPokemon, eventManager);
    case ItemEffect.RESTORE_HP:
      return applyRestoreHpEffect(item, targetPokemon, eventManager);
    case ItemEffect.RESTORE_PP:
      return applyRestorePpEffect(item, targetPokemon, eventManager, moveIndex);
    case ItemEffect.REVIVE:
      return applyReviveEffect(item, targetPokemon, eventManager);
    case ItemEffect.X_ITEM:
      return _apply_x_item_effect(item, targetPokemon, eventManager);
    case ItemEffect.X_ACCURACY:
      return _apply_x_item_effect(item, targetPokemon, eventManager);
    case ItemEffect.GUARD_SPEC:
      return applyGuardSpecEffect(targetPokemon, context, eventManager);
    case ItemEffect.DIRE_HIT:
      return applyDireHitEffect(targetPokemon, eventManager);
    case ItemEffect.POKE_DOLL:
      return applyPokeDollEffect(eventManager, context);
    case ItemEffect.NONE:
      throw new Error(`No battle effect defined for item ${JSON.stringify(item.name)}`);
    default:
      return handleUnsupportedItemEffect(effect, item, eventManager);
  }
}

function handleUnsupportedItemEffect(
  effect: ItemEffect,
  item: Item,
  _eventManager: EventManager | null,
): boolean {
  throw new Error(`Unsupported battle item effect ${String(effect)} for ${String(item.name)}.`);
}

export function getBallBonus(
  ballName: string,
  enemy: Pokemon,
  context: BattleContext,
  gameState: GameState | null,
): number {
  if (ballName === 'POKE_BALL') {
    return 1.0;
  }
  if (ballName === 'GREAT_BALL') {
    return 1.5;
  }
  if (ballName === 'ULTRA_BALL') {
    return 2.0;
  }
  if (ballName === 'PARK_BALL') {
    return 1.5;
  }

  if (ballName === 'LEVEL_BALL') {
    const player = context.playerPokemon;
    if (player.level <= enemy.level) {
      return 1.0;
    }
    if (player.level <= enemy.level * 2) {
      return 2.0;
    }
    if (player.level <= enemy.level * 4) {
      return 4.0;
    }
    return 8.0;
  }

  if (ballName === 'LURE_BALL') {
    const battleType = String(gameState?.wram.battle_type ?? '').toUpperCase();
    return battleType === 'BATTLETYPE_FISH' ? 3.0 : 1.0;
  }

  if (ballName === 'MOON_BALL') {
    return 1.0;
  }

  if (ballName === 'LOVE_BALL') {
    // Gen 2 bug: 8x if same species and SAME gender. Intended to be opposite gender.
    // This implementation faithfully reproduces the bug.
    const player = context.playerPokemon;
    if (player.species.id === enemy.species.id) {
      if (
        player.gender === enemy.gender &&
        player.gender !== null
      ) {
        return 8.0;
      }
    }
    return 1.0;
  }

  if (ballName === 'HEAVY_BALL') {
    // Weight based.
    // Using a very simplified weight map for now as weight data is not in PokemonSpecies.
    // Thresholds: <100kg: -20, 100-200kg: 0, 200-300kg: +20, 300-400kg: +30, >400kg: +40
    // Wait, heavy ball is additive to catch_rate, not a multiplier.
    // This function returns a multiplier. We need to handle additive bonus separately.
    // Returning 1.0 here as it's not a multiplier.
    return 1.0;
  }

  if (ballName === 'FAST_BALL') {
    if (CRYSTAL_FAST_BALL_SPECIES.has(String(enemy.species.id).toUpperCase())) {
      return 4.0;
    }
    return 1.0;
  }

  return 1.0;
}
function getHeavyBallModifier(enemy: Pokemon): number {
  if (CRYSTAL_HEAVY_BALL_JUNK_WEIGHT_SPECIES.has(String(enemy.species.id).toUpperCase())) {
    return 40;
  }
  const weight = enemy.species.weight / 10; // Weight is in hectograms, convert to kg
  if (weight < 102.4) {
    return -20;
  }
  if (weight < 204.8) {
    return 0;
  }
  if (weight < 307.2) {
    return 20;
  }
  if (weight < 409.6) {
    return 30;
  }
  return 40;
}

const WOBBLE_PROBABILITIES: Array<[number, number]> = [
  [1, 63],
  [2, 75],
  [3, 84],
  [4, 90],
  [5, 95],
  [7, 103],
  [10, 113],
  [15, 126],
  [20, 134],
  [30, 149],
  [40, 160],
  [50, 169],
  [60, 177],
  [80, 191],
  [100, 201],
  [120, 211],
  [140, 220],
  [160, 227],
  [180, 234],
  [200, 240],
  [220, 246],
  [240, 251],
  [254, 253],
  [255, 255],
];

type BallCatchRateResult = {
  rate: number;
  skipHpCalc: boolean;
};

type CatchOutcome = {
  caught: boolean;
  wobbleCount: number;
  finalCatchRate: number;
};

const clampCatchRate = (value: number, min = 0): number => {
  const truncated = Math.trunc(value);
  if (truncated > 0xff) {
    return 0xff;
  }
  if (truncated < min) {
    return min;
  }
  return truncated;
};

// ASM: engine/items/item_effects.asm::PokeBallEffect (HP portion).
const computeHpAdjustedCatchRate = (catchRate: number, hp: number, maxHp: number): number => {
  const hpValue = Math.max(0, Math.trunc(hp));
  const maxValue = Math.max(1, Math.trunc(maxHp));
  let hp2 = (hpValue * 2) & 0xffff;
  let max3 = (maxValue * 3) & 0xffff;
  if ((max3 & 0xff00) !== 0) {
    hp2 >>>= 2;
    max3 >>>= 2;
  }
  let hpLow = hp2 & 0xff;
  if (hpLow === 0) {
    hpLow = 1;
  }
  const maxLow = max3 & 0xff;
  if (maxLow === 0) {
    throw new Error(`Catch divisor is zero for max HP ${maxValue}.`);
  }
  const diff = (maxLow - hpLow) & 0xff;
  const product = (catchRate & 0xff) * diff;
  let result = Math.floor(product / maxLow) & 0xff;
  if (result === 0) {
    result = 1;
  }
  return result;
};

// ASM: engine/items/item_effects.asm::BallMultiplierFunctionTable.
const applyBallMultiplier = (
  ballName: string,
  baseRate: number,
  enemy: Pokemon,
  context: BattleContext,
  gameState: GameState | null,
): BallCatchRateResult => {
  const name = ballName.trim().toUpperCase();
  let rate = clampCatchRate(baseRate, 0);
  let skipHpCalc = false;

  switch (name) {
    case 'ULTRA_BALL':
      rate = clampCatchRate(rate << 1, 0);
      break;
    case 'GREAT_BALL':
    case 'SAFARI_BALL':
    case 'PARK_BALL':
      rate = clampCatchRate(rate + (rate >> 1), 0);
      break;
    case 'HEAVY_BALL': {
      const modified = rate + getHeavyBallModifier(enemy);
      rate = clampCatchRate(modified, 1);
      break;
    }
    case 'LEVEL_BALL': {
      const playerLevel = Math.trunc(context.playerPokemon.level);
      const enemyLevel = Math.trunc(enemy.level);
      if (playerLevel > enemyLevel) {
        rate = clampCatchRate(rate << 1, 0);
        if ((playerLevel >> 1) > enemyLevel) {
          rate = clampCatchRate(rate << 1, 0);
          if ((playerLevel >> 2) > enemyLevel) {
            rate = clampCatchRate(rate << 1, 0);
          }
        }
      }
      skipHpCalc = true;
      break;
    }
    case 'LURE_BALL': {
      const battleType = String(gameState?.wram.battle_type ?? '').toUpperCase();
      if (battleType === 'BATTLETYPE_FISH') {
        rate = clampCatchRate(rate * 3, 0);
      }
      break;
    }
    case 'MOON_BALL': {
      // Crystal bug: the routine compares against Pokemon Red's Moon Stone id
      // (Burn Heal in GSC), so Moon Ball never boosts a legal species.
      break;
    }
    case 'LOVE_BALL': {
      const player = context.playerPokemon;
      if (player.species.id === enemy.species.id) {
        if (player.gender === enemy.gender && player.gender !== null) {
          rate = clampCatchRate(rate * 8, 0);
        }
      }
      break;
    }
    case 'FAST_BALL':
      // Crystal bug: intended to scan all flee tables, but only checks the
      // first three entries of the 10% flee table.
      if (CRYSTAL_FAST_BALL_SPECIES.has(String(enemy.species.id).toUpperCase())) {
        rate = clampCatchRate(rate * 4, 0);
      }
      break;
    default:
      break;
  }

  return { rate, skipHpCalc };
};

const computeFinalCatchRate = (
  ballName: string,
  enemy: Pokemon,
  context: BattleContext,
  gameState: GameState | null,
): number => {
  const { rate, skipHpCalc } = applyBallMultiplier(ballName, enemy.species.catch_rate, enemy, context, gameState);
  if (skipHpCalc) {
    return clampCatchRate(rate, 0);
  }
  let finalRate = computeHpAdjustedCatchRate(rate, enemy.hp, enemy.max_hp);
  if (enemy.status === StatusCondition.SLEEP || enemy.status === StatusCondition.FREEZE) {
    finalRate = clampCatchRate(finalRate + 10, 1);
  }
  return clampCatchRate(finalRate, 1);
};

export const __test__computeFinalCatchRate = computeFinalCatchRate;

const wobbleChanceForRate = (finalCatchRate: number): number => {
  const rate = clampCatchRate(finalCatchRate, 0);
  for (const [threshold, chance] of WOBBLE_PROBABILITIES) {
    if (rate <= threshold) {
      return chance;
    }
  }
  return 255;
};

const resolveCatchOutcome = (
  ballName: string,
  enemy: Pokemon,
  context: BattleContext,
  gameState: GameState | null,
  { forceCatch = false }: { forceCatch?: boolean } = {},
): CatchOutcome => {
  const finalCatchRate = computeFinalCatchRate(ballName, enemy, context, gameState);
  if (forceCatch) {
    return { caught: true, wobbleCount: 3, finalCatchRate };
  }
  if (!gameState) {
    return { caught: false, wobbleCount: 0, finalCatchRate };
  }
  const rng = new HardwareRNG(gameState);
  const roll = rng.nextByte();
  if (roll <= finalCatchRate) {
    return { caught: true, wobbleCount: 3, finalCatchRate };
  }

  const wobbleChance = wobbleChanceForRate(finalCatchRate);
  let wobbleCount = 0;
  for (let i = 0; i < 3; i += 1) {
    if (rng.nextByte() < wobbleChance) {
      wobbleCount += 1;
    } else {
      break;
    }
  }
  return { caught: false, wobbleCount, finalCatchRate };
};

const formatBreakFreeText = (nickname: string, wobbleCount: number): string => {
  const count = Math.max(0, Math.min(3, Math.trunc(wobbleCount)));
  if (count === 0) {
    return `Oh no! The ${nickname}\nbroke free!`;
  }
  if (count === 1) {
    return 'Aww! It appeared\nto be caught!';
  }
  if (count === 2) {
    return 'Aargh!\nAlmost had it!';
  }
  return 'Shoot! It was so\nclose too!';
};
export function catchPokemon(
  item: Item,
  eventManager: EventManager,
  context: BattleContext,
  gameState: GameState | null,
): boolean {
  const enemy = context.enemyPokemon;

  if (context.trainerBattle) {
    eventManager.dispatch(
      new Event('show_text', { text: 'The opposing trainer blocked the BALL!' })
    );
    return false;
  }

  const battleType = String(gameState?.wram.battle_type ?? '').toUpperCase();
  const isContestBattle = [
    'BATTLETYPE_CONTEST',
    'BATTLETYPE_BUG_CONTEST',
    'BATTLETYPE_PARK',
  ].includes(battleType);
  const isTutorialBattle = battleType === 'BATTLETYPE_TUTORIAL';
  const ballName = (isContestBattle ? 'PARK_BALL' : item.script_name).trim().toUpperCase();

  if (isContestBattle) {
    if (!gameState) {
      return false;
    }
    const contestState = gameState.wram.bug_contest_state;
    if ((contestState.park_balls_remaining ?? 0) <= 0) {
      eventManager.dispatch(
        new Event('show_text', { text: "You're out of PARK BALLs!" })
      );
      return false;
    }
  }
  if (!isContestBattle && gameState && !hasCaptureStorageSpace(gameState)) {
    dispatchCatchText(eventManager, 'The BOX is full!');
    return false;
  }
  // ASM: engine/items/item_effects.asm::PokeBallEffect already prints ItemUsedText before the animation.

  if (isTutorialBattle) {
    dispatchPokeballAnimation(eventManager, gameState, 4, { ballName });
    return onTutorialCatchSuccess(enemy, eventManager, context);
  }

  const outcome = resolveCatchOutcome(ballName, enemy, context, gameState, {
    forceCatch: ballName === 'MASTER_BALL',
  });
  const shakeCount = outcome.caught ? 4 : outcome.wobbleCount;
  dispatchPokeballAnimation(eventManager, gameState, shakeCount, { ballName });

  let success = false;
  if (outcome.caught) {
    success = isContestBattle
      ? onContestCatchSuccess(enemy, eventManager, context, gameState)
      : onCatchSuccess(enemy, eventManager, context, gameState);
  } else {
    dispatchCatchText(eventManager, formatBreakFreeText(enemy.nickname, outcome.wobbleCount));
  }

  if (isContestBattle && gameState) {
    spendParkBall(gameState);
  }
  return success;
}

function applyPokeBallEffect(
  item: Item,
  eventManager: EventManager,
  context: BattleContext,
  gameState: GameState | null,
): boolean {
  return catchPokemon(item, eventManager, context, gameState);
}

function applyStatusHealEffect(
  item: Item,
  targetPokemon: Pokemon,
  eventManager: EventManager,
): boolean {
  const name = item.script_name;
  let cleared = false;
  const hasStatus =
    targetPokemon.status !== undefined &&
    targetPokemon.status !== null &&
    targetPokemon.status !== StatusCondition.NONE;

  if (CONFUSION_ITEMS.has(name) && targetPokemon.confusion_turns > 0) {
    targetPokemon.confusion_turns = 0;
    cleared = true;
  }

  if (STATUS_CLEAR_ALL.has(name)) {
    if (hasStatus) {
      targetPokemon.status = undefined;
      cleared = true;
    }
    if (targetPokemon.sleep_turns > 0) {
      targetPokemon.sleep_turns = 0;
      cleared = true;
    } else {
      targetPokemon.sleep_turns = 0;
    }
    if (!cleared) {
      dispatchNoEffect(eventManager);
    }
    return false;
  }

  const expectedStatus = STATUS_ITEM_MAP[name];
  if (expectedStatus === undefined) {
    if (!cleared) {
      dispatchNoEffect(eventManager);
    }
    return cleared;
  }

  if (hasStatus && targetPokemon.status === expectedStatus) {
    targetPokemon.status = undefined;
    targetPokemon.sleep_turns = 0;
    return false;
  }

  if (!cleared) {
    dispatchNoEffect(eventManager);
  }
  return cleared;
}

function applyFullRestoreEffect(
  targetPokemon: Pokemon,
  eventManager: EventManager,
): boolean {
  const hasStatus =
    targetPokemon.status !== undefined &&
    targetPokemon.status !== null &&
    targetPokemon.status !== StatusCondition.NONE;
  const hasSleep = targetPokemon.sleep_turns > 0;
  const hasConfusion = targetPokemon.confusion_turns > 0;
  if (targetPokemon.hp >= targetPokemon.max_hp && !hasStatus && !hasSleep && !hasConfusion) {
    dispatchNoEffect(eventManager);
    return false;
  }
  targetPokemon.hp = targetPokemon.max_hp;
  targetPokemon.status = undefined;
  targetPokemon.sleep_turns = 0;
  targetPokemon.confusion_turns = 0;
  return false;
}

function healingAmount(item: Item, targetPokemon: Pokemon): number {
  if (item.parameter === -1 || ['MAX_POTION', 'FULL_RESTORE'].includes(item.script_name)) {
    return targetPokemon.max_hp;
  }
  if (item.parameter > 0) {
    return item.parameter;
  }

  const match = item.description?.match(/(\d+)/);
  if (match) {
    return Number(match[1]);
  }

  throw new Error(`Unable to determine healing amount for ${item.name}`);
}

function applyRestoreHpEffect(
  item: Item,
  targetPokemon: Pokemon,
  eventManager: EventManager,
): boolean {
  if (targetPokemon.hp >= targetPokemon.max_hp) {
    dispatchNoEffect(eventManager);
    return false;
  }
  const heal = healingAmount(item, targetPokemon);
  targetPokemon.hp = Math.min(targetPokemon.max_hp, targetPokemon.hp + heal);
  return false;
}

function applyRestorePpEffect(
  item: Item,
  targetPokemon: Pokemon,
  eventManager: EventManager,
  moveIndex: number | null,
): boolean {
  const restoreAllMoves = ['ELIXER', 'MAX_ELIXER'].includes(item.script_name);
  const restoreValue = item.parameter !== 0 && item.parameter !== -1 ? item.parameter : null;

  if (!targetPokemon.moves) {
    dispatchNoEffect(eventManager);
    return false;
  }

  let restored = false;
  const restoreMovePp = (index: number): void => {
    const move = targetPokemon.moves[index];
    if (!move) {
      return;
    }
    const moveData = movesMap.get(move.name);
    if (!moveData) {
      return;
    }
    const restoreAmount = item.parameter === -1 ? moveData.pp : restoreValue ?? 10;
    const nextPp = Math.min(move.current_pp + restoreAmount, moveData.pp);
    if (nextPp !== move.current_pp) {
      restored = true;
    }
    move.current_pp = nextPp;
  };

  if (restoreAllMoves) {
    for (let index = 0; index < targetPokemon.moves.length; index += 1) {
      restoreMovePp(index);
    }
    if (!restored) {
      dispatchNoEffect(eventManager);
    }
    return false;
  }

  const targetIndex = moveIndex === null ? 0 : moveIndex;
  if (targetIndex >= 0 && targetIndex < targetPokemon.moves.length) {
    restoreMovePp(targetIndex);
  }
  if (!restored) {
    dispatchNoEffect(eventManager);
  }
  return false;
}

function applyReviveEffect(
  item: Item,
  targetPokemon: Pokemon,
  eventManager: EventManager,
): boolean {
  if (targetPokemon.hp === 0) {
    if (item.script_name === 'REVIVE') {
      targetPokemon.hp = Math.max(1, Math.floor(targetPokemon.max_hp / 2));
    } else if (['MAX_REVIVE', 'REVIVAL_HERB'].includes(item.script_name)) {
      targetPokemon.hp = targetPokemon.max_hp;
    }
    return false;
  }
  dispatchNoEffect(eventManager);
  return false;
}

export function _apply_x_item_effect(
  item: Item,
  targetPokemon: Pokemon,
  eventManager: EventManager,
): boolean {
  if (!targetPokemon.stat_boosts) {
    targetPokemon.stat_boosts = {
      HP: 0,
      ATTACK: 0,
      DEFENSE: 0,
      SPEED: 0,
      SPECIAL_ATTACK: 0,
      SPECIAL_DEFENSE: 0,
      ACCURACY: 0,
      EVASION: 0,
    };
  }
  let statToBoost: Stat | undefined;
  let statName = '';

  if (item.script_name === 'X_ATTACK') {
    statToBoost = Stat.ATTACK;
    statName = 'ATTACK';
  } else if (item.script_name === 'X_DEFEND') {
    statToBoost = Stat.DEFENSE;
    statName = 'DEFENSE';
  } else if (item.script_name === 'X_SPEED') {
    statToBoost = Stat.SPEED;
    statName = 'SPEED';
  } else if (item.script_name === 'X_SPECIAL') {
    statToBoost = Stat.SPECIAL_ATTACK;
    statName = 'SPECIAL ATTACK';
    // In Gen 2, X SPECIAL only boosts Special Attack, not Special Defense.
    // We are adhering to this behavior.
    const currentBoost = targetPokemon.stat_boosts[statToBoost] ?? 0;
    if (currentBoost < 6) {
      targetPokemon.stat_boosts[statToBoost] = currentBoost + 1;
      eventManager.dispatch(
        new Event('show_text', {
          text: `${targetPokemon.nickname}'s ${statName} rose!`,
        })
      );
    } else {
      eventManager.dispatch(
        new Event('show_text', {
          text: `${targetPokemon.nickname}'s ${statName} won't go any higher!`,
        })
      );
    }
    return false;
  } else if (item.script_name === 'X_ACCURACY') {
    statToBoost = Stat.ACCURACY;
    statName = 'ACCURACY';
  }

  if (statToBoost) {
    const currentBoost = targetPokemon.stat_boosts[statToBoost] ?? 0;
    if (currentBoost < 6) {
      targetPokemon.stat_boosts[statToBoost] = currentBoost + 1;
      eventManager.dispatch(
        new Event('show_text', {
          text: `${targetPokemon.nickname}'s ${statName} rose!`,
        })
      );
    } else {
      eventManager.dispatch(
        new Event('show_text', {
          text: `${targetPokemon.nickname}'s ${statName} won't go any higher!`,
        })
      );
    }
  }

  return false;
}

function applyGuardSpecEffect(
  targetPokemon: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
): boolean {
  const side = context.sideFor(targetPokemon);
  if (side !== undefined) {
    context.setBarrier(side, 'mist', 5);
    eventManager.dispatch(
      new Event('show_text', { text: `${targetPokemon.nickname} became shrouded in MIST!` })
    );
  }
  return false;
}

function applyDireHitEffect(targetPokemon: Pokemon, eventManager: EventManager): boolean {
  targetPokemon.focus_energy = true;
  eventManager.dispatch(
    new Event('show_text', { text: `${targetPokemon.nickname} is getting pumped!` })
  );
  return false;
}

function applyPokeDollEffect(eventManager: EventManager, context: BattleContext): boolean {
  if (context.trainerBattle) {
    eventManager.dispatch(new Event('show_text', { text: 'But it failed!' }));
    return false;
  }

  context.runAttemptSuccess = true;
  eventManager.dispatch(new Event('show_text', { text: 'Got away safely!' }));
  return true;
}

function onCatchSuccess(
  enemy: Pokemon,
  eventManager: EventManager,
  _context: BattleContext,
  gameState: GameState | null,
): boolean {
  dispatchCatchText(eventManager, `You caught ${enemy.nickname}!`);
  const capturedHp = Math.max(0, Math.trunc(enemy.hp));
  const capturedStatus = enemy.status;

  if (gameState) {
    const captured: Pokemon = {
      ...enemy,
      dvs: { ...enemy.dvs },
      stat_boosts: { ...enemy.stat_boosts },
      moves: enemy.moves ? enemy.moves.map((move) => (move ? { ...move } : move)) : [],
      status: capturedStatus,
      hp: capturedHp,
      original_trainer_name: gameState.sram.player_name || 'PLAYER',
      original_trainer_id: gameState.sram.player_id,
    };
    recordPokedexCaught(gameState, enemy);
    const storageLocation = registerCapture(gameState, captured);

    eventManager.dispatch(
      new Event('nickname_prompt', {
        pokemon: captured,
        species_name: captured.species.id,
        wait_for_animation: true,
      })
    );

    if (storageLocation === 'pc') {
      const nickname = (captured.nickname || captured.species.id).replace(/_/g, ' ');
      dispatchCatchText(eventManager, `${nickname} was\nsent to BILL's PC.`);
    }
  }

  return true;
}

function onTutorialCatchSuccess(
  enemy: Pokemon,
  eventManager: EventManager,
  _context: BattleContext,
): boolean {
  dispatchCatchText(eventManager, `Gotcha! ${enemy.nickname}\nwas caught!`);
  return true;
}

function onContestCatchSuccess(
  enemy: Pokemon,
  eventManager: EventManager,
  _context: BattleContext,
  gameState: GameState | null,
): boolean {
  const capturedHp = Math.max(0, Math.trunc(enemy.hp));
  const capturedStatus = enemy.status;

  if (gameState) {
    const captured: Pokemon = {
      ...enemy,
      dvs: { ...enemy.dvs },
      stat_boosts: { ...enemy.stat_boosts },
      moves: enemy.moves ? enemy.moves.map((move) => (move ? { ...move } : move)) : [],
      status: capturedStatus,
      hp: capturedHp,
    };
    recordPokedexCaught(gameState, enemy);
    bug_contest_set_caught_contest_mon(gameState, {
      event_manager: eventManager,
      caught_mon: captured,
    });
  }
  return true;
}

function hasCaptureStorageSpace(gameState: GameState): boolean {
  if (gameState.sram.party?.pokemon.some((slot) => slot === null)) {
    return true;
  }
  const boxes = gameState.sram.pc_boxes;
  if (!boxes.length || boxes.length < MAX_PC_BOXES) {
    return true;
  }
  return boxes.some((box) => box.pokemon.some((slot) => slot === null));
}

function registerCapture(gameState: GameState, pokemon: Pokemon): 'party' | 'pc' {
  const ensureCanonicalBox = (index: number): void => {
    const boxes = gameState.sram.pc_boxes;
    while (boxes.length <= index) {
      boxes.push(BoxSchema.parse({ name: formatDefaultBoxName(boxes.length) }));
    }
    const box = boxes[index];
    if (!box.name || !box.name.trim()) {
      box.name = formatDefaultBoxName(index);
    }
    const normalized = BoxSchema.parse({ ...box, name: box.name });
    boxes[index] = normalized;
  };

  const party = gameState.sram.party;
  if (party && addPartyPokemon(party, pokemon)) {
    return 'party';
  }

  const boxes = gameState.sram.pc_boxes;
  if (!boxes.length) {
    boxes.push(BoxSchema.parse({ name: formatDefaultBoxName(0) }));
  }
  for (let boxIndex = 0; boxIndex < boxes.length; boxIndex += 1) {
    ensureCanonicalBox(boxIndex);
    const box = boxes[boxIndex];
    if (addBoxPokemon(box, pokemon)) {
      return 'pc';
    }
  }

  if (boxes.length < MAX_PC_BOXES) {
    const index = boxes.length;
    ensureCanonicalBox(index);
    const box = boxes[index];
    if (addBoxPokemon(box, pokemon)) {
      return 'pc';
    }
  }

  throw new Error('Party and PC boxes are full; cannot store captured Pokemon.');
}

function shouldPlayBattleScene(gameState: GameState | null): boolean {
  if (gameState?.wram?.instant_mode) {
    return false;
  }
  const options = gameState?.sram?.options as { battle_scene?: unknown } | undefined;
  if (!options || options.battle_scene === undefined) {
    return true;
  }
  const setting = options.battle_scene as unknown;
  if (typeof setting === 'boolean') {
    return setting;
  }
  return setting === BattleScene.ON;
}

function dispatchPokeballAnimation(
  eventManager: EventManager | null,
  gameState: GameState | null,
  shakes: number | null,
  { ballName }: { ballName?: string } = {},
): void {
  if (shakes === null || shakes === undefined) {
    return;
  }
  if (!shouldPlayBattleScene(gameState)) {
    return;
  }
  if (!eventManager?.dispatch) {
    return;
  }
  const shakeCount = Math.max(0, Math.min(Math.trunc(shakes), 4));
  const ballLabel = (ballName || 'POKE_BALL').trim().toUpperCase();
  eventManager.dispatch(
    new Event('play_animation', {
      move_name: 'Throw Poke Ball',
      is_player_move: true,
      param: 0,
      param_label: ballLabel,
      shake_count: shakeCount,
    })
  );
}

function spendParkBall(gameState: GameState | null): void {
  if (!gameState) {
    return;
  }
  const contestState = gameState.wram.bug_contest_state;
  contestState.park_balls_remaining = Math.max(0, (contestState.park_balls_remaining ?? 0) - 1);
}

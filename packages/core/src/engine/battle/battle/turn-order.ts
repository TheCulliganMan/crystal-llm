import { Pokemon, Move as MoveData } from '@pokecrystal/core/core/models';
import { BattleActionType, BattleTurn, MoveName, Stat, StatusCondition } from '@pokecrystal/core/core/enums';
import { Battle } from './battle-logic';
import { applyStage, calculateBattleStat } from './stats';
import { HardwareRNG } from '@pokecrystal/core/engine/games/rng';
import { BattleAction } from './battle-context';
import { getBattleItem } from './item-lookup';

const movesMap: Map<MoveName, MoveData> = new Map();

function getItemHeldEffect(itemName?: string | null): string | undefined {
  return getBattleItem(itemName)?.held_effect;
}

function quickClawChance(battle: Battle, side: BattleTurn, pokemon: Pokemon): boolean {
  const heldEffect = getItemHeldEffect(pokemon.item || undefined);
  if (heldEffect !== 'HELD_QUICK_CLAW') {
    return false;
  }

  const item = getBattleItem(pokemon.item);
  const chance = item?.parameter ?? 60;

  const rng = new HardwareRNG(battle.gameState);
  if (rng.randrange(256) < chance) {
    if (side === BattleTurn.PLAYER) {
      battle.context.playerQuickClawActivated = true;
    } else {
      battle.context.enemyQuickClawActivated = true;
    }
    return true;
  }
  return false;
}


function calculateSpeed(battle: Battle, _side: BattleTurn, pokemon: Pokemon): number {
  return calculateBattleStat(battle, pokemon, Stat.SPEED);
}

function getMovePriority(moveName?: MoveName): number {
  if (!moveName) {
    return 0;
  }

  const HIGH_PRIORITY = new Set([
    MoveName.PROTECT,
    MoveName.DETECT,
    MoveName.ENDURE,
    MoveName.MACH_PUNCH,
    MoveName.QUICK_ATTACK,
    MoveName.EXTREMESPEED,
  ]);
  const LOW_PRIORITY = new Set([
    MoveName.VITAL_THROW,
    MoveName.COUNTER,
    MoveName.MIRROR_COAT,
    MoveName.ROAR,
    MoveName.WHIRLWIND,
  ]);

  if (HIGH_PRIORITY.has(moveName)) {
    return 1;
  }
  if (LOW_PRIORITY.has(moveName)) {
    return -1;
  }
  return 0;
}

export function determineTurnOrder(battle: Battle): BattleTurn[] {
  const { playerAction, enemyAction } = battle.context;

  let playerPriority = 0;
  if (playerAction?.actionType === BattleActionType.MOVE) {
    playerPriority = getMovePriority(playerAction.moveName);
  } else if (
    playerAction &&
    [BattleActionType.ITEM, BattleActionType.SWITCH, BattleActionType.RUN].includes(playerAction.actionType)
  ) {
    playerPriority = 10;
  }

  let enemyPriority = 0;
  if (enemyAction?.actionType === BattleActionType.MOVE) {
    enemyPriority = getMovePriority(enemyAction.moveName);
  } else if (
    enemyAction &&
    [BattleActionType.ITEM, BattleActionType.SWITCH, BattleActionType.RUN].includes(enemyAction.actionType)
  ) {
    enemyPriority = 10;
  }

  if (playerPriority !== enemyPriority) {
    return playerPriority > enemyPriority ? [BattleTurn.PLAYER, BattleTurn.ENEMY] : [BattleTurn.ENEMY, BattleTurn.PLAYER];
  }

  const playerClaw = quickClawChance(battle, BattleTurn.PLAYER, battle.context.playerPokemon);
  const enemyClaw = quickClawChance(battle, BattleTurn.ENEMY, battle.context.enemyPokemon);

  if (playerClaw && !enemyClaw) {
    return [BattleTurn.PLAYER, BattleTurn.ENEMY];
  }
  if (enemyClaw && !playerClaw) {
    return [BattleTurn.ENEMY, BattleTurn.PLAYER];
  }

  const playerSpeed = calculateSpeed(battle, BattleTurn.PLAYER, battle.context.playerPokemon);
  const enemySpeed = calculateSpeed(battle, BattleTurn.ENEMY, battle.context.enemyPokemon);

  if (playerSpeed > enemySpeed) {
    return [BattleTurn.PLAYER, BattleTurn.ENEMY];
  }
  if (enemySpeed > playerSpeed) {
    return [BattleTurn.ENEMY, BattleTurn.PLAYER];
  }

  const rng = new HardwareRNG(battle.gameState);
  return rng.coinFlip(0.5) ? [BattleTurn.PLAYER, BattleTurn.ENEMY] : [BattleTurn.ENEMY, BattleTurn.PLAYER];
}

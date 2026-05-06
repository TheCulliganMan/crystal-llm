import { z } from 'zod';

export enum BattleActionType {
  MOVE = 'move',
  SWITCH = 'switch',
  ITEM = 'item',
  RUN = 'run',
}

export enum BattleTurn {
  PLAYER = 'player',
  ENEMY = 'enemy',
}

export enum StatusCondition {
  NONE = 'none',
  POISON = 'poison',
  SLEEP = 'sleep',
  PARALYSIS = 'paralysis',
  BURN = 'burn',
  FREEZE = 'freeze',
  CONFUSION = 'confusion',
}
export const StatusConditionSchema = z.nativeEnum(StatusCondition);

export enum AILayer {
  AI_BASIC = 'AI_BASIC',
  AI_SETUP = 'AI_SETUP',
  AI_TYPES = 'AI_TYPES',
  AI_OFFENSIVE = 'AI_OFFENSIVE',
  AI_SMART = 'AI_SMART',
  AI_OPPORTUNIST = 'AI_OPPORTUNIST',
  AI_AGGRESSIVE = 'AI_AGGRESSIVE',
  AI_CAUTIOUS = 'AI_CAUTIOUS',
  AI_STATUS = 'AI_STATUS',
  AI_RISKY = 'AI_RISKY',
  AI_NONE = 'AI_NONE',
}

export enum BattleScene {
  ON = 'on',
  OFF = 'off',
}

export enum BattleStyle {
  SET = 'set',
  SHIFT = 'shift',
}

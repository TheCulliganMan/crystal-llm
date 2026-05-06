/**
 * Helpers for working with overworld collision data exported from the RGBDS
 * disassembly.
 *
 * This module hardcodes data from authoritative ASM sources:
 *
 * * `constants/collision_constants.asm` provides the numeric value attached to
 *   every `COLL_*` constant used by the `tilecoll` macro.
 */

import { getDataDir } from '@pokecrystal/core/core/paths';
import { readJsonAssetSync } from '@pokecrystal/core/core/asset-reader';
import path from 'path';

export enum Terrain {
  LAND = 'land',
  WATER = 'water',
  WALL = 'wall',
}

export interface CollisionAttributes {
  readonly value: number;
  readonly terrain: Terrain;
  readonly talk: boolean;
  readonly raw_expr: string;
  readonly comment: string | null;
}

const parseNumeric = (token: string): number => {
  const trimmed = token.trim();
  if (trimmed.startsWith('$')) {
    return parseInt(trimmed.substring(1), 16);
  }
  if (trimmed.startsWith('0x')) {
    // Note: parseInt handles '0x' prefix automatically
    return parseInt(trimmed, 16);
  }
  return parseInt(trimmed, 10);
};

// Manually ported from constants/collision_constants.asm
const COLLISION_CONSTANTS_RAW: { [key: string]: number } = {
  COLL_FLOOR: 0x00,
  COLL_01: 0x01,
  COLL_03: 0x03,
  COLL_04: 0x04,
  COLL_WALL: 0x07,
  COLL_CUT_08: 0x08,
  COLL_TALL_GRASS_10: 0x10,
  COLL_CUT_TREE: 0x12,
  COLL_LONG_GRASS: 0x14,
  COLL_HEADBUTT_TREE: 0x15,
  COLL_TALL_GRASS: 0x18,
  COLL_CUT_TREE_1A: 0x1a,
  COLL_LONG_GRASS_1C: 0x1c,
  COLL_HEADBUTT_TREE_1D: 0x1d,
  COLL_WATER_21: 0x21,
  COLL_ICE: 0x23,
  COLL_WHIRLPOOL: 0x24,
  COLL_BUOY: 0x27,
  COLL_CUT_28: 0x28,
  COLL_WATER: 0x29,
  COLL_ICE_2B: 0x2b,
  COLL_WHIRLPOOL_2C: 0x2c,
  COLL_WATERFALL_RIGHT: 0x30,
  COLL_WATERFALL_LEFT: 0x31,
  COLL_WATERFALL_UP: 0x32,
  COLL_WATERFALL: 0x33,
  COLL_CURRENT_RIGHT: 0x38,
  COLL_CURRENT_LEFT: 0x39,
  COLL_CURRENT_UP: 0x3a,
  COLL_CURRENT_DOWN: 0x3b,
  COLL_BRAKE: 0x40,
  COLL_WALK_RIGHT: 0x41,
  COLL_WALK_LEFT: 0x42,
  COLL_WALK_UP: 0x43,
  COLL_WALK_DOWN: 0x44,
  COLL_BRAKE_45: 0x45,
  COLL_BRAKE_46: 0x46,
  COLL_BRAKE_47: 0x47,
  COLL_GRASS_48: 0x48,
  COLL_GRASS_49: 0x49,
  COLL_GRASS_4A: 0x4a,
  COLL_GRASS_4B: 0x4b,
  COLL_GRASS_4C: 0x4c,
  COLL_WALK_RIGHT_ALT: 0x50,
  COLL_WALK_LEFT_ALT: 0x51,
  COLL_WALK_UP_ALT: 0x52,
  COLL_WALK_DOWN_ALT: 0x53,
  COLL_BRAKE_ALT: 0x54,
  COLL_BRAKE_55: 0x55,
  COLL_BRAKE_56: 0x56,
  COLL_BRAKE_57: 0x57,
  COLL_5B: 0x5b,
  COLL_PIT: 0x60,
  COLL_VIRTUAL_BOY: 0x61,
  COLL_64: 0x64,
  COLL_65: 0x65,
  COLL_PIT_68: 0x68,
  COLL_WARP_CARPET_DOWN: 0x70,
  COLL_DOOR: 0x71,
  COLL_LADDER: 0x72,
  COLL_STAIRCASE_73: 0x73,
  COLL_CAVE_74: 0x74,
  COLL_DOOR_75: 0x75,
  COLL_WARP_CARPET_LEFT: 0x76,
  COLL_WARP_77: 0x77,
  COLL_WARP_CARPET_UP: 0x78,
  COLL_DOOR_79: 0x79,
  COLL_STAIRCASE: 0x7a,
  COLL_CAVE: 0x7b,
  COLL_WARP_PANEL: 0x7c,
  COLL_DOOR_7D: 0x7d,
  COLL_WARP_CARPET_RIGHT: 0x7e,
  COLL_WARP_7F: 0x7f,
  COLL_COUNTER: 0x90,
  COLL_BOOKSHELF: 0x91,
  COLL_PC: 0x93,
  COLL_RADIO: 0x94,
  COLL_TOWN_MAP: 0x95,
  COLL_MART_SHELF: 0x96,
  COLL_TV: 0x97,
  COLL_COUNTER_98: 0x98,
  COLL_9C: 0x9c,
  COLL_WINDOW: 0x9d,
  COLL_INCENSE_BURNER: 0x9f,
  COLL_HOP_RIGHT: 0xa0,
  COLL_HOP_LEFT: 0xa1,
  COLL_HOP_UP: 0xa2,
  COLL_HOP_DOWN: 0xa3,
  COLL_HOP_DOWN_RIGHT: 0xa4,
  COLL_HOP_DOWN_LEFT: 0xa5,
  COLL_HOP_UP_RIGHT: 0xa6,
  COLL_HOP_UP_LEFT: 0xa7,
  COLL_RIGHT_WALL: 0xb0,
  COLL_LEFT_WALL: 0xb1,
  COLL_UP_WALL: 0xb2,
  COLL_DOWN_WALL: 0xb3,
  COLL_DOWN_RIGHT_WALL: 0xb4,
  COLL_DOWN_LEFT_WALL: 0xb5,
  COLL_UP_RIGHT_WALL: 0xb6,
  COLL_UP_LEFT_WALL: 0xb7,
  COLL_RIGHT_BUOY: 0xc0,
  COLL_LEFT_BUOY: 0xc1,
  COLL_UP_BUOY: 0xc2,
  COLL_DOWN_BUOY: 0xc3,
  COLL_DOWN_RIGHT_BUOY: 0xc4,
  COLL_DOWN_LEFT_BUOY: 0xc5,
  COLL_UP_RIGHT_BUOY: 0xc6,
  COLL_UP_LEFT_BUOY: 0xc7,
  COLL_FF: 0xff,
};

// Also expose the suffix without the COLL_ prefix for convenience.
const collisionConstants: { [key: string]: number } = {
  ...COLLISION_CONSTANTS_RAW,
};
for (const [key, value] of Object.entries(COLLISION_CONSTANTS_RAW)) {
  if (key.startsWith('COLL_')) {
    const shortKey = key.substring('COLL_'.length);
    collisionConstants[shortKey] = value;
  }
}

/**
 * Resolve a collision token used inside `tilecoll` (e.g. `WALL` or `FF`)
 * to its numeric `COLL_*` value.
 */

let _collisionPermissionsCache: Map<number, CollisionAttributes> | null = null;
const COLLISION_PERMISSIONS_PATH = path.join(
  getDataDir(),
  'collision',
  'collision_permissions.json'
);

function determineTerrain(exprTokens: Set<string>): Terrain {
  if (exprTokens.has('WALL_TILE')) {
    return Terrain.WALL;
  }
  if (exprTokens.has('WATER_TILE')) {
    return Terrain.WATER;
  }
  if (exprTokens.has('LAND_TILE')) {
    return Terrain.LAND;
  }
  return Terrain.LAND;
}

function loadCollisionPermissions(): Map<number, CollisionAttributes> {
  if (_collisionPermissionsCache) {
    return _collisionPermissionsCache;
  }

  const attributes = new Map<number, CollisionAttributes>();
  let payload: unknown;
  try {
    payload = readJsonAssetSync<unknown>(COLLISION_PERMISSIONS_PATH);
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    throw new Error(
      `Missing collision permissions asset at ${COLLISION_PERMISSIONS_PATH}: ${reason}`
    );
  }
  if (!Array.isArray(payload)) {
    throw new Error(
      `Collision permissions asset at ${COLLISION_PERMISSIONS_PATH} did not define any entries.`
    );
  }

  for (const entry of payload) {
    if (!entry || typeof entry !== 'object') {
      continue;
    }
    const record = entry as Partial<CollisionAttributes>;
    if (typeof record.value !== 'number' || typeof record.raw_expr !== 'string') {
      continue;
    }
    attributes.set(record.value, {
      value: record.value,
      terrain: record.terrain ?? determineTerrain(new Set(record.raw_expr.split('|').map(token => token.trim()))),
      talk: Boolean(record.talk),
      raw_expr: record.raw_expr,
      comment: record.comment ?? null,
    });
  }
  if (!attributes.size) {
    throw new Error(
      `Collision permissions asset at ${COLLISION_PERMISSIONS_PATH} did not define any entries.`
    );
  }
  _collisionPermissionsCache = attributes;
  return attributes;
}

export function describeCollision(value: number): CollisionAttributes {
  const permissions = loadCollisionPermissions();
  const attributes = permissions.get(value);
  if (!attributes) {
    throw new Error(`No collision permission entry for value ${value.toString(16)}`);
  }
  return attributes;
}

export const resolveCollisionValue = (token: string): number => {
  const cleaned = token.trim();
  if (!cleaned) {
    throw new Error('Cannot resolve empty collision token.');
  }

  const lookupKey = cleaned.toUpperCase();
  if (lookupKey in collisionConstants) {
    return collisionConstants[lookupKey];
  }

  // Support values written without the COLL_ prefix, hex without $ and decimal.
  try {
    return parseNumeric(cleaned);
  } catch (exc) {
    throw new Error(`Unknown collision token '${token}'.`);
  }
};

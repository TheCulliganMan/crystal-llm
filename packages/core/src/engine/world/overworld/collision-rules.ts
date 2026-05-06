/**
 * Shared helpers that faithfully mirror the overworld collision rules.
 */

import { getDataDir } from '@pokecrystal/core/core/paths';
import { readJsonAssetSync } from '@pokecrystal/core/core/asset-reader';
import { OverworldMap } from '@pokecrystal/core/engine/world/overworld/overworld-map';
import { METATILE_WIDTH } from '@pokecrystal/core/core/tileset-data';
import type { OverworldTilesetLike } from '@pokecrystal/core/engine/world/overworld/tileset-types';
import { FacingDirection } from '@pokecrystal/core/core/enums/overworld';
import {
  Terrain,
  describeCollision,
  resolveCollisionValue,
} from './collision-data';
import { PlayerState } from '@pokecrystal/core/core/enums/overworld';
import path from 'path';

export const SURF_STATES: readonly PlayerState[] = [
  PlayerState.SURF,
  PlayerState.SURF_PIKA,
];

export class CollisionSample {
  public readonly permission: number;
  public readonly metatileId: number;
  public readonly quadrant: number;
  public readonly tileX: number;
  public readonly tileY: number;
  public readonly stdScript: string | null;

  constructor(
    permission: number,
    metatileId: number,
    quadrant: number,
    tileX: number,
    tileY: number,
    stdScript: string | null = null
  ) {
    this.permission = permission;
    this.metatileId = metatileId;
    this.quadrant = quadrant;
    this.tileX = tileX;
    this.tileY = tileY;
    this.stdScript = stdScript;
  }
}

const COLLISION_STD_SCRIPTS_PATH = path.join(
  getDataDir(),
  'collision',
  'collision_stdscripts.json'
);

let _collisionStdScriptsCache: Map<number, string> | null = null;

export function loadCollisionStdScripts(): Map<number, string> {
  if (_collisionStdScriptsCache) {
    return _collisionStdScriptsCache;
  }

  let payload: unknown;
  try {
    payload = readJsonAssetSync<unknown>(COLLISION_STD_SCRIPTS_PATH);
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    throw new Error(
      `Missing standard collision scripts asset at ${COLLISION_STD_SCRIPTS_PATH}: ${reason}`
    );
  }

  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    throw new Error(
      `Standard collision scripts asset at ${COLLISION_STD_SCRIPTS_PATH} did not define any entries.`
    );
  }

  const mapping = new Map<number, string>();
  for (const [constant, scriptName] of Object.entries(payload as Record<string, unknown>)) {
    if (typeof scriptName !== 'string' || !scriptName.trim()) {
      continue;
    }
    const value = resolveCollisionValue(constant);
    mapping.set(value, scriptName.trim());
  }
  if (!mapping.size) {
    throw new Error(
      `Standard collision scripts asset at ${COLLISION_STD_SCRIPTS_PATH} did not define any entries.`
    );
  }
  _collisionStdScriptsCache = mapping;
  return mapping;
}

export function getCollisionStdScript(permission: number): string | null {
  return loadCollisionStdScripts().get(permission) ?? null;
}

let _sideWallHi: number | null = null;
function getSideWallHi(): number {
  if (_sideWallHi === null) {
    _sideWallHi = resolveCollisionValue('RIGHT_WALL') & 0xf0;
  }
  return _sideWallHi;
}

let _sideBuoyHi: number | null = null;
function getSideBuoyHi(): number {
  if (_sideBuoyHi === null) {
    _sideBuoyHi = resolveCollisionValue('RIGHT_BUOY') & 0xf0;
  }
  return _sideBuoyHi;
}

let _blockingLowBits: Record<FacingDirection, Set<number>> | null = null;
function getBlockingLowBits(): Record<FacingDirection, Set<number>> {
  if (_blockingLowBits === null) {
    const bits = (names: string[]): Set<number> => {
      return new Set(names.map(name => resolveCollisionValue(name) & 0x07));
    };
    _blockingLowBits = {
      [FacingDirection.DOWN]: bits(['UP_WALL', 'UP_RIGHT_WALL', 'UP_LEFT_WALL']),
      [FacingDirection.UP]: bits(['DOWN_WALL', 'DOWN_RIGHT_WALL', 'DOWN_LEFT_WALL']),
      [FacingDirection.LEFT]: bits(['RIGHT_WALL', 'DOWN_RIGHT_WALL', 'UP_RIGHT_WALL']),
      [FacingDirection.RIGHT]: bits(['LEFT_WALL', 'DOWN_LEFT_WALL', 'UP_LEFT_WALL']),
    };
  }
  return _blockingLowBits;
}

let _leavingLowBits: Record<FacingDirection, Set<number>> | null = null;
function getLeavingLowBits(): Record<FacingDirection, Set<number>> {
  if (_leavingLowBits === null) {
    const bits = (names: string[]): Set<number> => {
      return new Set(names.map(name => resolveCollisionValue(name) & 0x07));
    };
    _leavingLowBits = {
      [FacingDirection.DOWN]: bits(['DOWN_WALL', 'DOWN_RIGHT_WALL', 'DOWN_LEFT_WALL']),
      [FacingDirection.UP]: bits(['UP_WALL', 'UP_RIGHT_WALL', 'UP_LEFT_WALL']),
      [FacingDirection.LEFT]: bits(['LEFT_WALL', 'DOWN_LEFT_WALL', 'UP_LEFT_WALL']),
      [FacingDirection.RIGHT]: bits(['RIGHT_WALL', 'DOWN_RIGHT_WALL', 'UP_RIGHT_WALL']),
    };
  }
  return _leavingLowBits;
}

const LEDGE_HI = resolveCollisionValue('HOP_DOWN') & 0xf0;
const WALL_PERMISSION = resolveCollisionValue('WALL');

const WHIRLPOOL_COLLISION_PERMISSIONS = new Set([
  resolveCollisionValue('WHIRLPOOL'),
  resolveCollisionValue('WHIRLPOOL_2C'),
]);

const WATERFALL_COLLISION_PERMISSIONS = new Set([
  resolveCollisionValue('WATERFALL'),
  resolveCollisionValue('WATERFALL_RIGHT'),
  resolveCollisionValue('WATERFALL_LEFT'),
  resolveCollisionValue('WATERFALL_UP'),
  resolveCollisionValue('CURRENT_DOWN'),
]);

const LEDGE_DIRECTION_BITS: Record<FacingDirection, Set<number>> = {
  [FacingDirection.DOWN]: new Set(
    ['HOP_DOWN', 'HOP_DOWN_RIGHT', 'HOP_DOWN_LEFT'].map(
      token => resolveCollisionValue(token) & 0x0f
    )
  ),
  [FacingDirection.UP]: new Set(
    ['HOP_UP', 'HOP_UP_RIGHT', 'HOP_UP_LEFT'].map(
      token => resolveCollisionValue(token) & 0x0f
    )
  ),
  [FacingDirection.LEFT]: new Set(
    ['HOP_LEFT', 'HOP_DOWN_LEFT', 'HOP_UP_LEFT'].map(
      token => resolveCollisionValue(token) & 0x0f
    )
  ),
  [FacingDirection.RIGHT]: new Set(
    ['HOP_RIGHT', 'HOP_DOWN_RIGHT', 'HOP_UP_RIGHT'].map(
      token => resolveCollisionValue(token) & 0x0f
    )
  ),
};

const LEDGE_COMPLEMENTS: Record<FacingDirection, Record<number, number>> = {
  [FacingDirection.DOWN]: { 2: 0, 3: 1 },
  [FacingDirection.UP]: { 0: 2, 1: 3 },
  [FacingDirection.LEFT]: { 0: 1, 2: 3 },
  [FacingDirection.RIGHT]: { 1: 0, 3: 2 },
};

export function allowsLedgeDirection(
  permission: number,
  facing: FacingDirection
): boolean {
  if ((permission & 0xf0) !== LEDGE_HI) {
    return false;
  }
  return LEDGE_DIRECTION_BITS[facing].has(permission & 0x0f);
}

export function getLedgeComplementQuadrant(
  quadrant: number,
  facing: FacingDirection
): number | null {
  return LEDGE_COMPLEMENTS[facing]?.[quadrant] ?? null;
}

export function determineQuadrantIndex(tileX: number, tileY: number): number {
  const half = Math.floor(METATILE_WIDTH / 2);
  if (half === 0) {
    throw new Error('METATILE_WIDTH must be at least two tiles wide.');
  }
  const xHalf = Math.floor((tileX % METATILE_WIDTH) / half);
  const yHalf = Math.floor((tileY % METATILE_WIDTH) / half);
  return yHalf * 2 + xHalf;
}

export function getCoordCollision(
  mapData: OverworldMap,
  tileset: OverworldTilesetLike,
  tileX: number,
  tileY: number
): number {
  try {
    return sampleCollision(mapData, tileset, tileX, tileY).permission;
  } catch (e) {
    return -1;
  }
}

export function sampleCollision(
  mapData: OverworldMap,
  tileset: OverworldTilesetLike,
  tileX: number,
  tileY: number
): CollisionSample {
  if (tileX < 0 || tileY < 0) {
    throw new Error(`Negative tile coordinate (${tileX}, ${tileY}) is invalid.`);
  }

  const metatileX = Math.floor(tileX / METATILE_WIDTH);
  const subtileX = tileX % METATILE_WIDTH;
  const metatileY = Math.floor(tileY / METATILE_WIDTH);
  const subtileY = tileY % METATILE_WIDTH;

  if (metatileX >= mapData.width || metatileY >= mapData.height) {
    throw new Error(
      `Tile (${tileX}, ${tileY}) resolves to metatile (${metatileX}, ${metatileY}) ` +
        `outside map bounds ${mapData.width}x${mapData.height}.`
    );
  }

  const metatileId = mapData.getMetatileAt(metatileX, metatileY);
  if (metatileId < 0 || metatileId >= tileset.metatiles.length) {
    throw new Error(
      `Metatile id ${metatileId} at (${metatileX}, ${metatileY}) exceeds ` +
        `tileset '${tileset.tilesetName}' bounds.`
    );
  }

  const metatile = tileset.metatiles[metatileId];
  const quadrant = determineQuadrantIndex(tileX, tileY);

  if (quadrant >= metatile.collision.length) {
    throw new Error(
      `Metatile ${metatileId} collision table incomplete for quadrant ${quadrant}.`
    );
  }

  const permission = metatile.collision[quadrant];
  return new CollisionSample(
    permission,
    metatileId,
    quadrant,
    tileX,
    tileY,
    getCollisionStdScript(permission)
  );
}

export function isDirectionBlocked(
  permission: number,
  facing: FacingDirection
): boolean {
  const hi = permission & 0xf0;
  if (hi !== getSideWallHi() && hi !== getSideBuoyHi()) {
    return false;
  }
  const low = permission & 0x07;
  return getBlockingLowBits()[facing].has(low);
}

// ASM: engine/overworld/npc_movement.asm CanObjectLeaveTile
export function isDirectionBlockedLeaving(
  permission: number,
  facing: FacingDirection
): boolean {
  const hi = permission & 0xf0;
  if (hi !== getSideWallHi() && hi !== getSideBuoyHi()) {
    return false;
  }
  const low = permission & 0x07;
  return getLeavingLowBits()[facing].has(low);
}

export function isWhirlpoolCollision(permission: number): boolean {
  return WHIRLPOOL_COLLISION_PERMISSIONS.has(permission);
}

export function isWaterfallCollision(permission: number): boolean {
  return WATERFALL_COLLISION_PERMISSIONS.has(permission);
}

export interface PermissionPassabilityOptions {
  readonly allowWaterfall?: boolean;
}

export function isPermissionPassable(
  permission: number,
  facing: FacingDirection,
  playerState: PlayerState,
  options: PermissionPassabilityOptions = {}
): boolean {
  if (permission < 0) {
    return false;
  }

  const attributes = describeCollision(permission);
  if (SURF_STATES.includes(playerState) && isWaterfallCollision(permission) && options.allowWaterfall) {
    return attributes.terrain !== Terrain.WALL;
  }

  if (isDirectionBlocked(permission, facing)) {
    return false;
  }

  if (SURF_STATES.includes(playerState)) {
    if (isWhirlpoolCollision(permission)) {
      return false;
    }
    if (isWaterfallCollision(permission) && !options.allowWaterfall) {
      return false;
    }
    return attributes.terrain !== Terrain.WALL;
  }
  return attributes.terrain === Terrain.LAND;
}

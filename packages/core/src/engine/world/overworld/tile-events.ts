/**
 * Faithful helpers for tile collisions driven by `tile_events.asm`.
 *
 * The original Game Boy logic splits per-tile checks across a handful of
 * routines that classify warp carpets, door tiles, and other special cases.
 */

import { FacingDirection } from '@pokecrystal/core/core/enums/overworld';
import { resolveCollisionValue } from './collision-data';

const HI_NYBBLE_WARPS = 0x70;
const COLL_PIT = resolveCollisionValue('PIT');
const COLL_PIT_68 = resolveCollisionValue('PIT_68');
const COLL_WARP_PANEL = resolveCollisionValue('WARP_PANEL');
const COLL_WARP_DOOR = resolveCollisionValue('DOOR');
const COLL_WARP_DOOR_ALT = resolveCollisionValue('DOOR_79');
const COLL_WARP_DOOR_ALT2 = resolveCollisionValue('DOOR_75');
const COLL_WARP_DOOR_ALT3 = resolveCollisionValue('DOOR_7D');
const COLL_WARP_STAIRCASE = resolveCollisionValue('STAIRCASE');
const COLL_WARP_STAIRCASE_ALT = resolveCollisionValue('STAIRCASE_73');
const COLL_WARP_CAVE = resolveCollisionValue('CAVE');
const COLL_WARP_CAVE_ALT = resolveCollisionValue('CAVE_74');

const DIRECTIONAL_WARPS = new Map<number, FacingDirection>([
  [resolveCollisionValue('WARP_CARPET_DOWN'), FacingDirection.DOWN],
  [resolveCollisionValue('WARP_CARPET_UP'), FacingDirection.UP],
  [resolveCollisionValue('WARP_CARPET_LEFT'), FacingDirection.LEFT],
  [resolveCollisionValue('WARP_CARPET_RIGHT'), FacingDirection.RIGHT],
]);

const WARP_FACING_DOWN = new Set<number>([
  COLL_WARP_DOOR,
  COLL_WARP_DOOR_ALT,
  COLL_WARP_DOOR_ALT2,
  COLL_WARP_DOOR_ALT3,
  COLL_WARP_STAIRCASE,
  COLL_WARP_STAIRCASE_ALT,
  COLL_WARP_CAVE,
  COLL_WARP_CAVE_ALT,
]);

const DOOR_SOUND_COLLISIONS = new Set<number>([
  COLL_WARP_DOOR,
  COLL_WARP_DOOR_ALT,
  COLL_WARP_DOOR_ALT2,
  COLL_WARP_DOOR_ALT3,
  COLL_WARP_STAIRCASE,
  COLL_WARP_STAIRCASE_ALT,
  COLL_WARP_CAVE,
  COLL_WARP_CAVE_ALT,
]);

const maskPermission = (permission: number): number => permission & 0xff;

export function isWarpPermission(permission: number): boolean {
  if (permission === COLL_PIT || permission === COLL_PIT_68) {
    return true;
  }
  return (permission & 0xf0) === HI_NYBBLE_WARPS;
}

export function isPitPermission(permission: number): boolean {
  const masked = maskPermission(permission);
  return masked === COLL_PIT || masked === COLL_PIT_68;
}

export function isDirectionalWarp(permission: number): boolean {
  return DIRECTIONAL_WARPS.has(maskPermission(permission));
}

export function directionalWarpFacing(
  permission: number
): FacingDirection | null {
  return DIRECTIONAL_WARPS.get(maskPermission(permission)) ?? null;
}

export function requiresWarpFacingDown(permission: number): boolean {
  return WARP_FACING_DOWN.has(maskPermission(permission));
}

export function warpSoundForPermission(permission: number): string | null {
  const masked = maskPermission(permission);
  if (DOOR_SOUND_COLLISIONS.has(masked)) {
    return 'SFX_ENTER_DOOR';
  }
  if (masked === COLL_WARP_PANEL) {
    return 'SFX_WARP_TO';
  }
  return 'SFX_EXIT_BUILDING';
}

import type { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import type { OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";
import { resolveCollisionValue } from "./collision-data";
import { getCoordCollision } from "./collision-rules";

const COUNTER_PERMISSIONS = new Set([
  resolveCollisionValue("COUNTER"),
  resolveCollisionValue("COUNTER_98"),
]);

// ASM mapping: engine/overworld/player.asm (CheckFacingObject counter handling).
export function adjustCounterTile(
  map: OverworldMap,
  tileset: OverworldTilesetLike,
  playerX: number,
  playerY: number,
  tileX: number,
  tileY: number,
  stride: number
): [number, number] {
  const deltaX = tileX - playerX;
  const deltaY = tileY - playerY;
  if (deltaX === 0 && deltaY === 0) {
    return [tileX, tileY];
  }

  const step = Math.max(1, Math.trunc(stride));
  const candidates: Array<[number, number]> = [];
  if (deltaX === 0 && deltaY !== 0) {
    const frontY = playerY + deltaY;
    for (let offset = 0; offset <= step; offset += 1) {
      candidates.push([playerX - offset, frontY]);
    }
  } else if (deltaY === 0 && deltaX !== 0) {
    const frontX = playerX + deltaX;
    for (let offset = 0; offset <= step; offset += 1) {
      candidates.push([frontX, playerY - offset]);
    }
  } else {
    candidates.push([tileX, tileY]);
  }

  for (const [candidateX, candidateY] of candidates) {
    const permission = getCoordCollision(map, tileset, candidateX, candidateY);
    if (COUNTER_PERMISSIONS.has(permission)) {
      return [candidateX + deltaX, candidateY + deltaY];
    }
  }

  return [tileX, tileY];
}

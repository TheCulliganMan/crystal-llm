/**
 * Faithful helpers for identifying ledge transitions in the overworld.
 *
 * The original engine encodes ledge behaviour inside the collision tables. The
 * helpers in this module translate the collision quadrants into directional
 * predicates the Overworld controller can consume without duplicating the ASM.
 */

import { FacingDirection } from '@pokecrystal/core/core/enums/overworld';
import { resolveCollisionValue } from './collision-data';
import {
  CollisionSample,
  allowsLedgeDirection,
  getLedgeComplementQuadrant,
  sampleCollision,
} from './collision-rules';
import { OverworldMap } from './overworld-map';
import { METATILE_WIDTH } from '@pokecrystal/core/core/tileset-data';
import type { OverworldTilesetLike } from '@pokecrystal/core/engine/world/overworld/tileset-types';

const WALL_PERMISSION = resolveCollisionValue('WALL');

export function collectCollisionSamples(
  mapData: OverworldMap,
  tileset: OverworldTilesetLike,
  tileX: number,
  tileY: number,
  stride: number
): CollisionSample[] {
  /**
   * Return the collision samples that make up the ``stride``×``stride`` footprint
   * rooted at ``(tile_x, tile_y)``. Returns an empty list if the footprint extends
   * outside the map bounds.
   */

  const maxTileX = mapData.width * METATILE_WIDTH;
  const maxTileY = mapData.height * METATILE_WIDTH;
  const samples: CollisionSample[] = [];
  for (let dx = 0; dx < stride; dx++) {
    for (let dy = 0; dy < stride; dy++) {
      const subtileX = tileX - dx;
      const subtileY = tileY - dy;
      if (
        !(
          subtileX >= 0 &&
          subtileX < maxTileX &&
          subtileY >= 0 &&
          subtileY < maxTileY
        )
      ) {
        return [];
      }
      try {
        samples.push(sampleCollision(mapData, tileset, subtileX, subtileY));
      } catch {
        return [];
      }
    }
  }
  return samples;
}

export function frontFaceSamples(
  samples: CollisionSample[],
  facing: FacingDirection,
  tileX: number,
  tileY: number,
  stride: number
): CollisionSample[] {
  /** Filter ``samples`` down to the subtiles along the player's forward edge. */

  if (!samples.length) {
    return [];
  }

  if (facing === FacingDirection.DOWN || facing === FacingDirection.UP) {
    const yValues = samples.map(sample => sample.tileY);
    const frontY =
      facing === FacingDirection.DOWN ? Math.max(...yValues) : Math.min(...yValues);
    return samples.filter(sample => sample.tileY === frontY);
  }
  if (facing === FacingDirection.RIGHT || facing === FacingDirection.LEFT) {
    const xValues = samples.map(sample => sample.tileX);
    const frontX =
      facing === FacingDirection.RIGHT ? Math.max(...xValues) : Math.min(...xValues);
    return samples.filter(sample => sample.tileX === frontX);
  }
  return [];
}

function sampleSupportsLedge(
  sample: CollisionSample,
  facing: FacingDirection,
  tileset: OverworldTilesetLike
): boolean {
  /** Return True if ``sample`` belongs to a ledge that allows ``facing``. */

  const permission = sample.permission;
  if (allowsLedgeDirection(permission, facing)) {
    return true;
  }
  if (permission !== WALL_PERMISSION) {
    return false;
  }
  const complement = getLedgeComplementQuadrant(sample.quadrant, facing);
  if (complement === null) {
    return false;
  }
  const metatile = tileset.metatiles[sample.metatileId];
  if (complement >= metatile.collision.length) {
    return false;
  }
  return allowsLedgeDirection(metatile.collision[complement], facing);
}

export function isLedgeFace(
  samples: CollisionSample[],
  facing: FacingDirection,
  tileset: OverworldTilesetLike,
  tileX: number,
  tileY: number,
  stride: number
): boolean {
  /**
   * Determine whether the provided footprint corresponds to a ledge that the
   * player can jump over while facing ``facing``.
   */

  const front = frontFaceSamples(samples, facing, tileX, tileY, stride);
  if (!front.length) {
    return false;
  }
  return front.every(sample => sampleSupportsLedge(sample, facing, tileset));
}

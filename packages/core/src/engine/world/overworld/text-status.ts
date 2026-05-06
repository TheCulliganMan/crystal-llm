import { FacingDirection, PlayerState } from "@pokecrystal/core/core/enums/overworld";
import {
  CollisionSample,
  isPermissionPassable,
} from "@pokecrystal/core/engine/world/overworld/collision-rules";
import { describeCollision } from "@pokecrystal/core/engine/world/overworld/collision-data";

export function formatTileStatus(
  sample: CollisionSample,
  {
    facing,
    playerState,
  }: { facing: FacingDirection; playerState: PlayerState }
): string {
  const passable = isPermissionPassable(
    sample.permission,
    facing,
    playerState
  );
  const attributes = describeCollision(sample.permission);
  const status = passable ? "PASSABLE" : "BLOCKED";
  const terrain = attributes.terrain;
  const script = sample.stdScript ?? "none";
  return (
    `TILE: ${status} perm=${sample.permission} ` +
    `metatile=${sample.metatileId} quadrant=${sample.quadrant} ` +
    `terrain=${terrain} script=${script}`
  );
}

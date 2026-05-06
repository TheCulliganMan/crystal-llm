import { GameState } from '@pokecrystal/core/core/state';
import {
  applySpawn,
  findSpawnForMap,
  getMapMetadataByGroup,
  getMapMetadataByName,
  getSpawnPoint,
  Spawn,
} from '@pokecrystal/core/engine/world/maps';
import { METATILE_WIDTH } from '../world/tile';
import { scaleTileCoord } from '@pokecrystal/core/engine/world/overworld/tile-coords';

// Placeholder for the Overworld object
interface Overworld {
  load_map: (mapName: string) => void;
  player_x: number;
  player_y: number;
  prev_player_x: number;
  prev_player_y: number;
  target_tile_x: number;
  target_tile_y: number;
  is_moving: boolean;
  _queued_direction: string | null;
  step_progress_px: number;
  step_dx_px: number;
  step_dy_px: number;
  _sync_player_state?: () => void;
  clear_pending_white_fade?: () => void;
  _warp_cooldown?: number;
  WALK_FRAMES?: number;
  TILES_PER_COLLISION?: number;
}

export const warpToSpawnPoint = (
  gameState: GameState,
  { overworld }: { overworld?: Overworld }
): boolean => {
  const group = gameState.wram.wLastSpawnMapGroup;
  const mapId = gameState.wram.wLastSpawnMapNumber;
  const resolved = findSpawnForMap(group, mapId);

  let identifier: Spawn;
  let spawnPoint;

  if (!resolved) {
    identifier = Spawn.HOME;
    spawnPoint = getSpawnPoint(identifier);
  } else {
    [identifier, spawnPoint] = resolved;
  }

  applySpawn(gameState, identifier);

  if (!overworld) {
    return true;
  }

  const mapMeta = getMapMetadataByGroup(spawnPoint.groupId, spawnPoint.mapId);
  if (!mapMeta) {
    throw new Error(
      `Spawn point ${spawnPoint.mapName} lacks metadata for group ${spawnPoint.groupId} / map ${spawnPoint.mapId}.`
    );
  }

  overworld.load_map(spawnPoint.mapName);

  const stride = Math.max(1, Math.trunc(overworld.TILES_PER_COLLISION ?? 2));
  const minTile = Math.max(stride - 1, 0);
  const maxTileX = mapMeta.width * METATILE_WIDTH - 1;
  const maxTileY = mapMeta.height * METATILE_WIDTH - 1;

  const tileX = spawnPoint.tileX >= 0 ? spawnPoint.tileX : minTile;
  const tileY = spawnPoint.tileY >= 0 ? spawnPoint.tileY : minTile;
  const scaledX = scaleTileCoord(tileX, stride);
  const scaledY = scaleTileCoord(tileY, stride);
  overworld.player_x = Math.max(minTile, Math.min(maxTileX, scaledX));
  overworld.player_y = Math.max(minTile, Math.min(maxTileY, scaledY));
  overworld.prev_player_x = overworld.player_x;
  overworld.prev_player_y = overworld.player_y;

  overworld._sync_player_state?.();
  overworld.clear_pending_white_fade?.();

  if (overworld._warp_cooldown !== undefined && overworld.WALK_FRAMES !== undefined) {
    overworld._warp_cooldown = overworld.WALK_FRAMES;
  }

  return true;
};

export const warpToLastPokecenter = (
  gameState: GameState,
  { overworld }: { overworld?: Overworld }
): boolean => {
  if (!overworld) {
    return false;
  }

  const mapName =
    gameState.sram.last_pokecenter_map_name ??
    gameState.wram.last_pokecenter_map_name;

  if (!mapName) {
    return false;
  }

  const metadata = getMapMetadataByName(mapName);
  if (!metadata) {
    return false;
  }

  const playerX =
    gameState.sram.last_pokecenter_player_x ??
    gameState.wram.last_pokecenter_player_x;
  const playerY =
    gameState.sram.last_pokecenter_player_y ??
    gameState.wram.last_pokecenter_player_y;

  if (playerX === undefined || playerY === undefined) {
    return false;
  }

  overworld.load_map(mapName);

  const stride = Math.max(1, Math.trunc(overworld.TILES_PER_COLLISION ?? 2));
  const scaledX = scaleTileCoord(playerX, stride);
  const scaledY = scaleTileCoord(playerY, stride);
  overworld.player_x = scaledX;
  overworld.player_y = scaledY;
  overworld.prev_player_x = scaledX;
  overworld.prev_player_y = scaledY;
  overworld.target_tile_x = scaledX;
  overworld.target_tile_y = scaledY;
  overworld.is_moving = false;
  overworld._queued_direction = null;
  overworld.step_progress_px = 0.0;
  overworld.step_dx_px = 0.0;
  overworld.step_dy_px = 0.0;

  overworld._sync_player_state?.();
  overworld.clear_pending_white_fade?.();

  if (overworld._warp_cooldown !== undefined && overworld.WALK_FRAMES !== undefined) {
    overworld._warp_cooldown = overworld.WALK_FRAMES;
  }

  return true;
};

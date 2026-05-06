
import { GameState } from '../../../core/state';
import {
    applySpawn,
    findSpawnForMap,
    getMapMetadataByGroup,
    getMapMetadataByName,
    getSpawnPoint,
    Spawn,
} from '../maps';
import { METATILE_WIDTH } from '../tile';
import { scaleTileCoord, unscaleTileCoord } from '@pokecrystal/core/engine/world/overworld/tile-coords';

export type Overworld = {
    load_map: (mapName: string) => void;
    player_x: number;
    player_y: number;
    prev_player_x: number;
    prev_player_y: number;
    target_tile_x: number;
    target_tile_y: number;
    is_moving: boolean;
    _queued_direction?: string | null;
    step_progress_px: number;
    step_dx_px: number;
    step_dy_px: number;
    _sync_player_state?: () => void;
    clear_pending_white_fade?: () => void;
    _warp_cooldown?: number;
    WALK_FRAMES?: number;
    TILES_PER_COLLISION?: number;
};

export function warp_to_spawn_point(
    game_state: GameState,
    { overworld }: { overworld?: Overworld }
): boolean {
    // ASM: `engine/overworld/events.asm` -> `WarpToSpawnPoint`
    const group = game_state.wram.wLastSpawnMapGroup || game_state.sram.last_spawn_map_group;
    const mapId = game_state.wram.wLastSpawnMapNumber || game_state.sram.last_spawn_map_number;
    const resolved = findSpawnForMap(group, mapId);

    let identifier: Spawn;
    let spawnPoint;

    if (!resolved) {
        identifier = Spawn.HOME;
        spawnPoint = getSpawnPoint(identifier);
    } else {
        [identifier, spawnPoint] = resolved;
    }

    applySpawn(game_state, identifier);

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
}

export function warp_to_last_pokecenter(
    game_state: GameState,
    { overworld }: { overworld?: Overworld }
): boolean {
    // ASM: `engine/events/whiteout.asm` special flow to last Pokecenter warp.
    if (!overworld) {
        return false;
    }

    const mapName =
        game_state.sram.last_pokecenter_map_name ||
        game_state.wram.last_pokecenter_map_name;

    if (!mapName) {
        return false;
    }

    const metadata = getMapMetadataByName(mapName);
    if (!metadata) {
        return false;
    }

    let playerX =
        game_state.sram.last_pokecenter_player_x ??
        game_state.wram.last_pokecenter_player_x;
    let playerY =
        game_state.sram.last_pokecenter_player_y ??
        game_state.wram.last_pokecenter_player_y;

    if (playerX === undefined || playerY === undefined) {
        return false;
    }

    const blockStride = Math.max(1, Math.floor(METATILE_WIDTH / 2));
    const stride = overworld.TILES_PER_COLLISION ?? blockStride;
    const offset = Math.max(0, stride - 1);
    const maxTileX = metadata.width * blockStride - 1;
    const maxTileY = metadata.height * blockStride - 1;

    const units =
        game_state.sram.last_pokecenter_coordinate_units ||
        game_state.wram.last_pokecenter_coordinate_units ||
        'block';
    const normalizedUnits = units.trim().toLowerCase();

    let normalizedX: number;
    let normalizedY: number;
    if (normalizedUnits === 'tile' || normalizedUnits === 'subtile') {
        normalizedX = Math.max(0, Math.min(maxTileX, Math.trunc(playerX)));
        normalizedY = Math.max(offset, Math.min(maxTileY, Math.trunc(playerY)));
    } else {
        normalizedX = Math.max(
            0,
            Math.min(maxTileX, Math.trunc(playerX) * blockStride + offset)
        );
        normalizedY = Math.max(
            offset,
            Math.min(maxTileY, Math.trunc(playerY) * blockStride + offset)
        );
        game_state.wram.last_pokecenter_player_x = normalizedX;
        game_state.wram.last_pokecenter_player_y = normalizedY;
        game_state.sram.last_pokecenter_player_x = normalizedX;
        game_state.sram.last_pokecenter_player_y = normalizedY;
        game_state.wram.last_pokecenter_coordinate_units = 'tile';
        game_state.sram.last_pokecenter_coordinate_units = 'tile';
    }

    playerX = normalizedX;
    playerY = normalizedY;
    game_state.wram.last_pokecenter_coordinate_units = 'tile';
    game_state.sram.last_pokecenter_coordinate_units = 'tile';

    const backupGroup =
        game_state.sram.last_pokecenter_backup_map_group ||
        game_state.wram.last_pokecenter_backup_map_group ||
        game_state.wram.wBackupMapGroup;
    const backupNumber =
        game_state.sram.last_pokecenter_backup_map_number ||
        game_state.wram.last_pokecenter_backup_map_number ||
        game_state.wram.wBackupMapNumber;

    if (backupGroup !== undefined) {
        game_state.wram.wBackupMapGroup = backupGroup;
    }
    if (backupNumber !== undefined) {
        game_state.wram.wBackupMapNumber = backupNumber;
    }

    game_state.wram.wMapGroup = metadata.groupId;
    game_state.wram.wMapNumber = metadata.mapId;
    game_state.wram.current_map_group = metadata.groupId;
    game_state.wram.current_map_id = metadata.mapId;
    game_state.wram.wXCoord = playerX;
    game_state.wram.wYCoord = playerY;
    game_state.wram.player_x = Math.floor(playerX / stride);
    game_state.wram.player_y = Math.floor(playerY / stride);
    game_state.wram.player_subtile_x = playerX % stride;
    game_state.wram.player_subtile_y = playerY % stride;
    game_state.wram.scene_name = '';
    game_state.wram.wLastSpawnMapGroup = metadata.groupId;
    game_state.wram.wLastSpawnMapNumber = metadata.mapId;
    game_state.sram.last_spawn_map_group = metadata.groupId;
    game_state.sram.last_spawn_map_number = metadata.mapId;
    game_state.wram.last_pokecenter_map_group = metadata.groupId;
    game_state.wram.last_pokecenter_map_number = metadata.mapId;
    game_state.sram.last_pokecenter_map_group = metadata.groupId;
    game_state.sram.last_pokecenter_map_number = metadata.mapId;

    if (!overworld.load_map) {
        throw new Error('Warping to Pokemon Center requires a valid overworld.');
    }

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

    overworld.load_map(mapName);

    overworld._sync_player_state?.();
    overworld.clear_pending_white_fade?.();

    if (overworld._warp_cooldown !== undefined && overworld.WALK_FRAMES !== undefined) {
        overworld._warp_cooldown = overworld.WALK_FRAMES;
    }

    return true;
}

export function record_last_pokecenter_heal(
    game_state: GameState,
    { overworld }: { overworld?: Overworld }
): boolean {
    if (!overworld) {
        return false;
    }
    const mapName = (overworld as Overworld & { current_map_name?: string; currentMapName?: string }).current_map_name ??
        (overworld as Overworld & { current_map_name?: string; currentMapName?: string }).currentMapName;
    if (!mapName || typeof mapName !== 'string') {
        return false;
    }
    const metadata = getMapMetadataByName(mapName);
    if (!metadata) {
        return false;
    }

    const playerX = overworld.player_x;
    const playerY = overworld.player_y;
    const stride = Math.max(1, Math.trunc(overworld.TILES_PER_COLLISION ?? 2));
    const rawX = typeof playerX === 'number' ? unscaleTileCoord(playerX, stride) : null;
    const rawY = typeof playerY === 'number' ? unscaleTileCoord(playerY, stride) : null;

    const blockStride = Math.max(1, Math.floor(METATILE_WIDTH / 2));
    const maxTileX = Math.max(0, metadata.width * blockStride - 1);
    const maxTileY = Math.max(0, metadata.height * blockStride - 1);
    const offset = Math.max(0, stride - 1);
    const clampTileCoord = (value: number | null, min: number, max: number): number | null => {
        if (value === null || !Number.isFinite(value)) {
            return null;
        }
        const truncated = Math.trunc(value);
        if (truncated < min) {
            return min;
        }
        if (truncated > max) {
            return max;
        }
        return truncated;
    };
    const clampedX = clampTileCoord(rawX, 0, maxTileX);
    const clampedY = clampTileCoord(rawY, offset, Math.max(offset, maxTileY));

    game_state.wram.last_pokecenter_map_name = mapName;
    game_state.wram.last_pokecenter_map_group = metadata.groupId;
    game_state.wram.last_pokecenter_map_number = metadata.mapId;
    if (clampedX !== null) {
        game_state.wram.last_pokecenter_player_x = clampedX;
    }
    if (clampedY !== null) {
        game_state.wram.last_pokecenter_player_y = clampedY;
    }
    game_state.wram.last_pokecenter_coordinate_units = 'tile';

    game_state.sram.last_pokecenter_map_name = mapName;
    game_state.sram.last_pokecenter_map_group = metadata.groupId;
    game_state.sram.last_pokecenter_map_number = metadata.mapId;
    if (clampedX !== null) {
        game_state.sram.last_pokecenter_player_x = clampedX;
    }
    if (clampedY !== null) {
        game_state.sram.last_pokecenter_player_y = clampedY;
    }
    game_state.sram.last_pokecenter_coordinate_units = 'tile';

    const backupGroup = (game_state.wram as { wBackupMapGroup?: number }).wBackupMapGroup ?? 0;
    const backupNumber = (game_state.wram as { wBackupMapNumber?: number }).wBackupMapNumber ?? 0;
    game_state.wram.last_pokecenter_backup_map_group = backupGroup;
    game_state.wram.last_pokecenter_backup_map_number = backupNumber;
    game_state.sram.last_pokecenter_backup_map_group = backupGroup;
    game_state.sram.last_pokecenter_backup_map_number = backupNumber;

    return true;
}

import { createInitialGameState } from "@pokecrystal/core/core/state";
import { METATILE_WIDTH } from "@pokecrystal/core/core/tileset-data";
import { record_last_pokecenter_heal, warp_to_spawn_point } from "@pokecrystal/core/engine/world/special-events/map";
import {
    applySpawn,
    findSpawnForMap,
    getMapMetadataByGroup,
    getMapMetadataByName,
    type MapMetadata,
} from "@pokecrystal/core/engine/world/maps";

jest.mock("@pokecrystal/core/engine/world/maps", () => ({
    __esModule: true,
    applySpawn: jest.fn(),
    findSpawnForMap: jest.fn(),
    getMapMetadataByGroup: jest.fn(),
    getMapMetadataByName: jest.fn(),
}));

const mockedApplySpawn = applySpawn as jest.MockedFunction<typeof applySpawn>;
const mockedFindSpawnForMap = findSpawnForMap as jest.MockedFunction<typeof findSpawnForMap>;
const mockedGetMapMetadataByGroup = getMapMetadataByGroup as jest.MockedFunction<typeof getMapMetadataByGroup>;
const mockedGetMapMetadataByName = getMapMetadataByName as jest.MockedFunction<typeof getMapMetadataByName>;

describe("record_last_pokecenter_heal", () => {
    const metadata: MapMetadata = {
        constant: "TEST_CENTER",
        name: "TestCenter",
        groupName: "TestGroup",
        groupId: 12,
        mapId: 34,
        width: 28,
        height: 18,
        phoneService: 0,
        environment: "POKECENTER",
    };

    beforeEach(() => {
        mockedGetMapMetadataByName.mockReturnValue(metadata);
    });

    it("clamps recorded coordinates to the map bounds", () => {
        const gameState = createInitialGameState();
        const overworld = {
            player_x: 999,
            player_y: 999,
            TILES_PER_COLLISION: 2,
            current_map_name: metadata.name,
        };

        const success = record_last_pokecenter_heal(gameState, { overworld });

        expect(success).toBe(true);

        const blockStride = Math.max(1, Math.floor(METATILE_WIDTH / 2));
        const stride = Math.max(1, Math.trunc(overworld.TILES_PER_COLLISION ?? 2));
        const offset = Math.max(0, stride - 1);
        const maxTileX = Math.max(0, metadata.width * blockStride - 1);
        const maxTileY = Math.max(offset, metadata.height * blockStride - 1);

        expect(gameState.wram.last_pokecenter_player_x).toBe(maxTileX);
        expect(gameState.sram.last_pokecenter_player_x).toBe(maxTileX);
        expect(gameState.wram.last_pokecenter_player_y).toBe(maxTileY);
        expect(gameState.sram.last_pokecenter_player_y).toBe(maxTileY);
        expect(gameState.wram.last_pokecenter_coordinate_units).toBe("tile");
        expect(gameState.sram.last_pokecenter_coordinate_units).toBe("tile");
    });

    it("enforces minimum coordinate limits even when the player is below valid bounds", () => {
        const gameState = createInitialGameState();
        const overworld = {
            player_x: -50,
            player_y: -10,
            TILES_PER_COLLISION: 2,
            current_map_name: metadata.name,
        };

        const success = record_last_pokecenter_heal(gameState, { overworld });

        expect(success).toBe(true);

        const stride = Math.max(1, Math.trunc(overworld.TILES_PER_COLLISION ?? 2));
        const offset = Math.max(0, stride - 1);

        expect(gameState.wram.last_pokecenter_player_x).toBe(0);
        expect(gameState.sram.last_pokecenter_player_x).toBe(0);
        expect(gameState.wram.last_pokecenter_player_y).toBe(offset);
        expect(gameState.sram.last_pokecenter_player_y).toBe(offset);
    });
});

describe("warp_to_spawn_point", () => {
    const metadata: MapMetadata = {
        constant: "CHERRYGROVE_CITY",
        name: "CherrygroveCity",
        groupName: "Cherrygrove",
        groupId: 7,
        mapId: 5,
        width: 20,
        height: 18,
        phoneService: 0,
        environment: "TOWN",
    };

    beforeEach(() => {
        mockedFindSpawnForMap.mockReset();
        mockedGetMapMetadataByGroup.mockReset();
        mockedApplySpawn.mockReset();
    });

    it("uses the SRAM blackout spawn when WRAM no longer carries the respawn map", () => {
        const gameState = createInitialGameState();
        gameState.wram.wLastSpawnMapGroup = 0;
        gameState.wram.wLastSpawnMapNumber = 0;
        gameState.sram.last_spawn_map_group = metadata.groupId;
        gameState.sram.last_spawn_map_number = metadata.mapId;

        mockedFindSpawnForMap.mockReturnValue([
            15 as never,
            {
                identifier: 15 as never,
                mapConstant: metadata.constant,
                mapName: metadata.name,
                groupId: metadata.groupId,
                mapId: metadata.mapId,
                tileX: 29,
                tileY: 4,
                groupName: metadata.groupName,
                metatileX: 14,
                metatileY: 2,
                subtileX: 1,
                subtileY: 1,
            },
        ]);
        mockedGetMapMetadataByGroup.mockReturnValue(metadata);

        const overworld = {
            load_map: jest.fn(),
            player_x: 0,
            player_y: 0,
            prev_player_x: 0,
            prev_player_y: 0,
            target_tile_x: 0,
            target_tile_y: 0,
            is_moving: false,
            step_progress_px: 0,
            step_dx_px: 0,
            step_dy_px: 0,
            TILES_PER_COLLISION: 2,
        };

        const warped = warp_to_spawn_point(gameState, { overworld });

        expect(warped).toBe(true);
        expect(mockedFindSpawnForMap).toHaveBeenCalledWith(metadata.groupId, metadata.mapId);
        expect(overworld.load_map).toHaveBeenCalledWith(metadata.name);
    });
});

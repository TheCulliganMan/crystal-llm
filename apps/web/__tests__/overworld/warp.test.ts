
import { Overworld } from "@/engine/world/overworld/overworld";
import { createInitialGameState, GameState } from "@/core/state";
import { DataLoader } from "@/core/data-loader";
import { EventManager } from "@/engine/world/events";
import { MapEvents, WarpEvent } from "@/core/models/map";
import { AudioEngine } from "@/engine/systems/audio";
import { OverworldTilesetLike } from "@/engine/world/overworld/tileset-types";

// Mock the maps module
jest.mock("@/engine/world/maps", () => ({
  getMapMetadataByGroup: () => ({ name: "PlayersHouse2F", width: 10, height: 10 }),
  getMapEnvironment: () => "indoor",
  getMetatileAt: () => 0,
  getMetatileIds: () => [],
}));

// Mock dependencies
const mockGameState = createInitialGameState();
// Initialize WRAM properties needed for Overworld
mockGameState.wram = {
  wMapGroup: 1,
  wMapNumber: 1,
  wXCoord: 0,
  wYCoord: 0,
  player_x: 0,
  player_y: 0,
  player_subtile_x: 0,
  player_subtile_y: 0,
  wPlayerState: 0,
  wEnabledPlayerEvents: 0xFF,
  event_flags: {},
  map_scenes: {},
  map_scene_indices: {},
} as any;

const mockDataLoader = {
  ensure_overworld_data: jest.fn(),
  ensure_map_scripts: jest.fn(),
  map_events: new Map(),
  map_attributes: new Map(),
  map_metadata: new Map(),
  get_script: jest.fn(),
  get_text: jest.fn(),
} as unknown as DataLoader;

const mockEventManager = {
  on: jest.fn(),
  dispatch: jest.fn(),
} as unknown as EventManager;

const mockTileset: OverworldTilesetLike = {
  tilesetName: "johto",
  metatiles: [],
  coll: [],
  attr: [],
};

const mockAudioEngine = {
  play_sound: jest.fn(),
  play_music: jest.fn(),
} as unknown as AudioEngine;

const mockUI = {
  screen: {
    get_size: () => [160, 144],
  },
};

// Helper to create a warp event
function createWarp(index: number, x: number, y: number, targetMap: string, targetWarpId: number): WarpEvent {
  return {
    index,
    x,
    y,
    target_map_constant: targetMap.toUpperCase(),
    target_map: targetMap,
    target_warp_id: targetWarpId,
  };
}

describe("Overworld Warp Logic", () => {
  let overworld: Overworld;

  beforeEach(() => {
    jest.clearAllMocks();

    // Setup initial map data
    const playersHouse2F = "PlayersHouse2F";
    const playersHouse1F = "PlayersHouse1F";

    // Mock metadata
    (mockDataLoader as any).map_metadata.set(playersHouse2F, { name: playersHouse2F, width: 10, height: 10 });
    (mockDataLoader as any).map_metadata.set(playersHouse1F, { name: playersHouse1F, width: 10, height: 10 });

    // Mock attributes
    (mockDataLoader as any).map_attributes.set(playersHouse2F, { tileset_name: "johto", width: 10, height: 10 });
    (mockDataLoader as any).map_attributes.set(playersHouse1F, { tileset_name: "johto", width: 10, height: 10 });

    // Mock events
    const warpTo1F = createWarp(1, 3, 0, playersHouse1F, 1);
    const events2F = MapEvents.parse({ warps: [warpTo1F] });
    (mockDataLoader as any).map_events.set(playersHouse2F, events2F);

    const warpTo2F = createWarp(1, 5, 5, playersHouse2F, 1); // Dummy return warp
    const events1F = MapEvents.parse({ warps: [warpTo2F] });
    (mockDataLoader as any).map_events.set(playersHouse1F, events1F);

    // Initialize Overworld
    // We need to bypass the constructor's initial load_map or ensure it works with our mocks
    // The constructor calls _resolve_initial_metadata which uses getMapMetadataByGroup.
    // We might need to mock getMapMetadataByGroup or ensure wMapGroup/wMapNumber match our mock data.
    // For simplicity, we can cast and inject if needed, but let's try to make it work.
    
    // Mock getMapMetadataByGroup global function if possible, or just rely on it returning something if we set wMapGroup/wMapNumber correctly.
    // Since we can't easily mock the imported function here without jest.mock hoisting, 
    // let's assume we can set the initial state to something valid.
    
    // Actually, Overworld constructor calls `this.load_map(this.current_map_name)`.
    // We need to make sure `current_map_name` is set correctly.
    
    // Let's just instantiate it. We might need to fix `_resolve_initial_metadata` call.
    // For this test, we can subclass Overworld to override `_resolve_initial_metadata` or just mock the module.
  });

  it("warps from PlayersHouse2F to PlayersHouse1F when stepping on the warp tile", () => {
    // We need to mock the module function `getMapMetadataByGroup`
    // Since we are inside the test file, we can use jest.mock at the top level, but let's try to use a subclass to override the method if it was a method.
    // It is not a method.
    
    // Alternative: Mock the module.
    jest.mock("@/engine/world/maps", () => ({
      getMapMetadataByGroup: () => ({ name: "PlayersHouse2F", width: 10, height: 10 }),
      getMapEnvironment: () => "indoor",
    }));
    
    // Re-import Overworld to apply mock? No, jest.mock is hoisted.
    // But I'm writing this inside the test body which is too late for hoisting if I use create_file.
    // I'll put the mock at the top of the file content.
  });
});

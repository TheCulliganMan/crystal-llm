import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/world/events";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { getMapMetadataByConstant, getMapMetadataByName } from "@pokecrystal/core/engine/world/maps";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { resolveCollisionValue } from "@pokecrystal/core/engine/world/overworld/collision-data";
import {
  NewLoadMapCommand,
  WarpCheckCommand,
  WarpCommand,
  WarpFacingCommand,
  WarpSoundCommand,
} from "./overworld";

describe("Warp script commands", () => {
  test("WarpCommand writes raw tile coordinates (ASM Script_warp)", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const metadata = getMapMetadataByConstant("FAST_SHIP_1F");
    if (!metadata) {
      throw new Error("Test map metadata missing for FAST_SHIP_1F.");
    }
    const overworld = {
      TILES_PER_COLLISION: 2,
      WALK_FRAMES: 16,
      load_map: jest.fn(),
    } as unknown as OverworldEngine;

    const command = new WarpCommand("FAST_SHIP_1F", 25, 1);
    command.execute(gameState, eventManager, overworld);

    expect(gameState.wram.wXCoord).toBe(25);
    expect(gameState.wram.wYCoord).toBe(1);
    expect(gameState.hram.hMapEntryMethod).toBe(0xf1);
    expect((overworld as any).load_map).toHaveBeenCalledWith(metadata.name);
  });

  test("WarpFacingCommand sets player facing before warp (ASM Script_warpfacing)", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const metadata = getMapMetadataByConstant("NEW_BARK_TOWN");
    if (!metadata) {
      throw new Error("Test map metadata missing for NEW_BARK_TOWN.");
    }
    const player = { turn: jest.fn() };
    const overworld = {
      TILES_PER_COLLISION: 2,
      WALK_FRAMES: 16,
      player_direction: "down",
      player_object: player,
      load_map: jest.fn(),
    } as unknown as OverworldEngine;

    const command = new WarpFacingCommand("LEFT", "NEW_BARK_TOWN", 13, 6);
    command.execute(gameState, eventManager, overworld);

    expect((overworld as any).player_direction).toBe("left");
    expect(player.turn).toHaveBeenCalledWith("left");
    expect(gameState.wram.wXCoord).toBe(13);
    expect(gameState.wram.wYCoord).toBe(6);
    expect((overworld as any).load_map).toHaveBeenCalledWith(metadata.name);
  });

  test("WarpSoundCommand plays the warp SFX based on collision", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const audioEngine = { play_sound: jest.fn() };
    const overworld = {
      _current_tile_permission: () => resolveCollisionValue("DOOR"),
      audio_engine: audioEngine,
    } as unknown as OverworldEngine;
    const runner = { last_sound_effect: null };

    const command = new WarpSoundCommand();
    command.runner = runner as unknown as typeof command.runner;
    command.execute(gameState, eventManager, overworld);

    expect(runner.last_sound_effect).toBe("SFX_ENTER_DOOR");
    expect(audioEngine.play_sound).toHaveBeenCalledWith("SFX_ENTER_DOOR");
  });

  test("WarpCheckCommand forces a warp check and enables events", () => {
    const gameState = createInitialGameState();
    gameState.wram.wEnabledPlayerEvents = 0;
    const eventManager = new EventManager(gameState);
    const check_for_warp_event = jest.fn().mockReturnValue(true);
    const overworld = { check_for_warp_event } as unknown as OverworldEngine;

    const command = new WarpCheckCommand();
    command.execute(gameState, eventManager, overworld);

    expect(check_for_warp_event).toHaveBeenCalledWith({ allow_script: true, ignore_cooldown: true });
    expect(gameState.wram.wEnabledPlayerEvents).toBe(0xff);
  });

  test("NewLoadMapCommand reloads the current map with the requested entry method", () => {
    const gameState = createInitialGameState();
    const metadata = getMapMetadataByConstant("NEW_BARK_TOWN");
    if (!metadata) {
      throw new Error("Test map metadata missing for NEW_BARK_TOWN.");
    }
    gameState.wram.wMapGroup = metadata.groupId;
    gameState.wram.wMapNumber = metadata.mapId;
    gameState.wram.wXCoord = 5;
    gameState.wram.wYCoord = 6;
    const eventManager = new EventManager(gameState);
    const overworld = {
      load_map: jest.fn(),
    } as unknown as OverworldEngine;

    const command = new NewLoadMapCommand("MAPSETUP_LINKRETURN");
    command.execute(gameState, eventManager, overworld);

    expect(gameState.hram.hMapEntryMethod).toBe(0xf8);
    expect((overworld as any).player_x).toBe(11);
    expect((overworld as any).player_y).toBe(13);
    expect((overworld as any).load_map).toHaveBeenCalledWith(metadata.name);
  });

  test("NewLoadMapCommand returns from TradeCenter to Pokecenter2F via the room exit warp", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    dataLoader.ensure_overworld_data({ map_name: "TradeCenter" });
    dataLoader.ensure_overworld_data({ map_name: "Pokecenter2F" });

    const tradeCenter = getMapMetadataByName("TradeCenter");
    if (!tradeCenter) {
      throw new Error("Test map metadata missing for TradeCenter.");
    }
    gameState.wram.wMapGroup = tradeCenter.groupId;
    gameState.wram.wMapNumber = tradeCenter.mapId;

    const overworld = {
      TILES_PER_COLLISION: 2,
      current_map_name: "TradeCenter",
      load_map: jest.fn(),
    } as unknown as OverworldEngine;
    const runner = {
      dataLoader,
      data_loader: dataLoader,
      stop_all_scripts: jest.fn(),
    };

    const command = new NewLoadMapCommand("MAPSETUP_LINKRETURN");
    command.runner = runner as unknown as typeof command.runner;
    command.execute(gameState, eventManager, overworld);

    expect(gameState.hram.hMapEntryMethod).toBe(0xf8);
    expect((overworld as any).player_x).toBe(11);
    expect((overworld as any).player_y).toBe(1);
    expect((overworld as any).load_map).toHaveBeenCalledWith("Pokecenter2F");
    expect(gameState.wram.wMapGroup).toBe(getMapMetadataByName("Pokecenter2F")?.groupId);
    expect(gameState.wram.wMapNumber).toBe(getMapMetadataByName("Pokecenter2F")?.mapId);
  });
});

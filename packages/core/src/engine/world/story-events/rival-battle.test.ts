import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { ScriptRunnerImpl } from "./runner";
import { ApplyMovementCommand } from "./commands/overworld";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
// @ts-ignore - handlePostBattleSceneFixes is not exported yet
import { handlePostBattleSceneFixes } from "./commands/battle";

describe("Rival Battle Encounter Reproduction", () => {
  it("ApplyMovementCommand aligns rival Y coordinate correctly (reproduction of coordinate mismatch)", () => {
    const gameState = createInitialGameState();
    // Subtile Y coord (Raw Y)
    gameState.wram.wYCoord = 20; // This is raw/subtile coordinate. stride is 2. Tile Y would be 10.
    
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    
    const rivalObj = {
      objectId: "CHERRYGROVECITY_RIVAL",
      event: { x: 5, y: 5 }, // Raw coordinates in event
      x: 10, y: 10, // Subtile coordinates
      applyMovement: jest.fn(),
    };
    
    const moveObjectSpy = jest.fn();
    
    const overworld = {
      current_map_name: "CherrygroveCity",
      get_object_by_id: (id: string) => (id === "CHERRYGROVECITY_RIVAL" ? rivalObj : null),
      get_movement_data: () => ["step"],
      move_object: moveObjectSpy,
    } as unknown as OverworldEngine;
    
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const command = new ApplyMovementCommand("CHERRYGROVECITY_RIVAL", "CherrygroveCity_RivalWalksToYou");
    command.runner = runner;
    
    command.execute(gameState, eventManager, overworld);
    
    // FIXED BEHAVIOR:
    // It should divide by 2 to get the tile coordinate (10).
    expect(moveObjectSpy).toHaveBeenCalledWith("CHERRYGROVECITY_RIVAL", 5, 10);
  });

  it("handlePostBattleSceneFixes sets the rival flag (verification of correct visibility flag logic)", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    
    const overworld = {
      current_map_name: "CherrygroveCity",
    } as unknown as OverworldEngine;
    
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    
    gameState.wram.map_scenes["CherrygroveCity"] = "SCENE_CHERRYGROVECITY_MEET_RIVAL";
    gameState.wram.event_flags["EVENT_RIVAL_CHERRYGROVE_CITY"] = false; // Rival is visible
    
    handlePostBattleSceneFixes(runner, gameState, overworld, 0);
    
    // FIXED BEHAVIOR:
    expect(gameState.wram.event_flags["EVENT_RIVAL_CHERRYGROVE_CITY"]).toBe(true);
  });
});

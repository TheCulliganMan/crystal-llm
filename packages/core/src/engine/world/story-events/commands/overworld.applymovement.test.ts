import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/world/events";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { ApplyMovementCommand } from "./overworld";

describe("ApplyMovementCommand", () => {
  test("does not mirror leader movement onto follower (ASM ApplyMovementToFollower)", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const movementData = ["step DOWN", "step_end"];
    const leader = { applyMovement: jest.fn() };
    const follower = { applyMovement: jest.fn() };
    const overworld = {
      leader,
      follower,
      get_object_by_id: (id: string) => (id === "LEADER" ? leader : id === "FOLLOWER" ? follower : null),
      get_movement_data: (label: string) => (label === "Movement_Test" ? movementData : null),
    } as unknown as OverworldEngine;

    const command = new ApplyMovementCommand("LEADER", "Movement_Test");

    command.execute(gameState, eventManager, overworld);

    expect(leader.applyMovement).toHaveBeenCalledWith(movementData);
    expect(follower.applyMovement).not.toHaveBeenCalled();
  });

  test("spawns hidden movement targets through object constants", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const movementData = ["step UP", "step_end"];
    const rival = { applyMovement: jest.fn(), x: 11, y: 22 };
    const liveObjects = new Map<string | number, typeof rival>();
    const appearObject = jest.fn((id: string | number) => {
      if (id === 10) {
        liveObjects.set(10, rival);
        liveObjects.set("AZALEATOWN_RIVAL", rival);
      }
    });
    const overworld = {
      current_map_name: "AzaleaTown",
      appear_object: appearObject,
      get_object_by_id: (id: string | number) => liveObjects.get(id) ?? null,
      get_movement_data: (label: string) => (label === "Movement_AzaleaRivalExit" ? movementData : null),
      resolve_object_index: (id: string) => (id === "AZALEATOWN_RIVAL" ? 10 : null),
    } as unknown as OverworldEngine;

    const command = new ApplyMovementCommand("AZALEATOWN_RIVAL", "Movement_AzaleaRivalExit");

    command.execute(gameState, eventManager, overworld);

    expect(appearObject).toHaveBeenCalledWith(10, { force_spawn: true });
    expect(rival.applyMovement).toHaveBeenCalledWith(movementData);
  });

  test("resolves local movement labels against the current parent script", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const movementData = ["step LEFT", "step_end"];
    const target = { applyMovement: jest.fn() };
    const getMovementData = jest.fn((label: string, parentScript?: string | null) =>
      label === ".Movement1" && parentScript === "CeladonEusine" ? movementData : null
    );
    const overworld = {
      get_object_by_id: (id: string) => (id === "EUSINE" ? target : null),
      get_movement_data: getMovementData,
    } as unknown as OverworldEngine;
    const command = new ApplyMovementCommand("EUSINE", ".Movement1");
    command.runner = {
      _find_parent_script_name: () => "CeladonEusine",
      pause: jest.fn(),
      resume: jest.fn(),
    } as any;

    command.execute(gameState, eventManager, overworld);

    expect(getMovementData).toHaveBeenCalledWith(".Movement1", "CeladonEusine");
    expect(target.applyMovement).toHaveBeenCalledWith(movementData);
  });
});

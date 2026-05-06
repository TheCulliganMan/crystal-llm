import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/world/events";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import {
  StopFollowCommand,
  LockCommand,
  ReleaseCommand,
  LockAllCommand,
  ReleaseAllCommand,
  StopCommand,
} from "./overworld";

const createOverworldStub = (): OverworldEngine => {
  const overworld = {
    followStopped: false,
    playerLocked: false,
    playerUnlocked: false,
    allLocked: false,
    allUnlocked: false,
    playerStopped: false,
    stop_following() {
      this.followStopped = true;
    },
    lock_player_movement() {
      this.playerLocked = true;
    },
    unlock_player_movement() {
      this.playerUnlocked = true;
    },
    lock_all_movement() {
      this.allLocked = true;
    },
    unlock_all_movement() {
      this.allUnlocked = true;
    },
    stop_player_movement() {
      this.playerStopped = true;
    },
  };
  return overworld as unknown as OverworldEngine;
};

describe("Story event overworld commands", () => {
  test("invokes instance methods with overworld as this", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = createOverworldStub();

    const overworldContext = overworld as unknown as OverworldEngine;
    new StopFollowCommand().execute(gameState, eventManager, overworldContext);
    new LockCommand().execute(gameState, eventManager, overworldContext);
    new ReleaseCommand().execute(gameState, eventManager, overworldContext);
    new LockAllCommand().execute(gameState, eventManager, overworldContext);
    new ReleaseAllCommand().execute(gameState, eventManager, overworldContext);
    new StopCommand().execute(gameState, eventManager, overworldContext);

    expect(overworld.followStopped).toBe(true);
    expect(overworld.playerLocked).toBe(true);
    expect(overworld.playerUnlocked).toBe(true);
    expect(overworld.allLocked).toBe(true);
    expect(overworld.allUnlocked).toBe(true);
    expect(overworld.playerStopped).toBe(true);
  });
});

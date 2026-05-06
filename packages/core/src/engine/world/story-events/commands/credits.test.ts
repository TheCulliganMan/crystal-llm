import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager, type StartCreditsEvent } from "@pokecrystal/core/engine/events/events";
import { Spawn } from "@pokecrystal/core/engine/world/maps";
import { createOverworldStub } from "@pokecrystal/core/engine/world/story-events/test-utils";
import { CreditsCommand } from "./credits";

describe("CreditsCommand", () => {
  it("allows skipping when a legacy save has Hall of Fame entries but no migrated count", () => {
    const gameState = createInitialGameState();
    gameState.sram.hall_of_fame = [{ win_count: 1, team: [{ species: "CYNDAQUIL" }] }];
    gameState.wram.wHallOfFameCount = 0;
    const eventManager = new EventManager(gameState);
    const allowSkip: boolean[] = [];
    eventManager.on("start_credits", (event: StartCreditsEvent) => {
      allowSkip.push(event.data.allow_skip);
    });

    new CreditsCommand().execute(gameState, eventManager, createOverworldStub());

    expect(allowSkip).toEqual([true]);
  });

  it("respawns at New Bark after Lance credits when no credits screen listener is installed", () => {
    const gameState = createInitialGameState();
    gameState.wram.wSpawnAfterChampion = 1;
    const eventManager = new EventManager(gameState);
    const loadMap = jest.fn();

    new CreditsCommand().execute(
      gameState,
      eventManager,
      createOverworldStub({
        load_map: loadMap,
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
      }),
    );

    expect(gameState.wram.wSpawnAfterChampion).toBe(0);
    expect(gameState.wram.wDefaultSpawnpoint).toBe(Spawn.NEW_BARK);
    expect(gameState.wram.wMapGroup).toBe(24);
    expect(gameState.wram.wMapNumber).toBe(4);
    expect(loadMap).toHaveBeenCalledWith("NewBarkTown");
  });

  it("runs the post-credits spawn after a credits screen listener completes", () => {
    const gameState = createInitialGameState();
    gameState.wram.wSpawnAfterChampion = 1;
    const eventManager = new EventManager(gameState);
    let onComplete: (() => void) | null = null;
    eventManager.on("start_credits", (event: StartCreditsEvent) => {
      onComplete = event.data.on_complete;
    });

    new CreditsCommand().execute(gameState, eventManager, createOverworldStub());

    expect(gameState.wram.wSpawnAfterChampion).toBe(1);
    expect(onComplete).not.toBeNull();
    onComplete?.();
    expect(gameState.wram.wSpawnAfterChampion).toBe(0);
    expect(gameState.wram.wDefaultSpawnpoint).toBe(Spawn.NEW_BARK);
  });
});

import { createInitialGameState } from "@pokecrystal/core/core/state";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import { TownMapOverlay } from "@pokecrystal/core/ui/overlays/town-map-overlay";

describe("TownMapOverlay runner coordination", () => {
  it("avoids double-pausing when the runner is already awaiting resume", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overlay = new TownMapOverlay({} as unknown, gameState, {
      lock_movement: jest.fn(),
      unlock_movement: jest.fn(),
    });
    (overlay as any).renderMapSurface = jest.fn();
    overlay.register(eventManager);

    const runner = { pause: jest.fn(), resume: jest.fn(), _awaiting_resume: 1 };
    eventManager.dispatch(new Event("show_town_map", { runner }));

    expect(runner.pause).not.toHaveBeenCalled();

    overlay.close();
    expect(runner.resume).toHaveBeenCalledTimes(1);
  });

  it("pauses and resumes when the runner is active", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overlay = new TownMapOverlay({} as unknown, gameState, {
      lock_movement: jest.fn(),
      unlock_movement: jest.fn(),
    });
    (overlay as any).renderMapSurface = jest.fn();
    overlay.register(eventManager);

    const runner = { pause: jest.fn(), resume: jest.fn(), _awaiting_resume: 0 };
    eventManager.dispatch(new Event("show_town_map", { runner }));

    expect(runner.pause).toHaveBeenCalledTimes(1);

    overlay.close();
    expect(runner.resume).toHaveBeenCalledTimes(1);
  });
});

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

  it("resets the standalone cursor to the player and implements ASM Up/Down/B controls", () => {
    const gameState = createInitialGameState();
    const overlay = new TownMapOverlay({} as unknown, gameState, {
      lock_movement: jest.fn(),
      unlock_movement: jest.fn(),
    });
    const stateMachine = (overlay as any).stateMachine;
    stateMachine.refresh = jest.fn();
    stateMachine.resetMapCursorToPlayer = jest.fn();
    stateMachine.moveMapCursor = jest.fn();
    (overlay as any).renderMapSurface = jest.fn();

    overlay.show();

    expect(stateMachine.resetMapCursorToPlayer).toHaveBeenCalledTimes(1);
    expect(overlay.handle_input({ type: "keydown", direction: "up", is_press: true })).toBe(true);
    expect(stateMachine.moveMapCursor).toHaveBeenCalledWith(1);
    expect(overlay.visible).toBe(true);

    overlay.handle_input({ type: "keydown", direction: "down", is_press: true });
    expect(stateMachine.moveMapCursor).toHaveBeenCalledWith(-1);

    overlay.handle_input({ type: "keydown", button: "a", is_press: true });
    expect(overlay.visible).toBe(true);

    overlay.handle_input({ type: "keydown", button: "b", is_press: true });
    expect(overlay.visible).toBe(false);
  });
});

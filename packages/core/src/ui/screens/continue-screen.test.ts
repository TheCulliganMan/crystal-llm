import { createInitialGameState } from "@pokecrystal/core/core/state";
import { Surface } from "@pokecrystal/core/ui/surface";
import { ContinueScreen } from "./continue-screen";

describe("ContinueScreen", () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it("skips hidden pixel rendering for pure text UIs", () => {
    const gameState = createInitialGameState();
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      clearScreen: jest.fn(),
      drawBox: jest.fn(),
      renderSnapshot: jest.fn(),
    };
    const screen = new ContinueScreen(ui, gameState);

    screen.draw();

    expect(ui.clearScreen).not.toHaveBeenCalled();
    expect(ui.drawBox).not.toHaveBeenCalled();
    expect(ui.renderSnapshot).toHaveBeenCalled();
  });

  it("includes shared continue controls in the text snapshot", () => {
    const gameState = createInitialGameState();
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      clearScreen: jest.fn(),
      drawBox: jest.fn(),
      renderSnapshot: jest.fn(),
    };
    const screen = new ContinueScreen(ui, gameState);

    screen.draw();

    expect(ui.renderSnapshot).toHaveBeenCalledWith(
      ["CONTINUE"],
      expect.arrayContaining(["STATE: continue_screen", "A=Continue B=Back"]),
      "Continue",
      "Continue",
      null,
      null,
      null
    );
  });

  it("renders with a render_text-only font", () => {
    const gameState = createInitialGameState();
    const render_text = jest.fn();
    const ui = {
      screen: new Surface(160, 144),
      font: { render_text },
      clearScreen: jest.fn(),
      drawBox: jest.fn(),
    };
    const screen = new ContinueScreen(ui as any, gameState);

    screen.draw();

    expect(render_text).toHaveBeenCalledWith("PLAYER", 8, 16, ui.screen, { uppercase: true });
    expect(render_text).toHaveBeenCalledWith(" 0", 104, 32, ui.screen, { uppercase: true });
  });

  it("syncs the live clock before rendering snapshot data", () => {
    jest.setSystemTime(new Date("2024-01-01T07:05:00"));
    const gameState = createInitialGameState();
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      clearScreen: jest.fn(),
      drawBox: jest.fn(),
      renderSnapshot: jest.fn(),
    };
    const screen = new ContinueScreen(ui as any, gameState);

    screen.draw();

    expect(gameState.sram.game_time_hours).toBe(7);
    expect(gameState.sram.game_time_minutes).toBe(5);
    expect(ui.renderSnapshot).toHaveBeenCalledWith(
      ["CONTINUE"],
      expect.arrayContaining(["TIME:   7:05", "DAY: SUNDAY"]),
      "Continue",
      "Continue",
      null,
      null,
      null,
    );
  });
});

import { createInitialGameState } from "@pokecrystal/core/core/state";
import { KEYS } from "@pokecrystal/core/core/keycodes";
import { SCREEN_TILE_WIDTH, TEXTBOX_HEIGHT_TILES, TEXTBOX_Y_TILES } from "@pokecrystal/core/core/text-constants";
import { Surface } from "@pokecrystal/core/ui/surface";
import { DayOfWeekScreen } from "./day-of-week-screen";

const makeUi = () => {
  return {
    screen: new Surface(160, 144),
    drawTextBox: jest.fn(),
    drawWindow: jest.fn(),
    draw_window: jest.fn(),
    font: { renderText: jest.fn() },
    renderSnapshot: jest.fn(),
  };
};

describe("DayOfWeekScreen", () => {
  it("draws the weekday question in the standard ASM textbox region", () => {
    const gameState = createInitialGameState();
    const ui = makeUi();
    const screen = new DayOfWeekScreen(ui, gameState, null);

    screen.draw();

    expect(ui.drawTextBox).toHaveBeenCalledWith(
      ui.screen,
      "What day is it?",
      0,
      TEXTBOX_Y_TILES,
      SCREEN_TILE_WIDTH,
      TEXTBOX_HEIGHT_TILES,
      expect.any(Number),
      undefined,
      undefined,
      expect.any(Number),
    );
  });

  it("renders confirmation text in the same textbox region and records prompt snapshot context", () => {
    const gameState = createInitialGameState();
    const ui = makeUi();
    const screen = new DayOfWeekScreen(ui, gameState, null);

    screen.handleInput({ type: KEYS.KEYDOWN, key: KEYS.Z, code: "KeyZ" });
    screen.handleInput({ type: KEYS.KEYUP, key: KEYS.Z, code: "KeyZ" });
    screen.handleInput({ type: KEYS.KEYDOWN, key: KEYS.Z, code: "KeyZ" });
    ui.drawTextBox.mockClear();
    ui.renderSnapshot.mockClear();

    screen.draw();

    expect(ui.drawTextBox).toHaveBeenCalledWith(
      ui.screen,
      expect.stringContaining("is it?"),
      0,
      TEXTBOX_Y_TILES,
      SCREEN_TILE_WIDTH,
      TEXTBOX_HEIGHT_TILES,
      expect.any(Number),
      undefined,
      undefined,
      expect.any(Number),
    );
    expect(ui.renderSnapshot).toHaveBeenCalledWith(
      ["Prompt"],
      ["A=OK B=Cancel"],
      "Prompt",
      "Legend",
      null,
      null,
      [expect.stringContaining("is it?")],
    );
  });

  it("ignores one carried confirm press, then opens confirmation on a fresh press even if keyup was lost", () => {
    const gameState = createInitialGameState();
    const ui = makeUi();
    const screen = new DayOfWeekScreen(ui, gameState, null);

    screen.handleInput({ type: KEYS.KEYDOWN, key: KEYS.Z, code: "KeyZ" });
    screen.draw();

    expect(ui.renderSnapshot).toHaveBeenLastCalledWith(
      ["Prompt"],
      ["Up/Down=Choose A=OK"],
      "Prompt",
      "Legend",
      expect.any(Array),
      null,
      ["What day is it?"]
    );

    screen.handleInput({ type: KEYS.KEYDOWN, key: KEYS.Z, code: "KeyZ" });
    ui.renderSnapshot.mockClear();
    screen.draw();

    expect(ui.renderSnapshot).toHaveBeenLastCalledWith(
      ["Prompt"],
      ["A=OK B=Cancel"],
      "Prompt",
      "Legend",
      null,
      null,
      [expect.stringContaining("is it?")]
    );
  });

  it("accepts direct player direction and button events", () => {
    const gameState = createInitialGameState();
    const ui = makeUi();
    const screen = new DayOfWeekScreen(ui, gameState, null);

    screen.handleInput({ type: "keydown", direction: "up", is_press: true });
    expect(screen.getSelectedDay()).toBe(1);

    screen.handleInput({ type: "keyup", button: "a", is_press: false });
    screen.handleInput({ type: "keydown", button: "a", is_press: true });

    expect(screen.isConfirming()).toBe(true);
  });
});

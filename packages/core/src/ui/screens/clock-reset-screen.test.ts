import { createInitialGameState } from "@pokecrystal/core/core/state";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { Surface } from "@pokecrystal/core/ui/surface";
import { ClockResetScreen } from "./clock-reset-screen";

describe("ClockResetScreen", () => {
  it("draws the reset confirmation yes/no box at ASM YesNoBox coordinates", () => {
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawBox: jest.fn(),
      drawTextBox: jest.fn(),
      _recordWindowRegion: jest.fn(),
    };
    const screen = new ClockResetScreen(ui, createInitialGameState());

    screen.draw();

    expect(ui.drawBox).toHaveBeenCalledWith(
      ui.screen,
      14 * TILE_SIZE,
      7 * TILE_SIZE,
      6,
      4,
    );
    expect(ui._recordWindowRegion).toHaveBeenCalledWith(
      ui.screen,
      14 * TILE_SIZE,
      7 * TILE_SIZE,
      6,
      4,
      20,
    );
  });

  it("skips pixel boxes when rendering to a pure text UI", () => {
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawBox: jest.fn(),
      drawTextBox: jest.fn(),
      _recordWindowRegion: jest.fn(),
      renderSnapshot: jest.fn(),
    };
    const screen = new ClockResetScreen(ui, createInitialGameState());

    screen.draw();

    expect(ui.drawBox).not.toHaveBeenCalled();
    expect(ui.drawTextBox).not.toHaveBeenCalled();
    expect(ui.renderSnapshot).toHaveBeenCalled();
  });

  it("renders the reset confirmation with shared prompt snapshot semantics", () => {
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawBox: jest.fn(),
      drawTextBox: jest.fn(),
      _recordWindowRegion: jest.fn(),
      renderSnapshot: jest.fn(),
    };
    const screen = new ClockResetScreen(ui, createInitialGameState());

    screen.draw();

    expect(ui.renderSnapshot).toHaveBeenCalledWith(
      ["Prompt"],
      ["Up/Down=Choose A=OK B=Cancel"],
      "Prompt",
      "Legend",
      null,
      ["   YES", "▶ NO"],
      ["Reset clock?"]
    );
  });
});

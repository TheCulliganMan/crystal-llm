import { GameButton } from "@pokecrystal/core/input/buttons";
import { Surface } from "@pokecrystal/core/ui/surface";
import { Menu } from "./menu";
import type { MenuUI } from "./types";

const createMenuUi = (): MenuUI => ({
  screen: null,
  tileSize: 8,
  font: { renderText: jest.fn() } as MenuUI["font"],
  drawWindow: jest.fn(),
});

describe("Menu", () => {
  it("moves silently and only clicks on confirm/cancel", () => {
    const audioEngine = { playSound: jest.fn() };
    const menu = new Menu(createMenuUi(), ["YES", "NO"], 0, 0, 8, null, 1, 0, null, audioEngine);

    menu.handleInput({ type: "keydown", key: "ArrowDown" });
    expect(audioEngine.playSound).not.toHaveBeenCalled();

    menu.handleInput({
      type: "keydown",
      is_press: true,
      button: GameButton.A,
    });
    expect(audioEngine.playSound).toHaveBeenCalledWith("menu_option");
  });

  it("throws when confirmation targets an invalid option index instead of returning null", () => {
    const menu = new Menu(createMenuUi(), ["YES", "NO"], 0, 0, 8);
    menu.selectedOption = 5;

    expect(() =>
      menu.handleInput({
        type: "keydown",
        is_press: true,
        button: GameButton.A,
      }),
    ).toThrow("Menu selected invalid option index 5");
  });

  it("draws with a render_text-only font", () => {
    const render_text = jest.fn();
    const ui = {
      screen: new Surface(160, 144),
      tileSize: 8,
      font: { render_text },
      drawWindow: jest.fn(),
      getContextPalette: jest.fn().mockReturnValue([
        [255, 255, 255],
        [192, 192, 192],
        [96, 96, 96],
        [0, 0, 0],
      ]),
    } as unknown as MenuUI;
    const menu = new Menu(ui, ["YES", "NO"], 0, 0, 8);

    menu.draw();

    expect(render_text).toHaveBeenCalledWith("▶YES", 8, 16, ui.screen, {
      palette: expect.any(Array),
      textWidth: 48,
      maxLines: 1,
    });
  });
});

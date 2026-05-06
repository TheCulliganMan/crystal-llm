import { createInitialGameState } from "@pokecrystal/core/core/state";
import { GameButton } from "@pokecrystal/core/input/buttons";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { MainMenu } from "./main-menu";
import type { MenuUI } from "./types";

const createMenuUi = (): MenuUI => ({
  screen: new gameEngine.Surface(160, 144),
  tileSize: 8,
  font: { renderText: jest.fn() } as MenuUI["font"],
  drawWindow: jest.fn(),
});

describe("MainMenu", () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it("moves the cursor silently like the asm scrolling menu", () => {
    const gameState = createInitialGameState();
    const audioEngine = { playSound: jest.fn() } as any;
    const menu = new MainMenu(createMenuUi(), audioEngine, gameState, true) as MainMenu & {
      selectedOption: number;
    };

    menu.handleInput({ type: "keydown", key: gameEngine.K_DOWN });

    expect(menu.selectedOption).toBe(1);
    expect(audioEngine.playSound).not.toHaveBeenCalled();
  });

  it("plays the click sound when confirming CONTINUE", () => {
    const gameState = createInitialGameState();
    const audioEngine = { playSound: jest.fn() } as any;
    const menu = new MainMenu(createMenuUi(), audioEngine, gameState, true);

    const action = menu.handleInput({
      type: "keydown",
      is_press: true,
      button: GameButton.A,
    });

    expect(action).toBe("show_continue_screen");
    expect(audioEngine.playSound).toHaveBeenCalledWith("menu_option");
  });

  it("omits Mystery Gift when no save file exists", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "KRIS";
    gameState.sram.mystery_gift_unlocked = true;

    const menu = new MainMenu(createMenuUi(), { playSound: jest.fn() } as any, gameState, false) as MainMenu & {
      menuOptions: string[];
    };

    expect(menu.menuOptions).toEqual(["NEW GAME", "OPTION"]);
  });

  it("includes Mystery Gift only after CONTINUE becomes available", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "KRIS";
    gameState.sram.mystery_gift_unlocked = true;

    const menu = new MainMenu(createMenuUi(), { playSound: jest.fn() } as any, gameState, true) as MainMenu & {
      menuOptions: string[];
    };

    expect(menu.menuOptions).toEqual(["CONTINUE", "NEW GAME", "OPTION", "MYSTERY GIFT"]);
  });

  it("throws when a selected menu label does not map to an ASM action", () => {
    const gameState = createInitialGameState();
    const menu = new MainMenu(createMenuUi(), { playSound: jest.fn() } as any, gameState) as MainMenu & {
      menuOptions: string[];
      selectedOption: number;
    };

    menu.menuOptions = ["GLITCH"];
    menu.selectedOption = 0;

    expect(() =>
      menu.handleInput({
        type: "keydown",
        is_press: true,
        button: GameButton.A,
      }),
    ).toThrow("MainMenu selected unsupported ASM option 'GLITCH'");
  });

  it("emits a text snapshot when a text renderer is available", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "KRIS";
    const renderSnapshot = jest.fn();
    const ui = {
      ...createMenuUi(),
      renderSnapshot,
    } as MenuUI & {
      renderSnapshot: jest.Mock;
    };

    const menu = new MainMenu(ui, { playSound: jest.fn() } as any, gameState, true);

    menu.draw();

    expect(renderSnapshot).toHaveBeenCalledWith(
      ["MAIN MENU"],
      expect.arrayContaining(["STATE: main_menu", "SAVE EXISTS: yes", "Up/Down=Move A=Confirm"]),
      "Main Menu",
      "Main Menu",
      expect.arrayContaining(["▶ CONTINUE", "  NEW GAME", "  OPTION"]),
      null,
      null,
    );
  });

  it("skips hidden pixel drawing when the UI is pure text", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "KRIS";
    const ui = {
      ...createMenuUi(),
      renderSnapshot: jest.fn(),
    } as MenuUI & {
      renderSnapshot: jest.Mock;
    };
    const blitSpy = jest.spyOn(ui.screen, "blit");
    const menu = new MainMenu(ui, { playSound: jest.fn() } as any, gameState, true);

    menu.draw();

    expect(ui.drawWindow).not.toHaveBeenCalled();
    expect(blitSpy).not.toHaveBeenCalled();
    expect(ui.renderSnapshot).toHaveBeenCalled();
  });

  it("draws with a render_text-only font", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "KRIS";
    const render_text = jest.fn();
    const ui = {
      screen: new gameEngine.Surface(160, 144),
      tileSize: 8,
      font: { render_text },
      drawWindow: jest.fn(),
    } as unknown as MenuUI;
    const menu = new MainMenu(ui, { playSound: jest.fn() } as any, gameState, true);

    menu.draw();

    expect(render_text).toHaveBeenCalledWith("▶", 8, 16, expect.anything(), undefined);
    expect(render_text).toHaveBeenCalledWith("CONTINUE", 16, 16, expect.anything(), undefined);
  });

  it("syncs the live clock before text-only rendering", () => {
    jest.setSystemTime(new Date("2024-01-01T12:34:00"));
    const gameState = createInitialGameState();
    gameState.sram.player_name = "KRIS";
    const ui = {
      ...createMenuUi(),
      renderSnapshot: jest.fn(),
    } as MenuUI & {
      renderSnapshot: jest.Mock;
    };
    const menu = new MainMenu(ui, { playSound: jest.fn() } as any, gameState, true);

    menu.draw();

    expect(gameState.hram.hHours).toBe(12);
    expect(gameState.hram.hMinutes).toBe(34);
    expect(ui.renderSnapshot).toHaveBeenCalledWith(
      ["MAIN MENU"],
      expect.arrayContaining(["TIME: DAY12:34"]),
      "Main Menu",
      "Main Menu",
      expect.anything(),
      null,
      null,
    );
  });
});

import { createInitialGameState } from "@pokecrystal/core/core/state";
import { GameButton } from "@pokecrystal/core/input/buttons";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { MenuUI } from "./types";
import { StartMenu } from "./start-menu";

const createMenuUi = (): MenuUI => ({
  screen: null,
  tileSize: 8,
  font: { renderText: jest.fn() },
  drawWindow: jest.fn(),
});

describe("StartMenu", () => {
  it("plays the open sfx once and then moves silently until confirm like the asm start menu", () => {
    const gameState = createInitialGameState();
    const audioEngine = { playSound: jest.fn() } as any;
    const menu = new StartMenu(createMenuUi(), gameState, audioEngine);

    expect(audioEngine.playSound).toHaveBeenCalledWith("SFX_MENU");
    audioEngine.playSound.mockClear();

    menu.handleInput({ type: "keydown", key: gameEngine.K_DOWN });
    expect(audioEngine.playSound).not.toHaveBeenCalled();

    menu.handleInput({ type: "keydown", is_press: true, button: GameButton.A });
    expect(audioEngine.playSound).toHaveBeenCalledWith("menu_option");
  });

  it("replays the open sfx when the start menu is reopened", () => {
    const gameState = createInitialGameState();
    const audioEngine = { playSound: jest.fn() } as any;
    const menu = new StartMenu(createMenuUi(), gameState, audioEngine);

    audioEngine.playSound.mockClear();
    menu.resetCursorPosition();

    expect(audioEngine.playSound).toHaveBeenCalledWith("SFX_MENU");
  });

  it("omits the Pokedex entry before ENGINE_POKEDEX is set", () => {
    const gameState = createInitialGameState();
    const menu = new StartMenu(createMenuUi(), gameState);

    expect(menu.menuOptions).not.toContain("#DEX");
  });

  it("includes the Pokedex entry after ENGINE_POKEDEX is set", () => {
    const gameState = createInitialGameState();
    gameState.wram.engine_flags.ENGINE_POKEDEX = true;
    const menu = new StartMenu(createMenuUi(), gameState);

    expect(menu.menuOptions).toContain("#DEX");
  });

  it("throws if refresh produces no ASM-required entries", () => {
    const gameState = createInitialGameState();
    const menu = new StartMenu(createMenuUi(), gameState) as StartMenu & {
      orderedIdentifiers: () => string[];
    };
    menu.orderedIdentifiers = () => [];

    expect(() => menu.refresh()).toThrow(
      "StartMenu produced no entries; ASM always appends STATUS, SAVE/QUIT, OPTION, and EXIT",
    );
  });

  it("throws if ASM entry setup requests an unsupported identifier", () => {
    const gameState = createInitialGameState();
    const menu = new StartMenu(createMenuUi(), gameState) as StartMenu & {
      buildEntry: (identifier: string) => unknown;
    };

    expect(() => menu.buildEntry("totally_missing")).toThrow(
      "StartMenu requested unsupported ASM entry 'totally_missing'",
    );
  });
});

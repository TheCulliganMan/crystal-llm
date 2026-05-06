import { createInitialGameState } from "@pokecrystal/core/core/state";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { MenuUI } from "./types";
import { MenuState } from "./menu-state";

const createMenuUi = (): MenuUI => ({
  screen: null,
  tileSize: 8,
  font: { renderText: jest.fn(), fontTiles: {} },
  drawWindow: jest.fn(),
});

describe("MenuState bag item registration", () => {
  it("stores the registered key item for Select", () => {
    const gameState = createInitialGameState();
    gameState.sram.key_items.BICYCLE = 1;

    const audioEngine = { playSound: jest.fn() } as unknown as AudioEngine;
    const menuState = new MenuState(createMenuUi(), gameState, audioEngine, null);
    menuState.currentMenu = "bag_menu";

    menuState.handleInput(new gameEngine.event.Event("keydown", { key: gameEngine.K_RIGHT, code: gameEngine.K_RIGHT }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: gameEngine.K_RIGHT, code: gameEngine.K_RIGHT }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "Backspace", code: "Backspace" }));

    expect(gameState.wram.wRegisteredItem).toBe("BICYCLE");
    expect(gameState.wram.wWhichRegisteredItem).toBe(0x80 | 0x01);
  });
});

import { createInitialGameState } from "@pokecrystal/core/core/state";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { _CHAR_MAP, SPACE_TILE } from "@pokecrystal/core/ui/tilemap-surface";
import type { MenuUI } from "./types";
import { BagMenu } from "./bag-menu";

const createMenuUi = (): MenuUI => ({
  screen: null,
  tileSize: 8,
  font: { renderText: jest.fn(), fontTiles: {} },
  drawWindow: jest.fn(),
});

const buildBagTilemap = (bagMenu: BagMenu): { getTile: (x: number, y: number) => number } =>
  (bagMenu as unknown as { buildTilemap: () => { getTile: (x: number, y: number) => number } }).buildTilemap();

const expectTileText = (
  tilemap: { getTile: (x: number, y: number) => number },
  x: number,
  y: number,
  text: string,
): void => {
  Array.from(text).forEach((char, offset) => {
    expect(tilemap.getTile(x + offset, y)).toBe(char === " " ? SPACE_TILE : _CHAR_MAP[char]);
  });
};

describe("BagMenu register key item", () => {
  it("plays the pocket-switch sfx when changing pockets", () => {
    const gameState = createInitialGameState();
    const audioEngine = { playSound: jest.fn() } as unknown as AudioEngine;
    const bagMenu = new BagMenu(createMenuUi(), gameState, audioEngine);

    bagMenu.handleInput(new gameEngine.event.Event("keydown", { key: gameEngine.K_RIGHT, code: gameEngine.K_RIGHT }));

    expect(audioEngine.playSound).toHaveBeenCalledWith("SFX_SWITCH_POCKETS");
  });

  it("returns sel action on Select and exposes SEL in the action menu", () => {
    const gameState = createInitialGameState();
    gameState.sram.key_items.BICYCLE = 1;

    const audioEngine = { playSound: jest.fn() } as unknown as AudioEngine;
    const bagMenu = new BagMenu(createMenuUi(), gameState, audioEngine);

    bagMenu.handleInput(new gameEngine.event.Event("keydown", { key: gameEngine.K_RIGHT, code: gameEngine.K_RIGHT }));
    bagMenu.handleInput(new gameEngine.event.Event("keydown", { key: gameEngine.K_RIGHT, code: gameEngine.K_RIGHT }));

    const selection = bagMenu.handleInput(
      new gameEngine.event.Event("keydown", { key: "Backspace", code: "Backspace" }),
    );

    expect(selection).toEqual(["sel", "BICYCLE"]);

    bagMenu.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));

    expect(bagMenu.getMode()).toBe("actions");
    expect(bagMenu.getActionOptions()).toContain("SEL");
  });

  it("places item descriptions at the ASM textbox text origin", () => {
    const gameState = createInitialGameState();
    gameState.sram.items = { X_SPECIAL: 1 };

    const bagMenu = new BagMenu(createMenuUi(), gameState, null);
    const tilemap = buildBagTilemap(bagMenu);

    expect(tilemap.getTile(1, 13)).toBe(SPACE_TILE);
    expect(tilemap.getTile(1, 14)).toBe(_CHAR_MAP.R);
  });

  it("places the list cursor next to item names at the ASM scrolling menu columns", () => {
    const gameState = createInitialGameState();
    gameState.sram.items = { POTION: 1 };

    const bagMenu = new BagMenu(createMenuUi(), gameState, null);
    const tilemap = buildBagTilemap(bagMenu);

    expect(tilemap.getTile(6, 2)).toBe(SPACE_TILE);
    expect(tilemap.getTile(7, 2)).toBe(_CHAR_MAP["▶"]);
    expectTileText(tilemap, 8, 2, "POTION");
    expectTileText(tilemap, 16, 2, "×01");
  });

  it("renders key item names without the regular item quantity column", () => {
    const gameState = createInitialGameState();
    gameState.sram.key_items = { SQUIRTBOTTLE: 1 };

    const bagMenu = new BagMenu(createMenuUi(), gameState, null);
    bagMenu.handleInput(new gameEngine.event.Event("keydown", { key: gameEngine.K_RIGHT, code: gameEngine.K_RIGHT }));
    bagMenu.handleInput(new gameEngine.event.Event("keydown", { key: gameEngine.K_RIGHT, code: gameEngine.K_RIGHT }));
    const tilemap = buildBagTilemap(bagMenu);

    expectTileText(tilemap, 8, 2, "SQUIRTBOTTLE");
    expect(tilemap.getTile(16, 2)).toBe(_CHAR_MAP.T);
    expect(tilemap.getTile(19, 2)).toBe(_CHAR_MAP.E);
  });
});

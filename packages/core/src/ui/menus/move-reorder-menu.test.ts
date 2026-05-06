import { createInitialGameState } from "@pokecrystal/core/core/state";
import type { MenuUI } from "./types";
import { MoveReorderMenu } from "./move-reorder-menu";
import { MoveName } from "@pokecrystal/core/core/enums/move";
import { createTestPokemon } from "@pokecrystal/core/engine/world/story-events/test-utils";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { Surface } from "@pokecrystal/core/ui/surface";

const createMenuUi = (): MenuUI => ({
  screen: null,
  tileSize: 8,
  font: { renderText: jest.fn() },
  drawWindow: jest.fn(),
});

const tickMenu = (menu: MoveReorderMenu, frames: number = 12): void => {
  for (let i = 0; i < frames; i += 1) {
    menu.update();
  }
};

describe("MoveReorderMenu", () => {
  it("draws the ASM move screen textboxes at SetUpMoveScreenBG coordinates", () => {
    const gameState = createInitialGameState();
    const drawWindow = jest.fn();
    const renderText = jest.fn();
    const ui: MenuUI = {
      screen: new Surface(160, 144),
      tileSize: 8,
      font: { renderText },
      drawWindow,
    };
    const pokemon = createTestPokemon("TOTODILE", 17, {
      moves: [
        { name: MoveName.SCRATCH, current_pp: 11 },
        { name: MoveName.LEER, current_pp: 30 },
        { name: MoveName.RAGE, current_pp: 20 },
        { name: MoveName.WATER_GUN, current_pp: 25 },
      ],
    });
    const menu = new MoveReorderMenu(ui, gameState, null);

    menu.showPokemon(pokemon);
    menu.draw();

    expect(drawWindow).toHaveBeenNthCalledWith(
      1,
      ui.screen,
      0,
      8,
      20,
      11,
      expect.objectContaining({ fill: expect.any(Array) }),
    );
    expect(drawWindow).toHaveBeenNthCalledWith(
      2,
      ui.screen,
      0,
      88,
      20,
      7,
      expect.objectContaining({ fill: expect.any(Array) }),
    );
    expect(renderText).toHaveBeenCalledWith(
      "SCRATCH",
      16,
      24,
      ui.screen,
      expect.objectContaining({ textWidth: 136 }),
    );
    expect(renderText).toHaveBeenCalledWith(
      "PP 11/35",
      80,
      32,
      ui.screen,
      expect.any(Object),
    );
    expect(renderText).toHaveBeenCalledWith(
      "│TYPE/└",
      0,
      88,
      ui.screen,
      expect.any(Object),
    );
  });

  it("swaps moves after selecting two entries", () => {
    const gameState = createInitialGameState();
    const pokemon = createTestPokemon("TESTMON", 1, {
      moves: [
        { name: MoveName.TACKLE, current_pp: 35 },
        { name: MoveName.GROWL, current_pp: 40 },
      ],
    });
    const menu = new MoveReorderMenu(createMenuUi(), gameState, null);

    menu.showPokemon(pokemon);

    menu.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    tickMenu(menu);
    menu.handleInput(new gameEngine.event.Event("keydown", { key: "ArrowDown", code: "ArrowDown" }));
    menu.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));

    expect(pokemon.moves[0]?.name).toBe(MoveName.GROWL);
    expect(pokemon.moves[1]?.name).toBe(MoveName.TACKLE);
  });

  it("cancels swap without exiting on the first B press", () => {
    const gameState = createInitialGameState();
    const pokemon = createTestPokemon("TESTMON", 1, {
      moves: [
        { name: MoveName.TACKLE, current_pp: 35 },
        { name: MoveName.GROWL, current_pp: 40 },
      ],
    });
    const menu = new MoveReorderMenu(createMenuUi(), gameState, null);

    menu.showPokemon(pokemon);

    menu.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    tickMenu(menu);
    const firstCancel = menu.handleInput(new gameEngine.event.Event("keydown", { key: "x", code: "KeyX" }));

    expect(firstCancel).toBeNull();

    tickMenu(menu);
    const secondCancel = menu.handleInput(new gameEngine.event.Event("keydown", { key: "x", code: "KeyX" }));

    expect(secondCancel).toBe("exit");
  });

  it("returns the cursor to the swap origin when cancelling", () => {
    const gameState = createInitialGameState();
    const pokemon = createTestPokemon("TESTMON", 1, {
      moves: [
        { name: MoveName.TACKLE, current_pp: 35 },
        { name: MoveName.GROWL, current_pp: 40 },
      ],
    });
    const menu = new MoveReorderMenu(createMenuUi(), gameState, null);

    menu.showPokemon(pokemon);

    menu.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    tickMenu(menu);
    menu.handleInput(new gameEngine.event.Event("keydown", { key: "ArrowDown", code: "ArrowDown" }));

    expect(menu.getSelectionIndex()).toBe(1);

    tickMenu(menu);
    menu.handleInput(new gameEngine.event.Event("keydown", { key: "x", code: "KeyX" }));

    expect(menu.getSelectionIndex()).toBe(0);
  });
});

import { createInitialGameState } from "@pokecrystal/core/core/state";
import { gameEngine, Surface } from "@pokecrystal/core/ui/game-engine";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { MenuState } from "./menu-state";
import type { MenuUI } from "./types";

const createMenuUi = (): MenuUI => ({
  screen: new Surface(160, 144),
  tileSize: 8,
  font: { renderText: jest.fn() },
  drawWindow: jest.fn(),
});

describe("MenuState TM/HM prompt flow", () => {
  beforeEach(() => {
    jest.spyOn(gameEngine.display, "get_init").mockReturnValue(true);
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("routes TM/HM teach confirmation through frame-driven yes/no prompt", () => {
    const menuState = new MenuState(
      createMenuUi(),
      createInitialGameState(),
      { playSound: jest.fn() } as unknown as AudioEngine,
      null,
    );

    (menuState as any).tmhmContext = {
      itemName: "TM01",
      move: "DYNAMICPUNCH",
      isHm: false,
      stage: "INTRO_MOVE",
      nextStage: null,
      target: null,
      pendingCompletion: null,
      forgetOptions: [],
    };

    const startSelectionSpy = jest
      .spyOn(menuState as any, "startTmhmSelection")
      .mockImplementation(() => undefined);
    const showMessageSpy = jest.spyOn(menuState as any, "showTmhmMessage");

    (menuState as any).promptTmhmUse();
    expect((menuState as any).tmhmYesNoPrompt).not.toBeNull();

    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));

    expect(startSelectionSpy).toHaveBeenCalledTimes(1);
    expect(showMessageSpy).not.toHaveBeenCalled();
    expect((menuState as any).tmhmYesNoPrompt).toBeNull();
  });

  it("handles cancellation on TM/HM teach prompt without blocking", () => {
    const menuState = new MenuState(
      createMenuUi(),
      createInitialGameState(),
      { playSound: jest.fn() } as unknown as AudioEngine,
      null,
    );

    (menuState as any).tmhmContext = {
      itemName: "TM01",
      move: "DYNAMICPUNCH",
      isHm: false,
      stage: "INTRO_MOVE",
      nextStage: null,
      target: null,
      pendingCompletion: null,
      forgetOptions: [],
    };

    const startSelectionSpy = jest.spyOn(menuState as any, "startTmhmSelection");
    const showMessageSpy = jest.spyOn(menuState as any, "showTmhmMessage");

    (menuState as any).promptTmhmUse();
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "x", code: "KeyX" }));

    expect(startSelectionSpy).not.toHaveBeenCalled();
    expect(showMessageSpy).toHaveBeenCalledWith("The TM wasn't used.", "COMPLETE", false);
    expect((menuState as any).tmhmYesNoPrompt).toBeNull();
  });
});

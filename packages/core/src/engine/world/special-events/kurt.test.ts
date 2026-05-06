import { createInitialGameState } from "@pokecrystal/core/core/state";
import {
  createOverworldEngineStub,
  createScriptRunnerStub,
} from "@pokecrystal/core/engine/world/story-events/test-utils";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { Surface } from "@pokecrystal/core/ui/surface";
import * as asyncLoop from "@pokecrystal/core/ui/async-loop";
import { select_apricorn_for_kurt } from "./kurt";

const createPromptUi = () => ({
  screen: new Surface(160, 144),
  screenWidth: 160,
  screenHeight: 144,
  eventQueue: gameEngine.event.createQueue(),
  drawWindow: jest.fn(),
  draw_window: jest.fn(),
  clearScreen: jest.fn(),
  update: jest.fn(),
  font: { renderText: jest.fn() },
});

describe("select_apricorn_for_kurt", () => {
  it("uses runner-provided selection synchronously", () => {
    const gameState = createInitialGameState();
    gameState.sram.items.RED_APRICORN = 3;
    const runner = createScriptRunnerStub({
      variables: {
        _kurt_apricorn_type: "RED_APRICORN",
        _kurt_apricorn_quantity: 2,
      },
    });

    const result = select_apricorn_for_kurt(gameState, { runner });

    expect(result).toBe("RED_APRICORN");
    expect(gameState.sram.items.RED_APRICORN).toBe(1);
    expect(runner.variables.VAR_KURT_APRICORNS).toBe(2);
    expect(runner.last_condition_result).toBe(true);
  });

  it("runs the prompt asynchronously when UI input is required", async () => {
    const events = [
      [new gameEngine.event.Event("keydown", { code: "KeyZ" })],
      [new gameEngine.event.Event("keydown", { code: "KeyZ" })],
    ];
    const getEvents = jest.spyOn(gameEngine.event, "get").mockImplementation(() => events.shift() ?? []);
    const nextFrameMock = jest.spyOn(asyncLoop, "nextFrame").mockResolvedValue(undefined);

    const gameState = createInitialGameState();
    gameState.sram.items.RED_APRICORN = 2;
    const ui = createPromptUi();
    const overworld = createOverworldEngineStub({
      ui,
      draw: jest.fn(),
      audio_engine: null,
    });
    const runner = createScriptRunnerStub({ overworld });

    try {
      const result = await select_apricorn_for_kurt(gameState, { runner, overworld });

      expect(result).toBe("RED_APRICORN");
      expect(gameState.sram.items.RED_APRICORN).toBe(1);
      expect(runner.variables.VAR_KURT_APRICORNS).toBe(1);
      expect(runner.last_condition_result).toBe(true);
      expect(getEvents).toHaveBeenCalled();
      expect(nextFrameMock).toHaveBeenCalled();
    } finally {
      getEvents.mockRestore();
      nextFrameMock.mockRestore();
    }
  });
});

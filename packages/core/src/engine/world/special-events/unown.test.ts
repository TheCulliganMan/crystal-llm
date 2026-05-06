import { createInitialGameState } from "@pokecrystal/core/core/state";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import * as asyncLoop from "@pokecrystal/core/ui/async-loop";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { Surface } from "@pokecrystal/core/ui/surface";
import { display_unown_words } from "./unown";
import { getUnownOverlayLockDepth } from "./unown-overlay-lock";

type TestEventQueue = ReturnType<typeof gameEngine.event.createQueue>;

const createUi = (eventQueue: TestEventQueue | null) => ({
  screen: new Surface(160, 144),
  tile_size: 8,
  eventQueue,
  font: {
    render_text: jest.fn(),
  },
  draw_window: jest.fn(),
  get_context_palette: jest.fn(() => [[31, 31, 31]]),
  update: jest.fn(),
  default_frame_id: 1,
});

describe("display_unown_words", () => {
  it("draws the Unown word overlay and redraws the overworld after A/B clears it", async () => {
    const gameState = createInitialGameState();
    gameState.sram.options.frame = 1;
    const eventQueue = gameEngine.event.createQueue();
    const audioEngine = { playSound: jest.fn() };
    const ui = createUi(eventQueue);
    const overworld = {
      ui,
      audio_engine: audioEngine,
      draw: jest.fn(),
      input_capture_active: false,
    };

    let postedAck = false;
    const nextFrameMock = jest.spyOn(asyncLoop, "nextFrame").mockImplementation(async () => {
      if (!postedAck) {
        postedAck = true;
        gameEngine.event.post(
          new gameEngine.event.Event("keydown", { code: "KeyZ", key: "z" }),
          eventQueue,
        );
      }
    });
    const tickMock = jest
      .spyOn(gameEngine.time.Clock.prototype, "tick")
      .mockImplementation(() => undefined);

    const result = await display_unown_words(gameState, { overworld });

    expect(result).toBe("ESCAPE");
    expect(overworld.input_capture_active).toBe(false);
    expect(getUnownOverlayLockDepth(gameState)).toBe(0);
    expect(overworld.draw).toHaveBeenCalledTimes(2);
    expect(ui.update).toHaveBeenCalled();
    expect(ui.draw_window).toHaveBeenCalledTimes(1);
    expect(ui.draw_window).toHaveBeenCalledWith(
      ui.screen,
      expect.any(Number),
      expect.any(Number),
      expect.any(Number),
      expect.any(Number),
      expect.objectContaining({ frame_id: 2, frameId: 2 }),
    );
    expect(nextFrameMock).toHaveBeenCalled();
    expect(audioEngine.playSound).toHaveBeenCalledWith("SFX_READ_TEXT_2");
    expect(tickMock).not.toHaveBeenCalled();

    tickMock.mockRestore();
    nextFrameMock.mockRestore();
  });

  it("clears the Unown word overlay after a quick browser A tap", async () => {
    const gameState = createInitialGameState();
    const eventQueue = gameEngine.event.createQueue();
    const ui = createUi(eventQueue);
    const overworld = {
      ui,
      draw: jest.fn(),
      input_capture_active: false,
    };

    const nextFrameMock = jest.spyOn(asyncLoop, "nextFrame").mockImplementation(async () => {
      gameEngine.event.post(
        new gameEngine.event.Event(gameEngine.KEYDOWN, {
          key: "z",
          code: "KeyZ",
          is_press: true,
        }),
        eventQueue,
      );
      gameEngine.event.post(
        new gameEngine.event.Event(gameEngine.KEYUP, {
          key: "z",
          code: "KeyZ",
          is_press: false,
        }),
        eventQueue,
      );
    });
    const tickMock = jest
      .spyOn(gameEngine.time.Clock.prototype, "tick")
      .mockImplementation(() => undefined);

    try {
      await display_unown_words(gameState, { overworld });

      expect(overworld.draw).toHaveBeenCalledTimes(2);
      expect(ui.update).toHaveBeenCalledTimes(2);
      expect(overworld.input_capture_active).toBe(false);
      expect(getUnownOverlayLockDepth(gameState)).toBe(0);
      expect(tickMock).not.toHaveBeenCalled();
    } finally {
      tickMock.mockRestore();
      nextFrameMock.mockRestore();
    }
  });

  it("falls back to text events when UI has no input queue", async () => {
    const gameState = createInitialGameState();
    const ui = createUi(null);
    const overworld = {
      ui,
      draw: jest.fn(),
      input_capture_active: false,
    };
    const dispatch = jest.fn();
    const eventManager = { dispatch } as unknown as EventManager;
    const runner = {
      variables: {},
      pause: jest.fn(),
      last_value: null,
      last_condition_result: false,
    };

    const nextFrameMock = jest.spyOn(asyncLoop, "nextFrame");
    const result = await display_unown_words(gameState, {
      overworld,
      runner: runner as unknown as Parameters<typeof display_unown_words>[1]["runner"],
      event_manager: eventManager,
    });

    expect(result).toBe("ESCAPE");
    expect(runner.pause).toHaveBeenCalledTimes(1);
    const eventNames = dispatch.mock.calls.map((call) => call[0]?.name);
    expect(eventNames).toEqual(["show_text", "wait_for_input"]);
    expect(nextFrameMock).not.toHaveBeenCalled();
    expect(overworld.input_capture_active).toBe(false);
    expect(getUnownOverlayLockDepth(gameState)).toBe(0);

    nextFrameMock.mockRestore();
  });

  it("reacquires and releases lock across repeated word overlay runs", async () => {
    const gameState = createInitialGameState();
    const eventQueue = gameEngine.event.createQueue();
    const ui = createUi(eventQueue);
    const overworld = {
      ui,
      draw: jest.fn(),
      input_capture_active: false,
    };

    const nextFrameMock = jest.spyOn(asyncLoop, "nextFrame").mockImplementation(async () => {
      gameEngine.event.post(
        new gameEngine.event.Event("keydown", { button: "a", is_press: true }),
        eventQueue,
      );
    });
    const tickMock = jest
      .spyOn(gameEngine.time.Clock.prototype, "tick")
      .mockImplementation(() => undefined);

    await display_unown_words(gameState, { overworld });
    expect(getUnownOverlayLockDepth(gameState)).toBe(0);
    expect(overworld.input_capture_active).toBe(false);
    expect(overworld.draw).toHaveBeenCalledTimes(2);
    expect(tickMock).not.toHaveBeenCalled();

    await display_unown_words(gameState, { overworld });
    expect(getUnownOverlayLockDepth(gameState)).toBe(0);
    expect(overworld.input_capture_active).toBe(false);
    expect(overworld.draw).toHaveBeenCalledTimes(4);

    tickMock.mockRestore();
    nextFrameMock.mockRestore();
  });

});

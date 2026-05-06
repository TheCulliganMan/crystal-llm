import { set_day_of_week, type TimeSpecialOverworld, type TimeSpecialOverworldUI } from "./time";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import * as asyncLoop from "@pokecrystal/core/ui/async-loop";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { Surface } from "@pokecrystal/core/ui/surface";
import { EventManager } from "@pokecrystal/core/engine/events/events";

class TimeSpecialUIStub extends BaseUI implements TimeSpecialOverworldUI {
  public readonly eventQueue = gameEngine.event.createQueue();
  public readonly drawTextBox = jest.fn();
  public readonly drawWindow = jest.fn();
  public readonly draw_window = jest.fn();
  public readonly font = { renderText: jest.fn() };
  public readonly renderSnapshot = jest.fn();
  public readonly updateMock = jest.fn();

  constructor() {
    super(160, 144, 1);
  }

  protected createScreenSurface(): Surface {
    return new Surface(this.screenWidth, this.screenHeight);
  }

  public update(): void {
    this.updateMock();
  }
}

describe("set_day_of_week", () => {
  it("defaults immediately when UI exists but has no eventQueue", async () => {
    const ui = new (class extends BaseUI implements TimeSpecialOverworldUI {
      public readonly drawTextBox = jest.fn();
      public readonly drawWindow = jest.fn();
      public readonly draw_window = jest.fn();
      public readonly font = { renderText: jest.fn() };
      public readonly updateMock = jest.fn();

      constructor() {
        super(160, 144, 1);
      }

      protected createScreenSurface(): Surface {
        return new Surface(this.screenWidth, this.screenHeight);
      }

      public update(): void {
        this.updateMock();
      }
    })();

    // Explicitly simulate a "headless" UI missing an event queue.
    (ui as unknown as { eventQueue?: unknown }).eventQueue = null;

    const overworld: TimeSpecialOverworld = {
      ui,
      input_capture_active: false,
      update: jest.fn(),
      draw: jest.fn(),
      audio_engine: null,
    };

    const game_state = createInitialGameState();
    game_state.sram.day_of_week = 3;

    const nextFrameMock = jest.spyOn(asyncLoop, "nextFrame");

    const result = await set_day_of_week(game_state, {
      overworld,
      event_manager: { dispatch: jest.fn() } as unknown as EventManager,
      audio_engine: null,
      runner: undefined,
      rng: undefined,
    });

    expect(result).toBe(true);
    expect(game_state.sram.day_of_week).toBe(0);
    expect(overworld.input_capture_active).toBe(false);
    expect(nextFrameMock).not.toHaveBeenCalled();

    nextFrameMock.mockRestore();
  });

  it("runs the day-of-week screen asynchronously when UI is available", async () => {
    const events = [
      [new gameEngine.event.Event("keydown", { key: "ArrowUp" })],
      [new gameEngine.event.Event("keydown", { key: "ArrowUp" })],
      [new gameEngine.event.Event("keydown", { code: "KeyZ" })],
      [new gameEngine.event.Event("keyup", { code: "KeyZ" })],
      [new gameEngine.event.Event("keydown", { code: "KeyZ" })],
      [new gameEngine.event.Event("keydown", { code: "KeyZ" })],
    ];
    const getEvents = jest.spyOn(gameEngine.event, "get").mockImplementation(() => events.shift() ?? []);
    const nextFrameMock = jest.spyOn(asyncLoop, "nextFrame").mockResolvedValue(undefined);

    const ui = new TimeSpecialUIStub();
    const suspend = jest.fn();
    const resume = jest.fn();
    const forceCloseText = jest.fn();
    const overworld: TimeSpecialOverworld = {
      ui,
      input_capture_active: false,
      update: jest.fn(),
      draw: jest.fn(),
      audio_engine: null,
      dialogue: {
        suspend,
        resume,
        forceCloseText,
      },
    };

    const game_state = createInitialGameState();
    game_state.sram.day_of_week = 0;

    const result = await set_day_of_week(game_state, { overworld, event_manager: { dispatch: jest.fn() } as unknown as EventManager, audio_engine: null, runner: undefined, rng: undefined });

    expect(result).toBe(2);
    expect(game_state.sram.day_of_week).toBe(2);
    expect(overworld.input_capture_active).toBe(false);
    expect(getEvents).toHaveBeenCalled();
    expect(forceCloseText).toHaveBeenCalledTimes(1);
    expect(suspend).toHaveBeenCalledTimes(1);
    expect(resume).toHaveBeenCalledTimes(1);
    expect((overworld.draw as jest.Mock).mock.calls.length).toBeGreaterThanOrEqual(5);

    getEvents.mockRestore();
    nextFrameMock.mockRestore();
  });

  it("renders the initial day picker snapshot before consuming queued confirm input", async () => {
    const events = [
      [new gameEngine.event.Event("keydown", { code: "KeyZ", key: "z" })],
      [new gameEngine.event.Event("keyup", { code: "KeyZ", key: "z" })],
      [new gameEngine.event.Event("keydown", { code: "KeyZ", key: "z" })],
      [new gameEngine.event.Event("keydown", { code: "KeyZ", key: "z" })],
    ];
    const getEvents = jest.spyOn(gameEngine.event, "get").mockImplementation(() => events.shift() ?? []);
    const nextFrameMock = jest.spyOn(asyncLoop, "nextFrame").mockResolvedValue(undefined);

    const ui = new TimeSpecialUIStub();
    const overworld: TimeSpecialOverworld = {
      ui,
      input_capture_active: false,
      update: jest.fn(),
      draw: jest.fn(),
      audio_engine: null,
    };

    const game_state = createInitialGameState();
    const result = await set_day_of_week(game_state, {
      overworld,
      event_manager: { dispatch: jest.fn() } as unknown as EventManager,
      audio_engine: null,
      runner: undefined,
      rng: undefined,
    });

    expect(result).toBe(0);
    expect(ui.renderSnapshot).toHaveBeenCalledWith(
      ["Prompt"],
      ["Up/Down=Choose A=OK"],
      "Prompt",
      "Legend",
      expect.any(Array),
      null,
      ["What day is it?"]
    );

    getEvents.mockRestore();
    nextFrameMock.mockRestore();
  });
});

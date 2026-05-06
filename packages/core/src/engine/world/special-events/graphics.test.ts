import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { SpecialCommand } from "@pokecrystal/core/engine/world/story-events/commands/special";
import {
  createOverworldEngineStub,
  createScriptRunnerStub,
} from "@pokecrystal/core/engine/world/story-events/test-utils";

describe("special event fades", () => {
  it("queues fade frames for FadeOutToBlack using snake_case queue_delay", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const fadeToBlack = jest.fn();
    const queueDelay = jest.fn(() => true);
    const queueTask: (scheduler: (callback: () => void) => boolean | void) => void = jest.fn(
      (schedule: (callback: () => void) => boolean | void) => {
        schedule(jest.fn());
      },
    );
    const overworld = createOverworldEngineStub({
      fade_to_black: fadeToBlack,
      queue_delay: queueDelay,
    });
    const runner = createScriptRunnerStub({
      overworld,
      _queue_overworld_task: queueTask,
    });
    const command = new SpecialCommand("FadeOutToBlack");
    command.runner = runner;

    command.execute(gameState, eventManager, overworld);

    expect(fadeToBlack).toHaveBeenCalledWith(8);
    expect(queueTask).toHaveBeenCalledTimes(1);
    expect(queueDelay).toHaveBeenCalledWith(8, {
      on_complete: expect.any(Function),
      blocking: true,
    });
  });

  it("binds overworld when invoking FadeOutToBlack", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = createOverworldEngineStub({
      fade_to_black: function fade_to_black(this: { bound?: boolean }, frames: number): void {
        this.bound = true;
        expect(frames).toBe(8);
      },
    });
    const runner = createScriptRunnerStub({ overworld });
    const command = new SpecialCommand("FadeOutToBlack");
    command.runner = runner;

    expect(() => command.execute(gameState, eventManager, overworld)).not.toThrow();
    expect(overworld.bound).toBe(true);
  });

  it("queues fade frames for FadeInFromWhite using camelCase queueDelay", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const fadeFromWhite = jest.fn();
    const queueDelay = jest.fn(() => true);
    const queueTask: (scheduler: (callback: () => void) => boolean | void) => void = jest.fn(
      (schedule: (callback: () => void) => boolean | void) => {
        schedule(jest.fn());
      },
    );
    const overworld = createOverworldEngineStub({
      fade_from_white: fadeFromWhite,
      queueDelay,
    });
    const runner = createScriptRunnerStub({
      overworld,
      _queueOverworldTask: queueTask,
    });
    const command = new SpecialCommand("FadeInFromWhite");
    command.runner = runner;

    command.execute(gameState, eventManager, overworld);

    expect(fadeFromWhite).toHaveBeenCalledWith(8);
    expect(queueTask).toHaveBeenCalledTimes(1);
    expect(queueDelay).toHaveBeenCalledWith(8, {
      onComplete: expect.any(Function),
      blocking: true,
    });
  });

  it("binds runner when queueing fade frames", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const fadeToBlack = jest.fn();
    const queueDelay = jest.fn(() => true);
    const overworld = createOverworldEngineStub({
      fade_to_black: fadeToBlack,
      queue_delay: queueDelay,
    });
    const runner = createScriptRunnerStub({
      overworld,
      _script_stack: [],
      _queue_overworld_task: function _queue_overworld_task(
        this: { _script_stack: Array<{ label: string }> },
        schedule: (callback: () => void) => boolean | void,
      ): void {
        this._script_stack.push({ label: "snapshot" });
        schedule(jest.fn());
      },
    });
    const command = new SpecialCommand("FadeOutToBlack");
    command.runner = runner;

    expect(() => command.execute(gameState, eventManager, overworld)).not.toThrow();
    expect(runner._script_stack).toHaveLength(1);
  });

  it("binds overworld context when queueing fade frames with camelCase queueDelay", () => {
    const gameState = createInitialGameState();
    const queueDelay = jest.fn(function (this: { bound?: unknown }, _frames: number) {
      this.bound = this;
      return true;
    });
    const overworld = createOverworldEngineStub({
      fade_from_white: jest.fn(),
      queueDelay,
    });
    const runner = createScriptRunnerStub({
      overworld,
      _queueOverworldTask: (scheduler) => scheduler(jest.fn()),
    });

    expect(() => {
      // Uses queueFadeFrames internally.
      const command = new SpecialCommand("FadeInFromWhite");
      command.runner = runner;
      command.execute(gameState, new EventManager(gameState), overworld);
    }).not.toThrow();

    expect(queueDelay).toHaveBeenCalled();
    expect((overworld as { bound?: unknown }).bound).toBe(overworld);
  });

  it("binds overworld context when queueing fade frames with snake_case queue_delay", () => {
    const gameState = createInitialGameState();
    const queue_delay = jest.fn(function (this: { bound?: unknown }, _frames: number) {
      this.bound = this;
      return true;
    });
    const overworld = createOverworldEngineStub({
      fade_to_white: jest.fn(),
      queue_delay,
    });
    const runner = createScriptRunnerStub({
      overworld,
      _queue_overworld_task: (scheduler) => scheduler(jest.fn()),
    });

    expect(() => {
      const command = new SpecialCommand("FadeOutToWhite");
      command.runner = runner;
      command.execute(gameState, new EventManager(gameState), overworld);
    }).not.toThrow();

    expect(queue_delay).toHaveBeenCalled();
    expect((overworld as { bound?: unknown }).bound).toBe(overworld);
  });
});

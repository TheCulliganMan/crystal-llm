import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { SpecialCommand } from "./special";
import { SPECIAL_FUNCTIONS } from "@pokecrystal/core/engine/world/special-events/registry";
import {
  createOverworldStub,
  createOverworldEngineStub,
  createScriptRunnerStub,
  createTestPokemon,
} from "@pokecrystal/core/engine/world/story-events/test-utils";
import type { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";

describe("SpecialCommand", () => {
  it("strips asm comments from special names", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const runner = createScriptRunnerStub({ variables: {} });
    gameState.sram.party.pokemon[0] = createTestPokemon("ABRA", 63);
    const command = new SpecialCommand("GameboyCheck ; predef comment");
    command.runner = runner;

    expect(() =>
      command.execute(gameState, eventManager, createOverworldEngineStub()),
    ).not.toThrow();
    expect(runner.last_value).toBe("GBCHECK_CGB");
  });

  it("passes runner context to specials with defaulted option parameters", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const runner = createScriptRunnerStub({ variables: {} });

    const specialImpl = (
      _state: typeof gameState,
      { runner: ctxRunner }: { runner?: typeof runner } = {}
    ) => {
      if (!ctxRunner) {
        throw new Error("Missing runner context.");
      }
      ctxRunner.last_value = "OK";
      return true;
    };

    SPECIAL_FUNCTIONS.TestDefaultedContext = specialImpl as unknown as (typeof SPECIAL_FUNCTIONS)[string];
    const command = new SpecialCommand("TestDefaultedContext");
    command.runner = runner;

    expect(() =>
      command.execute(gameState, eventManager, createOverworldEngineStub()),
    ).not.toThrow();
    expect(runner.last_value).toBe("OK");

    delete SPECIAL_FUNCTIONS.TestDefaultedContext;
  });

  it("does not treat two-arg specials as game_state-object specials", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const runner = createScriptRunnerStub({ variables: {} });

    const specialImpl = (
      _state: typeof gameState,
      { runner: ctxRunner }: { runner?: typeof runner } = {}
    ) => {
      if (!ctxRunner) {
        throw new Error("Missing runner context.");
      }
      return true;
    };

    SPECIAL_FUNCTIONS.TestTwoArgSpecial = specialImpl as unknown as (typeof SPECIAL_FUNCTIONS)[string];
    const command = new SpecialCommand("TestTwoArgSpecial");
    command.runner = runner;

    expect(() =>
      command.execute(gameState, eventManager, createOverworldEngineStub()),
    ).not.toThrow();

    delete SPECIAL_FUNCTIONS.TestTwoArgSpecial;
  });

  it("falls back to runner overworld when overworld argument is missing", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const runner = createScriptRunnerStub({
      variables: {},
      overworld: createOverworldEngineStub({ ui: { label: "ui" } }),
    });

    const specialImpl = (
      _state: typeof gameState,
      { overworld }: { overworld?: typeof runner.overworld } = {}
    ) => {
      if (!overworld) {
        throw new Error("Missing overworld context.");
      }
      overworld.ui.label = "used";
      return true;
    };

    SPECIAL_FUNCTIONS.TestOverworldFallback = specialImpl as unknown as (typeof SPECIAL_FUNCTIONS)[string];
    const command = new SpecialCommand("TestOverworldFallback");
    command.runner = runner;

    expect(() =>
      command.execute(gameState, eventManager, null as unknown as OverworldMap),
    ).not.toThrow();
    expect(runner.overworld.ui.label).toBe("used");

    delete SPECIAL_FUNCTIONS.TestOverworldFallback;
  });

  it("prefers runner overworld over provided overworld", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const runner = createScriptRunnerStub({
      variables: {},
      overworld: createOverworldEngineStub({ ui: { label: "runner" } }),
    });
    const overworld = createOverworldEngineStub<{ ui: { label: string } }>({ ui: { label: "arg" } });

    const specialImpl = (
      _state: typeof gameState,
      { overworld }: { overworld?: { ui?: { label?: string } } } = {}
    ) => {
      if (!overworld?.ui) {
        throw new Error("Missing overworld UI.");
      }
      overworld.ui.label = "used";
      return true;
    };

    SPECIAL_FUNCTIONS.TestOverworldUiPreference = specialImpl as unknown as (typeof SPECIAL_FUNCTIONS)[string];
    const command = new SpecialCommand("TestOverworldUiPreference");
    command.runner = runner;

    expect(() =>
      command.execute(gameState, eventManager, overworld),
    ).not.toThrow();
    expect(runner.overworld.ui.label).toBe("used");
    expect(overworld.ui.label).toBe("arg");

    delete SPECIAL_FUNCTIONS.TestOverworldUiPreference;
  });

  it("pauses and resumes when a special returns a promise", async () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const pauseMock = jest.fn();
    const resumeMock = jest.fn();
    const runner = createScriptRunnerStub({
      variables: {},
      last_value: null,
      last_condition_result: false,
      pause: pauseMock,
      resume: resumeMock,
    });
    const resolved = { async: true };
    SPECIAL_FUNCTIONS.TestAsyncSpecial = () => Promise.resolve(resolved);
    const command = new SpecialCommand("TestAsyncSpecial");
    command.runner = runner;

    command.execute(gameState, eventManager, createOverworldStub());

    expect(runner.pause).toHaveBeenCalledTimes(1);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(runner.resume).toHaveBeenCalledTimes(1);
    expect(runner.last_value).toEqual(resolved);

    delete SPECIAL_FUNCTIONS.TestAsyncSpecial;
  });

  it("does not block command execution for unresolved async specials", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const pauseMock = jest.fn();
    const resumeMock = jest.fn();
    const runner = createScriptRunnerStub({
      variables: {},
      last_value: null,
      last_condition_result: false,
      pause: pauseMock,
      resume: resumeMock,
    });
    SPECIAL_FUNCTIONS.TestNeverResolvingSpecial = () => new Promise(() => {});
    const command = new SpecialCommand("TestNeverResolvingSpecial");
    command.runner = runner;

    expect(() =>
      command.execute(gameState, eventManager, createOverworldStub()),
    ).not.toThrow();
    expect(runner.pause).toHaveBeenCalledTimes(1);
    expect(runner.resume).not.toHaveBeenCalled();

    delete SPECIAL_FUNCTIONS.TestNeverResolvingSpecial;
  });

  it("pauses and resumes when HealMachineAnim uses async pokemon center playback", async () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const pauseMock = jest.fn();
    const resumeMock = jest.fn();
    const playHealMachineAnimationAsync = jest.fn(() => Promise.resolve());
    const runner = createScriptRunnerStub({
      variables: { _value: 2 },
      last_condition_result: false,
      pause: pauseMock,
      resume: resumeMock,
      pokemon_center: {
        playHealMachineAnimationAsync,
      },
    });
    const command = new SpecialCommand("HealMachineAnim");
    command.runner = runner;

    command.execute(gameState, eventManager, createOverworldStub());

    expect(playHealMachineAnimationAsync).toHaveBeenCalledTimes(1);
    expect(playHealMachineAnimationAsync).toHaveBeenCalledWith("2", expect.anything());
    expect(runner.last_condition_result).toBe(false);
    expect(runner.pause).toHaveBeenCalledTimes(1);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(runner.resume).toHaveBeenCalledTimes(1);
    expect(runner.last_condition_result).toBe(true);
  });
});

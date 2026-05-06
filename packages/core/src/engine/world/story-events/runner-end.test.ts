import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { ScriptRunnerImpl, ScriptRunnerState } from "./runner";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";

describe("ScriptRunner end handling", () => {
  it("allows fallthrough after closetext when the script has a successor", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const calls: string[] = [];
    const dataLoader = new DataLoader();
    dataLoader.get_script = function (name: string) {
      calls.push(name);
      if (name === "First") {
        return [{ command: "closetext", args: [] }];
      }
      if (name === "Second") {
        return [{ command: "end", args: [] }];
      }
      return null;
    };
    dataLoader.get_script_successor = (_scriptName: string, _parentScript?: string | null) => ["", "Second"];
    const overworld = {} as OverworldEngine;

    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    runner.run("First");

    expect(calls).toEqual(["First", "Second"]);
    expect(runner.state).toBe(ScriptRunnerState.IDLE);
  });

  it("prevents fallthrough after end", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const calls: string[] = [];
    const dataLoader = new DataLoader();
    dataLoader.get_script = function (name: string) {
      calls.push(name);
      if (name === "First") {
        return [{ command: "end", args: [] }];
      }
      if (name === "Second") {
        return [{ command: "end", args: [] }];
      }
      return null;
    };
    dataLoader.get_script_successor = (_scriptName: string, _parentScript?: string | null) => ["", "Second"];
    const overworld = {} as OverworldEngine;

    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    runner.run("First");

    expect(calls).toContain("First");
    expect(calls).not.toContain("Second");
    expect(runner.state).toBe(ScriptRunnerState.IDLE);
  });

  it("prevents fallthrough after itemball", () => {
    jest.useFakeTimers();
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const calls: string[] = [];
    const dataLoader = new DataLoader();
    dataLoader.get_script = function (name: string) {
      calls.push(name);
      if (name === "First") {
        return [{ command: "itemball", args: ["ANTIDOTE"] }];
      }
      if (name === "Second") {
        return [{ command: "end", args: [] }];
      }
      return null;
    };
    dataLoader.get_script_successor = (_scriptName: string, _parentScript?: string | null) => ["", "Second"];
    const overworld = {} as OverworldEngine;

    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    runner.run("First");
    jest.runOnlyPendingTimers();

    expect(calls).toContain("First");
    expect(calls).not.toContain("Second");
    jest.useRealTimers();
  });

  it("prevents fallthrough after hiddenitem", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const calls: string[] = [];
    const dataLoader = new DataLoader();
    dataLoader.get_script = function (name: string) {
      calls.push(name);
      if (name === "First") {
        return [{ command: "hiddenitem", args: ["POTION", "EVENT_TEST_HIDDEN_ITEM"] }];
      }
      if (name === "Second") {
        return [{ command: "end", args: [] }];
      }
      return null;
    };
    dataLoader.get_script_successor = (_scriptName: string, _parentScript?: string | null) => ["", "Second"];
    const overworld = {} as OverworldEngine;

    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    runner.run("First");
    runner.resume();

    expect(calls).toContain("First");
    expect(calls).not.toContain("Second");
  });

  it("restores temporary overworld music when the script runner goes idle", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    dataLoader.get_script = function (name: string) {
      if (name === "First") {
        return [{ command: "end", args: [] }];
      }
      return null;
    };
    const restartMapMusic = jest.fn();
    const overworld = {
      hasTemporaryMusicOverride: jest.fn(() => true),
      restartMapMusic,
    } as unknown as OverworldEngine;

    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    runner.run("First");

    expect(restartMapMusic).toHaveBeenCalledTimes(1);
    expect(runner.state).toBe(ScriptRunnerState.IDLE);
  });

  it("does not restore map music when no temporary override is active", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    dataLoader.get_script = function (name: string) {
      if (name === "First") {
        return [{ command: "end", args: [] }];
      }
      return null;
    };
    const restartMapMusic = jest.fn();
    const overworld = {
      hasTemporaryMusicOverride: jest.fn(() => false),
      restartMapMusic,
    } as unknown as OverworldEngine;

    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    runner.run("First");

    expect(restartMapMusic).not.toHaveBeenCalled();
    expect(runner.state).toBe(ScriptRunnerState.IDLE);
  });
});

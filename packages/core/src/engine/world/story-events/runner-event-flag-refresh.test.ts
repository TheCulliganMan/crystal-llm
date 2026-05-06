import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { ScriptRunnerImpl, ScriptRunnerState } from "./runner";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";

describe("ScriptRunner event flag refresh suppression", () => {
  it("restores allow_event_flag_refresh after a paused script completes", () => {
    jest.useFakeTimers();
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    dataLoader.get_script = (name: string) => {
      if (name === "TestScript") {
        return [{ command: "pause", args: ["1"] }];
      }
      return null;
    };
    dataLoader.get_text = () => null;
    const overworld = {} as OverworldEngine;

    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    runner.allow_event_flag_refresh = true;

    runner.run("TestScript");

    expect(runner.allow_event_flag_refresh).toBe(false);
    expect(runner.state).toBe(ScriptRunnerState.PAUSED);

    jest.runOnlyPendingTimers();

    expect(runner.state).toBe(ScriptRunnerState.IDLE);
    expect(runner.allow_event_flag_refresh).toBe(true);

    jest.useRealTimers();
  });
});

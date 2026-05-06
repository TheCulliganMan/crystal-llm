import { createInitialGameState } from "@pokecrystal/core/core/state";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import type { OverworldContext } from "./commands/base";
import { YesOrNoCommand } from "./commands/text";
import { ScriptRunnerImpl } from "./runner";
import { Event } from "@pokecrystal/core/engine/events/events";

const createDataLoaderStub = (): DataLoader => {
  const loader = new DataLoader();
  loader.get_script = () => null;
  loader.get_text = () => null;
  return loader;
};

const createRunner = (): ScriptRunnerImpl => {
  const gameState = createInitialGameState();
  const eventManager = new EventManager(gameState);
  const dataLoader = createDataLoaderStub();
  const overworld = {} as OverworldEngine;
  return new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
};

describe("yesorno command", () => {
  it("parses yesorno into a command instance", () => {
    const runner = createRunner();
    const commands = runner.parse([{ command: "yesorno", args: [] }]);
    expect(commands).toHaveLength(1);
    expect(commands[0]).toBeInstanceOf(YesOrNoCommand);
  });

  it("uses _consume_script_choice override when available", () => {
    const runner = createRunner();
    runner._consume_script_choice = (key: string) => {
      if (key === "_yesorno_choice") {
        return true;
      }
      return null;
    };

    const command = new YesOrNoCommand();
    command.runner = runner;
    command.execute(runner.gameState, runner.eventManager, {} as unknown as OverworldContext);

    expect(runner.last_yes_no_result).toBe(true);
    expect(runner.last_condition_result).toBe(true);
  });

  it("opens text, waits, and dispatches prompt_yes_no with callbacks", () => {
    const runner = createRunner();
    const command = new YesOrNoCommand();
    const dialogue = { active: false };
    const callback = jest.fn();
    command.runner = runner;
    command.on_result = callback;

    const dispatchSpy = jest.spyOn(runner.eventManager, "dispatch");
    const pauseSpy = jest.spyOn(runner, "pause");
    command.execute(runner.gameState, runner.eventManager, { dialogue } as unknown as OverworldContext);

    const names = dispatchSpy.mock.calls.map((call) => (call[0] as Event).name);
    expect(names).toEqual(["open_text", "wait_for_input", "prompt_yes_no"]);
    const promptEvent = dispatchSpy.mock.calls[2][0] as Event;
    expect(promptEvent.data.callback).toBe(callback);
    expect(pauseSpy).toHaveBeenCalled();
  });

  it("skips open_text when dialogue is already active", () => {
    const runner = createRunner();
    const command = new YesOrNoCommand();
    const dialogue = { active: true };
    command.runner = runner;

    const dispatchSpy = jest.spyOn(runner.eventManager, "dispatch");
    command.execute(runner.gameState, runner.eventManager, { dialogue } as unknown as OverworldContext);

    const names = dispatchSpy.mock.calls.map((call) => (call[0] as Event).name);
    expect(names).toEqual(["wait_for_input", "prompt_yes_no"]);
  });
});

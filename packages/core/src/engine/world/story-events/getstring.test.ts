import { DataLoader, type ScriptData } from "@pokecrystal/core/core/data-loader";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { GetStringCommand } from "./commands/text";
import { ScriptRunnerImpl } from "./runner";

const createRunner = (
  scripts: Record<string, ScriptData>,
  parentScript?: string,
): ScriptRunnerImpl => {
  const gameState = createInitialGameState();
  const eventManager = new EventManager(gameState);
  const dataLoader = new DataLoader();
  dataLoader.get_script = (name: string, parent?: string) => {
    if (parentScript && parent !== parentScript) {
      return null;
    }
    return scripts[name] ?? null;
  };
  dataLoader.get_text = () => null;
  const overworld = {} as OverworldEngine;
  const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
  if (parentScript) {
    (runner as unknown as { _find_parent_script_name?: () => string | null })._find_parent_script_name = () => parentScript;
  }
  return runner;
};

describe("getstring command", () => {
  it("parses getstring and loads db strings into buffers", () => {
    const scripts: Record<string, ScriptData> = {
      PokegearName: [{ command: "db", args: ["\"#GEAR@\""] }],
    };
    const runner = createRunner(scripts);
    const commands = runner.parse([
      { command: "getstring", args: ["STRING_BUFFER_4", "PokegearName"] },
    ]);

    expect(commands).toHaveLength(1);
    expect(commands[0]).toBeInstanceOf(GetStringCommand);

    const command = commands[0];
    command.runner = runner;
    command.execute(runner.game_state, runner.event_manager, runner.overworld);

    expect(runner.string_buffers.STRING_BUFFER_4).toBe("#GEAR");
  });

  it("resolves local labels using the parent script name", () => {
    const scripts: Record<string, ScriptData> = {
      ".mapcardname": [{ command: "db", args: ["\"MAP CARD@\""] }],
    };
    const runner = createRunner(scripts, "CherrygroveCityGuideGent");
    const command = new GetStringCommand("STRING_BUFFER_4", ".mapcardname");
    command.runner = runner;
    command.execute(runner.game_state, runner.event_manager, runner.overworld);

    expect(runner.string_buffers.STRING_BUFFER_4).toBe("MAP CARD");
  });
});

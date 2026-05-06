import { createInitialGameState } from "@pokecrystal/core/core/state";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { RefreshMapCommand } from "./commands/overworld";
import { ScriptRunnerImpl } from "./runner";

const createRunner = (): ScriptRunnerImpl => {
  const gameState = createInitialGameState();
  const eventManager = new EventManager(gameState);
  const dataLoader = new DataLoader();
  const overworld = {} as OverworldEngine;
  return new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
};

describe("ScriptRunner refreshmap parsing", () => {
  it("parses refreshmap into RefreshMapCommand", () => {
    const runner = createRunner();
    const commands = runner.parse([{ command: "refreshmap", args: [] }]);
    expect(commands).toHaveLength(1);
    expect(commands[0]).toBeInstanceOf(RefreshMapCommand);
  });
});

import { createInitialGameState, type GameState } from "@pokecrystal/core/core/state";
import { createScriptRunnerStub } from "@pokecrystal/core/engine/world/story-events/test-utils";
import { FruitTreeCommand } from "./fruit-tree";
import { Command, type ScriptFrame } from "./base";
import type { EventManagerLike, OverworldContext } from "./base";

class MarkerCommand extends Command {
  public called = false;

  public execute(_gameState: GameState, _eventManager: EventManagerLike, _overworld: OverworldContext): void {
    this.called = true;
  }
}

describe("FruitTreeCommand", () => {
  it("marks fruit trees collected by their FRUITTREE_* constant", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState });
    const command = new FruitTreeCommand("FRUITTREE_ROUTE_29");
    command.runner = runner;

    command.execute(gameState, runner.event_manager, runner.overworld);

    expect(gameState.wram.event_flags["FRUITTREE_ROUTE_29_COLLECTED"]).toBe(true);
    expect(gameState.sram.items.BERRY).toBe(1);

    command.execute(gameState, runner.event_manager, runner.overworld);

    expect(gameState.sram.items.BERRY).toBe(1);
  });

  it("terminates the parent script when the tree is already collected", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState });
    const skippedMarker = new MarkerCommand();
    const parentFrame: ScriptFrame = {
      name: "ParentScript",
      commands: [new MarkerCommand(), skippedMarker],
      index: 1,
      allowFallthrough: true,
    };
    runner._script_stack = [parentFrame];
    gameState.wram.event_flags["FRUITTREE_ROUTE_29_COLLECTED"] = true;

    const command = new FruitTreeCommand("FRUITTREE_ROUTE_29");
    command.runner = runner;
    command.execute(gameState, runner.event_manager, runner.overworld);

    const childFrame = runner._script_stack[runner._script_stack.length - 1];
    for (const cmd of childFrame.commands) {
      cmd.runner = runner;
      cmd.execute(gameState, runner.event_manager, runner.overworld);
    }

    expect(parentFrame.index).toBe(parentFrame.commands.length);
    expect(parentFrame.allowFallthrough).toBe(false);
    expect(skippedMarker.called).toBe(false);
  });
});

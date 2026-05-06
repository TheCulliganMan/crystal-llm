import { createInitialGameState } from "@pokecrystal/core/core/state";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { FacePlayerCommand } from "./commands/overworld";
import {
  CloseTextCommand,
  OpenTextCommand,
  TradeCommand,
  WaitButtonCommand,
  WriteTextCommand,
} from "./commands/text";
import { ScriptRunnerImpl } from "./runner";

const createRunner = (): ScriptRunnerImpl => {
  const gameState = createInitialGameState();
  const eventManager = new EventManager(gameState);
  const dataLoader = new DataLoader();
  const overworld = {} as OverworldEngine;
  return new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
};

describe("ScriptRunner jumptext parsing", () => {
  it("expands jumptext into open/write/wait/close commands", () => {
    const runner = createRunner();

    const commands = runner.parse([{ command: "jumptext", args: ["TVText"] }]);

    expect(commands).toHaveLength(4);
    expect(commands[0]).toBeInstanceOf(OpenTextCommand);
    const writeCommand = commands[1] as WriteTextCommand;
    expect(writeCommand).toBeInstanceOf(WriteTextCommand);
    expect(writeCommand.textLabel).toBe("TVText");
    expect(commands[2]).toBeInstanceOf(WaitButtonCommand);
    expect(commands[3]).toBeInstanceOf(CloseTextCommand);
  });

  it("expands jumptextfaceplayer into faceplayer + jumptext sequence", () => {
    const runner = createRunner();

    const commands = runner.parse([{ command: "jumptextfaceplayer", args: ["GreetingText"] }]);

    expect(commands).toHaveLength(5);
    expect(commands[0]).toBeInstanceOf(FacePlayerCommand);
    expect(commands[1]).toBeInstanceOf(OpenTextCommand);
    const faceWriteCommand = commands[2] as WriteTextCommand;
    expect(faceWriteCommand).toBeInstanceOf(WriteTextCommand);
    expect(faceWriteCommand.textLabel).toBe("GreetingText");
    expect(commands[3]).toBeInstanceOf(WaitButtonCommand);
    expect(commands[4]).toBeInstanceOf(CloseTextCommand);
  });

  it("expands farjumptext into open/write/wait/close commands", () => {
    const runner = createRunner();

    const commands = runner.parse([{ command: "farjumptext", args: ["FarText"] }]);

    expect(commands).toHaveLength(4);
    expect(commands[0]).toBeInstanceOf(OpenTextCommand);
    const farWriteCommand = commands[1] as WriteTextCommand;
    expect(farWriteCommand).toBeInstanceOf(WriteTextCommand);
    expect(farWriteCommand.textLabel).toBe("FarText");
    expect(commands[2]).toBeInstanceOf(WaitButtonCommand);
    expect(commands[3]).toBeInstanceOf(CloseTextCommand);
  });

  it("parses NPC trade commands", () => {
    const runner = createRunner();

    const commands = runner.parse([{ command: "trade", args: ["NPC_TRADE_TIM"] }]);

    expect(commands).toHaveLength(1);
    expect(commands[0]).toBeInstanceOf(TradeCommand);
  });
});

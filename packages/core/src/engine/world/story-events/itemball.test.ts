import { createInitialGameState } from "@pokecrystal/core/core/state";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { ScriptRunnerImpl } from "./runner";
import { ItemBallCommand } from "./commands/items";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";

type RunnerSetup = {
  runner: ScriptRunnerImpl;
  gameState: ReturnType<typeof createInitialGameState>;
  eventManager: EventManager;
};

const createRunner = (textMap: Record<string, string>): RunnerSetup => {
  const gameState = createInitialGameState();
  const eventManager = new EventManager(gameState);
  const dataLoader = new DataLoader();
  dataLoader.get_text = (label: string) => textMap[label] ?? "";
  dataLoader.get_script = () => null;
  const overworld = {} as OverworldEngine;
  const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
  return { runner, gameState, eventManager };
};

describe("ItemBallCommand", () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.runOnlyPendingTimers();
    jest.useRealTimers();
  });

  it("shows found-item text using STRING_BUFFER_3", () => {
    const { runner, gameState, eventManager } = createRunner({});
    gameState.sram.player_name = "KRIS";
    const showHandler = jest.fn();
    eventManager.on("show_text", showHandler);

    const command = new ItemBallCommand("ANTIDOTE");
    command.runner = runner;
    command.execute(gameState, eventManager, runner.overworld);
    jest.runOnlyPendingTimers();

    const event = showHandler.mock.calls[0]?.[0] as { data?: { text?: string } } | undefined;
    expect(event?.data?.text).toBe("KRIS found\nANTIDOTE!");
    expect(runner.last_sound_effect).toBe("SFX_ITEM");
  });

  it("shows the carry-capacity message when the bag is full", () => {
    const { runner, gameState, eventManager } = createRunner({
      CantCarryItemText: "But <PLAYER> can't carry any more items.",
    });
    gameState.sram.player_name = "KRIS";
    runner.variables = { VAR_CALLERID: "PHONE_SCHOOLBOY_JACK" };
    const showHandler = jest.fn();
    eventManager.on("show_text", showHandler);
    jest.spyOn(runner.itemSystem, "addItem").mockReturnValue(false);

    const command = new ItemBallCommand("ANTIDOTE");
    command.runner = runner;
    command.execute(gameState, eventManager, runner.overworld);

    const first = showHandler.mock.calls[0]?.[0] as { data?: { text?: string } } | undefined;
    const second = showHandler.mock.calls[1]?.[0] as { data?: { text?: string } } | undefined;
    expect(first?.data?.text).toBe("KRIS found\nANTIDOTE!");
    expect(second?.data?.text).toBe("But KRIS can't carry any more items.");
  });
});

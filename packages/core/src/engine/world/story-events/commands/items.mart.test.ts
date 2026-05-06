import { createInitialGameState } from "@pokecrystal/core/core/state";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { ItemSystem } from "@pokecrystal/core/engine/systems/items";
import { Surface } from "@pokecrystal/core/ui/surface";
import { MartInterface } from "@pokecrystal/core/ui/menus/mart";
import { PokemartCommand } from "./items";
import * as asyncLoop from "@pokecrystal/core/ui/async-loop";
import { gameEngine, type GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";

const makeUiStub = () => ({
  screen: new Surface(160, 144),
  screenHeight: 144,
  drawWindow: jest.fn(),
  font: { renderText: jest.fn() },
  update: jest.fn(),
  tileSize: 8,
});

describe("PokemartCommand", () => {
  it("queues async mart flow when the runner schedules tasks", async () => {
    const gameState = createInitialGameState();
    const dataLoader = new DataLoader();
    const itemSystem = new ItemSystem(gameState, dataLoader);
    const ui = makeUiStub();
    const overworld: Record<string, unknown> = {
      ui,
      dataLoader,
      pollEvents: jest.fn(),
      draw: jest.fn(),
      item_system: itemSystem,
    };
    const martInterface = new MartInterface(overworld, gameState, dataLoader, itemSystem);
    const openSpy = jest.spyOn(martInterface, "open");
    const openAsyncSpy = jest.spyOn(martInterface, "openAsync").mockResolvedValue(undefined);
    overworld._mart_interface = martInterface;

    const queueTask = jest.fn();
    const runner = { _queue_overworld_task: queueTask } as any;
    const command = new PokemartCommand("MARTTYPE_STANDARD", "TEST_MART");
    command.runner = runner;

    command.execute(gameState, {} as any, overworld as any);

    expect(queueTask).toHaveBeenCalledTimes(1);
    const scheduler = queueTask.mock.calls[0][0] as (callback: () => void) => boolean;
    const done = jest.fn();
    scheduler(done);
    await Promise.resolve();
    await Promise.resolve();

    expect(openAsyncSpy).toHaveBeenCalledWith("MARTTYPE_STANDARD", "TEST_MART");
    expect(openSpy).not.toHaveBeenCalled();
    expect(done).toHaveBeenCalledTimes(1);
  });

  it("falls back to async flow with pause/resume when no scheduler exists", async () => {
    const gameState = createInitialGameState();
    const dataLoader = new DataLoader();
    const itemSystem = new ItemSystem(gameState, dataLoader);
    const ui = makeUiStub();
    const overworld: Record<string, unknown> = {
      ui,
      dataLoader,
      pollEvents: jest.fn(),
      draw: jest.fn(),
      item_system: itemSystem,
    };
    const martInterface = new MartInterface(overworld, gameState, dataLoader, itemSystem);
    const openSpy = jest.spyOn(martInterface, "open");
    const openAsyncSpy = jest.spyOn(martInterface, "openAsync").mockResolvedValue(undefined);
    overworld._mart_interface = martInterface;

    const pause = jest.fn();
    const resume = jest.fn();
    const runner = { pause, resume } as any;
    const command = new PokemartCommand("MARTTYPE_STANDARD", "TEST_MART");
    command.runner = runner;

    command.execute(gameState, {} as any, overworld as any);

    expect(pause).toHaveBeenCalledTimes(1);
    await Promise.resolve();
    await Promise.resolve();

    expect(openAsyncSpy).toHaveBeenCalledWith("MARTTYPE_STANDARD", "TEST_MART");
    expect(openSpy).not.toHaveBeenCalled();
    expect(resume).toHaveBeenCalledTimes(1);
  });
});

describe("MartInterface.openAsync", () => {
  it("yields frames while waiting for input", async () => {
    const gameState = createInitialGameState();
    const dataLoader = new DataLoader();
    dataLoader.martData = new Map();
    const itemSystem = new ItemSystem(gameState, dataLoader);
    const ui = makeUiStub();
    const eventBatches = [
      [],
      [{ type: "keydown", code: "KeyZ" }],
    ];
    let pollCount = 0;
    const overworld: Record<string, unknown> = {
      ui,
      pollEvents: () => eventBatches[pollCount++] ?? [],
      draw: jest.fn(),
    };
    const martInterface = new MartInterface(overworld, gameState, dataLoader, itemSystem);
    const nextFrameMock = jest.spyOn(asyncLoop, "nextFrame").mockResolvedValue(undefined);

    await martInterface.openAsync("MARTTYPE_STANDARD", "EMPTY_MART");

    expect(nextFrameMock).toHaveBeenCalled();
    nextFrameMock.mockRestore();
  });

  it("uses the UI event queue and restores input capture", async () => {
    const gameState = createInitialGameState();
    const dataLoader = new DataLoader();
    dataLoader.martData = new Map();
    const itemSystem = new ItemSystem(gameState, dataLoader);
    const ui = makeUiStub() as ReturnType<typeof makeUiStub> & { eventQueue?: GameEngineEventQueue };
    ui.eventQueue = gameEngine.event.createQueue();
    const overworld = {
      ui,
      draw: jest.fn(),
      input_capture_active: false,
    };
    const martInterface = new MartInterface(overworld, gameState, dataLoader, itemSystem);

    const promise = martInterface.openAsync("MARTTYPE_STANDARD", "EMPTY_MART");

    expect(overworld.input_capture_active).toBe(true);
    gameEngine.event.post({ type: "keydown", code: "KeyZ" }, ui.eventQueue);
    await promise;
    expect(overworld.input_capture_active).toBe(false);
  });
});

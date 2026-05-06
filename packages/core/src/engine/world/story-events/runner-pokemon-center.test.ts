import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { ScriptRunnerImpl, ScriptRunnerState } from "@pokecrystal/core/engine/world/story-events/runner";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import * as asyncLoop from "@pokecrystal/core/ui/async-loop";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { FieldDialogueManager } from "@pokecrystal/core/ui/text/dialogue";
import { Surface } from "@pokecrystal/core/ui/surface";

describe("ScriptRunnerImpl pokemon center", () => {
  it("registers pokemon center on the runner and overworld", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const overworld = {} as OverworldEngine;

    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);

    expect(runner.pokemon_center).toBeDefined();
    expect(typeof runner.pokemon_center.heal_party).toBe("function");
    expect((overworld as unknown as { pokemon_center?: unknown }).pokemon_center).toBe(
      runner.pokemon_center
    );
  });

  it("releases the runner pause after a completed nurse interaction", async () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = {
      input_capture_active: false,
      current_map_name: "CherrygrovePokecenter1F",
      dialogue: {
        active: false,
        visible: false,
        waiting_for_input: false,
        pending_waits: 0,
        acknowledge_wait: jest.fn(() => true),
      },
      lock_player_movement: jest.fn(),
      unlock_player_movement: jest.fn(),
      ui: {
        eventQueue: [],
        update: jest.fn(),
      },
      update: jest.fn(),
      draw: jest.fn(),
      audio_engine: {
        playMusic: jest.fn(),
        restartMapMusic: jest.fn(),
      },
    } as unknown as OverworldEngine;
    const dataLoader = new DataLoader();
    dataLoader.get_script = () => null;

    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    runner._script_stack.push({
      name: "TestScript",
      commands: [],
      index: 0,
    });

    const promptSpy = jest
      .spyOn(
        runner.pokemon_center as unknown as {
          promptYesNo: typeof runner.pokemon_center["promptYesNo"];
        },
        "promptYesNo"
      )
      .mockResolvedValue(true);
    const waitSpy = jest
      .spyOn(
        runner.pokemon_center as unknown as {
          waitForButton: typeof runner.pokemon_center["waitForButton"];
        },
        "waitForButton"
      )
      .mockResolvedValue(undefined);
    const pauseSpy = jest
      .spyOn(
        runner.pokemon_center as unknown as {
          pauseFrames: typeof runner.pokemon_center["pauseFrames"];
        },
        "pauseFrames"
      )
      .mockResolvedValue(undefined);
    const healSpy = jest
      .spyOn(runner.pokemon_center, "playHealMachineAnimationAsync")
      .mockResolvedValue(undefined);

    await runner.pokemon_center.runNurseInteraction(runner, eventManager, overworld as any);

    expect(overworld.input_capture_active).toBe(false);
    expect(overworld.lock_player_movement).toHaveBeenCalledTimes(1);
    expect(overworld.unlock_player_movement).toHaveBeenCalledTimes(1);
    expect(runner._queued_overworld_task_count).toBe(0);
    expect(runner._awaiting_resume).toBe(0);
    expect(runner.stopExecution).toBe(false);
    expect(runner.state).toBe(ScriptRunnerState.IDLE);

    healSpy.mockRestore();
    pauseSpy.mockRestore();
    waitSpy.mockRestore();
    promptSpy.mockRestore();
  });

  it("restores live overworld control after a real nurse interaction flow", async () => {
    jest.useFakeTimers();
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const shownTexts: string[] = [];
    eventManager.on("show_text", (event) => {
      shownTexts.push(String((event.data as { text?: unknown }).text ?? ""));
    });
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawTextBox: jest.fn(),
      draw_window: jest.fn(),
    };
    const overworld = {
      input_capture_active: false,
      current_map_name: "CherrygrovePokecenter1F",
      lock_player_movement: jest.fn(),
      unlock_player_movement: jest.fn(),
      ui: {
        eventQueue: gameEngine.event.createQueue(),
        update: jest.fn(),
      },
      update: jest.fn(),
      draw: jest.fn(),
      audio_engine: {
        playMusic: jest.fn(),
        restartMapMusic: jest.fn(),
      },
    } as unknown as OverworldEngine & {
      handleInput: (event: unknown) => void;
      dialogue: FieldDialogueManager;
    };
    const dataLoader = new DataLoader();
    dataLoader.get_script = () => null;

    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const dialogue = new FieldDialogueManager(ui, gameState, runner);
    overworld.dialogue = dialogue;
    overworld.handleInput = (event: unknown) => {
      dialogue.handle_input(event as Parameters<typeof dialogue.handle_input>[0]);
    };
    for (const eventName of ["open_text", "close_text", "show_text", "wait_for_input", "prompt_yes_no"]) {
      eventManager.on(eventName, dialogue.handle_event.bind(dialogue));
    }
    runner._script_stack.push({
      name: "TestScript",
      commands: [],
      index: 0,
    });

    const promptSpy = jest
      .spyOn(
        runner.pokemon_center as unknown as {
          promptYesNo: typeof runner.pokemon_center["promptYesNo"];
        },
        "promptYesNo"
      )
      .mockResolvedValue(true);
    const nextFrameSpy = jest.spyOn(asyncLoop, "nextFrame").mockImplementation(async () => {
      dialogue.update();
    });
    const healSpy = jest
      .spyOn(runner.pokemon_center, "playHealMachineAnimationAsync")
      .mockResolvedValue(undefined);

    const interaction = runner.pokemon_center.runNurseInteraction(runner, eventManager, overworld);

    const pressA = async (): Promise<void> => {
      gameEngine.event.post({ type: "keyup", code: "KeyZ" }, overworld.ui.eventQueue);
      gameEngine.event.post({ type: "keydown", code: "KeyZ" }, overworld.ui.eventQueue);
      jest.advanceTimersByTime(20);
      dialogue.update();
      await Promise.resolve();
    };

    const pumpFrame = async (): Promise<void> => {
      jest.advanceTimersByTime(20);
      dialogue.update();
      await Promise.resolve();
    };

    for (let i = 0; i < 120 && !shownTexts.includes("We hope to see you again."); i += 1) {
      if (dialogue.waiting_for_input || dialogue.active) {
        await pressA();
      } else {
        await pumpFrame();
      }
    }

    expect(shownTexts).toContain("We hope to see you again.");

    for (let i = 0; i < 30 && overworld.input_capture_active; i += 1) {
      if (dialogue.waiting_for_input || dialogue.active) {
        await pressA();
      } else {
        await pumpFrame();
      }
    }
    await interaction;

    expect(overworld.input_capture_active).toBe(false);
    expect(overworld.lock_player_movement).toHaveBeenCalledTimes(1);
    expect(overworld.unlock_player_movement).toHaveBeenCalledTimes(1);
    expect(dialogue.active).toBe(false);
    expect(runner._queued_overworld_task_count).toBe(0);
    expect(runner._awaiting_resume).toBe(0);
    expect(runner.stopExecution).toBe(false);
    expect(runner.state).toBe(ScriptRunnerState.IDLE);

    healSpy.mockRestore();
    nextFrameSpy.mockRestore();
    promptSpy.mockRestore();
    jest.useRealTimers();
  });
});

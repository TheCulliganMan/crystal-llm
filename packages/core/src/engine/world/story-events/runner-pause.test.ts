import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/world/events";
import { ScriptRunnerState, ScriptRunnerImpl } from "./runner";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";

describe("ScriptRunner pause handling", () => {
  it("pauses when dialogue marks script paused after wait_for_input", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dialogue = { _script_paused: false } as { _script_paused: boolean };
    const overworld = {
      dialogue,
    } as unknown as OverworldEngine;
    const dataLoader = new DataLoader();
    dataLoader.get_script = (name: string) => {
      if (name === "TestScript") {
        return [
          { command: "describedecoration", args: ["DECODESC_CONSOLE"] },
        ];
      }
      return null;
    };
    dataLoader.get_text = () => null;

    eventManager.on("wait_for_input", () => {
      dialogue._script_paused = true;
    });

    const runner = new ScriptRunnerImpl(
      gameState,
      eventManager,
      dataLoader,
      overworld,
    );
    runner.run("TestScript");

    expect(runner.stopExecution).toBe(true);
    expect(runner._script_stack.length).toBeGreaterThan(0);
    expect(runner.state).toBe(ScriptRunnerState.PAUSED);
  });

  it("clears movement lock within a frame budget after Mom intro-style hidden waits", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dialogue = {
      active: false,
      waiting_for_input: true,
      pending_waits: 1,
      acknowledge_wait: jest.fn(function acknowledgeWait(this: {
        waiting_for_input: boolean;
        pending_waits: number;
      }) {
        this.pending_waits = 0;
        this.waiting_for_input = false;
        return true;
      }),
    };
    const overworld = {
      dialogue,
    } as unknown as OverworldEngine;
    const dataLoader = new DataLoader();
    dataLoader.get_script = () => null;

    const runner = new ScriptRunnerImpl(
      gameState,
      eventManager,
      dataLoader,
      overworld,
    );

    runner._script_stack.push({
      name: "TestScript",
      commands: [],
      index: 0,
    });

    runner._queue_overworld_task((callback) => {
      callback();
      return true;
    });

    const isMovementLocked = (): boolean =>
      runner.is_busy ||
      runner.stopExecution ||
      runner._script_stack.length > 0 ||
      runner._awaiting_resume > 0;
    let unlockedFrame: number | null = null;
    for (let frame = 0; frame < 120; frame += 1) {
      if (!isMovementLocked()) {
        unlockedFrame = frame;
        break;
      }
    }

    expect(dialogue.acknowledge_wait).toHaveBeenCalled();
    expect(runner._awaiting_resume).toBe(0);
    expect(runner.stopExecution).toBe(false);
    expect(runner.state).toBe(ScriptRunnerState.IDLE);
    expect(unlockedFrame).not.toBeNull();
  });

  it("does not preserve an endifjustbattled stop behind a stale pause flag", () => {
    const gameState = createInitialGameState();
    gameState.wram.wRunningTrainerBattleScript = -1;
    const eventManager = new EventManager(gameState);
    const overworld = {
      dialogue: {
        active: false,
        visible: false,
        waiting_for_input: false,
      },
    } as unknown as OverworldEngine;
    const dataLoader = new DataLoader();
    dataLoader.get_script = (name: string) => {
      if (name === "AfterTrainerScript") {
        return [{ command: "endifjustbattled", args: [] }];
      }
      return null;
    };

    const runner = new ScriptRunnerImpl(
      gameState,
      eventManager,
      dataLoader,
      overworld,
    );
    (runner as unknown as { _pause_execution: boolean })._pause_execution = true;

    runner.run("AfterTrainerScript");

    expect(runner._awaiting_resume).toBe(0);
    expect(runner._script_stack).toHaveLength(0);
    expect(runner.stopExecution).toBe(false);
    expect(runner.state).toBe(ScriptRunnerState.IDLE);
    expect(gameState.wram.wRunningTrainerBattleScript).toBe(0);
  });

  it("tracks queued overworld tasks until their callback resumes", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = {
      dialogue: null,
    } as unknown as OverworldEngine;
    const dataLoader = new DataLoader();
    dataLoader.get_script = () => null;

    const runner = new ScriptRunnerImpl(
      gameState,
      eventManager,
      dataLoader,
      overworld,
    );

    runner._script_stack.push({
      name: "TestScript",
      commands: [],
      index: 0,
    });

    let resumeCallback: (() => void) | null = null;
    runner._queue_overworld_task((callback) => {
      resumeCallback = callback;
      return true;
    });

    expect(runner._queued_overworld_task_count).toBe(1);
    expect(runner._awaiting_resume).toBeGreaterThan(0);

    resumeCallback?.();

    expect(runner._queued_overworld_task_count).toBe(0);
  });
});

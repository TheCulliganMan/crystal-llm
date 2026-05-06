import { createInitialGameState } from "@pokecrystal/core/core/state";
import { getMapMetadataByConstant } from "@pokecrystal/core/engine/world/maps";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import { EventManager as ConcreteEventManager } from "@pokecrystal/core/engine/events/events";
import { ElevatorCommand } from "./overworld";

describe("ElevatorCommand floor label parity", () => {
  const elevatorData = [
    { command: "elevfloor", args: ["FLOOR_1F", "4", "GOLDENROD_DEPT_STORE_1F"] },
    { command: "elevfloor", args: ["FLOOR_2F", "3", "GOLDENROD_DEPT_STORE_2F"] },
    { command: "db", args: ["-1"] },
  ];

  it("pauses for an elevator floor prompt instead of silently choosing the current floor", () => {
    const gameState = createInitialGameState();
    const eventManager = new ConcreteEventManager(gameState);
    const origin = getMapMetadataByConstant("GOLDENROD_DEPT_STORE_1F");
    const destination = getMapMetadataByConstant("GOLDENROD_DEPT_STORE_2F");
    if (!origin || !destination) {
      throw new Error("Expected Goldenrod Dept. Store elevator maps to exist.");
    }

    gameState.wram.wBackupMapGroup = origin.groupId;
    gameState.wram.wBackupMapNumber = origin.mapId;

    const pause = jest.fn();
    const startRide = jest.fn();
    const command = new ElevatorCommand("GoldenrodDeptStoreElevatorData");
    command.runner = {
      dataLoader: { getScript: () => elevatorData },
      _script_stack: [{ name: "GoldenrodDeptStoreElevatorScript" }],
      pause,
      resume: jest.fn(),
      last_condition_result: false,
      variables: {},
    } as any;

    const dispatchSpy = jest.spyOn(eventManager, "dispatch");
    command.execute(gameState, eventManager, { start_elevator_ride: startRide } as any);

    expect(pause).toHaveBeenCalledTimes(1);
    expect(dispatchSpy).toHaveBeenCalledTimes(1);
    const promptEvent = dispatchSpy.mock.calls[0]?.[0];
    expect(promptEvent?.name).toBe("prompt_selection");
    expect(promptEvent?.data).toEqual(
      expect.objectContaining({
        key: "_elevator_selection",
        options: ["1F", "2F"],
        initial_index: 0,
        cancel_index: 0,
      }),
    );

    (promptEvent?.data.callback as (selection: number) => void)(1);

    expect(gameState.wram.wBackupMapGroup).toBe(destination.groupId);
    expect(gameState.wram.wBackupMapNumber).toBe(destination.mapId);
    expect(gameState.wram.wBackupWarpNumber).toBe(3);
    expect(gameState.wram.script_memory["wScriptVar"]).toBe(1);
    expect(startRide).toHaveBeenCalledWith("1F", "2F", expect.objectContaining({ trigger_sound: true }));
  });

  it("throws instead of prettifying an unknown FLOOR_* label", () => {
    const gameState = createInitialGameState();
    const origin = getMapMetadataByConstant("GOLDENROD_DEPT_STORE_1F");
    const destination = getMapMetadataByConstant("GOLDENROD_DEPT_STORE_2F");
    if (!origin || !destination) {
      throw new Error("Expected Goldenrod Dept. Store elevator maps to exist.");
    }

    gameState.wram.wBackupMapGroup = origin.groupId;
    gameState.wram.wBackupMapNumber = origin.mapId;

    const command = new ElevatorCommand("TestElevatorData");
    command.runner = {
      dataLoader: {
        getScript: () => [
          { command: "elevfloor", args: ["FLOOR_1F", "4", "GOLDENROD_DEPT_STORE_1F"] },
          { command: "elevfloor", args: ["FLOOR_FAKE", "3", "GOLDENROD_DEPT_STORE_2F"] },
          { command: "db", args: ["-1"] },
        ],
      },
      _consume_script_choice: () => 1,
      _script_stack: [{ name: "GoldenrodDeptStoreElevatorScript" }],
      pause: jest.fn(),
      resume: jest.fn(),
      last_condition_result: false,
      variables: {},
    } as any;

    expect(() =>
      command.execute(gameState, {} as EventManager, {} as any),
    ).toThrow("Missing ASM floor label 'FLOOR_FAKE'.");
  });
});

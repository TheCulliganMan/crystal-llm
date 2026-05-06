import { createInitialGameState } from "@pokecrystal/core/core/state";
import { TARGET_LAYOUT } from "@pokecrystal/core/engine/games/unown-puzzle";
import { unown_puzzle_special } from "./games";
import { createScriptRunnerStub } from "@pokecrystal/core/engine/world/story-events/test-utils";
import { getUnownOverlayLockDepth } from "./unown-overlay-lock";
import { PUZZLE_IDS } from "@pokecrystal/assets/content/data/unown-puzzles/unown-puzzle-assets";

const mockOverlayRun = jest.fn(() => false);
const mockOverlayRunAsync = jest.fn(async () => false);

jest.mock("@pokecrystal/core/ui/overlays/unown-puzzle", () => ({
  UnownPuzzleOverlay: class {
    run(...args: unknown[]): boolean {
      return mockOverlayRun(...args);
    }

    async runAsync(...args: unknown[]): Promise<boolean> {
      return mockOverlayRunAsync(...args);
    }
  },
}));

const expectSyncUnownResult = (
  result: ReturnType<typeof unown_puzzle_special>,
): Awaited<ReturnType<typeof unown_puzzle_special>> => {
  if (result instanceof Promise) {
    throw new Error("Expected synchronous Unown puzzle result.");
  }
  return result;
};

describe("unown_puzzle_special", () => {
  afterEach(() => {
    mockOverlayRun.mockReset();
    mockOverlayRun.mockImplementation(() => false);
    mockOverlayRunAsync.mockReset();
    mockOverlayRunAsync.mockImplementation(async () => false);
    jest.restoreAllMocks();
  });

  it("reinitializes UI puzzles even if a solved layout was stored", async () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
    runner.variables["_value"] = "UNOWNPUZZLE_KABUTO";
    runner.variables["unown_layout_KABUTO"] = TARGET_LAYOUT;

    const overworld = { ui: {} } as unknown as { ui: Record<string, unknown> };

    const result = await unown_puzzle_special({ game_state: gameState, runner, overworld });

    expect(result.layout[1][1]).toBe(0);
    expect(result.solved).toBe(false);
    expect(gameState.wram.wSolvedUnownPuzzle).toBe(false);
    expect(mockOverlayRunAsync).toHaveBeenCalledTimes(1);
    expect(mockOverlayRun).not.toHaveBeenCalled();
  });

  it("normalizes numeric Unown puzzle ids to asset ids", async () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
    runner.variables["_value"] = 2;

    const overworld = { ui: {} } as unknown as { ui: Record<string, unknown> };

    await unown_puzzle_special({ game_state: gameState, runner, overworld });

    expect(mockOverlayRunAsync).toHaveBeenCalledTimes(1);
    expect(mockOverlayRunAsync).toHaveBeenCalledWith(
      "AERODACTYL",
      expect.any(Object),
      expect.any(Object),
    );
  });

  it.each([
    ["UNOWNPUZZLE_KABUTO", "KABUTO"],
    ["UNOWNPUZZLE_OMANYTE", "OMANYTE"],
    ["UNOWNPUZZLE_AERODACTYL", "AERODACTYL"],
    ["UNOWNPUZZLE_HO_OH", "HOOH"],
    [0, "KABUTO"],
    ["1", "OMANYTE"],
    [2, "AERODACTYL"],
    ["3", "HOOH"],
  ])("runs the headless action path for puzzle id %s as %s", (rawPuzzleId, puzzleId) => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
    runner.variables["_value"] = rawPuzzleId;
    runner.variables["unown_action"] = "noop";

    const result = expectSyncUnownResult(
      unown_puzzle_special({ game_state: gameState, runner, overworld: null }),
    );

    expect(result).toEqual(
      expect.objectContaining({
        puzzle_id: puzzleId,
        solved: false,
        holding_piece: null,
      }),
    );
    expect(runner.variables[`unown_layout_${puzzleId}`]).toBeDefined();
    expect(runner.last_value).toEqual(result);
    expect(runner.last_condition_result).toBe(false);
  });

  it("persists pickup and place actions without leaking transient coordinates", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
    runner.variables["_value"] = "UNOWNPUZZLE_KABUTO";
    runner.variables["unown_layout_KABUTO"] = TARGET_LAYOUT.map(row => [...row]);
    runner.variables["unown_action"] = "pickup";
    runner.variables["unown_x"] = 1;
    runner.variables["unown_y"] = 1;

    const pickupResult = expectSyncUnownResult(
      unown_puzzle_special({ game_state: gameState, runner, overworld: null }),
    );

    expect(pickupResult.holding_piece).toBe(1);
    expect(pickupResult.layout[1][1]).toBe(0);
    expect(pickupResult.moves).toBe(0);
    expect(runner.variables["unown_action"]).toBeUndefined();
    expect(runner.variables["unown_x"]).toBeUndefined();
    expect(runner.variables["unown_y"]).toBeUndefined();

    runner.variables["_value"] = "UNOWNPUZZLE_KABUTO";
    runner.variables["unown_action"] = "place";
    runner.variables["unown_x"] = 0;
    runner.variables["unown_y"] = 0;

    const placeResult = expectSyncUnownResult(
      unown_puzzle_special({ game_state: gameState, runner, overworld: null }),
    );

    expect(placeResult.holding_piece).toBeNull();
    expect(placeResult.layout[0][0]).toBe(1);
    expect(placeResult.moves).toBe(1);
    expect(runner.variables["unown_moves_KABUTO"]).toBe(1);
  });

  it("detects solved headless layouts and sets script condition state", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
    runner.variables["_value"] = "UNOWNPUZZLE_KABUTO";
    runner.variables["unown_layout_KABUTO"] = TARGET_LAYOUT.map(row => [...row]);
    runner.variables["unown_moves_KABUTO"] = 12;
    runner.variables["unown_action"] = "noop";

    const result = expectSyncUnownResult(
      unown_puzzle_special({ game_state: gameState, runner, overworld: null }),
    );

    expect(result.solved).toBe(true);
    expect(result.moves).toBe(12);
    expect(gameState.wram.wSolvedUnownPuzzle).toBe(true);
    expect(runner.last_condition_result).toBe(true);
  });

  it("keeps stored layouts isolated per fossil puzzle", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
    runner.variables["_value"] = "UNOWNPUZZLE_KABUTO";
    runner.variables["unown_layout"] = TARGET_LAYOUT.map(row => [...row]);
    runner.variables["unown_action"] = "noop";

    unown_puzzle_special({ game_state: gameState, runner, overworld: null });

    expect(runner.variables["unown_layout"]).toBeUndefined();
    expect(runner.variables["unown_layout_KABUTO"]).toEqual(TARGET_LAYOUT);

    runner.variables["_value"] = "UNOWNPUZZLE_OMANYTE";
    runner.variables["unown_action"] = "noop";
    unown_puzzle_special({ game_state: gameState, runner, overworld: null });

    expect(runner.variables["unown_layout_KABUTO"]).toEqual(TARGET_LAYOUT);
    expect(runner.variables["unown_layout_OMANYTE"]).toBeDefined();
    expect(runner.variables["unown_layout_OMANYTE"]).not.toEqual(TARGET_LAYOUT);
  });

  it("rejects impossible duplicated puzzle pieces from restored headless state", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
    const duplicateLayout = TARGET_LAYOUT.map(row => [...row]);
    duplicateLayout[1][2] = 1;
    runner.variables["_value"] = "UNOWNPUZZLE_KABUTO";
    runner.variables["unown_layout_KABUTO"] = duplicateLayout;
    runner.variables["unown_action"] = "noop";

    expect(() => unown_puzzle_special({ game_state: gameState, runner, overworld: null })).toThrow(
      "piece 1 appears more than once in the puzzle state",
    );
  });

  it("rejects malformed restored layouts instead of reshuffling them", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
    runner.variables["_value"] = "UNOWNPUZZLE_HO_OH";
    runner.variables["unown_layout_HOOH"] = TARGET_LAYOUT.slice(0, 5);
    runner.variables["unown_action"] = "noop";

    expect(() => unown_puzzle_special({ game_state: gameState, runner, overworld: null })).toThrow(
      "Stored Unown puzzle layout for HOOH is invalid.",
    );
  });

  it("stores UI-completed puzzle results for every fossil puzzle id", async () => {
    mockOverlayRunAsync.mockResolvedValue(true);

    for (const puzzleId of PUZZLE_IDS) {
      const gameState = createInitialGameState();
      const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
      runner.variables["_value"] = puzzleId;
      const overworld = { ui: {} } as unknown as { ui: Record<string, unknown> };

      const result = await unown_puzzle_special({ game_state: gameState, runner, overworld });

      expect(result.puzzle_id).toBe(puzzleId);
      expect(result.solved).toBe(true);
      expect(gameState.wram.wSolvedUnownPuzzle).toBe(true);
      expect(runner.last_condition_result).toBe(true);
      expect(runner.variables[`unown_layout_${puzzleId}`]).toBeDefined();
    }
  });

  it("throws when _value is not a known Unown puzzle id", async () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
    runner.variables["_value"] = "UNOWNPUZZLE_UNKNOWN";

    expect(() =>
      unown_puzzle_special({ game_state: gameState, runner, overworld: { ui: {} } as never }),
    ).toThrow("Unknown Unown puzzle id 'UNOWNPUZZLE_UNKNOWN'.");
  });

  it("clears transient headless action vars so future script calls can open the UI path", async () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
    runner.variables["_value"] = "UNOWNPUZZLE_KABUTO";
    runner.variables["unown_action"] = "noop";
    runner.variables["unown_x"] = 0;
    runner.variables["unown_y"] = 0;

    unown_puzzle_special({ game_state: gameState, runner, overworld: null });

    expect(runner.variables["unown_action"]).toBeUndefined();
    expect(runner.variables["unown_x"]).toBeUndefined();
    expect(runner.variables["unown_y"]).toBeUndefined();
    expect(mockOverlayRun).not.toHaveBeenCalled();
    expect(mockOverlayRunAsync).not.toHaveBeenCalled();

    await unown_puzzle_special({ game_state: gameState, runner, overworld: { ui: {} } as never });

    expect(mockOverlayRunAsync).toHaveBeenCalledTimes(1);
    expect(mockOverlayRun).not.toHaveBeenCalled();
  });

  it("locks overworld input capture during async puzzle UI and restores it afterward", async () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
    runner.variables["_value"] = "UNOWNPUZZLE_KABUTO";

    let resolveRun: ((value: boolean) => void) | null = null;
    mockOverlayRunAsync.mockImplementation(
      () =>
        new Promise<boolean>((resolve) => {
          resolveRun = resolve;
        }),
    );

    const overworld = { ui: {}, input_capture_active: false } as unknown as {
      ui: Record<string, unknown>;
      input_capture_active: boolean;
    };

    const pending = unown_puzzle_special({ game_state: gameState, runner, overworld });
    expect(overworld.input_capture_active).toBe(true);
    expect(getUnownOverlayLockDepth(gameState)).toBe(1);

    resolveRun?.(false);
    await pending;

    expect(overworld.input_capture_active).toBe(false);
    expect(getUnownOverlayLockDepth(gameState)).toBe(0);
  });

  it("releases the overlay lock after a solved async UI run so the script can resume", async () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
    runner.variables["_value"] = "UNOWNPUZZLE_KABUTO";
    let resolveRun: ((value: boolean) => void) | null = null;
    mockOverlayRunAsync.mockImplementation(
      () =>
        new Promise<boolean>((resolve) => {
          resolveRun = resolve;
        }),
    );
    const overworld = { ui: {}, input_capture_active: false } as unknown as {
      ui: Record<string, unknown>;
      input_capture_active: boolean;
    };

    const pending = unown_puzzle_special({ game_state: gameState, runner, overworld });
    expect(overworld.input_capture_active).toBe(true);
    expect(gameState.wram.wUnownState).toBe(1);
    expect(getUnownOverlayLockDepth(gameState)).toBe(1);

    resolveRun?.(true);
    const result = await pending;

    expect(result.solved).toBe(true);
    expect(gameState.wram.wSolvedUnownPuzzle).toBe(true);
    expect(runner.last_condition_result).toBe(true);
    expect(overworld.input_capture_active).toBe(false);
    expect(gameState.wram.wUnownState).toBe(0);
    expect(getUnownOverlayLockDepth(gameState)).toBe(0);
  });

  it("keeps overlay lock active until all overlapping puzzle overlays resolve", async () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
    runner.variables["_value"] = "UNOWNPUZZLE_KABUTO";

    let resolveFirst: ((value: boolean) => void) | null = null;
    let resolveSecond: ((value: boolean) => void) | null = null;
    mockOverlayRunAsync
      .mockImplementationOnce(
        () =>
          new Promise<boolean>((resolve) => {
            resolveFirst = resolve;
          }),
      )
      .mockImplementationOnce(
        () =>
          new Promise<boolean>((resolve) => {
            resolveSecond = resolve;
          }),
      );

    const overworld = { ui: {}, input_capture_active: false } as unknown as {
      ui: Record<string, unknown>;
      input_capture_active: boolean;
    };

    const first = unown_puzzle_special({ game_state: gameState, runner, overworld });
    const second = unown_puzzle_special({ game_state: gameState, runner, overworld });

    expect(overworld.input_capture_active).toBe(true);
    expect(gameState.wram.wUnownState).toBe(1);
    expect(getUnownOverlayLockDepth(gameState)).toBe(2);

    resolveFirst?.(false);
    await first;

    expect(overworld.input_capture_active).toBe(true);
    expect(gameState.wram.wUnownState).toBe(1);
    expect(getUnownOverlayLockDepth(gameState)).toBe(1);

    resolveSecond?.(false);
    await second;

    expect(overworld.input_capture_active).toBe(false);
    expect(gameState.wram.wUnownState).toBe(0);
    expect(getUnownOverlayLockDepth(gameState)).toBe(0);
  });

  it("reacquires and releases overlay lock across repeated UI runs", async () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ game_state: gameState, variables: {} });
    runner.variables["_value"] = "UNOWNPUZZLE_KABUTO";
    const overworld = { ui: {}, input_capture_active: false } as unknown as {
      ui: Record<string, unknown>;
      input_capture_active: boolean;
    };

    let resolveFirst: ((value: boolean) => void) | null = null;
    mockOverlayRunAsync.mockImplementationOnce(
      () =>
        new Promise<boolean>((resolve) => {
          resolveFirst = resolve;
        }),
    );
    const first = unown_puzzle_special({ game_state: gameState, runner, overworld });
    expect(getUnownOverlayLockDepth(gameState)).toBe(1);
    resolveFirst?.(false);
    await first;
    expect(getUnownOverlayLockDepth(gameState)).toBe(0);
    expect(overworld.input_capture_active).toBe(false);

    let resolveSecond: ((value: boolean) => void) | null = null;
    mockOverlayRunAsync.mockImplementationOnce(
      () =>
        new Promise<boolean>((resolve) => {
          resolveSecond = resolve;
        }),
    );
    const second = unown_puzzle_special({ game_state: gameState, runner, overworld });
    expect(getUnownOverlayLockDepth(gameState)).toBe(1);
    resolveSecond?.(false);
    await second;
    expect(getUnownOverlayLockDepth(gameState)).toBe(0);
    expect(overworld.input_capture_active).toBe(false);
  });
});

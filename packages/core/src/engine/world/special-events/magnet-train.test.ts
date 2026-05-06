import { createInitialGameState } from "@pokecrystal/core/core/state";
import { createScriptRunnerStub } from "@pokecrystal/core/engine/world/story-events/test-utils";
import { magnet_train } from "./magnet-train";
import { MagnetTrainAnimator } from "@pokecrystal/core/ui/screens/magnet-train-animation";

describe("magnet_train", () => {
  it("waits for async animation completion before applying map transition", async () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({
      variables: { _value: 1 },
      last_condition_result: false,
    });
    const loadMap = jest.fn();
    const restartMapMusic = jest.fn();
    const overworld = {
      ui: { screen: null, update: jest.fn() },
      audio_engine: null,
      load_map: loadMap,
      restartMapMusic,
      TILES_PER_COLLISION: 2,
    };

    let resolveAnimation!: () => void;
    const animationPromise = new Promise<void>((resolve) => {
      resolveAnimation = resolve;
    });
    const playAsyncSpy = jest
      .spyOn(MagnetTrainAnimator.prototype, "playAsync")
      .mockReturnValue(animationPromise);
    const playSpy = jest.spyOn(MagnetTrainAnimator.prototype, "play").mockImplementation(() => {});

    try {
      const result = magnet_train(gameState, { runner, overworld });

      expect(result).toBeInstanceOf(Promise);
      expect(loadMap).not.toHaveBeenCalled();
      expect(runner.last_condition_result).toBe(false);

      resolveAnimation();
      await expect(result).resolves.toBe(true);
      expect(loadMap).toHaveBeenCalledTimes(1);
      expect(restartMapMusic).toHaveBeenCalledTimes(1);
      expect(runner.last_condition_result).toBe(true);
      expect(playSpy).not.toHaveBeenCalled();
    } finally {
      playAsyncSpy.mockRestore();
      playSpy.mockRestore();
    }
  });

  it("falls back to sync animation when async playback is unavailable", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({
      variables: { _value: 1 },
      last_condition_result: false,
    });
    const loadMap = jest.fn();
    const restartMapMusic = jest.fn();
    const overworld = {
      ui: { screen: null, update: jest.fn() },
      audio_engine: null,
      load_map: loadMap,
      restartMapMusic,
      TILES_PER_COLLISION: 2,
    };

    const playAsyncSpy = jest
      .spyOn(MagnetTrainAnimator.prototype, "playAsync")
      .mockReturnValue(undefined as unknown as Promise<void>);
    const playSpy = jest.spyOn(MagnetTrainAnimator.prototype, "play").mockImplementation(() => {});

    try {
      const result = magnet_train(gameState, { runner, overworld });

      expect(result).toBe(true);
      expect(playSpy).toHaveBeenCalledTimes(1);
      expect(loadMap).toHaveBeenCalledTimes(1);
      expect(restartMapMusic).toHaveBeenCalledTimes(1);
      expect(runner.last_condition_result).toBe(true);
    } finally {
      playAsyncSpy.mockRestore();
      playSpy.mockRestore();
    }
  });
});

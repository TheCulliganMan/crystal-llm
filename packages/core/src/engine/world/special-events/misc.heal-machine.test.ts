import { createInitialGameState } from "@pokecrystal/core/core/state";
import { heal_machine_anim } from "./misc";

describe("heal_machine_anim", () => {
  it("uses async pokemon center playback when available", async () => {
    const gameState = createInitialGameState();
    const playHealMachineAnimationAsync = jest.fn(() => Promise.resolve());
    const runner = {
      variables: { _value: 2 },
      last_condition_result: false,
      pokemon_center: {
        heal_party: jest.fn(() => ({ healed_slots: [] })),
        playHealMachineAnimationAsync,
      },
    } as any;

    const result = heal_machine_anim({
      game_state: gameState,
      runner,
      overworld: undefined,
      event_manager: undefined,
    });

    expect(playHealMachineAnimationAsync).toHaveBeenCalledWith("2", null);
    expect(result).toBeInstanceOf(Promise);
    await expect(result).resolves.toBe(true);
    expect(runner.last_condition_result).toBe(true);
  });

  it("falls back to sync pokemon center playback when async is unavailable", () => {
    const gameState = createInitialGameState();
    const playHealMachineAnimation = jest.fn();
    const runner = {
      variables: { _value: 1 },
      last_condition_result: false,
      pokemon_center: {
        heal_party: jest.fn(() => ({ healed_slots: [] })),
        playHealMachineAnimation,
      },
    } as any;

    const result = heal_machine_anim({
      game_state: gameState,
      runner,
      overworld: undefined,
      event_manager: undefined,
    });

    expect(playHealMachineAnimation).toHaveBeenCalledWith("1", null);
    expect(result).toBe(true);
    expect(runner.last_condition_result).toBe(true);
  });
});

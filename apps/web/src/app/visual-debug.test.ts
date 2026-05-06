import { runVisualDebugScript } from "./visual-debug";

describe("runVisualDebugScript", () => {
  it("drives scripted input until completion", async () => {
    const postEvent = jest.fn();
    const tick = jest.fn();
    const getDebugStatus = jest
      .fn()
      .mockReturnValue({
        mode: "overworld",
        mapName: "New Bark Town",
        mapGroup: 24,
        mapNumber: 4,
        coords: { x: 7, y: 6 },
        prompt_pending: false,
        prompt_reason: null,
        in_dialog: false,
        in_menu: false,
        in_battle: false,
        movement_locked: false,
        script_busy: false,
        can_move: true,
        current_spawn: 14,
      });

    const game = {
      postEvent,
      tick,
      getDebugStatus,
      getState: () => "overworld",
      getMapName: () => "New Bark Town",
      getGameState: () => ({ frame_counter: 9 }),
    } as any;

    const result = await runVisualDebugScript(game, ["a", { wait: 2 }, "right"], {
      maxFrames: 20,
    });

    expect(postEvent).toHaveBeenCalled();
    expect(tick).toHaveBeenCalled();
    expect(result.frames).toBeGreaterThan(0);
    expect(result.events).toBeGreaterThan(0);
    expect(result.complete).toBe(true);
  });
});

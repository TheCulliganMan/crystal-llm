import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager, type Event } from "@pokecrystal/core/engine/events/events";
import { SelectionPrompt } from "@pokecrystal/core/ui/text/prompts";
import { bank_of_mom } from "./mom";

describe("bank_of_mom", () => {
  it("queues mom text when the bank is initialized", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");

    bank_of_mom(gameState, { event_manager: eventManager });

    const showTextEvents = dispatchSpy.mock.calls
      .map((call) => call[0] as Event)
      .filter((event) => event.name === "show_text");
    expect(showTextEvents.length).toBeGreaterThan(0);
  });

  it("uses the async prompt loop for Mom's rendered banking menu", async () => {
    const gameState = createInitialGameState();
    gameState.sram.mom_saving_active = true;
    const eventManager = new EventManager(gameState);
    const runSpy = jest
      .spyOn(SelectionPrompt.prototype, "run")
      .mockImplementation(() => {
        throw new Error("sync SelectionPrompt.run should not be used for Mom's banking menu");
      });
    const runAsyncSpy = jest
      .spyOn(SelectionPrompt.prototype, "runAsync")
      .mockResolvedValue(3);
    const runner: any = {
      event_manager: eventManager,
      last_yes_no_result: true,
      last_condition_result: false,
      command_map: {
        yesno: jest.fn(),
      },
    };
    runner.command_map.yesno = () => {
      const command = {
        runner: null as unknown,
        on_result: undefined as ((value: boolean) => void) | undefined,
        execute: jest.fn(function execute(this: { on_result?: (value: boolean) => void }) {
          this.on_result?.(true);
        }),
      };
      return command;
    };
    const overworld = {
      ui: {},
      draw: jest.fn(),
    };

    try {
      const result = bank_of_mom(gameState, {
        event_manager: eventManager,
        runner: runner as any,
        overworld,
      });

      expect(result).toBeInstanceOf(Promise);
      await result;
      expect(runSpy).not.toHaveBeenCalled();
      expect(runAsyncSpy).toHaveBeenCalledTimes(1);
      expect(runner.last_condition_result).toBe(true);
    } finally {
      runSpy.mockRestore();
      runAsyncSpy.mockRestore();
    }
  });
});

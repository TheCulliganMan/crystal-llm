import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { TextFormatter } from "../text-formatter";
import { TradeCommand, WaitButtonCommand, WriteTextCommand } from "./text";

describe("WaitButtonCommand", () => {
  it("still dispatches wait_for_input while instant mode is enabled", () => {
    const gameState = createInitialGameState();
    gameState.wram.instant_mode = true;
    const eventManager = new EventManager(gameState);
    const events: string[] = [];

    eventManager.on("wait_for_input", (event) => {
      events.push(event.name);
    });

    const command = new WaitButtonCommand();
    command.execute(gameState, eventManager, {} as never);

    expect(events).toEqual(["wait_for_input"]);
  });
});

describe("TradeCommand", () => {
  it("shows Tim's NPC trade intro instead of leaving the textbox blank", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const formatter = new TextFormatter(gameState);
    const shownTexts: string[] = [];

    eventManager.on("show_text", (event) => {
      shownTexts.push(String(event.data?.text ?? ""));
    });

    const command = new TradeCommand("NPC_TRADE_TIM");
    command.runner = {
      string_buffers: formatter.stringBuffers,
      formatText: (text: string) => formatter.formatText(text),
    } as never;

    command.execute(gameState, eventManager, {} as never);

    expect(shownTexts).toEqual([
      "Hi, I'm looking\nfor this POKéMON.\n\nIf you have\nKRABBY, would\n\nyou trade it for\nmy VOLTORB?",
    ]);
  });
});

describe("WriteTextCommand", () => {
  it("throws instead of showing an unresolved text label", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const shownTexts: string[] = [];

    eventManager.on("show_text", (event) => {
      shownTexts.push(String(event.data?.text ?? ""));
    });

    const command = new WriteTextCommand("MissingTextLabel");
    command.runner = {
      dataLoader: {
        getText: () => null,
      },
    } as never;

    expect(() => command.execute(gameState, eventManager, {} as never)).toThrow(
      "Missing text for label 'MissingTextLabel'."
    );
    expect(shownTexts).toEqual([]);
  });
});

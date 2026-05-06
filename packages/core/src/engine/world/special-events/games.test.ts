import { createInitialGameState } from "@pokecrystal/core/core/state";
import { MAX_COINS } from "@pokecrystal/core/core/constants";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { createScriptRunnerStub } from "@pokecrystal/core/engine/world/story-events/test-utils";
import { slot_machine_special, slot_machine_ui_special, card_flip_special } from "./games";
import { SlotMachine, SlotMachineMode } from "@pokecrystal/core/engine/games/slots";
import { CardFlipGame } from "@pokecrystal/core/engine/games/card-flip";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { SPECIAL_FUNCTIONS } from "./registry";

jest.mock("@pokecrystal/core/engine/games/slots", () => {
  const actual = jest.requireActual("@pokecrystal/core/engine/games/slots");
  return {
    ...actual,
    SlotMachine: jest.fn().mockImplementation(() => ({
      spin: jest.fn(() => ({
        windows: [
          [actual.SlotSymbol.SEVEN, actual.SlotSymbol.CHERRY, actual.SlotSymbol.STARYU],
          [actual.SlotSymbol.SEVEN, actual.SlotSymbol.PIKACHU, actual.SlotSymbol.CHERRY],
          [actual.SlotSymbol.SEVEN, actual.SlotSymbol.PIKACHU, actual.SlotSymbol.CHERRY],
        ],
        matchedSymbol: actual.SlotSymbol.SEVEN,
        winningLines: ["middle"],
        payout: 300,
      })),
    })),
  };
});

jest.mock("@pokecrystal/core/engine/games/card-flip", () => {
  return {
    CardFlipGame: jest.fn().mockImplementation(() => ({
      deck: Array(24).fill("PIKACHU"),
      revealed: Array(24).fill(false),
      shuffle: jest.fn(),
      flip: jest.fn(() => ({ cardIndex: 0, cardName: "PIKACHU", payout: 0 })),
    })),
  };
});

const captureTexts = (eventManager: EventManager): string[] => {
  const messages: string[] = [];
  eventManager.on("show_text", (event) => {
    messages.push(String(event.data.text ?? ""));
  });
  return messages;
};

describe("Game Corner specials", () => {
  beforeEach(() => {
    (SlotMachine as jest.Mock).mockClear();
    (CardFlipGame as jest.Mock).mockClear();
  });

  it("blocks slot machine when the player has no coins", () => {
    const gameState = createInitialGameState();
    gameState.sram.coins = 0;
    gameState.sram.key_items = { COIN_CASE: 1 };
    const runner = createScriptRunnerStub({ game_state: gameState });
    const messages = captureTexts(runner.event_manager);

    const outcome = slot_machine_special({
      game_state: gameState,
      runner,
      event_manager: runner.event_manager,
    });

    expect(outcome).toEqual({ played: false, reason: "no_coins" });
    expect(messages).toEqual(["You have no coins."]);
    expect(SlotMachine).not.toHaveBeenCalled();
  });

  it("blocks slot machine when the player lacks a coin case", () => {
    const gameState = createInitialGameState();
    gameState.sram.coins = 50;
    gameState.sram.key_items = {};
    const runner = createScriptRunnerStub({ game_state: gameState });
    const messages = captureTexts(runner.event_manager);

    const outcome = slot_machine_special({
      game_state: gameState,
      runner,
      event_manager: runner.event_manager,
    });

    expect(outcome).toEqual({ played: false, reason: "no_coin_case" });
    expect(messages).toEqual(["You don't have a\nCOIN CASE."]);
    expect(SlotMachine).not.toHaveBeenCalled();
  });

  it("uses the script value to select lucky slot mode", () => {
    const gameState = createInitialGameState();
    gameState.sram.coins = 100;
    gameState.sram.key_items = { COIN_CASE: 1 };
    const runner = createScriptRunnerStub({
      game_state: gameState,
      variables: { _value: "TRUE" },
    });

    const outcome = slot_machine_special({
      game_state: gameState,
      runner,
      event_manager: runner.event_manager,
    });

    expect(outcome.played).toBe(true);
    const instance = (SlotMachine as jest.Mock).mock.results[0]?.value as { spin: jest.Mock };
    const spinArgs = instance.spin.mock.calls[0][0];
    expect(spinArgs.mode).toBe(SlotMachineMode.LUCKY);
  });

  it("caps slot machine coin payouts at MAX_COINS", () => {
    const gameState = createInitialGameState();
    gameState.sram.coins = MAX_COINS;
    gameState.sram.key_items = { COIN_CASE: 1 };
    const runner = createScriptRunnerStub({ game_state: gameState });

    const outcome = slot_machine_special({
      game_state: gameState,
      runner,
      event_manager: runner.event_manager,
    });

    expect(outcome.played).toBe(true);
    if (outcome.played) {
      expect(outcome.coins).toBe(MAX_COINS);
    }
    expect(gameState.sram.coins).toBe(MAX_COINS);
  });

  it("opens a live slot-machine overlay that consumes A to spin and B to quit", async () => {
    const gameState = createInitialGameState();
    gameState.sram.coins = 100;
    gameState.sram.key_items = { COIN_CASE: 1 };
    const runner = createScriptRunnerStub({ game_state: gameState });
    const eventQueue = gameEngine.event.createQueue();
    const renderSnapshot = jest.fn();
    const overworld = {
      ui: {
        eventQueue,
        renderSnapshot,
        update: jest.fn(),
      },
      input_capture_active: false,
    };

    gameEngine.event.post(
      new gameEngine.event.Event("keydown", { button: "a", is_press: true }),
      eventQueue,
    );
    gameEngine.event.post(
      new gameEngine.event.Event("keydown", { button: "b", is_press: true }),
      eventQueue,
    );

    const outcome = await slot_machine_ui_special({
      game_state: gameState,
      runner,
      overworld,
      event_manager: runner.event_manager,
    });

    expect(outcome).toEqual(
      expect.objectContaining({
        played: true,
        bet: 3,
        payout: 300,
        coins: 397,
      }),
    );
    expect(gameState.sram.coins).toBe(397);
    expect(runner.last_value).toEqual(outcome);
    expect(overworld.input_capture_active).toBe(false);
    expect(renderSnapshot).toHaveBeenCalledWith(
      expect.arrayContaining(["SLOT MACHINE"]),
      expect.arrayContaining(["STATE: slot_machine"]),
      "Slot Machine",
      "Legend",
      null,
      null,
      null,
    );
  });

  it("routes the SlotMachine special registry entry through the live overlay when TUI events exist", async () => {
    const gameState = createInitialGameState();
    gameState.sram.coins = 100;
    gameState.sram.key_items = { COIN_CASE: 1 };
    const runner = createScriptRunnerStub({
      game_state: gameState,
      variables: { _value: "FALSE" },
    });
    const eventQueue = gameEngine.event.createQueue();
    const overworld = {
      ui: {
        eventQueue,
        renderSnapshot: jest.fn(),
        update: jest.fn(),
      },
      input_capture_active: false,
    };

    gameEngine.event.post(
      new gameEngine.event.Event("keydown", { button: "a", is_press: true }),
      eventQueue,
    );
    gameEngine.event.post(
      new gameEngine.event.Event("keydown", { button: "b", is_press: true }),
      eventQueue,
    );

    const result = SPECIAL_FUNCTIONS.SlotMachine({
      game_state: gameState,
      runner,
      overworld,
      event_manager: runner.event_manager,
    });

    await expect(result).resolves.toEqual(
      expect.objectContaining({
        played: true,
        coins: 397,
      }),
    );
    expect(overworld.ui.renderSnapshot).toHaveBeenCalledWith(
      expect.arrayContaining(["SLOT MACHINE"]),
      expect.arrayContaining(["STATE: slot_machine"]),
      "Slot Machine",
      "Legend",
      null,
      null,
      null,
    );
  });

  it("blocks card flip when the player lacks a coin case", () => {
    const gameState = createInitialGameState();
    gameState.sram.coins = 5;
    gameState.sram.key_items = {};
    const runner = createScriptRunnerStub({ game_state: gameState });
    const messages = captureTexts(runner.event_manager);

    const outcome = card_flip_special({
      game_state: gameState,
      runner,
      event_manager: runner.event_manager,
    });

    expect(outcome).toEqual({ played: false, reason: "no_coin_case" });
    expect(messages).toEqual(["You don't have a\nCOIN CASE."]);
    expect(CardFlipGame).not.toHaveBeenCalled();
  });

  it("blocks card flip when the player has fewer than three coins", () => {
    const gameState = createInitialGameState();
    gameState.sram.coins = 2;
    gameState.sram.key_items = { COIN_CASE: 1 };
    const runner = createScriptRunnerStub({ game_state: gameState });

    const outcome = card_flip_special({
      game_state: gameState,
      runner,
      event_manager: runner.event_manager,
    });

    expect(outcome).toEqual({ played: false, reason: "no_coins" });
    expect(CardFlipGame).not.toHaveBeenCalled();
  });

  it("deducts three coins before applying card flip payout", () => {
    const gameState = createInitialGameState();
    gameState.sram.coins = 5;
    gameState.sram.key_items = { COIN_CASE: 1 };
    const runner = createScriptRunnerStub({ game_state: gameState });

    const outcome = card_flip_special({
      game_state: gameState,
      runner,
      event_manager: runner.event_manager,
    });

    expect(outcome.played).toBe(true);
    if (outcome.played) {
      expect(outcome.coins).toBe(2);
    }
    expect(gameState.sram.coins).toBe(2);
  });

  it("caps card flip coin payouts at MAX_COINS", () => {
    (CardFlipGame as jest.Mock).mockImplementation(() => ({
      deck: Array(24).fill("PIKACHU"),
      revealed: Array(24).fill(false),
      shuffle: jest.fn(),
      flip: jest.fn(() => ({ cardIndex: 0, cardName: "PIKACHU", payout: 72 })),
    }));

    const gameState = createInitialGameState();
    gameState.sram.coins = MAX_COINS;
    gameState.sram.key_items = { COIN_CASE: 1 };
    const runner = createScriptRunnerStub({ game_state: gameState });

    const outcome = card_flip_special({
      game_state: gameState,
      runner,
      event_manager: runner.event_manager,
    });

    expect(outcome.played).toBe(true);
    if (outcome.played) {
      expect(outcome.coins).toBe(MAX_COINS);
    }
    expect(gameState.sram.coins).toBe(MAX_COINS);
  });
});

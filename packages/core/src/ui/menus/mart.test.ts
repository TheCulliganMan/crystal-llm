import { MartInterface } from "./mart";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { ItemSystem } from "@pokecrystal/core/engine/systems/items";
import { formatPrice, type MartItem } from "@pokecrystal/core/engine/systems/shop";
import { Surface } from "@pokecrystal/core/ui/surface";
import { KEYS } from "@pokecrystal/core/core/keycodes";

const createUi = () => {
  return {
    screen: new Surface(160, 144),
    screenHeight: 144,
    drawWindow: jest.fn(),
    font: {
      renderText: jest.fn(),
    },
    update: jest.fn(),
    renderSnapshot: jest.fn(),
    tileSize: 8,
  };
};

describe("MartInterface tile layout", () => {
  it("renders buy prices on the second row for each item", () => {
    const ui = createUi();
    const gameState = createInitialGameState();
    const dataLoader = new DataLoader();
    const itemSystem = new ItemSystem(gameState, dataLoader);
    const mart = new MartInterface({ ui }, gameState, dataLoader, itemSystem);
    const items: MartItem[] = [
      { identifier: "POTION", displayName: "POTION", price: 300 },
      { identifier: "CANCEL", displayName: "CANCEL", price: 0 },
    ];

    (mart as unknown as { drawItemList: Function }).drawItemList(ui, items, 0, 0, "buy");

    const calls = ui.font.renderText.mock.calls;
    const priceText = formatPrice(300);
    const priceCall = calls.find(([text]) => text === priceText);
    expect(priceCall).toBeDefined();

    const nameCall = calls.find(([text]) => typeof text === "string" && text.includes("POTION"));
    expect(nameCall).toBeDefined();
    expect(priceCall[2]).toBe(nameCall[2] + ui.tileSize);
  });

  it("draws the money box without a MONEY label", () => {
    const ui = createUi();
    const gameState = createInitialGameState();
    gameState.sram.money = 1234;
    const dataLoader = new DataLoader();
    const itemSystem = new ItemSystem(gameState, dataLoader);
    const mart = new MartInterface({ ui }, gameState, dataLoader, itemSystem);

    (mart as unknown as { drawMoney: Function }).drawMoney(ui);

    const calls = ui.font.renderText.mock.calls;
    const moneyText = formatPrice(1234);
    expect(calls.some(([text]) => text === moneyText)).toBe(true);
    expect(calls.some(([text]) => String(text).includes("MONEY"))).toBe(false);
  });
});

describe("MartInterface text overlay", () => {
  it("emits a snapshot for the top menu", async () => {
    const ui = createUi();
    const gameState = createInitialGameState();
    const dataLoader = new DataLoader();
    dataLoader.martData = new Map([["TEST_MART", ["POTION"]]]);
    const itemSystem = new ItemSystem(gameState, dataLoader);
    const events = [
      { type: KEYS.KEYDOWN, key: KEYS.Z, is_press: true },
      { type: KEYS.KEYDOWN, key: KEYS.X, is_press: true },
      { type: KEYS.KEYDOWN, key: KEYS.Z, is_press: true },
    ];
    const overworld = {
      ui,
      draw: jest.fn(),
      pollEvents: () => {
        const next = events.shift();
        return next ? [next] : [];
      },
    };
    const mart = new MartInterface(overworld, gameState, dataLoader, itemSystem);

    await mart.openAsync("MARTTYPE_STANDARD", "TEST_MART");

    const menuSnapshots = ui.renderSnapshot.mock.calls
      .map((call) => call[4])
      .filter((lines) => Array.isArray(lines)) as string[][];

    expect(menuSnapshots.some((lines) => lines.some((line) => line.includes("BUY")))).toBe(true);
  });
});

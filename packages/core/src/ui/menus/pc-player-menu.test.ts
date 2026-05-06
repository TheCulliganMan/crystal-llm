import { createInitialGameState } from "@pokecrystal/core/core/state";
import { Surface } from "@pokecrystal/core/ui/surface";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import { BitmapFont } from "@pokecrystal/core/ui/text/bitmap-font";
import { _CHAR_MAP } from "@pokecrystal/core/ui/tilemap-surface";
import type { KeyEvent } from "@pokecrystal/core/input/buttons";
import type { PlayerPCUI } from "./pc-player-menu";
import { PlayerPCMenu } from "./pc-player-menu";

const REVERSE_CHAR_MAP = new Map<number, string>(
  Object.entries(_CHAR_MAP).map(([char, tile]) => [tile, char]),
);

const makeUi = async (): Promise<PlayerPCUI> => {
  const font = new BitmapFont();
  await font.load();
  return {
    screen: new Surface(160, 144),
    drawWindow: jest.fn(),
    font,
    update: jest.fn(),
  };
};

const keyEvent = (button: string): KeyEvent => ({
  type: "keydown",
  key: button,
  code: button,
  is_press: true,
});

const tileLine = (
  tilemap: ReturnType<PlayerPCMenu["renderTilemap"]>,
  y: number,
  x = 0,
  width = 20,
): string =>
  Array.from({ length: width }, (_unused, offset) =>
    REVERSE_CHAR_MAP.get(tilemap.getTile(x + offset, y)) ?? "?",
  ).join("");

describe("PlayerPCMenu", () => {
  it("uses the ASM Pokecenter action set including mail box and log off", async () => {
    const ui = await makeUi();
    const gameState = createInitialGameState();
    const menu = new PlayerPCMenu(ui, gameState);

    expect((menu as unknown as { menuActions: readonly string[] }).menuActions).toEqual([
      "WITHDRAW ITEM",
      "DEPOSIT ITEM",
      "TOSS ITEM",
      "MAIL BOX",
      "LOG OFF",
    ]);
  });

  it("returns a mail_box action when MAIL BOX is selected from the top menu", async () => {
    const ui = await makeUi();
    const gameState = createInitialGameState();
    const menu = new PlayerPCMenu(ui, gameState);
    const down = keyEvent("ArrowDown");
    const confirm = keyEvent("KeyZ");

    menu.handleInput(down);
    menu.handleInput(down);
    menu.handleInput(down);
    const result = menu.handleInput(confirm);

    expect(result).toEqual({ action: "mail_box", status: "ok" });
  });

  it("treats LOG OFF as exiting the top menu", async () => {
    const ui = await makeUi();
    const gameState = createInitialGameState();
    const menu = new PlayerPCMenu(ui, gameState);
    const down = keyEvent("ArrowDown");
    const confirm = keyEvent("KeyZ");

    menu.handleInput(down);
    menu.handleInput(down);
    menu.handleInput(down);
    menu.handleInput(down);
    const result = menu.handleInput(confirm);

    expect(result).toBe("cancel");
  });

  it("draws Player's PC on TextUI without bitmap tile assets", () => {
    const textUi = new TextUI(160, 144, 1, false, false, 0, true);
    const ui: PlayerPCUI = {
      screen: textUi.screen,
      drawWindow: textUi.drawWindow.bind(textUi),
      font: textUi.font as PlayerPCUI["font"],
      update: textUi.update.bind(textUi),
      renderSnapshot: textUi.renderSnapshot.bind(textUi),
    };
    const gameState = createInitialGameState();
    const menu = new PlayerPCMenu(ui, gameState);

    expect(() => menu.draw()).not.toThrow();
    expect(textUi.getSnapshot()?.viewportTitle).toBe("Player's PC");
    expect(textUi.getSnapshot()?.menuLines).toEqual(
      expect.arrayContaining(["▶ WITHDRAW ITEM", "  LOG OFF"]),
    );
  });

  it("renders the ASM Player PC item storage textbox and 4-row scrolling menu", async () => {
    const ui = await makeUi();
    const gameState = createInitialGameState();
    gameState.sram.pc_items = [
      { item: "ANTIDOTE", quantity: 1 },
      { item: "AWAKENING", quantity: 2 },
      { item: "BASEMENT_KEY", quantity: 1 },
      { item: "BICYCLE", quantity: 1 },
      { item: "BITTER_BERRY", quantity: 7 },
    ] as never;
    const menu = new PlayerPCMenu(ui, gameState);

    menu.jumpToAction("WITHDRAW ITEM", { openList: true });
    const tilemap = menu.renderTilemap();

    expect(tilemap.getTile(0, 0)).toBe(_CHAR_MAP["┌"]);
    expect(tilemap.getTile(19, 11)).toBe(_CHAR_MAP["┘"]);
    expect(tilemap.getTile(0, 12)).toBe(_CHAR_MAP["┌"]);
    expect(tilemap.getTile(19, 17)).toBe(_CHAR_MAP["┘"]);
    expect(tileLine(tilemap, 0)).not.toContain("PLAYER");
    expect(tileLine(tilemap, 2, 4, 12)).toBe(">ANTIDOTE×01");
    expect(tileLine(tilemap, 4, 4, 12)).toBe(" AWAKENIN×02");
    expect(tileLine(tilemap, 6, 4, 12)).toBe(" BASEMENT×01");
    expect(tileLine(tilemap, 8, 4, 12)).toBe(" BICYCLE ×01");
    expect(tileLine(tilemap, 10)).not.toContain("BITTER");
  });

  it("scrolls Player PC items in ASM-sized 4-row pages", async () => {
    const ui = await makeUi();
    const gameState = createInitialGameState();
    gameState.sram.pc_items = [
      { item: "ANTIDOTE", quantity: 1 },
      { item: "AWAKENING", quantity: 2 },
      { item: "BASEMENT_KEY", quantity: 1 },
      { item: "BICYCLE", quantity: 1 },
      { item: "BITTER_BERRY", quantity: 7 },
    ] as never;
    const menu = new PlayerPCMenu(ui, gameState);

    menu.jumpToAction("WITHDRAW ITEM", { openList: true });
    menu.handleInput(keyEvent("ArrowDown"));
    menu.handleInput(keyEvent("ArrowDown"));
    menu.handleInput(keyEvent("ArrowDown"));
    menu.handleInput(keyEvent("ArrowDown"));
    const tilemap = menu.renderTilemap();

    expect(tileLine(tilemap, 2, 4, 12)).toBe(" AWAKENIN×02");
    expect(tileLine(tilemap, 8, 4, 12)).toBe(">BITTER B×07");
  });
});

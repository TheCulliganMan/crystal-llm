import { createInitialGameState } from "@pokecrystal/core/core/state";
import { Surface } from "@pokecrystal/core/ui/surface";
import { setBooleanFlag } from "@pokecrystal/core/engine/world/overworld/flag-collection";
import type { FontRenderer } from "./types";
import { TrainerCardScreen } from "./trainer-card";

const stubFont = {
  renderText: jest.fn(),
} as unknown as FontRenderer;

const readJohtoFlags = (screen: TrainerCardScreen): boolean[] =>
  (screen as unknown as { johtoBadgeFlags: () => boolean[] }).johtoBadgeFlags();

const readKantoFlags = (screen: TrainerCardScreen): boolean[] =>
  (screen as unknown as { kantoBadgeFlags: () => boolean[] }).kantoBadgeFlags();

const keyDown = (code: string) => ({ type: "keydown", code });

describe("TrainerCardScreen badge flags", () => {
  it("reads Johto badge bits from SRAM in ASM order", () => {
    const gameState = createInitialGameState();
    gameState.sram.badges.johto[0] = true;
    gameState.sram.badges.johto[7] = true;
    const screen = new TrainerCardScreen({ screen: new Surface(160, 144), font: stubFont }, gameState);

    const flags = readJohtoFlags(screen);
    expect(flags[0]).toBe(true);
    expect(flags[7]).toBe(true);
    expect(flags[1]).toBe(false);
  });

  it("falls back to ENGINE_* badge flags when SRAM bit is false", () => {
    const gameState = createInitialGameState();
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_HIVEBADGE", true);
    const screen = new TrainerCardScreen({ screen: new Surface(160, 144), font: stubFont }, gameState);

    const flags = readJohtoFlags(screen);
    expect(flags[1]).toBe(true);
  });

  it("throws for non-ASM Johto badge banks", () => {
    const gameState = createInitialGameState();
    gameState.sram.badges.johto = [true] as boolean[];
    const screen = new TrainerCardScreen({ screen: new Surface(160, 144), font: stubFont }, gameState);

    expect(() => readJohtoFlags(screen)).toThrow("must contain exactly 8");
  });

  it("reads Kanto badge bits from SRAM in ASM order", () => {
    const gameState = createInitialGameState();
    gameState.sram.badges.kanto[0] = true;
    gameState.sram.badges.kanto[7] = true;
    const screen = new TrainerCardScreen({ screen: new Surface(160, 144), font: stubFont }, gameState);

    const flags = readKantoFlags(screen);
    expect(flags[0]).toBe(true);
    expect(flags[7]).toBe(true);
    expect(flags[1]).toBe(false);
  });

  it("falls back to Kanto ENGINE_* badge flags when SRAM bit is false", () => {
    const gameState = createInitialGameState();
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_CASCADEBADGE", true);
    const screen = new TrainerCardScreen({ screen: new Surface(160, 144), font: stubFont }, gameState);

    const flags = readKantoFlags(screen);
    expect(flags[1]).toBe(true);
  });

  it("lets right advance from Johto to Kanto badges after earning a Kanto badge", () => {
    const gameState = createInitialGameState();
    gameState.sram.badges.kanto[3] = true;
    const screen = new TrainerCardScreen({ screen: new Surface(160, 144), font: stubFont }, gameState);

    screen.handleInput(keyDown("ArrowRight"));
    expect(screen.getActivePage()).toBe("johto_badges");

    screen.handleInput(keyDown("ArrowRight"));
    expect(screen.getActivePage()).toBe("kanto_badges");

    screen.handleInput(keyDown("ArrowLeft"));
    expect(screen.getActivePage()).toBe("johto_badges");
  });

  it("keeps right on Johto badges when no Kanto badge is owned", () => {
    const gameState = createInitialGameState();
    const screen = new TrainerCardScreen({ screen: new Surface(160, 144), font: stubFont }, gameState);

    screen.handleInput(keyDown("ArrowRight"));
    screen.handleInput(keyDown("ArrowRight"));

    expect(screen.getActivePage()).toBe("johto_badges");
  });
});

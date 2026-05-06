import { createInitialGameState } from "@pokecrystal/core/core/state";
import { decorations } from "@pokecrystal/assets/content/decorations";
import { toggle_decorations_visibility, toggle_maptile_decorations } from "./decorations";

const spriteValueFor = (id: number): number => {
  const entry = decorations.find((deco) => deco.index === id);
  if (!entry || typeof entry.sprite_value !== "number") {
    throw new Error(`Missing sprite_value for decoration ${id}`);
  }
  return entry.sprite_value;
};

const spriteTokenFor = (id: number): string => {
  const entry = decorations.find((deco) => deco.index === id);
  if (!entry) {
    throw new Error(`Missing decoration ${id}`);
  }
  return String(entry.sprite_token);
};

type MaptileWriter = {
  _write_metatile: (x: number, y: number, block: number) => void;
  refresh_event_flag?: jest.Mock;
};

type RunnerWithOverworld = {
  overworld: MaptileWriter;
};

describe("ToggleMaptileDecorations", () => {
  it("writes maptile decoration blocks using padded coordinates", () => {
    const gameState = createInitialGameState();
    gameState.wram.wDecoBed = 2;
    gameState.wram.wDecoPlant = 12;
    gameState.wram.wDecoPoster = 16;
    gameState.wram.wDecoCarpet = 7;

    const calls: Array<[number, number, number]> = [];
    const overworld: MaptileWriter = {
      _write_metatile: (x: number, y: number, block: number) => {
        calls.push([x, y, block]);
      },
      refresh_event_flag: jest.fn(),
    };

    toggle_maptile_decorations(gameState, { overworld });

    expect(calls).toEqual([
      [0, 2, spriteValueFor(2)],
      [3, 2, spriteValueFor(12)],
      [3, 0, spriteValueFor(16)],
      [0, 0, spriteValueFor(7)],
      [0, 1, spriteValueFor(7) + 1],
      [0, 1, spriteValueFor(7) + 2],
      [1, 1, spriteValueFor(7) + 1],
    ]);
    expect(gameState.wram.event_flags.EVENT_PLAYERS_ROOM_POSTER).toBe(false);
  });

  it("falls back to runner overworld when the command context lacks a writer", () => {
    const gameState = createInitialGameState();
    gameState.wram.wDecoBed = 2;
    gameState.wram.wDecoPlant = 0;
    gameState.wram.wDecoPoster = 0;
    gameState.wram.wDecoCarpet = 0;

    const calls: Array<[number, number, number]> = [];
    const runner: RunnerWithOverworld = {
      overworld: {
        _write_metatile: (x: number, y: number, block: number) => {
          calls.push([x, y, block]);
        },
      },
    };

    toggle_maptile_decorations(gameState, { runner, overworld: {} });

    expect(calls).toEqual([[0, 2, spriteValueFor(2)]]);
  });

  it("binds the overworld when using a prototype writer", () => {
    const gameState = createInitialGameState();
    gameState.wram.wDecoBed = 2;
    gameState.wram.wDecoPlant = 0;
    gameState.wram.wDecoPoster = 0;
    gameState.wram.wDecoCarpet = 0;

    class OverworldStub {
      public map = { metatileIds: [] as number[] };
      public tileset: Record<string, unknown> = {};
      public _write_metatile(this: OverworldStub, x: number, y: number, block: number) {
        return [this.map, this.tileset, x, y, block];
      }
    }
    const overworld = new OverworldStub();

    expect(() => toggle_maptile_decorations(gameState, { overworld })).not.toThrow();
  });
});

describe("ToggleDecorationsVisibility", () => {
  it("updates variable sprites and event flags for decoration objects", () => {
    const gameState = createInitialGameState();
    gameState.wram.wDecoConsole = 21;
    gameState.wram.wDecoLeftOrnament = 30;
    gameState.wram.wDecoRightOrnament = 0;
    gameState.wram.wDecoBigDoll = 26;

    const overworld = { refresh_event_flag: jest.fn() };

    toggle_decorations_visibility(gameState, { overworld });

    expect(gameState.wram.variable_sprites.SPRITE_CONSOLE).toBe(spriteTokenFor(21));
    expect(gameState.wram.variable_sprites.SPRITE_DOLL_1).toBe(spriteTokenFor(30));
    expect(gameState.wram.variable_sprites.SPRITE_DOLL_2).toBeUndefined();
    expect(gameState.wram.variable_sprites.SPRITE_BIG_DOLL).toBe(spriteTokenFor(26));
    expect(gameState.wram.event_flags.EVENT_PLAYERS_HOUSE_2F_CONSOLE).toBe(false);
    expect(gameState.wram.event_flags.EVENT_PLAYERS_HOUSE_2F_DOLL_1).toBe(false);
    expect(gameState.wram.event_flags.EVENT_PLAYERS_HOUSE_2F_DOLL_2).toBe(true);
    expect(gameState.wram.event_flags.EVENT_PLAYERS_HOUSE_2F_BIG_DOLL).toBe(false);
  });
});

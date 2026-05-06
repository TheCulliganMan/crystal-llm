jest.mock("@pokecrystal/core/ui/overlays/battle-ui", () => ({
  create_battle_ui: jest.fn(() => ({ active: false })),
  set_audio_engine: jest.fn(),
}));

import { Game } from "./game";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import type { BaseFontRenderer } from "@pokecrystal/core/ui/base-ui";
import type { Surface } from "@pokecrystal/core/ui/surface";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";

type MockFontRenderer = BaseFontRenderer & {
  font_tiles?: Record<number, InstanceType<typeof gameEngine.Surface>>;
};

type MockTextUI = TextUI & {
  tile_size?: number;
  font: MockFontRenderer;
};

interface TilesetInstance {
  tilesetName: string;
  metatiles: Array<{ collision: number[] }>;
  renderMetatile(): void;
  renderPriorityMetatile(): void;
}

type TilesetConstructor = new (tilesetName: string) => TilesetInstance;

type GlobalOverrides = {
  fetch?: typeof globalThis.fetch | undefined;
  createImageBitmap?: typeof globalThis.createImageBitmap | undefined;
  Tileset?: TilesetConstructor;
};

type GameInternals = {
  handleInput: () => void;
};

type TestOverworld = OverworldEngine & {
  ui: {
    eventQueue?: GameEngineEventQueue;
  };
};

const getInternals = (target: Game): GameInternals =>
  target as unknown as GameInternals;

const invokeHandleInput = (target: Game): void => {
  getInternals(target).handleInput();
};

const getEventQueue = (overworld: TestOverworld): GameEngineEventQueue => {
  const queue = overworld.ui.eventQueue;
  if (!queue) {
    throw new Error("Overworld UI event queue is missing.");
  }
  return queue;
};

const buildGame = async (): Promise<Game> => {
  const ui = new TextUI(160, 144, 1, null, false, 0) as MockTextUI;
  const fontTiles: Record<number, InstanceType<typeof gameEngine.Surface>> = {};
  for (let i = 0; i < 256; i += 1) {
    fontTiles[i] = new gameEngine.Surface(8, 8);
  }
  ui.tile_size = 8;
  const fontRenderer = ui.font;
  fontRenderer.font_tiles = fontTiles as unknown as Record<number, Surface>;
  const noopRender: (..._args: Parameters<NonNullable<BaseFontRenderer["renderText"]>>) => void = () => {};
  fontRenderer.render_text = noopRender;
  fontRenderer.renderText = noopRender;

  const globalScope = globalThis as GlobalOverrides;
  const originalFetch = globalScope.fetch;
  const originalCreateImageBitmap = globalScope.createImageBitmap;
  const originalTileset = globalScope.Tileset;
  const originalImageLoad = gameEngine.image.load;
  const originalInitAssets = OverworldEngine.prototype.init_assets;

  class TilesetStub implements TilesetInstance {
    public tilesetName: string;
    public metatiles: Array<{ collision: number[] }>;

    constructor(tilesetName: string) {
      this.tilesetName = tilesetName || "placeholder";
      this.metatiles = Array.from({ length: 256 }, () => ({ collision: [0, 0, 0, 0] }));
    }

    renderMetatile(): void {
      // No-op for headless test coverage.
    }

    renderPriorityMetatile(): void {
      // No-op for headless test coverage.
    }
  }

  globalScope.fetch = undefined;
  globalScope.createImageBitmap = undefined;
  globalScope.Tileset = TilesetStub;
  gameEngine.image.load = async () => new gameEngine.Surface(24, 16);
  OverworldEngine.prototype.init_assets = async () => {};

  try {
    return await Game.create(ui);
  } finally {
    globalScope.fetch = originalFetch;
    globalScope.createImageBitmap = originalCreateImageBitmap;
    globalScope.Tileset = originalTileset;
    gameEngine.image.load = originalImageLoad;
    OverworldEngine.prototype.init_assets = originalInitAssets;
  }
};

describe("Game start menu locks", () => {
  it("blocks the start menu while dialogue is active", async () => {
    const game = await buildGame();
    const overworld = game.getOverworld() as TestOverworld;
    getEventQueue(overworld);

    if (overworld.dialogue) {
      overworld.dialogue.visible = true;
    }

    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_RETURN }));
    invokeHandleInput(game);

    expect(game.getState()).toBe("overworld");
  });

  it("blocks the start menu while the script runner is busy", async () => {
    const game = await buildGame();
    const overworld = game.getOverworld() as TestOverworld;
    getEventQueue(overworld);

    overworld.script_runner = { is_busy: true } as unknown as TestOverworld["script_runner"];

    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_RETURN }));
    invokeHandleInput(game);

    expect(game.getState()).toBe("overworld");
  });

  it("reports menu open when the script runner is busy", async () => {
    const game = await buildGame();
    const overworld = game.getOverworld() as TestOverworld;

    overworld.script_runner = { is_busy: true } as unknown as TestOverworld["script_runner"];

    expect(game.isMenuOpen()).toBe(true);
  });

  it("blocks the start menu while player movement is locked", async () => {
    const game = await buildGame();
    const overworld = game.getOverworld() as TestOverworld;
    getEventQueue(overworld);

    overworld.lock_player_movement();

    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_RETURN }));
    invokeHandleInput(game);

    expect(game.getState()).toBe("overworld");
  });

  it("reopens the start menu from a fresh cursor state after closing", async () => {
    const game = await buildGame();

    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_RETURN }));
    invokeHandleInput(game);
    expect(game.getState()).toBe("menu");

    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_DOWN }));
    invokeHandleInput(game);

    const menuState = game as unknown as {
      menuState: {
        startMenu: { cursorIndex: number };
      };
      gameState: {
        wram: { start_menu_cursor: number };
      };
    };
    expect(menuState.menuState.startMenu.cursorIndex).toBe(1);
    expect(menuState.gameState.wram.start_menu_cursor).toBe(1);

    game.postEvent(new gameEngine.event.Event("keydown", { key: "x", code: "KeyX" }));
    invokeHandleInput(game);
    expect(game.getState()).toBe("overworld");

    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_RETURN }));
    invokeHandleInput(game);

    expect(game.getState()).toBe("menu");
    expect(menuState.menuState.startMenu.cursorIndex).toBe(0);
    expect(menuState.gameState.wram.start_menu_cursor).toBe(0);
  });
});

jest.mock("@pokecrystal/core/ui/overlays/battle-ui", () => ({
  create_battle_ui: jest.fn(() => ({ active: false })),
  set_audio_engine: jest.fn(),
}));

jest.mock("@pokecrystal/core/ui/screens/title-screen", () => ({
  TitleScreen: {
    create: jest.fn(async () => ({
      startFromGameStart: jest.fn(),
      update: jest.fn(),
      popAction: jest.fn(() => null),
      draw: jest.fn(),
    })),
  },
  TitleScreenOption: {
    MAIN_MENU: "main_menu",
    DELETE_SAVE_DATA: "delete_save_data",
    RESET_CLOCK: "reset_clock",
    RESTART: "restart_intro",
  },
}));

jest.mock("@pokecrystal/core/core/save", () => {
  class SaveFileNotFoundError extends Error {}
  return {
    saveGame: jest.fn().mockResolvedValue(true),
    saveGameWithHistory: jest.fn().mockResolvedValue(true),
    loadGame: jest.fn(),
    hasSaveGame: jest.fn().mockResolvedValue(false),
    SaveFileNotFoundError,
  };
});

import { Game } from "./game";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { BaseFontRenderer } from "@pokecrystal/core/ui/base-ui";
import type { Surface } from "@pokecrystal/core/ui/surface";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { saveGame } from "@pokecrystal/core/core/save";
import { AUTOSAVE_SLOT, MANUAL_SAVE_SLOT } from "@pokecrystal/core/core/save-slots";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import { hasSaveGame } from "@pokecrystal/core/core/save";

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
  currentState: string;
  fatalError?: Error | null;
  quitRequested?: boolean;
  hasPersistedSaveData?: boolean;
};

const getInternals = (target: Game): GameInternals =>
  target as unknown as GameInternals;

const invokeHandleInput = (target: Game): void => {
  getInternals(target).handleInput();
};

const flushPromises = async (): Promise<void> => {
  await Promise.resolve();
  await Promise.resolve();
};

const buildGame = async (options: Parameters<typeof Game.create>[1] = {}): Promise<Game> => {
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

    renderMetatile(): void {}

    renderPriorityMetatile(): void {}
  }

  globalScope.fetch = undefined;
  globalScope.createImageBitmap = undefined;
  globalScope.Tileset = TilesetStub;
  gameEngine.image.load = async () => new gameEngine.Surface(24, 16);
  OverworldEngine.prototype.init_assets = async () => {};

  try {
    return await Game.create(ui, options);
  } finally {
    globalScope.fetch = originalFetch;
    globalScope.createImageBitmap = originalCreateImageBitmap;
    globalScope.Tileset = originalTileset;
    gameEngine.image.load = originalImageLoad;
    OverworldEngine.prototype.init_assets = originalInitAssets;
  }
};

describe("Game save flow", () => {
  it("invokes saveGame when the menu save action is selected", async () => {
    const game = await buildGame();
    const internals = getInternals(game);
    internals.currentState = "menu";

    const menuState = {
      handleInput: jest.fn(() => "save"),
      beginSaveFlow: jest.fn(),
    };
    (game as unknown as { menuState: typeof menuState }).menuState = menuState;

    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_RETURN }));
    invokeHandleInput(game);

    expect(menuState.beginSaveFlow).toHaveBeenCalled();
    expect(saveGame).not.toHaveBeenCalled();
    expect(game.getState()).toBe("menu");
  });

  it("autosaves after 250 player steps", async () => {
    const game = await buildGame();
    const eventManager = (game as unknown as { eventManager: EventManager }).eventManager;
    const saveGameMock = saveGame as jest.Mock;
    saveGameMock.mockClear();

    for (let i = 0; i < 249; i += 1) {
      eventManager.dispatch(new Event("player_step", { count: 1 }));
    }
    expect(saveGameMock).not.toHaveBeenCalled();

    eventManager.dispatch(new Event("player_step", { count: 1 }));
    expect(saveGameMock).toHaveBeenCalledWith(game.getGameState(), AUTOSAVE_SLOT);
  });

  it("autosaves after battle completion", async () => {
    const game = await buildGame();
    const eventManager = (game as unknown as { eventManager: EventManager }).eventManager;
    const saveGameMock = saveGame as jest.Mock;
    saveGameMock.mockClear();

    eventManager.dispatch(new Event("battle_complete", { result: 0 }));
    expect(saveGameMock).toHaveBeenCalledWith(game.getGameState(), AUTOSAVE_SLOT);
  });

  it("does not fatal when battle autosave rejects", async () => {
    const game = await buildGame();
    const eventManager = (game as unknown as { eventManager: EventManager }).eventManager;
    const saveGameMock = saveGame as jest.Mock;
    const warnSpy = jest.spyOn(console, "warn").mockImplementation(() => {});
    saveGameMock.mockReset();
    saveGameMock.mockRejectedValueOnce(new Error("save failed"));

    eventManager.dispatch(new Event("battle_complete", { result: 0 }));
    await flushPromises();

    const internals = getInternals(game);
    expect(internals.fatalError ?? null).toBeNull();
    expect(internals.quitRequested).not.toBe(true);
    expect(internals.hasPersistedSaveData).not.toBe(true);
    expect(warnSpy).toHaveBeenCalledWith("[save] Autosave failed:", expect.any(Error));
  });

  it("does not fatal when step autosave rejects", async () => {
    const game = await buildGame();
    const eventManager = (game as unknown as { eventManager: EventManager }).eventManager;
    const saveGameMock = saveGame as jest.Mock;
    const warnSpy = jest.spyOn(console, "warn").mockImplementation(() => {});
    saveGameMock.mockReset();
    saveGameMock.mockRejectedValueOnce(new Error("save failed"));

    for (let i = 0; i < 250; i += 1) {
      eventManager.dispatch(new Event("player_step", { count: 1 }));
    }
    await flushPromises();

    const internals = getInternals(game);
    expect(internals.fatalError ?? null).toBeNull();
    expect(internals.quitRequested).not.toBe(true);
    expect(internals.hasPersistedSaveData).not.toBe(true);
    expect(warnSpy).toHaveBeenCalledWith("[save] Autosave failed:", expect.any(Error));
  });

  it("marks persisted save data after a successful autosave", async () => {
    const game = await buildGame();
    const eventManager = (game as unknown as { eventManager: EventManager }).eventManager;
    const saveGameMock = saveGame as jest.Mock;
    saveGameMock.mockReset();
    saveGameMock.mockResolvedValueOnce(true);

    eventManager.dispatch(new Event("battle_complete", { result: 0 }));
    await flushPromises();

    expect(getInternals(game).hasPersistedSaveData).toBe(true);
  });

  it("rethrows non-not-found load errors when strictLoadSlot is enabled", async () => {
    const loadGameMock = require("@pokecrystal/core/core/save").loadGame as jest.Mock;
    loadGameMock.mockRejectedValueOnce(new Error("supabase unavailable"));

    await expect(
      buildGame({ loadSlot: "mcp-strict-load.sav", strictLoadSlot: true })
    ).rejects.toThrow("supabase unavailable");
  });

  it("enables CONTINUE when the boot save slot exists", async () => {
    const hasSaveGameMock = hasSaveGame as jest.Mock;
    hasSaveGameMock.mockResolvedValueOnce(true);

    const game = await buildGame({ initialState: "main_menu" });
    const mainMenu = (game as unknown as { mainMenu: { menuOptions: string[] } }).mainMenu;

    expect(hasSaveGameMock).toHaveBeenCalledWith(MANUAL_SAVE_SLOT);
    expect(mainMenu.menuOptions).toContain("CONTINUE");
  });

});

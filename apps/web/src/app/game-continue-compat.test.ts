/**
 * @jest-environment jsdom
 */

const titleScreenActionQueue: string[] = [];
const continueScreenActionQueue: Array<"confirm" | "cancel" | null> = [];
const deleteSaveScreenActionQueue: Array<"confirm" | "cancel" | null> = [];

jest.mock("@pokecrystal/core/ui/overlays/battle-ui", () => ({
  create_battle_ui: jest.fn(() => ({ active: false })),
  set_audio_engine: jest.fn(),
  set_game_state: jest.fn(),
}));

jest.mock("@pokecrystal/core/ui/screens/title-screen", () => ({
  TitleScreen: {
    create: jest.fn(async () => ({
      startFromGameStart: jest.fn(),
      update: jest.fn(),
      popAction: jest.fn(() => titleScreenActionQueue.shift() ?? null),
      draw: jest.fn(),
      handleInput: jest.fn(),
    })),
  },
  TitleScreenOption: {
    MAIN_MENU: "main_menu",
    DELETE_SAVE_DATA: "delete_save_data",
    RESET_CLOCK: "reset_clock",
    RESTART: "restart_intro",
  },
}));

jest.mock("@pokecrystal/assets/content/data/pokegear-landmarks", () => ({
  POKEGEAR_LANDMARKS: [],
  MAP_TO_LANDMARK: {},
}));

jest.mock("@pokecrystal/core/engine/world/overworld/overworld", () => ({
  OverworldEngine: class OverworldEngine {
    public script_runner = null;
    public current_map = null;
    public fatal_error = null;
    public dialogue = null;
    public is_moving = false;
    public input_capture_active = false;
    public _movement_lock_count = 0;
    public _text_lock_active = false;

    async init_assets(): Promise<void> {}
    reset_input_state(): void {}
    update(): void {}
    draw(): void {}
    handle_input(): void {}
    start_map_music(): void {}
    restart_map_music(): void {}
    player_movement_locked(): boolean {
      return false;
    }
    script_tasks_active(): boolean {
      return false;
    }
  },
}));

jest.mock("@pokecrystal/core/ui/menus/menu-state", () => ({
  MenuState: class MenuState {
    reset(): void {}
    openOptionsMenu(): void {}
    handleInput(): null {
      return null;
    }
    beginSaveFlow(): void {}
  },
}));

jest.mock("@pokecrystal/core/ui/menus/main-menu", () => ({
  MainMenu: class MainMenu {
    public menuOptions: string[];

    constructor(
      _ui: unknown,
      _audioEngine: unknown,
      _gameState: unknown,
      saveExists: boolean
    ) {
      this.menuOptions = saveExists ? ["CONTINUE", "NEW GAME", "OPTION"] : ["NEW GAME", "OPTION"];
    }

    refresh(saveExists: boolean): void {
      this.menuOptions = saveExists ? ["CONTINUE", "NEW GAME", "OPTION"] : ["NEW GAME", "OPTION"];
    }

    startFadeIn(): void {}
    handleInput(): null {
      return null;
    }
  },
}));

jest.mock("@pokecrystal/core/engine/battle/battle/battle-logic", () => ({
  Battle: class Battle {
    handle_input(): void {}
  },
}));

jest.mock("@pokecrystal/core/engine/battle/battle/trainer-battle", () => ({
  TrainerBattle: class TrainerBattle {},
}));

jest.mock("@pokecrystal/core/multiplayer/multiplayer-battle", () => ({
  MultiplayerBattle: class MultiplayerBattle {
    destroy(): void {}
  },
}));

jest.mock("@pokecrystal/core/engine/world/whiteout", () => ({
  WhiteoutManager: class WhiteoutManager {
    update(): void {}
  },
}));

jest.mock("@pokecrystal/core/ui/screens/intro/intro-sequence", () => ({
  IntroSequence: class IntroSequence {
    reset(): void {}
    handleInput(): boolean {
      return false;
    }
  },
}));

jest.mock("@pokecrystal/core/ui/screens/continue-screen", () => ({
  ContinueScreen: class ContinueScreen {
    draw(): void {}
    handleInput(): "confirm" | "cancel" | null {
      return continueScreenActionQueue.shift() ?? null;
    }
  },
}));

jest.mock("@pokecrystal/core/ui/screens/delete-save-screen", () => ({
  DeleteSaveScreen: class DeleteSaveScreen {
    reset(): void {}
    handleInput(): "confirm" | "cancel" | null {
      return deleteSaveScreenActionQueue.shift() ?? null;
    }
  },
}));

jest.mock("@pokecrystal/core/ui/screens/clock-reset-screen", () => ({
  ClockResetScreen: class ClockResetScreen {
    reset(): void {}
    handleInput(): null {
      return null;
    }
  },
}));

jest.mock("@pokecrystal/core/ui/screens/intro/oak-intro-sequence", () => ({
  OakIntroSequence: class OakIntroSequence {
    static async create(): Promise<OakIntroSequence> {
      return new OakIntroSequence();
    }
    reset(): void {}
    setPlayerGender(): void {}
    setInstantMode(): void {}
    handleInput(): void {}
  },
}));

jest.mock("@pokecrystal/core/ui/screens/intro/gender-selection", () => ({
  GenderSelectionScreen: class GenderSelectionScreen {
    reset(): void {}
    handleInput(): void {}
  },
}));

jest.mock("@pokecrystal/core/ui/screens/name-entry-screen", () => ({
  NameEntryScreen: class NameEntryScreen {
    reset(): void {}
    fillName(): void {}
    handleInput(): void {}
  },
}));

jest.mock("@pokecrystal/core/ui/text-overlays", () => ({
  renderTextSnapshot: jest.fn(),
}));

jest.mock("@pokecrystal/core/engine/world/maps", () => ({
  Spawn: {
    HOME: 0,
  },
  applySpawn: jest.fn(),
  getMapMetadataByGroup: jest.fn(() => ({
    name: "NEW_BARK_TOWN",
    constant: "NEW_BARK_TOWN",
  })),
}));

jest.mock("@pokecrystal/core/engine/world/overworld/overworld-tileset", () => ({
  OverworldTileset: class OverworldTileset {
    constructor(_tilesetName: string, _timeOfDay: string) {}
  },
}));

import { Game } from "./game";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { BaseFontRenderer } from "@pokecrystal/core/ui/base-ui";
import type { Surface } from "@pokecrystal/core/ui/surface";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { normalizeSaveSnapshot } from "@pokecrystal/core/core/save";
import { saveGame } from "@pokecrystal/core/core/save";
import { writeGuestSessionSlot } from "@pokecrystal/core/core/guest-session-storage";
import { MANUAL_SAVE_HISTORY_SLOTS, MANUAL_SAVE_SLOT } from "@pokecrystal/core/core/save-slots";

type MockFontRenderer = BaseFontRenderer & {
  font_tiles?: Record<number, InstanceType<typeof gameEngine.Surface>>;
};

type MockTextUI = TextUI & {
  tile_size?: number;
  font: MockFontRenderer;
};

type TilesetInstance = {
  tilesetName: string;
  metatiles: Array<{ collision: number[] }>;
  renderMetatile(): void;
  renderPriorityMetatile(): void;
};

type TilesetConstructor = new (tilesetName: string) => TilesetInstance;

type GlobalOverrides = {
  fetch?: typeof globalThis.fetch | undefined;
  createImageBitmap?: typeof globalThis.createImageBitmap | undefined;
  Tileset?: TilesetConstructor;
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

  try {
    return await Game.create(ui, {
      preloadMode: "none",
      ...options,
    });
  } finally {
    globalScope.fetch = originalFetch;
    globalScope.createImageBitmap = originalCreateImageBitmap;
    globalScope.Tileset = originalTileset;
    gameEngine.image.load = originalImageLoad;
  }
};

const getMainMenuOptions = (game: Game): string[] =>
  ((game as unknown as { mainMenu?: { menuOptions?: string[] } }).mainMenu?.menuOptions ?? []);

const writeValidSnapshot = (slot: string): void => {
  const gameState = createInitialGameState();
  gameState.sram.player_name = "ContinueReady";
  const payload = JSON.stringify(normalizeSaveSnapshot(gameState, `test:${slot}`));
  writeGuestSessionSlot(slot, payload);
};

describe("Game continue compatibility", () => {
  afterEach(() => {
    titleScreenActionQueue.length = 0;
    continueScreenActionQueue.length = 0;
    deleteSaveScreenActionQueue.length = 0;
    window.localStorage.clear();
    window.sessionStorage.clear();
    jest.restoreAllMocks();
  });

  it("shows CONTINUE when only guest-session save data exists", async () => {
    writeValidSnapshot("savegame.sav");

    const game = await buildGame({ initialState: "main_menu" });

    expect(getMainMenuOptions(game)).toContain("CONTINUE");
  });

  it("shows CONTINUE when only legacy browser fs save data exists", async () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "LegacyContinue";
    const payload = JSON.stringify(normalizeSaveSnapshot(gameState, "legacy:savegame.sav"));
    window.localStorage.setItem("fs:/legacy/savegame.sav", payload);

    const game = await buildGame({ initialState: "main_menu" });

    expect(getMainMenuOptions(game)).toContain("CONTINUE");
  });

  it("shows CONTINUE when only a recent manual save slot exists", async () => {
    writeValidSnapshot(MANUAL_SAVE_HISTORY_SLOTS[0]);

    const game = await buildGame({ initialState: "main_menu" });

    expect(getMainMenuOptions(game)).toContain("CONTINUE");
  });

  it("preserves CONTINUE through the title-screen boot path", async () => {
    writeValidSnapshot("savegame.sav");
    titleScreenActionQueue.push("main_menu");

    const game = await buildGame({ initialState: "title", newGame: true });
    game.tick();

    expect(getMainMenuOptions(game)).toContain("CONTINUE");
  });

  it("shows CONTINUE after saving a live runtime state to the manual slot", async () => {
    const liveGame = await buildGame({ initialState: "overworld" });
    liveGame.getGameState().sram.player_name = "RuntimeSaved";

    await expect(saveGame(liveGame.getGameState(), MANUAL_SAVE_SLOT)).resolves.toBe(true);

    const rebootedGame = await buildGame({ initialState: "main_menu" });

    expect(getMainMenuOptions(rebootedGame)).toContain("CONTINUE");
  });

  it("loads the first available recent manual save when the primary slot is missing", async () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "RecentSlotLoaded";
    const payload = JSON.stringify(
      normalizeSaveSnapshot(gameState, `test:${MANUAL_SAVE_HISTORY_SLOTS[0]}`)
    );
    writeGuestSessionSlot(MANUAL_SAVE_HISTORY_SLOTS[0], payload);
    const game = await buildGame({ initialState: "continue" });

    continueScreenActionQueue.push("confirm");
    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_RETURN }));
    game.tick();
    await (
      (game as unknown as { bootTransitionTask?: Promise<void> | null }).bootTransitionTask ??
      Promise.resolve()
    );

    expect(game.getGameState().sram.player_name).toBe("RecentSlotLoaded");
  });

  it("falls back to the second recent manual save slot when earlier manual slots are empty", async () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "SecondRecentLoaded";
    const payload = JSON.stringify(
      normalizeSaveSnapshot(gameState, `test:${MANUAL_SAVE_HISTORY_SLOTS[1]}`)
    );
    writeGuestSessionSlot(MANUAL_SAVE_HISTORY_SLOTS[1], payload);

    const game = await buildGame({ initialState: "continue" });

    continueScreenActionQueue.push("confirm");
    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_RETURN }));
    game.tick();
    await (
      (game as unknown as { bootTransitionTask?: Promise<void> | null }).bootTransitionTask ??
      Promise.resolve()
    );

    expect(game.getGameState().sram.player_name).toBe("SecondRecentLoaded");
  });

  it("prefers the primary manual slot over recent history when both exist", async () => {
    const primaryState = createInitialGameState();
    primaryState.sram.player_name = "PrimaryLoaded";
    writeGuestSessionSlot(
      MANUAL_SAVE_SLOT,
      JSON.stringify(normalizeSaveSnapshot(primaryState, `test:${MANUAL_SAVE_SLOT}`))
    );

    const recentState = createInitialGameState();
    recentState.sram.player_name = "RecentShouldNotWin";
    writeGuestSessionSlot(
      MANUAL_SAVE_HISTORY_SLOTS[0],
      JSON.stringify(normalizeSaveSnapshot(recentState, `test:${MANUAL_SAVE_HISTORY_SLOTS[0]}`))
    );

    const game = await buildGame({ initialState: "continue" });

    continueScreenActionQueue.push("confirm");
    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_RETURN }));
    game.tick();
    await (
      (game as unknown as { bootTransitionTask?: Promise<void> | null }).bootTransitionTask ??
      Promise.resolve()
    );

    expect(game.getGameState().sram.player_name).toBe("PrimaryLoaded");
  });

  it("clears CONTINUE and deletes the full manual save family from delete-save flow", async () => {
    writeValidSnapshot(MANUAL_SAVE_HISTORY_SLOTS[0]);
    writeValidSnapshot(MANUAL_SAVE_HISTORY_SLOTS[1]);

    const game = await buildGame({ initialState: "delete_save" });

    deleteSaveScreenActionQueue.push("confirm");
    titleScreenActionQueue.push("main_menu");
    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_RETURN }));
    game.tick();
    await (
      (game as unknown as { bootTransitionTask?: Promise<void> | null }).bootTransitionTask ??
      Promise.resolve()
    );
    game.tick();

    expect(getMainMenuOptions(game)).not.toContain("CONTINUE");
    expect(window.localStorage.getItem(`pokecrystal:guest-save:${MANUAL_SAVE_SLOT}`)).toBeNull();
    expect(window.localStorage.getItem(`pokecrystal:guest-save:${MANUAL_SAVE_HISTORY_SLOTS[0]}`)).toBeNull();
    expect(window.localStorage.getItem(`pokecrystal:guest-save:${MANUAL_SAVE_HISTORY_SLOTS[1]}`)).toBeNull();
  });
});

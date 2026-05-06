jest.mock("@pokecrystal/core/ui/overlays/battle-ui", () => ({
  create_battle_ui: jest.fn(() => ({ active: false })),
  set_audio_engine: jest.fn(),
  set_game_state: jest.fn(),
}));

jest.mock("@pokecrystal/core/ui/screens/name-entry-screen", () => ({
  NameEntryScreen: jest.fn().mockImplementation(() => ({
    finished: false,
    name: "",
    reset: jest.fn(),
    fillName: jest.fn(),
    handleInput: jest.fn(),
    update: jest.fn(),
    draw: jest.fn(),
  })),
}));

import { Game } from "./game";
import { TitleScreen } from "@pokecrystal/core/ui/screens/title-screen";
import { OakIntroSequence } from "@pokecrystal/core/ui/screens/intro/oak-intro-sequence";
import { NameEntryScreen } from "@pokecrystal/core/ui/screens/name-entry-screen";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { BaseFontRenderer } from "@pokecrystal/core/ui/base-ui";
import type { Surface } from "@pokecrystal/core/ui/surface";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { ScriptRunnerImpl } from "@pokecrystal/core/engine/world/story-events/runner";

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
  update: () => void;
  draw: () => void;
  flushPendingOverworldMusicAction: () => void;
  transitionToOverworldState: (nextState?: ReturnType<Game["getGameState"]>) => Promise<void>;
  currentState: string;
  pendingOverworldMusicAction: "start" | "restart" | null;
  isOakIntroFinalSequence: boolean;
  introSequence: {
    draw: (ctx: CanvasRenderingContext2D) => void;
    getTextSnapshot: () => {
      viewportLines: string[];
      infoLines: string[];
      viewportTitle: string;
      infoTitle: string;
      menuLines?: string[] | null;
      promptLines?: string[] | null;
      dialogueLines?: string[] | null;
    };
  } | null;
  titleScreen: {
    draw: (ctx: CanvasRenderingContext2D) => void;
    getTextSnapshot: () => {
      viewportLines: string[];
      infoLines: string[];
      viewportTitle: string;
      infoTitle: string;
      menuLines?: string[] | null;
      promptLines?: string[] | null;
      dialogueLines?: string[] | null;
    };
  } | null;
  genderSelection: {
    draw: (ctx: CanvasRenderingContext2D) => void;
    getTextSnapshot: () => {
      viewportLines: string[];
      infoLines: string[];
      viewportTitle: string;
      infoTitle: string;
      menuLines?: string[] | null;
      promptLines?: string[] | null;
      dialogueLines?: string[] | null;
    };
  } | null;
  playerNameEntryScreen: {
    finished: boolean;
    name: string;
    reset: (options?: unknown) => void;
    fillName: (name: string) => void;
    handleInput: (event: unknown) => void;
    update: () => void;
    draw: () => void;
  } | null;
  oakIntroSequence: {
    update: () => boolean;
    updateFinalEncouragement: () => boolean;
    startFinalEncouragement: (playerName: string) => void;
  } | null;
};

const getInternals = (target: Game): GameInternals => target as unknown as GameInternals;

const buildGame = async (options: ConstructorParameters<typeof Game.create>[1] = {}): Promise<Game> => {
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
  fontRenderer.get_char_tile = jest.fn();
  fontRenderer.getCharTile = jest.fn();

  const globalScope = globalThis as GlobalOverrides;
  const originalFetch = globalScope.fetch;
  const originalCreateImageBitmap = globalScope.createImageBitmap;
  const originalTileset = globalScope.Tileset;
  const originalImageLoad = gameEngine.image.load;
  const originalInitAssets = OverworldEngine.prototype.init_assets;
  const loadMapSpy = jest.spyOn(OverworldEngine.prototype, "load_map").mockImplementation(() => {});
  const ensureOverworldDataSpy = jest
    .spyOn(DataLoader.prototype, "ensure_overworld_data")
    .mockImplementation(() => {});
  const reloadStoryEventsSpy = jest
    .spyOn(DataLoader.prototype, "reload_story_events")
    .mockImplementation(() => {});
  const scriptRunnerRunSpy = jest
    .spyOn(ScriptRunnerImpl.prototype, "run")
    .mockImplementation(() => {});

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
  gameEngine.image.load = async () => new gameEngine.Surface(16, 16);
  OverworldEngine.prototype.init_assets = async () => {};

  try {
    return await Game.create(ui, options);
  } finally {
    ensureOverworldDataSpy.mockRestore();
    reloadStoryEventsSpy.mockRestore();
    scriptRunnerRunSpy.mockRestore();
    loadMapSpy.mockRestore();
    globalScope.fetch = originalFetch;
    globalScope.createImageBitmap = originalCreateImageBitmap;
    globalScope.Tileset = originalTileset;
    gameEngine.image.load = originalImageLoad;
    OverworldEngine.prototype.init_assets = originalInitAssets;
  }
};

describe("Game intro/title startup parity", () => {
  afterEach(() => {
    jest.restoreAllMocks();
    jest.clearAllMocks();
  });

  it("enters the player naming step before Oak final encouragement", async () => {
    const game = await buildGame();
    const internals = getInternals(game);
    const startFinalEncouragement = jest.fn();
    internals.currentState = "oak_intro";
    internals.isOakIntroFinalSequence = false;
    internals.playerNameEntryScreen = null;
    internals.oakIntroSequence = {
      update: () => true,
      updateFinalEncouragement: () => false,
      startFinalEncouragement,
    };

    internals.update();

    expect(NameEntryScreen).toHaveBeenCalled();
    expect(startFinalEncouragement).not.toHaveBeenCalled();
    expect(internals.isOakIntroFinalSequence).toBe(false);
    expect(internals.currentState).toBe("name_entry");
  });

  it("does not fabricate PLAYER during Oak intro final encouragement", async () => {
    const game = await buildGame();
    const internals = getInternals(game);
    const startFinalEncouragement = jest.fn();
    internals.currentState = "name_entry";
    internals.isOakIntroFinalSequence = false;
    internals.playerNameEntryScreen = {
      finished: true,
      name: "   ",
      reset: jest.fn(),
      fillName: jest.fn(),
      handleInput: jest.fn(),
      update: jest.fn(),
      draw: jest.fn(),
    };
    internals.oakIntroSequence = {
      update: () => false,
      updateFinalEncouragement: () => false,
      startFinalEncouragement,
    };
    game.getGameState().sram.player_name = "";

    expect(() => internals.update()).not.toThrow();
    expect(game.getGameState().sram.player_name).not.toBe("PLAYER");
    expect(internals.isOakIntroFinalSequence).toBe(true);
  });

  it("does not restart the title entrance when booting directly into oak intro", async () => {
    const titleScreen = {
      startFromGameStart: jest.fn(),
    } as unknown as TitleScreen;
    const oakIntro = {
      setPlayerGender: jest.fn(),
      setInstantMode: jest.fn(),
      reset: jest.fn(),
    } as unknown as OakIntroSequence;

    jest.spyOn(TitleScreen, "create").mockResolvedValue(titleScreen);
    jest.spyOn(OakIntroSequence, "create").mockResolvedValue(oakIntro);

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
    fontRenderer.get_char_tile = jest.fn();
    fontRenderer.getCharTile = jest.fn();

    const globalScope = globalThis as GlobalOverrides;
    const originalFetch = globalScope.fetch;
    const originalCreateImageBitmap = globalScope.createImageBitmap;
    const originalTileset = globalScope.Tileset;
    const originalImageLoad = gameEngine.image.load;
    const originalInitAssets = OverworldEngine.prototype.init_assets;
    const startMapMusicSpy = jest.spyOn(OverworldEngine.prototype, "start_map_music").mockImplementation(() => {});
    const loadMapSpy = jest.spyOn(OverworldEngine.prototype, "load_map").mockImplementation(() => {});
    const ensureOverworldDataSpy = jest
      .spyOn(DataLoader.prototype, "ensure_overworld_data")
      .mockImplementation(() => {});
    const reloadStoryEventsSpy = jest
      .spyOn(DataLoader.prototype, "reload_story_events")
      .mockImplementation(() => {});
    const scriptRunnerRunSpy = jest
      .spyOn(ScriptRunnerImpl.prototype, "run")
      .mockImplementation(() => {});

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
    gameEngine.image.load = async () => new gameEngine.Surface(16, 16);
    OverworldEngine.prototype.init_assets = async () => {};

    try {
      await Game.create(ui, { initialState: "oak_intro" });
    } finally {
      ensureOverworldDataSpy.mockRestore();
      reloadStoryEventsSpy.mockRestore();
      scriptRunnerRunSpy.mockRestore();
      startMapMusicSpy.mockRestore();
      loadMapSpy.mockRestore();
      globalScope.fetch = originalFetch;
      globalScope.createImageBitmap = originalCreateImageBitmap;
      globalScope.Tileset = originalTileset;
      gameEngine.image.load = originalImageLoad;
      OverworldEngine.prototype.init_assets = originalInitAssets;
    }

    expect(titleScreen.startFromGameStart).not.toHaveBeenCalled();
    expect(oakIntro.reset).toHaveBeenCalled();
    expect(startMapMusicSpy).not.toHaveBeenCalled();
  });

  it("does not start overworld music while booting to the title screen", async () => {
    const startMapMusicSpy = jest.spyOn(OverworldEngine.prototype, "start_map_music").mockImplementation(() => {});

    try {
      await buildGame();
    } finally {
      startMapMusicSpy.mockRestore();
    }

    expect(startMapMusicSpy).not.toHaveBeenCalled();
  });

  it("starts overworld music only after the first overworld draw on direct boot", async () => {
    const earlyStartMapMusicSpy = jest.spyOn(OverworldEngine.prototype, "start_map_music").mockImplementation(() => {});

    try {
      const game = await buildGame({ initialState: "overworld" });
      const internals = getInternals(game);
      const startMapMusicSpy = jest.spyOn(game.getOverworld(), "start_map_music").mockImplementation(() => {});
      expect(game.getState()).toBe("overworld");

      expect(earlyStartMapMusicSpy).not.toHaveBeenCalled();
      expect(internals.pendingOverworldMusicAction).toBe("start");

      internals.flushPendingOverworldMusicAction();
      expect(startMapMusicSpy).toHaveBeenCalledTimes(1);

      startMapMusicSpy.mockRestore();
    } finally {
      earlyStartMapMusicSpy.mockRestore();
    }
  });

  it("exposes map music in the playback snapshot on the first overworld draw", async () => {
    const game = await buildGame({ initialState: "overworld" });
    const internals = getInternals(game);

    expect(game.getAudioPlaybackSnapshot().musicToken).toBeNull();

    internals.draw();

    const snapshot = game.getAudioPlaybackSnapshot();
    const musicSource = snapshot.musicSource?.replace(/\\/g, "/") ?? "";
    expect(snapshot.musicToken).toBe("MUSIC_NEW_BARK_TOWN");
    expect(musicSource).toContain("/assets/audio/newbarktown.mp3");
    expect(musicSource).not.toContain("/assets/data/audio/");
  });

  it("does not run overworld initialization scripts while booting to the title screen", async () => {
    const scriptRunnerRunSpy = jest
      .spyOn(ScriptRunnerImpl.prototype, "run")
      .mockImplementation(() => {});

    try {
      await buildGame();
    } finally {
      scriptRunnerRunSpy.mockRestore();
    }

    expect(scriptRunnerRunSpy).not.toHaveBeenCalledWith("InitializeEventsScript");
    expect(scriptRunnerRunSpy).not.toHaveBeenCalledWith("PlayersHouse2FInitializeRoomCallback");
  });

  it("does not start overworld music when new game enters the gender setup flow", async () => {
    const game = await buildGame();
    const startMapMusicSpy = jest.spyOn(OverworldEngine.prototype, "start_map_music").mockImplementation(() => {});
    const gameAny = game as unknown as {
      enterNewGameFlow: () => Promise<void>;
      currentState: string;
    };

    try {
      await gameAny.enterNewGameFlow();
    } finally {
      startMapMusicSpy.mockRestore();
    }

    expect(gameAny.currentState).toBe("gender");
    expect(startMapMusicSpy).not.toHaveBeenCalled();
  });

  it("stops carried music and defers overworld music until draw during overworld transitions", async () => {
    const game = await buildGame();
    const internals = getInternals(game);
    const earlyStartMapMusicSpy = jest.spyOn(OverworldEngine.prototype, "start_map_music").mockImplementation(() => {});
    const stopMusicSpy = jest.spyOn(
      (game as unknown as { audioEngine: { stopMusic: () => void } }).audioEngine,
      "stopMusic",
    );
    game.getOverworld().draw = jest.fn();

    try {
      await internals.transitionToOverworldState(game.getGameState());
      const startMapMusicSpy = jest.spyOn(game.getOverworld(), "start_map_music").mockImplementation(() => {});
      expect(game.getState()).toBe("overworld");

      expect(stopMusicSpy).toHaveBeenCalledTimes(1);
      expect(earlyStartMapMusicSpy).not.toHaveBeenCalled();
      expect(internals.pendingOverworldMusicAction).toBe("start");

      internals.flushPendingOverworldMusicAction();
      expect(startMapMusicSpy).toHaveBeenCalledTimes(1);

      startMapMusicSpy.mockRestore();
    } finally {
      stopMusicSpy.mockRestore();
      earlyStartMapMusicSpy.mockRestore();
    }
  });

  it("clears remembered map music before restarting the crystal intro", async () => {
    const game = await buildGame();
    const gameAny = game as unknown as {
      audioEngine: {
        clearMapMusic: () => void;
        playMusic: (name: string, role?: string) => void;
      };
      enterIntroSequenceState: () => void;
      currentState: string;
    };
    const clearMapMusicSpy = jest.spyOn(gameAny.audioEngine, "clearMapMusic");
    const playMusicSpy = jest.spyOn(gameAny.audioEngine, "playMusic");

    gameAny.enterIntroSequenceState();

    expect(clearMapMusicSpy).toHaveBeenCalledTimes(1);
    expect(playMusicSpy).toHaveBeenCalledWith("MUSIC_NONE", "intro");
    expect(gameAny.currentState).toBe("intro");
  });

  it("clears remembered map music before entering the title screen", async () => {
    const game = await buildGame();
    const gameAny = game as unknown as {
      audioEngine: {
        clearMapMusic: () => void;
      };
      enterTitleState: () => void;
      currentState: string;
    };
    const clearMapMusicSpy = jest.spyOn(gameAny.audioEngine, "clearMapMusic");

    gameAny.enterTitleState();

    expect(clearMapMusicSpy).toHaveBeenCalledTimes(1);
    expect(gameAny.currentState).toBe("title");
  });

  it("clears remembered map music before entering Oak intro", async () => {
    const game = await buildGame();
    const gameAny = game as unknown as {
      audioEngine: {
        clearMapMusic: () => void;
      };
      enterOakIntroState: () => Promise<void>;
      currentState: string;
    };
    const clearMapMusicSpy = jest.spyOn(gameAny.audioEngine, "clearMapMusic");

    await gameAny.enterOakIntroState();

    expect(clearMapMusicSpy).toHaveBeenCalledTimes(1);
    expect(gameAny.currentState).toBe("oak_intro");
  });

  it("enables instant Oak intro pacing when boot animations are suppressed", async () => {
    const game = await buildGame({ initialState: "oak_intro", suppressBootAnimations: true });
    const sequence = (game as unknown as { oakIntroSequence: unknown }).oakIntroSequence as
      | { instantMode?: boolean }
      | null;

    expect(sequence?.instantMode).toBe(true);
  });

  it("continues into Oak intro after direct-player input completes clock setup", async () => {
    const loadAssetsSpy = jest
      .spyOn(OakIntroSequence.prototype as unknown as { loadAssets: () => Promise<void> }, "loadAssets")
      .mockResolvedValue(undefined);
    const updatePlayerPicSpy = jest
      .spyOn(
        OakIntroSequence.prototype as unknown as { updatePlayerPicSurface: () => Promise<void> },
        "updatePlayerPicSurface",
      )
      .mockResolvedValue(undefined);
    try {
      const game = await buildGame({ initialState: "oak_intro" });
      const pressA = (): void => {
        game.postEvent(new gameEngine.event.Event("keydown", { button: "a", is_press: true }));
        game.tick();
        game.postEvent(new gameEngine.event.Event("keyup", { button: "a", is_press: false }));
        game.tick();
      };
      const settle = (frames = 20): void => {
        for (let i = 0; i < frames; i += 1) {
          game.tick();
        }
      };

      for (let i = 0; i < 8; i += 1) {
        pressA();
        settle(2);
      }
      expect(game.getDebugStatus().oak_intro?.timeSetPhase).toBe("set_hour");

      pressA();
      settle(60);
      expect(game.getDebugStatus().oak_intro?.timeSetPhase).toBe("hour_confirm");

      pressA();
      settle(20);
      expect(game.getDebugStatus().oak_intro?.timeSetPhase).toBe("set_minute");

      pressA();
      settle(60);
      expect(game.getDebugStatus().oak_intro?.timeSetPhase).toBe("minute_confirm");

      pressA();
      settle(80);
      expect(game.getDebugStatus().oak_intro?.timeSetPhase).toBe("final_reaction");

      pressA();
      pressA();
      settle(100);

      const status = game.getDebugStatus();
      expect(status.mode).toBe("oak_intro");
      expect(status.oak_intro?.timeSetComplete).toBe(true);
      expect(status.oak_intro?.timeSetPhase).toBeNull();
      expect(status.oak_intro?.sceneState).toBe("oak_intro_1");
      expect(status.oak_intro?.visibleText).toContain("Hello");
      const snapshot = ((game as unknown as { ui: TextUI }).ui).getSnapshot();
      expect(snapshot.dialogueLines?.join("\n")).not.toContain("What time is it?");
      expect(snapshot.dialogueLines?.join("\n")).toContain("Hello");
    } finally {
      loadAssetsSpy.mockRestore();
      updatePlayerPicSpy.mockRestore();
    }
  });

  it("mirrors boot-only draw states into the shared text snapshot renderer", async () => {
    const game = await buildGame();
    const internals = getInternals(game);
    const ui = (game as unknown as { ui: TextUI }).ui;
    (game as unknown as { bootRenderContext: CanvasRenderingContext2D }).bootRenderContext = {
      setTransform: jest.fn(),
      globalAlpha: 1,
      globalCompositeOperation: "source-over",
      imageSmoothingEnabled: false,
      canvas: { width: 160, height: 144 },
    } as unknown as CanvasRenderingContext2D;
    const drawBootCtx = jest.fn();
    const bootPayload = {
      viewportLines: ["BOOT SNAPSHOT"],
      infoLines: ["STATE: boot"],
      viewportTitle: "Boot",
      infoTitle: "Boot",
      menuLines: ["▶ OPTION"],
      promptLines: ["A: Confirm"],
      dialogueLines: ["READY"],
    };

    internals.introSequence = {
      draw: drawBootCtx,
      getTextSnapshot: () => bootPayload,
    };
    internals.currentState = "intro";
    internals.draw();
    expect(ui.getSnapshot()).toMatchObject({
      viewportTitle: "Boot",
      infoTitle: "Boot",
      viewportLines: ["BOOT SNAPSHOT"],
    });

    internals.titleScreen = {
      draw: drawBootCtx,
      getTextSnapshot: () => ({
        ...bootPayload,
        viewportLines: ["TITLE SNAPSHOT"],
        infoLines: ["STATE: title"],
        viewportTitle: "Title",
        infoTitle: "Title",
      }),
    };
    internals.currentState = "title";
    internals.draw();
    expect(ui.getSnapshot()).toMatchObject({
      viewportTitle: "Title",
      infoLines: ["STATE: title"],
    });

    internals.genderSelection = {
      draw: drawBootCtx,
      getTextSnapshot: () => ({
        ...bootPayload,
        viewportLines: ["GENDER SNAPSHOT"],
        infoLines: ["STATE: gender"],
        viewportTitle: "Gender",
        infoTitle: "Gender",
      }),
    };
    internals.currentState = "gender";
    internals.draw();
    expect(ui.getSnapshot()).toMatchObject({
      viewportTitle: "Gender",
      infoLines: ["STATE: gender"],
    });

    internals.oakIntroSequence = {
      draw: drawBootCtx,
      update: () => false,
      updateFinalEncouragement: () => false,
      startFinalEncouragement: jest.fn(),
      getTextSnapshot: () => ({
        ...bootPayload,
        viewportLines: ["OAK SNAPSHOT"],
        infoLines: ["STATE: oak_intro"],
        viewportTitle: "Oak Intro",
        infoTitle: "Oak Intro",
      }),
    };
    internals.currentState = "oak_intro";
    internals.draw();
    expect(ui.getSnapshot()).toMatchObject({
      viewportTitle: "Oak Intro",
      infoLines: ["STATE: oak_intro"],
    });
  });
});

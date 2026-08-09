jest.mock("@pokecrystal/core/ui/overlays/battle-ui", () => ({
  create_battle_ui: jest.fn(() => ({ active: false })),
  set_audio_engine: jest.fn(),
}));

import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { Game } from "./game";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
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

type GameLoopInternals = Game & {
  gameLoop: (timestamp?: number) => void;
  tick: () => void;
  handleInput: () => void;
  update: () => void;
  draw: () => void;
  loopCircuitBreaker: () => void;
  lastFrameTimeMs: number | null;
  frameRemainderMs: number;
  _nextLoopDelayMs: () => number;
  _scheduleGameLoop: (delayMs: number) => void;
  eventQueue: ReturnType<typeof gameEngine.event.createQueue>;
};

const installRafMock = (): {
  rafMock: jest.Mock;
  restore: () => void;
} => {
  const globalScope = globalThis as typeof globalThis & {
    requestAnimationFrame?: typeof requestAnimationFrame;
  };
  const previousRaf = globalScope.requestAnimationFrame;
  const rafMock = jest.fn();
  Object.defineProperty(globalThis, "requestAnimationFrame", {
    configurable: true,
    writable: true,
    value: rafMock,
  });
  return {
    rafMock,
    restore: () => {
      if (previousRaf === undefined) {
        delete globalScope.requestAnimationFrame;
        return;
      }
      Object.defineProperty(globalThis, "requestAnimationFrame", {
        configurable: true,
        writable: true,
        value: previousRaf,
      });
    },
  };
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
  const noopGetCharTile: NonNullable<BaseFontRenderer["getCharTile"]> = () => fontTiles[0] ?? null;
  fontRenderer.get_char_tile = noopGetCharTile;
  fontRenderer.getCharTile = noopGetCharTile;

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
  gameEngine.image.load = async () => new gameEngine.Surface(16, 16);
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

describe("Game loop timing", () => {
  it("enables frame benchmarking automatically when flare_plot debug is active", async () => {
    const originalDebug = process.env.NEXT_PUBLIC_POKE_DEBUG;
    process.env.NEXT_PUBLIC_POKE_DEBUG = "flare_plot";

    try {
      const game = await buildGame();
      expect(game.getBenchmark()).not.toBeNull();
    } finally {
      if (originalDebug === undefined) {
        delete process.env.NEXT_PUBLIC_POKE_DEBUG;
      } else {
        process.env.NEXT_PUBLIC_POKE_DEBUG = originalDebug;
      }
    }
  });

  it("schedules with setTimeout at GB cadence even when requestAnimationFrame exists", async () => {
    const game = await buildGame();
    const internals = game as GameLoopInternals;
    const setTimeoutSpy = jest.spyOn(globalThis, "setTimeout").mockImplementation(
      (() => 1 as ReturnType<typeof setTimeout>) as typeof setTimeout
    );
    const { rafMock, restore } = installRafMock();

    try {
      internals._scheduleGameLoop(GB_FRAME_DURATION_MS);
      expect(setTimeoutSpy).toHaveBeenCalledTimes(1);
      const [_, delay] = setTimeoutSpy.mock.calls[0] as [TimerHandler, number];
      expect(delay).toBeCloseTo(GB_FRAME_DURATION_MS, 3);
      expect(rafMock).not.toHaveBeenCalled();
    } finally {
      setTimeoutSpy.mockRestore();
      restore();
    }
  });

  it("keeps the local HTML canvas game out of MCP instant mode", async () => {
    const game = await buildGame();

    expect(game.getGameState().wram.instant_mode).toBe(false);
  });

  it("uses frame remainder to compute next loop delay", async () => {
    const game = await buildGame();
    const internals = game as GameLoopInternals;

    internals.frameRemainderMs = GB_FRAME_DURATION_MS * 0.25;
    expect(internals._nextLoopDelayMs()).toBeCloseTo(GB_FRAME_DURATION_MS * 0.75, 5);

    internals.frameRemainderMs = GB_FRAME_DURATION_MS * 2;
    expect(internals._nextLoopDelayMs()).toBe(0);

    internals.frameRemainderMs = -1;
    expect(internals._nextLoopDelayMs()).toBeCloseTo(GB_FRAME_DURATION_MS, 5);
  });

  it("processes catch-up frames and schedules only the remaining cadence", async () => {
    const game = await buildGame();
    const internals = game as GameLoopInternals;
    const tickSpy = jest.spyOn(internals, "tick").mockImplementation(() => {});
    const setTimeoutSpy = jest.spyOn(globalThis, "setTimeout").mockImplementation(
      (() => 1 as ReturnType<typeof setTimeout>) as typeof setTimeout
    );

    try {
      internals.lastFrameTimeMs = 1_000;
      internals.frameRemainderMs = 0;
      internals.loopCircuitBreaker = () => {};
      internals.gameLoop(1_000 + GB_FRAME_DURATION_MS * 2.4);

      expect(tickSpy).toHaveBeenCalledTimes(2);
      const [_, delay] = setTimeoutSpy.mock.calls[0] as [TimerHandler, number];
      expect(delay).toBeCloseTo(GB_FRAME_DURATION_MS * 0.6, 3);
    } finally {
      tickSpy.mockRestore();
      setTimeoutSpy.mockRestore();
    }
  });

  it("skips active-queue churn when the game queue is already bound", async () => {
    const game = await buildGame();
    const internals = game as GameLoopInternals;
    const setActiveQueueSpy = jest.spyOn(gameEngine.event, "setActiveQueue");
    const handleInputSpy = jest.spyOn(internals, "handleInput").mockImplementation(() => {});
    const updateSpy = jest.spyOn(internals, "update").mockImplementation(() => {});
    const drawSpy = jest.spyOn(internals, "draw").mockImplementation(() => {});

    gameEngine.event.setActiveQueue(internals.eventQueue);
    setActiveQueueSpy.mockClear();

    try {
      game.tick();
      expect(setActiveQueueSpy).not.toHaveBeenCalled();
      expect(handleInputSpy).toHaveBeenCalledTimes(1);
      expect(updateSpy).toHaveBeenCalledTimes(1);
      expect(drawSpy).toHaveBeenCalledTimes(1);
    } finally {
      gameEngine.event.setActiveQueue(null);
      setActiveQueueSpy.mockRestore();
      handleInputSpy.mockRestore();
      updateSpy.mockRestore();
      drawSpy.mockRestore();
    }
  });

  it("keeps the last completed overworld frame while a replacement tileset is loading", async () => {
    const game = await buildGame();
    const internals = game as GameLoopInternals;
    const ui = (game as unknown as { ui: { clearScreen: () => void; update: () => void } }).ui;
    const clearScreenSpy = jest.spyOn(ui, "clearScreen");
    const updateSpy = jest.spyOn(ui, "update");
    const overworld = game.getOverworld() as Overworld & {
      map_surface: Surface | null;
      _composite_surface: Surface | null;
    };
    const originalMapSurface = overworld.map_surface;
    const originalCompositeSurface = overworld._composite_surface;

    try {
      // Map loading clears these surfaces until the tileset promise resolves.
      overworld.map_surface = null;
      overworld._composite_surface = null;

      internals.draw();

      expect(clearScreenSpy).not.toHaveBeenCalled();
      expect(updateSpy).not.toHaveBeenCalled();
    } finally {
      overworld.map_surface = originalMapSurface;
      overworld._composite_surface = originalCompositeSurface;
      clearScreenSpy.mockRestore();
      updateSpy.mockRestore();
    }
  });
});

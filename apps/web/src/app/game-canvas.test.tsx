/** @jest-environment jsdom */
import React from "react";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { GameCanvas } from "./game-canvas";
import { Game } from "./game";
import { buildUi } from "./ui";
import type { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { MANUAL_SAVE_SLOT } from "@pokecrystal/core/core/save-slots";

type GameStub = {
  start: jest.Mock;
  postEvent: jest.Mock;
  unlockAudio: jest.Mock;
  setAudioMuted: jest.Mock;
  setMusicMuted: jest.Mock;
  tick: jest.Mock;
  debugJumpToScene: jest.Mock;
  debugJumpToSpawn: jest.Mock;
  getDebugStatus: jest.Mock;
  getState: jest.Mock;
  getGameState: jest.Mock;
  getOverworld: jest.Mock;
  getBenchmark: jest.Mock;
  clearBenchmark: jest.Mock;
  destroy: jest.Mock;
};

jest.mock("./game", () => ({
  Game: {
    create: jest.fn(),
  },
}));

jest.mock("./ui", () => ({
  buildUi: jest.fn(() => ({
    ui: {} as BaseUI,
    textUi: null,
  })),
}));

const flushPromises = async (): Promise<void> => {
  await Promise.resolve();
};

describe("GameCanvas", () => {
  const originalEnv = process.env.NEXT_PUBLIC_LOAD_SLOT;
  const originalGetContext = HTMLCanvasElement.prototype.getContext;
  const originalFetch = globalThis.fetch;
  const originalImage = globalThis.Image;
  const buildGameStub = (): GameStub => ({
    start: jest.fn(),
    postEvent: jest.fn(),
    unlockAudio: jest.fn(),
    setAudioMuted: jest.fn(),
    setMusicMuted: jest.fn(),
    tick: jest.fn(),
    debugJumpToScene: jest.fn(),
    debugJumpToSpawn: jest.fn(),
    getDebugStatus: jest.fn(() => ({ mode: "overworld", prompt_pending: false })),
    getState: jest.fn(() => "overworld"),
    getOverworld: jest.fn(() => ({ player_direction: "down" })),
    getBenchmark: jest.fn(() => ({
      getRecentFrames: jest.fn(() => [
        {
          frame: 42,
          timestamp: 1234,
          totalDuration: 6,
          phaseDurations: {
            handleInput: 1,
            update: 2,
            draw: 3,
          },
        },
      ]),
      getSlowFrames: jest.fn((thresholdMs: number) =>
        thresholdMs <= 6
          ? [
              {
                frame: 42,
                timestamp: 1234,
                totalDuration: 6,
                phaseDurations: {
                  handleInput: 1,
                  update: 2,
                  draw: 3,
                },
              },
            ]
          : []
      ),
    })),
    clearBenchmark: jest.fn(),
    destroy: jest.fn(),
    getGameState: jest.fn(() => ({
      frame_counter: 42,
      wram: {
        wMapGroup: 1,
        wMapNumber: 2,
        wXCoord: 7,
        wYCoord: 9,
      },
    })),
  });

  beforeEach(() => {
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      get: () => "visible",
    });
    (buildUi as jest.Mock).mockClear();
    (Game.create as jest.Mock).mockClear();
    (HTMLCanvasElement.prototype.focus as jest.Mock | undefined)?.mockClear?.();
    (Game.create as jest.Mock).mockResolvedValue(buildGameStub());
    globalThis.fetch = jest.fn(async () => ({
      ok: true,
      status: 200,
      json: async () => ({
        ok: true,
        image: "dGVzdA==",
        width: 160,
        height: 144,
        frame: 1,
      }),
    })) as unknown as typeof globalThis.fetch;
    globalThis.Image = class MockImage {
      onload: null | (() => void) = null;
      onerror: null | (() => void) = null;
      set src(_value: string) {
        this.onload?.();
      }
    } as unknown as typeof Image;
  });

  afterEach(() => {
    HTMLCanvasElement.prototype.getContext = originalGetContext;
    globalThis.fetch = originalFetch;
    globalThis.Image = originalImage;
    jest.useRealTimers();
    if (originalEnv === undefined) {
      delete process.env.NEXT_PUBLIC_LOAD_SLOT;
    } else {
      process.env.NEXT_PUBLIC_LOAD_SLOT = originalEnv;
    }
  });

  it("defaults to the savegame slot when no env slot is configured", async () => {
    delete process.env.NEXT_PUBLIC_LOAD_SLOT;

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" />);
      await flushPromises();
    });

    const [, options] = (Game.create as jest.Mock).mock.calls[0];
    expect(options.loadSlot).toBe(MANUAL_SAVE_SLOT);
    expect(options.preloadMode).toBe("auto");

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("uses the env slot when configured", async () => {
    process.env.NEXT_PUBLIC_LOAD_SLOT = "slot-01";

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" />);
      await flushPromises();
    });

    const [, options] = (Game.create as jest.Mock).mock.calls[0];
    expect(options.loadSlot).toBe("slot-01");

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("prefers an explicit load slot over the env slot", async () => {
    process.env.NEXT_PUBLIC_LOAD_SLOT = "slot-01";

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" loadSlot={MANUAL_SAVE_SLOT} />);
      await flushPromises();
    });

    const [, options] = (Game.create as jest.Mock).mock.calls[0];
    expect(options.loadSlot).toBe(MANUAL_SAVE_SLOT);

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("still preloads the save slot when intro playback is enabled", async () => {
    process.env.NEXT_PUBLIC_LOAD_SLOT = "slot-01";

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" playIntro />);
      await flushPromises();
    });

    const [, options] = (Game.create as jest.Mock).mock.calls[0];
    expect(options.loadSlot).toBe("slot-01");

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("still preloads the save slot when title-screen startup is enabled", async () => {
    process.env.NEXT_PUBLIC_LOAD_SLOT = "slot-01";

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" newGame />);
      await flushPromises();
    });

    const [, options] = (Game.create as jest.Mock).mock.calls[0];
    expect(options.loadSlot).toBe("slot-01");

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("installs visual debug window hooks", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" />);
      await flushPromises();
      await flushPromises();
    });

    expect(typeof (window as any).jump_game_scene).toBe("function");
    expect(typeof (window as any).jump_game_spawn).toBe("function");
    expect(typeof (window as any).get_game_debug_status).toBe("function");
    expect(typeof (window as any).run_game_script).toBe("function");
    expect(typeof (window as any).post_game_event).toBe("function");

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("posts raw visual debug events with text payloads", async () => {
    const game = buildGameStub();
    (Game.create as jest.Mock).mockResolvedValueOnce(game);

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" />);
      await flushPromises();
      await flushPromises();
    });

    await act(async () => {
      (window as any).post_game_event?.({
        type: "keydown",
        key: "K",
        code: "KeyK",
        is_press: true,
        text: "K",
      });
    });

    expect(game.postEvent).toHaveBeenCalledWith(
      expect.objectContaining({
        type: "keydown",
        key: "K",
        code: "KeyK",
        text: "K",
        unicode: "K",
        is_press: true,
      })
    );

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("passes renderer mode to the UI builder", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas rendererMode="text" runtimeMode="local" />);
      await flushPromises();
    });

    expect(buildUi).toHaveBeenCalledWith(
      expect.any(HTMLCanvasElement),
      expect.objectContaining({ rendererMode: "both" })
    );

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("does not recreate the local game when switching renderer modes", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas rendererMode="tile" runtimeMode="local" />);
      await flushPromises();
    });

    expect(Game.create).toHaveBeenCalledTimes(1);

    await act(async () => {
      root.render(<GameCanvas rendererMode="both" runtimeMode="local" />);
      await flushPromises();
    });

    await act(async () => {
      root.render(<GameCanvas rendererMode="text" runtimeMode="local" />);
      await flushPromises();
    });

    expect(Game.create).toHaveBeenCalledTimes(1);

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("destroys the local game on unmount", async () => {
    const game = buildGameStub();
    (Game.create as jest.Mock).mockResolvedValueOnce(game);
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" />);
      await flushPromises();
      await flushPromises();
    });

    await act(async () => {
      root.unmount();
    });

    expect(game.destroy).toHaveBeenCalledTimes(1);
    container.remove();
  });

  it("renders only the tile canvas in tile mode", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas rendererMode="tile" />);
      await flushPromises();
    });

    expect(container.querySelectorAll("canvas")).toHaveLength(1);

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("renders both canvases in both mode", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas rendererMode="both" />);
      await flushPromises();
    });

    expect(container.querySelectorAll("canvas")).toHaveLength(2);

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("renders only the text canvas in text mode", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas rendererMode="text" />);
      await flushPromises();
    });

    expect(container.querySelectorAll("canvas")).toHaveLength(1);

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("uses an explicit server session id for remote frame mode", async () => {
    const drawImage = jest.fn();
    HTMLCanvasElement.prototype.getContext = jest.fn(() => ({
      drawImage,
      clearRect: jest.fn(),
      imageSmoothingEnabled: false,
    })) as typeof HTMLCanvasElement.prototype.getContext;

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    try {
      await act(async () => {
        root.render(
          <GameCanvas
            rendererMode="tile"
            runtimeMode="server"
            readOnly
            sessionId="watch-session-1"
            remoteVisualMode="frame"
            remoteFrameScale={3}
            remoteAdvanceFrames={12}
          />
        );
        await flushPromises();
      });

      expect(globalThis.fetch).toHaveBeenCalledWith(
        expect.stringContaining("/api/arena/frame?session_id=watch-session-1&scale=3&advance=12"),
        expect.objectContaining({ cache: "no-store" })
      );
      expect(drawImage).toHaveBeenCalled();
    } finally {
      await act(async () => {
        root.unmount();
      });
      container.remove();
    }
  });

  it("sizes remote frame canvases from the returned PNG dimensions", async () => {
    globalThis.fetch = jest.fn(async () => ({
      ok: true,
      status: 200,
      json: async () => ({
        ok: true,
        image: "dGVzdA==",
        width: 320,
        height: 288,
        frame: 1,
      }),
    })) as unknown as typeof globalThis.fetch;
    HTMLCanvasElement.prototype.getContext = jest.fn(() => ({
      drawImage: jest.fn(),
      clearRect: jest.fn(),
      imageSmoothingEnabled: false,
    })) as typeof HTMLCanvasElement.prototype.getContext;

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    try {
      await act(async () => {
        root.render(
          <GameCanvas
            rendererMode="tile"
            runtimeMode="server"
            readOnly
            sessionId="watch-session-size"
            remoteVisualMode="frame"
            remoteFrameScale={2}
          />
        );
        await flushPromises();
      });

      const canvas = container.querySelector("canvas") as HTMLCanvasElement;
      expect(canvas.width).toBe(320);
      expect(canvas.height).toBe(288);
    } finally {
      await act(async () => {
        root.unmount();
      });
      container.remove();
    }
  });

  it("keeps server frame mode read-only and does not expose a postEvent handler", async () => {
    const drawImage = jest.fn();
    HTMLCanvasElement.prototype.getContext = jest.fn(() => ({
      drawImage,
      clearRect: jest.fn(),
      imageSmoothingEnabled: false,
    })) as typeof HTMLCanvasElement.prototype.getContext;

    const postEventReady = jest.fn();
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    try {
      await act(async () => {
        root.render(
          <GameCanvas
            rendererMode="tile"
            runtimeMode="server"
            readOnly
            sessionId="watch-session-2"
            remoteVisualMode="frame"
            onPostEventReady={postEventReady}
          />
        );
        await flushPromises();
      });

      expect(postEventReady).toHaveBeenCalledWith(null);
    } finally {
      await act(async () => {
        root.unmount();
      });
      container.remove();
    }
  });

  it("refreshes a remote frame immediately when the realtime refresh key changes", async () => {
    const drawImage = jest.fn();
    HTMLCanvasElement.prototype.getContext = jest.fn(() => ({
      drawImage,
      clearRect: jest.fn(),
      imageSmoothingEnabled: false,
    })) as typeof HTMLCanvasElement.prototype.getContext;

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    try {
      await act(async () => {
        root.render(
          <GameCanvas
            rendererMode="tile"
            runtimeMode="server"
            readOnly
            sessionId="watch-session-refresh"
            remoteVisualMode="frame"
            remoteFrameRefreshKey={0}
            remoteRefreshMs={60_000}
          />
        );
        await flushPromises();
      });

      const fetchMock = globalThis.fetch as jest.Mock;
      expect(fetchMock).toHaveBeenCalledTimes(1);

      await act(async () => {
        root.render(
          <GameCanvas
            rendererMode="tile"
            runtimeMode="server"
            readOnly
            sessionId="watch-session-refresh"
            remoteVisualMode="frame"
            remoteFrameRefreshKey={1}
            remoteRefreshMs={60_000}
          />
        );
        await flushPromises();
      });

      expect(fetchMock).toHaveBeenCalledTimes(2);
      expect(fetchMock).toHaveBeenLastCalledWith(
        expect.stringContaining("/api/arena/frame?session_id=watch-session-refresh"),
        expect.objectContaining({ cache: "no-store" })
      );
    } finally {
      await act(async () => {
        root.unmount();
      });
      container.remove();
    }
  });

  it("does not fetch remote frames while the document is hidden", async () => {
    const drawImage = jest.fn();
    HTMLCanvasElement.prototype.getContext = jest.fn(() => ({
      drawImage,
      clearRect: jest.fn(),
      imageSmoothingEnabled: false,
    })) as typeof HTMLCanvasElement.prototype.getContext;
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      get: () => "hidden",
    });

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    try {
      await act(async () => {
        root.render(
          <GameCanvas
            rendererMode="tile"
            runtimeMode="server"
            readOnly
            sessionId="watch-session-hidden"
            remoteVisualMode="frame"
            remoteFrameRefreshKey={0}
          />
        );
        await flushPromises();
      });

      expect(globalThis.fetch).not.toHaveBeenCalled();

      await act(async () => {
        root.render(
          <GameCanvas
            rendererMode="tile"
            runtimeMode="server"
            readOnly
            sessionId="watch-session-hidden"
            remoteVisualMode="frame"
            remoteFrameRefreshKey={1}
          />
        );
        await flushPromises();
      });

      expect(globalThis.fetch).not.toHaveBeenCalled();
    } finally {
      await act(async () => {
        root.unmount();
      });
      container.remove();
      Object.defineProperty(document, "visibilityState", {
        configurable: true,
        get: () => "visible",
      });
    }
  });

  it("uses timer-based text snapshot refresh instead of a dedicated paint RAF loop", async () => {
    const setTimeoutSpy = jest
      .spyOn(window, "setTimeout")
      .mockImplementation((() => 1) as typeof window.setTimeout);

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    try {
      await act(async () => {
        root.render(<GameCanvas rendererMode="text" runtimeMode="local" />);
        await flushPromises();
      });

      expect(setTimeoutSpy).toHaveBeenCalledWith(expect.any(Function), 100);
    } finally {
      await act(async () => {
        root.unmount();
      });
      setTimeoutSpy.mockRestore();
      container.remove();
    }
  });

  it("skips text canvas rerasterization when the snapshot reference stays stable", async () => {
    jest.useFakeTimers();
    const snapshot = {
      viewportLines: ["OVERWORLD"],
      infoLines: ["INFO"],
      menuLines: null,
      promptLines: null,
      dialogueLines: null,
      viewportTitle: "Overworld",
      infoTitle: "Info",
      marker: null,
      actionLog: [],
    };
    const textUi = {
      getSnapshot: jest.fn(() => snapshot),
    };
    (buildUi as jest.Mock).mockReturnValueOnce({
      ui: {} as BaseUI,
      textUi,
    });

    const putImageData = jest.fn();
    const createImageData = jest.fn((width: number, height: number) => ({
      data: new Uint8ClampedArray(width * height * 4),
      width,
      height,
    }));
    HTMLCanvasElement.prototype.getContext = jest.fn(() => ({
      createImageData,
      getImageData: jest.fn(),
      putImageData,
      imageSmoothingEnabled: false,
    })) as typeof HTMLCanvasElement.prototype.getContext;

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    try {
      await act(async () => {
        root.render(<GameCanvas rendererMode="text" runtimeMode="local" />);
        await flushPromises();
      });

      await act(async () => {
        jest.advanceTimersByTime(100);
        await flushPromises();
      });
      await act(async () => {
        jest.advanceTimersByTime(100);
        await flushPromises();
      });

      expect(putImageData).toHaveBeenCalledTimes(1);

      await act(async () => {
        jest.advanceTimersByTime(100);
        await flushPromises();
      });

      expect(putImageData).toHaveBeenCalledTimes(1);
      expect(textUi.getSnapshot).toHaveBeenCalledTimes(4);
    } finally {
      await act(async () => {
        root.unmount();
      });
      container.remove();
    }
  });

  it("uses pixelated image rendering for desktop crispness", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas rendererMode="tile" />);
      await flushPromises();
    });

    const canvas = container.querySelector("canvas");
    expect(canvas).toBeTruthy();
    expect((canvas as HTMLCanvasElement).style.imageRendering).toBe("pixelated");

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("resizes canvas display dimensions continuously with viewport changes", async () => {
    const originalInnerWidth = window.innerWidth;
    const originalInnerHeight = window.innerHeight;
    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: 1200 });
    Object.defineProperty(window, "innerHeight", { configurable: true, writable: true, value: 900 });

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas rendererMode="tile" runtimeMode="local" />);
      await flushPromises();
    });

    const canvas = container.querySelector("canvas") as HTMLCanvasElement | null;
    expect(canvas).toBeTruthy();
    const firstWidth = Number.parseInt(canvas?.style.width ?? "0", 10);
    expect(firstWidth).toBeGreaterThan(160);

    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: 640 });
    Object.defineProperty(window, "innerHeight", { configurable: true, writable: true, value: 480 });

    await act(async () => {
      window.dispatchEvent(new Event("resize"));
      await flushPromises();
    });

    const secondWidth = Number.parseInt(canvas?.style.width ?? "0", 10);
    expect(secondWidth).toBeGreaterThan(0);
    expect(secondWidth).toBeLessThan(firstWidth);

    await act(async () => {
      root.unmount();
    });
    container.remove();
    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: originalInnerWidth });
    Object.defineProperty(window, "innerHeight", { configurable: true, writable: true, value: originalInnerHeight });
  });

  it("keeps the canvas display inside its wrapper box", async () => {
    const originalInnerWidth = window.innerWidth;
    const originalInnerHeight = window.innerHeight;
    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: 1800 });
    Object.defineProperty(window, "innerHeight", { configurable: true, writable: true, value: 1200 });

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas rendererMode="tile" runtimeMode="local" />);
      await flushPromises();
    });

    const shell = container.firstElementChild as HTMLDivElement | null;
    expect(shell).toBeTruthy();
    Object.defineProperty(container, "clientWidth", {
      configurable: true,
      get: () => 300,
    });
    Object.defineProperty(container, "clientHeight", {
      configurable: true,
      get: () => 220,
    });
    Object.defineProperty(container, "getBoundingClientRect", {
      configurable: true,
      value: () =>
        ({
          width: 300,
          height: 220,
          top: 0,
          right: 300,
          bottom: 220,
          left: 0,
          x: 0,
          y: 0,
          toJSON: () => ({}),
        }) as DOMRect,
    });
    Object.defineProperty(shell as HTMLDivElement, "clientWidth", {
      configurable: true,
      get: () => 300,
    });
    Object.defineProperty(shell as HTMLDivElement, "clientHeight", {
      configurable: true,
      get: () => 220,
    });
    Object.defineProperty(shell as HTMLDivElement, "getBoundingClientRect", {
      configurable: true,
      value: () =>
        ({
          width: 300,
          height: 220,
          top: 0,
          right: 300,
          bottom: 220,
          left: 0,
          x: 0,
          y: 0,
          toJSON: () => ({}),
        }) as DOMRect,
    });

    await act(async () => {
      window.dispatchEvent(new Event("resize"));
      await flushPromises();
    });

    const canvas = container.querySelector("canvas") as HTMLCanvasElement | null;
    expect(canvas).toBeTruthy();
    const displayWidth = Number.parseInt(canvas?.style.width ?? "0", 10);
    const displayHeight = Number.parseInt(canvas?.style.height ?? "0", 10);
    expect(displayWidth).toBeGreaterThan(0);
    expect(displayHeight).toBeGreaterThan(0);
    expect(displayWidth).toBeLessThanOrEqual(300);
    expect(displayHeight).toBeLessThanOrEqual(220);

    await act(async () => {
      root.unmount();
    });
    container.remove();
    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: originalInnerWidth });
    Object.defineProperty(window, "innerHeight", { configurable: true, writable: true, value: originalInnerHeight });
  });

  it("uses integer upscale factors for crisp tile rendering when viewport allows", async () => {
    const originalInnerWidth = window.innerWidth;
    const originalInnerHeight = window.innerHeight;
    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: 1010 });
    Object.defineProperty(window, "innerHeight", { configurable: true, writable: true, value: 730 });

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas rendererMode="tile" runtimeMode="local" />);
      await flushPromises();
    });

    const canvas = container.querySelector("canvas") as HTMLCanvasElement | null;
    expect(canvas).toBeTruthy();
    const displayWidth = Number.parseInt(canvas?.style.width ?? "0", 10);
    const displayHeight = Number.parseInt(canvas?.style.height ?? "0", 10);
    expect(displayWidth).toBeGreaterThanOrEqual(160);
    expect(displayHeight).toBeGreaterThanOrEqual(144);
    expect(displayWidth % 160).toBe(0);
    expect(displayHeight % 144).toBe(0);

    await act(async () => {
      root.unmount();
    });
    container.remove();
    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: originalInnerWidth });
    Object.defineProperty(window, "innerHeight", { configurable: true, writable: true, value: originalInnerHeight });
  });

  it("exposes deterministic window hooks for local stepping and text snapshots", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" />);
      await flushPromises();
    });

    const game = ((Game.create as jest.Mock).mock.results[0]?.value
      ? await (Game.create as jest.Mock).mock.results[0].value
      : null) as GameStub | null;

    const hookWindow = window as Window & {
      advanceTime?: (ms: number) => Promise<void> | void;
      render_game_to_text?: () => string;
    };
    expect(typeof hookWindow.advanceTime).toBe("function");
    expect(typeof hookWindow.render_game_to_text).toBe("function");

    await act(async () => {
      await hookWindow.advanceTime?.(1000 / 30);
    });
    expect(game?.tick).toHaveBeenCalledTimes(2);

    game?.tick.mockClear();
    await act(async () => {
      await hookWindow.advanceTime?.(GB_FRAME_DURATION_MS * 4.49);
    });
    expect(game?.tick).toHaveBeenCalledTimes(4);

    const payload = JSON.parse(hookWindow.render_game_to_text?.() ?? "{}");
    expect(payload.coordinate_system).toBe("origin_top_left_x_right_y_down_tiles");
    expect(payload.player).toEqual(expect.objectContaining({ x: 7, y: 9 }));

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("exposes benchmark hooks for recent and slow frames", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" />);
      await flushPromises();
    });

    const game = ((Game.create as jest.Mock).mock.results[0]?.value
      ? await (Game.create as jest.Mock).mock.results[0].value
      : null) as GameStub | null;

    const hookWindow = window as Window & {
      get_game_benchmark?: (slowFrameThresholdMs?: number) => {
        enabled: boolean;
        thresholdMs: number;
        slowFrames?: Array<{ totalDuration: number }>;
        latestFrame?: { totalDuration: number } | null;
      };
      clear_game_benchmark?: () => void;
    };

    const benchmark = hookWindow.get_game_benchmark?.(5);
    expect(benchmark).toEqual(
      expect.objectContaining({
        enabled: true,
        thresholdMs: 5,
        latestFrame: expect.objectContaining({ totalDuration: 6 }),
      })
    );
    expect(benchmark?.slowFrames).toEqual([
      expect.objectContaining({ totalDuration: 6 }),
    ]);

    act(() => {
      hookWindow.clear_game_benchmark?.();
    });
    expect(game?.clearBenchmark).toHaveBeenCalledTimes(1);

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("profiles text snapshot rendering phases in text mode", async () => {
    jest.useFakeTimers();
    const snapshot = {
      viewportLines: ["OVERWORLD"],
      infoLines: ["INFO"],
      menuLines: null,
      promptLines: null,
      dialogueLines: null,
      viewportTitle: "Overworld",
      infoTitle: "Info",
      marker: null,
      actionLog: [],
    };
    const textUi = {
      getSnapshot: jest.fn(() => snapshot),
    };
    (buildUi as jest.Mock).mockReturnValueOnce({
      ui: {} as BaseUI,
      textUi,
    });

    HTMLCanvasElement.prototype.getContext = jest.fn(() => ({
      createImageData: (width: number, height: number) => ({
        data: new Uint8ClampedArray(width * height * 4),
        width,
        height,
      }),
      getImageData: jest.fn(),
      putImageData: jest.fn(),
      imageSmoothingEnabled: false,
    })) as typeof HTMLCanvasElement.prototype.getContext;

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    try {
      await act(async () => {
        root.render(<GameCanvas rendererMode="text" runtimeMode="local" />);
        await flushPromises();
      });

      await act(async () => {
        jest.advanceTimersByTime(100);
        await flushPromises();
      });
      await act(async () => {
        jest.advanceTimersByTime(100);
        await flushPromises();
      });

      const hookWindow = window as Window & {
        get_text_render_benchmark?: () => {
          enabled: boolean;
          iterations: number;
          phases: Record<string, { count: number }>;
          lastFrame: { painted: boolean } | null;
        };
        clear_text_render_benchmark?: () => void;
      };

      const benchmark = hookWindow.get_text_render_benchmark?.();
      expect(benchmark).toEqual(
        expect.objectContaining({
          enabled: true,
          iterations: expect.any(Number),
          lastFrame: expect.objectContaining({ lineCount: expect.any(Number) }),
        }),
      );
      expect(benchmark?.phases.snapshotRead.count).toBeGreaterThan(0);
      expect(benchmark?.phases.layoutBuild.count).toBeGreaterThan(0);
      expect(benchmark?.phases.paint.count).toBeGreaterThan(0);

      act(() => {
        hookWindow.clear_text_render_benchmark?.();
      });

      expect(hookWindow.get_text_render_benchmark?.()).toEqual(
        expect.objectContaining({
          enabled: false,
          iterations: 0,
          lastFrame: null,
        }),
      );
    } finally {
      await act(async () => {
        root.unmount();
      });
      container.remove();
    }
  });

  it("does not crash text rendering while the compact bitmap font is still loading", async () => {
    jest.useFakeTimers();
    const originalImageLoad = gameEngine.image.load;
    let resolveFontLoad: ((value: { getImageData: () => ImageData }) => void) | null = null;
    gameEngine.image.load = jest.fn(
      () =>
        new Promise((resolve) => {
          resolveFontLoad = resolve;
        }) as never
    );

    const snapshot = {
      viewportLines: ["OVERWORLD"],
      infoLines: ["INFO"],
      menuLines: null,
      promptLines: null,
      dialogueLines: null,
      viewportTitle: "Overworld",
      infoTitle: "Info",
      marker: null,
      actionLog: [],
    };
    const textUi = {
      getSnapshot: jest.fn(() => snapshot),
    };
    (buildUi as jest.Mock).mockReturnValueOnce({
      ui: {} as BaseUI,
      textUi,
    });

    const putImageData = jest.fn();
    HTMLCanvasElement.prototype.getContext = jest.fn(() => ({
      createImageData: (width: number, height: number) => ({
        data: new Uint8ClampedArray(width * height * 4),
        width,
        height,
      }),
      getImageData: jest.fn(),
      putImageData,
      imageSmoothingEnabled: false,
    })) as typeof HTMLCanvasElement.prototype.getContext;

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    try {
      await act(async () => {
        root.render(<GameCanvas rendererMode="text" runtimeMode="local" />);
        await flushPromises();
      });

      await act(async () => {
        jest.advanceTimersByTime(100);
        await flushPromises();
      });

      expect(gameEngine.image.load).toHaveBeenCalled();
      expect(putImageData).not.toHaveBeenCalled();

      await act(async () => {
        resolveFontLoad?.({
          getImageData: () => ({
            data: new Uint8ClampedArray(16 * 16 * 4).fill(255),
            width: 16,
            height: 16,
            colorSpace: "srgb",
          } as ImageData),
        });
        await flushPromises();
      });

      await act(async () => {
        jest.advanceTimersByTime(100);
        await flushPromises();
      });

      expect(putImageData).toHaveBeenCalled();
    } finally {
      gameEngine.image.load = originalImageLoad;
      await act(async () => {
        root.unmount();
      });
      container.remove();
    }
  });

  it("ignores key events when canvas focus is lost and no modal input capture is active", async () => {
    const game = buildGameStub();
    (Game.create as jest.Mock).mockResolvedValueOnce(game);

    const container = document.createElement("div");
    document.body.appendChild(container);
    const input = document.createElement("input");
    document.body.appendChild(input);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" />);
      await flushPromises();
    });

    input.focus();
    act(() => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "z", code: "KeyZ", bubbles: true }));
    });

    expect(game.postEvent).not.toHaveBeenCalled();

    await act(async () => {
      root.unmount();
    });
    input.remove();
    container.remove();
  });

  it("accepts key events without canvas focus while Unown modal input is active", async () => {
    const game = buildGameStub();
    game.getGameState.mockReturnValue({
      frame_counter: 42,
      wram: {
        wMapGroup: 1,
        wMapNumber: 2,
        wXCoord: 7,
        wYCoord: 9,
        wUnownState: 1,
      },
    });
    (Game.create as jest.Mock).mockResolvedValueOnce(game);

    const container = document.createElement("div");
    document.body.appendChild(container);
    const input = document.createElement("input");
    document.body.appendChild(input);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" />);
      await flushPromises();
    });

    input.focus();
    act(() => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "z", code: "KeyZ", bubbles: true }));
    });

    expect(game.postEvent).toHaveBeenCalledTimes(1);

    await act(async () => {
      root.unmount();
    });
    input.remove();
    container.remove();
  });

  it("ignores repeated control keydown events so held-input ordering stays deterministic", async () => {
    const game = buildGameStub();
    (Game.create as jest.Mock).mockResolvedValueOnce(game);

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" />);
      await flushPromises();
    });

    const canvas = container.querySelector("canvas") as HTMLCanvasElement | null;
    canvas?.focus();

    act(() => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", code: "ArrowRight", bubbles: true }));
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "ArrowRight",
          code: "ArrowRight",
          repeat: true,
          bubbles: true,
        })
      );
      window.dispatchEvent(new KeyboardEvent("keyup", { key: "ArrowRight", code: "ArrowRight", bubbles: true }));
    });

    expect(game.postEvent).toHaveBeenCalledTimes(2);
    expect(game.postEvent.mock.calls[0][0]?.type).toBe("keydown");
    expect(game.postEvent.mock.calls[1][0]?.type).toBe("keyup");

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("resolves the bound keyboard A key before ignoring duplicate held keydowns", async () => {
    const game = buildGameStub();
    (Game.create as jest.Mock).mockResolvedValueOnce(game);

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" />);
      await flushPromises();
    });

    const canvas = container.querySelector("canvas") as HTMLCanvasElement | null;
    canvas?.focus();

    act(() => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "a", code: "KeyA", bubbles: true }));
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "a", code: "KeyA", bubbles: true }));
      window.dispatchEvent(new KeyboardEvent("keyup", { key: "a", code: "KeyA", bubbles: true }));
    });

    expect(game.postEvent).toHaveBeenCalledTimes(2);
    expect(game.postEvent.mock.calls[0][0]).toMatchObject({
      type: "keydown",
      key: "a",
      code: "KeyA",
      direction: null,
      button: "a",
      is_press: true,
    });
    expect(game.postEvent.mock.calls[1][0]).toMatchObject({
      type: "keyup",
      key: "a",
      code: "KeyA",
      direction: null,
      button: "a",
      is_press: false,
    });

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("clears stuck held keyboard controls when the Electron window blurs", async () => {
    const game = buildGameStub();
    (Game.create as jest.Mock).mockResolvedValueOnce(game);
    const inputStateSpy = jest.fn();

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GameCanvas runtimeMode="local" onInputStateChange={inputStateSpy} />);
      await flushPromises();
    });

    const canvas = container.querySelector("canvas") as HTMLCanvasElement | null;
    canvas?.focus();

    act(() => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "z", code: "KeyZ", bubbles: true }));
      window.dispatchEvent(new Event("blur"));
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "z", code: "KeyZ", bubbles: true }));
    });

    const keydownCalls = game.postEvent.mock.calls.filter(([event]) => event?.type === "keydown");
    expect(keydownCalls).toHaveLength(2);
    expect(inputStateSpy).toHaveBeenLastCalledWith({
      pressedButtons: ["a"],
      pressedKeys: ["KeyZ"],
    });

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("does not synthesize directional repeats during overworld movement", async () => {
    const game = buildGameStub();
    (Game.create as jest.Mock).mockResolvedValueOnce(game);

    const originalRequestAnimationFrame = window.requestAnimationFrame;
    const originalCancelAnimationFrame = window.cancelAnimationFrame;
    let rafId = 0;
    const rafCallbacks = new Map<number, FrameRequestCallback>();
    const stepRaf = (timestamp: number): void => {
      const callbacks = Array.from(rafCallbacks.entries());
      rafCallbacks.clear();
      for (const [, callback] of callbacks) {
        callback(timestamp);
      }
    };
    window.requestAnimationFrame = ((callback: FrameRequestCallback): number => {
      rafId += 1;
      rafCallbacks.set(rafId, callback);
      return rafId;
    }) as typeof window.requestAnimationFrame;
    window.cancelAnimationFrame = ((id: number): void => {
      rafCallbacks.delete(id);
    }) as typeof window.cancelAnimationFrame;

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    try {
      await act(async () => {
        root.render(<GameCanvas runtimeMode="local" />);
        await flushPromises();
      });

      const canvas = container.querySelector("canvas") as HTMLCanvasElement | null;
      canvas?.focus();

      act(() => {
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", code: "ArrowDown", bubbles: true }));
      });
      const baseKeydownCalls = game.postEvent.mock.calls.filter(([event]) => event?.type === "keydown").length;
      expect(baseKeydownCalls).toBe(1);

      await act(async () => {
        for (let frame = 1; frame <= 20; frame += 1) {
          stepRaf(frame * GB_FRAME_DURATION_MS);
        }
      });

      const heldKeydownCalls = game.postEvent.mock.calls.filter(([event]) => event?.type === "keydown").length;
      expect(heldKeydownCalls).toBe(1);

      act(() => {
        window.dispatchEvent(new KeyboardEvent("keyup", { key: "ArrowDown", code: "ArrowDown", bubbles: true }));
      });
      const keydownCallsAfterRelease = game.postEvent.mock.calls.filter(([event]) => event?.type === "keydown").length;
      expect(keydownCallsAfterRelease).toBe(1);

      await act(async () => {
        for (let frame = 21; frame <= 60; frame += 1) {
          stepRaf(frame * GB_FRAME_DURATION_MS);
        }
      });

      const finalKeydownCalls = game.postEvent.mock.calls.filter(([event]) => event?.type === "keydown").length;
      expect(finalKeydownCalls).toBe(keydownCallsAfterRelease);
    } finally {
      await act(async () => {
        root.unmount();
      });
      container.remove();
      window.requestAnimationFrame = originalRequestAnimationFrame;
      window.cancelAnimationFrame = originalCancelAnimationFrame;
    }
  });

  it("applies deterministic GB-style directional repeats for menu navigation", async () => {
    const game = buildGameStub();
    game.getState.mockReturnValue("menu");
    (Game.create as jest.Mock).mockResolvedValueOnce(game);

    const originalRequestAnimationFrame = window.requestAnimationFrame;
    const originalCancelAnimationFrame = window.cancelAnimationFrame;
    let rafId = 0;
    const rafCallbacks = new Map<number, FrameRequestCallback>();
    const stepRaf = (timestamp: number): void => {
      const callbacks = Array.from(rafCallbacks.entries());
      rafCallbacks.clear();
      for (const [, callback] of callbacks) {
        callback(timestamp);
      }
    };
    window.requestAnimationFrame = ((callback: FrameRequestCallback): number => {
      rafId += 1;
      rafCallbacks.set(rafId, callback);
      return rafId;
    }) as typeof window.requestAnimationFrame;
    window.cancelAnimationFrame = ((id: number): void => {
      rafCallbacks.delete(id);
    }) as typeof window.cancelAnimationFrame;

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    try {
      await act(async () => {
        root.render(<GameCanvas runtimeMode="local" />);
        await flushPromises();
      });

      const canvas = container.querySelector("canvas") as HTMLCanvasElement | null;
      canvas?.focus();

      act(() => {
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", code: "ArrowDown", bubbles: true }));
      });
      const baseKeydownCalls = game.postEvent.mock.calls.filter(([event]) => event?.type === "keydown").length;
      expect(baseKeydownCalls).toBe(1);

      await act(async () => {
        for (let frame = 1; frame <= 20; frame += 1) {
          stepRaf(frame * GB_FRAME_DURATION_MS);
        }
      });

      const heldKeydownCalls = game.postEvent.mock.calls.filter(([event]) => event?.type === "keydown").length;
      expect(heldKeydownCalls).toBeGreaterThan(1);
    } finally {
      await act(async () => {
        root.unmount();
      });
      container.remove();
      window.requestAnimationFrame = originalRequestAnimationFrame;
      window.cancelAnimationFrame = originalCancelAnimationFrame;
    }
  });
});

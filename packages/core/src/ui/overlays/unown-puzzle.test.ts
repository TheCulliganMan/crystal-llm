import fs from "node:fs";
import path from "node:path";
import { GB_FRAME_RATE } from "@pokecrystal/core/core/gb-timing";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EngineUnownPuzzle } from "@pokecrystal/core/engine/games/unown-puzzle";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import * as asyncLoop from "@pokecrystal/core/ui/async-loop";
import * as unownPuzzleAssets from "@pokecrystal/assets/content/data/unown-puzzles/unown-puzzle-assets";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { Surface } from "@pokecrystal/core/ui/surface";
import { UnownPuzzleOverlay } from "@pokecrystal/core/ui/overlays/unown-puzzle";

const TILE_SIZE = 8;

const CURSOR_COLOR: [number, number, number, number] = [220, 40, 40, 255];
const PIECE_COLOR: [number, number, number, number] = [40, 180, 40, 255];

const makeSolidTile = (color: [number, number, number, number]): Surface => {
  const tile = new Surface(TILE_SIZE, TILE_SIZE);
  tile.fill(color);
  return tile;
};

const buildCursorTestTiles = (): Record<number, Surface> => {
  const tiles: Record<number, Surface> = {};
  [0xe0, 0xe1, 0xe2, 0xe3].forEach((id) => {
    tiles[id] = makeSolidTile(CURSOR_COLOR);
  });
  [0x7f, 0xed, 0xee, 0xef, 0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd].forEach((id) => {
    tiles[id] = makeSolidTile([0, 0, 0, 255]);
  });
  [0, 1, 2, 12, 13, 14, 24, 25, 26].forEach((id) => {
    tiles[id] = makeSolidTile(PIECE_COLOR);
  });
  return tiles;
};

const emptyLayout = (): number[][] =>
  Array.from({ length: 6 }, () => Array.from({ length: 6 }, () => 0));

const createOverlayHarness = () => {
  const gameState = createInitialGameState();
  const rng = new HardwareRNG(gameState);
  const puzzle = new EngineUnownPuzzle(rng);
  const eventQueue = gameEngine.event.createQueue();
  const overlay = new UnownPuzzleOverlay(
    {
      screen: new Surface(160, 144),
      update: jest.fn(),
      eventQueue,
    },
    gameState,
    null,
  );
  const target = new Surface(160, 144);
  (overlay as unknown as { puzzle: EngineUnownPuzzle }).puzzle = puzzle;
  (overlay as unknown as { tiles: Record<number, Surface> }).tiles = buildCursorTestTiles();
  (overlay as unknown as { cursor: { position: number } }).cursor.position = 0;
  return { overlay, puzzle, target, eventQueue, gameState };
};

const createRunSession = () => ({
  puzzleId: "KABUTO",
  heldKeys: new Set<string>(),
  pressedKeys: new Set<string>(),
  heldButtonCodes: new Set<number>(),
  pressedButtonCodes: new Set<number>(),
  solved: false,
  screen: new Surface(160, 144),
  previousInMenu: 0,
});

const stepOverlay = (
  overlay: UnownPuzzleOverlay,
  session: ReturnType<typeof createRunSession>,
): boolean =>
  (overlay as unknown as { stepRunSession: (session: typeof session) => boolean }).stepRunSession(session);

const cursorPosition = (overlay: UnownPuzzleOverlay): number =>
  (overlay as unknown as { cursor: { position: number } }).cursor.position;

beforeEach(() => {
  unownPuzzleAssets.setUnownPuzzleAssetLoader((assetPath) => {
    const fullPath = path.resolve(__dirname, "../../../../../apps/web", assetPath);
    return new Uint8Array(fs.readFileSync(fullPath));
  });
});

const emptyLayoutWithLeadingPiece: number[][] = [
  [1, 0, 0, 0, 0, 0],
  [2, 3, 4, 5, 6, 7],
  [8, 9, 10, 11, 12, 13],
  [14, 15, 16, 0, 0, 0],
  [0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0],
];

describe("UnownPuzzleOverlay cursor rendering", () => {
  it("loads unown puzzle graphics from exported .2bpp assets without probing .2bpp.lz", () => {
    const requestedPaths: string[] = [];
    unownPuzzleAssets.setUnownPuzzleAssetLoader((assetPath) => {
      requestedPaths.push(assetPath);
      if (assetPath.endsWith("tile_borders.2bpp")) {
        return new Uint8Array(8 * 16);
      }
      if (assetPath.endsWith("start_cancel.2bpp")) {
        return new Uint8Array(4 * 16);
      }
      if (assetPath.endsWith("kabuto.2bpp")) {
        return new Uint8Array(36 * 16);
      }
      throw new Error(`Unexpected unown puzzle asset request: ${assetPath}`);
    });

    expect(unownPuzzleAssets.loadStartCancelTiles()).toHaveLength(4);
    expect(unownPuzzleAssets.convertPuzzleTiles("KABUTO")).toHaveLength(144);

    expect(requestedPaths).toEqual([
      "assets/gfx/unown_puzzle/start_cancel.2bpp",
      "assets/gfx/unown_puzzle/kabuto.2bpp",
      "assets/gfx/unown_puzzle/tile_borders.2bpp",
    ]);
    expect(requestedPaths.some((assetPath) => assetPath.endsWith(".2bpp.lz"))).toBe(false);
  });

  it("decodes every ASM Unown puzzle id in script-constant order", () => {
    expect(unownPuzzleAssets.PUZZLE_IDS).toEqual(["KABUTO", "OMANYTE", "AERODACTYL", "HOOH"]);

    for (const puzzleId of unownPuzzleAssets.PUZZLE_IDS) {
      const tiles = unownPuzzleAssets.convertPuzzleTiles(puzzleId);

      expect(tiles).toHaveLength(144);
      tiles.forEach((tile) => {
        expect(tile).toHaveLength(16);
      });
    }
  });

  it("uses idle cursor graphics when not holding a piece, even over an occupied tile", () => {
    const { overlay, puzzle, target } = createOverlayHarness();
    const layout = emptyLayout();
    layout[0][0] = 1;
    puzzle.loadState(layout);

    (overlay as unknown as { drawCursor: (surface: Surface) => void }).drawCursor(target);

    expect(target.getAt(16, 16)).toEqual(CURSOR_COLOR);
  });

  it("anchors cursor sprites using raw GB OAM offsets from UnownPuzzleCoordData", () => {
    const { overlay, puzzle, target } = createOverlayHarness();
    puzzle.loadState(emptyLayout());

    (overlay as unknown as { drawCursor: (surface: Surface) => void }).drawCursor(target);

    expect(target.getAt(10, 2)).toEqual(CURSOR_COLOR);
    expect(target.getAt(34, 30)).toEqual([0, 0, 0, 0]);
  });

  it("uses held-piece graphics while carrying a puzzle piece", () => {
    const { overlay, puzzle, target } = createOverlayHarness();
    puzzle.loadState(emptyLayout(), { holding_piece: 1 });

    (overlay as unknown as { drawCursor: (surface: Surface) => void }).drawCursor(target);

    expect(target.getAt(16, 16)).toEqual(PIECE_COLOR);
  });

  it("moves the cursor from TUI button-direction events", () => {
    const { overlay, puzzle, eventQueue } = createOverlayHarness();
    puzzle.loadState(emptyLayoutWithLeadingPiece);
    const session = createRunSession();

    gameEngine.event.post({ type: "keydown", button: "right" }, eventQueue);
    stepOverlay(overlay, session);

    expect(cursorPosition(overlay)).toBe(1);
  });

  it.each([
    ["button token", { type: "keydown", button: "right" }],
    ["direction token", { type: "keydown", direction: "right" }],
    ["DOM code token", { type: "keydown", code: "ArrowRight" }],
    ["DOM key token", { type: "keydown", key: "ArrowRight" }],
    [
      "GameCanvas browser KeyboardEvent payload",
      new gameEngine.event.Event(gameEngine.KEYDOWN, {
        key: "ArrowRight",
        code: "ArrowRight",
        is_press: true,
      }),
    ],
    ["button press event", { type: "button", button: "right", is_press: true }],
    ["uppercase button token", { type: "keydown", button: "RIGHT" }],
    ["numeric DOM keycode", { type: 768, code: 39 }],
  ])("moves the cursor right from %s", (_label, event) => {
    const { overlay, puzzle, eventQueue } = createOverlayHarness();
    puzzle.loadState(emptyLayoutWithLeadingPiece);
    const session = createRunSession();

    gameEngine.event.post(event, eventQueue);
    stepOverlay(overlay, session);

    expect(cursorPosition(overlay)).toBe(1);
  });

  it("moves the cursor in all four directions from button tokens", () => {
    const { overlay, puzzle, eventQueue } = createOverlayHarness();
    puzzle.loadState(emptyLayoutWithLeadingPiece);
    const session = createRunSession();
    (overlay as unknown as { cursor: { position: number } }).cursor.position = 7;

    gameEngine.event.post({ type: "keydown", button: "up" }, eventQueue);
    stepOverlay(overlay, session);
    expect(cursorPosition(overlay)).toBe(1);

    gameEngine.event.post({ type: "keyup", button: "up" }, eventQueue);
    gameEngine.event.post({ type: "keydown", button: "left" }, eventQueue);
    stepOverlay(overlay, session);
    expect(cursorPosition(overlay)).toBe(0);

    gameEngine.event.post({ type: "keyup", button: "left" }, eventQueue);
    gameEngine.event.post({ type: "keydown", button: "down" }, eventQueue);
    stepOverlay(overlay, session);
    expect(cursorPosition(overlay)).toBe(6);

    gameEngine.event.post({ type: "keyup", button: "down" }, eventQueue);
    gameEngine.event.post({ type: "keydown", button: "right" }, eventQueue);
    stepOverlay(overlay, session);
    expect(cursorPosition(overlay)).toBe(7);
  });

  it("moves the cursor from a quick browser tap when keydown and keyup land in the same frame", () => {
    const { overlay, puzzle, eventQueue, gameState } = createOverlayHarness();
    puzzle.loadState(emptyLayoutWithLeadingPiece);
    gameState.hram.hInMenu = 1;
    const session = createRunSession();

    gameEngine.event.post(
      new gameEngine.event.Event(gameEngine.KEYDOWN, {
        key: "ArrowRight",
        code: "ArrowRight",
        is_press: true,
      }),
      eventQueue,
    );
    gameEngine.event.post(
      new gameEngine.event.Event(gameEngine.KEYUP, {
        key: "ArrowRight",
        code: "ArrowRight",
        is_press: false,
      }),
      eventQueue,
    );

    stepOverlay(overlay, session);

    expect(cursorPosition(overlay)).toBe(1);
  });

  it("continues moving the held cursor after the text-delay repeat window expires", () => {
    const { overlay, puzzle, eventQueue, gameState } = createOverlayHarness();
    puzzle.loadState(emptyLayoutWithLeadingPiece);
    gameState.hram.hInMenu = 1;
    const session = createRunSession();

    gameEngine.event.post({ type: "keydown", button: "right" }, eventQueue);
    stepOverlay(overlay, session);
    expect(cursorPosition(overlay)).toBe(1);

    for (let frame = 0; frame < 14; frame += 1) {
      stepOverlay(overlay, session);
    }
    expect(cursorPosition(overlay)).toBe(1);

    stepOverlay(overlay, session);
    expect(cursorPosition(overlay)).toBe(2);
  });

  it("exits a solved puzzle from a quick browser A tap when keydown and keyup land in the same frame", () => {
    const { overlay, puzzle, eventQueue, gameState } = createOverlayHarness();
    puzzle.loadState(emptyLayout());
    gameState.hram.hInMenu = 1;
    (overlay as unknown as { awaitingAck: boolean }).awaitingAck = true;
    const session = createRunSession();

    gameEngine.event.post(
      new gameEngine.event.Event(gameEngine.KEYDOWN, {
        key: "z",
        code: "KeyZ",
        is_press: true,
      }),
      eventQueue,
    );
    gameEngine.event.post(
      new gameEngine.event.Event(gameEngine.KEYUP, {
        key: "z",
        code: "KeyZ",
        is_press: false,
      }),
      eventQueue,
    );

    expect(stepOverlay(overlay, session)).toBe(true);
    expect(session.solved).toBe(true);
    expect(gameState.wram.wUnownState).toBe(overlay.STATE_SOLVED);
  });
});

describe("UnownPuzzleOverlay runAsync pacing", () => {
  it("uses GB frame cadence for synchronous run pacing", () => {
    const gameState = createInitialGameState();
    const rng = new HardwareRNG(gameState);
    const overlay = new UnownPuzzleOverlay(
      {
        screen: new Surface(160, 144),
        update: jest.fn(),
      },
      gameState,
      null,
    );

    const fakeSession = {
      puzzleId: "KABUTO",
      heldKeys: new Set<string>(),
      pressedKeys: new Set<string>(),
      heldButtonCodes: new Set<number>(),
      pressedButtonCodes: new Set<number>(),
      solved: false,
      screen: new Surface(160, 144),
      previousInMenu: 0,
    };

    const overlayHarness = overlay as unknown as {
      beginRunSession: jest.Mock;
      stepRunSession: jest.Mock;
      finishRunSession: jest.Mock;
    };
    overlayHarness.beginRunSession = jest.fn(() => fakeSession);
    overlayHarness.stepRunSession = jest.fn()
      .mockReturnValueOnce(false)
      .mockReturnValueOnce(true);
    overlayHarness.finishRunSession = jest.fn();

    const tickMock = jest.spyOn(gameEngine.time.Clock.prototype, "tick").mockImplementation(() => undefined);

    try {
      const solved = overlay.run("KABUTO", rng, null);

      expect(solved).toBe(false);
      expect(overlayHarness.stepRunSession).toHaveBeenCalledTimes(2);
      expect(tickMock).toHaveBeenCalledTimes(1);
      const [fps] = tickMock.mock.calls[0] ?? [];
      expect(Number(fps)).toBeCloseTo(GB_FRAME_RATE, 6);
    } finally {
      tickMock.mockRestore();
    }
  });

  it("uses frame awaits without busy-wait clock ticks between async steps", async () => {
    const gameState = createInitialGameState();
    const rng = new HardwareRNG(gameState);
    const overlay = new UnownPuzzleOverlay(
      {
        screen: new Surface(160, 144),
        update: jest.fn(),
      },
      gameState,
      null,
    );

    const fakeSession = {
      puzzleId: "KABUTO",
      heldKeys: new Set<string>(),
      pressedKeys: new Set<string>(),
      heldButtonCodes: new Set<number>(),
      pressedButtonCodes: new Set<number>(),
      solved: false,
      screen: new Surface(160, 144),
      previousInMenu: 0,
    };

    const overlayHarness = overlay as unknown as {
      beginRunSession: jest.Mock;
      stepRunSession: jest.Mock;
      finishRunSession: jest.Mock;
    };
    overlayHarness.beginRunSession = jest.fn(() => fakeSession);
    overlayHarness.stepRunSession = jest.fn()
      .mockReturnValueOnce(false)
      .mockReturnValueOnce(true);
    overlayHarness.finishRunSession = jest.fn();

    const nextFrameMock = jest.spyOn(asyncLoop, "nextFrame").mockResolvedValue(undefined);
    const tickMock = jest.spyOn(gameEngine.time.Clock.prototype, "tick").mockImplementation(() => undefined);

    const solved = await overlay.runAsync("KABUTO", rng, null);

    expect(solved).toBe(false);
    expect(overlayHarness.stepRunSession).toHaveBeenCalledTimes(2);
    expect(nextFrameMock).toHaveBeenCalledTimes(1);
    expect(tickMock).not.toHaveBeenCalled();

    tickMock.mockRestore();
    nextFrameMock.mockRestore();
  });

  it("waits through BG-map and sound windows before releasing input", () => {
    const gameState = createInitialGameState();
    const audioEngine = {
      isSoundPlaying: jest.fn(),
    } as unknown as AudioEngine;
    const overlay = new UnownPuzzleOverlay(
      {
        screen: new Surface(160, 144),
        update: jest.fn(),
      },
      gameState,
      audioEngine,
    );

    let soundCheckCalls = 0;
    audioEngine.isSoundPlaying.mockImplementation(() => {
      soundCheckCalls += 1;
      return soundCheckCalls <= 3;
    });

    (overlay as unknown as { beginSfxWait: (soundId: string) => void }).beginSfxWait("SFX_MEGA_KICK");
    const state = overlay as unknown as {
      isActionBlockedBySfxWait: () => boolean;
    };

    let blocked = true;
    let loops = 0;
    while (blocked && loops < 8) {
      blocked = state.isActionBlockedBySfxWait();
      loops += 1;
    }
    expect(blocked).toBe(false);
    expect(loops).toBeGreaterThan(1);
    expect(audioEngine.isSoundPlaying).toHaveBeenCalledTimes(4);
  });

  it("unblocks input when a stuck SFX never stops", () => {
    const gameState = createInitialGameState();
    const audioEngine = {
      isSoundPlaying: jest.fn(() => true),
    } as unknown as AudioEngine;
    const overlay = new UnownPuzzleOverlay(
      {
        screen: new Surface(160, 144),
        update: jest.fn(),
      },
      gameState,
      audioEngine,
    );

    (overlay as unknown as { beginSfxWait: (soundId: string) => void }).beginSfxWait("SFX_MEGA_KICK");

    let loops = 0;
    let blocked = true;
    while (blocked && loops < 1000) {
      blocked = (overlay as unknown as { isActionBlockedBySfxWait: () => boolean }).isActionBlockedBySfxWait();
      loops += 1;
    }

    expect(blocked).toBe(false);
    expect(loops).toBeLessThan(1000);
    expect(loops).toBeGreaterThan(1);
  });
});

"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { assertAsmScale, assertAsmScreenDimensions, assertAsmUiInvariants } from "@/app/asm-rendering-invariants";
import { MAX_COINS } from "@pokecrystal/core/core/constants";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { mapKeyToButton, mapKeyToDirection } from "@pokecrystal/core/input/controls";
import {
  REEL_LENGTH,
  REEL_TILEMAPS,
  SlotMachine,
  SlotMachineMode,
  SlotSymbol,
  type SlotMachineResult,
} from "@pokecrystal/core/engine/games/slots";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile/constants";
import { DomCanvasUI } from "@pokecrystal/core/ui/dom-canvas-ui";
import { gameEngine, Surface } from "@pokecrystal/core/ui/game-engine";
import { TextRenderer } from "@pokecrystal/core/ui/text/text-renderer";
import { DEFAULT_GAME_CORNER_TAB, type GameCornerTab } from "./tabs";
import {
  SLOT_ICON_SURFACE_SIZE,
  SLOT_ICON_TILE_SIZE,
  SLOT_ICON_TILE_STRIDE,
  slotIconTileIndices,
} from "./slot-icon-layout";
import { SLOT_REEL_X_TILES, SLOT_REEL_Y_TILES } from "./slot-reel-layout";

const STARTING_COINS = 200;
const TILE_CANVAS_SCALE = 2;
const TILE_CANVAS_WIDTH = 160 * TILE_CANVAS_SCALE;
const TILE_CANVAS_HEIGHT = 144 * TILE_CANVAS_SCALE;
const TILE_DISPLAY_MARGIN_PX = 32;

const resolveSlotsUiTilesPath = () => getAssetPath("gfx", "slots", "slots_1.png");
const resolveSlotsSymbolTilesPath = () => getAssetPath("gfx", "slots", "slots_2.png");
const resolveSlotsTilemapPath = () => getAssetPath("gfx", "slots", "slots.tilemap");

const SLOTS_TILEMAP_WIDTH = 20;
const SLOTS_TILEMAP_HEIGHT = 12;
const SLOTS_TILEMAP_LENGTH = SLOTS_TILEMAP_WIDTH * SLOTS_TILEMAP_HEIGHT;
const SLOTS_VTILES2_OVERLAY_START_TILE = 0x25;
const SLOT_COIN_COUNT_TILE_X = 5;
const SLOT_PAYOUT_TILE_X = 11;
const SLOT_COUNTER_TILE_Y = 1;
const SLOT_REEL_SPIN_STEP_MS = 42;
const SLOT_REEL_STOP_MS = [420, 620, 820] as const;

const TILE_COLORS = {
  background: [226, 233, 242] as [number, number, number],
  fill: [240, 246, 252] as [number, number, number],
  ink: [33, 41, 56] as [number, number, number],
  accent: [28, 86, 168] as [number, number, number],
};

const SLOT_SHORT_CODES: Record<SlotSymbol, string> = {
  [SlotSymbol.SEVEN]: "7N",
  [SlotSymbol.POKEBALL]: "PB",
  [SlotSymbol.CHERRY]: "CH",
  [SlotSymbol.PIKACHU]: "PK",
  [SlotSymbol.SQUIRTLE]: "SQ",
  [SlotSymbol.STARYU]: "ST",
};

const clampCoins = (coins: number): number => Math.max(0, Math.min(MAX_COINS, Math.trunc(coins)));

type ReelWindow = [SlotSymbol, SlotSymbol, SlotSymbol];
type ReelWindows = [ReelWindow, ReelWindow, ReelWindow];

const wrapReelIndex = (index: number): number => ((index % REEL_LENGTH) + REEL_LENGTH) % REEL_LENGTH;

const buildEngineReelWindow = (reelIndex: number, offset: number): ReelWindow => {
  const reel = REEL_TILEMAPS[reelIndex];
  const start = wrapReelIndex(offset);
  return [
    reel[start],
    reel[wrapReelIndex(start + 1)],
    reel[wrapReelIndex(start + 2)],
  ];
};

const buildVisibleReelWindow = (reelIndex: number, offset: number): ReelWindow => {
  const [bottom, middle, top] = buildEngineReelWindow(reelIndex, offset);
  return [top, middle, bottom];
};

const buildReelWindowsFromOffsets = (offsets: [number, number, number]): ReelWindows => ([
  buildVisibleReelWindow(0, offsets[0]),
  buildVisibleReelWindow(1, offsets[1]),
  buildVisibleReelWindow(2, offsets[2]),
]);

const visibleWindowsFromEngineWindows = (windows: ReelWindows): ReelWindows => [
  [windows[0][2], windows[0][1], windows[0][0]],
  [windows[1][2], windows[1][1], windows[1][0]],
  [windows[2][2], windows[2][1], windows[2][0]],
];

const findReelOffsetForWindow = (reelIndex: number, window: ReelWindow): number => {
  const reel = REEL_TILEMAPS[reelIndex];
  for (let offset = 0; offset < REEL_LENGTH; offset += 1) {
    const top = reel[offset];
    const middle = reel[wrapReelIndex(offset + 1)];
    const bottom = reel[wrapReelIndex(offset + 2)];
    if (top === window[0] && middle === window[1] && bottom === window[2]) {
      return offset;
    }
  }
  throw new Error(`ASM reel mapping mismatch for reel ${reelIndex}.`);
};

const loadAssetBytes = async (assetPath: string): Promise<Uint8Array> => {
  const response = await fetch(assetPath);
  if (!response.ok) {
    throw new Error(`Failed to load ${assetPath}: ${response.status}`);
  }
  return new Uint8Array(await response.arrayBuffer());
};

type GameCornerClientProps = {
  initialTab?: GameCornerTab;
};

export const GameCornerClient = ({ initialTab = DEFAULT_GAME_CORNER_TAB }: GameCornerClientProps) => {
  assertAsmScale(TILE_CANVAS_SCALE, "GameCornerClient");
  if (initialTab !== DEFAULT_GAME_CORNER_TAB) {
    throw new Error(`Unsupported Game Corner tab: ${initialTab}`);
  }

  const rngStateRef = useRef(createInitialGameState());
  const [coins, setCoins] = useState<number>(STARTING_COINS);
  const [slotBet, setSlotBet] = useState<1 | 2 | 3>(3);
  const [slotResult, setSlotResult] = useState<SlotMachineResult | null>(null);
  const [animatedWindows, setAnimatedWindows] = useState<ReelWindows | null>(null);
  const [isSpinning, setIsSpinning] = useState(false);
  const [slotMessage, setSlotMessage] = useState<string | null>(null);

  const tileCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const canvasShellRef = useRef<HTMLDivElement | null>(null);
  const canvasFrameRef = useRef<HTMLDivElement | null>(null);
  const tileUiRef = useRef<DomCanvasUI | null>(null);
  const tileTextRef = useRef<TextRenderer | null>(null);
  const slotUiSheetRef = useRef<Surface | null>(null);
  const slotSymbolSheetRef = useRef<Surface | null>(null);
  const slotTilemapRef = useRef<Uint8Array | null>(null);
  const slotIconCacheRef = useRef<Map<SlotSymbol, Surface>>(new Map());
  const spinFrameRequestRef = useRef<number | null>(null);
  const spinStartMsRef = useRef<number>(0);
  const spinBaseOffsetsRef = useRef<[number, number, number]>([0, 0, 0]);
  const spinCurrentOffsetsRef = useRef<[number, number, number]>([0, 0, 0]);
  const spinTargetOffsetsRef = useRef<[number, number, number] | null>(null);
  const spinFinalMessageRef = useRef<string | null>(null);

  const [tileRendererReady, setTileRendererReady] = useState(false);
  const [tileRendererError, setTileRendererError] = useState<string | null>(null);
  const [slotGraphicsReady, setSlotGraphicsReady] = useState(false);
  const [slotGraphicsError, setSlotGraphicsError] = useState<string | null>(null);

  const getSlotIconSurface = useCallback((symbol: SlotSymbol): Surface | null => {
    const sheet = slotSymbolSheetRef.current;
    if (!sheet) {
      return null;
    }

    const cached = slotIconCacheRef.current.get(symbol);
    if (cached) {
      return cached;
    }

    const baseTileIndex = symbol * SLOT_ICON_TILE_STRIDE;
    const columns = Math.floor(sheet.get_width() / SLOT_ICON_TILE_SIZE);
    const totalTiles = columns * Math.floor(sheet.get_height() / SLOT_ICON_TILE_SIZE);
    if (baseTileIndex < 0 || baseTileIndex + 3 >= totalTiles) {
      return null;
    }

    const icon = new Surface(SLOT_ICON_SURFACE_SIZE, SLOT_ICON_SURFACE_SIZE);
    const tileLayout = slotIconTileIndices(baseTileIndex);
    const destPositions = [
      [0, 0],
      [SLOT_ICON_TILE_SIZE, 0],
      [0, SLOT_ICON_TILE_SIZE],
      [SLOT_ICON_TILE_SIZE, SLOT_ICON_TILE_SIZE],
    ];

    for (let i = 0; i < tileLayout.length; i += 1) {
      const tileIndex = tileLayout[i];
      const sourceX = (tileIndex % columns) * SLOT_ICON_TILE_SIZE;
      const sourceY = Math.floor(tileIndex / columns) * SLOT_ICON_TILE_SIZE;
      const [destinationX, destinationY] = destPositions[i];
      icon.blit(sheet, [destinationX, destinationY], {
        x: sourceX,
        y: sourceY,
        width: SLOT_ICON_TILE_SIZE,
        height: SLOT_ICON_TILE_SIZE,
      });
    }

    slotIconCacheRef.current.set(symbol, icon);
    return icon;
  }, []);

  const blitTile = useCallback((sheet: Surface, tileIndex: number, destX: number, destY: number): boolean => {
    const columns = Math.floor(sheet.get_width() / TILE_SIZE);
    const rows = Math.floor(sheet.get_height() / TILE_SIZE);
    const totalTiles = columns * rows;
    if (tileIndex < 0 || tileIndex >= totalTiles) {
      return false;
    }

    const sourceX = (tileIndex % columns) * TILE_SIZE;
    const sourceY = Math.floor(tileIndex / columns) * TILE_SIZE;
    const ui = tileUiRef.current;
    if (!ui) {
      return false;
    }

    ui.screen.blit(sheet, [destX, destY], {
      x: sourceX,
      y: sourceY,
      width: TILE_SIZE,
      height: TILE_SIZE,
    });
    return true;
  }, []);

  const drawSlotsBackground = useCallback((): boolean => {
    const tilemap = slotTilemapRef.current;
    const uiSheet = slotUiSheetRef.current;
    const symbolSheet = slotSymbolSheetRef.current;
    if (!tilemap || !uiSheet || !symbolSheet) {
      return false;
    }
    if (tilemap.length !== SLOTS_TILEMAP_LENGTH) {
      return false;
    }

    for (let tileOffset = 0; tileOffset < tilemap.length; tileOffset += 1) {
      const tileId = tilemap[tileOffset];
      const destTileX = tileOffset % SLOTS_TILEMAP_WIDTH;
      const destTileY = Math.floor(tileOffset / SLOTS_TILEMAP_WIDTH);
      const destX = destTileX * TILE_SIZE;
      const destY = destTileY * TILE_SIZE;

      if (tileId < SLOTS_VTILES2_OVERLAY_START_TILE) {
        if (!blitTile(uiSheet, tileId, destX, destY)) {
          return false;
        }
      } else if (!blitTile(symbolSheet, tileId - SLOTS_VTILES2_OVERLAY_START_TILE, destX, destY)) {
        return false;
      }
    }

    return true;
  }, [blitTile]);

  const stopSpinAnimation = useCallback(() => {
    if (spinFrameRequestRef.current !== null) {
      cancelAnimationFrame(spinFrameRequestRef.current);
      spinFrameRequestRef.current = null;
    }
    spinStartMsRef.current = 0;
    spinTargetOffsetsRef.current = null;
  }, []);

  const animateReels = useCallback((timestamp: number) => {
    const targetOffsets = spinTargetOffsetsRef.current;
    if (!targetOffsets) {
      return;
    }

    if (spinStartMsRef.current === 0) {
      spinStartMsRef.current = timestamp;
    }
    const elapsedMs = timestamp - spinStartMsRef.current;

    const nextOffsets: [number, number, number] = [...spinCurrentOffsetsRef.current] as [number, number, number];
    let allStopped = true;
    for (let reelIndex = 0; reelIndex < SLOT_REEL_STOP_MS.length; reelIndex += 1) {
      const stopAtMs = SLOT_REEL_STOP_MS[reelIndex];
      if (elapsedMs >= stopAtMs) {
        nextOffsets[reelIndex] = targetOffsets[reelIndex];
        continue;
      }
      allStopped = false;
      const stepCount = Math.max(1, Math.floor(elapsedMs / SLOT_REEL_SPIN_STEP_MS) + 1 + reelIndex * 2);
      nextOffsets[reelIndex] = wrapReelIndex(spinBaseOffsetsRef.current[reelIndex] + stepCount);
    }

    spinCurrentOffsetsRef.current = nextOffsets;
    setAnimatedWindows(buildReelWindowsFromOffsets(nextOffsets));

    if (!allStopped) {
      spinFrameRequestRef.current = requestAnimationFrame(animateReels);
      return;
    }

    stopSpinAnimation();
    setIsSpinning(false);
    setAnimatedWindows(null);
    setSlotMessage(spinFinalMessageRef.current ?? "DARN");
    spinFinalMessageRef.current = null;
  }, [stopSpinAnimation]);

  const spinSlots = useCallback(() => {
    if (isSpinning) {
      return;
    }
    setCoins((currentCoins) => {
      if (currentCoins < slotBet) {
        setSlotMessage("NEED MORE COINS");
        return currentCoins;
      }

      const machine = new SlotMachine(new HardwareRNG(rngStateRef.current));
      const result = machine.spin({ bet: slotBet, mode: SlotMachineMode.NORMAL });
      const resultWindows = result.windows as ReelWindows;
      const targetOffsets: [number, number, number] = [
        findReelOffsetForWindow(0, resultWindows[0]),
        findReelOffsetForWindow(1, resultWindows[1]),
        findReelOffsetForWindow(2, resultWindows[2]),
      ];
      const startOffsets: [number, number, number] = [
        wrapReelIndex(spinCurrentOffsetsRef.current[0] + 3),
        wrapReelIndex(spinCurrentOffsetsRef.current[1] + 5),
        wrapReelIndex(spinCurrentOffsetsRef.current[2] + 7),
      ];

      stopSpinAnimation();
      spinBaseOffsetsRef.current = startOffsets;
      spinCurrentOffsetsRef.current = startOffsets;
      spinTargetOffsetsRef.current = targetOffsets;
      spinFinalMessageRef.current = result.payout > 0 ? `WIN ${result.payout}` : "DARN";
      setAnimatedWindows(buildReelWindowsFromOffsets(startOffsets));
      setIsSpinning(true);
      setSlotResult(result);
      setSlotMessage("SPINNING");
      spinFrameRequestRef.current = requestAnimationFrame(animateReels);
      return clampCoins(currentCoins - slotBet + result.payout);
    });
  }, [animateReels, isSpinning, slotBet, stopSpinAnimation]);

  const handleGameCornerKey = useCallback((event: globalThis.KeyboardEvent): void => {
    if (event.type !== "keydown" || event.repeat || event.key === "Tab") {
      return;
    }

    const keyRef = event.code ?? event.key;
    const direction = mapKeyToDirection(keyRef ?? "");
    const button = mapKeyToButton(keyRef ?? "");
    if (!direction && !button) {
      return;
    }
    if (isSpinning) {
      event.preventDefault();
      return;
    }

    if (button === "a") {
      spinSlots();
      event.preventDefault();
      return;
    }

    if (direction === "left") {
      setSlotBet((current) => Math.max(1, current - 1) as 1 | 2 | 3);
      event.preventDefault();
      return;
    }

    if (direction === "right") {
      setSlotBet((current) => Math.min(3, current + 1) as 1 | 2 | 3);
      event.preventDefault();
      return;
    }

  }, [isSpinning, spinSlots]);

  const renderTileViewport = useCallback(() => {
    const ui = tileUiRef.current;
    const text = tileTextRef.current;
    if (!ui || !text) {
      return;
    }

    const writeLine = (
      tileX: number,
      tileY: number,
      message: string,
      color: [number, number, number] = TILE_COLORS.ink,
    ) => {
      const maxChars = Math.max(0, 20 - tileX);
      const clipped = message.toUpperCase().slice(0, maxChars);
      text.renderText(clipped, tileX * TILE_SIZE, tileY * TILE_SIZE, ui.screen, { color });
    };

    ui.clearScreen(TILE_COLORS.background);
    const drewSlotsBackground = drawSlotsBackground();

    if (!drewSlotsBackground) {
      ui.drawWindow(ui.screen, 0, 0, 20, 18, { frameId: 1, fill: TILE_COLORS.fill });
      writeLine(1, 2, "SLOT MACHINE", TILE_COLORS.accent);
      writeLine(1, 4, "LOADING SLOTS GFX");
      ui.update();
      return;
    }

    const visibleWindows =
      animatedWindows ??
      (slotResult
        ? visibleWindowsFromEngineWindows(slotResult.windows as ReelWindows)
        : buildReelWindowsFromOffsets([REEL_LENGTH - 1, REEL_LENGTH - 1, REEL_LENGTH - 1]));
    const visiblePayout = isSpinning ? 0 : (slotResult?.payout ?? 0);
    writeLine(SLOT_COIN_COUNT_TILE_X, SLOT_COUNTER_TILE_Y, String(coins).padStart(4, "0"));
    writeLine(SLOT_PAYOUT_TILE_X, SLOT_COUNTER_TILE_Y, String(visiblePayout).padStart(4, "0"));

    if (visibleWindows && slotGraphicsReady) {
      for (let reelIndex = 0; reelIndex < SLOT_REEL_X_TILES.length; reelIndex += 1) {
        for (let rowIndex = 0; rowIndex < SLOT_REEL_Y_TILES.length; rowIndex += 1) {
          const symbol = visibleWindows[reelIndex][rowIndex];
          const icon = getSlotIconSurface(symbol);
          if (!icon) {
            writeLine(SLOT_REEL_X_TILES[reelIndex], SLOT_REEL_Y_TILES[rowIndex], SLOT_SHORT_CODES[symbol]);
            continue;
          }
          ui.screen.blit(icon, [SLOT_REEL_X_TILES[reelIndex] * TILE_SIZE, SLOT_REEL_Y_TILES[rowIndex] * TILE_SIZE]);
        }
      }
    }

    writeLine(1, 15, slotMessage ?? "A SPIN");
    ui.update();
  }, [
    animatedWindows,
    coins,
    drawSlotsBackground,
    getSlotIconSurface,
    isSpinning,
    slotBet,
    slotGraphicsReady,
    slotMessage,
    slotResult,
  ]);

  useEffect(() => {
    const canvas = tileCanvasRef.current;
    if (!canvas) {
      return;
    }

    const computeDisplayScale = (): number => {
      if (typeof window === "undefined") {
        return 1;
      }

      const frameBox = canvasFrameRef.current;
      const shellBox = canvasShellRef.current;
      const layoutBox =
        frameBox ??
        shellBox ??
        (canvas.parentElement as HTMLElement | null) ??
        canvas;
      const layoutRect = layoutBox?.getBoundingClientRect();
      const shellWidth = layoutRect?.width ?? layoutBox?.clientWidth ?? 0;
      const shellHeight = layoutRect?.height ?? layoutBox?.clientHeight ?? 0;
      const viewportWidth = window.innerWidth - TILE_DISPLAY_MARGIN_PX;
      const availableWidth = Math.max(1, shellWidth > 0 ? shellWidth : viewportWidth);
      const availableHeight = Math.max(
        1,
        shellHeight > 0 ? Math.min(shellHeight, window.innerHeight) : window.innerHeight,
      );
      const widthScale = availableWidth / TILE_CANVAS_WIDTH;
      const heightScale = availableHeight / TILE_CANVAS_HEIGHT;
      const fittedScale = Math.min(widthScale, heightScale);
      return Math.max(0.25, Number.isFinite(fittedScale) ? fittedScale : 1);
    };

    const updateCanvasDisplaySize = () => {
      const displayScale = computeDisplayScale();
      const displayWidth = Math.max(1, Math.floor(TILE_CANVAS_WIDTH * displayScale));
      const displayHeight = Math.max(1, Math.floor(TILE_CANVAS_HEIGHT * displayScale));
      canvas.style.width = `${displayWidth}px`;
      canvas.style.height = `${displayHeight}px`;
      canvas.style.maxWidth = `${displayWidth}px`;
      canvas.style.maxHeight = `${displayHeight}px`;
    };

    canvas.style.display = "block";
    canvas.style.imageRendering = "pixelated";
    updateCanvasDisplaySize();

    const resizeObserver =
      typeof ResizeObserver !== "undefined" && canvasShellRef.current
        ? new ResizeObserver(() => {
            updateCanvasDisplaySize();
          })
        : null;
    if (resizeObserver && canvasShellRef.current) {
      resizeObserver.observe(canvasShellRef.current);
    }
    if (resizeObserver && canvasFrameRef.current) {
      resizeObserver.observe(canvasFrameRef.current);
    }
    if (typeof window !== "undefined") {
      window.addEventListener("resize", updateCanvasDisplaySize);
      window.addEventListener("orientationchange", updateCanvasDisplaySize);
      window.visualViewport?.addEventListener("resize", updateCanvasDisplaySize);
    }

    return () => {
      resizeObserver?.disconnect();
      if (typeof window !== "undefined") {
        window.removeEventListener("resize", updateCanvasDisplaySize);
        window.removeEventListener("orientationchange", updateCanvasDisplaySize);
        window.visualViewport?.removeEventListener("resize", updateCanvasDisplaySize);
      }
    };
  }, []);

  useEffect(() => {
    if (process.env.NODE_ENV === "test") {
      return;
    }

    let cancelled = false;
    const canvas = tileCanvasRef.current;
    if (!canvas) {
      return;
    }

    const initRenderer = async () => {
      try {
        const ui = new DomCanvasUI(undefined, undefined, TILE_CANVAS_SCALE, undefined, canvas);
        assertAsmUiInvariants(ui, "GameCornerClient.tileViewportUi");
        assertAsmScreenDimensions(
          Math.floor(canvas.width / TILE_CANVAS_SCALE),
          Math.floor(canvas.height / TILE_CANVAS_SCALE),
          "GameCornerClient.tileViewportCanvas",
        );

        const renderer = new TextRenderer();
        await renderer.load();
        ui.font = {
          ...(ui.font ?? {}),
          font_tiles: renderer.font_tiles,
          fontTiles: renderer.fontTiles,
          render_text: renderer.render_text.bind(renderer) as any,
          renderText: renderer.renderText.bind(renderer) as any,
          paletteVariants: renderer.paletteVariants.bind(renderer),
        };
        await ui.preloadWindowFrames([1]);

        if (cancelled) {
          return;
        }
        tileUiRef.current = ui;
        tileTextRef.current = renderer;
        setTileRendererError(null);
        setTileRendererReady(true);
      } catch (error) {
        if (cancelled) {
          return;
        }
        setTileRendererReady(false);
        setTileRendererError(error instanceof Error ? error.message : String(error));
      }
    };

    void initRenderer();

    return () => {
      cancelled = true;
      tileUiRef.current = null;
      tileTextRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (process.env.NODE_ENV === "test") {
      return;
    }

    let cancelled = false;

    const loadSlotGraphics = async () => {
      try {
        const [uiSheet, symbolSheet, tilemap] = await Promise.all([
          gameEngine.image.load(resolveSlotsUiTilesPath()),
          gameEngine.image.load(resolveSlotsSymbolTilesPath()),
          loadAssetBytes(resolveSlotsTilemapPath()),
        ]);

        if (tilemap.length !== SLOTS_TILEMAP_LENGTH) {
          throw new Error(`Unexpected slots tilemap size: ${tilemap.length}`);
        }

        if (cancelled) {
          return;
        }

        slotUiSheetRef.current = uiSheet;
        slotSymbolSheetRef.current = symbolSheet;
        slotTilemapRef.current = tilemap;
        slotIconCacheRef.current.clear();
        setSlotGraphicsError(null);
        setSlotGraphicsReady(true);
      } catch (error) {
        if (cancelled) {
          return;
        }
        slotUiSheetRef.current = null;
        slotSymbolSheetRef.current = null;
        slotTilemapRef.current = null;
        slotIconCacheRef.current.clear();
        setSlotGraphicsReady(false);
        setSlotGraphicsError(error instanceof Error ? error.message : String(error));
      }
    };

    void loadSlotGraphics();

    return () => {
      cancelled = true;
      slotUiSheetRef.current = null;
      slotSymbolSheetRef.current = null;
      slotTilemapRef.current = null;
      slotIconCacheRef.current.clear();
    };
  }, []);

  useEffect(() => () => {
    stopSpinAnimation();
  }, [stopSpinAnimation]);

  useEffect(() => {
    if (!tileRendererReady) {
      return;
    }
    renderTileViewport();
  }, [renderTileViewport, tileRendererReady]);

  useEffect(() => {
    const listener = (event: globalThis.KeyboardEvent): void => {
      const target = event.target as Element | null;
      if (target) {
        const tagName = target.tagName?.toLowerCase();
        if (tagName === "input" || tagName === "textarea" || (target as HTMLElement).isContentEditable) {
          return;
        }
      }
      handleGameCornerKey(event);
    };

    window.addEventListener("keydown", listener);
    return () => {
      window.removeEventListener("keydown", listener);
    };
  }, [handleGameCornerKey]);

  return (
    <div className="grid w-full gap-2 justify-items-center" data-testid="slot-machine-only-layout">
      <div className="card card-bordered border border-base-300 bg-base-200 shadow-lg w-full max-w-4xl">
        <div className="card-body p-3">
          <div
            ref={canvasShellRef}
            className="flex min-h-0 w-full justify-center overflow-hidden"
            data-testid="game-corner-canvas-shell"
          >
            <div
              ref={canvasFrameRef}
              className="flex min-h-0 w-full items-center justify-center overflow-hidden"
              data-testid="game-corner-canvas-frame"
            >
              <canvas
                data-testid="game-corner-tile-canvas"
                ref={tileCanvasRef}
                width={TILE_CANVAS_WIDTH}
                height={TILE_CANVAS_HEIGHT}
                className="block rounded-box border border-base-300 bg-base-300 shadow-md [image-rendering:pixelated]"
              />
            </div>
          </div>
        </div>
      </div>
      {tileRendererError ? <div className="alert alert-warning" role="alert">Tile renderer unavailable: {tileRendererError}</div> : null}
      {slotGraphicsError ? <div className="alert alert-warning" role="alert">Slot graphics unavailable: {slotGraphicsError}</div> : null}

      <div data-testid="game-corner-coins" className="hidden">
        Coins: {coins}
      </div>
      <div
        data-testid="game-corner-state"
        className="hidden"
        data-slot-bet={slotBet}
        data-slot-payout={slotResult?.payout ?? ""}
        data-slot-message={slotMessage ?? ""}
        data-slot-graphics-ready={String(slotGraphicsReady)}
        data-slot-is-spinning={String(isSpinning)}
      />
    </div>
  );
};

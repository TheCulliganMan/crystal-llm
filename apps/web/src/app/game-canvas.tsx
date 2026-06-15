"use client";

import "@/shims/setup-browser";

import React, { useEffect, useRef } from "react";
import { Game, type GameLoadProgress } from "./game";
import type { FrameMetrics } from "./game-benchmark";
import { buildUi, RendererMode } from "./ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { mapKeyToButton, mapKeyToDirection } from "@pokecrystal/core/input/controls";
import { defaultKeyBindings, GameButton } from "@pokecrystal/core/input/config";
import type { GameEngineEvent } from "@pokecrystal/core/ui/game-engine";
import type { TextSnapshot, TextUI } from "@pokecrystal/core/ui/text-ui";
import type { SnapshotLine } from "@pokecrystal/core/ui/text-snapshot-render";
import { buildTextSnapshotLayout, MAX_TEXT_RENDER_CHARS } from "@pokecrystal/core/ui/text-snapshot-render";
import type { AudioPlaybackSnapshot } from "@pokecrystal/core/engine/systems/audio";
import { callMcpTool, type McpToolResult } from "./mcp-client";
import { PRIMARY_MCP_SESSION_ID } from "@/app/mcp/session-id";
import logger from "@pokecrystal/core/core/logger";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { MANUAL_SAVE_SLOT } from "@pokecrystal/core/core/save-slots";
import { parseVisualDebugScript, runVisualDebugScript } from "./visual-debug";

const BASE_WIDTH = 160;
const BASE_HEIGHT = 144;
const TILE_CANVAS_WIDTH = BASE_WIDTH;
const TILE_CANVAS_HEIGHT = BASE_HEIGHT;
const TEXT_CANVAS_WIDTH = 1280;
const TEXT_CANVAS_HEIGHT = 640;
const DISPLAY_MARGIN_PX = 32;
const DUAL_CANVAS_GAP_PX = 12;
const SM_BREAKPOINT_PX = 640;
const MCP_SESSION_STORAGE_KEY = "pokecrystal.play.session";
const MCP_POLL_MS = 100;
const MCP_ADVANCE_FRAMES = 0;
const MCP_BASE_URL = process.env.NEXT_PUBLIC_MCP_ENTRYPOINT ?? "/api/mcp";
const GAMEPAD_AXIS_THRESHOLD = 0.45;
const TEXT_SNAPSHOT_REFRESH_MS = 100;
// ASM mapping: pokecrystal_disassembly/home/joypad.asm::JoyTextDelay
// Initial held-input repeat delay is 15 frames, then repeats every 5 frames.
const CONTROL_REPEAT_INITIAL_DELAY_FRAMES = 15;
const CONTROL_REPEAT_INTERVAL_FRAMES = 5;
const MAX_CONTROL_REPEAT_ACCUMULATED_MS = GB_FRAME_DURATION_MS * 5;
const PREVENT_DEFAULT_KEY_CODES = new Set([
  "ArrowUp",
  "ArrowDown",
  "ArrowLeft",
  "ArrowRight",
  "Space",
  "Enter",
  "Backspace",
  "ShiftLeft",
  "ShiftRight",
]);

type FullscreenCapableElement = Element & {
  webkitRequestFullscreen?: () => Promise<void> | void;
};

type FullscreenCapableDocument = Document & {
  webkitFullscreenElement?: Element | null;
};

const withSessionId = (baseUrl: string, sessionId: string, origin?: string): string => {
  if (!sessionId) {
    return baseUrl;
  }
  try {
    const fallbackOrigin = origin ?? "http://localhost";
    const url = new URL(baseUrl, fallbackOrigin);
    url.searchParams.set("session_id", sessionId);
    return url.toString();
  } catch {
    const delimiter = baseUrl.includes("?") ? "&" : "?";
    return `${baseUrl}${delimiter}session_id=${encodeURIComponent(sessionId)}`;
  }
};

type InputState = {
  pressedButtons: string[];
  pressedKeys: Array<string | number>;
};

type GameCanvasProps = {
  onInputStateChange?: (state: InputState) => void;
  onPostEventReady?: (postEvent: ((event: GameEngineEvent) => void) | null) => void;
  onGameReady?: (game: Game | null) => void;
  onLoadProgress?: (progress: GameLoadProgress) => void;
  loadSlot?: string;
  muted?: boolean;
  musicMuted?: boolean;
  rendererMode?: RendererMode;
  runtimeMode?: "local" | "server";
  canvasHeightReservePx?: number;
  autoStart?: boolean;
  preloadMode?: "auto" | "none";
  playIntro?: boolean;
  newGame?: boolean;
  canvasClassName?: string;
  canvasStyle?: React.CSSProperties;
  sessionId?: string;
  readOnly?: boolean;
  remoteVisualMode?: "frame" | "text";
  remoteRefreshMs?: number;
  remoteFrameScale?: number;
  remoteAdvanceFrames?: number;
  remoteInstantMode?: boolean;
  remoteFrameRefreshKey?: number;
  mcpActionMirrorSessionId?: string;
  mcpActionMirrorPollMs?: number;
};

type MirroredMcpInput =
  | { kind: "move"; value: "up" | "down" | "left" | "right" }
  | { kind: "button"; value: "a" | "b" | "start" | "select" };
type MirroredMcpButton = Extract<MirroredMcpInput, { kind: "button" }>["value"];

type PostKeyboardEventOptions = {
  mirrorToMcp?: boolean;
};

type SyntheticRepeatPolicyArgs = {
  key: string | number | null;
  mappedControl: boolean;
  direction: "up" | "down" | "left" | "right" | null;
  gameState: string | null;
  inputCaptureActive: boolean;
  unownInputActive: boolean;
};

const normalizeRepeatDirection = (
  value: string | null
): "up" | "down" | "left" | "right" | null => {
  if (value === "up" || value === "down" || value === "left" || value === "right") {
    return value;
  }
  return null;
};

export const shouldApplySyntheticRepeatPolicy = ({
  key,
  mappedControl,
  direction,
  gameState,
  inputCaptureActive,
  unownInputActive,
}: SyntheticRepeatPolicyArgs): boolean => {
  if (key == null || !mappedControl) {
    return false;
  }

  // Avoid cascading confirm/cancel presses across nested captured menus
  // (e.g., Pokecenter PC submenus) while still allowing deliberate repeats
  // in non-captured contexts.
  if (!direction) {
    if (gameState === "overworld" && inputCaptureActive) {
      return false;
    }
    return true;
  }

  if (gameState !== "overworld") {
    return true;
  }
  if (inputCaptureActive) {
    return true;
  }
  if (unownInputActive) {
    return true;
  }
  // ASM parity: held overworld movement is sourced from latched joypad bits,
  // not synthetic repeated direction-press events.
  return false;
};

type GameWindowHooks = Window & typeof globalThis & {
  advanceTime?: (ms: number) => Promise<void> | void;
  render_game_to_text?: () => string;
  jump_game_scene?: (scene: string) => Promise<void>;
  jump_game_spawn?: (spawn: string | number) => Promise<void>;
  get_game_debug_status?: () => string;
  save_game_to_slot?: (slot: string, options?: { withHistory?: boolean }) => Promise<boolean>;
  delete_save_slot?: (slot: string) => Promise<boolean>;
  has_save_slot?: (slot: string) => Promise<boolean>;
  trigger_game_autosave?: (
    reason?: "battle_complete" | "player_steps",
    count?: number
  ) => Promise<void>;
  get_game_benchmark?: (slowFrameThresholdMs?: number) => {
    enabled: boolean;
    thresholdMs: number;
    state?: string;
    latestFrame?: FrameMetrics | null;
    recentFrames?: FrameMetrics[];
    slowFrames?: FrameMetrics[];
    reason?: string;
  };
  clear_game_benchmark?: () => void;
  get_text_render_benchmark?: () => TextRenderBenchmarkSnapshot;
  clear_text_render_benchmark?: () => void;
  run_game_script?: (script: string | unknown[]) => Promise<string>;
  post_game_event?: (event: {
    type: string | number;
    key?: string | number | null;
    code?: string | number | null;
    button?: string | null;
    direction?: string | null;
    is_press?: boolean | null;
    text?: string | null;
  }) => void;
};

type LocalTextSnapshotPayload = {
  coordinate_system: "origin_top_left_x_right_y_down_tiles";
  mode: string;
  frame: number;
  map: {
    id: string;
    group: number;
    number: number;
  };
  player: {
    x: number;
    y: number;
    facing: string;
  };
};

type TextRenderBenchmarkPhase = "snapshotRead" | "layoutBuild" | "paint";

type TextRenderBenchmarkPhaseMetrics = {
  count: number;
  totalMs: number;
  maxMs: number;
  lastMs: number;
};

type TextRenderBenchmarkFrame = {
  snapshotChanged: boolean;
  layoutBuilt: boolean;
  painted: boolean;
  width: number;
  height: number;
  lineCount: number;
};

type TextRenderBenchmarkSnapshot = {
  enabled: boolean;
  refreshMs: number;
  iterations: number;
  phases: Record<TextRenderBenchmarkPhase, TextRenderBenchmarkPhaseMetrics>;
  lastFrame: TextRenderBenchmarkFrame | null;
};

type MutableTextRenderBenchmark = TextRenderBenchmarkSnapshot;

const createTextRenderBenchmarkPhaseMetrics = (): TextRenderBenchmarkPhaseMetrics => ({
  count: 0,
  totalMs: 0,
  maxMs: 0,
  lastMs: 0,
});

const createTextRenderBenchmark = (): MutableTextRenderBenchmark => ({
  enabled: false,
  refreshMs: TEXT_SNAPSHOT_REFRESH_MS,
  iterations: 0,
  phases: {
    snapshotRead: createTextRenderBenchmarkPhaseMetrics(),
    layoutBuild: createTextRenderBenchmarkPhaseMetrics(),
    paint: createTextRenderBenchmarkPhaseMetrics(),
  },
  lastFrame: null,
});

const recordTextRenderBenchmarkPhase = (
  benchmark: MutableTextRenderBenchmark,
  phase: TextRenderBenchmarkPhase,
  durationMs: number,
): void => {
  const entry = benchmark.phases[phase];
  entry.count += 1;
  entry.totalMs += durationMs;
  entry.lastMs = durationMs;
  entry.maxMs = Math.max(entry.maxMs, durationMs);
};

const cloneTextRenderBenchmark = (
  benchmark: MutableTextRenderBenchmark,
): TextRenderBenchmarkSnapshot => ({
  enabled: benchmark.enabled,
  refreshMs: benchmark.refreshMs,
  iterations: benchmark.iterations,
  phases: {
    snapshotRead: { ...benchmark.phases.snapshotRead },
    layoutBuild: { ...benchmark.phases.layoutBuild },
    paint: { ...benchmark.phases.paint },
  },
  lastFrame: benchmark.lastFrame ? { ...benchmark.lastFrame } : null,
});

const resetTextRenderBenchmark = (
  benchmark: MutableTextRenderBenchmark,
): void => {
  const next = createTextRenderBenchmark();
  benchmark.enabled = next.enabled;
  benchmark.refreshMs = next.refreshMs;
  benchmark.iterations = next.iterations;
  benchmark.phases.snapshotRead = next.phases.snapshotRead;
  benchmark.phases.layoutBuild = next.phases.layoutBuild;
  benchmark.phases.paint = next.phases.paint;
  benchmark.lastFrame = next.lastFrame;
};

const getNowMs = (): number =>
  typeof performance !== "undefined" && typeof performance.now === "function"
    ? performance.now()
    : Date.now();

const buildLocalTextSnapshotPayload = (game: Game): LocalTextSnapshotPayload => {
  const state = game.getGameState();
  const overworldFacing =
    (game.getOverworld() as { player_direction?: string } | null)?.player_direction ?? "unknown";
  return {
    coordinate_system: "origin_top_left_x_right_y_down_tiles",
    mode: game.getState(),
    frame: state.frame_counter,
    map: {
      id: `${state.wram.wMapGroup}:${state.wram.wMapNumber}`,
      group: state.wram.wMapGroup,
      number: state.wram.wMapNumber,
    },
    player: {
      x: state.wram.wXCoord,
      y: state.wram.wYCoord,
      facing: String(overworldFacing),
    },
  };
};

type GamepadControl = "up" | "down" | "left" | "right" | "a" | "b" | "start" | "select";

const GAMEPAD_CONTROLS: GamepadControl[] = [
  "up",
  "down",
  "left",
  "right",
  "a",
  "b",
  "start",
  "select",
];

const primaryKeyForControl = (control: GamepadControl): string => {
  return (
    {
      up: "ArrowUp",
      down: "ArrowDown",
      left: "ArrowLeft",
      right: "ArrowRight",
      a: defaultKeyBindings[GameButton.A][0] ?? "KeyZ",
      b: defaultKeyBindings[GameButton.B][0] ?? "KeyX",
      start: defaultKeyBindings[GameButton.Start][0] ?? "Enter",
      select: defaultKeyBindings[GameButton.Select][0] ?? "Backspace",
    }[control]
  );
};

const buildGamepadEvent = (control: GamepadControl, pressed: boolean): GameEngineEvent => {
  const key = primaryKeyForControl(control);
  const type = pressed ? gameEngine.KEYDOWN : gameEngine.KEYUP;
  const payload: Omit<GameEngineEvent, "type"> = {
    key,
    code: key,
    is_press: pressed,
  };
  if (control === "up" || control === "down" || control === "left" || control === "right") {
    payload.direction = control;
  } else {
    payload.button = control;
  }
  return new gameEngine.event.Event(type, payload);
};

const isGamepadButtonPressed = (button: GamepadButton | null | undefined): boolean => {
  if (!button) {
    return false;
  }
  return button.pressed || button.value > 0.5;
};

type GamepadState = Record<GamepadControl, boolean>;

const GAMEPAD_BUTTON_MAP_ARRAY: ReadonlyArray<[number, GamepadControl]> = [
  [0, "a"],
  [1, "b"],
  [8, "select"],
  [9, "start"],
  [12, "up"],
  [13, "down"],
  [14, "left"],
  [15, "right"],
];

const createEmptyGamepadState = (): GamepadState => ({
  up: false,
  down: false,
  left: false,
  right: false,
  a: false,
  b: false,
  start: false,
  select: false,
});

const readGamepadState = (gamepad: Gamepad | null, state: GamepadState): void => {
  state.up = false;
  state.down = false;
  state.left = false;
  state.right = false;
  state.a = false;
  state.b = false;
  state.start = false;
  state.select = false;
  if (!gamepad) {
    return;
  }
  for (let i = 0; i < GAMEPAD_BUTTON_MAP_ARRAY.length; i += 1) {
    const [buttonIndex, control] = GAMEPAD_BUTTON_MAP_ARRAY[i];
    const button = gamepad.buttons?.[buttonIndex];
    if (isGamepadButtonPressed(button)) {
      state[control] = true;
    }
  }
  const axisX = gamepad.axes?.[0] ?? 0;
  const axisY = gamepad.axes?.[1] ?? 0;
  if (axisX <= -GAMEPAD_AXIS_THRESHOLD) {
    state.left = true;
  }
  if (axisX >= GAMEPAD_AXIS_THRESHOLD) {
    state.right = true;
  }
  if (axisY <= -GAMEPAD_AXIS_THRESHOLD) {
    state.up = true;
  }
  if (axisY >= GAMEPAD_AXIS_THRESHOLD) {
    state.down = true;
  }
};

const pickActiveGamepad = (pads: ArrayLike<Gamepad | null> | null | undefined): Gamepad | null => {
  if (!pads) {
    return null;
  }
  for (let i = 0; i < pads.length; i += 1) {
    const pad = pads[i];
    if (pad && pad.connected) {
      return pad;
    }
  }
  return null;
};

export const GameCanvas = React.memo(({
  onInputStateChange,
  onPostEventReady,
  onGameReady,
  onLoadProgress,
  loadSlot,
  muted,
  musicMuted = false,
  rendererMode = "tile",
  runtimeMode = "server",
  canvasHeightReservePx,
  autoStart = true,
  preloadMode = "auto",
  playIntro = false,
  newGame = false,
  canvasClassName,
  canvasStyle,
  sessionId,
  readOnly = false,
  remoteVisualMode = "text",
  remoteRefreshMs = MCP_POLL_MS,
  remoteFrameScale = 2,
  remoteAdvanceFrames = MCP_ADVANCE_FRAMES,
  remoteInstantMode,
  remoteFrameRefreshKey = 0,
  mcpActionMirrorSessionId,
  mcpActionMirrorPollMs = 150,
}: GameCanvasProps) => {
  const canvasShellRef = useRef<HTMLDivElement>(null);
  const tileCanvasRef = useRef<HTMLCanvasElement>(null);
  const textCanvasRef = useRef<HTMLCanvasElement>(null);
  const gameRef = useRef<Game | null>(null);
  const remoteAudioMirrorGameRef = useRef<Game | null>(null);
  const textUiRef = useRef<TextUI | null>(null);
  const textBitmapFontRef = useRef<CompactBitmapFont | null>(null);
  const textRenderBenchmarkRef = useRef<MutableTextRenderBenchmark>(createTextRenderBenchmark());
  const sessionIdRef = useRef<string | null>(null);
  const remoteQueueRef = useRef<Promise<unknown>>(Promise.resolve());
  const remoteActiveRef = useRef(false);
  const remoteFrameRefreshRef = useRef<(() => void) | null>(null);
  const remoteAudioMusicTokenRef = useRef<string | null>(null);
  const lastRemoteFrameRefreshKeyRef = useRef(remoteFrameRefreshKey);
  const remoteDirectionTimersRef = useRef<Map<string, number>>(new Map());
  const initialMutedRef = useRef(muted ?? false);
  const showTileCanvas = rendererMode !== "text";
  const showTextCanvas = rendererMode !== "tile";
  const showTileCanvasRef = useRef(showTileCanvas);
  const showTextCanvasRef = useRef(showTextCanvas);
  const isServerMode = runtimeMode === "server";
  const isRemoteFrameMode = isServerMode && remoteVisualMode === "frame";
  const clampCanvasReserve = (value: number | undefined): number => {
    if (typeof value !== "number" || !Number.isFinite(value)) {
      return 0;
    }
    return Math.max(0, Math.round(value));
  };

  useEffect(() => {
    showTileCanvasRef.current = showTileCanvas;
    showTextCanvasRef.current = showTextCanvas;
  }, [showTileCanvas, showTextCanvas]);

  useEffect(() => {
    if (!showTextCanvas) {
      textBitmapFontRef.current = null;
      return;
    }
    let active = true;
    void getCompactBitmapTextFont()
      .then((font) => {
        if (!active) {
          return;
        }
        textBitmapFontRef.current = font;
      })
      .catch((error: unknown) => {
        logger.error("[game-canvas] failed to load compact bitmap text font", error);
      });
    return () => {
      active = false;
    };
  }, [showTextCanvas]);

  useEffect(() => {
    const tileCanvas = tileCanvasRef.current;
    const textCanvas = textCanvasRef.current;
    const canvasShell = canvasShellRef.current;
    const canvasEntries = [
      tileCanvas
        ? {
            canvas: tileCanvas,
            visible: showTileCanvas,
            width: TILE_CANVAS_WIDTH,
            height: TILE_CANVAS_HEIGHT,
          }
        : null,
      textCanvas
        ? {
            canvas: textCanvas,
            visible: showTextCanvas,
            width: TEXT_CANVAS_WIDTH,
            height: TEXT_CANVAS_HEIGHT,
          }
        : null,
    ].filter(Boolean) as Array<{
      canvas: HTMLCanvasElement;
      visible: boolean;
      width: number;
      height: number;
    }>;
    if (!canvasEntries.length) {
      return;
    }

    const computeDisplaySize = (
      entryWidth: number,
      entryHeight: number,
    ): { width: number; height: number } => {
      if (typeof window === "undefined") {
        return {
          width: Math.max(1, entryWidth),
          height: Math.max(1, entryHeight),
        };
      }
      const primaryCanvas = (showTileCanvas ? tileCanvas : textCanvas) ?? tileCanvas ?? textCanvas;
      if (!primaryCanvas) {
        return {
          width: Math.max(1, entryWidth),
          height: Math.max(1, entryHeight),
        };
      }
      const fullscreenDoc = document as FullscreenCapableDocument;
      const fullscreenElement =
        (fullscreenDoc.fullscreenElement ??
          fullscreenDoc.webkitFullscreenElement ??
          null) as FullscreenCapableElement | null;
      const isFullscreen = Boolean(fullscreenElement && fullscreenElement.contains(primaryCanvas));
      const reservedHeight = isFullscreen ? 0 : clampCanvasReserve(canvasHeightReservePx);
      const frameBox = canvasShell?.closest('[data-testid="play-canvas-frame"]') as HTMLElement | null;
      const shellBox = canvasShell?.closest('[data-testid="play-canvas-shell"]') as HTMLElement | null;
      const layoutBox =
        frameBox ??
        shellBox ??
        (canvasShell?.parentElement as HTMLElement | null) ??
        canvasShell;
      const layoutRect = layoutBox?.getBoundingClientRect();
      const layoutStyle =
        layoutBox && typeof window !== "undefined" ? window.getComputedStyle(layoutBox) : null;
      const horizontalInsets = layoutStyle
        ? (Number.parseFloat(layoutStyle.paddingLeft || "0") || 0) +
          (Number.parseFloat(layoutStyle.paddingRight || "0") || 0) +
          (Number.parseFloat(layoutStyle.borderLeftWidth || "0") || 0) +
          (Number.parseFloat(layoutStyle.borderRightWidth || "0") || 0)
        : 0;
      const verticalInsets = layoutStyle
        ? (Number.parseFloat(layoutStyle.paddingTop || "0") || 0) +
          (Number.parseFloat(layoutStyle.paddingBottom || "0") || 0) +
          (Number.parseFloat(layoutStyle.borderTopWidth || "0") || 0) +
          (Number.parseFloat(layoutStyle.borderBottomWidth || "0") || 0)
        : 0;
      const shellWidth = Math.max(0, (layoutRect?.width ?? 0) - horizontalInsets);
      const shellHeight = Math.max(0, (layoutRect?.height ?? 0) - verticalInsets);
      const baseViewportWidth = isFullscreen ? window.innerWidth : window.innerWidth - DISPLAY_MARGIN_PX;
      const stackedCanvases =
        showTileCanvas &&
        showTextCanvas &&
        typeof window !== "undefined" &&
        window.innerWidth < SM_BREAKPOINT_PX;
      const columns = showTileCanvas && showTextCanvas && !stackedCanvases ? 2 : 1;
      const availableWidth = Math.max(
        1,
        (shellWidth > 0 ? shellWidth : baseViewportWidth) -
          (columns > 1 ? DUAL_CANVAS_GAP_PX : 0),
      );
      const viewportHeight = isFullscreen ? window.innerHeight : window.innerHeight - reservedHeight;
      const perCanvasHeight =
        stackedCanvases && shellHeight > 0 ? Math.max(1, (shellHeight - DUAL_CANVAS_GAP_PX) / 2) : shellHeight;
      const availableHeight = Math.max(1, shellHeight > 0 ? Math.min(shellHeight, viewportHeight) : viewportHeight);
      const fittedHeight = stackedCanvases
        ? Math.max(1, Math.min(perCanvasHeight, viewportHeight))
        : availableHeight;
      const textOnlyCanvas = !showTileCanvas && showTextCanvas && entryWidth === TEXT_CANVAS_WIDTH;

      if (textOnlyCanvas) {
        const widthScale = availableWidth / entryWidth;
        const heightScale = fittedHeight / entryHeight;
        const displayScale = Math.max(
          0.25,
          Number.isFinite(Math.min(widthScale, heightScale)) ? Math.min(widthScale, heightScale) : 1
        );
        return {
          width: Math.max(1, Math.floor(entryWidth * displayScale)),
          height: Math.max(1, Math.floor(entryHeight * displayScale)),
        };
      }

      const computeDisplayScale = (widthBudget: number, width: number, height: number): number => {
        const widthScale = widthBudget / width;
        const heightScale = fittedHeight / height;
        const fittedScale = Math.min(widthScale, heightScale);
        const safeScale = Number.isFinite(fittedScale) ? fittedScale : 1;
        return safeScale >= 1
          ? Math.max(1, Math.floor(safeScale))
          : Math.max(0.25, safeScale);
      };

      const defaultWidthBudget = columns > 1 ? availableWidth / columns : availableWidth;
      let widthBudget = defaultWidthBudget;
      if (columns > 1 && entryWidth === TEXT_CANVAS_WIDTH) {
        const tileScale = computeDisplayScale(defaultWidthBudget, TILE_CANVAS_WIDTH, TILE_CANVAS_HEIGHT);
        const tileDisplayWidth = Math.max(1, Math.floor(TILE_CANVAS_WIDTH * tileScale));
        widthBudget = Math.max(defaultWidthBudget, availableWidth - tileDisplayWidth);
      }

      const displayScale = computeDisplayScale(widthBudget, entryWidth, entryHeight);
      return {
        width: Math.max(1, Math.floor(entryWidth * displayScale)),
        height: Math.max(1, Math.floor(entryHeight * displayScale)),
      };
    };
    const updateCanvasDisplaySize = () => {
      for (const entry of canvasEntries) {
        const { width: displayWidth, height: displayHeight } = computeDisplaySize(entry.width, entry.height);
        entry.canvas.style.display = entry.visible ? "block" : "none";
        entry.canvas.style.width = `${displayWidth}px`;
        entry.canvas.style.height = `${displayHeight}px`;
        entry.canvas.style.maxWidth = `${displayWidth}px`;
        entry.canvas.style.maxHeight = `${displayHeight}px`;
        entry.canvas.style.imageRendering = "pixelated";
        entry.canvas.style.justifySelf =
          entry.canvas === textCanvas && showTileCanvas && showTextCanvas ? "start" : "center";
      }
    };
    updateCanvasDisplaySize();
    const resizeObserver =
      typeof ResizeObserver !== "undefined" && canvasShell
        ? new ResizeObserver(() => {
            updateCanvasDisplaySize();
          })
        : null;
    if (resizeObserver && canvasShell) {
      resizeObserver.observe(canvasShell);
    }
    if (typeof window !== "undefined") {
      window.addEventListener("resize", updateCanvasDisplaySize);
      window.visualViewport?.addEventListener("resize", updateCanvasDisplaySize);
    }
    if (typeof document !== "undefined") {
      document.addEventListener("fullscreenchange", updateCanvasDisplaySize);
      document.addEventListener("webkitfullscreenchange", updateCanvasDisplaySize as EventListener);
    }

    return () => {
      resizeObserver?.disconnect();
      if (typeof window !== "undefined") {
        window.removeEventListener("resize", updateCanvasDisplaySize);
        window.visualViewport?.removeEventListener("resize", updateCanvasDisplaySize);
      }
      if (typeof document !== "undefined") {
        document.removeEventListener("fullscreenchange", updateCanvasDisplaySize);
        document.removeEventListener("webkitfullscreenchange", updateCanvasDisplaySize as EventListener);
      }
    };
  }, [canvasHeightReservePx, isServerMode, showTileCanvas, showTextCanvas]);

  useEffect(() => {
    const tileCanvas = tileCanvasRef.current;
    const textCanvas = textCanvasRef.current;
    logger.debug("[game-canvas] mounting", { showTileCanvas, showTextCanvas, isServerMode, autoStart });
    const primaryCanvas = tileCanvas ?? textCanvas;
    if (!primaryCanvas) {
      return;
    }

    const canvasEntries = [
      tileCanvas ? { canvas: tileCanvas, width: TILE_CANVAS_WIDTH, height: TILE_CANVAS_HEIGHT } : null,
      textCanvas ? { canvas: textCanvas, width: TEXT_CANVAS_WIDTH, height: TEXT_CANVAS_HEIGHT } : null,
    ].filter(Boolean) as Array<{ canvas: HTMLCanvasElement; width: number; height: number }>;
    for (const entry of canvasEntries) {
      entry.canvas.width = entry.width;
      entry.canvas.height = entry.height;
      entry.canvas.style.imageRendering = "pixelated";
    }

    let mounted = true;
    let cleanupRenderLoop: (() => void) | null = null;
    let cleanupRemoteAudioMirror: (() => void) | null = null;
    let postEvent: ((event: GameEngineEvent) => void) | null = null;
    const remoteDirectionTimers = remoteDirectionTimersRef.current;

    const ensureSessionId = (): string | null => {
      if (sessionId && sessionId.trim().length > 0) {
        const explicitSessionId = sessionId.trim();
        if (sessionIdRef.current !== explicitSessionId) {
          sessionIdRef.current = explicitSessionId;
        }
        return sessionIdRef.current;
      }
      if (sessionIdRef.current) {
        return sessionIdRef.current;
      }
      if (typeof window === "undefined") {
        return null;
      }
      const next = PRIMARY_MCP_SESSION_ID;
      try {
        window.localStorage.setItem(MCP_SESSION_STORAGE_KEY, next);
      } catch {
        // Ignore storage failures; session will be memory-only.
      }
      sessionIdRef.current = next;
      return next;
    };

    const callRemoteTool = (
      name: string,
      args: Record<string, unknown>
    ): Promise<McpToolResult> => {
      const sessionId = ensureSessionId();
      const origin = typeof window !== "undefined" ? window.location.origin : undefined;
      const url = sessionId ? withSessionId(MCP_BASE_URL, sessionId, origin) : MCP_BASE_URL;
      const task = remoteQueueRef.current.then(() =>
        callMcpTool(name, args, {
          baseUrl: url,
          headers: remoteInstantMode === undefined
            ? undefined
            : { "x-pokecrystal-instant-mode": remoteInstantMode ? "1" : "0" },
        })
      );
      remoteQueueRef.current = task.catch(() => null);
      return task;
    };

    const postRemoteInputEvent = (
      event: GameEngineEvent,
      isPress: boolean,
      direction: string | null,
      button: string | null
    ): void => {
      const sessionId = ensureSessionId();
      if (!sessionId || !remoteActiveRef.current) {
        return;
      }
      const key = String(event.code ?? event.key ?? direction ?? button ?? "");
      if (!key) {
        return;
      }
      void fetch("/api/arena/input", {
        method: "POST",
        headers: {
          "content-type": "application/json",
          ...(PUBLIC_SNAPSHOT_TOKEN ? { "x-mcp-token": PUBLIC_SNAPSHOT_TOKEN } : {}),
        },
        body: JSON.stringify({
          session_id: sessionId,
          key,
          direction,
          button,
          is_press: isPress,
          instant: remoteInstantMode ?? false,
        }),
        cache: "no-store",
      }).catch((error) => {
        if (remoteActiveRef.current) {
          logger.debug("[game-canvas] remote input event failed", error);
        }
      });
    };

    const enqueueRemoteInput = (event: GameEngineEvent): void => {
      const sessionId = ensureSessionId();
      if (!sessionId || !remoteActiveRef.current) {
        return;
      }
      const isPress = event.is_press ?? event.type === gameEngine.KEYDOWN;
      const direction =
        event.direction ?? mapKeyToDirection(event.code ?? event.key ?? null);
      const button =
        event.button ?? mapKeyToButton(event.code ?? event.key ?? null);
      if (remoteInstantMode === false) {
        postRemoteInputEvent(event, isPress, direction, button);
        return;
      }
      if (!isPress) {
        if (direction) {
          const timerId = remoteDirectionTimers.get(direction);
          if (timerId !== undefined) {
            window.clearInterval(timerId);
            remoteDirectionTimers.delete(direction);
          }
        }
        return;
      }
      if (direction) {
        for (const [heldDirection, timerId] of remoteDirectionTimers.entries()) {
          if (heldDirection !== direction) {
            window.clearInterval(timerId);
            remoteDirectionTimers.delete(heldDirection);
          }
        }
        if (remoteDirectionTimers.has(direction)) {
          return;
        }
        const sendMove = () => {
          if (!remoteActiveRef.current) {
            return;
          }
          void callRemoteTool("move", { direction }).catch((error) => {
            if (remoteActiveRef.current) {
              logger.debug("[game-canvas] remote input failed", error);
            }
          });
        };
        sendMove();
        const timerId = window.setInterval(sendMove, MCP_POLL_MS);
        remoteDirectionTimers.set(direction, timerId);
        return;
      }
      if (!button) {
        return;
      }
      void callRemoteTool("press", { button }).catch((error) => {
        if (remoteActiveRef.current) {
          logger.debug("[game-canvas] remote input failed", error);
        }
      });
    };

    if (isServerMode) {
      textUiRef.current = null;
      gameRef.current = null;
      onGameReady?.(null);
      remoteActiveRef.current = true;
      const sessionId = ensureSessionId();
      postEvent = readOnly ? null : enqueueRemoteInput;
      onPostEventReady?.(postEvent);

      if (!readOnly && typeof document !== "undefined") {
        let mirrorStopped = false;
        const mirrorCanvas = document.createElement("canvas");
        mirrorCanvas.width = TILE_CANVAS_WIDTH;
        mirrorCanvas.height = TILE_CANVAS_HEIGHT;
        const { ui: mirrorUi } = buildUi(mirrorCanvas, { rendererMode: "tile", scale: 1 });
        void Game.create(mirrorUi, {
          loadSlot: MANUAL_SAVE_SLOT,
          muted: muted ?? false,
          playIntro: false,
          newGame: false,
          preloadMode,
        }).then((game) => {
          if (mirrorStopped) {
            game.destroy();
            return;
          }
          game.setAudioMuted(muted ?? false);
          game.setMusicMuted(musicMuted);
          game.start();
          remoteAudioMirrorGameRef.current = game;
        }).catch((error) => {
          logger.debug("[game-canvas] remote audio mirror failed", error);
        });
        cleanupRemoteAudioMirror = () => {
          mirrorStopped = true;
          remoteAudioMirrorGameRef.current?.destroy();
          remoteAudioMirrorGameRef.current = null;
          remoteAudioMusicTokenRef.current = null;
        };
      }

      const handleRemoteError = (error: unknown) => {
        if (mounted) {
          logger.debug("[game-canvas] remote render failed", error);
        }
      };

      if (sessionId) {
        if (remoteVisualMode === "frame") {
          const loop = startRemoteFrameRenderLoop({
                tileCanvas,
                sessionId,
                refreshMs: remoteRefreshMs,
                scale: remoteFrameScale,
                advanceFrames: remoteAdvanceFrames,
                instantMode: remoteInstantMode,
                onAudioSnapshot: (audio) => {
                  const token = audio?.musicToken ?? null;
                  if (!token || token === remoteAudioMusicTokenRef.current) {
                    return;
                  }
                  remoteAudioMusicTokenRef.current = token;
                  remoteAudioMirrorGameRef.current?.playMusic(token, audio?.musicRole ?? "map");
                },
                onError: handleRemoteError,
              });
          remoteFrameRefreshRef.current = loop.refresh;
          cleanupRenderLoop = loop.stop;
        } else {
          cleanupRenderLoop = startRemoteRenderLoop({
                tileCanvas,
                textCanvas,
                advanceFrames: remoteAdvanceFrames,
                refreshMs: remoteRefreshMs,
                shouldShowTileCanvas: () => showTileCanvasRef.current,
                shouldShowTextCanvas: () => showTextCanvasRef.current,
                callTool: (name, args) => callRemoteTool(name, args),
                onError: handleRemoteError,
              });
        }
      }
    } else if (!autoStart) {
      textUiRef.current = null;
      gameRef.current = null;
      onGameReady?.(null);
      postEvent = null;
      onPostEventReady?.(postEvent);
    } else {
      // Keep both local renderers alive so toggling renderer mode does not
      // force a full Game.create() restart.
      const renderCanvas = tileCanvas ?? primaryCanvas;
      const { ui, textUi } = buildUi(renderCanvas, { rendererMode: "both", scale: 1 });
      textUiRef.current = textUi;
      const startGame = async () => {
        logger.debug("[game-canvas] startGame execution", { autoStart, isServerMode });
        const envSlot = process.env.NEXT_PUBLIC_LOAD_SLOT;
        const resolvedLoadSlot =
          loadSlot?.trim() || (envSlot && envSlot.trim().length > 0 ? envSlot : MANUAL_SAVE_SLOT);
        const startLoadSlot = resolvedLoadSlot;
        logger.debug("[game-canvas] asset load triggered", {
          loadSlot: startLoadSlot,
          rendererMode,
          runtimeMode,
          playIntro,
          newGame,
        });
        const game = await Game.create(ui, {
          loadSlot: startLoadSlot,
          muted: initialMutedRef.current,
          initialState: playIntro ? "intro" : undefined,
          playIntro,
          newGame,
          onLoadProgress,
          preloadMode,
        });
        logger.debug("[game-canvas] Game.create resolved, assets ready", {
          loadSlot: startLoadSlot,
          runtimeMode,
          playIntro,
        });
        if (!mounted) {
          game.destroy();
          return;
        }
        gameRef.current = game;
        game.setAudioMuted(muted ?? false);
        game.setMusicMuted(musicMuted);
        game.start();
        onGameReady?.(game);
      };
      void startGame();
      postEvent = (event: GameEngineEvent) => {
        const game = gameRef.current;
        if (!game) {
          return;
        }
        game.postEvent(event as InstanceType<typeof gameEngine.event.Event>);
      };
      onPostEventReady?.(postEvent);
    }

    const previousAdvanceTime = (window as GameWindowHooks).advanceTime;
    const previousRenderToText = (window as GameWindowHooks).render_game_to_text;
    const previousJumpScene = (window as GameWindowHooks).jump_game_scene;
    const previousJumpSpawn = (window as GameWindowHooks).jump_game_spawn;
    const previousDebugStatus = (window as GameWindowHooks).get_game_debug_status;
    const previousSaveGameToSlot = (window as GameWindowHooks).save_game_to_slot;
    const previousDeleteSaveSlot = (window as GameWindowHooks).delete_save_slot;
    const previousHasSaveSlot = (window as GameWindowHooks).has_save_slot;
    const previousTriggerGameAutosave = (window as GameWindowHooks).trigger_game_autosave;
    const previousGetGameBenchmark = (window as GameWindowHooks).get_game_benchmark;
    const previousClearGameBenchmark = (window as GameWindowHooks).clear_game_benchmark;
    const previousGetTextRenderBenchmark = (window as GameWindowHooks).get_text_render_benchmark;
    const previousClearTextRenderBenchmark = (window as GameWindowHooks).clear_text_render_benchmark;
    const previousRunGameScript = (window as GameWindowHooks).run_game_script;
    const previousPostGameEvent = (window as GameWindowHooks).post_game_event;
    (window as GameWindowHooks).advanceTime = async (ms: number) => {
      const game = gameRef.current;
      if (!game) {
        return;
      }
      const steps = Math.max(1, Math.round(Math.max(0, ms) / GB_FRAME_DURATION_MS));
      for (let i = 0; i < steps; i += 1) {
        game.tick();
      }
      await Promise.resolve();
    };
    (window as GameWindowHooks).render_game_to_text = (): string => {
      const game = gameRef.current;
      if (!game) {
        return JSON.stringify({ mode: "uninitialized" });
      }
      return JSON.stringify(buildLocalTextSnapshotPayload(game));
    };
    (window as GameWindowHooks).jump_game_scene = async (scene: string) => {
      const game = gameRef.current;
      if (!game) {
        return;
      }
      await game.debugJumpToScene(scene as Parameters<Game["debugJumpToScene"]>[0]);
    };
    (window as GameWindowHooks).jump_game_spawn = async (spawn: string | number) => {
      const game = gameRef.current;
      if (!game) {
        return;
      }
      const numericSpawn =
        typeof spawn === "number"
          ? spawn
          : Number.parseInt(String(spawn), 10);
      if (!Number.isFinite(numericSpawn)) {
        throw new Error(`Invalid spawn '${String(spawn)}'.`);
      }
      await game.debugJumpToSpawn(numericSpawn as Parameters<Game["debugJumpToSpawn"]>[0]);
    };
    (window as GameWindowHooks).get_game_debug_status = (): string => {
      const game = gameRef.current;
      if (!game) {
        return JSON.stringify({ mode: "uninitialized" });
      }
      return JSON.stringify(game.getDebugStatus());
    };
    (window as GameWindowHooks).save_game_to_slot = async (
      slot: string,
      options?: { withHistory?: boolean }
    ): Promise<boolean> => {
      const game = gameRef.current;
      if (!game) {
        return false;
      }
      return game.debugSaveToSlot(slot, options);
    };
    (window as GameWindowHooks).delete_save_slot = async (slot: string): Promise<boolean> => {
      const game = gameRef.current;
      if (!game) {
        return false;
      }
      return game.debugDeleteSaveSlot(slot);
    };
    (window as GameWindowHooks).has_save_slot = async (slot: string): Promise<boolean> => {
      const game = gameRef.current;
      if (!game) {
        return false;
      }
      return game.debugHasSaveSlot(slot);
    };
    (window as GameWindowHooks).trigger_game_autosave = async (
      reason: "battle_complete" | "player_steps" = "battle_complete",
      count?: number
    ): Promise<void> => {
      const game = gameRef.current;
      if (!game) {
        return;
      }
      await game.debugTriggerAutosave(reason, count);
    };
    (window as GameWindowHooks).get_game_benchmark = (
      slowFrameThresholdMs: number = GB_FRAME_DURATION_MS
    ) => {
      const thresholdMs =
        Number.isFinite(slowFrameThresholdMs) && slowFrameThresholdMs >= 0
          ? Number(slowFrameThresholdMs)
          : GB_FRAME_DURATION_MS;
      const game = gameRef.current;
      if (!game) {
        return {
          enabled: false,
          thresholdMs,
          reason: "uninitialized",
        };
      }
      const benchmark = game.getBenchmark();
      if (!benchmark) {
        return {
          enabled: false,
          thresholdMs,
          state: game.getState(),
          reason: "disabled",
        };
      }
      const recentFrames = benchmark.getRecentFrames();
      return {
        enabled: true,
        thresholdMs,
        state: game.getState(),
        latestFrame: recentFrames[0] ?? null,
        recentFrames,
        slowFrames: benchmark.getSlowFrames(thresholdMs),
      };
    };
    (window as GameWindowHooks).clear_game_benchmark = (): void => {
      gameRef.current?.clearBenchmark();
    };
    (window as GameWindowHooks).get_text_render_benchmark = (): TextRenderBenchmarkSnapshot =>
      cloneTextRenderBenchmark(textRenderBenchmarkRef.current);
    (window as GameWindowHooks).clear_text_render_benchmark = (): void => {
      resetTextRenderBenchmark(textRenderBenchmarkRef.current);
    };
    (window as GameWindowHooks).run_game_script = async (
      script: string | unknown[]
    ): Promise<string> => {
      const game = gameRef.current;
      if (!game) {
        return JSON.stringify({ complete: false, reason: "uninitialized" });
      }
      const parsed = Array.isArray(script) ? script : parseVisualDebugScript(script);
      const result = await runVisualDebugScript(game, parsed);
      return JSON.stringify(result);
    };
    (window as GameWindowHooks).post_game_event = (event) => {
      const game = gameRef.current;
      if (!game) {
        return;
      }
      game.postEvent(
        new gameEngine.event.Event(event.type, {
          key: event.key ?? null,
          code: event.code ?? event.key ?? null,
          button: event.button ?? null,
          direction: event.direction ?? null,
          is_press: event.is_press ?? null,
          text: event.text ?? null,
          unicode: event.text ?? null,
        }) as InstanceType<typeof gameEngine.event.Event>
      );
    };

    if (!readOnly) {
      setTimeout(() => {
        primaryCanvas.focus({ preventScroll: true });
      }, 0);
    }

    const heldKeys = new Set<string | number>();
    const repeatFramesByKey = new Map<string | number, number>();
    const heldGamepadControls = new Set<string>();
    const nextGamepadState = createEmptyGamepadState();
    const pressedButtonSet = new Set<string>();
    const emitInputState = () => {
      if (!onInputStateChange || !mounted) {
        return;
      }
      pressedButtonSet.clear();
      for (const control of heldGamepadControls) {
        pressedButtonSet.add(control);
      }
      for (const key of heldKeys) {
        const button = mapKeyToButton(key);
        if (button) {
          pressedButtonSet.add(button);
        }
        const direction = mapKeyToDirection(key);
        if (direction) {
          pressedButtonSet.add(direction);
        }
      }
      const pressedKeys: Array<string | number> = [];
      for (const key of heldKeys) {
        pressedKeys.push(key);
      }
      onInputStateChange({
        pressedButtons: Array.from(pressedButtonSet),
        pressedKeys,
      });
    };
    const clearHeldKeyboardInput = () => {
      if (heldKeys.size === 0 && repeatFramesByKey.size === 0) {
        return;
      }
      heldKeys.clear();
      repeatFramesByKey.clear();
      emitInputState();
    };
    const shouldIgnoreKeyEvent = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (!target) {
        return false;
      }
      const tag = target.tagName;
      return target.isContentEditable || tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
    };
    const isTextEntryElement = (element: Element | null): boolean => {
      const target = element as HTMLElement | null;
      if (!target) {
        return false;
      }
      const tag = target.tagName;
      return target.isContentEditable || tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
    };
    const shouldIgnorePointerEvent = (event: PointerEvent) => {
      const target = event.target as HTMLElement | null;
      if (!target) {
        return false;
      }
      const tag = target.tagName;
      return target.isContentEditable || tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || tag === "BUTTON";
    };
    const isInputTarget = (element: Element | null): boolean => {
      return element === tileCanvas || element === textCanvas;
    };
    const isMappedControlKey = (key: string | number | null): boolean => {
      if (key == null) {
        return false;
      }
      return mapKeyToButton(key) !== null || mapKeyToDirection(key) !== null;
    };
    const shouldApplySyntheticRepeat = (key: string | number | null): boolean => {
      const game = gameRef.current;
      const direction = key == null ? null : normalizeRepeatDirection(mapKeyToDirection(key));
      const mappedControl = isMappedControlKey(key);
      const gameStateName = game ? game.getState() : null;
      const overworld = game ? ((game.getOverworld() as { input_capture_active?: boolean } | null) ?? null) : null;
      const inputCaptureActive = Boolean(overworld?.input_capture_active);
      const unownInputActive = Boolean(game?.getGameState()?.wram?.wUnownState ?? 0);
      return shouldApplySyntheticRepeatPolicy({
        key,
        mappedControl,
        direction,
        gameState: gameStateName,
        inputCaptureActive,
        unownInputActive,
      });
    };
    const canAcceptKeyboardInput = (): boolean => {
      if (isInputTarget(document.activeElement)) {
        return true;
      }
      if (isServerMode) {
        return true;
      }
      if (!isTextEntryElement(document.activeElement)) {
        return true;
      }
      const game = gameRef.current;
      const state = game?.getGameState?.();
      const isUnownModalActive = Boolean(state?.wram?.wUnownState);
      const isOverworldCaptureActive = Boolean(
        (game?.getOverworld?.() as { input_capture_active?: boolean } | null)
          ?.input_capture_active,
      );
      return isUnownModalActive || isOverworldCaptureActive;
    };
    const pendingLocalMcpEchoInputs: MirroredMcpInput[] = [];
    const mirrorNativeInputToMcp = (event: GameEngineEvent): void => {
      if (isServerMode || readOnly || !mcpActionMirrorSessionId) {
        return;
      }
      const isPress = event.is_press ?? event.type === gameEngine.KEYDOWN;
      if (!isPress) {
        return;
      }
      const direction = normalizeRepeatDirection(
        event.direction ?? mapKeyToDirection(event.code ?? event.key ?? null)
      );
      const button = normalizeMcpMirrorButton(
        event.button ?? mapKeyToButton(event.code ?? event.key ?? null)
      );
      const input: MirroredMcpInput | null =
        direction ? { kind: "move", value: direction } :
          button ? { kind: "button", value: button } :
            null;
      if (!input) {
        return;
      }
      pendingLocalMcpEchoInputs.push(input);
      const toolName = input.kind === "move" ? "move" : "press";
      const args = input.kind === "move" ? { direction: input.value } : { button: input.value };
      void callRemoteTool(toolName, args).catch((error) => {
        const index = pendingLocalMcpEchoInputs.findIndex((pending) => isSameMirroredMcpInput(pending, input));
        if (index >= 0) {
          pendingLocalMcpEchoInputs.splice(index, 1);
        }
        logger.debug("[game-canvas] native MCP input mirror failed", error);
      });
    };
    const consumePendingLocalMcpEcho = (input: MirroredMcpInput): boolean => {
      const index = pendingLocalMcpEchoInputs.findIndex((pending) => isSameMirroredMcpInput(pending, input));
      if (index < 0) {
        return false;
      }
      pendingLocalMcpEchoInputs.splice(index, 1);
      return true;
    };
    const postKeyboardEvent = (
      type: string | number,
      key: string,
      code: string,
      options: PostKeyboardEventOptions = {}
    ): void => {
      const lookupKey = code || key;
      const direction = mapKeyToDirection(lookupKey) ?? mapKeyToDirection(key);
      const button = mapKeyToButton(lookupKey) ?? mapKeyToButton(key);
      const engineEvent = new gameEngine.event.Event(type, {
        key,
        code,
        direction,
        button,
        is_press: type === gameEngine.KEYDOWN,
      });
      postEvent?.(engineEvent);
      if (options.mirrorToMcp !== false) {
        mirrorNativeInputToMcp(engineEvent);
      }
    };
    let mcpMirrorTimer: number | null = null;
    const mcpMirrorReleaseTimers: number[] = [];
    const mcpMirrorLastTotalRef = { current: null as number | null };
    const replayMirroredMcpInput = (input: MirroredMcpInput): void => {
      if (!mounted || readOnly || isServerMode || !gameRef.current) {
        return;
      }
      const key =
        input.kind === "move"
          ? ({
              up: "ArrowUp",
              down: "ArrowDown",
              left: "ArrowLeft",
              right: "ArrowRight",
            } as const)[input.value]
          : ({
              a: "KeyZ",
              b: "KeyX",
              start: "Enter",
              select: "Backspace",
            } as const)[input.value];
      const keyValue =
        key === "KeyZ" ? "z" :
          key === "KeyX" ? "x" :
            key === "Enter" ? "Enter" :
              key === "Backspace" ? "Backspace" :
                key;
      postKeyboardEvent(gameEngine.KEYDOWN, keyValue, key, { mirrorToMcp: false });
      const holdMs = input.kind === "move" ? 180 : 60;
      const timerId = window.setTimeout(() => {
        postKeyboardEvent(gameEngine.KEYUP, keyValue, key, { mirrorToMcp: false });
      }, holdMs);
      mcpMirrorReleaseTimers.push(timerId);
    };
    const pollMcpActionMirror = async (): Promise<void> => {
      if (!mounted || readOnly || isServerMode || !mcpActionMirrorSessionId || !gameRef.current) {
        return;
      }
      const origin = typeof window !== "undefined" ? window.location.origin : undefined;
      const result = await callMcpTool("recent_events", { limit: 20 }, {
        baseUrl: withSessionId(MCP_BASE_URL, mcpActionMirrorSessionId, origin),
      });
      const text = extractTextBlock(result);
      if (!text) {
        return;
      }
      let payload: unknown;
      try {
        payload = JSON.parse(text);
      } catch {
        return;
      }
      const eventPayload = payload as {
        total?: unknown;
        events?: Array<{ action?: unknown }>;
      };
      const total = typeof eventPayload.total === "number" ? eventPayload.total : null;
      const events = Array.isArray(eventPayload.events) ? eventPayload.events : [];
      if (total === null) {
        return;
      }
      const previousTotal = mcpMirrorLastTotalRef.current;
      mcpMirrorLastTotalRef.current = total;
      if (previousTotal === null || total <= previousTotal) {
        return;
      }
      const newCount = Math.min(total - previousTotal, events.length);
      for (const event of events.slice(events.length - newCount)) {
        if (typeof event.action !== "string") {
          continue;
        }
        const input = parseMirroredMcpAction(event.action);
        if (input && !consumePendingLocalMcpEcho(input)) {
          replayMirroredMcpInput(input);
        }
      }
    };
    if (
      !isServerMode &&
      !readOnly &&
      mcpActionMirrorSessionId &&
      typeof window !== "undefined"
    ) {
      const poll = () => {
        void pollMcpActionMirror().catch((error) => {
          if (mounted) {
            logger.debug("[game-canvas] MCP action mirror failed", error);
          }
        });
      };
      poll();
      mcpMirrorTimer = window.setInterval(
        poll,
        Math.max(50, Math.floor(mcpActionMirrorPollMs))
      );
    }
    const postCanvasConfirmTap = (): void => {
      const game = gameRef.current;
      if (!game || game.getState() !== "title") {
        return;
      }
      postKeyboardEvent(gameEngine.KEYDOWN, "z", "KeyZ");
      postKeyboardEvent(gameEngine.KEYUP, "z", "KeyZ");
    };
    const stepHeldControlRepeats = (framesToAdvance: number): void => {
      if (framesToAdvance <= 0 || !canAcceptKeyboardInput()) {
        return;
      }
      for (let frame = 0; frame < framesToAdvance; frame += 1) {
        const activeKeys = Array.from(repeatFramesByKey.keys());
        for (const keyValue of activeKeys) {
          if (!heldKeys.has(keyValue)) {
            repeatFramesByKey.delete(keyValue);
            continue;
          }
          if (!shouldApplySyntheticRepeat(keyValue)) {
            repeatFramesByKey.delete(keyValue);
            continue;
          }
          const nextFrameBudget = (repeatFramesByKey.get(keyValue) ?? CONTROL_REPEAT_INITIAL_DELAY_FRAMES) - 1;
          if (nextFrameBudget <= 0) {
            const keyToken = String(keyValue);
            postKeyboardEvent(gameEngine.KEYDOWN, keyToken, keyToken);
            repeatFramesByKey.set(keyValue, CONTROL_REPEAT_INTERVAL_FRAMES);
          } else {
            repeatFramesByKey.set(keyValue, nextFrameBudget);
          }
        }
      }
    };
    const handleKeyEvent = (event: KeyboardEvent) => {
      if (!mounted || shouldIgnoreKeyEvent(event)) {
        return;
      }
      const keyValue = event.code ?? event.key ?? null;
      const mappedControl = isMappedControlKey(keyValue);
      const isRepeatedControlPress =
        event.type === "keydown" && event.repeat && mappedControl;
      if (readOnly || !canAcceptKeyboardInput()) {
        return;
      }
      if (isRepeatedControlPress) {
        if (event.code && PREVENT_DEFAULT_KEY_CODES.has(event.code)) {
          event.preventDefault();
        }
        return;
      }
      if (keyValue !== null) {
        if (event.type === "keyup") {
          heldKeys.delete(keyValue);
          repeatFramesByKey.delete(keyValue);
        } else {
          const firstPress = !heldKeys.has(keyValue);
          if (mappedControl && !firstPress) {
            if (event.code && PREVENT_DEFAULT_KEY_CODES.has(event.code)) {
              event.preventDefault();
            }
            return;
          }
          heldKeys.add(keyValue);
          if (mappedControl && firstPress && shouldApplySyntheticRepeat(keyValue)) {
            repeatFramesByKey.set(keyValue, CONTROL_REPEAT_INITIAL_DELAY_FRAMES);
          }
          gameRef.current?.unlockAudio();
        }
        emitInputState();
      }
      const type = event.type === "keyup" ? gameEngine.KEYUP : gameEngine.KEYDOWN;
      postKeyboardEvent(type, String(event.key ?? keyValue ?? ""), String(event.code ?? event.key ?? keyValue ?? ""));
      if (event.code && PREVENT_DEFAULT_KEY_CODES.has(event.code)) {
        event.preventDefault();
      }
    };
    window.addEventListener("keydown", handleKeyEvent, { capture: true });
    window.addEventListener("keyup", handleKeyEvent, { capture: true });
    window.addEventListener("blur", clearHeldKeyboardInput);
    document.addEventListener("visibilitychange", clearHeldKeyboardInput);
    const handlePointerDown = (event: PointerEvent) => {
      if (!mounted) {
        return;
      }
      if (!readOnly) {
        gameRef.current?.unlockAudio();
      }
      if (shouldIgnorePointerEvent(event)) {
        return;
      }
      const target = event.target as Element | null;
      const targetCanvas = isInputTarget(target) ? (target as HTMLCanvasElement) : primaryCanvas;
      targetCanvas?.focus({ preventScroll: true });
      if (isInputTarget(target)) {
        postCanvasConfirmTap();
      }
    };
    window.addEventListener("pointerdown", handlePointerDown, { capture: true });

    const gamepadState: GamepadState = createEmptyGamepadState();
    const updateGamepadState = (nextState: GamepadState) => {
      let changed = false;
      for (const control of GAMEPAD_CONTROLS) {
        const wasPressed = gamepadState[control];
        const isPressed = nextState[control];
        if (wasPressed === isPressed) {
          continue;
        }
        gamepadState[control] = isPressed;
        changed = true;
        if (isPressed) {
          heldGamepadControls.add(control);
          gameRef.current?.unlockAudio();
        } else {
          heldGamepadControls.delete(control);
        }
        const event = buildGamepadEvent(control, isPressed);
        postEvent?.(event);
        mirrorNativeInputToMcp(event);
      }
      if (changed) {
        emitInputState();
      }
    };
    let inputFrame: number | null = null;
    let controlRepeatLastTimestampMs: number | null = null;
    let controlRepeatRemainderMs = 0;
    const pollInputs = (timestampMs: number) => {
      if (!mounted || readOnly) {
        return;
      }
      if (typeof document !== "undefined" && document.visibilityState === "hidden") {
        controlRepeatLastTimestampMs = timestampMs;
        controlRepeatRemainderMs = 0;
        readGamepadState(null, nextGamepadState);
        updateGamepadState(nextGamepadState);
        inputFrame = window.requestAnimationFrame(pollInputs);
        return;
      }
      if (controlRepeatLastTimestampMs === null) {
        controlRepeatLastTimestampMs = timestampMs;
      } else {
        const deltaMs = Math.max(0, timestampMs - controlRepeatLastTimestampMs);
        controlRepeatLastTimestampMs = timestampMs;
        controlRepeatRemainderMs = Math.min(
          controlRepeatRemainderMs + deltaMs,
          MAX_CONTROL_REPEAT_ACCUMULATED_MS,
        );
        const framesToAdvance = Math.floor(controlRepeatRemainderMs / GB_FRAME_DURATION_MS);
        if (framesToAdvance > 0) {
          controlRepeatRemainderMs -= framesToAdvance * GB_FRAME_DURATION_MS;
          stepHeldControlRepeats(framesToAdvance);
        }
      }
      const pads = typeof navigator !== "undefined" ? navigator.getGamepads?.() : null;
      const pad = typeof pads !== "undefined" && pads !== null ? pickActiveGamepad(pads) : null;
      readGamepadState(pad, nextGamepadState);
      updateGamepadState(nextGamepadState);
      inputFrame = window.requestAnimationFrame(pollInputs);
    };
    const canPollInputs =
      typeof window !== "undefined" &&
      typeof window.requestAnimationFrame === "function";
    if (canPollInputs) {
      inputFrame = window.requestAnimationFrame(pollInputs);
    }

    return () => {
      mounted = false;
      remoteActiveRef.current = false;
      remoteFrameRefreshRef.current = null;
      if (mcpMirrorTimer !== null) {
        window.clearInterval(mcpMirrorTimer);
      }
      for (const timerId of mcpMirrorReleaseTimers) {
        window.clearTimeout(timerId);
      }
      cleanupRemoteAudioMirror?.();
      cleanupRenderLoop?.();
      for (const timerId of remoteDirectionTimers.values()) {
        window.clearInterval(timerId);
      }
      remoteDirectionTimers.clear();
      readGamepadState(null, nextGamepadState);
      updateGamepadState(nextGamepadState);
      repeatFramesByKey.clear();
      if (inputFrame !== null && typeof window.cancelAnimationFrame === "function") {
        window.cancelAnimationFrame(inputFrame);
      }
      gameRef.current?.destroy();
      gameRef.current = null;
      textUiRef.current = null;
      onPostEventReady?.(null);
      onGameReady?.(null);
      window.removeEventListener("keydown", handleKeyEvent, { capture: true });
      window.removeEventListener("keyup", handleKeyEvent, { capture: true });
      window.removeEventListener("blur", clearHeldKeyboardInput);
      document.removeEventListener("visibilitychange", clearHeldKeyboardInput);
      window.removeEventListener("pointerdown", handlePointerDown, { capture: true });
      (window as GameWindowHooks).advanceTime = previousAdvanceTime;
      (window as GameWindowHooks).render_game_to_text = previousRenderToText;
      (window as GameWindowHooks).jump_game_scene = previousJumpScene;
      (window as GameWindowHooks).jump_game_spawn = previousJumpSpawn;
      (window as GameWindowHooks).get_game_debug_status = previousDebugStatus;
      (window as GameWindowHooks).save_game_to_slot = previousSaveGameToSlot;
      (window as GameWindowHooks).delete_save_slot = previousDeleteSaveSlot;
      (window as GameWindowHooks).has_save_slot = previousHasSaveSlot;
      (window as GameWindowHooks).trigger_game_autosave = previousTriggerGameAutosave;
      (window as GameWindowHooks).get_game_benchmark = previousGetGameBenchmark;
      (window as GameWindowHooks).clear_game_benchmark = previousClearGameBenchmark;
      (window as GameWindowHooks).get_text_render_benchmark = previousGetTextRenderBenchmark;
      (window as GameWindowHooks).clear_text_render_benchmark = previousClearTextRenderBenchmark;
      (window as GameWindowHooks).run_game_script = previousRunGameScript;
      (window as GameWindowHooks).post_game_event = previousPostGameEvent;
    };
  }, [
    autoStart,
    onGameReady,
    onInputStateChange,
    onLoadProgress,
    onPostEventReady,
    isServerMode,
    isRemoteFrameMode,
    readOnly,
    preloadMode,
    playIntro,
    remoteAdvanceFrames,
    remoteFrameScale,
    remoteInstantMode,
    remoteRefreshMs,
    remoteVisualMode,
    runtimeMode,
    sessionId,
    mcpActionMirrorPollMs,
    mcpActionMirrorSessionId,
  ]);

  useEffect(() => {
    gameRef.current?.setAudioMuted(muted ?? false);
    remoteAudioMirrorGameRef.current?.setAudioMuted(muted ?? false);
  }, [muted]);

  useEffect(() => {
    gameRef.current?.setMusicMuted(musicMuted);
    remoteAudioMirrorGameRef.current?.setMusicMuted(musicMuted);
  }, [musicMuted]);

  useEffect(() => {
    if (lastRemoteFrameRefreshKeyRef.current === remoteFrameRefreshKey) {
      return;
    }
    lastRemoteFrameRefreshKeyRef.current = remoteFrameRefreshKey;
    if (!isServerMode || !isRemoteFrameMode) {
      return;
    }
    remoteFrameRefreshRef.current?.();
  }, [isRemoteFrameMode, isServerMode, remoteFrameRefreshKey]);

  useEffect(() => {
    if (isServerMode || isRemoteFrameMode) {
      return;
    }
    resetTextRenderBenchmark(textRenderBenchmarkRef.current);
    const canvas = textCanvasRef.current;
    if (!canvas) {
      return;
    }
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      return;
    }
    let timerId: number | null = null;

    let lastSnapshot: TextSnapshot | null = null;
    let lastWidth = 0;
    let lastHeight = 0;
    let lastLayout: TextRenderLayout | null = null;
    let lastLayoutLines: SnapshotLine[] | null = null;
    let lastFont: CompactBitmapFont | null = null;
    let fontLoadRequested = false;
    const textRenderBenchmark = textRenderBenchmarkRef.current;
    textRenderBenchmark.enabled = true;

    const renderLoop = () => {
      if (typeof document !== "undefined" && document.visibilityState === "hidden") {
        timerId = window.setTimeout(renderLoop, TEXT_SNAPSHOT_REFRESH_MS);
        return;
      }
      const snapshotReadStart = getNowMs();
      const snapshot = textUiRef.current?.getSnapshot() ?? null;
      recordTextRenderBenchmarkPhase(
        textRenderBenchmark,
        "snapshotRead",
        Math.max(0, getNowMs() - snapshotReadStart),
      );
      const width = canvas.width;
      const height = canvas.height;
      const snapshotChanged = snapshot !== lastSnapshot;
      const needsLayout = snapshotChanged || width !== lastWidth || height !== lastHeight;
      const font = textBitmapFontRef.current ?? getLoadedCompactBitmapTextFont();
      if (!font) {
        if (!fontLoadRequested) {
          fontLoadRequested = true;
          void getCompactBitmapTextFont()
            .then((loadedFont) => {
              textBitmapFontRef.current = loadedFont;
            })
            .catch((error: unknown) => {
              logger.error("[game-canvas] failed to load compact bitmap text font", error);
            })
            .finally(() => {
              fontLoadRequested = false;
            });
        }
        timerId = window.setTimeout(renderLoop, TEXT_SNAPSHOT_REFRESH_MS);
        return;
      }
      const needsPaint = needsLayout || font !== lastFont;
      if (needsLayout) {
        const layoutStart = getNowMs();
        if (snapshotChanged || !lastLayoutLines) {
          lastLayoutLines = buildTextSnapshotLayout(snapshot);
        }
        lastSnapshot = snapshot;
        lastWidth = width;
        lastHeight = height;
        const layoutLines = lastLayoutLines ?? buildTextSnapshotLayout(snapshot);
        lastLayout = buildTextRenderLayout(canvas, layoutLines);
        recordTextRenderBenchmarkPhase(
          textRenderBenchmark,
          "layoutBuild",
          Math.max(0, getNowMs() - layoutStart),
        );
      }
      let painted = false;
      if (needsPaint && lastLayout && font) {
        const paintStart = getNowMs();
        drawTextRenderLayout(canvas, ctx, lastLayout, font);
        recordTextRenderBenchmarkPhase(
          textRenderBenchmark,
          "paint",
          Math.max(0, getNowMs() - paintStart),
        );
        painted = true;
      }
      textRenderBenchmark.iterations += 1;
      textRenderBenchmark.lastFrame = {
        snapshotChanged,
        layoutBuilt: needsLayout,
        painted,
        width,
        height,
        lineCount: lastLayout?.visibleLines.length ?? 0,
      };
      lastFont = font;
      timerId = window.setTimeout(renderLoop, TEXT_SNAPSHOT_REFRESH_MS);
    };

    renderLoop();

    return () => {
      if (timerId !== null) {
        window.clearTimeout(timerId);
      }
    };
  }, [rendererMode, isRemoteFrameMode, isServerMode, canvasHeightReservePx]);

  const showBothCanvases = showTileCanvas && showTextCanvas;
  const renderTileCanvas = showTileCanvas || !isServerMode;
  const renderTextCanvas = showTextCanvas || !isServerMode;

  return (
    <div
      ref={canvasShellRef}
      className={
        showBothCanvases
          ? "grid w-full grid-cols-1 justify-items-center items-center gap-3 overflow-hidden sm:grid-cols-2"
          : "flex w-full justify-center items-center overflow-hidden"
      }
    >
      {renderTileCanvas && (
        <canvas
          className={canvasClassName}
          style={canvasStyle}
          ref={tileCanvasRef}
          width={TILE_CANVAS_WIDTH}
          height={TILE_CANVAS_HEIGHT}
          tabIndex={0}
          aria-label="KrabbyClaw game canvas"
          onPointerDown={() => tileCanvasRef.current?.focus()}
        />
      )}
      {renderTextCanvas && (
        <canvas
          className={canvasClassName}
          style={canvasStyle}
          ref={textCanvasRef}
          width={TEXT_CANVAS_WIDTH}
          height={TEXT_CANVAS_HEIGHT}
          tabIndex={0}
          aria-label="KrabbyClaw text snapshot"
          onPointerDown={() => textCanvasRef.current?.focus()}
        />
      )}
    </div>
  );
});

GameCanvas.displayName = "GameCanvas";

export default GameCanvas;

type RemoteRenderOptions = {
  tileCanvas: HTMLCanvasElement | null;
  textCanvas: HTMLCanvasElement | null;
  advanceFrames: number;
  refreshMs: number;
  shouldShowTileCanvas: () => boolean;
  shouldShowTextCanvas: () => boolean;
  callTool: (name: string, args: Record<string, unknown>) => Promise<McpToolResult>;
  onError?: (error: unknown) => void;
};

type FrameResponse = {
  ok?: boolean;
  image?: string;
  width?: number;
  height?: number;
  frame?: number;
  audio?: AudioPlaybackSnapshot;
  error?: string;
};

type RemoteFrameRenderOptions = {
  tileCanvas: HTMLCanvasElement | null;
  sessionId: string;
  refreshMs: number;
  scale: number;
  advanceFrames: number;
  instantMode?: boolean;
  onAudioSnapshot?: (audio: AudioPlaybackSnapshot | undefined) => void;
  onError?: (error: unknown) => void;
};

const PUBLIC_SNAPSHOT_TOKEN =
  process.env.NEXT_PUBLIC_POKECRYSTAL_ARENA_SNAPSHOT_TOKEN?.trim() ??
  process.env.NEXT_PUBLIC_ARENA_SNAPSHOT_TOKEN?.trim() ??
  "";

const buildArenaFrameUrl = (
  sessionId: string,
  scale: number,
  advanceFrames: number,
  instantMode?: boolean
): string => {
  const params = new URLSearchParams();
  params.set("session_id", sessionId);
  params.set("scale", String(Math.min(8, Math.max(1, Math.floor(scale)))));
  params.set("advance", String(Math.max(0, Math.floor(advanceFrames))));
  if (instantMode !== undefined) {
    params.set("instant", instantMode ? "1" : "0");
  }
  return `/api/arena/frame?${params.toString()}`;
};

const extractTextBlock = (result: McpToolResult | null | undefined): string | null => {
  const content = result?.content ?? [];
  for (const block of content) {
    if (block?.type === "text" && typeof block.text === "string") {
      return block.text;
    }
  }
  return null;
};

const parseMirroredMcpAction = (action: string): MirroredMcpInput | null => {
  const parts = action.toLowerCase().split(":").filter(Boolean);
  for (let index = parts.length - 2; index >= 0; index -= 1) {
    const kind = parts[index];
    const value = parts[index + 1];
    if (kind === "move" && (value === "up" || value === "down" || value === "left" || value === "right")) {
      return { kind, value };
    }
    if (kind === "button" && (value === "a" || value === "b" || value === "start" || value === "select")) {
      return { kind, value };
    }
    if (kind === "press" && (value === "a" || value === "b" || value === "start" || value === "select")) {
      return { kind: "button", value };
    }
  }
  return null;
};

const normalizeMcpMirrorButton = (value: string | null | undefined): MirroredMcpButton | null => {
  const normalized = value?.trim().toLowerCase();
  if (normalized === "a" || normalized === "b" || normalized === "start" || normalized === "select") {
    return normalized;
  }
  return null;
};

const isSameMirroredMcpInput = (left: MirroredMcpInput, right: MirroredMcpInput): boolean =>
  left.kind === right.kind && left.value === right.value;

const buildSnapshotLinesFromText = (text: string): SnapshotLine[] => {
  const lines = text.split(/\r?\n/);
  if (!lines.length) {
    return [{ text: "(no text snapshot)", kind: "normal" }];
  }
  return lines.map((line) => ({ text: line, kind: "normal" }));
};

const drawTextSnapshotToCanvas = async (
  canvas: HTMLCanvasElement,
  text: string
): Promise<void> => {
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    return;
  }
  const font = await getCompactBitmapTextFont();
  const lines = buildSnapshotLinesFromText(text);
  const layout = buildTextRenderLayout(canvas, lines);
  drawTextRenderLayout(canvas, ctx, layout, font);
};

const startRemoteRenderLoop = (options: RemoteRenderOptions): (() => void) => {
  const {
    tileCanvas,
    textCanvas,
    advanceFrames,
    refreshMs,
    shouldShowTileCanvas,
    shouldShowTextCanvas,
    callTool,
    onError,
  } = options;
  let stopped = false;
  let inflight = false;

  const renderFrame = async () => {
    if (stopped || inflight) {
      return;
    }
    if (typeof document !== "undefined" && document.visibilityState === "hidden") {
      return;
    }
    inflight = true;
    try {
      const args = advanceFrames > 0 ? { advance_frames: advanceFrames } : {};
      const result = await callTool("observe", args);
      const text = extractTextBlock(result);
      if (!text) {
        return;
      }
      const showTileCanvas = shouldShowTileCanvas();
      const showTextCanvas = shouldShowTextCanvas();
      if (showTileCanvas && tileCanvas) {
        await drawTextSnapshotToCanvas(tileCanvas, text);
      }
      if (showTextCanvas && textCanvas) {
        await drawTextSnapshotToCanvas(textCanvas, text);
      }
    } catch (error) {
      onError?.(error);
    } finally {
      inflight = false;
    }
  };

  if (typeof window !== "undefined") {
    const intervalId = window.setInterval(renderFrame, refreshMs);
    void renderFrame();
    return () => {
      stopped = true;
      window.clearInterval(intervalId);
    };
  }

  return () => {
    stopped = true;
  };
};

const loadFrameImage = (source: string): Promise<HTMLImageElement> =>
  new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error("Unable to decode remote frame."));
    image.src = source;
  });

const drawFrameImageToCanvas = async (
  canvas: HTMLCanvasElement,
  source: string,
  dimensions?: { width?: number; height?: number },
): Promise<void> => {
  const image = await loadFrameImage(source);
  const width =
    typeof dimensions?.width === "number" && Number.isFinite(dimensions.width) && dimensions.width > 0
      ? Math.floor(dimensions.width)
      : canvas.width;
  const height =
    typeof dimensions?.height === "number" && Number.isFinite(dimensions.height) && dimensions.height > 0
      ? Math.floor(dimensions.height)
      : canvas.height;
  if (canvas.width !== width) {
    canvas.width = width;
  }
  if (canvas.height !== height) {
    canvas.height = height;
  }
  const context = canvas.getContext("2d");
  if (!context) {
    return;
  }
  context.imageSmoothingEnabled = false;
  if (typeof context.clearRect === "function") {
    context.clearRect(0, 0, canvas.width, canvas.height);
  }
  context.drawImage(image, 0, 0, canvas.width, canvas.height);
};

const startRemoteFrameRenderLoop = (options: RemoteFrameRenderOptions): { stop: () => void; refresh: () => void } => {
  const { tileCanvas, sessionId, refreshMs, scale, advanceFrames, instantMode, onAudioSnapshot, onError } = options;
  if (typeof window === "undefined" || !tileCanvas) {
    return { stop: () => undefined, refresh: () => undefined };
  }

  let stopped = false;
  let inflight = false;

  const renderFrame = async () => {
    if (stopped || inflight) {
      return;
    }
    if (typeof document !== "undefined" && document.visibilityState === "hidden") {
      return;
    }
    inflight = true;
    try {
      const response = await fetch(buildArenaFrameUrl(sessionId, scale, advanceFrames, instantMode), {
        cache: "no-store",
        headers: PUBLIC_SNAPSHOT_TOKEN ? { "x-mcp-token": PUBLIC_SNAPSHOT_TOKEN } : undefined,
      });
      if (!response.ok) {
        throw new Error(`Frame error: ${response.status}`);
      }
      const payload = (await response.json()) as FrameResponse;
      if (!payload.ok || !payload.image) {
        throw new Error(payload.error ?? "Frame missing.");
      }
      await drawFrameImageToCanvas(tileCanvas, `data:image/png;base64,${payload.image}`, {
        width: payload.width,
        height: payload.height,
      });
      onAudioSnapshot?.(payload.audio);
    } catch (error) {
      onError?.(error);
    } finally {
      inflight = false;
    }
  };

  const intervalId = window.setInterval(renderFrame, refreshMs);
  void renderFrame();
  return {
    stop: () => {
      stopped = true;
      window.clearInterval(intervalId);
    },
    refresh: () => {
      void renderFrame();
    },
  };
};

const TEXT_RENDER_FONT_SIZE_PX = 7;
const TEXT_RENDER_LINE_HEIGHT_PX = 8;
const TEXT_RENDER_GLYPH_WIDTH_PX = 8;
const TEXT_RENDER_GLYPH_HEIGHT_PX = 8;
const TEXT_RENDER_PRINTABLE_ASCII_START = 0x20;
const TEXT_RENDER_PRINTABLE_ASCII_END = 0x7e;
const TEXT_RENDER_FALLBACK_GLYPH_INDEX = 0x3f - TEXT_RENDER_PRINTABLE_ASCII_START;
const TEXT_RENDER_FONT_PATH = getAssetPath("gfx", "mobile", "ascii_font.png");
const TEXT_RENDER_BG_RGB = [3, 3, 18] as const;
const TEXT_RENDER_SELECTED_BG_RGB = [40, 48, 122] as const;
const TEXT_RENDER_HEADING_RGB = [159, 180, 255] as const;
const TEXT_RENDER_HINT_RGB = [166, 212, 255] as const;
const TEXT_RENDER_NORMAL_RGB = [231, 235, 255] as const;

type TextRenderRgb = readonly [number, number, number];
type CompactBitmapFont = {
  glyphWidth: number;
  glyphHeight: number;
  glyphMasks: Uint8Array[];
};

type MutableImageDataLike = {
  data: Uint8ClampedArray;
  width: number;
  height: number;
};

type TextRenderLayout = {
  margin: number;
  availableWidth: number;
  lineHeight: number;
  fontSize: number;
  visibleLines: SnapshotLine[];
  rasterLines: Array<{
    text: string;
    kind: SnapshotLine["kind"];
    glyphIndices: Uint8Array;
  }>;
};
let compactBitmapFont: CompactBitmapFont | null = null;
let compactBitmapFontPromise: Promise<CompactBitmapFont> | null = null;
const textRenderImageCache = new WeakMap<object, MutableImageDataLike>();
const textWrapCache = new Map<string, SnapshotLine[]>();
const textGlyphIndexCache = new Map<string, Uint8Array>();
const textRenderLineCache = new Map<
  string,
  {
    text: string;
    kind: SnapshotLine["kind"];
    glyphIndices: Uint8Array;
  }
>();
const textRenderPackedFillSupported = (() => {
  const buffer = new ArrayBuffer(4);
  const words = new Uint32Array(buffer);
  const bytes = new Uint8Array(buffer);
  words[0] = 0x11223344;
  return bytes[0] === 0x44;
})();
const TEXT_RENDER_CACHE_LIMIT = 512;

const pruneTextRenderCache = <T,>(cache: Map<string, T>): void => {
  if (cache.size < TEXT_RENDER_CACHE_LIMIT) {
    return;
  }
  const oldestKey = cache.keys().next().value;
  if (oldestKey !== undefined) {
    cache.delete(oldestKey);
  }
};

const wrapLayoutLinesToWidth = (lines: SnapshotLine[], maxChars: number): SnapshotLine[] => {
  const safeMaxChars = Math.max(1, Math.min(maxChars, MAX_TEXT_RENDER_CHARS));
  const wrapped: SnapshotLine[] = [];

  const wrapLine = (line: SnapshotLine): void => {
    const cacheKey = `${safeMaxChars}:${line.kind}:${line.text}`;
    const cached = textWrapCache.get(cacheKey);
    if (cached) {
      wrapped.push(...cached);
      return;
    }
    const wrappedLines: SnapshotLine[] = [];
    if (line.text.length === 0) {
      wrappedLines.push({ ...line, text: "" });
      pruneTextRenderCache(textWrapCache);
      textWrapCache.set(cacheKey, wrappedLines);
      wrapped.push(...wrappedLines);
      return;
    }
    if (line.text.length <= safeMaxChars) {
      wrappedLines.push(line);
      pruneTextRenderCache(textWrapCache);
      textWrapCache.set(cacheKey, wrappedLines);
      wrapped.push(...wrappedLines);
      return;
    }
    const words = line.text.split(" ");
    let current = "";
    const flush = () => {
      if (current.length) {
        wrappedLines.push({ ...line, text: current });
        current = "";
      }
    };
    for (const word of words) {
      const next = current ? `${current} ${word}` : word;
      if (next.length <= safeMaxChars) {
        current = next;
        continue;
      }
      flush();
      if (word.length <= safeMaxChars) {
        current = word;
        continue;
      }
      let chunk = "";
      for (const char of word) {
        if ((chunk + char).length > safeMaxChars) {
          if (chunk.length) {
            wrappedLines.push({ ...line, text: chunk });
          }
          chunk = char;
        } else {
          chunk += char;
        }
      }
      if (chunk.length) {
        current = chunk;
      }
    }
    flush();
    pruneTextRenderCache(textWrapCache);
    textWrapCache.set(cacheKey, wrappedLines);
    wrapped.push(...wrappedLines);
  };

  for (const line of lines) {
    wrapLine(line);
  }

  return wrapped.length ? wrapped : [{ text: "", kind: "normal" }];
};

const rgbToCss = ([red, green, blue]: TextRenderRgb): string =>
  `rgb(${red} ${green} ${blue})`;

const isPrintableAsciiCode = (code: number): boolean =>
  code >= TEXT_RENDER_PRINTABLE_ASCII_START && code <= TEXT_RENDER_PRINTABLE_ASCII_END;

export const normalizeBitmapTextChar = (value: string): string => {
  const firstChar = value[0] ?? "?";
  switch (firstChar) {
    case "▶":
    case "▷":
    case "►":
    case "▸":
      return ">";
    case "◀":
    case "◁":
    case "◄":
    case "▹":
      return "<";
    case "—":
    case "–":
      return "-";
    case "…":
      return ".";
    case "\t":
      return " ";
    default:
      break;
  }

  const normalized = firstChar.normalize("NFKD").replace(/[\u0300-\u036f]/g, "");
  const candidate = normalized[0] ?? firstChar;
  const code = candidate.codePointAt(0) ?? 0x3f;
  if (code >= 0x20 && code <= 0x7e) {
    return String.fromCodePoint(code);
  }
  return "?";
};

export const bitmapGlyphIndexForChar = (value: string): number => {
  const normalized = normalizeBitmapTextChar(value);
  const code = normalized.codePointAt(0) ?? 0x3f;
  if (!isPrintableAsciiCode(code)) {
    return TEXT_RENDER_FALLBACK_GLYPH_INDEX;
  }
  return code - TEXT_RENDER_PRINTABLE_ASCII_START;
};

const buildBitmapGlyphIndexLine = (text: string): Uint8Array => {
  const cached = textGlyphIndexCache.get(text);
  if (cached) {
    return cached;
  }
  const glyphIndices = new Uint8Array(text.length);
  for (let index = 0; index < text.length; index += 1) {
    glyphIndices[index] = bitmapGlyphIndexForChar(text[index]);
  }
  pruneTextRenderCache(textGlyphIndexCache);
  textGlyphIndexCache.set(text, glyphIndices);
  return glyphIndices;
};

const getRasterLine = (
  line: SnapshotLine,
): {
  text: string;
  kind: SnapshotLine["kind"];
  glyphIndices: Uint8Array;
} => {
  const cacheKey = `${line.kind}:${line.text}`;
  const cached = textRenderLineCache.get(cacheKey);
  if (cached) {
    return cached;
  }
  const rasterLine = {
    text: line.text,
    kind: line.kind,
    glyphIndices: buildBitmapGlyphIndexLine(line.text),
  };
  pruneTextRenderCache(textRenderLineCache);
  textRenderLineCache.set(cacheKey, rasterLine);
  return rasterLine;
};

const buildCompactBitmapFont = (
  sheet: { getImageData: () => ImageData }
): CompactBitmapFont => {
  const image = sheet.getImageData();
  const sheetColumns = Math.floor(image.width / TEXT_RENDER_GLYPH_WIDTH_PX);
  const sheetRows = Math.floor(image.height / TEXT_RENDER_GLYPH_HEIGHT_PX);
  const glyphCount = sheetColumns * sheetRows;
  const glyphMasks: Uint8Array[] = [];
  for (let glyphIndex = 0; glyphIndex < glyphCount; glyphIndex += 1) {
    const glyphX = (glyphIndex % sheetColumns) * TEXT_RENDER_GLYPH_WIDTH_PX;
    const glyphY =
      Math.floor(glyphIndex / sheetColumns) * TEXT_RENDER_GLYPH_HEIGHT_PX;
    const mask = new Uint8Array(TEXT_RENDER_GLYPH_WIDTH_PX * TEXT_RENDER_GLYPH_HEIGHT_PX);
    for (let y = 0; y < TEXT_RENDER_GLYPH_HEIGHT_PX; y += 1) {
      for (let x = 0; x < TEXT_RENDER_GLYPH_WIDTH_PX; x += 1) {
        const sourceX = glyphX + x;
        const sourceY = glyphY + y;
        const pixelIndex = (sourceY * image.width + sourceX) * 4;
        const alpha = image.data[pixelIndex + 3] ?? 0;
        const red = image.data[pixelIndex] ?? 255;
        const green = image.data[pixelIndex + 1] ?? 255;
        const blue = image.data[pixelIndex + 2] ?? 255;
        const luminance = (red + green + blue) / 3;
        mask[y * TEXT_RENDER_GLYPH_WIDTH_PX + x] =
          alpha > 0 && luminance < 200 ? 1 : 0;
      }
    }
    glyphMasks.push(mask);
  }
  return {
    glyphWidth: TEXT_RENDER_GLYPH_WIDTH_PX,
    glyphHeight: TEXT_RENDER_GLYPH_HEIGHT_PX,
    glyphMasks,
  };
};

const buildFallbackCompactBitmapFont = (): CompactBitmapFont => {
  const glyphMasks: Uint8Array[] = [];
  for (let code = TEXT_RENDER_PRINTABLE_ASCII_START; code <= TEXT_RENDER_PRINTABLE_ASCII_END; code += 1) {
    const mask = new Uint8Array(TEXT_RENDER_GLYPH_WIDTH_PX * TEXT_RENDER_GLYPH_HEIGHT_PX);
    if (code !== TEXT_RENDER_PRINTABLE_ASCII_START) {
      for (let y = 1; y < TEXT_RENDER_GLYPH_HEIGHT_PX - 1; y += 1) {
        for (let x = 1; x < TEXT_RENDER_GLYPH_WIDTH_PX - 1; x += 1) {
          if (x === 1 || x === TEXT_RENDER_GLYPH_WIDTH_PX - 2 || y === 1 || y === TEXT_RENDER_GLYPH_HEIGHT_PX - 2) {
            mask[y * TEXT_RENDER_GLYPH_WIDTH_PX + x] = 1;
          }
        }
      }
    }
    glyphMasks.push(mask);
  }
  return {
    glyphWidth: TEXT_RENDER_GLYPH_WIDTH_PX,
    glyphHeight: TEXT_RENDER_GLYPH_HEIGHT_PX,
    glyphMasks,
  };
};

const getCompactBitmapTextFont = async (): Promise<CompactBitmapFont> => {
  if (compactBitmapFont) {
    return compactBitmapFont;
  }
  if (compactBitmapFontPromise) {
    return compactBitmapFontPromise;
  }
  compactBitmapFontPromise = (async () => {
    let font: CompactBitmapFont;
    try {
      const loaded = await Promise.resolve(gameEngine.image.load(TEXT_RENDER_FONT_PATH));
      font = buildCompactBitmapFont(loaded);
      compactBitmapFont = font;
    } catch (error) {
      logger.debug("[game-canvas] compact bitmap font unavailable; using fallback", error);
      font = buildFallbackCompactBitmapFont();
    }
    return font;
  })();
  try {
    return await compactBitmapFontPromise;
  } finally {
    compactBitmapFontPromise = null;
  }
};

const getLoadedCompactBitmapTextFont = (): CompactBitmapFont | null => compactBitmapFont;

const resolveForegroundColour = (kind: SnapshotLine["kind"]): TextRenderRgb => {
  if (kind === "heading") {
    return TEXT_RENDER_HEADING_RGB;
  }
  if (kind === "hint") {
    return TEXT_RENDER_HINT_RGB;
  }
  return TEXT_RENDER_NORMAL_RGB;
};

export const buildTextRenderLayout = (
  canvas: HTMLCanvasElement,
  layoutLines: SnapshotLine[]
): TextRenderLayout => {
  const margin = 0;
  const availableWidth = Math.max(canvas.width - margin * 2, 1);
  const availableHeight = Math.max(canvas.height - margin * 2, 1);
  const fontSize = TEXT_RENDER_FONT_SIZE_PX;
  const lineHeight = TEXT_RENDER_LINE_HEIGHT_PX;
  const maxChars = Math.max(
    8,
    Math.floor(availableWidth / TEXT_RENDER_GLYPH_WIDTH_PX)
  );
  const wrappedLayout = wrapLayoutLinesToWidth(layoutLines, maxChars);
  const maxLines = Math.max(1, Math.floor(availableHeight / lineHeight));
  const visibleLines = wrappedLayout.slice(0, maxLines);
  const rasterLines = visibleLines.map((line) => getRasterLine(line));

  return {
    margin,
    availableWidth,
    lineHeight,
    fontSize,
    visibleLines,
    rasterLines,
  };
};

const drawBitmapTextLine = (
  image: MutableImageDataLike,
  font: CompactBitmapFont,
  glyphIndices: Uint8Array,
  x: number,
  y: number,
  color: TextRenderRgb
): void => {
  if (!glyphIndices.length) {
    return;
  }
  const data = image.data;
  let cursorX = x;
  for (let index = 0; index < glyphIndices.length; index += 1) {
    const glyphIndex = glyphIndices[index] ?? TEXT_RENDER_FALLBACK_GLYPH_INDEX;
    const mask =
      font.glyphMasks[glyphIndex] ?? font.glyphMasks[TEXT_RENDER_FALLBACK_GLYPH_INDEX];
    if (mask) {
      for (let pixelY = 0; pixelY < font.glyphHeight; pixelY += 1) {
        const targetY = y + pixelY;
        if (targetY < 0 || targetY >= image.height) {
          continue;
        }
        for (let pixelX = 0; pixelX < font.glyphWidth; pixelX += 1) {
          if (!mask[pixelY * font.glyphWidth + pixelX]) {
            continue;
          }
          const targetX = cursorX + pixelX;
          if (targetX < 0 || targetX >= image.width) {
            continue;
          }
          const offset = (targetY * image.width + targetX) * 4;
          data[offset] = color[0];
          data[offset + 1] = color[1];
          data[offset + 2] = color[2];
          data[offset + 3] = 255;
        }
      }
    }
    cursorX += font.glyphWidth;
  }
};

const packTextRenderColour = ([red, green, blue]: TextRenderRgb): number => {
  if (!textRenderPackedFillSupported) {
    return 0;
  }
  return ((255 << 24) | (blue << 16) | (green << 8) | red) >>> 0;
};

const fillImageRect = (
  image: MutableImageDataLike,
  x: number,
  y: number,
  width: number,
  height: number,
  color: TextRenderRgb,
): void => {
  const xStart = Math.max(0, x);
  const yStart = Math.max(0, y);
  const xEnd = Math.min(image.width, x + width);
  const yEnd = Math.min(image.height, y + height);
  if (xStart >= xEnd || yStart >= yEnd) {
    return;
  }
  if (textRenderPackedFillSupported && image.data.byteOffset % 4 === 0) {
    const packed = packTextRenderColour(color);
    const pixels = new Uint32Array(image.data.buffer, image.data.byteOffset, image.data.byteLength / 4);
    const imageWidth = image.width;
    for (let row = yStart; row < yEnd; row += 1) {
      pixels.fill(packed, row * imageWidth + xStart, row * imageWidth + xEnd);
    }
    return;
  }
  for (let row = yStart; row < yEnd; row += 1) {
    let offset = (row * image.width + xStart) * 4;
    for (let col = xStart; col < xEnd; col += 1) {
      image.data[offset] = color[0];
      image.data[offset + 1] = color[1];
      image.data[offset + 2] = color[2];
      image.data[offset + 3] = 255;
      offset += 4;
    }
  }
};

const createTextRenderImage = (
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
): MutableImageDataLike => {
  if (typeof ctx.createImageData === "function") {
    return ctx.createImageData(width, height);
  }
  return ctx.getImageData(0, 0, width, height);
};

const getReusableTextRenderImage = (
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
): MutableImageDataLike => {
  const cached = textRenderImageCache.get(ctx as object);
  if (cached && cached.width === width && cached.height === height) {
    return cached;
  }
  const created = createTextRenderImage(ctx, width, height);
  textRenderImageCache.set(ctx as object, created);
  return created;
};

export const drawTextRenderLayout = (
  canvas: HTMLCanvasElement,
  ctx: CanvasRenderingContext2D,
  layout: TextRenderLayout,
  font: CompactBitmapFont
): void => {
  ctx.imageSmoothingEnabled = false;
  const image = getReusableTextRenderImage(ctx, canvas.width, canvas.height);
  fillImageRect(image, 0, 0, canvas.width, canvas.height, TEXT_RENDER_BG_RGB);

  const { margin, availableWidth, lineHeight, rasterLines } = layout;

  for (let idx = 0; idx < rasterLines.length; idx += 1) {
    const y = margin + idx * lineHeight;
    const line = rasterLines[idx];
    if (line.kind === "selected") {
      fillImageRect(image, margin - 2, y - 1, availableWidth + 4, lineHeight + 2, TEXT_RENDER_SELECTED_BG_RGB);
    }
    const foreground = resolveForegroundColour(line.kind);
    drawBitmapTextLine(image, font, line.glyphIndices, margin, y, foreground);
  }
  ctx.putImageData(image as ImageData, 0, 0);
};

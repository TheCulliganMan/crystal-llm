"use client";

import "@/lib/pokecrystal-core/register-browser-adapters";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import type { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import { faAngleLeft } from "@fortawesome/free-solid-svg-icons/faAngleLeft";
import { faAngleRight } from "@fortawesome/free-solid-svg-icons/faAngleRight";
import { faBars } from "@fortawesome/free-solid-svg-icons/faBars";
import { faBug } from "@fortawesome/free-solid-svg-icons/faBug";
import { faCompress } from "@fortawesome/free-solid-svg-icons/faCompress";
import { faExpand } from "@fortawesome/free-solid-svg-icons/faExpand";
import { faFloppyDisk } from "@fortawesome/free-solid-svg-icons/faFloppyDisk";
import { faGamepad } from "@fortawesome/free-solid-svg-icons/faGamepad";
import { faGear } from "@fortawesome/free-solid-svg-icons/faGear";
import { faPlay } from "@fortawesome/free-solid-svg-icons/faPlay";
import { faUsers } from "@fortawesome/free-solid-svg-icons/faUsers";
import { GameCanvas } from "./game-canvas";
import { VirtualGamepad } from "./virtual-gamepad";
import { GuestSavePanel } from "./guest-save-panel";
import { SettingsPanel } from "./settings-panel";
import { VisualDebugPanel } from "./visual-debug-panel";
import { KeybindingsEditor } from "./keybindings-editor";
import type { BrandTheme } from "./settings-panel";
import {
  MultiplayerMenu,
  type MultiplayerLeaderboardEntry,
} from "@/components/multiplayer-menu";
import {
  MultiplayerChat,
  type MultiplayerChatLine,
} from "@/components/multiplayer-chat";
import { WebRTCConnection } from "@pokecrystal/core/multiplayer/webrtc-connection";
import { WebRTCBattleTransport } from "@pokecrystal/core/multiplayer/webrtc-battle-transport";
import { useMultiplayerStore } from "@pokecrystal/core/multiplayer/multiplayer-store";
import {
  MatchmakingService,
  type MatchmakingMode,
} from "@pokecrystal/core/multiplayer/matchmaking-service";
import {
  OverworldPresenceManager,
  type MultiplayerInteractionKind,
  type MultiplayerInteractionRequest,
  type MultiplayerInteractionResponse,
  type MultiplayerChatChannel,
  type MultiplayerChatMessage,
} from "@pokecrystal/core/multiplayer/overworld-presence";
import { LinkCableEmulator } from "@pokecrystal/core/multiplayer/link-cable";
import { TradeManager } from "@pokecrystal/core/multiplayer/trade-manager";
import type { RemoteOverworldPlayer } from "@pokecrystal/core/types/overworld";
import { pokemonSpeciesDisplayName, type Pokemon } from "@pokecrystal/core/core/models";
import { PlayerGender, TimeOfDay } from "@pokecrystal/core/core/enums";
import { canonicaliseTimeOfDay, DAY_HOUR, MORN_HOUR, NITE_HOUR } from "@pokecrystal/core/engine/systems/time";
import { GameButton } from "@pokecrystal/core/input/config";
import { getActiveKeyBindings, getKeyBindingsChangeEventName } from "@pokecrystal/core/input/user-bindings";
import type { Game, GameLoadProgress, MultiplayerBattleCompleteResult } from "./game";
import type { GameEngineEvent } from "@pokecrystal/core/ui/game-engine";
import type { RendererMode } from "./ui";
import {
  getNextRendererMode,
  getRendererModeActionLabel,
} from "./renderer-mode";
import { computeFullscreenCanvasLayout } from "./play-layout";
import { createSupabaseBrowserClient } from "@/lib/supabase/browser";
import {
  BRAND_THEME_STORAGE_KEY,
  THEME_STORAGE_KEY,
  isBrandThemeKey,
  isThemeKey,
  type ThemeKey,
} from "./theme-preferences";
import { applyBrandThemeToDocument } from "./brand-theme-dom";
import { MANUAL_SAVE_SLOT } from "@pokecrystal/core/core/save-slots";

type InputState = {
  pressedButtons: string[];
  pressedKeys: Array<string | number>;
};

type FullscreenCapableElement = HTMLElement & {
  webkitRequestFullscreen?: () => Promise<void> | void;
};

type FullscreenCapableDocument = Document & {
  webkitFullscreenElement?: Element | null;
  webkitExitFullscreen?: () => Promise<void> | void;
  webkitFullscreenEnabled?: boolean;
};

const THEME_CHANGED_EVENT = "pokecrystal:theme-change";
const KEY_LABEL_OVERRIDES: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  NumpadEnter: "Numpad Enter",
  Backspace: "Backspace",
  ShiftLeft: "Shift",
  ShiftRight: "Shift",
  Escape: "Escape",
};

const getLocalDayOfWeek = (): number => new Date().getDay();

const getLocalTimeOfDay = (): TimeOfDay => {
  const hour = new Date().getHours();
  if (hour < MORN_HOUR) {
    return TimeOfDay.NIGHT;
  }
  if (hour < DAY_HOUR) {
    return TimeOfDay.MORN;
  }
  if (hour < NITE_HOUR) {
    return TimeOfDay.DAY;
  }
  return TimeOfDay.NIGHT;
};

const RENDERER_MODE_LAPTOP_QUERY = "(min-width: 1200px)";
const PLACEHOLDER_SCALE = 2;
const PLACEHOLDER_WIDTH = 160 * PLACEHOLDER_SCALE;
const PLACEHOLDER_HEIGHT = 144 * PLACEHOLDER_SCALE;

const formatKeyLabel = (value: string): string => {
  if (value.startsWith("Key") && value.length > 3) {
    return value.slice(3);
  }
  return KEY_LABEL_OVERRIDES[value] ?? value;
};

const formatKeyLabels = (bindings: string[]): string[] => {
  const labels = bindings.map(formatKeyLabel);
  return Array.from(new Set(labels));
};

const areArraysEqual = <T,>(left: readonly T[], right: readonly T[]): boolean => {
  if (left === right) {
    return true;
  }
  if (left.length !== right.length) {
    return false;
  }
  for (let idx = 0; idx < left.length; idx += 1) {
    if (left[idx] !== right[idx]) {
      return false;
    }
  }
  return true;
};

const toPresenceDirection = (value: unknown): "up" | "down" | "left" | "right" => {
  if (value === "up" || value === "down" || value === "left" || value === "right") {
    return value;
  }
  return "down";
};

type ControlTone = "neutral" | "accent" | "ember";
type UtilityPanelView = "multiplayer" | "settings" | "saves" | "debug";
export type PlayPanelProps = {
  variant?: "default" | "desktop";
};
type MultiplayerSessionHello = {
  kind: MultiplayerInteractionKind;
  playerName: string;
  party: Pokemon[];
};

type ActiveMultiplayerSession = {
  requestId: string;
  kind: MultiplayerInteractionKind;
  isHost: boolean;
  peerUserId: string;
  peerName: string;
  connection: WebRTCConnection;
  timeoutId: number | null;
  connected: boolean;
  started: boolean;
  helloSent: boolean;
  remoteHello: MultiplayerSessionHello | null;
};

const isMultiplayerSessionHello = (value: unknown): value is MultiplayerSessionHello => {
  if (!value || typeof value !== "object") {
    return false;
  }
  const record = value as { kind?: unknown; playerName?: unknown; party?: unknown };
  return (
    (record.kind === "battle" || record.kind === "trade") &&
    typeof record.playerName === "string" &&
    Array.isArray(record.party)
  );
};

type ControlRow = {
  label: string;
  description: string;
  keys: string[];
  tone?: ControlTone;
  layout?: "keys" | "dpad";
  extraKey?: string | null;
};

const DPAD_LAYOUT = [
  { key: "Up", area: "up" },
  { key: "Left", area: "left" },
  { key: "Right", area: "right" },
  { key: "Down", area: "down" },
];

const buildControlRows = (bindings: Record<GameButton, string[]>): ControlRow[] => [
  {
    label: "Move",
    description: "Navigate the world",
    keys: DPAD_LAYOUT.map((entry) => entry.key),
    layout: "dpad",
    extraKey: "Touch D-pad",
  },
  {
    label: "A",
    description: "Confirm / interact",
    keys: formatKeyLabels(bindings[GameButton.A] ?? []),
    tone: "accent",
  },
  {
    label: "B",
    description: "Cancel / back",
    keys: formatKeyLabels(bindings[GameButton.B] ?? []),
    tone: "ember",
  },
  {
    label: "Start",
    description: "Open menu",
    keys: formatKeyLabels(bindings[GameButton.Start] ?? []),
  },
  {
    label: "Select",
    description: "Utility actions",
    keys: formatKeyLabels(bindings[GameButton.Select] ?? []),
  },
];
const CONTROL_TIPS = [
  "Click or tap the canvas to focus keyboard input.",
  "Use the on-screen controls on touch devices.",
  "Audio starts once you press a key or tap the canvas.",
  "Gamepads (including Steam Deck) support D-pad/left stick, A/B, Start, and Select.",
];
const UTILITY_PANEL_OPTIONS: Array<{ view: UtilityPanelView; label: string; icon: IconDefinition }> = [
  { view: "multiplayer", label: "Lobby", icon: faUsers },
  { view: "settings", label: "Settings", icon: faGear },
  { view: "saves", label: "Saves", icon: faFloppyDisk },
  { view: "debug", label: "Debug", icon: faBug },
];
const DESKTOP_UTILITY_PANEL_OPTIONS = UTILITY_PANEL_OPTIONS.filter(
  (option) => option.view !== "multiplayer" && option.view !== "debug"
);
const MODAL_MAX_HEIGHT = "calc(min(100vh, 92vh) - 1.5rem)";
const MODAL_BODY_MAX_HEIGHT = "calc(100% - 5.5rem)";
const MODAL_DIALOG_STYLE = {
  maxHeight: MODAL_MAX_HEIGHT,
  height: MODAL_MAX_HEIGHT,
  overflow: "hidden" as const,
};
const PLAY_SESSION_STORAGE_KEY = "pokecrystal.play.session";
const PLAY_INTRO_STORAGE_KEY = "pokecrystal.play.playIntro";
const DESKTOP_SIDEBAR_VISIBLE_STORAGE_KEY = "pokecrystal.desktop.sidebarVisible";
const CORE_ASSET_PROGRESS_CAP = 0.75;
const CORE_DATA_PROGRESS_CAP = 0.95;
const LOADING_STALL_THRESHOLD_MS = 3500;
const LOADING_STALL_ANIMATION_MS = 24000;
const LOADING_STALL_PROGRESS_FLOOR = CORE_ASSET_PROGRESS_CAP;
const LOADING_STALL_PROGRESS_CEIL = CORE_DATA_PROGRESS_CAP - 0.01;
const MULTIPLAYER_SESSION_TIMEOUT_MS = 20_000;
const MULTIPLAYER_WORLD_ID = process.env.NEXT_PUBLIC_POKECRYSTAL_WORLD_ID?.trim() || "main";
const MULTIPLAYER_MODPACK_ID =
  process.env.NEXT_PUBLIC_POKECRYSTAL_MODPACK_ID?.trim() || "core-modular";

const readMultiplayerIceServers = (): RTCIceServer[] | undefined => {
  const raw = process.env.NEXT_PUBLIC_POKECRYSTAL_ICE_SERVERS?.trim();
  if (!raw) {
    return undefined;
  }
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) {
      throw new Error("expected a JSON array");
    }
    const servers = parsed.filter((entry): entry is RTCIceServer => {
      if (!entry || typeof entry !== "object") return false;
      const urls = (entry as { urls?: unknown }).urls;
      return typeof urls === "string" || (
        Array.isArray(urls) && urls.length > 0 && urls.every((url) => typeof url === "string")
      );
    });
    if (servers.length !== parsed.length || servers.length === 0) {
      throw new Error("every ICE server requires urls");
    }
    return servers;
  } catch (error) {
    console.error("[multiplayer] Invalid NEXT_PUBLIC_POKECRYSTAL_ICE_SERVERS", error);
    return undefined;
  }
};

const MULTIPLAYER_ICE_SERVERS = readMultiplayerIceServers();

const isSupabaseClientUnavailableError = (error: unknown): boolean =>
  error instanceof Error && error.message === "Supabase client not initialized";

const clampRatio = (value: number): number => {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.min(1, Math.max(0, value));
};

const formatLoadingDetail = (rawLabel?: string): string | null => {
  const normalized = String(rawLabel ?? "").trim();
  if (!normalized) {
    return null;
  }
  const withoutLeadingSlash = normalized.replace(/^[\\/]+/, "");
  const withoutAssetPrefix = withoutLeadingSlash.replace(/^assets[\\/]/i, "");
  const slashNormalized = withoutAssetPrefix.replace(/\\/g, "/");
  return slashNormalized.length > 0 ? `Loading ${slashNormalized}` : null;
};

const getLoadProgressRatio = (progress: GameLoadProgress | null): number => {
  if (!progress) {
    return 0;
  }
  if (progress.phase === "ready") {
    return 1;
  }
  const phaseRatio = clampRatio(progress.ratio);
  if (progress.phase === "core-data") {
    return CORE_ASSET_PROGRESS_CAP + (CORE_DATA_PROGRESS_CAP - CORE_ASSET_PROGRESS_CAP) * phaseRatio;
  }
  return CORE_ASSET_PROGRESS_CAP * phaseRatio;
};

const getStalledLoadingRatio = (stalledMs: number): number => {
  if (stalledMs <= LOADING_STALL_THRESHOLD_MS) {
    return LOADING_STALL_PROGRESS_FLOOR;
  }
  const elapsed = stalledMs - LOADING_STALL_THRESHOLD_MS;
  const ratio = clampRatio(elapsed / LOADING_STALL_ANIMATION_MS);
  return (
    LOADING_STALL_PROGRESS_FLOOR +
    (LOADING_STALL_PROGRESS_CEIL - LOADING_STALL_PROGRESS_FLOOR) * ratio
  );
};

const getDisplayLoadProgressRatio = (
  progress: GameLoadProgress | null,
  stalledMs: number
): number => {
  const baseRatio = getLoadProgressRatio(progress);
  if (!progress || progress.phase !== "core-assets" || progress.ratio < 1) {
    return baseRatio;
  }
  return Math.max(baseRatio, getStalledLoadingRatio(stalledMs));
};

const getLoadProgressLabel = (progress: GameLoadProgress | null): string => {
  if (!progress) {
    return "Preparing game startup";
  }
  if (progress.phase === "ready") {
    return "Startup complete";
  }
  if (progress.phase === "core-assets") {
    if (progress.ratio >= 1) {
      return "Initializing game systems";
    }
    return formatLoadingDetail(progress.label) ?? "Loading core assets";
  }
  if (progress.phase === "core-data") {
    return formatLoadingDetail(progress.label) ?? "Loading data assets";
  }
  return "Preparing game startup";
};

const getDisplayLoadProgressLabel = (
  progress: GameLoadProgress | null,
  stalledMs: number
): string => {
  const label = getLoadProgressLabel(progress);
  if (
    progress?.phase === "core-assets" &&
    progress.ratio >= 1 &&
    stalledMs > LOADING_STALL_THRESHOLD_MS
  ) {
    return "Initializing game systems (this can take a few more seconds)";
  }
  return label;
};

const getStoredPlayIntroEnabled = (): boolean => {
  if (typeof window === "undefined") {
    return true;
  }
  try {
    const stored = window.localStorage.getItem(PLAY_INTRO_STORAGE_KEY);
    if (stored === "true" || stored === "false") {
      return stored === "true";
    }
  } catch {
    // Ignore storage failures and fall back to default.
  }
  return true;
};

const getStoredDesktopSidebarVisible = (): boolean => {
  if (typeof window === "undefined") {
    return true;
  }
  try {
    const stored = window.localStorage.getItem(DESKTOP_SIDEBAR_VISIBLE_STORAGE_KEY);
    if (stored === "true" || stored === "false") {
      return stored === "true";
    }
  } catch {
    // Ignore storage failures and keep the desktop tools collapsed by default.
  }
  return false;
};

type SupabaseUserSettings = {
  user_id: string;
  player_name: string;
  player_gender: PlayerGender;
  time_of_day: TimeOfDay;
  sound_enabled: boolean;
  instant_mode_enabled: boolean;
  brand_theme: BrandTheme;
};

const normalisePlayerName = (value: string): string => {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : "Ryan";
};

const isPlayerGender = (value: unknown): value is PlayerGender =>
  value === PlayerGender.MALE || value === PlayerGender.FEMALE;

const isTimeOfDay = (value: unknown): value is TimeOfDay =>
  value === TimeOfDay.MORN || value === TimeOfDay.DAY || value === TimeOfDay.NIGHT;

const isBrandTheme = (value: unknown): value is BrandTheme =>
  isBrandThemeKey(value);

const parseSupabaseUserSettings = (value: unknown): SupabaseUserSettings | null => {
  if (!value || typeof value !== "object") {
    return null;
  }
  const candidate = value as Partial<SupabaseUserSettings>;
  if (
    typeof candidate.user_id !== "string" ||
    typeof candidate.player_name !== "string" ||
    !isPlayerGender(candidate.player_gender) ||
    !isTimeOfDay(candidate.time_of_day) ||
    typeof candidate.sound_enabled !== "boolean" ||
    typeof candidate.instant_mode_enabled !== "boolean" ||
    !isBrandTheme(candidate.brand_theme)
  ) {
    return null;
  }
  return {
    user_id: candidate.user_id,
    player_name: candidate.player_name,
    player_gender: candidate.player_gender,
    time_of_day: candidate.time_of_day,
    sound_enabled: candidate.sound_enabled,
    instant_mode_enabled: candidate.instant_mode_enabled,
    brand_theme: candidate.brand_theme,
  };
};

const getFullscreenElement = (doc: FullscreenCapableDocument): Element | null =>
  doc.fullscreenElement ?? doc.webkitFullscreenElement ?? null;

const canUseFullscreen = (doc: FullscreenCapableDocument): boolean => {
  if (typeof doc.fullscreenEnabled === "boolean") {
    return doc.fullscreenEnabled;
  }
  if (typeof doc.webkitFullscreenEnabled === "boolean") {
    return doc.webkitFullscreenEnabled;
  }
  const root = doc.documentElement as FullscreenCapableElement;
  return (
    typeof root.requestFullscreen === "function" ||
    typeof root.webkitRequestFullscreen === "function"
  );
};

const requestFullscreenForElement = async (element: FullscreenCapableElement): Promise<void> => {
  if (typeof element.requestFullscreen === "function") {
    await element.requestFullscreen();
    return;
  }
  if (typeof element.webkitRequestFullscreen === "function") {
    await element.webkitRequestFullscreen();
  }
};

const exitFullscreenForDocument = async (doc: FullscreenCapableDocument): Promise<void> => {
  if (typeof doc.exitFullscreen === "function") {
    await doc.exitFullscreen();
    return;
  }
  if (typeof doc.webkitExitFullscreen === "function") {
    await doc.webkitExitFullscreen();
  }
};

const ensureComponent = <T,>(component: T, name: string): T => {
  if (!component) {
    throw new Error(`[play-panel] ${name} component is undefined`);
  }
  return component;
};

export const PlayPanel = ({ variant = "default" }: PlayPanelProps) => {
  const isDesktopVariant = variant === "desktop";
  ensureComponent(GameCanvas, "GameCanvas");
  ensureComponent(VirtualGamepad, "VirtualGamepad");
  ensureComponent(GuestSavePanel, "GuestSavePanel");
  ensureComponent(SettingsPanel, "SettingsPanel");
  ensureComponent(KeybindingsEditor, "KeybindingsEditor");
  ensureComponent(MultiplayerMenu, "MultiplayerMenu");
  const postEventRef = useRef<((event: GameEngineEvent) => void) | null>(null);
  const gameRef = useRef<Game | null>(null);
  const mpConnectionRef = useRef<WebRTCConnection | null>(null);
  const activeSessionRef = useRef<ActiveMultiplayerSession | null>(null);
  const presenceManagerRef = useRef<OverworldPresenceManager | null>(null);
  const matchmakingServiceRef = useRef<MatchmakingService | null>(null);
  const handledMatchIdRef = useRef<string | null>(null);
  const presenceSyncTimerRef = useRef<number | null>(null);
  const remotePlayersHandlerRef = useRef<((players: RemoteOverworldPlayer[]) => void) | null>(null);
  const interactionRequestHandlerRef = useRef<
    ((request: MultiplayerInteractionRequest) => void) | null
  >(null);
  const interactionResponseHandlerRef = useRef<
    ((response: MultiplayerInteractionResponse) => void) | null
  >(null);
  const chatMessageHandlerRef = useRef<((message: MultiplayerChatMessage) => void) | null>(null);
  const frontendPlayerCountRef = useRef(0);
  const presenceAiCountRef = useRef(0);
  const apiMcpCountRef = useRef(0);
  const fullscreenContainerRef = useRef<HTMLDivElement | null>(null);
  const canvasContainerRef = useRef<HTMLDivElement | null>(null);
  const mountedRef = useRef(false);
  const [isCompactLayout, setIsCompactLayout] = useState(false);
  const [hasMounted, setHasMounted] = useState(false);
  // Playback runs locally; MCP is reserved for external clients.
  const secureMode = false;
  const [gameInstanceKey, setGameInstanceKey] = useState(0);
  const [playerGender, setPlayerGender] = useState<PlayerGender>(PlayerGender.MALE);
  const playerGenderRef = useRef<PlayerGender>(PlayerGender.MALE);
  const [playerName, setPlayerName] = useState<string>("Ryan");
  const playerNameRef = useRef("Ryan");
  const [timeOfDay, setTimeOfDay] = useState<TimeOfDay>(TimeOfDay.DAY);
  const [dayOfWeek, setDayOfWeek] = useState<number>(0);
  const [startToken, setStartToken] = useState<string | null>(null);
  const [loadingGame, setLoadingGame] = useState(false);
  const [loadProgress, setLoadProgress] = useState<GameLoadProgress | null>(null);
  const loadProgressUpdatedAtRef = useRef<number | null>(null);
  const [, setLoadingOverlayTick] = useState(0);
  const defaultSoundEnabled = isDesktopVariant;
  const [soundEnabled, setSoundEnabled] = useState<boolean>(defaultSoundEnabled);
  const soundEnabledRef = useRef(defaultSoundEnabled);
  const [instantModeEnabled, setInstantModeEnabled] = useState<boolean>(false);
  const [brandTheme, setBrandTheme] = useState<BrandTheme>("krabby");
  const [playIntroEnabled, setPlayIntroEnabled] = useState<boolean>(getStoredPlayIntroEnabled);
  const [keyboardButtons, setKeyboardButtons] = useState<string[]>([]);
  const [keyboardKeys, setKeyboardKeys] = useState<Array<string | number>>([]);
  const [virtualButtons, setVirtualButtons] = useState<string[]>([]);
  const [rendererMode, setRendererMode] = useState<RendererMode>("tile");
  const [rendererModeAuto, setRendererModeAuto] = useState<boolean>(false);
  const [showTouchControls, setShowTouchControls] = useState(true);
  const [controlsOpen, setControlsOpen] = useState(false);
  const [utilityPanelOpen, setUtilityPanelOpen] = useState(false);
  const [utilityPanelView, setUtilityPanelView] = useState<UtilityPanelView>("settings");
  const [desktopSidebarVisible, setDesktopSidebarVisible] = useState<boolean>(getStoredDesktopSidebarVisible);
  const [keyBindingsVersion, setKeyBindingsVersion] = useState(0);
  const [fullscreenAvailable, setFullscreenAvailable] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [viewportSize, setViewportSize] = useState({ width: 1280, height: 720 });
  const [incomingRequest, setIncomingRequest] = useState<MultiplayerInteractionRequest | null>(null);
  const [outgoingRequest, setOutgoingRequest] = useState<{
    requestId: string;
    targetUserId: string;
    targetName: string;
    kind: MultiplayerInteractionKind;
  } | null>(null);
  const [interactionStatus, setInteractionStatus] = useState<string | null>(null);
  const [chatMessages, setChatMessages] = useState<MultiplayerChatLine[]>([]);
  const [blockedChatUserIds, setBlockedChatUserIds] = useState<Set<string>>(() => new Set());
  const [remotePlayers, setRemotePlayers] = useState<RemoteOverworldPlayer[]>([]);
  const [selectedRemoteUserId, setSelectedRemoteUserId] = useState<string | null>(null);
  const [multiplayerLeaderboard, setMultiplayerLeaderboard] = useState<MultiplayerLeaderboardEntry[]>([]);
  // `playIntroEnabled` is persisted under a legacy key where `true` means the intro/title flow is active.
  // The UI setting is "Skip to play", so we expose the inverse to `SettingsPanel`.
  const skipToPlayEnabled = !playIntroEnabled;
  const effectiveSkipToPlayEnabled = isDesktopVariant ? false : skipToPlayEnabled;
  const shouldStartFromTitleScreen = !effectiveSkipToPlayEnabled;
  const shouldPlayIntro = !effectiveSkipToPlayEnabled && Boolean(startToken);
  const onlinePlayerCount = useMultiplayerStore((state) => state.onlinePlayerCount);
  const onlineAiCount = useMultiplayerStore((state) => state.onlineAiCount);
  const remoteSpritesVisible = useMultiplayerStore((state) => state.remoteSpritesVisible);
  const crowdViewEnabled = useMultiplayerStore((state) => state.crowdViewEnabled);
  const matchmakingQueueMode = useMultiplayerStore((state) => state.queueMode);
  const currentMatchId = useMultiplayerStore((state) => state.currentMatchId);
  const currentMatchChannelName = useMultiplayerStore((state) => state.currentMatchChannelName);
  const currentMatchMode = useMultiplayerStore((state) => state.currentMode);
  const currentOpponentId = useMultiplayerStore((state) => state.opponentId);
  const currentOpponentName = useMultiplayerStore((state) => state.opponentName);
  const currentMatchIsHost = useMultiplayerStore((state) => state.isHost);
  const latestRemotePlayersRef = useRef<RemoteOverworldPlayer[]>([]);
  const supabaseClientRef = useRef(createSupabaseBrowserClient());
  const hydratedSupabaseSettingsRef = useRef(false);
  const [supabaseUserId, setSupabaseUserId] = useState<string | null>(null);
  const applyDesktopSidebarVisible = useCallback((visible: boolean) => {
    setDesktopSidebarVisible(visible);
    if (typeof window === "undefined") {
      return;
    }
    try {
      window.localStorage.setItem(DESKTOP_SIDEBAR_VISIBLE_STORAGE_KEY, String(visible));
    } catch {
      // Ignore storage failures; the in-memory state is enough for this session.
    }
  }, []);
  const refreshOnlineCounts = useCallback(() => {
    const mp = useMultiplayerStore.getState();
    const frontendCount = Math.max(0, Math.trunc(frontendPlayerCountRef.current));
    const apiCount = Math.max(
      0,
      Math.trunc(presenceAiCountRef.current + apiMcpCountRef.current)
    );
    mp.setOnlineCounts(frontendCount, apiCount);
  }, []);
  const syncTimeOfDayFromStorage = useCallback(() => {
    if (typeof window === "undefined") {
      return;
    }
    const storedTheme = window.localStorage.getItem(THEME_STORAGE_KEY);
    if (isThemeKey(storedTheme)) {
      setTimeOfDay(
        storedTheme === "morning"
          ? TimeOfDay.MORN
          : storedTheme === "night"
            ? TimeOfDay.NIGHT
            : TimeOfDay.DAY
      );
      return;
    }
    setTimeOfDay(getLocalTimeOfDay());
  }, []);
  useEffect(() => {
    mountedRef.current = true;
    setHasMounted(true);
    return () => {
      mountedRef.current = false;
      activeSessionRef.current = null;
      mpConnectionRef.current?.destroy();
      mpConnectionRef.current = null;
      gameRef.current?.clearMultiplayerBattleTransport();
      gameRef.current?.onMultiplayerBattleComplete(null);
      gameRef.current?.clearOverworldRemotePlayers();
      frontendPlayerCountRef.current = 0;
      presenceAiCountRef.current = 0;
      setRemotePlayers([]);
      setSelectedRemoteUserId(null);
      if (presenceSyncTimerRef.current !== null) {
        window.clearInterval(presenceSyncTimerRef.current);
        presenceSyncTimerRef.current = null;
      }
      const manager = presenceManagerRef.current;
      if (manager && remotePlayersHandlerRef.current) {
        manager.offRemotePlayersChange(remotePlayersHandlerRef.current);
      }
      if (manager && interactionRequestHandlerRef.current) {
        manager.offInteractionRequest(interactionRequestHandlerRef.current);
      }
      if (manager && interactionResponseHandlerRef.current) {
        manager.offInteractionResponse(interactionResponseHandlerRef.current);
      }
      if (manager && chatMessageHandlerRef.current) {
        manager.offChatMessage(chatMessageHandlerRef.current);
      }
      void manager?.disconnect();
      presenceManagerRef.current = null;
      remotePlayersHandlerRef.current = null;
      interactionRequestHandlerRef.current = null;
      interactionResponseHandlerRef.current = null;
      chatMessageHandlerRef.current = null;
      const matchmaking = matchmakingServiceRef.current;
      matchmakingServiceRef.current = null;
      if (matchmaking) {
        void matchmaking.leaveQueue().catch(() => undefined).finally(() => matchmaking.destroy());
      }
    };
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    const media = window.matchMedia("(max-width: 599.95px)");
    const sync = () => setIsCompactLayout(media.matches);
    sync();
    if (typeof media.addEventListener === "function") {
      media.addEventListener("change", sync);
      return () => media.removeEventListener("change", sync);
    }
    media.addListener(sync);
    return () => media.removeListener(sync);
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    const eventName = getKeyBindingsChangeEventName();
    const handleConfigUpdate = () => setKeyBindingsVersion((value) => value + 1);
    const handleThemeUpdate = () => {
      handleConfigUpdate();
      syncTimeOfDayFromStorage();
    };
    syncTimeOfDayFromStorage();
    window.addEventListener(eventName, handleConfigUpdate);
    window.addEventListener("storage", handleThemeUpdate);
    window.addEventListener(THEME_CHANGED_EVENT, handleThemeUpdate);
    return () => {
      window.removeEventListener(eventName, handleConfigUpdate);
      window.removeEventListener("storage", handleThemeUpdate);
      window.removeEventListener(THEME_CHANGED_EVENT, handleThemeUpdate);
    };
  }, [syncTimeOfDayFromStorage]);

  const isCompactLayoutReady = hasMounted ? isCompactLayout : false;
  const isMobileEmulator = isCompactLayoutReady && !isFullscreen;
  const startOverlayPrompt = isCompactLayoutReady
    ? "Start your session to play."
    : "Start your session, then join multiplayer to battle or trade.";

  const handleInputStateChange = useCallback((state: InputState) => {
    if (!mountedRef.current) {
      return;
    }
    setKeyboardButtons((prev) =>
      areArraysEqual(prev, state.pressedButtons) ? prev : state.pressedButtons
    );
    setKeyboardKeys((prev) => (areArraysEqual(prev, state.pressedKeys) ? prev : state.pressedKeys));
  }, []);

  const handlePostEventReady = useCallback((postEvent: ((event: GameEngineEvent) => void) | null) => {
    postEventRef.current = postEvent;
  }, []);

  const handleVirtualButtonsChange = useCallback((buttons: string[]) => {
    if (!mountedRef.current) {
      return;
    }
    setVirtualButtons((prev) => (areArraysEqual(prev, buttons) ? prev : buttons));
  }, []);

  const handleGameReady = useCallback((game: Game | null) => {
    if (!mountedRef.current) {
      return;
    }
    gameRef.current = game;
    setLoadingGame(false);
    loadProgressUpdatedAtRef.current = Date.now();
    if (game) {
      setLoadProgress((prev) =>
        prev
          ? {
              ...prev,
              phase: "ready",
              completed: prev.total > 0 ? prev.total : 1,
              total: prev.total > 0 ? prev.total : 1,
              ratio: 1,
            }
          : {
              phase: "ready",
              completed: 1,
              total: 1,
              ratio: 1,
            }
      );
    }
    if (!game) {
      return;
    }
    if (secureMode) {
      return;
    }
    game.setOverworldRemoteRenderEnabled(remoteSpritesVisible);
    game.setOverworldRemoteCrowdView(crowdViewEnabled);
    game.setOverworldRemotePlayers(
      remoteSpritesVisible ? latestRemotePlayersRef.current : []
    );
    game.setAudioMuted(!soundEnabledRef.current);
    if (isDesktopVariant) {
      game.setPlayerGender(playerGenderRef.current);
      game.setPlayerName(playerNameRef.current);
      game.setTimeOfDay(timeOfDay);
      game.setDayOfWeek(dayOfWeek);
      game.getGameState().wram.instant_mode = instantModeEnabled;
      return;
    }
    const rawGender = game.getGameState().sram.player_gender;
    const nextGender = rawGender === PlayerGender.FEMALE ? PlayerGender.FEMALE : PlayerGender.MALE;
    playerGenderRef.current = nextGender;
    setPlayerGender(nextGender);
    const rawName = String(game.getGameState().sram.player_name ?? "").trim();
    const nextName = rawName.length > 0 ? rawName : "Ryan";
    playerNameRef.current = nextName;
    setPlayerName(nextName);
    game.setPlayerName(nextName);
    const rawTime = String(game.getGameState().wram.time_of_day ?? "day");
    setTimeOfDay(canonicaliseTimeOfDay(rawTime));
    const rawDay = Number(game.getGameState().sram.day_of_week ?? 0);
    const normalizedDay = ((rawDay % 7) + 7) % 7;
    setDayOfWeek(normalizedDay);
    setInstantModeEnabled(Boolean(game.getGameState().wram.instant_mode));
  }, [secureMode, remoteSpritesVisible, crowdViewEnabled, isDesktopVariant, timeOfDay, dayOfWeek, instantModeEnabled]);

  const clearActiveSession = useCallback((nextStatus?: string | null) => {
    const activeSession = activeSessionRef.current;
    activeSessionRef.current = null;
    mpConnectionRef.current = null;
    if (activeSession?.timeoutId !== null && activeSession?.timeoutId !== undefined) {
      window.clearTimeout(activeSession.timeoutId);
    }
    gameRef.current?.onMultiplayerBattleComplete(null);
    gameRef.current?.clearMultiplayerBattleTransport();
    activeSession?.connection.destroy();
    handledMatchIdRef.current = null;
    useMultiplayerStore.getState().clearMatch();
    if (nextStatus !== undefined) {
      setInteractionStatus(nextStatus);
    }
  }, []);

  const persistMultiplayerMatch = useCallback(
    async (
      session: ActiveMultiplayerSession,
      outcome: "local" | "remote" | "draw" | "cancelled",
      metadata: Record<string, unknown> = {}
    ): Promise<void> => {
      try {
        const response = await fetch("/api/multiplayer/matches", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            action: "complete",
            channelName: session.requestId,
            peerUserId: session.peerUserId,
            mode: session.kind,
            modpackId: MULTIPLAYER_MODPACK_ID,
            outcome,
            metadata,
          }),
        });
        if (!response.ok) {
          const body = (await response.json().catch(() => null)) as { error?: string } | null;
          throw new Error(body?.error ?? `Match persistence failed (${response.status})`);
        }
      } catch (error) {
        console.warn("[play-panel] failed to persist multiplayer match", error);
        setInteractionStatus((previous) =>
          previous ? `${previous} Ranked result could not be saved.` : "Ranked result could not be saved."
        );
      }
    },
    []
  );

  const beginMultiplayerSession = useCallback((
    kind: MultiplayerInteractionKind,
    requestId: string,
    isHost: boolean,
    peerUserId: string,
    peerName: string
  ) => {
    const game = gameRef.current;
    if (!game) {
      setInteractionStatus(`Game is not ready for a ${kind} session.`);
      return;
    }

    if (useMultiplayerStore.getState().inQueue) {
      void matchmakingServiceRef.current?.leaveQueue().catch((error) => {
        console.warn("[play-panel] failed to leave queue before starting session", error);
      });
    }
    clearActiveSession();

    const connection = new WebRTCConnection({
      matchId: requestId,
      isHost,
      iceServers: MULTIPLAYER_ICE_SERVERS,
    });
    const session: ActiveMultiplayerSession = {
      requestId,
      kind,
      isHost,
      peerUserId,
      peerName,
      connection,
      timeoutId: null,
      connected: false,
      started: false,
      helloSent: false,
      remoteHello: null,
    };
    activeSessionRef.current = session;
    mpConnectionRef.current = connection;
    setInteractionStatus(`Connecting ${kind} session with ${peerName}...`);

    session.timeoutId = window.setTimeout(() => {
      if (activeSessionRef.current === session && !session.started) {
        clearActiveSession(`${kind} session with ${peerName} timed out.`);
      }
    }, MULTIPLAYER_SESSION_TIMEOUT_MS);

    const maybeStartSession = () => {
      if (activeSessionRef.current !== session || session.started || !session.connected || !session.remoteHello) {
        return;
      }

      const activeGame = gameRef.current;
      if (!activeGame) {
        clearActiveSession(`${kind} session ended before the game was ready.`);
        return;
      }

      if (kind === "battle") {
        try {
          activeGame.setMultiplayerBattleTransport(new WebRTCBattleTransport(connection), { isHost });
          activeGame.onMultiplayerBattleComplete((result: MultiplayerBattleCompleteResult) => {
            if (activeSessionRef.current !== session) {
              return;
            }
            const outcome =
              result.result === 0 ? "local" : result.result === 1 ? "remote" : "cancelled";
            void persistMultiplayerMatch(session, outcome, { battleResult: result.result }).finally(() => {
              if (activeSessionRef.current === session) {
                clearActiveSession(`Battle with ${session.remoteHello?.playerName ?? peerName} finished.`);
              }
            });
          });
          activeGame.startMultiplayerBattle(session.remoteHello.party);
          session.started = true;
          if (session.timeoutId !== null) {
            window.clearTimeout(session.timeoutId);
            session.timeoutId = null;
          }
          setInteractionStatus(`Started battle with ${session.remoteHello.playerName}.`);
        } catch (error) {
          clearActiveSession(
            error instanceof Error ? error.message : "Failed to start multiplayer battle."
          );
        }
        return;
      }

      const tradeOffer = activeGame.getFirstPartyPokemon();
      if (!tradeOffer) {
        clearActiveSession("You need at least one Pokemon in your party to trade.");
        return;
      }

      session.started = true;
      if (session.timeoutId !== null) {
        window.clearTimeout(session.timeoutId);
        session.timeoutId = null;
      }
      setInteractionStatus(`Trading with ${session.remoteHello.playerName}...`);
      const link = new LinkCableEmulator(connection, isHost);
      const manager = new TradeManager(link, { isHost });
      void manager
        .trade(tradeOffer.pokemon, { confirm: true })
        .then((result) => {
          if (activeSessionRef.current !== session) {
            return;
          }
          if (result.cancelled) {
            void persistMultiplayerMatch(session, "cancelled", { trade: "cancelled" });
            clearActiveSession(`Trade with ${session.remoteHello?.playerName ?? peerName} was cancelled.`);
            return;
          }
          activeGame.replacePartyPokemon(tradeOffer.index, result.receivedPokemon);
          void persistMultiplayerMatch(session, "draw", { trade: "completed" }).finally(() => {
            clearActiveSession(
              `Trade complete with ${session.remoteHello?.playerName ?? peerName}.`
            );
          });
        })
        .catch((error) => {
          clearActiveSession(
            error instanceof Error ? error.message : "Trade failed."
          );
        });
    };

    const sendHello = () => {
      if (activeSessionRef.current !== session || session.helloSent) {
        return;
      }
      const activeGame = gameRef.current;
      if (!activeGame) {
        clearActiveSession(`${kind} session ended before the game was ready.`);
        return;
      }
      connection.send({
        type: "session:hello",
        data: {
          kind,
          playerName: String(activeGame.getGameState().sram.player_name ?? "").trim() || playerName,
          party: activeGame.getPartyPokemon(),
        },
      });
      session.helloSent = true;
      maybeStartSession();
    };

    connection.onData((message) => {
      if (activeSessionRef.current !== session || message.type !== "session:hello") {
        return;
      }
      if (!isMultiplayerSessionHello(message.data) || message.data.kind !== kind) {
        clearActiveSession(`Received invalid ${kind} session payload.`);
        return;
      }
      session.remoteHello = message.data;
      maybeStartSession();
    });

    connection.onStatus({
      onConnect: () => {
        if (activeSessionRef.current !== session) {
          return;
        }
        session.connected = true;
        sendHello();
        maybeStartSession();
      },
      onDisconnect: () => {
        if (activeSessionRef.current !== session) {
          return;
        }
        clearActiveSession(`${kind} session with ${peerName} disconnected.`);
      },
      onError: (error) => {
        if (activeSessionRef.current !== session) {
          return;
        }
        clearActiveSession(error.message || `Failed to connect ${kind} session.`);
      },
    });
  }, [clearActiveSession, persistMultiplayerMatch, playerName]);

  const handleJoinMatchmaking = useCallback(async (mode: MatchmakingMode) => {
    const store = useMultiplayerStore.getState();
    if (!supabaseUserId) {
      store.setError("Sign in to use matchmaking.");
      return;
    }
    try {
      const service = matchmakingServiceRef.current ?? new MatchmakingService();
      matchmakingServiceRef.current = service;
      if (store.inQueue) {
        await service.leaveQueue();
      }
      const partyPreview = gameRef.current?.getPartyPokemon().map((pokemon) => ({
        species: pokemonSpeciesDisplayName(pokemon.species),
        level: pokemon.level,
      }));
      const rating = multiplayerLeaderboard.find((entry) => entry.id === supabaseUserId)
        ?.link_battle_rating ?? undefined;
      await service.joinQueue({
        mode,
        modpackId: MULTIPLAYER_MODPACK_ID,
        rating: rating ?? undefined,
        partyPreview,
      });
      setInteractionStatus(`Searching the ${mode} queue for a compatible trainer...`);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Failed to join matchmaking.";
      store.setError(message);
      setInteractionStatus(message);
    }
  }, [multiplayerLeaderboard, supabaseUserId]);

  const handleLeaveMatchmaking = useCallback(async () => {
    try {
      await matchmakingServiceRef.current?.leaveQueue();
      setInteractionStatus("Left the matchmaking queue.");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Failed to leave matchmaking.";
      useMultiplayerStore.getState().setError(message);
    }
  }, []);

  useEffect(() => {
    if (
      !currentMatchId
      || !currentMatchChannelName
      || handledMatchIdRef.current === currentMatchId
      || !currentOpponentId
      || !currentOpponentName
      || (currentMatchMode !== "battle" && currentMatchMode !== "trade")
      || !gameRef.current
    ) {
      return;
    }
    handledMatchIdRef.current = currentMatchId;
    setInteractionStatus(`Match found: ${currentOpponentName}. Connecting...`);
    beginMultiplayerSession(
      currentMatchMode,
      currentMatchChannelName,
      currentMatchIsHost,
      currentOpponentId,
      currentOpponentName,
    );
  }, [
    beginMultiplayerSession,
    currentMatchId,
    currentMatchChannelName,
    currentMatchIsHost,
    currentMatchMode,
    currentOpponentId,
    currentOpponentName,
    loadingGame,
  ]);

  const handleLoadProgress = useCallback((progress: GameLoadProgress) => {
    if (!mountedRef.current) {
      return;
    }
    setLoadProgress(progress);
    loadProgressUpdatedAtRef.current = Date.now();
    if (progress.phase === "ready") {
      setLoadingGame(false);
      return;
    }
    if (startToken) {
      setLoadingGame(true);
    }
  }, [startToken]);

  const pressedButtons = useMemo(() => {
    return Array.from(new Set([...keyboardButtons, ...virtualButtons]));
  }, [keyboardButtons, virtualButtons]);

  const sendEvent = useCallback((event: GameEngineEvent) => {
    postEventRef.current?.(event);
  }, []);

  const handleLoadSave = useCallback(() => {
    if (secureMode && typeof window !== "undefined") {
      try {
        window.localStorage.removeItem(PLAY_SESSION_STORAGE_KEY);
      } catch {
        // Ignore storage failures.
      }
    }
    setGameInstanceKey((value) => value + 1);
    if (startToken) {
      setLoadingGame(true);
      setLoadProgress(null);
      loadProgressUpdatedAtRef.current = Date.now();
    }
  }, [secureMode, startToken]);

  const requestStart = useCallback((event?: React.SyntheticEvent | Event) => {
    if (secureMode) {
      return;
    }
    const allowUntrustedStart =
      typeof globalThis !== "undefined" &&
      Boolean(
        (globalThis as { __POKECRYSTAL_ALLOW_UNTRUSTED_START__?: boolean })
          .__POKECRYSTAL_ALLOW_UNTRUSTED_START__
      );
    // Removed isTrusted check as it was blocking legitimate user interaction in some environments
    const token =
      typeof crypto !== "undefined" && "randomUUID" in crypto
        ? crypto.randomUUID()
        : `start-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
    setStartToken(token);
    setLoadingGame(true);
    setLoadProgress(null);
    loadProgressUpdatedAtRef.current = Date.now();
  }, [secureMode]);

  useEffect(() => {
    if (secureMode || startToken) {
      return;
    }
    if (!isDesktopVariant && shouldStartFromTitleScreen) {
      return;
    }
    requestStart();
  }, [requestStart, secureMode, isDesktopVariant, shouldStartFromTitleScreen, startToken]);

  const applyGender = useCallback((value: PlayerGender) => {
    playerGenderRef.current = value;
    setPlayerGender(value);
    gameRef.current?.setPlayerGender(value);
  }, []);

  const applyName = useCallback((value: string) => {
    playerNameRef.current = value;
    setPlayerName(value);
    gameRef.current?.setPlayerName(value);
  }, []);

  const applyTimeOfDay = useCallback((value: TimeOfDay) => {
    setTimeOfDay(value);
    gameRef.current?.setTimeOfDay(value);
  }, []);

  const applyDayOfWeek = useCallback((value: number) => {
    setDayOfWeek(value);
    gameRef.current?.setDayOfWeek(value);
  }, []);

  const applySoundEnabled = useCallback((enabled: boolean) => {
    soundEnabledRef.current = enabled;
    setSoundEnabled(enabled);
    gameRef.current?.setAudioMuted(!enabled);
  }, []);

  const applyInstantModeEnabled = useCallback((enabled: boolean) => {
    setInstantModeEnabled(enabled);
    const game = gameRef.current;
    if (game) {
      game.getGameState().wram.instant_mode = enabled;
    }
  }, []);

  const applyBrandTheme = useCallback((value: BrandTheme) => {
    setBrandTheme(value);
  }, []);

  const applySkipToPlayEnabled = useCallback((enabled: boolean) => {
    setPlayIntroEnabled(!enabled);
    if (typeof window === "undefined") {
      return;
    }
    try {
      window.localStorage.setItem(PLAY_INTRO_STORAGE_KEY, String(!enabled));
    } catch {
      // ignore
    }
  }, []);

  const toggleRendererMode = useCallback(() => {
    setRendererModeAuto(false);
    setRendererMode((mode) => getNextRendererMode(mode));
  }, []);

  const handleMultiplayerConnect = useCallback(() => {
    const mp = useMultiplayerStore.getState();
    console.log("[play-panel] connecting to multiplayer server");
    if (!supabaseClientRef.current || !supabaseUserId) {
      mp.setConnectionState("error");
      mp.setError("Sign in to use multiplayer.");
      setInteractionStatus("Sign in to use multiplayer.");
      return;
    }
    mp.setConnectionState("connecting");
    mp.setError(null);

    const syncPresence = async (manager: OverworldPresenceManager): Promise<void> => {
      const game = gameRef.current;
      const overworld = game?.getOverworld?.();
      if (!game || !overworld) {
        return;
      }
      const mapName = game.getCurrentMapName();
      await manager.updateLocalState({
        playerName,
        mapName,
        tileX: overworld.player_x ?? 0,
        tileY: overworld.player_y ?? 0,
        direction: toPresenceDirection(overworld.player_direction),
      });
    };

    void (async () => {
      try {
        const manager = new OverworldPresenceManager({
          worldId: MULTIPLAYER_WORLD_ID,
          modpackId: MULTIPLAYER_MODPACK_ID,
        });
        const initialGame = gameRef.current;
        const initialOverworld = initialGame?.getOverworld?.();
        const initialMapName = initialGame ? initialGame.getCurrentMapName() : "Unknown";
        await manager.connect({
          playerName,
          entityType: "player",
          mapName: initialMapName,
          tileX: initialOverworld?.player_x ?? 0,
          tileY: initialOverworld?.player_y ?? 0,
          direction: toPresenceDirection(initialOverworld?.player_direction),
        });
        const handler = (players: RemoteOverworldPlayer[]) => {
          latestRemotePlayersRef.current = players;
          let playerCount = 0;
          let aiCount = 0;
          for (const participant of players) {
            if (participant.entityType === "ai") {
              aiCount += 1;
            } else {
              playerCount += 1;
            }
          }
          frontendPlayerCountRef.current = playerCount + 1;
          presenceAiCountRef.current = aiCount;
          const frontendPlayers = players.filter((participant) => participant.entityType === "player");
          setRemotePlayers(frontendPlayers);
          setSelectedRemoteUserId((current) => {
            if (current && frontendPlayers.some((participant) => participant.userId === current)) {
              return current;
            }
            return frontendPlayers[0]?.userId ?? null;
          });
          refreshOnlineCounts();
          const state = useMultiplayerStore.getState();
          gameRef.current?.setOverworldRemotePlayers(state.remoteSpritesVisible ? players : []);
        };
        const requestHandler = (request: MultiplayerInteractionRequest) => {
          setIncomingRequest(request);
          setInteractionStatus(`${request.fromPlayerName} sent a ${request.kind} request.`);
        };
        const responseHandler = (response: MultiplayerInteractionResponse) => {
          setOutgoingRequest((pending) => {
            if (!pending || pending.requestId !== response.requestId) {
              return pending;
            }
            const verdict = response.accepted ? "accepted" : "declined";
            setInteractionStatus(`${pending.targetName} ${verdict} your ${pending.kind} request.`);
            if (response.accepted) {
              beginMultiplayerSession(
                pending.kind,
                pending.requestId,
                true,
                pending.targetUserId,
                pending.targetName
              );
            }
            return null;
          });
        };
        const chatHandler = (message: MultiplayerChatMessage) => {
          setChatMessages((current) => [
            ...current.slice(-99),
            {
              messageId: message.messageId,
              userId: message.fromUserId,
              playerName: message.fromPlayerName,
              text: message.text,
              outgoing: false,
              channel: message.channel,
              timestampMs: message.timestampMs,
            },
          ]);
          if (message.channel === "whisper") {
            setSelectedRemoteUserId(message.fromUserId);
          }
        };
        manager.onRemotePlayersChange(handler);
        manager.onInteractionRequest(requestHandler);
        manager.onInteractionResponse(responseHandler);
        manager.onChatMessage(chatHandler);
        remotePlayersHandlerRef.current = handler;
        interactionRequestHandlerRef.current = requestHandler;
        interactionResponseHandlerRef.current = responseHandler;
        chatMessageHandlerRef.current = chatHandler;
        presenceManagerRef.current = manager;
        setIncomingRequest(null);
        setOutgoingRequest(null);
        await syncPresence(manager);
        if (presenceSyncTimerRef.current !== null) {
          window.clearInterval(presenceSyncTimerRef.current);
        }
        presenceSyncTimerRef.current = window.setInterval(() => {
          void syncPresence(manager);
        }, 200);
        mp.setConnectionState("connected");
        setInteractionStatus("Joined the live multiplayer world.");
        refreshOnlineCounts();
      } catch (error) {
        if (isSupabaseClientUnavailableError(error)) {
          console.warn("[play-panel] multiplayer unavailable: Supabase client not initialized");
          mp.setError("Multiplayer is unavailable in this environment.");
        } else {
          console.error("[play-panel] multiplayer connection failed", error);
          mp.setError(error instanceof Error ? error.message : "Failed to connect to multiplayer");
        }
        mp.setConnectionState("error");
      }
    })();
  }, [beginMultiplayerSession, playerName, refreshOnlineCounts, supabaseUserId]);

  const handleMultiplayerDisconnect = useCallback(() => {
    const mp = useMultiplayerStore.getState();
    console.log("[play-panel] disconnecting from multiplayer server");

    // Clean up connections
    activeSessionRef.current = null;
    mpConnectionRef.current?.destroy();
    mpConnectionRef.current = null;
    gameRef.current?.onMultiplayerBattleComplete(null);
    gameRef.current?.clearMultiplayerBattleTransport();
    gameRef.current?.clearOverworldRemotePlayers();
    latestRemotePlayersRef.current = [];
    setRemotePlayers([]);
    setSelectedRemoteUserId(null);
    frontendPlayerCountRef.current = 0;
    presenceAiCountRef.current = 0;
    if (presenceSyncTimerRef.current !== null) {
      window.clearInterval(presenceSyncTimerRef.current);
      presenceSyncTimerRef.current = null;
    }
    const manager = presenceManagerRef.current;
    if (manager && remotePlayersHandlerRef.current) {
      manager.offRemotePlayersChange(remotePlayersHandlerRef.current);
    }
    if (manager && interactionRequestHandlerRef.current) {
      manager.offInteractionRequest(interactionRequestHandlerRef.current);
    }
    if (manager && interactionResponseHandlerRef.current) {
      manager.offInteractionResponse(interactionResponseHandlerRef.current);
    }
    if (manager && chatMessageHandlerRef.current) {
      manager.offChatMessage(chatMessageHandlerRef.current);
    }
    void manager?.disconnect();
    presenceManagerRef.current = null;
    remotePlayersHandlerRef.current = null;
    interactionRequestHandlerRef.current = null;
    interactionResponseHandlerRef.current = null;
    chatMessageHandlerRef.current = null;
    setIncomingRequest(null);
    setOutgoingRequest(null);
    setChatMessages([]);
    setInteractionStatus(null);

    mp.setConnectionState("disconnected");
    mp.setError(null);
    refreshOnlineCounts();
  }, [refreshOnlineCounts]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    const media = window.matchMedia("(max-width: 960px), (pointer: coarse)");
    const syncTouchControls = () => setShowTouchControls(media.matches);
    syncTouchControls();
    if (typeof media.addEventListener === "function") {
      media.addEventListener("change", syncTouchControls);
      return () => media.removeEventListener("change", syncTouchControls);
    }
    media.addListener(syncTouchControls);
    return () => media.removeListener(syncTouchControls);
  }, []);

  const handleToggleRemoteSprites = useCallback(() => {
    const mp = useMultiplayerStore.getState();
    if (!mp.remoteSpritesVisible && mp.crowdViewEnabled) {
      mp.setCrowdViewEnabled(false);
    }
    mp.setRemoteSpritesVisible(!mp.remoteSpritesVisible);
  }, []);

  const handleToggleCrowdView = useCallback(() => {
    const mp = useMultiplayerStore.getState();
    if (mp.connectionState !== "connected") {
      if (mp.connectionState !== "connecting") {
        handleMultiplayerConnect();
      }
      mp.setRemoteSpritesVisible(true);
      mp.setCrowdViewEnabled(true);
      return;
    }
    if (!mp.remoteSpritesVisible) {
      mp.setRemoteSpritesVisible(true);
    }
    mp.setCrowdViewEnabled(!mp.crowdViewEnabled);
  }, [handleMultiplayerConnect]);

  const sendInteractionRequest = useCallback((kind: MultiplayerInteractionKind) => {
    const manager = presenceManagerRef.current;
    if (!manager) {
      setInteractionStatus("Connect to multiplayer first.");
      return;
    }
    const eligiblePlayers = latestRemotePlayersRef.current.filter((player) => player.entityType === "player");
    const target =
      eligiblePlayers.find((player) => player.userId === selectedRemoteUserId) ??
      eligiblePlayers[0] ??
      null;
    if (!target) {
      setInteractionStatus("No online frontend player is available for requests.");
      return;
    }
    void manager
      .sendInteractionRequest(target.userId, kind)
      .then((requestId) => {
        setOutgoingRequest({ requestId, targetUserId: target.userId, targetName: target.playerName, kind });
        setInteractionStatus(`Sent ${kind} request to ${target.playerName}.`);
      })
      .catch((error) => {
        setInteractionStatus(
          error instanceof Error ? error.message : `Failed to send ${kind} request.`
        );
      });
  }, [selectedRemoteUserId]);

  const handleRequestBattle = useCallback(() => {
    sendInteractionRequest("battle");
  }, [sendInteractionRequest]);

  const handleRequestTrade = useCallback(() => {
    sendInteractionRequest("trade");
  }, [sendInteractionRequest]);

  const handleSendChat = useCallback((channel: MultiplayerChatChannel, text: string) => {
    const manager = presenceManagerRef.current;
    const eligiblePlayers = latestRemotePlayersRef.current.filter((player) => player.entityType === "player");
    const target = eligiblePlayers.find((player) => player.userId === selectedRemoteUserId)
      ?? eligiblePlayers[0]
      ?? null;
    if (!manager || (channel === "whisper" && !target)) {
      setInteractionStatus(channel === "whisper"
        ? "Select an online trainer before whispering."
        : "Connect to multiplayer before sending a message.");
      return;
    }
    void manager.sendChatMessage(channel, text, channel === "whisper" ? target?.userId ?? null : null).then((message) => {
      setChatMessages((current) => [
        ...current.slice(-99),
        {
          messageId: message.messageId,
          userId: supabaseUserId ?? "local",
          playerName,
          text: message.text,
          outgoing: true,
          channel: message.channel,
          timestampMs: message.timestampMs,
        },
      ]);
    }).catch((error) => {
      setInteractionStatus(error instanceof Error ? error.message : "Failed to send message.");
    });
  }, [playerName, selectedRemoteUserId, supabaseUserId]);

  const handleToggleChatBlock = useCallback((userId: string) => {
    setBlockedChatUserIds((current) => {
      const next = new Set(current);
      if (next.has(userId)) {
        next.delete(userId);
      } else {
        next.add(userId);
      }
      presenceManagerRef.current?.setBlockedUserIds(next);
      return next;
    });
  }, []);

  const handleReportChat = useCallback((message: MultiplayerChatLine) => {
    void fetch("/api/multiplayer/chat/reports", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        messageId: message.messageId,
        reportedUserId: message.userId,
        playerName: message.playerName,
        channel: message.channel,
        text: message.text,
      }),
    }).then(async (response) => {
      if (!response.ok) {
        const body = await response.json().catch(() => null) as { error?: string } | null;
        throw new Error(body?.error ?? "Failed to report message.");
      }
      setInteractionStatus(`Reported ${message.playerName}'s message to moderators.`);
    }).catch((error) => {
      setInteractionStatus(error instanceof Error ? error.message : "Failed to report message.");
    });
  }, []);

  const respondToIncomingRequest = useCallback((accepted: boolean) => {
    const manager = presenceManagerRef.current;
    const request = incomingRequest;
    if (!manager || !request) {
      return;
    }
    void manager
      .sendInteractionResponse(request, accepted)
      .then(() => {
        setInteractionStatus(
          accepted
            ? `Accepted ${request.fromPlayerName}'s ${request.kind} request.`
            : `Declined ${request.fromPlayerName}'s ${request.kind} request.`
        );
        setIncomingRequest(null);
        if (accepted) {
          beginMultiplayerSession(
            request.kind,
            request.requestId,
            false,
            request.fromUserId,
            request.fromPlayerName
          );
        }
      })
      .catch((error) => {
        setInteractionStatus(
          error instanceof Error ? error.message : "Failed to respond to request."
        );
      });
  }, [beginMultiplayerSession, incomingRequest]);

  const handleAcceptRequest = useCallback(() => {
    respondToIncomingRequest(true);
  }, [respondToIncomingRequest]);

  const handleDeclineRequest = useCallback(() => {
    respondToIncomingRequest(false);
  }, [respondToIncomingRequest]);

  const toggleFullscreen = useCallback(async () => {
    if (typeof document === "undefined") {
      return;
    }
    const fullscreenDoc = document as FullscreenCapableDocument;
    const container = fullscreenContainerRef.current as FullscreenCapableElement | null;
    if (!container) {
      return;
    }
    try {
      if (getFullscreenElement(fullscreenDoc) === container) {
        await exitFullscreenForDocument(fullscreenDoc);
        return;
      }
      await requestFullscreenForElement(container);
      const focusTarget = container.querySelector("canvas");
      if (focusTarget instanceof HTMLCanvasElement) {
        focusTarget.focus({ preventScroll: true });
      }
    } catch {
      // Silently ignore fullscreen errors (often triggered by browser restrictions).
    }
  }, []);

  const rendererActionLabel = getRendererModeActionLabel(rendererMode);
  const rendererToggleControl = (
    <button
      type="button"
      className="btn btn-sm btn-outline w-full rounded-full normal-case"
      onClick={toggleRendererMode}
    >
      {rendererActionLabel}
    </button>
  );
  const openUtilityPanel = useCallback((view: UtilityPanelView) => {
    setUtilityPanelView(view);
    setUtilityPanelOpen(true);
  }, []);
  const utilityPanelOptions = isDesktopVariant ? DESKTOP_UTILITY_PANEL_OPTIONS : UTILITY_PANEL_OPTIONS;

  useEffect(() => {
    if (isDesktopVariant && (utilityPanelView === "multiplayer" || utilityPanelView === "debug")) {
      setUtilityPanelView("settings");
    }
  }, [isDesktopVariant, utilityPanelView]);

  const utilityPanelTitle = useMemo(() => {
    switch (utilityPanelView) {
      case "multiplayer":
        return "Multiplayer Lobby";
      case "saves":
        return "Local Save Snapshots";
      case "debug":
        return "Visual Debugger";
      case "settings":
      default:
        return "Game Options";
    }
  }, [utilityPanelView]);
  const mobileQuickActionClassName = isCompactLayoutReady
    ? "btn btn-xs min-h-8 gap-1.5 rounded-lg px-2 normal-case sm:btn-sm sm:px-3"
    : "btn btn-sm min-h-9 gap-2 rounded-lg normal-case";
  const quickActionIconClassName = isCompactLayoutReady ? "h-3.5 w-3.5" : "h-4 w-4";
  const showStartOverlay = !secureMode && shouldStartFromTitleScreen && !startToken;
  const showLoadingOverlay = Boolean(startToken) && loadingGame;

  useEffect(() => {
    if (!showLoadingOverlay) {
      return;
    }
    const timer = window.setInterval(() => {
      setLoadingOverlayTick((value) => value + 1);
    }, 500);
    return () => {
      window.clearInterval(timer);
    };
  }, [showLoadingOverlay]);

  const stalledLoadMs =
    showLoadingOverlay && loadProgressUpdatedAtRef.current
      ? Math.max(0, Date.now() - loadProgressUpdatedAtRef.current)
      : 0;
  const loadPercent = Math.round(getDisplayLoadProgressRatio(loadProgress, stalledLoadMs) * 100);
  const loadLabel = getDisplayLoadProgressLabel(loadProgress, stalledLoadMs);
  const showGameCanvas = hasMounted && (secureMode || Boolean(startToken));
  // Avoid render-time logging to reduce dev refresh noise.
  const controlRows = useMemo(() => {
    void keyBindingsVersion;
    return buildControlRows(getActiveKeyBindings());
  }, [keyBindingsVersion]);

  const controlTips = useMemo(() => {
    if (!secureMode) {
      return CONTROL_TIPS;
    }
    return CONTROL_TIPS.filter((tip) => !tip.toLowerCase().includes("audio"));
  }, [secureMode]);
  const embedControls = !isDesktopVariant && isCompactLayoutReady;
  const canvasHeightReservePx = useMemo(() => {
    if (!hasMounted || !isCompactLayoutReady || !showTouchControls) {
      return undefined;
    }
    const viewportHeight = window.innerHeight;
    if (!Number.isFinite(viewportHeight)) {
      return undefined;
    }
    return Math.min(Math.max(260, Math.round(viewportHeight * 0.46)), 420);
  }, [hasMounted, isCompactLayoutReady, showTouchControls]);
  const gameCanvasClassName = isFullscreen
    ? "playui-screen-canvas block h-auto w-full rounded-xl bg-base-200 shadow-2xl outline outline-1 outline-base-300/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-base-200"
    : "playui-screen-canvas block h-auto w-full rounded-none bg-base-200 outline outline-1 outline-base-300/70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-base-200";
  const gameCanvasStyle = isFullscreen
    ? {
        boxShadow: "0 18px 50px rgba(5, 10, 18, 0.45)",
      }
    : undefined;
  const fullscreenCanvasLayout = useMemo(
    () =>
      computeFullscreenCanvasLayout({
        viewportWidth: viewportSize.width,
        viewportHeight: viewportSize.height,
      }),
    [viewportSize.height, viewportSize.width]
  );

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    const syncViewport = () => {
      setViewportSize({ width: window.innerWidth, height: window.innerHeight });
    };
    syncViewport();
    window.addEventListener("resize", syncViewport);
    window.addEventListener("orientationchange", syncViewport);
    return () => {
      window.removeEventListener("resize", syncViewport);
      window.removeEventListener("orientationchange", syncViewport);
    };
  }, []);

  useEffect(() => {
    if (typeof window !== "undefined") {
      try {
        const storedTheme = window.localStorage.getItem(THEME_STORAGE_KEY);
        if (isThemeKey(storedTheme)) {
          setTimeOfDay(
            storedTheme === "morning"
              ? TimeOfDay.MORN
              : storedTheme === "night"
                ? TimeOfDay.NIGHT
                : TimeOfDay.DAY
          );
        } else {
          setTimeOfDay(getLocalTimeOfDay());
        }
        const storedBrandTheme = window.localStorage.getItem(BRAND_THEME_STORAGE_KEY);
        if (isBrandThemeKey(storedBrandTheme)) {
          setBrandTheme(storedBrandTheme);
        }
      } catch {
        setTimeOfDay(getLocalTimeOfDay());
      }
    }
    setDayOfWeek(getLocalDayOfWeek());
  }, []);

  useEffect(() => {
    const supabase = supabaseClientRef.current;
    if (!supabase) {
      hydratedSupabaseSettingsRef.current = true;
      return;
    }
    let active = true;
    void supabase.auth.getUser().then(({ data }) => {
      if (!active) {
        return;
      }
      setSupabaseUserId(data.user?.id ?? null);
    }).catch(() => {
      if (active) {
        setSupabaseUserId(null);
      }
    });
    const { data } = supabase.auth.onAuthStateChange((_event, session) => {
      setSupabaseUserId(session?.user?.id ?? null);
    });
    return () => {
      active = false;
      data.subscription.unsubscribe();
    };
  }, []);

  useEffect(() => {
    const supabase = supabaseClientRef.current;
    if (!supabaseUserId || !supabase) {
      hydratedSupabaseSettingsRef.current = true;
      return;
    }
    hydratedSupabaseSettingsRef.current = false;
    let cancelled = false;
    void (async () => {
      try {
        const { data, error } = await supabase
          .from("play_user_settings")
          .select("user_id, player_name, player_gender, time_of_day, sound_enabled, instant_mode_enabled, brand_theme")
          .eq("user_id", supabaseUserId)
          .maybeSingle();
        if (cancelled) {
          return;
        }
        if (error) {
          console.warn("[play-panel] failed to load user settings", error.message);
          hydratedSupabaseSettingsRef.current = true;
          return;
        }
        const stored = parseSupabaseUserSettings(data);
        if (stored) {
          const nextName = normalisePlayerName(stored.player_name);
          setPlayerName(nextName);
          setPlayerGender(stored.player_gender);
          setTimeOfDay(stored.time_of_day);
          soundEnabledRef.current = stored.sound_enabled;
          setSoundEnabled(stored.sound_enabled);
          setInstantModeEnabled(stored.instant_mode_enabled);
          setBrandTheme(stored.brand_theme);
          const game = gameRef.current;
          game?.setPlayerName(nextName);
          game?.setPlayerGender(stored.player_gender);
          game?.setTimeOfDay(stored.time_of_day);
          game?.setAudioMuted(!stored.sound_enabled);
          if (game) {
            game.getGameState().wram.instant_mode = stored.instant_mode_enabled;
          }
        }
      } catch {
        // Ignore transient Supabase read errors; defaults remain active.
      } finally {
        if (!cancelled) {
          hydratedSupabaseSettingsRef.current = true;
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [supabaseUserId]);

  useEffect(() => {
    const supabase = supabaseClientRef.current;
    if (!supabaseUserId || !supabase || !hydratedSupabaseSettingsRef.current) {
      return;
    }
    const payload = {
      user_id: supabaseUserId,
      player_name: normalisePlayerName(playerName),
      player_gender: playerGender,
      time_of_day: timeOfDay,
      sound_enabled: soundEnabled,
      instant_mode_enabled: instantModeEnabled,
      brand_theme: brandTheme,
    };
    void (async () => {
      try {
        const { error } = await supabase
          .from("play_user_settings")
          .upsert(payload, { onConflict: "user_id" });
        if (error) {
          console.warn("[play-panel] failed to save user settings", error.message);
        }
      } catch {
        // Ignore transient Supabase write errors; settings stay in memory.
      }
    })();
  }, [supabaseUserId, playerName, playerGender, timeOfDay, soundEnabled, instantModeEnabled, brandTheme]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    let cancelled = false;
    const loadLeaderboard = async () => {
      try {
        const response = await fetch("/api/multiplayer/matches?limit=5", { cache: "no-store" });
        if (!response.ok) {
          return;
        }
        const payload = (await response.json()) as {
          ok?: boolean;
          leaderboard?: MultiplayerLeaderboardEntry[];
        };
        if (!cancelled && payload.ok) {
          setMultiplayerLeaderboard(payload.leaderboard ?? []);
        }
      } catch {
        // Leaderboard is auxiliary; the live world should still work.
      }
    };
    void loadLeaderboard();
    const timerId = window.setInterval(() => {
      void loadLeaderboard();
    }, 30_000);
    return () => {
      cancelled = true;
      window.clearInterval(timerId);
    };
  }, []);

  useEffect(() => {
    if (typeof document === "undefined") {
      return;
    }
    const root = document.documentElement;
    const themeKey: ThemeKey =
      timeOfDay === TimeOfDay.MORN ? "morning" : timeOfDay === TimeOfDay.NIGHT ? "night" : "day";
    root.setAttribute("data-theme", themeKey);
    if (typeof window !== "undefined") {
      window.localStorage.setItem(THEME_STORAGE_KEY, themeKey);
    }
    const game = gameRef.current;
    if (game) {
      game.setTimeOfDay(timeOfDay);
    }
  }, [timeOfDay]);

  useEffect(() => {
    if (typeof document === "undefined") {
      return;
    }
    applyBrandThemeToDocument(brandTheme);
    if (typeof window !== "undefined") {
      window.localStorage.setItem(BRAND_THEME_STORAGE_KEY, brandTheme);
    }
  }, [brandTheme]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    if (!rendererModeAuto) {
      return;
    }
    const media = window.matchMedia(RENDERER_MODE_LAPTOP_QUERY);
    const handleChange = () => {
      setRendererMode(media.matches ? "both" : "tile");
    };
    handleChange();
    if (typeof media.addEventListener === "function") {
      media.addEventListener("change", handleChange);
      return () => media.removeEventListener("change", handleChange);
    }
    media.addListener(handleChange);
    return () => media.removeListener(handleChange);
  }, [rendererModeAuto]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    let cancelled = false;
    const pullActivity = async (): Promise<void> => {
      try {
        const response = await fetch("/api/mcp/activity", {
          method: "GET",
          cache: "no-store",
        });
        if (!response.ok) {
          return;
        }
        const payload = (await response.json()) as {
          ok?: boolean;
          apiSkillsMcpCount?: number;
        };
        if (!payload.ok || cancelled) {
          return;
        }
        apiMcpCountRef.current = Math.max(
          0,
          Math.trunc(payload.apiSkillsMcpCount ?? 0)
        );
        refreshOnlineCounts();
      } catch {
        // Keep UI responsive if the activity endpoint is unavailable.
      }
    };

    void pullActivity();
    const timerId = window.setInterval(() => {
      void pullActivity();
    }, 5000);

    return () => {
      cancelled = true;
      window.clearInterval(timerId);
    };
  }, [refreshOnlineCounts]);

  useEffect(() => {
    gameRef.current?.setOverworldRemoteRenderEnabled(remoteSpritesVisible);
    gameRef.current?.setOverworldRemoteCrowdView(crowdViewEnabled);
    gameRef.current?.setOverworldRemotePlayers(
      remoteSpritesVisible ? latestRemotePlayersRef.current : []
    );
  }, [remoteSpritesVisible, crowdViewEnabled]);

  useEffect(() => {
    if (typeof document === "undefined") {
      return;
    }
    const fullscreenDoc = document as FullscreenCapableDocument;
    const canFullscreen =
      canUseFullscreen(fullscreenDoc);
    setFullscreenAvailable(Boolean(canFullscreen));
    const handleFullscreenChange = () => {
      setIsFullscreen(getFullscreenElement(fullscreenDoc) === fullscreenContainerRef.current);
    };
    handleFullscreenChange();
    document.addEventListener("fullscreenchange", handleFullscreenChange);
    document.addEventListener("webkitfullscreenchange", handleFullscreenChange as EventListener);
    return () => {
      document.removeEventListener("fullscreenchange", handleFullscreenChange);
      document.removeEventListener("webkitfullscreenchange", handleFullscreenChange as EventListener);
    };
  }, []);

  const desktopPanelContent = useMemo(() => {
    if (utilityPanelView === "multiplayer") {
      return (
        <MultiplayerMenu
          onConnect={handleMultiplayerConnect}
          onDisconnect={handleMultiplayerDisconnect}
          onToggleRemoteSprites={handleToggleRemoteSprites}
          onToggleCrowdView={handleToggleCrowdView}
          onRequestBattle={handleRequestBattle}
          onRequestTrade={handleRequestTrade}
          onSelectRemotePlayer={setSelectedRemoteUserId}
          onAcceptRequest={handleAcceptRequest}
          onDeclineRequest={handleDeclineRequest}
          onJoinMatchmaking={handleJoinMatchmaking}
          onLeaveMatchmaking={handleLeaveMatchmaking}
          isAuthenticated={Boolean(supabaseUserId)}
          authLabel="Sign in from the account menu to join live multiplayer."
          remotePlayers={remotePlayers}
          selectedRemoteUserId={selectedRemoteUserId}
          leaderboard={multiplayerLeaderboard}
          remoteSpritesVisible={remoteSpritesVisible}
          crowdViewEnabled={crowdViewEnabled}
          onlinePlayerCount={onlinePlayerCount}
          onlineAiCount={onlineAiCount}
          canRequestInteraction={remotePlayers.length > 0}
          pendingOutgoingLabel={
            outgoingRequest
              ? `Waiting for ${outgoingRequest.targetName} to respond to ${outgoingRequest.kind}...`
              : null
          }
          incomingRequestLabel={
            incomingRequest
              ? `${incomingRequest.fromPlayerName} requests a ${incomingRequest.kind}.`
              : null
          }
          interactionStatusLabel={interactionStatus}
          queueMode={matchmakingQueueMode}
        />
      );
    }
    if (utilityPanelView === "saves") {
      return <GuestSavePanel onLoadSave={handleLoadSave} />;
    }
    if (utilityPanelView === "debug") {
      return <VisualDebugPanel game={gameRef.current} />;
    }
    return (
      <SettingsPanel
        playerGender={playerGender}
        onPlayerGenderChange={applyGender}
        timeOfDay={timeOfDay}
        onTimeOfDayChange={applyTimeOfDay}
        playerName={playerName}
        onPlayerNameChange={applyName}
        soundEnabled={soundEnabled}
        onSoundEnabledChange={applySoundEnabled}
        instantModeEnabled={instantModeEnabled}
        onInstantModeEnabledChange={applyInstantModeEnabled}
        brandTheme={brandTheme}
        onBrandThemeChange={applyBrandTheme}
        playIntroEnabled={skipToPlayEnabled}
        onPlayIntroEnabledChange={applySkipToPlayEnabled}
        showBootModeControl={!isDesktopVariant}
      />
    );
  }, [
    utilityPanelView,
    handleMultiplayerConnect,
    handleMultiplayerDisconnect,
    handleToggleRemoteSprites,
    handleToggleCrowdView,
    handleRequestBattle,
    handleRequestTrade,
    handleAcceptRequest,
    handleDeclineRequest,
    handleJoinMatchmaking,
    handleLeaveMatchmaking,
    supabaseUserId,
    remotePlayers,
    selectedRemoteUserId,
    multiplayerLeaderboard,
    remoteSpritesVisible,
    crowdViewEnabled,
    onlinePlayerCount,
    onlineAiCount,
    outgoingRequest,
    incomingRequest,
    interactionStatus,
    matchmakingQueueMode,
    handleLoadSave,
    playerGender,
    applyGender,
    timeOfDay,
    applyTimeOfDay,
    playerName,
    applyName,
    soundEnabled,
    applySoundEnabled,
    instantModeEnabled,
    applyInstantModeEnabled,
    brandTheme,
    applyBrandTheme,
    isDesktopVariant,
    skipToPlayEnabled,
    applySkipToPlayEnabled,
  ]);

  const multiplayerChatDock = (
    <MultiplayerChat
      messages={chatMessages}
      systemMessages={interactionStatus ? [interactionStatus] : undefined}
      remotePlayers={remotePlayers}
      selectedRemoteUserId={selectedRemoteUserId}
      blockedUserIds={blockedChatUserIds}
      onSelectRemotePlayer={setSelectedRemoteUserId}
      onSend={handleSendChat}
      onReport={handleReportChat}
      onToggleBlock={handleToggleChatBlock}
      onOpenLobby={() => {
        setUtilityPanelView("multiplayer");
        if (!isDesktopVariant) {
          setUtilityPanelOpen(true);
        }
      }}
    />
  );

  if (isDesktopVariant) {
    return (
      <div className="flex h-full min-h-0 w-full overflow-hidden bg-black text-white">
        <main className="relative flex min-w-0 flex-1 flex-col p-3">
          {!desktopSidebarVisible ? (
            <button
              type="button"
              className="btn btn-sm btn-outline absolute right-4 top-4 z-10 gap-2 rounded border-white/25 bg-black/70 text-white normal-case shadow hover:bg-white/10"
              onClick={() => applyDesktopSidebarVisible(true)}
              aria-label="Show sidebar"
              aria-expanded={false}
              aria-controls="desktop-sidebar"
            >
              <FontAwesomeIcon icon={faAngleLeft} className="h-4 w-4" aria-hidden="true" />
              <span>Show Sidebar</span>
            </button>
          ) : null}
          <div
            ref={fullscreenContainerRef}
            className="flex min-h-0 flex-1 flex-col overflow-hidden rounded border border-white/10 bg-black"
          >
            <div
              ref={canvasContainerRef}
              className="relative flex min-h-0 flex-1 items-center justify-center overflow-hidden p-3"
            >
              {showLoadingOverlay ? (
                <div className="absolute inset-0 z-[3] flex items-center justify-center bg-black/80 p-4 text-center">
                  <div className="w-[min(22rem,88vw)] space-y-2">
                    <p className="text-xs font-semibold uppercase tracking-[0.2em] text-white/75">{loadLabel}</p>
                    <progress className="progress progress-primary w-full" value={loadPercent} max={100} />
                    <p className="text-xs text-white/75">{loadPercent}% complete</p>
                  </div>
                </div>
              ) : null}
              {showGameCanvas ? (
                <GameCanvas
                  key={gameInstanceKey}
                  autoStart={Boolean(startToken)}
                  loadSlot={MANUAL_SAVE_SLOT}
                  muted={!soundEnabled}
                  playIntro={shouldPlayIntro}
                  newGame={shouldStartFromTitleScreen}
                  preloadMode="auto"
                  rendererMode={rendererMode}
                  runtimeMode="local"
                  canvasClassName="playui-screen-canvas block h-auto w-full bg-black outline-none focus-visible:ring-2 focus-visible:ring-primary"
                  canvasStyle={{ maxWidth: "100%", maxHeight: "100%" }}
                  onInputStateChange={handleInputStateChange}
                  onPostEventReady={handlePostEventReady}
                  onGameReady={handleGameReady}
                  onLoadProgress={handleLoadProgress}
                />
              ) : (
                <canvas
                  width={PLACEHOLDER_WIDTH}
                  height={PLACEHOLDER_HEIGHT}
                  aria-label="KrabbyClaw game canvas"
                  tabIndex={0}
                  className="playui-screen-canvas block h-auto w-full bg-black outline-none"
                  style={{ maxWidth: `${PLACEHOLDER_WIDTH}px`, maxHeight: `${PLACEHOLDER_HEIGHT}px` }}
                />
              )}
            </div>
          </div>
        </main>
        {desktopSidebarVisible ? (
          <aside
            id="desktop-sidebar"
            data-testid="desktop-sidebar"
            className="flex w-[26rem] max-w-[40vw] shrink-0 flex-col overflow-hidden border-l border-white/10 bg-[#101010]"
          >
            <div className="border-b border-white/10 p-4">
              <div className="mb-3 flex items-center justify-between gap-2">
                <div>
                  <p className="text-xs font-semibold uppercase tracking-[0.16em] text-white/45">KrabbyClaw</p>
                  <h1 className="text-lg font-semibold">Desktop</h1>
                </div>
                <div className="flex items-center gap-2">
                  <span className="rounded border border-emerald-400/30 px-2 py-1 text-[0.65rem] font-semibold uppercase tracking-[0.12em] text-emerald-200">
                    Local
                  </span>
                  <button
                    type="button"
                    className="btn btn-square btn-sm btn-ghost rounded text-white/75 hover:bg-white/10 hover:text-white"
                    onClick={() => applyDesktopSidebarVisible(false)}
                    aria-label="Hide sidebar"
                    aria-expanded={true}
                    aria-controls="desktop-sidebar"
                  >
                    <FontAwesomeIcon icon={faAngleRight} className="h-4 w-4" aria-hidden="true" />
                    <span className="sr-only">Hide Sidebar</span>
                  </button>
                </div>
              </div>
              <div className="grid grid-cols-2 gap-2">
                <button type="button" className="btn btn-sm btn-primary rounded normal-case" onClick={toggleRendererMode}>
                  {rendererActionLabel}
                </button>
                {fullscreenAvailable ? (
                  <button
                    type="button"
                    className="btn btn-sm btn-outline rounded normal-case"
                    onClick={() => void toggleFullscreen()}
                  >
                    {isFullscreen ? "Exit Fullscreen" : "Fullscreen"}
                  </button>
                ) : null}
              </div>
            </div>
            <div className="border-b border-white/10 p-3">
              <div className="grid grid-cols-2 gap-2">
                {utilityPanelOptions.map((option) => (
                  <button
                    key={`desktop-${option.view}`}
                    type="button"
                    className={`btn btn-sm gap-2 rounded normal-case ${
                      utilityPanelView === option.view ? "btn-primary" : "btn-outline"
                    }`}
                    onClick={() => setUtilityPanelView(option.view)}
                  >
                    <FontAwesomeIcon icon={option.icon} className="h-4 w-4" aria-hidden="true" />
                    <span>{option.label}</span>
                  </button>
                ))}
              </div>
            </div>
            <div className="min-h-[18rem] flex-[1.2] p-3">
              {multiplayerChatDock}
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto border-t border-white/10 p-3 text-base-content">
              <div className="rounded border border-white/10 bg-base-100 p-3">
                {desktopPanelContent}
              </div>
            </div>
          </aside>
        ) : null}
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 w-full flex-col overflow-hidden">
      <div
        className="grid h-full min-h-0 gap-2"
        style={{
          gridTemplateColumns: hasMounted && !isCompactLayoutReady && !isFullscreen && !secureMode
            ? "minmax(0, 1fr) 18rem"
            : "1fr",
        }}
      >
        <div className="min-h-0 min-w-0 overflow-hidden">
          <div
            ref={fullscreenContainerRef}
            className="flex h-full min-h-0 flex-col gap-2"
            style={
              isFullscreen
                ? {
                    position: "relative",
                    background:
                      "linear-gradient(145deg, color-mix(in oklab, var(--color-panel) 92%, var(--color-accent) 8%) 0%, color-mix(in oklab, var(--color-panel-strong) 90%, var(--color-accent) 10%) 55%, color-mix(in oklab, var(--color-panel) 92%, var(--color-accent) 8%) 100%)",
                    overflow: "hidden",
                  }
                : undefined
            }
          >
            <section
              className={isFullscreen ? "playui-shell flex min-h-0 flex-1 flex-col" : "playui-shell kc-surface-card flex min-h-0 flex-1 flex-col"}
              data-fullscreen={isFullscreen ? "true" : "false"}
              style={isMobileEmulator ? { padding: 0 } : { padding: "0.75rem" }}
            >
              <div
                ref={canvasContainerRef}
                className="focus-within:ring-primary focus-within:ring-offset-base-200 relative flex min-h-0 flex-1 flex-col focus-within:ring-2 focus-within:ring-offset-2"
                style={{
                  justifyContent: embedControls ? "flex-start" : "center",
                  alignItems: embedControls ? "stretch" : "center",
                  borderRadius: isMobileEmulator ? 0 : "6px",
                  padding: embedControls ? 0 : "0.75rem",
                  gap: embedControls ? "0.5rem" : 0,
                  backgroundColor: isFullscreen ? "transparent" : "var(--color-panel-ghost)",
                  border: isMobileEmulator || isFullscreen ? "none" : "1px solid var(--color-panel-border)",
                }}
              >
                <div
                  data-testid="play-quick-actions"
                  className={embedControls
                    ? "relative z-[4] flex w-full shrink-0 items-center justify-end gap-1.5 px-2 pt-2 sm:gap-2"
                    : "absolute right-3 top-3 z-[4] flex max-w-[calc(100%-1.5rem)] flex-wrap items-center justify-end gap-2"
                  }
                >
                  {embedControls ? (
                    <button
                      type="button"
                      className={`${mobileQuickActionClassName} btn-outline border-white/25 text-white shadow-sm hover:bg-white/10`}
                      onClick={() => openUtilityPanel("settings")}
                      style={{ backgroundColor: "rgba(5, 10, 20, 0.42)" }}
                    >
                      <FontAwesomeIcon icon={faBars} className={quickActionIconClassName} aria-hidden="true" />
                      <span>Menu</span>
                    </button>
                  ) : null}
                  <button
                    type="button"
                    className={`${mobileQuickActionClassName} btn-outline border-white/25 text-white shadow-sm hover:bg-white/10`}
                    onClick={() => setControlsOpen(true)}
                    style={{ backgroundColor: "rgba(5, 10, 20, 0.42)" }}
                  >
                    <FontAwesomeIcon icon={faGamepad} className={quickActionIconClassName} aria-hidden="true" />
                    <span>Controls</span>
                  </button>
                  {fullscreenAvailable ? (
                    <button
                      type="button"
                      className={`${mobileQuickActionClassName} btn-primary`}
                      onClick={() => void toggleFullscreen()}
                      aria-pressed={isFullscreen}
                    >
                      <FontAwesomeIcon icon={isFullscreen ? faCompress : faExpand} className={quickActionIconClassName} aria-hidden="true" />
                      <span>{isFullscreen ? "Exit Fullscreen" : "Fullscreen"}</span>
                    </button>
                  ) : null}
                </div>

                <div
                  className="playui-bezel flex min-h-0 w-full flex-1 justify-center overflow-hidden"
                  data-testid="play-canvas-shell"
                  style={
                    isFullscreen
                      ? {
                          height: "100%",
                          alignItems: "center",
                          padding: `${fullscreenCanvasLayout.shellPaddingY}px ${fullscreenCanvasLayout.shellPaddingX}px`,
                        }
                      : undefined
                  }
                >
                  <div
                    className="playui-screen-frame relative flex min-h-0 w-full items-center justify-center overflow-hidden"
                    data-testid="play-canvas-frame"
                    style={
                      isFullscreen
                        ? {
                            width: `${fullscreenCanvasLayout.frameWidth}px`,
                            height: `${fullscreenCanvasLayout.frameHeight}px`,
                            maxWidth: "100%",
                            maxHeight: "100%",
                            aspectRatio: "160 / 144",
                            padding: `${fullscreenCanvasLayout.framePadding}px`,
                            borderRadius: "var(--radius-xl)",
                            border: "1px solid var(--color-panel-border)",
                            background:
                              "linear-gradient(160deg, color-mix(in oklab, var(--color-panel-strong) 90%, var(--color-accent) 10%) 0%, color-mix(in oklab, var(--color-panel) 92%, var(--color-accent) 8%) 60%, color-mix(in oklab, var(--color-panel-strong) 90%, var(--color-accent) 10%) 100%)",
                          }
                        : {
                            height: "100%",
                            maxWidth: "100%",
                            maxHeight: "100%",
                          }
                    }
                  >
                    {showStartOverlay || showLoadingOverlay ? (
                      <div className="absolute inset-0 z-[3] flex items-center justify-center p-4 text-center" style={{ backgroundColor: "rgba(5, 10, 20, 0.8)" }}>
                        <div className="mx-auto w-[min(22rem,88vw)] space-y-3 text-white">
                          {showStartOverlay ? (
                            <>
                              <p className="hidden text-xs font-semibold uppercase tracking-[0.24em] text-white/70 sm:block">Ready to play</p>
                              <h2 className="text-lg font-semibold leading-tight sm:text-xl">{startOverlayPrompt}</h2>
                              <button type="button" className="btn btn-primary btn-lg gap-2 rounded-lg normal-case" onClick={requestStart}>
                                <FontAwesomeIcon icon={faPlay} className="h-4 w-4" aria-hidden="true" />
                                <span>Start Game</span>
                              </button>
                            </>
                          ) : null}
                          {showLoadingOverlay ? (
                            <div className="mx-auto w-full space-y-2">
                              <p className="text-xs font-semibold uppercase tracking-[0.2em] text-white/75">{loadLabel}</p>
                              <progress className="progress progress-primary w-full" value={loadPercent} max={100} />
                              <p className="text-xs text-white/75">{loadPercent}% complete</p>
                            </div>
                          ) : null}
                        </div>
                      </div>
                    ) : null}
                    {showGameCanvas ? (
                      <GameCanvas
                        key={gameInstanceKey}
                        autoStart={Boolean(startToken) || secureMode}
                        loadSlot={MANUAL_SAVE_SLOT}
                        muted={!soundEnabled}
                        playIntro={shouldPlayIntro}
                        newGame={shouldStartFromTitleScreen}
                        preloadMode="auto"
                        rendererMode={rendererMode}
                        runtimeMode={secureMode ? "server" : "local"}
                        canvasClassName={gameCanvasClassName}
                        canvasStyle={gameCanvasStyle}
                        canvasHeightReservePx={canvasHeightReservePx}
                        onInputStateChange={handleInputStateChange}
                        onPostEventReady={handlePostEventReady}
                        onGameReady={handleGameReady}
                        onLoadProgress={handleLoadProgress}
                      />
                    ) : (
                      <canvas
                        width={PLACEHOLDER_WIDTH}
                        height={PLACEHOLDER_HEIGHT}
                        aria-label="KrabbyClaw game canvas"
                        tabIndex={0}
                        className={gameCanvasClassName}
                        style={{
                          ...gameCanvasStyle,
                          maxWidth: `${PLACEHOLDER_WIDTH}px`,
                          maxHeight: `${PLACEHOLDER_HEIGHT}px`,
                        }}
                        onPointerDown={(event) => {
                          event.currentTarget.focus();
                        }}
                      />
                    )}
                  </div>
                </div>

                {embedControls && showTouchControls ? (
                  <VirtualGamepad
                    pressedButtons={pressedButtons}
                    pressedKeys={keyboardKeys}
                    onVirtualButtonsChange={handleVirtualButtonsChange}
                    postEvent={sendEvent}
                    compact={embedControls}
                    embedded
                    showHeader={false}
                    systemControl={rendererToggleControl}
                  />
                ) : null}
              </div>
            </section>

            {!isFullscreen && !secureMode && !embedControls ? (
              <div className="px-1 pb-1">
                <div className="kc-surface-card space-y-3 p-3">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <div>
                      <p className="text-xs font-semibold uppercase tracking-[0.22em] text-base-content/60">Play Console</p>
                      <p className="text-sm text-base-content/75">Launch utility panels and tune your current session.</p>
                    </div>
                    <div className="flex flex-wrap items-center gap-2 text-xs">
                      <span className="badge badge-outline">Renderer: {rendererMode === "text" ? "Text" : "Pixel"}</span>
                      <span className="badge badge-outline">Sound: {soundEnabled ? "On" : "Muted"}</span>
                      <span className="badge badge-outline">Theme: {brandTheme}</span>
                    </div>
                  </div>

                  <div className="flex flex-wrap gap-2">
                    <button
                      type="button"
                      className="btn btn-sm btn-primary gap-2 rounded-lg normal-case"
                      onClick={toggleRendererMode}
                    >
                      {rendererActionLabel}
                    </button>
                    <button
                      type="button"
                      className="btn btn-sm btn-outline gap-2 rounded-lg normal-case"
                      onClick={() => setControlsOpen(true)}
                    >
                      <FontAwesomeIcon icon={faGamepad} className="h-4 w-4" aria-hidden="true" />
                      <span>Controls</span>
                    </button>
                    {embedControls ? (
                      <button
                        type="button"
                        className="btn btn-sm btn-outline rounded-lg normal-case"
                        onClick={() => setShowTouchControls((value) => !value)}
                      >
                        {showTouchControls ? "Hide Touch Pad" : "Show Touch Pad"}
                      </button>
                    ) : null}
                    {UTILITY_PANEL_OPTIONS.map((option) => (
                      <button
                        key={option.view}
                        type="button"
                        className="btn btn-sm btn-outline gap-2 rounded-lg normal-case"
                        onClick={() => openUtilityPanel(option.view)}
                      >
                        <FontAwesomeIcon icon={option.icon} className="h-4 w-4" aria-hidden="true" />
                        <span>{option.label}</span>
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            ) : null}
          </div>
        </div>
        {hasMounted && !isCompactLayoutReady && !isFullscreen && !secureMode ? (
          <aside className="min-h-0 p-1 pl-0" data-testid="play-chat-dock">
            {multiplayerChatDock}
          </aside>
        ) : null}
      </div>

      {controlsOpen ? (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center p-3"
          style={{ backgroundColor: "rgba(4, 9, 20, 0.82)" }}
        >
          <div
            role="dialog"
            aria-modal="true"
            className="flex w-full max-w-2xl flex-col rounded-box border border-base-300 bg-base-100 shadow-2xl"
            style={MODAL_DIALOG_STYLE}
          >
            <div className="border-b border-base-300 px-4 py-3">
              <div className="flex items-center gap-2">
                <span className="inline-block h-2.5 w-2.5 rounded-full bg-primary" />
                <h2 className="text-lg font-semibold">Controls</h2>
              </div>
              <p className="text-xs text-base-content/70">Keyboard and on-screen controls.</p>
            </div>
            <div className="min-h-0 flex-1 space-y-3 overflow-y-auto p-4" style={{ maxHeight: MODAL_BODY_MAX_HEIGHT }}>
              <p className="text-sm text-base-content/80">Use keyboard or on-screen controls.</p>
              <div className="grid gap-3 sm:grid-cols-[1.2fr_0.8fr]">
                <div className="space-y-2">
                  {controlRows.map((row) => {
                    const tone = row.tone ?? "neutral";
                    const toneClass = tone === "accent" ? "border-primary/50" : tone === "ember" ? "border-warning/60" : "border-base-300";
                    return (
                      <div key={row.label} className={`rounded-box border bg-base-200/50 p-3 ${toneClass}`}>
                        <div className="mb-1">
                          <p className="text-xs font-semibold uppercase tracking-[0.2em]">{row.label}</p>
                          <p className="text-sm text-base-content/70">{row.description}</p>
                        </div>
                        {row.layout === "dpad" ? (
                          <div className="flex flex-wrap items-center gap-2">
                            <div className="grid grid-cols-3 grid-rows-3 gap-1">
                              {DPAD_LAYOUT.map((entry) => (
                                <span
                                  key={entry.key}
                                  data-keycap={entry.key}
                                  className="inline-flex min-h-8 min-w-8 items-center justify-center rounded border border-base-300 bg-base-100 px-2 text-xs font-semibold"
                                  style={{ gridArea: entry.area }}
                                >
                                  {entry.key}
                                </span>
                              ))}
                              <span aria-hidden="true" className="col-[2] row-[2] h-8 w-8 rounded border border-base-300 bg-base-100" />
                            </div>
                            {row.extraKey ? <span className="badge badge-outline">{row.extraKey}</span> : null}
                          </div>
                        ) : (
                          <div className="flex flex-wrap gap-1.5">
                            {row.keys.map((key) => (
                              <span
                                key={`${row.label}-${key}`}
                                data-keycap={key}
                                className="inline-flex min-h-8 min-w-8 items-center justify-center rounded border border-base-300 bg-base-100 px-2 text-xs font-semibold"
                              >
                                {key}
                              </span>
                            ))}
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
                <div className="rounded-box border border-base-300 bg-base-200/40 p-3">
                  <p className="mb-2 text-xs font-semibold uppercase tracking-[0.2em] text-base-content/70">Tips</p>
                  <div className="space-y-2">
                    {controlTips.map((tip) => (
                      <div key={tip} className="flex items-start gap-2">
                        <span className="mt-1.5 inline-block h-1.5 w-1.5 rounded-full bg-primary" />
                        <p className="text-sm text-base-content/80">{tip}</p>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
              <KeybindingsEditor />
            </div>
            <div className="flex justify-end border-t border-base-300 p-3">
              <button type="button" className="btn btn-sm btn-primary" onClick={() => setControlsOpen(false)}>
                Close
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {utilityPanelOpen ? (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center p-3"
          style={{ backgroundColor: "rgba(4, 9, 20, 0.82)" }}
        >
          <div
            role="dialog"
            aria-modal="true"
            className="flex w-full max-w-4xl flex-col rounded-box border border-base-300 bg-base-100 shadow-2xl"
            style={MODAL_DIALOG_STYLE}
          >
            <div className="flex flex-wrap items-center justify-between gap-2 border-b border-base-300 px-4 py-3">
              <h2 className="text-lg font-semibold">{utilityPanelTitle}</h2>
              <div className="join flex max-w-full flex-wrap overflow-hidden rounded-lg border border-base-300 bg-base-200/40 p-0.5">
                {utilityPanelOptions.map((option) => (
                  <button
                    key={`modal-${option.view}`}
                    type="button"
                    className={`btn btn-xs join-item min-w-20 gap-1.5 rounded-md border-0 normal-case sm:btn-sm sm:min-w-24 ${
                      utilityPanelView === option.view ? "btn-primary" : "btn-ghost"
                    }`}
                    onClick={() => setUtilityPanelView(option.view)}
                  >
                    <FontAwesomeIcon icon={option.icon} className="h-3.5 w-3.5" aria-hidden="true" />
                    <span>{option.label}</span>
                  </button>
                ))}
                <button
                  type="button"
                  className="btn btn-xs btn-ghost join-item min-w-16 rounded-md border-0 normal-case sm:btn-sm"
                  onClick={() => setUtilityPanelOpen(false)}
                >
                  Close
                </button>
              </div>
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto p-4" style={{ maxHeight: MODAL_BODY_MAX_HEIGHT }}>
              {utilityPanelView === "multiplayer" ? (
                <div className="grid min-h-0 gap-4 lg:grid-cols-[minmax(0,1.15fr)_minmax(18rem,0.85fr)]">
                  <div className="h-[min(32rem,58dvh)] min-h-[20rem]">{multiplayerChatDock}</div>
                  <MultiplayerMenu
                  onConnect={handleMultiplayerConnect}
                  onDisconnect={handleMultiplayerDisconnect}
                  onToggleRemoteSprites={handleToggleRemoteSprites}
                  onToggleCrowdView={handleToggleCrowdView}
                  onRequestBattle={handleRequestBattle}
                  onRequestTrade={handleRequestTrade}
                  onSelectRemotePlayer={setSelectedRemoteUserId}
                  onAcceptRequest={handleAcceptRequest}
                  onDeclineRequest={handleDeclineRequest}
                  onJoinMatchmaking={handleJoinMatchmaking}
                  onLeaveMatchmaking={handleLeaveMatchmaking}
                  isAuthenticated={Boolean(supabaseUserId)}
                  authLabel="Sign in from the account menu to join live multiplayer."
                  remotePlayers={remotePlayers}
                  selectedRemoteUserId={selectedRemoteUserId}
                  leaderboard={multiplayerLeaderboard}
                  remoteSpritesVisible={remoteSpritesVisible}
                  crowdViewEnabled={crowdViewEnabled}
                  onlinePlayerCount={onlinePlayerCount}
                  onlineAiCount={onlineAiCount}
                  canRequestInteraction={remotePlayers.length > 0}
                  pendingOutgoingLabel={
                    outgoingRequest
                      ? `Waiting for ${outgoingRequest.targetName} to respond to ${outgoingRequest.kind}...`
                      : null
                  }
                  incomingRequestLabel={
                    incomingRequest
                      ? `${incomingRequest.fromPlayerName} requests a ${incomingRequest.kind}.`
                      : null
                  }
                  interactionStatusLabel={interactionStatus}
                  queueMode={matchmakingQueueMode}
                  />
                </div>
              ) : null}
              {utilityPanelView === "settings" ? (
                <SettingsPanel
                  playerGender={playerGender}
                  onPlayerGenderChange={applyGender}
                  timeOfDay={timeOfDay}
                  onTimeOfDayChange={applyTimeOfDay}
                  playerName={playerName}
                  onPlayerNameChange={applyName}
                  soundEnabled={soundEnabled}
                  onSoundEnabledChange={applySoundEnabled}
                  instantModeEnabled={instantModeEnabled}
                  onInstantModeEnabledChange={applyInstantModeEnabled}
                  brandTheme={brandTheme}
                  onBrandThemeChange={applyBrandTheme}
                  playIntroEnabled={skipToPlayEnabled}
                  onPlayIntroEnabledChange={applySkipToPlayEnabled}
                />
              ) : null}
              {utilityPanelView === "saves" ? <GuestSavePanel onLoadSave={handleLoadSave} /> : null}
              {utilityPanelView === "debug" ? <VisualDebugPanel game={gameRef.current} /> : null}
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
};

export default PlayPanel;

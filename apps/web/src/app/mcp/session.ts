import "@/lib/pokecrystal-core/register-server-adapters";
import { Game } from "@/app/game";
import fs from "node:fs/promises";
import path from "node:path";
import { TextUI, type TextSnapshot } from "@pokecrystal/core/ui/text-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { getSettings } from "@pokecrystal/core/core/config";
import { pushDebugLog } from "@pokecrystal/core/core/debug-log";
import {
  buildSnapshotPayload,
  isNonBlockingPcUiSnapshot,
  promptFromSnapshot,
  renderFrameToText,
} from "./text-render";
import type {
  McpMacroTraceAction,
  McpMacroTraceStep,
  McpMeta,
  TextSnapshotPayload,
} from "./text-render";
import type { McpFlowStateSnapshot } from "./flow-state";
import { normalizeSessionId } from "./session-guards";
import { PRIMARY_MCP_SESSION_ID } from "./session-id";
import { encodeSurfaceToPng } from "./image-encoding";
import { CompositeUI, type CompositeChild } from "@pokecrystal/core/ui/composite-ui";
import { DomCanvasUI } from "@pokecrystal/core/ui/dom-canvas-ui";
import { getMapEnvironment, mapConstantToName, Spawn } from "@pokecrystal/core/engine/world/maps";
import { isPermissionPassable } from "@pokecrystal/core/engine/world/overworld/collision-rules";
import {
  buildRouteRenderSnapshot,
  buildUnavailableRouteRenderSnapshot,
  renderRouteRenderTileSurface,
  type RouteRenderDetail,
  type RouteRenderSnapshot,
} from "@pokecrystal/core/engine/world/overworld/route-render";
import {
  getMcpIdentityContext,
  runWithMcpIdentityContext,
} from "@pokecrystal/core/core/mcp-identity-context.server";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import { facingDirectionFromString, PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { defaultKeyBindings, GameButton } from "@pokecrystal/core/input/config";
import {
  hasSaveGame,
  saveGame,
  SaveGameError,
  SaveGameValidationError,
} from "@pokecrystal/core/core/save";
import type { GameState } from "@pokecrystal/core/core/state";
import {
  buildMapInfoSnapshot,
  type McpMapHotspotType,
  type McpMapInfoSnapshot,
  type OverworldMapInfoSource,
} from "./map-info";
import { YesNoPrompt } from "@pokecrystal/core/ui/text/dialogue";
import { createSupabaseServiceRoleClient } from "@/lib/supabase/server";
import type { AudioPlaybackSnapshot } from "@pokecrystal/core/engine/systems/audio";

export type Direction = "up" | "down" | "left" | "right";
export type Button = "a" | "b" | "start" | "select" | Direction;
export type ActionResultReason = "blocked" | "no_change" | "menu" | "busy" | "unknown";

export type ActionResult = {
  ok: boolean;
  changed: boolean;
  reason?: ActionResultReason;
  events?: string[];
};

export type ActionResultWithSnapshot = {
  result: ActionResult;
  snapshotText?: string;
};

type ActionSnapshotOptions = {
  settleSnapshot?: boolean;
};

type TypeTextOptions = {
  clear?: boolean;
  submit?: boolean;
};

type NameEntryCursor = {
  row: number;
  column: number;
};

type NameEntryTarget = NameEntryCursor & {
  caseMode: "upper" | "lower";
};

const NAME_ENTRY_SPECIAL_TARGETS = new Map<string, NameEntryTarget>([
  ["-", { row: 3, column: 0, caseMode: "upper" }],
  ["?", { row: 3, column: 1, caseMode: "upper" }],
  ["!", { row: 3, column: 2, caseMode: "upper" }],
  ["/", { row: 3, column: 3, caseMode: "upper" }],
  [".", { row: 3, column: 4, caseMode: "upper" }],
  [",", { row: 3, column: 5, caseMode: "upper" }],
  ["×", { row: 3, column: 0, caseMode: "lower" }],
  ["(", { row: 3, column: 1, caseMode: "lower" }],
  [")", { row: 3, column: 2, caseMode: "lower" }],
  [":", { row: 3, column: 3, caseMode: "lower" }],
  [";", { row: 3, column: 4, caseMode: "lower" }],
  ["[", { row: 3, column: 5, caseMode: "lower" }],
  ["]", { row: 3, column: 6, caseMode: "lower" }],
]);
const NAME_ENTRY_ROW_COUNT = 5;
const NAME_ENTRY_COLUMN_COUNT = 9;
const NAME_ENTRY_BOTTOM_CASE_COLUMN = 0;

const isLowAuthorityHotspotType = (type: McpMapHotspotType | undefined): boolean =>
  type === "sign" || type === "landmark" || type === "utility" || type === "trigger";

const normalizeSurfaceKindToken = (value: string): string =>
  value
    .trim()
    .toLowerCase()
    .replace(/['’]/g, "")
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");

const readSnapshotLabelValue = (
  lines: readonly string[] | null | undefined,
  label: string
): string | undefined => {
  const prefix = `${label.toUpperCase()}:`;
  const line = (lines ?? []).find((entry) => entry.trim().toUpperCase().startsWith(prefix));
  return line ? line.slice(line.indexOf(":") + 1).trim() : undefined;
};

const selectedSnapshotLine = (lines: readonly string[] | null | undefined): string | undefined => {
  const line = (lines ?? []).find((entry) => /^\s*[>▶▷]/.test(entry));
  return line?.replace(/^\s*[>▶▷]\s*/, "").trim() || undefined;
};

const snapshotLooksLikePcSurface = (frame: TextSnapshotPayload): boolean => {
  const lines = [
    ...(frame.viewport ?? []),
    ...(frame.menu ?? []),
    ...(frame.info ?? []),
    frame.titles?.viewport,
    frame.titles?.info,
  ]
    .filter((line): line is string => typeof line === "string")
    .map((line) => line.trim().toUpperCase());
  return lines.some((line) =>
    line.includes("BILL'S PC") ||
    line === "DEPOSIT #MON" ||
    line === "WITHDRAW #MON" ||
    line === "MOVE #MON W/O MAIL" ||
    line.includes("WITHDRAW <PK><MN>") ||
    line.includes("DEPOSIT <PK><MN>") ||
    line.startsWith("BOX ") ||
    line.startsWith("SELECTED:")
  );
};

const deriveSurfaceKind = (frame: TextSnapshotPayload, mode: string): string => {
  if (snapshotLooksLikePcSurface(frame)) {
    return "pc";
  }
  const info = frame.info ?? [];
  const viewport = frame.viewport ?? [];
  const titles = frame.titles ?? {};
  const state = readSnapshotLabelValue(info, "STATE");
  const title = titles.viewport || titles.info || viewport[0] || mode;
  const candidates = [
    state,
    title,
    viewport.find((line) => /title screen/i.test(line)),
    viewport.find((line) => /name entry/i.test(line)),
  ].filter((entry): entry is string => Boolean(entry && entry.trim()));

  for (const candidate of candidates) {
    const normalized = normalizeSurfaceKindToken(candidate);
    if (normalized === "title" || normalized === "title_screen") return "title";
    if (normalized === "oak_intro" || normalized === "oak_finale") return "oak_intro";
    if (normalized.includes("pc")) return "pc";
    if (normalized === "name_entry") return "name_entry";
    if (normalized) return normalized;
  }
  return normalizeSurfaceKindToken(mode) || "unknown";
};

const INPUT_OWNING_SURFACE_KINDS = new Set(["pc", "pokegear", "slot_machine", "fly_to_where"]);
const INPUT_OWNING_SURFACE_SETTLE_MS = 20;

const isInputOwningSurfaceSnapshot = (frame: TextSnapshotPayload | null | undefined): boolean => {
  if (!frame) {
    return false;
  }
  return INPUT_OWNING_SURFACE_KINDS.has(deriveSurfaceKind(frame, "overworld"));
};

const buildSurfaceStatus = (
  frame: TextSnapshotPayload,
  mode: string
): NonNullable<McpStatusSnapshot["surface"]> => {
  const legacyFrame = frame as TextSnapshotPayload & {
    viewportLines?: string[];
    infoLines?: string[];
    menuLines?: string[] | null;
    promptLines?: string[] | null;
    dialogueLines?: string[] | null;
    viewportTitle?: string;
    infoTitle?: string;
  };
  const viewport = frame.viewport ?? legacyFrame.viewportLines ?? [];
  const info = frame.info ?? legacyFrame.infoLines ?? [];
  const menu = frame.menu ?? legacyFrame.menuLines ?? null;
  const prompt = frame.prompt ?? legacyFrame.promptLines ?? null;
  const dialogue = frame.dialogue ?? legacyFrame.dialogueLines ?? null;
  const titles = frame.titles ?? {
    viewport: legacyFrame.viewportTitle ?? "Surface",
    info: legacyFrame.infoTitle ?? "Surface",
  };
  const normalizedFrame = {
    ...frame,
    viewport,
    info,
    menu,
    prompt,
    dialogue,
    titles,
  };
  const controls = info
    .map((line) => line.trim())
    .filter((line) => /(?:=|WAIT:)/.test(line) && !/^(STATE|MODE|SCENE|PHASE|TIMER|SCX|WY|PENDING ACTION):/i.test(line));
  const primaryText =
    dialogue?.find((line) => line.trim()) ??
    prompt?.find((line) => line.trim()) ??
    menu?.find((line) => line.trim()) ??
    viewport.find((line) => line.trim());
  return {
    kind: deriveSurfaceKind(normalizedFrame, mode),
    title: titles.viewport || titles.info || "Surface",
    state: readSnapshotLabelValue(info, "STATE"),
    phase: readSnapshotLabelValue(info, "PHASE"),
    waiting: readSnapshotLabelValue(info, "WAITING")?.toLowerCase() === "yes" || undefined,
    menu_open: Boolean(menu?.length) || undefined,
    prompt_open: Boolean(prompt?.length) || undefined,
    dialogue_open: Boolean(dialogue?.length) || undefined,
    selected: selectedSnapshotLine(menu) ?? selectedSnapshotLine(prompt),
    controls: controls.length ? controls : undefined,
    primary_text: primaryText?.trim() || undefined,
  };
};

export type McpStatusSnapshot = {
  mode: string;
  menu: boolean;
  instant_mode?: boolean;
  surface?: {
    kind: string;
    title: string;
    state?: string;
    phase?: string;
    waiting?: boolean;
    menu_open?: boolean;
    prompt_open?: boolean;
    dialogue_open?: boolean;
    selected?: string;
    controls?: string[];
    primary_text?: string;
  };
  notices?: string[];
  audio?: AudioPlaybackSnapshot;
  battle_state?: string;
  battle_is_trainer?: boolean;
  battle_turn_cursor?: number;
  battle_has_player_action?: boolean;
  battle_has_enemy_action?: boolean;
  engine_debug?: {
    movement_lock_count?: number;
    text_lock_active?: boolean;
    blocking_task_count?: number;
    blocking_movement_lock_active?: boolean;
    script_runner?: {
      stack_depth?: number;
      awaiting_resume?: number;
      stop_execution?: boolean;
      is_busy?: boolean;
      state?: string;
    };
  };
  in_menu?: boolean;
  in_battle?: boolean;
  in_dialog?: boolean;
  textbox_open?: boolean;
  text_box_open?: boolean;
  text_advance_pending?: boolean;
  prompt_pending?: boolean;
  unown_puzzle_active?: boolean;
  unown_state?: number;
  movement_locked?: boolean;
  script_busy?: boolean;
  can_move?: boolean;
  input_blocked_reason?: string | null;
  facing?: Direction;
  interaction_tile?: { x: number; y: number };
  interaction_target?: {
    x: number;
    y: number;
    kind: "npc" | "bg_event";
    label?: string;
    token?: string;
    hotspot_type?: McpMapHotspotType;
    script?: string;
    object_index?: number;
  };
  scene?: {
    active_script?: string;
    scene_owner?: {
      kind: "npc" | "bg_event";
      x?: number;
      y?: number;
      label?: string;
      token?: string;
      hotspot_type?: McpMapHotspotType;
      script?: string;
    };
  };
  current_hotspot?: {
    x: number;
    y: number;
    label?: string;
    token?: string;
    hotspot_type?: McpMapHotspotType;
  };
  interaction_setup?: {
    hotspot: {
      x: number;
      y: number;
      label?: string;
      token?: string;
      hotspot_type?: McpMapHotspotType;
    };
    recommended_approach?: {
      x: number;
      y: number;
      facing: Direction;
    };
  };
  interaction_lane?: {
    hotspot: {
      x: number;
      y: number;
      label?: string;
      token?: string;
      hotspot_type?: McpMapHotspotType;
    };
    lane: {
      x: number;
      y: number;
      facing: Direction;
      facing_aligned: boolean;
      facing_move_leaves_lane: boolean;
      target_confirmed: boolean;
    };
  };
  local_focus?: {
    source:
      | "scene_owner"
      | "interaction_pivot"
      | "current_hotspot"
      | "interaction_setup"
      | "interaction_lane"
      | "interaction_target"
      | "visible_objective";
    target: {
      kind: "npc" | "bg_event";
      x?: number;
      y?: number;
      label?: string;
      token?: string;
      hotspot_type?: McpMapHotspotType;
      script?: string;
    };
    recommended_approach?: {
      x: number;
      y: number;
      facing: "up" | "down" | "left" | "right";
    };
  };
  prompt?: {
    pending: boolean;
    reason?: string;
  };
  dialogue?: {
    waiting_for_input?: boolean;
  };
  coords?: { x: number; y: number };
  map?: string;
  map_details?: McpMapInfoSnapshot;
  flow_state?: McpFlowStateSnapshot | null;
  location_name?: string;
  map_id?: string;
  badges_count?: number;
  money?: number;
  moms_money?: number;
  mom_saving_some_money?: boolean;
  resources?: {
    money: number;
    moms_money: number;
    mom_saving_some_money: boolean;
  };
  party?: {
    count: number;
    lead?: {
      species: string;
      level?: number;
      hp?: number;
      maxHp?: number;
      status?: string;
      };
  };
  party_summary?: {
    count: number;
    lead_species?: string;
    lead_level?: number;
  };
  last_n_events?: Array<{
    action: string;
    frame: number;
    mode?: string;
    summary?: string;
    map?: string;
    coords?: { x: number; y: number };
  }>;
  last_action_result?: ActionResult;
  // Extra diagnostic metadata about the last move/macro (coords before/after, etc.).
  last_mcp_meta?: McpMeta;
};

export type McpPlayerContext = {
  map: string | null;
  coords: { x: number; y: number } | null;
  facing: "up" | "down" | "left" | "right" | "unknown";
  menu_open: boolean;
  dialogue_open: boolean;
  text_advance_pending?: boolean;
};

export type McpActionEvent = {
  frame: number;
  timestamp_ms: number;
  timestamp_iso: string;
  action: string;
  mode?: string;
  map?: string;
  coords?: { x: number; y: number };
  prompt?: string;
  moments?: string[];
  summary?: string;
  result: ActionResult;
};

export type McpRecentEventsSnapshot = {
  recap: string;
  total: number;
  session_started_at_ms: number;
  session_started_at_iso: string;
  time_played_ms: number;
  truncated?: boolean;
  events: McpActionEvent[];
};

export type McpRouteRenderSnapshot = RouteRenderSnapshot;

type SessionMapDataLoader = {
  map_events?: Map<string, { bg_events?: unknown[]; coord_events?: unknown[] }>;
  get_script_event_flags?: (scriptName: string) => string[];
};

const resolveSessionMapDataLoader = (state: {
  [key: string]: unknown;
}): SessionMapDataLoader | null =>
  (state["data_loader"] as SessionMapDataLoader | null) ?? (state["dataLoader"] as SessionMapDataLoader | null) ?? null;

const formatMcpSpeciesLabel = (value: unknown): string => {
  if (typeof value === "string") {
    const normalized = value.trim();
    return normalized.length ? normalized : "UNKNOWN";
  }
  if (value && typeof value === "object") {
    const candidate = value as { id?: unknown; name?: unknown; species?: unknown };
    for (const entry of [candidate.id, candidate.name, candidate.species]) {
      if (typeof entry !== "string") {
        continue;
      }
      const normalized = entry.trim();
      if (normalized.length) {
        return normalized;
      }
    }
  }
  return "UNKNOWN";
};

const normalizeQuotedToken = (value: string | undefined): string =>
  (value ?? "").trim().replace(/^"+|"+$/g, "");

type ScheduledEvent = {
  frame: number;
  event: InstanceType<typeof gameEngine.event.Event>;
};

type MacroAction =
  | {
      type: "move";
      value: Direction;
      times?: number;
      hold_frames?: number;
      delay_frames?: number;
    }
  | {
      type: "button";
      value: Button;
      times?: number;
      hold_frames?: number;
      delay_frames?: number;
    }
  | {
      type: "wait";
      frames?: number;
      delay_frames?: number;
    };

type MacroOptions = {
  delay_frames?: number;
  stop_on_event?: boolean;
};

type NamedMacroName = "advance_dialog" | "mash_a";

type NamedMacroOptions = {
  maxPresses?: number;
  settleFrames?: number;
};

type NamedMacroReasonCode =
  | "advanced"
  | "busy_wait"
  | "closed_menu"
  | "nudged_choice"
  | "prompt_opened"
  | "no_effect";

type MapIdentity = {
  name: string;
  group: number | null;
  number: number | null;
};

type LoadedSavePlayerTile = {
  x: number;
  y: number;
};

type AuthoritativeLoadedSaveState = {
  map: string | null;
  player: LoadedSavePlayerTile | null;
};

type IdentityPlayProfile = {
  playerName: string | null;
  playerGender: PlayerGender | null;
};

type ModalUiState = {
  in_battle: boolean;
  in_menu: boolean;
  in_dialog: boolean;
  text_box_open: boolean;
  text_advance_pending: boolean;
  prompt_pending: boolean;
  input_capture_active: boolean;
  movement_locked: boolean;
  script_busy: boolean;
  input_blocked_reason: string | null;
  can_move: boolean;
};

type DialogueUiState = {
  visible: boolean;
  waiting_for_input: boolean;
  pending_waits: number;
  yes_no_prompt_open: boolean;
  text_advance_pending: boolean;
  input_owned: boolean;
  dialog_active: boolean;
  text_box_open: boolean;
};

type SceneSignal = {
  mode: string;
  menu: boolean;
  map?: string;
  promptReason: string | null;
  textAdvancePending?: boolean;
  viewportText: string;
  menuText: string;
  dialogueText: string;
  promptText: string;
  markerText: string;
};

type MoveOutcome = {
  requested: number;
  completed: number;
  start: [number, number];
  end: [number, number];
  map: string;
  blocked: boolean;
  blockReason: string | null;
  stopReason: string | null;
};

type RuntimeSnapshotFrame = {
  name: string;
  index: number;
  allowFallthrough?: boolean;
  parent?: string;
};

type RuntimeSnapshotJsonValue =
  | string
  | number
  | boolean
  | null
  | RuntimeSnapshotJsonValue[]
  | { [key: string]: RuntimeSnapshotJsonValue };

type RuntimeSnapshotNpc = {
  objectIndex: number;
  x: number;
  y: number;
  prevX: number;
  prevY: number;
  direction: string;
};

type RuntimeSnapshot = {
  version: 1;
  frameCounter: number;
  sessionStartedAtMs?: number;
  actionEventTotal?: number;
  map: string | null;
  player: {
    x: number;
    y: number;
    prevX: number;
    prevY: number;
    direction: string | null;
  } | null;
  npcs: RuntimeSnapshotNpc[];
  runner: {
    stack: RuntimeSnapshotFrame[];
    awaitingResume: number;
    queuedOverworldTasks: number;
    stopExecution: boolean;
    lastYesNoResult: boolean;
    lastConditionResult: boolean;
    pendingReloadMap: string | null;
    lastInteractionObjectIndex: number | null;
    variables?: Record<string, RuntimeSnapshotJsonValue>;
  } | null;
  dialogue: {
    visible: boolean;
    waitingForInput: boolean;
    scriptPaused: boolean;
    pendingWaits: number;
    pendingScriptWaits: number;
    currentText: string;
    pendingText: string[];
    autoCloseRequested: boolean;
    ignoreConfirmUntilRelease: boolean;
    yesNoSelection: number | null;
  } | null;
  actionEvents?: Array<{
    frame: number;
    timestamp_ms: number;
    timestamp_iso: string;
    action: string;
    mode?: string;
    map?: string;
    coords?: { x: number; y: number };
    prompt?: string;
    moments?: string[];
    summary?: string;
    result: ActionResult;
  }>;
};

const isValidRuntimeSnapshotPlayer = (
  player: RuntimeSnapshot["player"] | null | undefined
): player is NonNullable<RuntimeSnapshot["player"]> =>
  Boolean(
    player &&
      Number.isFinite(player.x) &&
      Number.isFinite(player.y) &&
      Number.isFinite(player.prevX) &&
      Number.isFinite(player.prevY) &&
      player.x >= 0 &&
      player.y >= 0 &&
      player.prevX >= 0 &&
      player.prevY >= 0
  );

const isValidRuntimeSnapshotNpc = (
  npc: RuntimeSnapshotNpc | null | undefined
): npc is RuntimeSnapshotNpc =>
  Boolean(
    npc &&
      Number.isFinite(npc.objectIndex) &&
      Number.isFinite(npc.x) &&
      Number.isFinite(npc.y) &&
      Number.isFinite(npc.prevX) &&
      Number.isFinite(npc.prevY) &&
      typeof npc.direction === "string" &&
      npc.objectIndex > 0 &&
      npc.x >= 0 &&
      npc.y >= 0 &&
      npc.prevX >= 0 &&
      npc.prevY >= 0
  );

const mergeRuntimeSnapshotWithPrevious = (
  previous: RuntimeSnapshot | null,
  next: RuntimeSnapshot
): RuntimeSnapshot => {
  if (!previous || previous.version !== 1 || previous.map !== next.map) {
    return next;
  }
  const merged: RuntimeSnapshot = { ...next };
  if (!isValidRuntimeSnapshotPlayer(merged.player) && isValidRuntimeSnapshotPlayer(previous.player)) {
    merged.player = { ...previous.player };
  }
  if (
    (!Array.isArray(merged.npcs) || merged.npcs.length === 0) &&
    Array.isArray(previous.npcs) &&
    previous.npcs.some((npc) => isValidRuntimeSnapshotNpc(npc))
  ) {
    merged.npcs = previous.npcs.filter((npc): npc is RuntimeSnapshotNpc => isValidRuntimeSnapshotNpc(npc));
  }
  return merged;
};

const ACTION_LOG_LIMIT = 64;
const ACTION_EVENT_LOG_LIMIT = 256;
const APPROACH_HISTORY_WINDOW = 32;
const RECENT_INERT_OBJECTIVE_PIVOT_RADIUS = 4;
const RUNTIME_SNAPSHOT_ACTION_EVENT_LIMIT = 24;
const RECENT_EVENT_LIMIT = 50;
const EVENT_STRING_MAX_LEN = 96;
const EVENT_SUMMARY_MAX_LEN = 160;
const EVENT_MOMENTS_MAX = 5;
const EVENT_RESULT_EVENTS_MAX = 5;
const EVENT_RECAP_MAX_LEN = 240;
const MOVEMENT_LOCK_RECOVERY_MAX_FRAMES = 12;
const POST_MOVE_SETTLE_MAX_FRAMES = 2;
const DIALOG_MACRO_BUSY_BACKOFF_FRAMES = [1, 2, 3, 5, 8];
const DIALOG_MACRO_MIN_PRESS_INTERVAL_FRAMES = 3;
const MACRO_TRACE_ACTION_LIMIT = 20;
const MACRO_TRACE_STEP_LIMIT = 20;
const DEFAULT_MCP_INSTANT_MODE = true;
const normalizeRuntimeSnapshotSessionStem = (sessionId: string): string => {
  const trimmed = String(sessionId ?? "").trim();
  if (!trimmed) {
    return sessionId;
  }
  const normalized = trimmed.replace(/(?:[-_]?runtime)+$/i, "");
  return normalized || trimmed;
};

const resolveRuntimeSnapshotSlot = (sessionId: string): string =>
  path.resolve(
    process.cwd(),
    `mcp-${normalizeRuntimeSnapshotSessionStem(sessionId)}-runtime.json`,
  );

const resolveLegacyRuntimeSnapshotSlot = (sessionId: string): string =>
  path.resolve(process.cwd(), `mcp-${sessionId}-runtime.json`);

const isNonFatalRuntimeSnapshotPersistenceError = (error: unknown): boolean => {
  const code =
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof (error as { code?: unknown }).code === "string"
      ? (error as { code: string }).code.toUpperCase()
      : "";
  if (code === "EROFS" || code === "EACCES" || code === "EPERM") {
    return true;
  }
  const message = error instanceof Error ? error.message.toLowerCase() : String(error).toLowerCase();
  return message.includes("read-only") || message.includes("permission denied");
};

const isNonFatalAutosavePersistenceError = (error: unknown): boolean =>
  error instanceof SaveGameError && !(error instanceof SaveGameValidationError);

type RuntimeSnapshotLoadResult = {
  slot: string;
  snapshot: RuntimeSnapshot;
};

const readStableRuntimeCoords = (
  overworld: {
    player_x?: number;
    player_y?: number;
    prev_player_x?: number;
    prev_player_y?: number;
    player_object?: {
      x?: number;
      y?: number;
      direction?: string;
    } | null;
  } | null | undefined,
  gameState: {
    wram?: {
      wXCoord?: number;
      wYCoord?: number;
    };
  } | null | undefined,
): {
  player: { x: number; y: number } | null;
  previous: { x: number; y: number } | null;
} => {
  const candidates = [
    { x: overworld?.player_object?.x, y: overworld?.player_object?.y },
    { x: overworld?.player_x, y: overworld?.player_y },
    { x: gameState?.wram?.wXCoord, y: gameState?.wram?.wYCoord },
    { x: overworld?.prev_player_x, y: overworld?.prev_player_y },
  ];
  const pick = (entries: Array<{ x?: number; y?: number }>): { x: number; y: number } | null => {
    for (const entry of entries) {
      if (
        typeof entry.x === "number" &&
        typeof entry.y === "number" &&
        Number.isFinite(entry.x) &&
        Number.isFinite(entry.y) &&
        entry.x >= 0 &&
        entry.y >= 0
      ) {
        return { x: entry.x, y: entry.y };
      }
    }
    return null;
  };
  const player = pick(candidates);
  const previous = pick([
    { x: overworld?.prev_player_x, y: overworld?.prev_player_y },
    { x: overworld?.player_object?.x, y: overworld?.player_object?.y },
    { x: overworld?.player_x, y: overworld?.player_y },
    { x: gameState?.wram?.wXCoord, y: gameState?.wram?.wYCoord },
  ]);
  return { player, previous };
};

const toRuntimeSnapshotJsonValue = (
  value: unknown,
  depth = 0,
): RuntimeSnapshotJsonValue | undefined => {
  if (value === null) {
    return null;
  }
  if (typeof value === "string" || typeof value === "boolean") {
    return value;
  }
  if (typeof value === "number") {
    return Number.isFinite(value) ? value : undefined;
  }
  if (depth >= 4) {
    return undefined;
  }
  if (Array.isArray(value)) {
    const entries = value
      .map((entry) => toRuntimeSnapshotJsonValue(entry, depth + 1))
      .filter((entry): entry is RuntimeSnapshotJsonValue => entry !== undefined);
    return entries;
  }
  if (typeof value === "object") {
    const output: Record<string, RuntimeSnapshotJsonValue> = {};
    for (const [key, entry] of Object.entries(value as Record<string, unknown>)) {
      const serialized = toRuntimeSnapshotJsonValue(entry, depth + 1);
      if (serialized !== undefined) {
        output[key] = serialized;
      }
    }
    return output;
  }
  return undefined;
};

const serializeRuntimeSnapshotVariables = (
  variables: Record<string, unknown> | undefined,
): Record<string, RuntimeSnapshotJsonValue> | undefined => {
  if (!variables || typeof variables !== "object") {
    return undefined;
  }
  const serialized = toRuntimeSnapshotJsonValue(variables);
  if (!serialized || Array.isArray(serialized) || typeof serialized !== "object") {
    return undefined;
  }
  return Object.keys(serialized).length > 0 ? serialized : undefined;
};

const serializeRuntimeSnapshot = (
  frameCounter: number,
  game: Pick<Game, "getMapName" | "getOverworld" | "getGameState">,
  actionEvents: McpActionEvent[] = [],
  metadata: {
    sessionStartedAtMs: number;
    actionEventTotal: number;
  } = {
    sessionStartedAtMs: Date.now(),
    actionEventTotal: actionEvents.length,
  }
): RuntimeSnapshot | null => {
  const overworld = game.getOverworld() as {
    current_map_name?: string;
    player_x?: number;
    player_y?: number;
    prev_player_x?: number;
    prev_player_y?: number;
    player_object?: {
      x?: number;
      y?: number;
      direction?: string;
    } | null;
    player_direction?: string;
    npcs?: Array<{
      objectIndex?: number;
      x?: number;
      y?: number;
      prevX?: number;
      prevY?: number;
      direction?: string;
    }>;
    dialogue?: {
      visible?: boolean;
      waiting_for_input?: boolean;
      is_script_paused?: boolean;
      pending_waits?: number;
      pending_text?: string[];
      current_text?: string;
      pending_script_waits_count?: number;
      _yes_no_prompt?: { selection?: number } | null;
      auto_close_requested?: boolean;
      ignore_confirm_until_release?: boolean;
    } | null;
    script_runner?: {
      _script_stack?: Array<{
        name?: string;
        index?: number;
        allowFallthrough?: boolean;
        parent?: string;
      }>;
      _awaiting_resume?: number;
      _queued_overworld_task_count?: number;
      stop_execution?: boolean;
      last_yes_no_result?: boolean;
      last_condition_result?: boolean;
      pending_reload_map?: string | null;
      last_interaction_object_index?: number | null;
      variables?: Record<string, unknown>;
    } | null;
  };

  if (!overworld) {
    return null;
  }
  const stableCoords = readStableRuntimeCoords(
    overworld,
    (game as { getGameState?: () => unknown }).getGameState?.() as
      | {
          wram?: { wXCoord?: number; wYCoord?: number };
        }
      | undefined
  );

  const npcs = Array.isArray(overworld.npcs)
    ? overworld.npcs
        .map((npc) => {
          const objectIndex =
            typeof npc.objectIndex === "number" && Number.isFinite(npc.objectIndex)
              ? npc.objectIndex
              : null;
          const x = typeof npc.x === "number" && Number.isFinite(npc.x) ? npc.x : null;
          const y = typeof npc.y === "number" && Number.isFinite(npc.y) ? npc.y : null;
          const prevXCandidate =
            typeof npc.prevX === "number" && Number.isFinite(npc.prevX)
              ? npc.prevX
              : x;
          const prevYCandidate =
            typeof npc.prevY === "number" && Number.isFinite(npc.prevY)
              ? npc.prevY
              : y;
          const direction =
            typeof npc.direction === "string" && npc.direction.trim().length > 0
              ? npc.direction
              : null;
          if (
            objectIndex === null ||
            x === null ||
            y === null ||
            prevXCandidate === null ||
            prevYCandidate === null ||
            direction === null
          ) {
            return null;
          }
          return {
            objectIndex,
            x,
            y,
            prevX: prevXCandidate,
            prevY: prevYCandidate,
            direction,
          };
        })
        .filter((entry): entry is RuntimeSnapshotNpc => isValidRuntimeSnapshotNpc(entry))
    : [];

  return {
    version: 1,
    frameCounter,
    sessionStartedAtMs: metadata.sessionStartedAtMs,
    actionEventTotal: metadata.actionEventTotal,
    map: overworld.current_map_name ?? game.getMapName?.() ?? null,
    player: stableCoords.player
      ? {
          x: stableCoords.player.x,
          y: stableCoords.player.y,
          prevX: stableCoords.previous?.x ?? stableCoords.player.x,
          prevY: stableCoords.previous?.y ?? stableCoords.player.y,
          direction:
            typeof overworld.player_object?.direction === "string"
              ? overworld.player_object.direction
              : typeof overworld.player_direction === "string"
                ? overworld.player_direction
                : null,
        }
      : null,
    npcs,
    runner: overworld.script_runner
      ? {
          stack: Array.isArray(overworld.script_runner._script_stack)
            ? overworld.script_runner._script_stack.reduce<RuntimeSnapshotFrame[]>((frames, frame) => {
                const name = String(frame.name ?? "").trim();
                if (!name) {
                  return frames;
                }
                frames.push({
                  name,
                  index: typeof frame.index === "number" ? frame.index : 0,
                  allowFallthrough: frame.allowFallthrough === true,
                  parent: frame.parent ? String(frame.parent) : undefined,
                });
                return frames;
              }, [])
            : [],
          awaitingResume: Math.max(0, Number(overworld.script_runner._awaiting_resume ?? 0)),
          queuedOverworldTasks: Math.max(0, Number(overworld.script_runner._queued_overworld_task_count ?? 0)),
          stopExecution: Boolean(overworld.script_runner.stop_execution),
          lastYesNoResult: Boolean(overworld.script_runner.last_yes_no_result),
          lastConditionResult: Boolean(overworld.script_runner.last_condition_result),
          pendingReloadMap:
            typeof overworld.script_runner.pending_reload_map === "string"
              ? overworld.script_runner.pending_reload_map
              : null,
          lastInteractionObjectIndex:
            typeof overworld.script_runner.last_interaction_object_index === "number"
              ? overworld.script_runner.last_interaction_object_index
              : null,
          variables: serializeRuntimeSnapshotVariables(overworld.script_runner.variables),
        }
      : null,
    dialogue: overworld.dialogue
      ? {
          visible: Boolean(overworld.dialogue.visible),
          waitingForInput: Boolean(overworld.dialogue.waiting_for_input),
          scriptPaused: Boolean(overworld.dialogue.is_script_paused),
          pendingWaits: Math.max(0, Number(overworld.dialogue.pending_waits ?? 0)),
          pendingScriptWaits: Math.max(0, Number(overworld.dialogue.pending_script_waits_count ?? 0)),
          currentText: String(overworld.dialogue.current_text ?? ""),
          pendingText: Array.isArray(overworld.dialogue.pending_text)
            ? overworld.dialogue.pending_text.map((entry) => String(entry))
            : [],
          autoCloseRequested: Boolean(overworld.dialogue.auto_close_requested),
          ignoreConfirmUntilRelease: Boolean(overworld.dialogue.ignore_confirm_until_release),
          yesNoSelection:
            typeof overworld.dialogue._yes_no_prompt?.selection === "number"
              ? overworld.dialogue._yes_no_prompt.selection
              : null,
        }
      : null,
    actionEvents: actionEvents
      .slice(-RUNTIME_SNAPSHOT_ACTION_EVENT_LIMIT)
      .map((event) => ({
        frame: event.frame,
        timestamp_ms: event.timestamp_ms,
        timestamp_iso: event.timestamp_iso,
        action: event.action,
        mode: event.mode,
        map: event.map,
        coords: event.coords ? { ...event.coords } : undefined,
        prompt: event.prompt,
        moments: event.moments ? [...event.moments] : undefined,
        summary: event.summary,
        result: {
          ok: event.result.ok,
          changed: event.result.changed,
          reason: event.result.reason,
          events: event.result.events ? [...event.result.events] : undefined,
        },
      })),
  };
};

const normalizeRuntimeSnapshotActionEvents = (
  events: RuntimeSnapshot["actionEvents"]
): McpActionEvent[] => {
  if (!Array.isArray(events)) {
    return [];
  }
  const normalized = events
    .map<McpActionEvent | null>((event) => {
      if (
        !event ||
        typeof event.frame !== "number" ||
        !Number.isFinite(event.frame) ||
        typeof event.timestamp_ms !== "number" ||
        !Number.isFinite(event.timestamp_ms) ||
        typeof event.timestamp_iso !== "string" ||
        typeof event.action !== "string" ||
        !event.result ||
        typeof event.result.ok !== "boolean" ||
        typeof event.result.changed !== "boolean"
      ) {
        return null;
      }
      const mode = typeof event.mode === "string" && event.mode.trim() ? event.mode : undefined;
      const reason =
        event.result.reason === "blocked" ||
        event.result.reason === "no_change" ||
        event.result.reason === "menu" ||
        event.result.reason === "busy" ||
        event.result.reason === "unknown"
          ? event.result.reason
          : undefined;
      return {
        frame: event.frame,
        timestamp_ms: event.timestamp_ms,
        timestamp_iso: event.timestamp_iso,
        action: event.action,
        mode,
        map: typeof event.map === "string" ? event.map : undefined,
        coords:
          typeof event.coords?.x === "number" && typeof event.coords?.y === "number"
            ? { x: event.coords.x, y: event.coords.y }
            : undefined,
        prompt: typeof event.prompt === "string" ? event.prompt : undefined,
        moments: Array.isArray(event.moments)
          ? event.moments.filter((entry): entry is string => typeof entry === "string").slice(0, EVENT_MOMENTS_MAX)
          : undefined,
        summary: typeof event.summary === "string" ? event.summary : undefined,
        result: {
          ok: event.result.ok,
          changed: event.result.changed,
          reason,
          events: Array.isArray(event.result.events)
            ? event.result.events
                .filter((entry): entry is string => typeof entry === "string")
                .slice(0, EVENT_RESULT_EVENTS_MAX)
            : undefined,
        },
      };
    })
    .filter((event): event is McpActionEvent => event !== null);
  return normalized.slice(-RUNTIME_SNAPSHOT_ACTION_EVENT_LIMIT);
};

type ApproachRecommendationContext = {
  coords: { x: number; y: number };
  hotspotType?: McpMapHotspotType;
  mapName?: string;
  preferLateralNpcRecovery?: boolean;
  allowSpentNpcRecovery?: boolean;
  avoidImmediateBacktrackCoords?: { x: number; y: number };
};

const applyRuntimeSnapshot = (
  game: Pick<Game, "getMapName" | "getOverworld">,
  snapshot: RuntimeSnapshot
): boolean => {
  if (snapshot.version !== 1) {
    return false;
  }
  const overworld = game.getOverworld() as {
    current_map_name?: string;
    player_x?: number;
    player_y?: number;
    prev_player_x?: number;
    prev_player_y?: number;
    player_direction?: string;
    target_tile_x?: number;
    target_tile_y?: number;
    is_moving?: boolean;
    step_progress_px?: number;
    step_dx_px?: number;
    step_dy_px?: number;
    _queued_direction?: string | null;
    _turning_direction?: string | null;
    _turn_frames_remaining?: number;
    _turn_should_force_step?: boolean;
    _pending_auto_step?: string | null;
    _sync_player_state?: () => void;
    player_object?: {
      x?: number;
      y?: number;
      prevX?: number;
      prevY?: number;
      direction?: string;
      updatePixelPosition?: () => void;
    } | null;
    _script_task_queue?: unknown[];
    _active_script_task?: unknown | null;
    _blocking_task_count?: number;
    _blocking_movement_lock_active?: boolean;
    _movement_lock_count?: number;
    _text_lock_active?: boolean;
    _clear_stale_blocking_tasks?: () => void;
    reset_input_state?: () => void;
    _ignore_a_until_release?: boolean;
    input_capture_active?: boolean;
    npcs?: Array<{
      objectIndex?: number;
      x?: number;
      y?: number;
      prevX?: number;
      prevY?: number;
      direction?: string;
      updatePixelPosition?: () => void;
    }>;
    script_runner?: {
      dataLoader?: { get_script: (name: string, parent?: string) => unknown[] | null };
      parse?: (scriptData: unknown[]) => unknown[];
      _script_stack?: unknown[];
      _awaiting_resume?: number;
      _queued_overworld_task_count?: number;
      stop_execution?: boolean;
      _pause_execution?: boolean;
      last_yes_no_result?: boolean;
      last_condition_result?: boolean;
      pending_reload_map?: string | null;
      last_interaction_object_index?: number | null;
      variables?: Record<string, RuntimeSnapshotJsonValue>;
    } | null;
    dialogue?: {
      ui?: ConstructorParameters<typeof YesNoPrompt>[0];
      window?: { clear?: () => void; open?: (text: string) => void; complete?: () => void };
      forceCloseText?: () => void;
      resume?: () => void;
      visible?: boolean;
      waiting_for_input?: boolean;
      script_paused?: boolean;
      pendingWaits?: number;
      pending_script_waits?: number;
      current_text?: string;
      pending_text?: string[];
      auto_close_requested?: boolean;
      ignore_confirm_until_release?: boolean;
      pending_yes_no_request?: boolean;
      pending_yes_no_callback?: ((result: boolean) => void) | null;
      yes_no_callback?: ((result: boolean) => void) | null;
      suspended?: boolean;
      _suppress_orphan_close?: boolean;
      yes_no_prompt?: YesNoPrompt | null;
    } | null;
  };

  const currentMap = overworld?.current_map_name ?? game.getMapName?.() ?? null;
  if (!overworld || (snapshot.map && currentMap && snapshot.map !== currentMap)) {
    return false;
  }

  if (snapshot.player) {
    overworld.player_x = snapshot.player.x;
    overworld.player_y = snapshot.player.y;
    overworld.prev_player_x = snapshot.player.prevX;
    overworld.prev_player_y = snapshot.player.prevY;
    if (snapshot.player.direction) {
      overworld.player_direction = snapshot.player.direction;
    }
    // MCP autosaves happen at completed input boundaries, so resumed player movement
    // should restart from a settled tile even if boot-time scripts queued stale tasks.
    overworld.target_tile_x = snapshot.player.x;
    overworld.target_tile_y = snapshot.player.y;
    overworld.is_moving = false;
    overworld.step_progress_px = 0;
    overworld.step_dx_px = 0;
    overworld.step_dy_px = 0;
    overworld._queued_direction = null;
    overworld._turning_direction = null;
    overworld._turn_frames_remaining = 0;
    overworld._turn_should_force_step = false;
    overworld._pending_auto_step = null;
    overworld._sync_player_state?.();

    const playerObject = overworld.player_object ?? null;
    if (playerObject) {
      playerObject.x = snapshot.player.x;
      playerObject.y = snapshot.player.y;
      playerObject.prevX = snapshot.player.prevX;
      playerObject.prevY = snapshot.player.prevY;
      if (snapshot.player.direction) {
        playerObject.direction = snapshot.player.direction;
      }
      playerObject.updatePixelPosition?.();
    }

    const primeActiveWarpTile =
      (overworld as {
        _prime_active_warp_tile_for_current_position?: () => void;
      })._prime_active_warp_tile_for_current_position;
    if (typeof primeActiveWarpTile === "function") {
      primeActiveWarpTile.call(overworld);
    }
  }

  // MCP autosaves span separate process lifetimes, so edge-trigger release gates
  // cannot survive restore; the matching keyup will never arrive in the resumed process.
  overworld._ignore_a_until_release = false;

  if (Array.isArray(overworld.npcs) && snapshot.npcs.length > 0) {
    const byIndex = new Map(snapshot.npcs.map((npc) => [npc.objectIndex, npc]));
    for (const npc of overworld.npcs) {
      const restored = byIndex.get(npc.objectIndex ?? -1);
      if (!restored) {
        continue;
      }
      npc.x = restored.x;
      npc.y = restored.y;
      npc.prevX = restored.prevX;
      npc.prevY = restored.prevY;
      npc.direction = restored.direction;
      npc.updatePixelPosition?.();
    }
  }

  if (
    overworld.script_runner &&
    snapshot.runner &&
    Math.max(0, Number(snapshot.runner.queuedOverworldTasks ?? 0)) === 0
  ) {
    const runner = overworld.script_runner;
    const restoredFrames = snapshot.runner.stack.reduce<
      Array<{ name: string; commands: unknown[]; index: number; allowFallthrough?: boolean; parent?: string }>
    >((frames, frame) => {
        const scriptData = runner.dataLoader?.get_script(frame.name, frame.parent);
        if (!Array.isArray(scriptData) || typeof runner.parse !== "function") {
          return frames;
        }
        const commands = runner.parse(scriptData);
        if (!Array.isArray(commands)) {
          return frames;
        }
        frames.push({
          name: frame.name,
          commands,
          index: Math.max(0, Math.min(frame.index, commands.length)),
          allowFallthrough: frame.allowFallthrough === true,
          parent: frame.parent,
        });
        return frames;
      }, []);

    runner._script_stack = restoredFrames;
    runner._awaiting_resume = snapshot.runner.awaitingResume;
    runner._queued_overworld_task_count = 0;
    runner.stop_execution = snapshot.runner.stopExecution;
    runner._pause_execution = snapshot.runner.awaitingResume > 0 || snapshot.runner.stopExecution;
    runner.last_yes_no_result = snapshot.runner.lastYesNoResult;
    runner.last_condition_result = snapshot.runner.lastConditionResult;
    runner.pending_reload_map = snapshot.runner.pendingReloadMap;
    runner.last_interaction_object_index = snapshot.runner.lastInteractionObjectIndex;
    runner.variables = { ...(snapshot.runner.variables ?? {}) };
  } else if (overworld.script_runner) {
    const runner = overworld.script_runner;
    runner._script_stack = [];
    runner._awaiting_resume = 0;
    runner._queued_overworld_task_count = 0;
    runner.stop_execution = false;
    runner._pause_execution = false;
    runner.pending_reload_map = null;
    runner.last_interaction_object_index = null;
    runner.variables = {};
  }

  overworld.reset_input_state?.();
  overworld._active_script_task = null;
  overworld._script_task_queue = [];
  overworld._blocking_task_count = 0;
  overworld._blocking_movement_lock_active = false;
  overworld._movement_lock_count = 0;
  overworld._text_lock_active = false;
  overworld.input_capture_active = false;
  overworld._clear_stale_blocking_tasks?.();

  if (overworld.dialogue) {
    const dialogue = overworld.dialogue;
    dialogue.resume?.();
    dialogue.forceCloseText?.();
    dialogue.window?.clear?.();
    dialogue.visible = false;
    dialogue.waiting_for_input = false;
    dialogue.script_paused = false;
    dialogue.pendingWaits = 0;
    dialogue.pending_script_waits = 0;
    dialogue.current_text = "";
    dialogue.pending_text = [];
    dialogue.auto_close_requested = false;
    dialogue.ignore_confirm_until_release = false;
    dialogue.pending_yes_no_request = false;
    dialogue.pending_yes_no_callback = null;
    dialogue.yes_no_callback = null;
    dialogue.suspended = false;
    dialogue._suppress_orphan_close = false;
    dialogue.yes_no_prompt = null;
    if (snapshot.dialogue?.currentText) {
      dialogue.window?.open?.(snapshot.dialogue.currentText);
      dialogue.window?.complete?.();
    }
    if (snapshot.dialogue) {
      dialogue.visible =
        snapshot.dialogue.visible || snapshot.dialogue.waitingForInput || snapshot.dialogue.pendingWaits > 0;
      dialogue.waiting_for_input = snapshot.dialogue.waitingForInput;
      dialogue.script_paused = snapshot.dialogue.scriptPaused;
      dialogue.pendingWaits = snapshot.dialogue.pendingWaits;
      dialogue.pending_script_waits = snapshot.dialogue.pendingScriptWaits;
      dialogue.current_text = snapshot.dialogue.currentText;
      dialogue.pending_text = [...snapshot.dialogue.pendingText];
      dialogue.auto_close_requested = snapshot.dialogue.autoCloseRequested;
    }
    const yesNoSelection = snapshot.dialogue?.yesNoSelection ?? null;
    if (yesNoSelection !== null && dialogue.ui) {
      dialogue.yes_no_prompt = new YesNoPrompt(dialogue.ui, null);
      dialogue.yes_no_prompt.selection = yesNoSelection;
    }
  }

  return true;
};

const keyForDirection = (direction: Direction): string => {
  switch (direction) {
    case "up":
      return gameEngine.K_UP;
    case "down":
      return gameEngine.K_DOWN;
    case "left":
      return gameEngine.K_LEFT;
    case "right":
      return gameEngine.K_RIGHT;
  }
};

const keyForButton = (button: Button): string => {
  if (button === "up" || button === "down" || button === "left" || button === "right") {
    return keyForDirection(button);
  }
  switch (button) {
    case "a":
      return defaultKeyBindings[GameButton.A][0] ?? "KeyZ";
    case "b":
      return defaultKeyBindings[GameButton.B][0] ?? "KeyX";
    case "start":
      return defaultKeyBindings[GameButton.Start][0] ?? gameEngine.K_RETURN;
    case "select":
      return defaultKeyBindings[GameButton.Select][0] ?? "Backspace";
  }
};

const oppositeDirection = (direction: Direction): Direction | null => {
  switch (direction) {
    case "up":
      return "down";
    case "down":
      return "up";
    case "left":
      return "right";
    case "right":
      return "left";
  }
};

const formatActionLabel = (kind: "move" | "button", value: string, source = "mcp"): string =>
  `${source}:${kind}:${value}`;

const normalizeIdentityPlayerName = (name: string | null | undefined): string | null => {
  const trimmed = String(name ?? "").trim();
  if (!trimmed) {
    return null;
  }
  const sanitized = trimmed.replace(/[^A-Za-z0-9 .'\-]/g, "").trim();
  if (!sanitized) {
    return null;
  }
  return sanitized.slice(0, 10);
};

const resolveSessionPlayerName = (
  identityProfileName: string | null | undefined,
  identityContextName: string | null | undefined,
  currentName: string | null | undefined
): string | null => {
  const normalizedCurrent = String(currentName ?? "").trim();
  if (normalizedCurrent && normalizedCurrent !== "?????") {
    return normalizedCurrent;
  }
  return (
    normalizeIdentityPlayerName(identityProfileName) ??
    normalizeIdentityPlayerName(identityContextName) ??
    normalizeIdentityPlayerName(getSettings().mcpPlayerName) ??
    null
  );
};

const normalizePlayerGender = (value: unknown): PlayerGender | null => {
  if (value === PlayerGender.MALE || value === PlayerGender.FEMALE) {
    return value;
  }
  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    if (normalized === "0" || normalized === "male") {
      return PlayerGender.MALE;
    }
    if (normalized === "1" || normalized === "female") {
      return PlayerGender.FEMALE;
    }
  }
  if (typeof value === "number") {
    if (value === PlayerGender.MALE || value === PlayerGender.FEMALE) {
      return value;
    }
  }
  return null;
};

const isIgnorablePlaySettingsError = (error: unknown): boolean => {
  if (!error || typeof error !== "object") {
    return false;
  }
  const candidate = error as { code?: unknown; message?: unknown };
  const code = String(candidate.code ?? "").toUpperCase();
  const message = String(candidate.message ?? "").toLowerCase();
  return (
    code === "PGRST116" ||
    code === "PGRST205" ||
    code === "42P01" ||
    message.includes("0 rows") ||
    message.includes("play_user_settings")
  );
};

const loadIdentityPlayProfile = async (
  playerId: string | null | undefined,
  createClient: typeof createSupabaseServiceRoleClient = createSupabaseServiceRoleClient
): Promise<IdentityPlayProfile | null> => {
  const normalizedPlayerId = String(playerId ?? "").trim();
  if (!normalizedPlayerId) {
    return null;
  }
  const supabase = createClient();
  if (!supabase) {
    return null;
  }
  const { data, error } = await supabase
    .from("play_user_settings")
    .select("player_name, player_gender")
    .eq("user_id", normalizedPlayerId)
    .maybeSingle();
  if (error) {
    if (isIgnorablePlaySettingsError(error)) {
      return null;
    }
    throw new Error(error.message || "Failed to load MCP play settings.");
  }
  if (!data) {
    return null;
  }
  return {
    playerName: normalizeIdentityPlayerName(
      typeof data.player_name === "string" ? data.player_name : null
    ),
    playerGender: normalizePlayerGender(data.player_gender),
  };
};

const resolveSessionPlayerGender = (
  profile: IdentityPlayProfile | null,
  gameState:
    | {
        sram?: { player_gender?: unknown };
        wram?: { player_gender?: unknown };
      }
    | null
    | undefined
): PlayerGender | null =>
  normalizePlayerGender(profile?.playerGender) ??
  normalizePlayerGender(gameState?.sram?.player_gender) ??
  normalizePlayerGender(gameState?.wram?.player_gender);

const isValidLoadedSaveCoord = (value: unknown): value is number =>
  typeof value === "number" && Number.isFinite(value) && value >= 0;

const formatLoadedSaveCoords = (coords: LoadedSavePlayerTile | null): string =>
  coords ? `${coords.x},${coords.y}` : "unknown";

const readAuthoritativeLoadedSaveState = (
  game: Pick<Game, "getGameState" | "getMapName" | "getOverworld">
): AuthoritativeLoadedSaveState => {
  const overworld = game.getOverworld() as
    | {
        current_map_name?: string | null;
        player_x?: number;
        player_y?: number;
        player_object?: {
          x?: number;
          y?: number;
        } | null;
      }
    | undefined;
  const gameState = game.getGameState?.() as
    | {
        wram?: {
          wXCoord?: number;
          wYCoord?: number;
        };
      }
    | null
    | undefined;

  const playerObject = overworld?.player_object ?? null;
  const player =
    playerObject && isValidLoadedSaveCoord(playerObject.x) && isValidLoadedSaveCoord(playerObject.y)
      ? { x: playerObject.x, y: playerObject.y }
      : isValidLoadedSaveCoord(overworld?.player_x) && isValidLoadedSaveCoord(overworld?.player_y)
        ? { x: overworld.player_x, y: overworld.player_y }
        : isValidLoadedSaveCoord(gameState?.wram?.wXCoord) && isValidLoadedSaveCoord(gameState?.wram?.wYCoord)
          ? { x: gameState.wram.wXCoord, y: gameState.wram.wYCoord }
          : null;

  return {
    map: overworld?.current_map_name ?? game.getMapName?.() ?? null,
    player,
  };
};

const getRuntimeSnapshotStaleReason = (
  game: Pick<Game, "getGameState" | "getMapName" | "getOverworld">,
  snapshot: RuntimeSnapshot
): string | null => {
  const loadedSave = readAuthoritativeLoadedSaveState(game);
  if (snapshot.map && loadedSave.map && snapshot.map !== loadedSave.map) {
    return `runtime map ${snapshot.map} does not match live map ${loadedSave.map}.`;
  }
  if (
    snapshot.player &&
    loadedSave.player &&
    (snapshot.player.x !== loadedSave.player.x || snapshot.player.y !== loadedSave.player.y)
  ) {
    return `runtime coords ${snapshot.player.x},${snapshot.player.y} do not match live coords ${formatLoadedSaveCoords(loadedSave.player)}.`;
  }
  return null;
};

const formatWhiteoutAsmText = (playerName: string | null | undefined): string => {
  const normalizedName = String(playerName ?? "").trim() || "PLAYER";
  // Mirrors WhiteoutManager._display_text / ASM whiteout wording so MCP surfaces
  // the same loss message the real front end shows instead of a paraphrase.
  return `${normalizedName} is out of useable POKeMON! ${normalizedName} whited out!`;
};

class McpGameSession {
  private readonly sessionId: string;
  private readonly ui: BaseUI;
  private readonly renderUi: BaseUI;
  private readonly textUi: TextUI;
  private game: Game | null = null;
  private gamePromise: Promise<Game> | null = null;
  private holdFrames: number;
  private readonly maxActionsPerCall: number;
  private readonly maxFramesPerCall: number;
  private readonly debugInputEnabled: boolean;
  private readonly frameLimiter = {
    consume: (_frames: number, _now?: number): void => {},
  };
  private frameCounter = 0;
  private sessionStartedAtMs = Date.now();
  private actionEventTotal = 0;
  private autosaveQueue: Promise<void> = Promise.resolve();
  private autosaveLastFrame = -1;
  private scheduledEvents: ScheduledEvent[] = [];
  private actionLog: string[] = [];
  private actionEvents: McpActionEvent[] = [];
  private lastSnapshot: TextSnapshotPayload | null = null;
  private lastSnapshotFrameCounter: number | null = null;
  private lastMcpMeta: McpMeta | null = null;
  private lastActionResult: ActionResult | null = null;
  private ready = false;
  private interactiveMode = false;
  private instantMode: boolean;

  constructor(options?: {
    sessionId?: string;
    maxActionsPerCall?: number;
    maxFramesPerCall?: number;
    instantMode?: boolean;
  }) {
    this.sessionId = options?.sessionId ?? PRIMARY_MCP_SESSION_ID;
    const settings = getSettings();
    const instantMode = options?.instantMode ?? DEFAULT_MCP_INSTANT_MODE;
    this.instantMode = instantMode;
    const renderUi = new DomCanvasUI(160, 144, 1);
    const textUi = new TextUI(160, 144, 1, null, true, null, true);
    const composite = new CompositeUI(
      renderUi as unknown as CompositeChild,
      textUi as unknown as CompositeChild
    );
    this.ui = composite as unknown as BaseUI;
    this.renderUi = renderUi;
    this.textUi = textUi;
    this.holdFrames = settings.mcpHoldFrames;
    this.maxActionsPerCall = options?.maxActionsPerCall ?? settings.mcpMaxActionsPerCall ?? 25;
    this.debugInputEnabled = settings.mcpDebugInput;
    this.maxFramesPerCall = options?.maxFramesPerCall ?? settings.mcpMaxFramesPerMinute ?? 3600;
    if (instantMode) {
      this.maxActionsPerCall = Math.max(this.maxActionsPerCall, 1000);
      this.holdFrames = 1;
    }
  }

  private isInstantMode(): boolean {
    return this.instantMode;
  }

  public setInstantMode(enabled: boolean): void {
    this.instantMode = enabled;
    const settings = getSettings();
    this.holdFrames = enabled ? 1 : settings.mcpHoldFrames;
    this.applyInputModeToGameState(this.game?.getGameState?.());
  }

  private applyInputModeToGameState(
    gameState:
      | {
          sram?: { options?: { no_text_scroll?: boolean } };
          wram?: { wOptions?: number; instant_mode?: boolean };
        }
      | null
      | undefined
  ): void {
    if (!gameState?.sram?.options) {
      return;
    }
    // Keep text at the fastest legal option speed. MCP-backed runtimes, including the
    // interactive TUI, must advance synchronously instead of waiting on visible frames.
    gameState.sram.options.no_text_scroll = true;
    if (gameState.wram && typeof gameState.wram.wOptions === "number") {
      gameState.wram.wOptions |= 1 << 4;
    }
    if (gameState.wram) {
      gameState.wram.instant_mode = this.isInstantMode();
    }
  }

  public setInteractiveMode(interactive: boolean): void {
    this.interactiveMode = interactive;
    const settings = getSettings();
    this.holdFrames = this.isInstantMode() ? 1 : settings.mcpHoldFrames;
    this.applyInputModeToGameState(this.game?.getGameState?.());
  }

  async ensureReady(): Promise<void> {
    if (this.ready) {
      return;
    }
    await this.initGame();
    this.ready = true;
  }

  observeText(): string {
    if (!this.lastSnapshot) {
      this.stepFrames(1);
    }
    if (this.game) {
      this.settlePassiveBattleHandoffIfNeeded(this.game);
    }
    return renderFrameToText(this.ensureFrameConsistentSnapshot("observe"));
  }

  observePayload(): TextSnapshotPayload | null {
    if (!this.lastSnapshot) {
      this.stepFrames(1);
    }
    if (this.game) {
      this.settlePassiveBattleHandoffIfNeeded(this.game);
    }
    return this.ensureFrameConsistentSnapshot("observe");
  }

  async observeTilemapImage(options: { scale?: number } = {}): Promise<{
    data: string;
    width: number;
    height: number;
  }> {
    await this.ensureReady();
    if (!this.lastSnapshot) {
      this.stepFrames(1);
    }
    await this.waitForRenderableImageSurface();
    this.ensureFrameConsistentSnapshot("observe_image");
    return encodeSurfaceToPng(this.renderUi.screen, { scale: options.scale });
  }

  private async waitForRenderableImageSurface(): Promise<void> {
    const game = this.getGame();
    const drawableGame = game as unknown as { draw?: () => void };
    const overworld = game.getOverworld?.() as
      | {
          tileset?: { ready?: Promise<unknown>; loaded?: boolean } | null;
          map_surface?: unknown;
          _composite_surface?: unknown;
          current_map_name?: string;
          load_map?: (mapName: string) => void;
        }
      | null
      | undefined;
    if (!overworld) {
      drawableGame.draw?.();
      return;
    }

    await this.waitForTilesetReady(overworld.tileset);
    if (
      overworld.tileset?.loaded &&
      !overworld.map_surface &&
      typeof overworld.load_map === "function"
    ) {
      const mapName =
        typeof overworld.current_map_name === "string" && overworld.current_map_name.length > 0
          ? overworld.current_map_name
          : this.readBestMapName(game) ?? game.getMapName?.();
      if (mapName) {
        overworld.load_map(mapName);
        await this.waitForTilesetReady(overworld.tileset);
      }
    }

    await this.flushAsyncRenderWork();
    drawableGame.draw?.();
    if (!this.isImageSurfaceBlank() || overworld.map_surface || overworld._composite_surface) {
      return;
    }

    this.stepFrames(1);
    await this.flushAsyncRenderWork();
    drawableGame.draw?.();
  }

  private async waitForTilesetReady(
    tileset: {
      ready?: Promise<unknown> | { then: (handler: () => void, reject?: (error: unknown) => void) => unknown };
      loaded?: boolean;
    } | null | undefined
  ): Promise<void> {
    if (!tileset?.ready || tileset.loaded) {
      return;
    }
    await Promise.race([
      Promise.resolve(tileset.ready).catch(() => undefined),
      new Promise<void>((resolve) => {
        setTimeout(resolve, 3000);
      }),
    ]);
  }

  private async flushAsyncRenderWork(): Promise<void> {
    await new Promise<void>((resolve) => {
      setTimeout(resolve, 0);
    });
  }

  private isImageSurfaceBlank(): boolean {
    const data = this.renderUi.screen.getImageData().data;
    if (data.length < 4) {
      return true;
    }
    const firstR = data[0];
    const firstG = data[1];
    const firstB = data[2];
    const firstA = data[3];
    for (let i = 4; i < data.length; i += 4) {
      if (
        data[i] !== firstR ||
        data[i + 1] !== firstG ||
        data[i + 2] !== firstB ||
        data[i + 3] !== firstA
      ) {
        return false;
      }
    }
    return true;
  }

  private ensureFrameConsistentSnapshot(operation: string): TextSnapshotPayload {
    if (
      this.lastSnapshot &&
      this.lastSnapshotFrameCounter === this.frameCounter &&
      !this.lastMcpMeta
    ) {
      return this.lastSnapshot;
    }
    try {
      this.captureSnapshot();
    } catch {
      // Fall through to a minimal fallback below; MCP callers still need a state
      // payload even if a headless renderer hook fails during snapshot capture.
    }
    if (!this.lastSnapshot) {
      this.lastSnapshot = buildSnapshotPayload(this.buildMinimalFallbackTextSnapshot(operation), {
        actionLog: this.actionLog,
        script: {},
        tasks: [],
        mcp: this.lastMcpMeta ?? undefined,
      });
      this.lastMcpMeta = null;
      this.lastSnapshotFrameCounter = this.frameCounter;
    }
    return this.lastSnapshot;
  }

  getFrameCount(): number {
    return this.frameCounter;
  }

  async advanceFrames(frames = 1): Promise<void> {
    await this.ensureReady();
    const normalizedFrames = this.normalizeFrames(frames);
    this.stepFrames(normalizedFrames);
  }

  async move(direction: Direction, times = 1, options: ActionSnapshotOptions = {}): Promise<ActionResultWithSnapshot> {
    await this.ensureReady();
    const game = this.getGame();
    const staleDirectionalInputClearedBefore = this.clearStaleDirectionalInput(game);
    const beforeSignal = this.captureSceneSignal(game);
    const before = this.buildStateFingerprint(game);
    const outcome = await this.performMove(direction, times, { stopOnEvent: true });
    const staleDirectionalInputClearedAfter = this.clearStaleDirectionalInput(game);
    const afterSignal = this.captureSceneSignal(game);
    const changed = before !== this.buildStateFingerprint(game) || outcome.completed > 0;
    const events: string[] = [];
    if (staleDirectionalInputClearedBefore || staleDirectionalInputClearedAfter) {
      events.push("stale_input_cleared");
    }
    if (outcome.completed > 0) {
      events.push(`moved:${outcome.completed}`);
    }
    if (outcome.blocked) {
      events.push(`blocked:${outcome.blockReason ?? "unknown"}`);
    }
    if (outcome.stopReason) {
      events.push(`interrupted:${outcome.stopReason}`);
    }
    const mappedReason = this.mapActionReason({
      blocked: outcome.blocked,
      stopReason: outcome.stopReason,
      changed,
    });
    const result = await this.finalizeActionResult({
      reason: mappedReason,
      changed,
      events: events.length ? events : undefined,
    });
    this.recordActionEvent(`move:${direction}:${times ?? 1}`, result, {
      before: beforeSignal,
      after: afterSignal,
    });
    return { result, snapshotText: this.actionSnapshotText(options) };
  }

  async press(button: Button, times = 1, options: ActionSnapshotOptions = {}): Promise<ActionResultWithSnapshot> {
    await this.ensureReady();
    const game = this.getGame();
    this.resetStaleButtonReleaseGuards(game, button);
    this.alignMcpHealCounterFacingBeforePress(game, button);
    const beforeIntroFingerprint = this.buildStateFingerprint(game);
    const startedInBattleIntro = this.isBattleIntroTransitionSnapshot(this.lastSnapshot);
    this.settleBattleIntroIfNeeded(game);
    const beforeSignal = this.captureSceneSignal(game);
    const startedOnInputOwningSurface = isInputOwningSurfaceSnapshot(this.lastSnapshot);
    const instantBattleInput =
      beforeSignal.mode === "battle" &&
      Boolean(game.getGameState?.()?.wram?.instant_mode);
    if (
      startedInBattleIntro &&
      button === "a" &&
      beforeSignal.mode === "battle" &&
      beforeSignal.menu &&
      !beforeSignal.promptReason &&
      !beforeSignal.dialogueText.trim()
    ) {
      const changed = beforeIntroFingerprint !== this.buildStateFingerprint(game);
      const result = await this.finalizeActionResult({
        reason: changed ? undefined : this.inferNoChangeReason(beforeSignal),
        changed,
        events: [`pressed:${button}:1`],
      });
      this.recordActionEvent(`press:${button}:1`, result, {
        before: {
          mode: "battle",
          menu: false,
          promptReason: null,
          dialogueText: "",
          viewportText: "",
          menuText: "",
          promptText: "",
          markerText: "",
        },
        after: beforeSignal,
      });
      return { result, snapshotText: this.actionSnapshotText(options) };
    }
    const normalizedTimes = this.normalizeTimes(times);
    const isDirectionalButton =
      button === "up" || button === "down" || button === "left" || button === "right";
    const staleDirectionalInputCleared =
      !isDirectionalButton ? this.clearStaleDirectionalInput(game) : false;
    const before = this.buildStateFingerprint(game);
    const overworldInteractionAheadBeforePress =
      button === "a" &&
      normalizedTimes === 1 &&
      beforeSignal.mode === "overworld" &&
      !beforeSignal.menu &&
      !beforeSignal.promptReason &&
      !beforeSignal.dialogueText.trim()
        ? this.readInteractionTarget(game, this.readInteractionTile(game), this.buildSnapshotMapInfo())
        : undefined;
      this.recordAction(formatActionLabel("button", button));
    for (let i = 0; i < normalizedTimes; i += 1) {
      const modalBeforePress = this.getModalUiState(game);
      const startedOnNonBlockingPcUi = isNonBlockingPcUiSnapshot(this.lastSnapshot);
      const inputOwningSurfaceHoldFrames =
        startedOnInputOwningSurface && !isDirectionalButton
          ? Math.max(this.holdFrames, 4)
          : this.holdFrames;
      const interactionHoldFrames =
        button === "a" &&
        beforeSignal.mode === "overworld" &&
        !modalBeforePress.in_dialog &&
        !modalBeforePress.prompt_pending &&
        !modalBeforePress.in_menu &&
        overworldInteractionAheadBeforePress
          ? Math.max(this.holdFrames, 2)
          : inputOwningSurfaceHoldFrames;
      this.scheduleKeyPress({
        key: keyForButton(button),
        button: isDirectionalButton ? undefined : button,
        direction: isDirectionalButton ? button : undefined,
        holdFrames: isDirectionalButton ? this.holdFrames : interactionHoldFrames,
        repeatPressFrames: startedOnInputOwningSurface && !isDirectionalButton && !startedOnNonBlockingPcUi,
      });
      const scheduledHoldFrames = isDirectionalButton ? this.holdFrames : interactionHoldFrames;
      const settleFrames =
        instantBattleInput
          ? 0
        : button === "a" && (modalBeforePress.in_dialog || modalBeforePress.prompt_pending)
          ? 1
          : button === "a" && modalBeforePress.in_battle && modalBeforePress.in_menu
            ? 3
          : isDirectionalButton && modalBeforePress.in_battle && modalBeforePress.in_menu
            ? 2
          : isDirectionalButton && (modalBeforePress.in_menu || modalBeforePress.prompt_pending)
            ? 2
          : button === "b" || button === "start"
            ? 1
            : 0;
      if (startedOnInputOwningSurface) {
        for (let frame = 0; frame < scheduledHoldFrames; frame += 1) {
          this.stepFrames(1);
          await this.waitForInputOwningSurfaceSettle();
        }
        this.stepFrames(1 + settleFrames);
      } else {
        this.stepFrames(scheduledHoldFrames + 1 + settleFrames);
      }
    }
    let changed = before !== this.buildStateFingerprint(game);
    let afterSignal = this.captureSceneSignal(game);
    const needsExtraBattleSettle =
      !instantBattleInput &&
      normalizedTimes === 1 &&
      beforeSignal.mode === "battle" &&
      (
        (button === "a" &&
          (
            (!changed && beforeSignal.menu) ||
            (changed &&
              Boolean(beforeSignal.promptReason) &&
              !afterSignal.promptReason &&
              afterSignal.mode === "battle" &&
              !afterSignal.menu &&
              !afterSignal.dialogueText.trim()) ||
            (changed &&
              Boolean(beforeSignal.dialogueText.trim()) &&
              afterSignal.mode === "battle" &&
              !afterSignal.menu &&
              !afterSignal.promptReason &&
              !afterSignal.dialogueText.trim())
          )) ||
        (isDirectionalButton && !changed && beforeSignal.menu)
      );
    if (needsExtraBattleSettle) {
      // Interactive battle confirms can land a few frames late as the menu hands off
      // into the move picker or turn text. The same late landing can happen right after
      // closing battle dialogue, where the prompt disappears first and the next menu/text
      // frame arrives a few frames later. Battle menu cursor moves can also visually land
      // a few frames late after the main command menu redraws. Give those paths one extra
      // bounded settle pass before reporting a no-progress battle stall.
      this.stepFrames(4);
      changed = before !== this.buildStateFingerprint(game);
      afterSignal = this.captureSceneSignal(game);
      if (isDirectionalButton && !changed && beforeSignal.menu && afterSignal.menu) {
        // The main battle menu can sometimes need one more redraw pass after backing out of
        // a submenu before the cursor visibly moves. Some late Route 30 battle captures
        // showed the cursor landing even later than that, so keep retry windows bounded
        // but allow one short extension while the scene is still the same battle menu.
        for (const extraFrames of [4, 8]) {
          this.stepFrames(extraFrames);
          changed = before !== this.buildStateFingerprint(game);
          afterSignal = this.captureSceneSignal(game);
          if (changed || !afterSignal.menu || afterSignal.promptReason || afterSignal.dialogueText.trim()) {
            break;
          }
        }
      } else if (
        button === "a" &&
        !changed &&
        beforeSignal.menu &&
        afterSignal.mode === "battle" &&
        afterSignal.menu &&
        !afterSignal.promptReason &&
        !afterSignal.dialogueText.trim()
      ) {
        // Tutorial and other battle move confirms can sometimes settle extremely late after
        // the move picker closes. Keep retry windows bounded and only extend them while the
        // scene is still the same blank battle-menu handoff.
        for (const extraFrames of [8, 16, 32]) {
          this.stepFrames(extraFrames);
          changed = before !== this.buildStateFingerprint(game);
          afterSignal = this.captureSceneSignal(game);
          if (changed || afterSignal.promptReason || afterSignal.dialogueText.trim() || !afterSignal.menu) {
            break;
          }
        }
      } else if (
        button === "a" &&
        Boolean(beforeSignal.promptReason) &&
        afterSignal.mode === "battle" &&
        !afterSignal.menu &&
        !afterSignal.promptReason &&
        !afterSignal.dialogueText.trim()
      ) {
        // Catch resolution and similar post-dialogue battle paths can close the prompt first
        // and then stay blank for several redraws while delayed item/animation work settles.
        // Keep retry windows bounded, but allow additional frame passes while the battle is
        // still in that blank no-menu handoff so interactive capture flow doesn't look frozen.
        for (const extraFrames of [8, 16, 32]) {
          this.stepFrames(extraFrames);
          changed = before !== this.buildStateFingerprint(game);
          afterSignal = this.captureSceneSignal(game);
          if (afterSignal.promptReason || afterSignal.dialogueText.trim() || afterSignal.menu) {
            break;
          }
        }
      } else if (
        button === "a" &&
        changed &&
        beforeSignal.mode === "battle" &&
        Boolean(beforeSignal.dialogueText.trim()) &&
        afterSignal.mode === "battle" &&
        !afterSignal.menu &&
        !afterSignal.promptReason &&
        !afterSignal.dialogueText.trim()
      ) {
        // Wild-battle KO flow can close the faint/EXP dialogue first and then stay on a blank
        // battle shell for several redraws before the engine returns to overworld or opens the
        // next menu. Keep the retry window bounded, but continue settling while the battle is
        // still in that blank no-menu handoff so live Route 30 battles do not look frozen.
        for (const extraFrames of [4, 8, 16, 32]) {
          this.stepFrames(extraFrames);
          changed = before !== this.buildStateFingerprint(game);
          afterSignal = this.captureSceneSignal(game);
          if (
            afterSignal.mode !== "battle" ||
            afterSignal.promptReason ||
            afterSignal.dialogueText.trim() ||
            afterSignal.menu
          ) {
            break;
          }
        }
      }
    }
    const needsExtraOverworldASettle =
      button === "a" &&
      normalizedTimes === 1 &&
      beforeSignal.mode === "overworld" &&
      !beforeSignal.menu &&
      !beforeSignal.promptReason &&
      !beforeSignal.dialogueText.trim() &&
      !changed;
    if (needsExtraOverworldASettle) {
      // Overworld talk/interact flows can occasionally surface dialogue a few frames late
      // after the hardware-accurate A press has already completed, especially right after
      // a resumed runtime snapshot rehydrates the live shell. Keep the retry window small
      // and only continue while the scene is still a blank overworld handoff.
      // Script-backed object interactions can land even later than ordinary talk/sign
      // confirms, so keep one extra bounded settle pass available when the runtime
      // already confirms an interaction target directly ahead.
      const extraSettlePasses = overworldInteractionAheadBeforePress ? [4, 8, 16, 32, 64] : [4, 8];
      for (const extraFrames of extraSettlePasses) {
        this.stepFrames(extraFrames);
        changed = before !== this.buildStateFingerprint(game);
        afterSignal = this.captureSceneSignal(game);
        if (
          changed ||
          afterSignal.mode !== "overworld" ||
          afterSignal.menu ||
          Boolean(afterSignal.promptReason) ||
          afterSignal.dialogueText.trim()
        ) {
          break;
        }
      }
    }
    const fallbackScriptedInteractionStarted =
      button === "a" &&
      normalizedTimes === 1 &&
      beforeSignal.mode === "overworld" &&
      !beforeSignal.menu &&
      !beforeSignal.promptReason &&
      !beforeSignal.dialogueText.trim() &&
      Boolean(overworldInteractionAheadBeforePress?.script) &&
      afterSignal.mode === "overworld" &&
      !afterSignal.menu &&
      !afterSignal.promptReason &&
      !afterSignal.dialogueText.trim() &&
      overworldInteractionAheadBeforePress !== undefined &&
      this.runConfirmedScriptedInteraction(game, overworldInteractionAheadBeforePress);
    if (fallbackScriptedInteractionStarted) {
      this.stepFrames(1);
      this.settleMovementLock(game, null);
      changed = before !== this.buildStateFingerprint(game);
      afterSignal = this.captureSceneSignal(game);
    }
    const needsPostDialoguePromptSettle =
      button === "a" &&
      normalizedTimes === 1 &&
      beforeSignal.mode === "overworld" &&
      !beforeSignal.menu &&
      (Boolean(beforeSignal.promptReason) || Boolean(beforeSignal.dialogueText.trim())) &&
      afterSignal.mode === "overworld" &&
      !afterSignal.menu &&
      !afterSignal.promptReason &&
      !afterSignal.dialogueText.trim();
    if (needsPostDialoguePromptSettle) {
      const modalAfterPress = this.getModalUiState(game);
      if (modalAfterPress.movement_locked || modalAfterPress.script_busy) {
        this.settleMovementLock(game, null);
        changed = before !== this.buildStateFingerprint(game);
        afterSignal = this.captureSceneSignal(game);
      }
    }
    if (!changed && startedOnInputOwningSurface) {
      for (const extraFrames of [2, 2, 4, 8, 16]) {
        await this.waitForInputOwningSurfaceSettle();
        this.stepFrames(extraFrames);
        changed = before !== this.buildStateFingerprint(game);
        afterSignal = this.captureSceneSignal(game);
        if (changed || !isInputOwningSurfaceSnapshot(this.lastSnapshot)) {
          break;
        }
      }
    }
    const result = await this.finalizeActionResult({
      reason: changed ? undefined : this.inferNoChangeReason(afterSignal),
      changed,
      events: [
        `pressed:${button}:${normalizedTimes}`,
        ...(fallbackScriptedInteractionStarted ? ["confirmed_scripted_interaction_retried"] : []),
        ...(fallbackScriptedInteractionStarted && overworldInteractionAheadBeforePress?.hotspot_type === "heal"
          ? ["confirmed_heal_interaction_retried"]
          : []),
        ...(staleDirectionalInputCleared ? ["stale_input_cleared"] : []),
      ],
    });
    this.recordActionEvent(`press:${button}:${normalizedTimes}`, result, {
      before: beforeSignal,
      after: afterSignal,
    });
    return { result, snapshotText: this.actionSnapshotText(options) };
  }

  private actionSnapshotText(options: ActionSnapshotOptions = {}): string {
    if (options.settleSnapshot === false) {
      if (!this.lastSnapshot) {
        this.stepFrames(1);
      }
      return renderFrameToText(this.ensureFrameConsistentSnapshot("action"));
    }
    return this.observeText();
  }

  private resetStaleButtonReleaseGuards(game: Game, button: Button): void {
    if (button !== "a" && button !== "b" && button !== "start") {
      return;
    }
    const overworld = game.getOverworld?.() as
      | {
          _ignore_a_until_release?: boolean;
          dialogue?: { ignore_confirm_until_release?: boolean } | null;
        }
      | null
      | undefined;
    if (!overworld) {
      return;
    }
    if (button === "a") {
      overworld._ignore_a_until_release = false;
    }
    if (overworld.dialogue) {
      overworld.dialogue.ignore_confirm_until_release = false;
    }
  }

  private settleBattleIntroIfNeeded(game: Game): void {
    if (!(game.isBattleActive?.() ?? false)) {
      return;
    }
    const modal = this.getModalUiState(game);
    if (modal.in_menu || modal.prompt_pending || modal.in_dialog) {
      return;
    }
    if (!this.isBattleIntroTransitionSnapshot(this.lastSnapshot)) {
      return;
    }

    for (const extraFrames of [8, 16, 32, 64, 64]) {
      this.stepFrames(extraFrames);
      const settledModal = this.getModalUiState(game);
      if (!(game.isBattleActive?.() ?? false)) {
        break;
      }
      if (settledModal.in_menu || settledModal.prompt_pending || settledModal.in_dialog) {
        break;
      }
      if (!this.isBattleIntroTransitionSnapshot(this.lastSnapshot)) {
        break;
      }
    }
  }

  private settlePassiveBattleHandoffIfNeeded(game: Game): void {
    if (!(game.isBattleActive?.() ?? false)) {
      return;
    }
    const modal = this.getModalUiState(game);
    if (modal.in_menu || modal.prompt_pending || modal.in_dialog) {
      return;
    }
    const signal = this.captureSceneSignal(game);
    if (
      signal.mode !== "battle" ||
      signal.menu ||
      signal.promptReason ||
      signal.dialogueText.trim()
    ) {
      return;
    }

    for (const extraFrames of [4, 8, 16, 32, 64]) {
      this.stepFrames(extraFrames);
      const settledModal = this.getModalUiState(game);
      const settledSignal = this.captureSceneSignal(game);
      if (!(game.isBattleActive?.() ?? false)) {
        break;
      }
      if (
        settledModal.in_menu ||
        settledModal.prompt_pending ||
        settledModal.in_dialog ||
        settledSignal.mode !== "battle" ||
        settledSignal.menu ||
        settledSignal.promptReason ||
        settledSignal.dialogueText.trim()
      ) {
        break;
      }
    }
  }

  async waitForPrompt(timeoutSeconds: number | null = null): Promise<string> {
    await this.ensureReady();
    const settings = getSettings();
    const effectiveTimeout =
      timeoutSeconds ?? settings.mcpFrameTimeout ?? 5;
    const deadline = Date.now() + effectiveTimeout * 1000;
    let framesWaited = 0;
    while (true) {
      const snapshot = this.lastSnapshot;
      const status = promptFromSnapshot(snapshot);
      if (status.pending) {
        return this.observeText();
      }
      if (Date.now() >= deadline) {
        throw new Error(
          `Timed out waiting for prompt after ${effectiveTimeout} seconds.`
        );
      }
      if (framesWaited >= this.maxFramesPerCall) {
        throw new Error(
          `Wait for prompt exceeded ${this.maxFramesPerCall} frames.`
        );
      }
      this.frameLimiter.consume(1);
      this.stepFrames(1);
      framesWaited += 1;
    }
  }

  async holdButton(button: Button, frames: number): Promise<ActionResultWithSnapshot> {
    await this.ensureReady();
    const game = this.getGame();
    const beforeSignal = this.captureSceneSignal(game);
    const normalizedFrames = Math.max(1, Math.min(frames, this.maxFramesPerCall));
    const before = this.buildStateFingerprint(game);
    const isDirectionalButton =
      button === "up" || button === "down" || button === "left" || button === "right";
    this.scheduleKeyPress({
        key: keyForButton(button),
        button: isDirectionalButton ? undefined : button,
        direction: isDirectionalButton ? button : undefined,
        holdFrames: normalizedFrames
    });
    this.stepFrames(normalizedFrames + 1);
    const changed = before !== this.buildStateFingerprint(game);
    const afterSignal = this.captureSceneSignal(game);
    const result = await this.finalizeActionResult({
      reason: changed ? undefined : this.inferNoChangeReason(afterSignal),
      changed,
      events: [`held:${button}:${normalizedFrames}`],
    });
    this.recordActionEvent(`hold_button:${button}:${normalizedFrames}`, result, {
      before: beforeSignal,
      after: afterSignal,
    });
    return { result, snapshotText: this.observeText() };
  }

  async typeText(text: string, options: TypeTextOptions = {}): Promise<ActionResultWithSnapshot> {
    await this.ensureReady();
    const game = this.getGame();
    const beforeSignal = this.captureSceneSignal(game);
    const before = this.buildStateFingerprint(game);
    const chars = [...text].slice(0, this.maxFramesPerCall);
    const events: string[] = [];
    if (this.isNameEntryActive(game)) {
      if (options.clear) {
        const nameLength = this.readNameEntryName(game).length;
        for (let index = 0; index < nameLength; index += 1) {
          this.recordAction("button:b");
          this.scheduleNameEntryButton("b");
          events.push("deleted");
        }
      }
      for (const char of chars) {
        if (this.scheduleNameEntryTextInput(char)) {
          events.push(`typed:${char}`);
        }
      }
      if (options.submit) {
        this.recordAction("button:start");
        this.scheduleNameEntryButton("start");
        this.recordAction("button:a");
        this.scheduleNameEntryButton("a");
        events.push("submitted");
      }
    } else {
      for (const char of chars) {
        this.recordAction(`text:${char}`);
        this.scheduleTextInput(char);
        this.stepFrames(1);
      }
      if (chars.length) {
        events.push(`typed:${chars.join("")}`);
      }
    }
    const changed = before !== this.buildStateFingerprint(game);
    const afterSignal = this.captureSceneSignal(game);
    const result = await this.finalizeActionResult({
      reason: changed ? undefined : this.inferNoChangeReason(afterSignal),
      changed,
      events: events.length ? events : undefined,
    });
    const suffix = [
      chars.join(""),
      options.clear ? "clear" : null,
      options.submit ? "submit" : null,
    ].filter(Boolean).join(":");
    this.recordActionEvent(`type_text:${suffix}`, result, {
      before: beforeSignal,
      after: afterSignal,
    });
    return { result, snapshotText: this.observeText() };
  }

  private readNameEntryName(game: Game): string {
    const debugState = game.getDebugStatus?.() as { name_entry?: { name?: unknown } | null } | null | undefined;
    const name = debugState?.name_entry?.name;
    return typeof name === "string" ? name : "";
  }

  private scheduleNameEntryButton(button: Button): void {
    const isDirection = button === "up" || button === "down" || button === "left" || button === "right";
    this.scheduleKeyPress({
      key: keyForButton(button),
      button: isDirection ? undefined : button,
      direction: isDirection ? button : undefined,
      holdFrames: this.holdFrames,
    });
    this.stepFrames(this.holdFrames + 1);
  }

  private scheduleNameEntryTextInput(char: string): boolean {
    if (/^[a-z]$/i.test(char) || char === " ") {
      this.recordAction(`text:${char}`);
      this.scheduleTextInput(char);
      this.stepFrames(1);
      return true;
    }
    const target = NAME_ENTRY_SPECIAL_TARGETS.get(char);
    if (!target) {
      return false;
    }
    this.moveNameEntryCursorTo(target);
    this.recordAction("button:a");
    this.scheduleNameEntryButton("a");
    return true;
  }

  private moveNameEntryCursorTo(target: NameEntryTarget): void {
    const current = this.readNameEntryCursor();
    if (this.readNameEntryCase() !== target.caseMode) {
      this.moveNameEntryCursorBetween(current, { row: NAME_ENTRY_ROW_COUNT - 1, column: NAME_ENTRY_BOTTOM_CASE_COLUMN });
      this.recordAction("button:a");
      this.scheduleNameEntryButton("a");
      current.row = NAME_ENTRY_ROW_COUNT - 1;
      current.column = NAME_ENTRY_BOTTOM_CASE_COLUMN;
    }
    this.moveNameEntryCursorBetween(current, target);
  }

  private moveNameEntryCursorBetween(current: NameEntryCursor, target: NameEntryCursor): void {
    for (const direction of this.shortestNameEntryDirections(
      current.row,
      target.row,
      NAME_ENTRY_ROW_COUNT,
      "down",
      "up"
    )) {
      this.recordAction(`button:${direction}`);
      this.scheduleNameEntryButton(direction);
    }
    current.row = target.row;

    if (target.row === NAME_ENTRY_ROW_COUNT - 1) {
      const currentBottomGroup = Math.floor(current.column / 3);
      const targetBottomGroup = Math.floor(target.column / 3);
      for (const direction of this.shortestNameEntryDirections(currentBottomGroup, targetBottomGroup, 3, "right", "left")) {
        this.recordAction(`button:${direction}`);
        this.scheduleNameEntryButton(direction);
      }
    } else {
      for (const direction of this.shortestNameEntryDirections(
        current.column,
        target.column,
        NAME_ENTRY_COLUMN_COUNT,
        "right",
        "left"
      )) {
        this.recordAction(`button:${direction}`);
        this.scheduleNameEntryButton(direction);
      }
    }
    current.column = target.column;
  }

  private shortestNameEntryDirections(
    current: number,
    target: number,
    size: number,
    forward: Direction,
    backward: Direction
  ): Direction[] {
    const forwardCount = (target - current + size) % size;
    const backwardCount = (current - target + size) % size;
    return Array(forwardCount <= backwardCount ? forwardCount : backwardCount).fill(
      forwardCount <= backwardCount ? forward : backward
    );
  }

  private readNameEntryCursor(): NameEntryCursor {
    const cursorLine = this.lastSnapshot?.info?.find((line) => /^CURSOR:/i.test(String(line).trim()));
    const match = String(cursorLine ?? "").match(/row\s+(\d+)\s+col\s+(\d+)/i);
    return {
      row: Math.max(0, Math.min(NAME_ENTRY_ROW_COUNT - 1, Number(match?.[1] ?? 0))),
      column: Math.max(0, Math.min(NAME_ENTRY_COLUMN_COUNT - 1, Number(match?.[2] ?? 0))),
    };
  }

  private readNameEntryCase(): "upper" | "lower" {
    const caseLine = this.lastSnapshot?.info?.find((line) => /^CASE:/i.test(String(line).trim()));
    return /lower/i.test(String(caseLine ?? "")) ? "lower" : "upper";
  }

  async getGameStateData(): Promise<McpStatusSnapshot> {
    return this.status();
  }

  private readGameMode(game: Game): string | undefined {
    const debugStatus = (game as { getDebugStatus?: () => { mode?: unknown } }).getDebugStatus?.();
    return typeof debugStatus?.mode === "string" && debugStatus.mode.trim()
      ? debugStatus.mode.trim()
      : undefined;
  }

  private isBootUiMode(mode: string | undefined): mode is string {
    return Boolean(mode && mode !== "overworld" && mode !== "menu" && mode !== "battle");
  }

  private bootUiLocation(mode: string): string {
    return mode.toUpperCase().replaceAll("_", " ");
  }

  async playerContext(): Promise<McpPlayerContext> {
    await this.ensureReady();
    const game = this.getGame();
    const mode = this.readGameMode(game);
    if (this.isBootUiMode(mode)) {
      return {
        map: this.bootUiLocation(mode),
        coords: null,
        facing: "unknown",
        menu_open: true,
        dialogue_open: true,
      };
    }
    const snapshot = this.lastSnapshot;
    const map = this.readBestMapName(game) ?? null;
    const coords = this.readBestCoords(game) ?? null;
    const dialogueState = this.getDialogueUiState(game);
    const dialogueOpen = Boolean(snapshot?.dialogue?.length) || dialogueState.dialog_active;
    return {
      map,
      coords,
      facing: this.readFacingDirection(game),
      menu_open: this.isMenuOpenForSession(game),
      dialogue_open: dialogueOpen,
      text_advance_pending: dialogueState.text_advance_pending || undefined,
    };
  }

  async status(): Promise<McpStatusSnapshot> {
    await this.ensureReady();
    const game = this.getGame();
    this.settlePassiveBattleHandoffIfNeeded(game);
    let synchronizedSnapshot = this.ensureFrameConsistentSnapshot("status");
    let promptStatus = promptFromSnapshot(synchronizedSnapshot);
    let modal = this.getModalUiState(game);
    if (
      !modal.in_battle &&
      !modal.in_menu &&
      !modal.in_dialog &&
      !modal.prompt_pending &&
      !this.isUnownPuzzleInputActive(game) &&
      (modal.movement_locked || modal.script_busy)
    ) {
      this.settleMovementLock(game, null);
      synchronizedSnapshot = this.ensureFrameConsistentSnapshot("status");
      promptStatus = promptFromSnapshot(synchronizedSnapshot);
      modal = this.getModalUiState(game);
    }
    const state = game.getGameState();
    const gameMode = this.readGameMode(game);
    const bootMode = this.isBootUiMode(gameMode) ? gameMode : undefined;
    const mode: McpStatusSnapshot["mode"] = bootMode ?? (modal.in_battle
      ? "battle"
      : modal.in_menu
        ? "menu"
        : "overworld");
    const coords = bootMode ? undefined : this.readBestCoords(game);
    const map = bootMode ? this.bootUiLocation(bootMode) : this.readBestMapName(game);
    const mapIdentity = this.readMapIdentity(game);
    const fallbackMapName = mapIdentity.name.trim();
    const locationName = bootMode
      ? this.bootUiLocation(bootMode)
      : map ?? (fallbackMapName.length ? fallbackMapName : "Unknown");
    const mapGroup = bootMode
      ? null
      : mapIdentity.group ??
        (typeof state.wram.current_map_group === "number" && Number.isFinite(state.wram.current_map_group)
          ? state.wram.current_map_group
          : null);
    const mapNumber = bootMode
      ? null
      : mapIdentity.number ??
        (typeof state.wram.current_map_id === "number" && Number.isFinite(state.wram.current_map_id)
          ? state.wram.current_map_id
          : null);
    const mapId = bootMode ? bootMode : `${mapGroup ?? "unknown"}:${mapNumber ?? "unknown"}`;
    const facingRaw = this.readFacingDirection(game);
    const facing: McpStatusSnapshot["facing"] =
      bootMode || facingRaw === "unknown" ? undefined : (facingRaw as Direction);
    const interactionTile = bootMode ? undefined : this.readInteractionTile(game);
    const partyPokemon = Array.isArray((state as { sram?: { party?: { pokemon?: unknown[] } } }).sram?.party?.pokemon)
      ? ((state as { sram?: { party?: { pokemon?: unknown[] } } }).sram?.party?.pokemon ?? []).filter(Boolean)
      : [];
    const badges = (state as { sram?: { badges?: { johto?: unknown[]; kanto?: unknown[] } } }).sram?.badges;
    const johtoBadges = Array.isArray(badges?.johto) ? badges?.johto.filter(Boolean).length : 0;
    const kantoBadges = Array.isArray(badges?.kanto) ? badges?.kanto.filter(Boolean).length : 0;
    const badgesCount = johtoBadges + kantoBadges;
    const sram = (state as {
      sram?: {
        money?: unknown;
        moms_money?: unknown;
        mom_saving_some_money?: unknown;
      };
    }).sram;
    const walletMoney = Math.max(
      0,
      Math.min(999999, Number.isFinite(Number(sram?.money)) ? Math.trunc(Number(sram?.money)) : 0)
    );
    const momsMoney = Math.max(
      0,
      Math.min(999999, Number.isFinite(Number(sram?.moms_money)) ? Math.trunc(Number(sram?.moms_money)) : 0)
    );
    const momSavingSomeMoney = Boolean(sram?.mom_saving_some_money);
    const leadPokemon = (partyPokemon[0] ?? null) as {
      species?: unknown;
      level?: unknown;
      hp?: unknown;
      max_hp?: unknown;
      status?: unknown;
    } | null;
    const overworld = game.getOverworld() as unknown as {
      _movement_lock_count?: number;
      _text_lock_active?: boolean;
      _blocking_task_count?: number;
      _blocking_movement_lock_active?: boolean;
      _active_bg_event?: { event_type?: unknown; script?: unknown; x?: unknown; y?: unknown } | null;
      script_runner?: {
        _script_stack?: Array<{ name?: unknown }> | unknown[];
        _awaiting_resume?: number;
        stop_execution?: boolean;
        is_busy?: boolean;
        state?: unknown;
      } | null;
    };
    const battle = (game.getBattle?.() ?? null) as
      | {
          context?: {
            currentState?: number;
            trainerBattle?: unknown;
            enemyTrainer?: unknown;
            playerAction?: unknown;
            enemyAction?: unknown;
          } | null;
          _turnCursor?: number;
        }
      | null;
    const battleState =
      modal.in_battle && battle?.context
        ? String(battle.context.currentState)
        : undefined;
    const mapDetails =
      synchronizedSnapshot.map ??
      buildMapInfoSnapshot({
        map,
        mapGroup,
        mapNumber,
        overworld: overworld as unknown as OverworldMapInfoSource,
        playerCoords: coords,
        facing,
        dataLoader: resolveSessionMapDataLoader(state),
        eventFlags: (state.wram.event_flags ?? null) as Record<string, boolean | undefined> | null,
      });
    const interactionTarget = this.readInteractionTarget(game, interactionTile, mapDetails);
    const currentHotspot = this.readCurrentHotspot(coords, mapDetails);
    const interactionSetup = this.readInteractionSetup(coords, mapDetails, currentHotspot, interactionTarget);
    const interactionLane = this.readInteractionLane(coords, facing, mapDetails, interactionTarget);
    const scriptStack = Array.isArray(overworld?.script_runner?._script_stack) ? overworld.script_runner._script_stack : [];
    const activeScript =
      [...scriptStack]
        .reverse()
        .map((frame) =>
          typeof frame === "object" && frame !== null && "name" in frame ? String(frame.name ?? "").trim() : ""
        )
        .find((name) => name.length > 0) ?? undefined;
    const activeBgEvent =
      typeof overworld?._active_bg_event === "object" && overworld._active_bg_event !== null
        ? overworld._active_bg_event
        : null;
    const activeBgCoords = {
      x: typeof activeBgEvent?.x === "number" && Number.isFinite(activeBgEvent.x) ? activeBgEvent.x : undefined,
      y: typeof activeBgEvent?.y === "number" && Number.isFinite(activeBgEvent.y) ? activeBgEvent.y : undefined,
    };
    const activeBgHotspot =
      activeBgCoords.x !== undefined && activeBgCoords.y !== undefined
        ? (mapDetails.hotspots.find((hotspot) =>
            hotspot.coords.x === activeBgCoords.x &&
            hotspot.coords.y === activeBgCoords.y
          ) ?? null)
        : null;
    const activeNpcSceneOwner = activeScript ? this.readActiveNpcSceneOwner(game, mapDetails) : undefined;
    const sceneOwner =
      activeBgEvent
        ? {
            kind: "bg_event" as const,
            x: activeBgCoords.x,
            y: activeBgCoords.y,
            label: activeBgHotspot?.label,
            token: activeBgHotspot?.token,
            hotspot_type: activeBgHotspot?.type,
            script: String(activeBgEvent.script ?? "").trim() || undefined,
          }
        : activeNpcSceneOwner;
    const interactionPivot = this.readInteractionPivot(coords, mapDetails, interactionLane, interactionTarget);
    const rawLocalFocus = this.readLocalFocus(
      coords,
      mapDetails,
      sceneOwner,
      interactionPivot,
      currentHotspot,
      interactionSetup,
      interactionLane,
      interactionTarget
    );
    const stabilizedLocalFocus = this.suppressStaleInteractionPivot(
      coords,
      mapDetails,
      interactionLane,
      rawLocalFocus
    );
    const localFocus = this.withLocalFocusApproach(coords, mapDetails, stabilizedLocalFocus);
    const surfacedInteractionLane = this.suppressLowAuthorityInteractionLaneForNpcPivot(
      interactionLane,
      localFocus
    );
    return {
      mode,
      menu: bootMode ? true : modal.in_menu,
      instant_mode: Boolean(state.wram.instant_mode),
      surface: buildSurfaceStatus(synchronizedSnapshot, mode),
      notices: synchronizedSnapshot.notices ?? this.buildSessionNotices(game),
      audio: game.getAudioPlaybackSnapshot?.(),
      battle_state: battleState,
      battle_is_trainer: modal.in_battle
        ? Boolean(battle?.context?.trainerBattle ?? battle?.context?.enemyTrainer)
        : undefined,
      battle_turn_cursor:
        modal.in_battle && typeof battle?._turnCursor === "number" && Number.isFinite(battle._turnCursor)
          ? battle._turnCursor
          : undefined,
      battle_has_player_action: modal.in_battle ? Boolean(battle?.context?.playerAction) : undefined,
      battle_has_enemy_action: modal.in_battle ? Boolean(battle?.context?.enemyAction) : undefined,
      in_menu: bootMode ? true : modal.in_menu,
      in_battle: bootMode ? false : modal.in_battle,
      in_dialog: bootMode ? true : modal.in_dialog,
      textbox_open: bootMode ? false : modal.text_box_open,
      text_box_open: bootMode ? false : modal.text_box_open,
      text_advance_pending: bootMode ? false : modal.text_advance_pending,
      prompt_pending: bootMode ? true : modal.prompt_pending,
      unown_puzzle_active: bootMode ? false : this.isUnownPuzzleInputActive(game),
      unown_state: bootMode ? 0 : Number(state.wram.wUnownState ?? 0),
      movement_locked: bootMode ? true : modal.movement_locked,
      script_busy: modal.script_busy,
      can_move: bootMode ? false : modal.can_move,
      input_blocked_reason: bootMode ?? modal.input_blocked_reason,
      facing,
      interaction_tile: interactionTile,
      interaction_target: interactionTarget,
      current_hotspot: currentHotspot,
      interaction_setup: interactionSetup,
      interaction_lane: surfacedInteractionLane,
      local_focus: localFocus,
      scene:
        activeScript || activeBgEvent
          ? {
              active_script: activeScript,
              scene_owner: sceneOwner,
            }
          : undefined,
      prompt: {
        pending: bootMode ? true : modal.prompt_pending,
        reason: bootMode ?? promptStatus.reason ?? (modal.prompt_pending ? "prompt" : undefined),
      },
      dialogue: {
        waiting_for_input: modal.text_advance_pending || undefined,
      },
      engine_debug: {
        movement_lock_count:
          typeof overworld?._movement_lock_count === "number" && Number.isFinite(overworld._movement_lock_count)
            ? overworld._movement_lock_count
            : undefined,
        text_lock_active: typeof overworld?._text_lock_active === "boolean" ? overworld._text_lock_active : undefined,
        blocking_task_count:
          typeof overworld?._blocking_task_count === "number" && Number.isFinite(overworld._blocking_task_count)
            ? overworld._blocking_task_count
            : undefined,
        blocking_movement_lock_active:
          typeof overworld?._blocking_movement_lock_active === "boolean"
            ? overworld._blocking_movement_lock_active
            : undefined,
        script_runner: overworld?.script_runner
          ? {
              stack_depth: Array.isArray(overworld.script_runner._script_stack)
                ? overworld.script_runner._script_stack.length
                : undefined,
              awaiting_resume:
                typeof overworld.script_runner._awaiting_resume === "number" &&
                Number.isFinite(overworld.script_runner._awaiting_resume)
                  ? overworld.script_runner._awaiting_resume
                  : undefined,
              stop_execution:
                typeof overworld.script_runner.stop_execution === "boolean"
                  ? overworld.script_runner.stop_execution
                  : undefined,
              is_busy:
                typeof overworld.script_runner.is_busy === "boolean" ? overworld.script_runner.is_busy : undefined,
              state:
                overworld.script_runner.state === undefined || overworld.script_runner.state === null
                  ? undefined
                  : String(overworld.script_runner.state),
            }
          : undefined,
      },
      coords,
      map,
      map_details: mapDetails,
      location_name: locationName,
      map_id: mapId,
      badges_count: badgesCount,
      money: walletMoney,
      moms_money: momsMoney,
      mom_saving_some_money: momSavingSomeMoney,
      resources: {
        money: walletMoney,
        moms_money: momsMoney,
        mom_saving_some_money: momSavingSomeMoney,
      },
      party: partyPokemon.length
        ? {
            count: partyPokemon.length,
            lead: leadPokemon
              ? {
                  species: formatMcpSpeciesLabel(leadPokemon.species),
                  level:
                    typeof leadPokemon.level === "number" && Number.isFinite(leadPokemon.level)
                      ? leadPokemon.level
                      : undefined,
                  hp:
                    typeof leadPokemon.hp === "number" && Number.isFinite(leadPokemon.hp)
                      ? leadPokemon.hp
                      : undefined,
                  maxHp:
                    typeof leadPokemon.max_hp === "number" && Number.isFinite(leadPokemon.max_hp)
                      ? leadPokemon.max_hp
                      : undefined,
                  status: leadPokemon.status ? String(leadPokemon.status) : undefined,
                }
              : undefined,
          }
        : undefined,
      party_summary: partyPokemon.length
        ? {
            count: partyPokemon.length,
            lead_species: leadPokemon ? formatMcpSpeciesLabel(leadPokemon.species) : undefined,
            lead_level:
              leadPokemon && typeof leadPokemon.level === "number" && Number.isFinite(leadPokemon.level)
                ? leadPokemon.level
                : undefined,
          }
        : {
            count: 0,
          },
      last_n_events: this.actionEvents.slice(-3).map((event) => ({
        action: event.action,
        frame: event.frame,
        mode: event.mode,
        summary: event.summary,
        map: event.map,
        coords: event.coords ? { ...event.coords } : undefined,
      })),
      last_action_result: this.lastActionResult ?? undefined,
      last_mcp_meta: this.lastSnapshot?.mcp ?? this.lastMcpMeta ?? undefined,
    };
  }

  async mapInfo(): Promise<McpMapInfoSnapshot> {
    await this.ensureReady();
    const game = this.getGame();
    this.settlePassiveBattleHandoffIfNeeded(game);
    const synchronizedSnapshot = this.ensureFrameConsistentSnapshot("map_info");
    if (synchronizedSnapshot.map) {
      return synchronizedSnapshot.map;
    }
    const state = game.getGameState();
    const overworld = game.getOverworld() as unknown as OverworldMapInfoSource;

    const map = this.readBestMapName(game) ?? null;
    const facing = this.readFacingDirection(game);
    return buildMapInfoSnapshot({
      map,
      mapGroup: state.wram.wMapGroup,
      mapNumber: state.wram.wMapNumber,
      overworld,
      playerCoords: this.readBestCoords(game),
      facing: facing === "unknown" ? undefined : facing,
      dataLoader: resolveSessionMapDataLoader(state),
      eventFlags: (state.wram.event_flags ?? null) as Record<string, boolean | undefined> | null,
    });
  }

  async routeRender(options: { detail?: RouteRenderDetail } = {}): Promise<McpRouteRenderSnapshot> {
    await this.ensureReady();
    const status = await this.status();
    const mapName = status.map ?? null;
    const mapId = status.map_id ?? null;
    if (status.mode !== "overworld") {
      return buildUnavailableRouteRenderSnapshot(
        `route_render is only available in overworld mode; current mode is ${status.mode}.`,
        mapName,
        mapId
      );
    }

    const game = this.getGame();
    const state = game.getGameState();
    const overworld = game.getOverworld() as unknown as OverworldMapInfoSource & {
      _map_events?: { coord_events?: Array<{ x: number; y: number; scene_id?: string; script_name?: string }> } | null;
    };
    const mapData = overworld.map;
    const tileset = overworld.tileset;
    if (!mapData || !tileset) {
      return buildUnavailableRouteRenderSnapshot(
        "route_render requires the live overworld map and tileset.",
        mapName,
        mapId
      );
    }
    if (!Array.isArray(tileset.metatiles) || tileset.metatiles.length === 0) {
      return buildUnavailableRouteRenderSnapshot(
        "route_render requires loaded tileset metatiles.",
        mapName,
        mapId
      );
    }

    const mapInfo = await this.mapInfo();
    const player = mapInfo.player
      ? {
          coords: mapInfo.player.coords,
          facing: mapInfo.player.facing,
        }
      : undefined;
    return buildRouteRenderSnapshot({
      map: mapInfo.map,
      mapId: mapInfo.map_id,
      coordStride: mapInfo.coord_stride,
      player,
      mapData,
      tileset,
      playerState: overworld.player_state ?? PlayerState.NORMAL,
      warps: mapInfo.warps,
      hotspots: mapInfo.hotspots,
      mapEvents: overworld._map_events ?? null,
      currentScene: typeof state.wram.scene_name === "string" ? state.wram.scene_name : null,
      eventFlags: (state.wram.event_flags ?? null) as Record<string, boolean | undefined> | null,
      dataLoader: resolveSessionMapDataLoader(state),
      detail: options.detail ?? "compact",
    });
  }

  async routeRenderImage(
    snapshot: McpRouteRenderSnapshot,
    options: { cellSize?: number } = {}
  ): Promise<InstanceType<typeof gameEngine.Surface>> {
    await this.ensureReady();
    if (!snapshot.available) {
      throw new Error(snapshot.reason ?? "route_render image is unavailable for the current session.");
    }

    const game = this.getGame();
    const state = game.getGameState();
    const overworld = game.getOverworld() as unknown as OverworldMapInfoSource;
    const mapData = overworld.map;
    const tileset = overworld.tileset;
    if (!mapData || !tileset) {
      throw new Error("route_render image requires the live overworld map and tileset.");
    }
    await this.waitForTilesetReady(tileset);
    if (!Array.isArray(tileset.metatiles) || tileset.metatiles.length === 0 || tileset.loaded === false) {
      throw new Error("route_render image requires loaded high-fidelity tileset data.");
    }

    return renderRouteRenderTileSurface({
      snapshot,
      mapData,
      tileset,
      vram: state.vram,
    }, {
      cellSize: options.cellSize,
    });
  }

  getAudioPlaybackSnapshot(): AudioPlaybackSnapshot | undefined {
    return this.game?.getAudioPlaybackSnapshot?.();
  }

  async recentEvents(limit = 10): Promise<McpRecentEventsSnapshot> {
    await this.ensureReady();
    const now = Date.now();
    const normalizedLimit = Math.max(1, Math.min(Math.floor(limit || 1), RECENT_EVENT_LIMIT));
    const events = this.actionEvents
      .slice(-normalizedLimit)
      .map((event) => this.buildStableActionEvent(event));
    const latest = events[events.length - 1];
    const recapRaw = latest
      ? `${latest.summary ?? this.describeActionResult(latest.action, latest.result)}${latest.map ? ` @ ${latest.map}` : ""}${latest.coords ? ` ${latest.coords.x},${latest.coords.y}` : ""}${latest.moments?.length ? ` | ${this.humanizeMoment(latest.moments[0] ?? "")}` : ""}`
      : "no_events";
    const recap = this.clampText(recapRaw, EVENT_RECAP_MAX_LEN);
    return {
      recap,
      total: this.actionEventTotal,
      session_started_at_ms: this.sessionStartedAtMs,
      session_started_at_iso: new Date(this.sessionStartedAtMs).toISOString(),
      time_played_ms: Math.max(0, now - this.sessionStartedAtMs),
      truncated: this.actionEventTotal > events.length,
      events,
    };
  }

  private clampText(value: string | null | undefined, maxLength: number): string {
    const normalized = String(value ?? "").replace(/\s+/g, " ").trim();
    if (!normalized) {
      return "";
    }
    if (normalized.length <= maxLength) {
      return normalized;
    }
    return normalized.slice(0, maxLength - 1).trimEnd() + "…";
  }

  private buildStableActionEvent(event: McpActionEvent): McpActionEvent {
    return {
      frame: event.frame,
      timestamp_ms: event.timestamp_ms,
      timestamp_iso: event.timestamp_iso,
      action: this.clampText(event.action, EVENT_STRING_MAX_LEN),
      mode: event.mode,
      map: event.map ? this.clampText(event.map, EVENT_STRING_MAX_LEN) : undefined,
      coords: event.coords ? { ...event.coords } : undefined,
      prompt: event.prompt ? this.clampText(event.prompt, EVENT_STRING_MAX_LEN) : undefined,
      moments: event.moments
        ? event.moments
            .slice(0, EVENT_MOMENTS_MAX)
            .map((moment) => this.clampText(moment, EVENT_STRING_MAX_LEN))
            .filter((moment) => moment.length > 0)
        : undefined,
      summary: event.summary ? this.clampText(event.summary, EVENT_SUMMARY_MAX_LEN) : undefined,
      result: {
        ok: event.result.ok,
        changed: event.result.changed,
        reason: event.result.reason,
        events: event.result.events
          ? event.result.events
              .slice(0, EVENT_RESULT_EVENTS_MAX)
              .map((detail) => this.clampText(detail, EVENT_STRING_MAX_LEN))
              .filter((detail) => detail.length > 0)
          : undefined,
      },
    };
  }

  private getScriptBusyReason(game: Game): "script_tasks" | "script_runner" | null {
    const overworld = game.getOverworld();
    if (
      typeof (overworld as { script_tasks_active?: () => boolean }).script_tasks_active === "function" &&
      (overworld as { script_tasks_active?: () => boolean }).script_tasks_active?.()
    ) {
      return "script_tasks";
    }
    const runner = (overworld as {
      script_runner?: { is_busy?: boolean; _script_stack?: unknown[]; _awaiting_resume?: number } | null;
    }).script_runner;
    if (runner && runner.is_busy) {
      return "script_runner";
    }
    if (runner && ((runner._script_stack?.length ?? 0) > 0 || (runner._awaiting_resume ?? 0) > 0)) {
      return "script_runner";
    }
    return null;
  }

  private getModalUiState(game: Game): ModalUiState {
    const bootMode = this.readGameMode(game);
    if (this.isBootUiMode(bootMode)) {
      return {
        in_battle: false,
        in_menu: true,
        in_dialog: true,
        text_box_open: false,
        text_advance_pending: false,
        prompt_pending: true,
        input_capture_active: false,
        movement_locked: true,
        script_busy: false,
        input_blocked_reason: bootMode,
        can_move: false,
      };
    }
    const promptStatus = promptFromSnapshot(this.lastSnapshot);
    const inBattle = game.isBattleActive?.() ?? false;
    const inMenu = this.isMenuOpenForSession(game);
    const dialogueState = this.getDialogueUiState(game);
    const unownPuzzleActive = this.isUnownPuzzleInputActive(game);
    const nonBlockingPcUi = isNonBlockingPcUiSnapshot(this.lastSnapshot);
    const dialogueOwnsInput = nonBlockingPcUi ? false : dialogueState.dialog_active;
    const inDialog = Boolean(this.lastSnapshot?.dialogue?.length) || dialogueOwnsInput;
    const inputCaptureActive = this.isInputCaptureActive(game);
    const promptPending = nonBlockingPcUi
      ? false
      : Boolean(
          !unownPuzzleActive &&
            (promptStatus.pending ||
              dialogueState.yes_no_prompt_open ||
              (!inMenu && !inDialog && inputCaptureActive))
        );
    const textBoxOpen = unownPuzzleActive
      ? false
      : promptPending || inDialog || (nonBlockingPcUi ? false : dialogueState.text_box_open);
    const movementLocked = this.isMovementLocked(game);
    const scriptBusyReason = this.getScriptBusyReason(game);
    let inputBlockedReason = this.getStopReason(game, null);
    if (!unownPuzzleActive && !inputBlockedReason && inDialog) {
      inputBlockedReason = "dialogue";
    }
    const canMove = unownPuzzleActive || !inputBlockedReason;
    return {
      in_battle: inBattle,
      in_menu: inMenu,
      in_dialog: inDialog,
      text_box_open: textBoxOpen,
      text_advance_pending: unownPuzzleActive || nonBlockingPcUi ? false : dialogueState.text_advance_pending,
      prompt_pending: promptPending,
      input_capture_active: inputCaptureActive,
      movement_locked: movementLocked,
      script_busy: Boolean(scriptBusyReason),
      input_blocked_reason: inputBlockedReason,
      can_move: canMove,
    };
  }

  private buildMacroRawInput(actions: MacroAction[]): {
    actions: McpMacroTraceAction[];
    truncated?: boolean;
    total?: number;
  } {
    const limited = actions.slice(0, MACRO_TRACE_ACTION_LIMIT);
    const traceActions = limited.map((action, index) => this.buildMacroTraceAction(action, index));
    return {
      actions: traceActions,
      truncated: actions.length > traceActions.length ? true : undefined,
      total: actions.length,
    };
  }

  private buildNormalizedMacroActions(
    actions: MacroAction[],
    defaultDelay: number
  ): McpMacroTraceAction[] {
    return actions.slice(0, MACRO_TRACE_ACTION_LIMIT).map((action, index) => {
      if (action.type === "move") {
        return {
          index,
          type: "move",
          value: action.value,
          times: this.normalizeTimes(action.times ?? 1),
          hold_frames: this.normalizeHoldFrames(action.hold_frames),
          delay_frames: this.normalizeDelayFrames(action.delay_frames ?? defaultDelay),
        };
      }
      if (action.type === "button") {
        return {
          index,
          type: "button",
          value: action.value,
          times: this.normalizeTimes(action.times ?? 1),
          hold_frames: this.normalizeHoldFrames(action.hold_frames),
          delay_frames: this.normalizeDelayFrames(action.delay_frames ?? defaultDelay),
        };
      }
      return {
        index,
        type: "wait",
        frames: this.normalizeFrames(action.frames ?? 1),
        delay_frames: this.normalizeDelayFrames(action.delay_frames ?? defaultDelay),
      };
    });
  }

  private buildMacroTraceAction(action: MacroAction, index: number): McpMacroTraceAction {
    if (action.type === "move") {
      return {
        index,
        type: "move",
        value: action.value,
        times: action.times,
        hold_frames: action.hold_frames,
        delay_frames: action.delay_frames,
      };
    }
    if (action.type === "button") {
      return {
        index,
        type: "button",
        value: action.value,
        times: action.times,
        hold_frames: action.hold_frames,
        delay_frames: action.delay_frames,
      };
    }
    return {
      index,
      type: "wait",
      frames: action.frames,
      delay_frames: action.delay_frames,
    };
  }

  private readCoordsAndMap(game: Game): { coords?: { x: number; y: number }; map?: string } {
    return {
      coords: this.readBestCoords(game),
      map: this.readBestMapName(game),
    };
  }

  private appendTraceStep(steps: McpMacroTraceStep[], step: McpMacroTraceStep): void {
    steps.push(step);
    if (steps.length > MACRO_TRACE_STEP_LIMIT) {
      steps.splice(0, steps.length - MACRO_TRACE_STEP_LIMIT);
    }
  }

  private clearStaleDirectionalInput(game: Game): boolean {
    let cleared = false;
    if (this.scheduledEvents.length > 0) {
      this.scheduledEvents = [];
      cleared = true;
    }
    const overworld = game.getOverworld() as {
      _queued_direction?: unknown;
      _held_directions?: unknown;
    };
    if (Object.prototype.hasOwnProperty.call(overworld, "_queued_direction") && overworld._queued_direction !== null) {
      overworld._queued_direction = null;
      cleared = true;
    }
    if (overworld._held_directions instanceof Map && overworld._held_directions.size > 0) {
      overworld._held_directions.clear();
      cleared = true;
    }
    return cleared;
  }

  private debugMacroLog(message: string, details?: Record<string, unknown>): void {
    if (!this.debugInputEnabled) {
      return;
    }
    if (!details) {
      pushDebugLog(message);
      return;
    }
    pushDebugLog(message, {
      payload: JSON.stringify(details),
    });
  }

  async executeNamedMacro(
    macro: NamedMacroName,
    options: NamedMacroOptions = {}
  ): Promise<ActionResultWithSnapshot> {
    await this.ensureReady();
    const game = this.getGame();
    this.resetStaleButtonReleaseGuards(game, "a");
    this.resetStaleButtonReleaseGuards(game, "b");
    const beforeSignal = this.captureSceneSignal(game);
    const before = this.buildStateFingerprint(game);
    const requested = this.normalizeTimes(options.maxPresses ?? 8);
    const settleFrames = this.normalizeDelayFrames(options.settleFrames ?? 0);
    const holdFrames = this.normalizeHoldFrames(1);
    let completed = 0;
    let finishedByClear = false;
    let stoppedOnPrompt = false;
    let elapsedFrames = 0;
    let lastPressAt = Number.NEGATIVE_INFINITY;
    let busyWaitCount = 0;
    let closeMenuCount = 0;
    let nudgedChoiceCount = 0;
    const reasonCodes = new Set<NamedMacroReasonCode>();
    const nudgeDirections: Direction[] = ["right", "left", "up", "down"];

    const advanceWithAccounting = (frames: number): void => {
      if (frames <= 0) {
        return;
      }
      this.stepFrames(frames);
      elapsedFrames += frames;
    };

    const shouldAttemptChoiceNudge = (iteration: number): boolean => {
      return iteration > 0 && iteration % 3 === 0;
    };

    for (let i = 0; i < requested; i += 1) {
      const modalBefore = this.getModalUiState(game);
      if (
        !modalBefore.prompt_pending &&
        !modalBefore.in_dialog &&
        !modalBefore.in_menu &&
        !modalBefore.movement_locked &&
        !modalBefore.script_busy
      ) {
        reasonCodes.add("no_effect");
        break;
      }

      if (modalBefore.in_menu && !modalBefore.prompt_pending && !modalBefore.in_dialog) {
        reasonCodes.add("closed_menu");
        closeMenuCount += 1;
        this.recordAction(formatActionLabel("button", "b", `macro:${macro}:close_menu`));
        this.resetStaleButtonReleaseGuards(game, "b");
        this.scheduleKeyPress({
          key: keyForButton("b"),
          button: "b",
          holdFrames,
        });
        advanceWithAccounting(holdFrames + 2);
        continue;
      }

      if (
        (modalBefore.movement_locked || modalBefore.script_busy) &&
        !(modalBefore.prompt_pending || modalBefore.in_dialog)
      ) {
        reasonCodes.add("busy_wait");
        const backoffIndex = Math.min(busyWaitCount, DIALOG_MACRO_BUSY_BACKOFF_FRAMES.length - 1);
        const waitFrames = DIALOG_MACRO_BUSY_BACKOFF_FRAMES[backoffIndex] ?? 1;
        busyWaitCount += 1;
        advanceWithAccounting(waitFrames);
        continue;
      }

      busyWaitCount = 0;

      if (modalBefore.prompt_pending && shouldAttemptChoiceNudge(i)) {
        const nudge = nudgeDirections[nudgedChoiceCount % nudgeDirections.length] ?? "down";
        reasonCodes.add("nudged_choice");
        nudgedChoiceCount += 1;
        this.recordAction(formatActionLabel("move", nudge, `macro:${macro}:nudge`));
        this.scheduleKeyPress({
          key: keyForDirection(nudge),
          direction: nudge,
          holdFrames,
        });
        advanceWithAccounting(holdFrames + 1);
      }

      const framesSinceLastPress = elapsedFrames - lastPressAt;
      if (Number.isFinite(lastPressAt) && framesSinceLastPress < DIALOG_MACRO_MIN_PRESS_INTERVAL_FRAMES) {
        advanceWithAccounting(DIALOG_MACRO_MIN_PRESS_INTERVAL_FRAMES - framesSinceLastPress);
      }

      const jitter = i % 2;
      const effectiveHold = modalBefore.prompt_pending ? Math.max(holdFrames, 2 + jitter) : holdFrames + jitter;
      const postPressJitter = (i % 3) + 1;

      this.recordAction(formatActionLabel("button", "a", `macro:${macro}`));
      this.resetStaleButtonReleaseGuards(game, "a");
      this.scheduleKeyPress({
        key: keyForButton("a"),
        button: "a",
        holdFrames: effectiveHold,
      });
      lastPressAt = elapsedFrames;
      advanceWithAccounting(effectiveHold + 1 + settleFrames + postPressJitter);
      completed += 1;

      const modalAfter = this.getModalUiState(game);
      if (!modalBefore.prompt_pending && modalAfter.prompt_pending) {
        reasonCodes.add("prompt_opened");
        finishedByClear = true;
        stoppedOnPrompt = true;
        break;
      }
      if (
        (modalBefore.in_dialog || modalBefore.prompt_pending) &&
        modalAfter.in_menu &&
        !modalAfter.prompt_pending &&
        !modalAfter.in_dialog
      ) {
        reasonCodes.add("advanced");
        finishedByClear = true;
        break;
      }
      if (!modalAfter.input_blocked_reason) {
        reasonCodes.add("advanced");
        finishedByClear = true;
        break;
      }
      if (!modalAfter.in_menu && !modalAfter.prompt_pending && !modalAfter.in_dialog && !modalAfter.movement_locked) {
        reasonCodes.add("advanced");
      }
      if (
        (modalAfter.movement_locked || modalAfter.script_busy) &&
        !(modalAfter.prompt_pending || modalAfter.in_dialog)
      ) {
        reasonCodes.add("busy_wait");
        const backoffIndex = Math.min(busyWaitCount, DIALOG_MACRO_BUSY_BACKOFF_FRAMES.length - 1);
        const waitFrames = DIALOG_MACRO_BUSY_BACKOFF_FRAMES[backoffIndex] ?? 1;
        busyWaitCount += 1;
        advanceWithAccounting(waitFrames);
      }
    }

    const changed = before !== this.buildStateFingerprint(game);
    if (!reasonCodes.has("advanced") && changed) {
      reasonCodes.add("advanced");
    }
    if (!reasonCodes.size) {
      reasonCodes.add("no_effect");
    }
    const reasonOrder = [
      "advanced",
      "busy_wait",
      "closed_menu",
      "nudged_choice",
      "prompt_opened",
      "no_effect",
    ] as const;
    const orderedReasons = reasonOrder.filter((code) => reasonCodes.has(code));
    const primaryReason: NamedMacroReasonCode = orderedReasons[0] ?? "no_effect";

    this.lastMcpMeta = {
      macro_summary: {
        requested,
        completed,
        stopped: finishedByClear,
        reason: primaryReason,
        reason_codes: orderedReasons,
        close_menu_count: closeMenuCount,
        busy_wait_count: busyWaitCount,
        nudged_choice_count: nudgedChoiceCount,
      },
      macro_execution_trace: {
        raw_input: {
          macro,
          actions: [],
          total: 0,
        },
        normalized_actions: {
          actions: [],
          total: 0,
        },
        executed_actions: {
          steps: [],
          total: 0,
        },
      },
    };
    this.captureSnapshot();
    const result = await this.finalizeActionResult({
      reason: changed ? undefined : "no_change",
      changed,
      events: [
        `macro:${macro}`,
        `pressed:${completed}/${requested}`,
        stoppedOnPrompt ? "stopped_on_prompt" : finishedByClear ? "cleared_modal" : "limit_reached",
        ...orderedReasons.map((reasonCode) => `reason:${reasonCode}`),
      ],
    });
    this.recordActionEvent(`execute_macro:${macro}:${completed}/${requested}`, result, {
      before: beforeSignal,
    });
    return { result, snapshotText: this.observeText() };
  }

  async executeMacro(actions: MacroAction[], options: MacroOptions = {}): Promise<ActionResultWithSnapshot> {
    await this.ensureReady();
    const game = this.getGame();
    const beforeSignal = this.captureSceneSignal(game);
    const before = this.buildStateFingerprint(game);
    const baselineMap = this.readMapIdentity(game);
    const stopOnEvent = options.stop_on_event === true;
    const defaultDelay = this.normalizeDelayFrames(options.delay_frames);
    const rawInput = this.buildMacroRawInput(actions);
    const normalizedActions = this.buildNormalizedMacroActions(actions, defaultDelay);
    const staleInputCleared = this.clearStaleDirectionalInput(game);
    const executedSteps: McpMacroTraceStep[] = [];
    let executedTotal = 0;
    let completedActions = 0;
    let stopReason: string | null = null;

    this.debugMacroLog("[mcp] execute_macro request", {
      macro: null,
      stop_on_event: stopOnEvent,
      delay_frames: defaultDelay,
      raw_input_actions: rawInput.actions,
      raw_input_total: rawInput.total,
      stale_input_cleared: staleInputCleared,
    });

    for (let index = 0; index < actions.length; index += 1) {
      const action = actions[index];
      const normalized = normalizedActions[index] ?? this.buildMacroTraceAction(action, index);
      const beforeAction = this.readCoordsAndMap(game);
      if (stopOnEvent) {
        stopReason = this.getStopReason(game, baselineMap);
        if (stopReason) {
          break;
        }
      }

      if (action.type === "move") {
        const outcome = await this.performMove(action.value as Direction, normalized.times ?? 1, {
          stopOnEvent,
          holdFrames: normalized.hold_frames,
        });
        executedTotal += 1;
        this.appendTraceStep(executedSteps, {
          index,
          action: normalized,
          before: beforeAction,
          after: {
            coords: { x: outcome.end[0], y: outcome.end[1] },
            map: outcome.map,
          },
          stop_reason: outcome.stopReason ?? undefined,
          block_reason: outcome.blockReason,
        });
        this.debugMacroLog("[mcp] execute_macro step", {
          index,
          action: normalized,
          before: beforeAction,
          after: { coords: { x: outcome.end[0], y: outcome.end[1] }, map: outcome.map },
          stop_reason: outcome.stopReason,
          block_reason: outcome.blockReason,
        });
        completedActions += 1;
        if (stopOnEvent && outcome.stopReason) {
          stopReason = outcome.stopReason;
          break;
        }
      } else if (action.type === "button") {
        const holdFrames = normalized.hold_frames ?? this.normalizeHoldFrames(action.hold_frames);
        const times = normalized.times ?? this.normalizeTimes(action.times ?? 1);
        for (let count = 0; count < times; count += 1) {
          this.recordAction(formatActionLabel("button", action.value as Button));
          this.scheduleKeyPress({
            key: keyForButton(action.value as Button),
            button: action.value as Button,
            holdFrames,
          });
          this.stepFrames(holdFrames + 1);
        }
        executedTotal += 1;
        const afterAction = this.readCoordsAndMap(game);
        this.appendTraceStep(executedSteps, {
          index,
          action: {
            ...normalized,
            times,
            hold_frames: holdFrames,
          },
          before: beforeAction,
          after: afterAction,
        });
        this.debugMacroLog("[mcp] execute_macro step", {
          index,
          action: {
            ...normalized,
            times,
            hold_frames: holdFrames,
          },
          before: beforeAction,
          after: afterAction,
        });
        completedActions += 1;
      } else if (action.type === "wait") {
        const waitFrames = normalized.frames ?? this.normalizeFrames(action.frames ?? 1);
        await this.advanceFrames(waitFrames);
        executedTotal += 1;
        const afterAction = this.readCoordsAndMap(game);
        this.appendTraceStep(executedSteps, {
          index,
          action: {
            ...normalized,
            frames: waitFrames,
          },
          before: beforeAction,
          after: afterAction,
        });
        this.debugMacroLog("[mcp] execute_macro step", {
          index,
          action: {
            ...normalized,
            frames: waitFrames,
          },
          before: beforeAction,
          after: afterAction,
        });
        completedActions += 1;
      }

      if (stopOnEvent) {
        stopReason = this.getStopReason(game, baselineMap);
        if (stopReason) {
          break;
        }
      }

      if (index < actions.length - 1) {
        const delayFrames = normalized.delay_frames ?? this.normalizeDelayFrames(action.delay_frames ?? defaultDelay);
        if (delayFrames > 0) {
          await this.advanceFrames(delayFrames);
        }
      }
    }

    this.lastMcpMeta = {
      macro_summary: {
        requested: actions.length,
        completed: completedActions,
        stopped: Boolean(stopReason),
        reason: stopReason ?? undefined,
      },
      macro_execution_trace: {
        raw_input: {
          macro: undefined,
          actions: rawInput.actions,
          truncated: rawInput.truncated,
          total: rawInput.total,
        },
        normalized_actions: {
          actions: normalizedActions,
          truncated: actions.length > normalizedActions.length ? true : undefined,
          total: actions.length,
        },
        executed_actions: {
          steps: [...executedSteps],
          truncated: executedTotal > executedSteps.length ? true : undefined,
          total: executedTotal,
        },
        stop_reason: stopReason,
        interruption: stopReason,
        stale_input_cleared: staleInputCleared || undefined,
      },
    };
    this.captureSnapshot();
    const changed = before !== this.buildStateFingerprint(game);
    const events = [`macro:${completedActions}/${actions.length}`];
    if (stopReason) {
      events.push(`interrupted:${stopReason}`);
    }
    events.push(`trace_steps:${executedSteps.length}`);
    if (staleInputCleared) {
      events.push("stale_input_cleared");
    }
    const result = await this.finalizeActionResult({
      reason: this.mapActionReason({
        blocked: false,
        stopReason,
        changed,
      }),
      changed,
      events,
    });
    this.recordActionEvent(`execute_macro:${completedActions}/${actions.length}`, result, {
      before: beforeSignal,
    });
    this.debugMacroLog("[mcp] execute_macro complete", {
      completed_actions: completedActions,
      requested_actions: actions.length,
      stop_reason: stopReason,
      trace_steps: executedSteps.length,
    });
    return { result, snapshotText: this.observeText() };
  }

  private scheduleKeyPress(options: {
    key: string;
    direction?: string;
    button?: string;
    holdFrames?: number;
    repeatPressFrames?: boolean;
  }): void {
    const pressEvent = new gameEngine.event.Event(gameEngine.KEYDOWN, {
      key: options.key,
      code: options.key,
      direction: options.direction ?? null,
      button: options.button ?? null,
      is_press: true,
    });
    const releaseEvent = new gameEngine.event.Event(gameEngine.KEYUP, {
      key: options.key,
      code: options.key,
      direction: options.direction ?? null,
      button: options.button ?? null,
      is_press: false,
    });
    const holdFrames = options.holdFrames ?? this.holdFrames;
    const releaseFrame = this.frameCounter + holdFrames;
    this.scheduledEvents.push({
      frame: this.frameCounter,
      event: pressEvent,
    });
    if (options.repeatPressFrames) {
      for (let offset = 1; offset < holdFrames; offset += 1) {
        this.scheduledEvents.push({
          frame: this.frameCounter + offset,
          event: new gameEngine.event.Event(gameEngine.KEYDOWN, {
            key: options.key,
            code: options.key,
            direction: options.direction ?? null,
            button: options.button ?? null,
            is_press: true,
          }),
        });
      }
    }
    this.scheduledEvents.push({
      frame: releaseFrame,
      event: releaseEvent,
    });
  }

  async postInputEvent(input: {
    key: string;
    direction?: string | null;
    button?: string | null;
    isPress: boolean;
  }): Promise<void> {
    await this.ensureReady();
    const event = new gameEngine.event.Event(input.isPress ? gameEngine.KEYDOWN : gameEngine.KEYUP, {
      key: input.key,
      code: input.key,
      direction: input.direction ?? null,
      button: input.button ?? null,
      is_press: input.isPress,
    });
    this.getGame().postEvent(event);
  }

  private scheduleTextInput(text: string): void {
    const key = /^[a-z]$/i.test(text) ? `Key${text.toUpperCase()}` : text;
    this.scheduledEvents.push({
      frame: this.frameCounter,
      event: new gameEngine.event.Event(gameEngine.KEYDOWN, {
        key,
        code: key,
        text,
        unicode: text,
        is_press: true,
      }),
    });
  }

  private async waitForInputOwningSurfaceSettle(): Promise<void> {
    await new Promise<void>((resolve) => {
      setTimeout(resolve, INPUT_OWNING_SURFACE_SETTLE_MS);
    });
  }

  private stepFrames(count: number): void {
    const game = this.getGame();
    const isInstantMode = this.isInstantMode();

    for (let i = 0; i < count; i += 1) {
      this.flushScheduledEvents();
      this.withMcpDownhillSuppressed(game, () => game.tick());
      // Skip snapshot capture for middle frames if instant mode is active
      // to avoid excessive overhead. snapshot is captured at the very end.
      if (!isInstantMode || i === count - 1) {
        this.captureSnapshot();
      }
      this.frameCounter += 1;
    }

    // Ensure at least one snapshot is captured if count is 0
    if (count <= 0 && !this.lastSnapshot) {
      this.captureSnapshot();
    }
  }

  private withMcpDownhillSuppressed(game: Game, callback: () => void): void {
    const gameState = game.getGameState?.() as {
      wram?: {
        wBikeFlags?: number;
        engine_flags?: { ENGINE_DOWNHILL?: boolean };
      };
    } | null | undefined;
    const wram = gameState?.wram;
    if (!wram) {
      callback();
      return;
    }

    const hadBikeFlags = typeof wram.wBikeFlags === "number";
    const originalBikeFlags = Number(wram.wBikeFlags ?? 0);
    const engineFlags = wram.engine_flags;
    const hadEngineDownhill = Boolean(
      engineFlags && Object.prototype.hasOwnProperty.call(engineFlags, "ENGINE_DOWNHILL")
    );
    const originalEngineDownhill = engineFlags?.ENGINE_DOWNHILL;

    wram.wBikeFlags = originalBikeFlags & ~0x04;
    if (engineFlags) {
      engineFlags.ENGINE_DOWNHILL = false;
    }
    try {
      callback();
    } finally {
      if (hadBikeFlags) {
        wram.wBikeFlags = originalBikeFlags;
      } else {
        delete wram.wBikeFlags;
      }
      if (engineFlags && hadEngineDownhill) {
        engineFlags.ENGINE_DOWNHILL = originalEngineDownhill;
      } else if (engineFlags) {
        delete engineFlags.ENGINE_DOWNHILL;
      }
    }
  }

  private getGame(): Game {
    if (!this.game) {
      throw new Error("Game session not initialized; call ensureReady() before stepping.");
    }
    return this.game;
  }

  private async initGame(): Promise<void> {
    const settings = getSettings();
    if (this.game) {
      return;
    }
    const identityContext = getMcpIdentityContext();
    // Keep long-lived MCP sessions resumable from the local autosave slot even when
    // the API request itself is running under an identity-scoped context.
    const strictLoadSlot = false;
    if (!this.gamePromise) {
      const autosaveSlot = `mcp-${this.sessionId}-autosave.sav`;
      const hasAutosave = await runWithMcpIdentityContext(null, () =>
        hasSaveGame(autosaveSlot).catch(() => false)
      );
      const startInteractiveNewGame = this.interactiveMode && !hasAutosave;
      const muteAudio = this.isInstantMode() ? true : !settings.mcpAllowAudio;
      this.gamePromise = runWithMcpIdentityContext(null, () =>
        Game.create(this.ui, {
          initialState: startInteractiveNewGame ? "main_menu" : "overworld",
          newGame: false,
          suppressBootAnimations: this.isInstantMode(),
          muted: muteAudio,
          loadSlot: startInteractiveNewGame ? undefined : autosaveSlot,
          strictLoadSlot,
          autosaveSlot,
        })
      );
    }
    this.game = await this.gamePromise;
    this.holdFrames = this.isInstantMode() ? 1 : settings.mcpHoldFrames;
    const identityProfile = await loadIdentityPlayProfile(identityContext?.playerId);
    const gameState = (
      this.game as {
        getGameState?: () => {
          sram?: {
            player_name?: string;
            player_gender?: PlayerGender;
            options?: { no_text_scroll?: boolean };
          };
          wram?: {
            player_gender?: PlayerGender;
            wPlayerGender?: PlayerGender;
            wOptions?: number;
            instant_mode?: boolean;
          };
        };
      }
    ).getGameState?.();
    const identityName = resolveSessionPlayerName(
      identityProfile?.playerName,
      identityContext?.name,
      gameState?.sram?.player_name
    );
    const currentName = String(gameState?.sram?.player_name ?? "").trim();
    if (gameState?.sram && identityName && (!currentName || currentName === "?????")) {
      gameState.sram.player_name = identityName;
    }
    this.applyInputModeToGameState(gameState);
    await this.restoreRuntimeSnapshot();
    this.applyInputModeToGameState(this.game.getGameState?.());
    const resolvedGender = resolveSessionPlayerGender(identityProfile, gameState);
    if (resolvedGender !== null) {
      this.game.setPlayerGender(resolvedGender);
    }
  }

  private flushScheduledEvents(): void {
    if (!this.scheduledEvents.length) {
      return;
    }
    const due: ScheduledEvent[] = [];
    const remaining: ScheduledEvent[] = [];
    for (const scheduled of this.scheduledEvents) {
      if (scheduled.frame <= this.frameCounter) {
        due.push(scheduled);
      } else {
        remaining.push(scheduled);
      }
    }
    this.scheduledEvents = remaining;
    const game = this.getGame();
    for (const scheduled of due) {
      game.postEvent(scheduled.event);
    }
  }

  private captureSnapshot(): void {
    this.syncOverworldPlayerState();
    let game = this.game as unknown as { draw?: () => void } | null;
    if (!game) {
      try {
        game = this.getGame() as unknown as { draw?: () => void };
      } catch {
        game = null;
      }
    }
    game?.draw?.();
    let snapshot = this.textUi.getSnapshot();
    if (!snapshot) {
      if (this.lastSnapshot) {
        return;
      }
      snapshot = this.buildFallbackTextSnapshot();
    }
    this.lastSnapshot = buildSnapshotPayload(snapshot, {
      actionLog: this.actionLog,
      script: {},
      tasks: [],
      mcp: this.lastMcpMeta ?? undefined,
      map: this.safeBuildSnapshotMapInfo(),
      notices: this.safeBuildSessionNotices(),
    });
    this.lastMcpMeta = null;
    this.lastSnapshotFrameCounter = this.frameCounter;
  }

  private safeBuildSnapshotMapInfo(): McpMapInfoSnapshot | undefined {
    try {
      return this.buildSnapshotMapInfo();
    } catch {
      return undefined;
    }
  }

  private safeBuildSessionNotices(): string[] | undefined {
    if (!this.game) {
      return undefined;
    }
    try {
      return this.buildSessionNotices(this.game);
    } catch {
      return undefined;
    }
  }

  private buildFallbackTextSnapshot(): TextSnapshot {
    const game = this.game;
    let map = "Unknown";
    let playerCoords: { x: number; y: number } | null = null;
    let facing: "up" | "down" | "left" | "right" | "unknown" = "unknown";
    let dialogueActive = false;
    let menuOpen = false;
    if (game) {
      try {
        map = this.readBestMapName(game) ?? "Unknown";
      } catch {
        map = "Unknown";
      }
      try {
        playerCoords = this.readBestCoords(game) ?? null;
      } catch {
        playerCoords = null;
      }
      try {
        facing = this.readFacingDirection(game);
      } catch {
        facing = "unknown";
      }
      try {
        dialogueActive = this.getDialogueUiState(game).dialog_active;
      } catch {
        dialogueActive = false;
      }
      try {
        menuOpen = this.isMenuOpenForSession(game);
      } catch {
        menuOpen = false;
      }
    }
    const coords = playerCoords
      ? `${playerCoords.x},${playerCoords.y}`
      : "unknown";
    return {
      viewportTitle: "Overworld",
      infoTitle: "Legend",
      viewportLines: [`MAP: ${map}`, `PLAYER: ${coords} ${facing}`],
      infoLines: [
        `map: ${map}`,
        `coords: ${coords}`,
        `facing: ${facing}`,
        `menu: ${menuOpen ? "open" : "closed"}`,
        `dialogue: ${dialogueActive ? "open" : "closed"}`,
      ],
      marker: playerCoords ? [playerCoords.x, playerCoords.y, "@"] : null,
      actionLog: [...this.actionLog],
      menuLines: null,
      promptLines: null,
      dialogueLines: null,
    };
  }

  private buildMinimalFallbackTextSnapshot(operation: string): TextSnapshot {
    return {
      viewportTitle: "Overworld",
      infoTitle: "Legend",
      viewportLines: [`MCP ${operation}: snapshot fallback`],
      infoLines: ["map: Unknown", "coords: unknown", "facing: unknown"],
      marker: null,
      actionLog: [...this.actionLog],
      menuLines: null,
      promptLines: null,
      dialogueLines: null,
    };
  }

  private buildSnapshotMapInfo(): McpMapInfoSnapshot | undefined {
    if (!this.game) {
      return undefined;
    }
    const game = this.game as typeof this.game & {
      getGameState?: () => {
        wram?: {
          wMapGroup?: number;
          wMapNumber?: number;
          event_flags?: Record<string, boolean | undefined> | null;
        };
        sram?: {
          badges?: {
            johto?: readonly boolean[];
            kanto?: readonly boolean[];
          } | null;
        };
      };
    };
    if (typeof game.getGameState !== "function") {
      return undefined;
    }
    const state = game.getGameState();
    const map = this.readBestMapName(game) ?? null;
    const facingValue = this.readFacingDirection(game);
    return buildMapInfoSnapshot({
      map,
      mapGroup: state.wram.wMapGroup,
      mapNumber: state.wram.wMapNumber,
      overworld: game.getOverworld() as unknown as OverworldMapInfoSource,
      playerCoords: this.readBestCoords(game),
      facing: facingValue === "unknown" ? undefined : facingValue,
      dataLoader: resolveSessionMapDataLoader(state),
      eventFlags: (state.wram.event_flags ?? null) as Record<string, boolean | undefined> | null,
    });
  }

  private buildSessionNotices(game: Game): string[] {
    const currentMap = (() => {
      try {
        return this.readBestMapName(game);
      } catch {
        const fallbackOverworld = game.getOverworld?.() as { current_map_name?: string } | null | undefined;
        return fallbackOverworld?.current_map_name ?? game.getMapName?.() ?? "";
      }
    })();
    const state = (() => {
      try {
        return game.getGameState?.();
      } catch {
        return null;
      }
    })();
    const recentBattleEnd = [...this.actionEvents]
      .slice(-8)
      .reverse()
      .find((event) => Array.isArray(event.moments) && event.moments.includes("battle_ended"));
    if (!recentBattleEnd?.map || !currentMap || recentBattleEnd.map === currentMap) {
      return [];
    }

    const destinationLooksSafe = /POKECENTER|CENTER|HOUSE/i.test(currentMap);
    if (destinationLooksSafe) {
      return [formatWhiteoutAsmText(state?.sram?.player_name)];
    }

    return [
      `Post-battle relocation: the recent battle ended on ${recentBattleEnd.map}, and you are now in ${currentMap}.`,
    ];
  }

  private recordAction(label: string): void {
    this.actionLog.push(label);
    if (this.actionLog.length > ACTION_LOG_LIMIT) {
      this.actionLog = this.actionLog.slice(-ACTION_LOG_LIMIT);
    }
  }

  private recordActionResult(result: ActionResult): ActionResult {
    this.lastActionResult = result;
    return result;
  }

  private async finalizeActionResult(options: {
    reason?: ActionResultReason;
    changed: boolean;
    events?: string[];
  }): Promise<ActionResult> {
    const reason = options.reason;
    const ok = reason === undefined || reason === "no_change";
    const result = this.recordActionResult({
      ok,
      reason,
      changed: options.changed,
      events: options.events,
    });

    // Freeze MCP session state after every completed input so the same session id
    // can resume across separate CLI invocations, even when the latest action only
    // affected UI state or ended in a blocked/no-change outcome.
    await this.requestAutosave({ force: true });

    return result;
  }

  private readAutosaveState(): GameState | null {
    if (!this.canPersistRealGameSave()) {
      return null;
    }
    this.syncOverworldPlayerState();
    const gameState = (this.game as unknown as { getGameState?: () => unknown } | null)?.getGameState?.();
    if (!gameState || typeof gameState !== "object") {
      return null;
    }
    const candidate = gameState as { sram?: unknown; wram?: unknown };
    if (!candidate.sram || !candidate.wram) {
      return null;
    }
    return gameState as GameState;
  }

  private canPersistRealGameSave(): boolean {
    const game = this.game as
      | {
          getDebugStatus?: () => {
            mode?: string;
            can_move?: boolean;
            canMove?: boolean;
            prompt_pending?: boolean;
            text_advance_pending?: boolean;
            in_dialog?: boolean;
            in_menu?: boolean;
            in_battle?: boolean;
            movement_locked?: boolean;
            script_busy?: boolean;
          };
          isBattleActive?: () => boolean;
          getOverworld?: () => unknown;
        }
      | null;
    if (!game) {
      return false;
    }

    const status = game.getDebugStatus?.();
    if (status) {
      const canMove = status.can_move ?? status.canMove ?? false;
      if (
        status.mode !== "overworld" ||
        !canMove ||
        status.prompt_pending ||
        status.text_advance_pending ||
        status.in_dialog ||
        status.in_menu ||
        status.in_battle ||
        status.movement_locked ||
        status.script_busy
      ) {
        return false;
      }
    }

    if (game.isBattleActive?.()) {
      return false;
    }

    const overworld = game.getOverworld?.() as
      | {
          player_x?: number;
          player_y?: number;
          player_direction?: string;
          player_state?: PlayerState | string | number | null;
          is_moving?: boolean;
          dialogue?: {
            active?: boolean;
            visible?: boolean;
            waiting_for_input?: boolean;
            pending_waits?: number;
            pendingWaits?: number;
            pending_script_waits?: number;
            pending_yes_no_request?: boolean;
            yes_no_prompt?: unknown;
            _yes_no_prompt?: unknown;
          } | null;
          script_runner?: {
            is_busy?: boolean;
            _script_stack?: unknown[];
            _awaiting_resume?: number;
            _queued_overworld_task_count?: number;
            stop_execution?: boolean;
          } | null;
          script_tasks_active?: () => boolean;
          player_movement_locked?: () => boolean;
          _movement_lock_count?: number;
          _text_lock_active?: boolean;
          input_capture_active?: boolean;
          _current_tile_permission?: () => number | null;
        }
      | null
      | undefined;
    if (!overworld) {
      return false;
    }

    const dialogue = overworld.dialogue;
    if (
      dialogue &&
      (dialogue.active ||
        dialogue.visible ||
        dialogue.waiting_for_input ||
        Number(dialogue.pending_waits ?? dialogue.pendingWaits ?? 0) > 0 ||
        Number(dialogue.pending_script_waits ?? 0) > 0 ||
        dialogue.pending_yes_no_request ||
        Boolean(dialogue.yes_no_prompt ?? dialogue._yes_no_prompt))
    ) {
      return false;
    }

    const runner = overworld.script_runner;
    if (
      runner &&
      (runner.is_busy ||
        (runner._script_stack?.length ?? 0) > 0 ||
        Number(runner._awaiting_resume ?? 0) > 0 ||
        Number(runner._queued_overworld_task_count ?? 0) > 0 ||
        runner.stop_execution)
    ) {
      return false;
    }

    if (
      overworld.is_moving ||
      overworld.script_tasks_active?.() ||
      overworld.player_movement_locked?.() ||
      Number(overworld._movement_lock_count ?? 0) > 0 ||
      overworld._text_lock_active ||
      overworld.input_capture_active
    ) {
      return false;
    }

    if (!Number.isFinite(overworld.player_x) || !Number.isFinite(overworld.player_y)) {
      return false;
    }
    if ((overworld.player_x ?? -1) < 0 || (overworld.player_y ?? -1) < 0) {
      return false;
    }

    const permission = overworld._current_tile_permission?.();
    if (typeof permission === "number") {
      const facing = facingDirectionFromString(overworld.player_direction ?? "down");
      const playerState =
        typeof overworld.player_state === "string"
          ? (overworld.player_state as PlayerState)
          : PlayerState.NORMAL;
      if (!isPermissionPassable(permission, facing, playerState)) {
        return false;
      }
    }

    return true;
  }

  private syncOverworldPlayerState(): void {
    const overworld = (this.game as unknown as { getOverworld?: () => unknown } | null)?.getOverworld?.() as
      | {
          player_x?: number;
          player_y?: number;
          _sync_player_state?: () => void;
        }
      | undefined;
    if (!overworld || typeof overworld._sync_player_state !== "function") {
      return;
    }
    if (!Number.isFinite(overworld.player_x) || !Number.isFinite(overworld.player_y)) {
      return;
    }
    overworld._sync_player_state();
  }

  private async readRuntimeSnapshot(): Promise<RuntimeSnapshotLoadResult | null> {
    const slots = [resolveRuntimeSnapshotSlot(this.sessionId)];
    const legacySlot = resolveLegacyRuntimeSnapshotSlot(this.sessionId);
    if (legacySlot !== slots[0]) {
      slots.push(legacySlot);
    }
    for (const slot of slots) {
      try {
        const raw = await fs.readFile(slot, "utf8");
        const parsed = JSON.parse(raw) as RuntimeSnapshot;
        if (parsed?.version === 1) {
          return { slot, snapshot: parsed };
        }
      } catch {
        continue;
      }
    }
    return null;
  }

  private async restoreRuntimeSnapshot(): Promise<void> {
    if (!this.game) {
      return;
    }
    const loaded = await this.readRuntimeSnapshot();
    if (!loaded) {
      return;
    }
    const staleReason = getRuntimeSnapshotStaleReason(this.game, loaded.snapshot);
    if (!staleReason && applyRuntimeSnapshot(this.game, loaded.snapshot)) {
      this.frameCounter = Math.max(this.frameCounter, loaded.snapshot.frameCounter ?? 0);
      this.actionEvents = normalizeRuntimeSnapshotActionEvents(loaded.snapshot.actionEvents);
      if (
        typeof loaded.snapshot.sessionStartedAtMs === "number" &&
        Number.isFinite(loaded.snapshot.sessionStartedAtMs) &&
        loaded.snapshot.sessionStartedAtMs > 0
      ) {
        this.sessionStartedAtMs = loaded.snapshot.sessionStartedAtMs;
      }
      this.actionEventTotal = Math.max(
        this.actionEvents.length,
        typeof loaded.snapshot.actionEventTotal === "number" && Number.isFinite(loaded.snapshot.actionEventTotal)
          ? Math.floor(loaded.snapshot.actionEventTotal)
          : 0,
      );
      return;
    }
    const staleMessage = `[mcp] Discarding stale runtime snapshot ${loaded.slot} for session ${this.sessionId}: ${staleReason ?? "runtime snapshot could not be applied to the loaded save state."}`;
    console.warn(staleMessage);
    pushDebugLog(staleMessage);
    try {
      await fs.rm(loaded.slot, { force: true });
    } catch {
      // Best-effort cleanup only; the live save remains authoritative.
    }
  }

  private async writeRuntimeSnapshot(): Promise<void> {
    if (!this.game) {
      return;
    }
    const snapshot = serializeRuntimeSnapshot(this.frameCounter, this.game, this.actionEvents, {
      sessionStartedAtMs: this.sessionStartedAtMs,
      actionEventTotal: this.actionEventTotal,
    });
    if (!snapshot) {
      return;
    }
    const previous = await this.readRuntimeSnapshot();
    const merged = mergeRuntimeSnapshotWithPrevious(previous?.snapshot ?? null, snapshot);
    try {
      await fs.writeFile(resolveRuntimeSnapshotSlot(this.sessionId), JSON.stringify(merged), "utf8");
    } catch (error) {
      if (!isNonFatalRuntimeSnapshotPersistenceError(error)) {
        throw error;
      }
      const message = `[mcp] Runtime snapshot persistence unavailable for session ${this.sessionId}; continuing without local runtime snapshot: ${String(error)}`;
      console.warn(message);
      pushDebugLog(message);
    }
  }

  private requestAutosave(options: { force?: boolean } = {}): Promise<void> {
    const force = options.force === true;
    // Avoid hammering disk: at most one autosave per ~30 frames (~0.5s at 60fps).
    if (!force && this.frameCounter - this.autosaveLastFrame < 30) {
      return this.autosaveQueue;
    }
    const gameState = this.readAutosaveState();
    if (!gameState) {
      return this.autosaveQueue;
    }

    const slot = `mcp-${this.sessionId}-autosave.sav`;
    const queuedSave = this.autosaveQueue
      .catch(() => undefined)
      .then(async () => {
        this.autosaveLastFrame = this.frameCounter;
        try {
          await runWithMcpIdentityContext(null, () => saveGame(gameState, slot));
        } catch (error) {
          if (!isNonFatalAutosavePersistenceError(error)) {
            throw error;
          }
          const message = `[mcp] Autosave failed for session ${this.sessionId} (${slot}); continuing without updated local save: ${String(error)}`;
          console.warn(message);
          pushDebugLog(message);
          return;
        }
        await this.writeRuntimeSnapshot();
      });
    this.autosaveQueue = queuedSave;
    return queuedSave;
  }

  private inferNoChangeReason(signal: SceneSignal): ActionResultReason {
    if (signal.menu || signal.mode === "menu") {
      return "menu";
    }
    if (signal.promptReason || signal.textAdvancePending || signal.mode === "battle") {
      return "busy";
    }
    return "no_change";
  }

  private isBattleIntroTransitionSnapshot(snapshot: TextSnapshotPayload | null): boolean {
    if (!snapshot) {
      return false;
    }
    const viewportText = (snapshot.viewport ?? []).join("\n");
    return (
      viewportText.includes("BATTLE TRANSITION") ||
      viewportText.includes("The battle is starting...") ||
      viewportText.includes("Wait: battle intro animation")
    );
  }

  private captureSceneSignal(game: Game): SceneSignal {
    const debugStatus = (game as { getDebugStatus?: () => { mode?: unknown } }).getDebugStatus?.();
    const gameMode =
      typeof debugStatus?.mode === "string" && debugStatus.mode.trim()
        ? debugStatus.mode.trim()
        : undefined;
    const bootMode =
      gameMode && gameMode !== "overworld" && gameMode !== "menu" && gameMode !== "battle"
        ? gameMode
        : undefined;
    const isMenuOpen = this.isMenuOpenForSession(game);
    const mode: SceneSignal["mode"] =
      bootMode ??
      ((game.isBattleActive?.() ?? false)
        ? "battle"
        : isMenuOpen
          ? "menu"
          : "overworld");
    const promptStatus = promptFromSnapshot(this.lastSnapshot);
    const dialogueState = this.getDialogueUiState(game);
    const nonBlockingPcUi = isNonBlockingPcUiSnapshot(this.lastSnapshot);
    const viewportText = this.lastSnapshot?.viewport?.join("\n") ?? "";
    const menuText = this.lastSnapshot?.menu?.join("\n") ?? "";
    const dialogueText = this.lastSnapshot?.dialogue?.join("\n") ?? "";
    const promptText = this.lastSnapshot?.prompt?.join("\n") ?? "";
    const markerText = this.lastSnapshot?.marker?.join(",") ?? "";
    return {
      mode,
      menu: isMenuOpen || Boolean(bootMode),
      map: bootMode ? bootMode.toUpperCase() : this.readBestMapName(game),
      promptReason: bootMode
        ? bootMode
        : promptStatus.pending
          ? promptStatus.reason
          : !nonBlockingPcUi && dialogueState.yes_no_prompt_open
            ? "prompt"
            : null,
      textAdvancePending: nonBlockingPcUi ? false : dialogueState.text_advance_pending,
      viewportText,
      menuText,
      dialogueText,
      promptText,
      markerText,
    };
  }

  private deriveSceneMoments(before: SceneSignal, after: SceneSignal, action: string): string[] {
    const moments: string[] = [];
    if (before.mode !== after.mode) {
      moments.push(`mode:${before.mode}->${after.mode}`);
      if (after.mode === "battle") {
        moments.push("battle_started");
      }
      if (before.mode === "battle") {
        moments.push("battle_ended");
      }
    }
    if (before.map && after.map && before.map !== after.map) {
      moments.push(`map:${before.map}->${after.map}`);
    }
    if (!before.menu && after.menu) {
      moments.push("menu_opened");
    }
    if (before.menu && !after.menu) {
      moments.push("menu_closed");
    }
    if (!before.promptReason && after.promptReason) {
      moments.push(`prompt_opened:${after.promptReason}`);
    }
    if (before.promptReason && !after.promptReason) {
      moments.push(`prompt_closed:${before.promptReason}`);
    }
    if (before.promptReason && after.promptReason && before.promptReason !== after.promptReason) {
      moments.push(`prompt_changed:${before.promptReason}->${after.promptReason}`);
    }
    if (!before.textAdvancePending && after.textAdvancePending) {
      moments.push("text_advance_opened");
    }
    if (before.textAdvancePending && !after.textAdvancePending) {
      moments.push("text_advance_closed");
    }
    if (before.menuText !== after.menuText) {
      moments.push("menu_changed");
    }
    if (before.viewportText !== after.viewportText) {
      moments.push("viewport_changed");
    }
    if (before.dialogueText && after.dialogueText && before.dialogueText !== after.dialogueText) {
      moments.push("dialogue_advanced");
    }
    if (!before.dialogueText && after.dialogueText && action.startsWith("press:a")) {
      moments.push("talked_to_npc");
    }
    if (before.markerText !== after.markerText) {
      moments.push("marker_changed");
    }
    if (before.promptText !== after.promptText && before.promptText && after.promptText) {
      moments.push("prompt_text_changed");
    }
    return Array.from(new Set(moments)).slice(0, 5);
  }

  private describeActionResult(action: string, result: ActionResult): string {
    const outcome = result.reason ?? (result.ok ? "ok" : "failed");
    return `${action} ${outcome}`;
  }

  private humanizeMoment(moment: string): string {
    const compact = moment.trim();
    if (!compact) {
      return "none";
    }
    return compact.replaceAll("_", " ");
  }

  private summarizeActionEvent(action: string, result: ActionResult, moments: string[]): string {
    if (moments.length) {
      return this.humanizeMoment(moments[0] ?? "");
    }
    return this.describeActionResult(action, result);
  }

  private recordActionEvent(
    action: string,
    result: ActionResult,
    options: {
      before?: SceneSignal;
      after?: SceneSignal;
    } = {}
  ): void {
    const game = this.getGame();
    const before = options.before ?? this.captureSceneSignal(game);
    const after = options.after ?? this.captureSceneSignal(game);
    const moments = this.deriveSceneMoments(before, after, action);
    const timestampMs = Date.now();
    this.actionEvents.push({
      frame: this.frameCounter,
      timestamp_ms: timestampMs,
      timestamp_iso: new Date(timestampMs).toISOString(),
      action,
      mode: after.mode,
      map: after.map,
      coords: this.readBestCoords(game),
      prompt: after.promptReason ?? undefined,
      moments: moments.length ? moments : undefined,
      summary: this.summarizeActionEvent(action, result, moments),
      result,
    });
    this.actionEventTotal += 1;
    if (this.actionEvents.length > ACTION_EVENT_LOG_LIMIT) {
      this.actionEvents = this.actionEvents.slice(-ACTION_EVENT_LOG_LIMIT);
    }
  }

  private mapActionReason(options: {
    blocked: boolean;
    stopReason: string | null;
    changed: boolean;
  }): ActionResultReason | undefined {
    if (options.blocked) {
      return "blocked";
    }
    if (!options.stopReason) {
      return options.changed ? undefined : "no_change";
    }
    if (options.stopReason === "menu") {
      return "menu";
    }
    return "busy";
  }

  private buildStateFingerprint(game: Game): string {
    const state = game.getGameState();
    const mode =
      game.isBattleActive?.() ?? false
        ? "battle"
        : this.isNameEntryActive(game)
          ? "name_entry"
          : this.isMenuOpenForSession(game)
            ? "menu"
            : "overworld";
    const map = game.getMapName();
    const x = state.wram.player_x ?? "na";
    const y = state.wram.player_y ?? "na";
    const prompt = promptFromSnapshot(this.lastSnapshot);
    const dialogueState = this.getDialogueUiState(game);
    const interaction = this.buildInteractionFingerprint(game);
    const snapshotTextFingerprint = [
      this.lastSnapshot?.titles ? JSON.stringify(this.lastSnapshot.titles) : "",
      this.lastSnapshot?.viewport?.join("|") ?? "",
      this.lastSnapshot?.menu?.join("|") ?? "",
      this.lastSnapshot?.prompt?.join("|") ?? "",
      this.lastSnapshot?.dialogue?.join("|") ?? "",
      this.lastSnapshot?.info?.join("|") ?? "",
      this.lastSnapshot?.marker?.join(",") ?? "",
    ].join("::");
    return `${mode}|${map}|${x},${y}|${prompt.pending ? prompt.reason ?? "prompt" : "none"}|advance:${dialogueState.text_advance_pending ? 1 : 0}|${interaction}|${snapshotTextFingerprint}`;
  }

  private buildInteractionFingerprint(game: Game): string {
    const state = game.getGameState();
    const overworld = game.getOverworld() as unknown as
      | {
          _active_bg_event?: { event_type?: unknown; script?: unknown; x?: unknown; y?: unknown } | null;
          _text_lock_active?: boolean;
          _blocking_task_count?: number;
          _blocking_movement_lock_active?: boolean;
          _active_script_task?: { kind?: unknown; name?: unknown } | null;
          _script_task_queue?: Array<{ kind?: unknown; name?: unknown }> | null;
          pokepic_overlay?: { isVisible?: boolean } | null;
          pokepicOverlay?: { isVisible?: boolean } | null;
          script_runner?: {
            _script_stack?: unknown[];
            _awaiting_resume?: number;
            _queued_overworld_task_count?: number;
            stop_execution?: boolean;
            is_busy?: boolean;
            state?: unknown;
            pending_reload_map?: unknown;
            last_interaction_object_index?: number | null;
          } | null;
        }
      | null
      | undefined;
    const runner = overworld?.script_runner ?? null;
    const activeBgEvent = overworld?._active_bg_event ?? null;
    const textLockActive = Boolean(overworld?._text_lock_active);
    const blockingTaskCount =
      typeof overworld?._blocking_task_count === "number" && Number.isFinite(overworld._blocking_task_count)
        ? Math.max(0, overworld._blocking_task_count)
        : 0;
    const blockingMovementLockActive = Boolean(overworld?._blocking_movement_lock_active);
    const activeScriptTask = overworld?._active_script_task ?? null;
    const activeScriptTaskFingerprint = activeScriptTask
      ? [
          String(activeScriptTask.kind ?? "").trim(),
          String(activeScriptTask.name ?? "").trim(),
        ].join(":")
      : "";
    const queuedScriptTaskFingerprint = Array.isArray(overworld?._script_task_queue)
      ? overworld._script_task_queue
          .slice(0, 4)
          .map((task) => [
            String(task?.kind ?? "").trim(),
            String(task?.name ?? "").trim(),
          ].join(":"))
          .join("|")
      : "";
    const pokepicVisible = Boolean(overworld?.pokepic_overlay?.isVisible ?? overworld?.pokepicOverlay?.isVisible);
    const stackDepth = Array.isArray(runner?._script_stack) ? runner._script_stack.length : 0;
    const awaitingResume =
      typeof runner?._awaiting_resume === "number" && Number.isFinite(runner._awaiting_resume)
        ? runner._awaiting_resume
        : 0;
    const queuedOverworldTasks =
      typeof runner?._queued_overworld_task_count === "number" &&
      Number.isFinite(runner._queued_overworld_task_count)
        ? runner._queued_overworld_task_count
        : 0;
    const lastTalked =
      typeof state.wram.last_talked === "number" && Number.isFinite(state.wram.last_talked)
        ? state.wram.last_talked
        : 0;
    const lastInteractionObjectIndex =
      typeof runner?.last_interaction_object_index === "number" &&
      Number.isFinite(runner.last_interaction_object_index)
        ? runner.last_interaction_object_index
        : 0;
    const activeBgEventFingerprint = activeBgEvent
      ? [
          String(activeBgEvent.event_type ?? "").trim(),
          String(activeBgEvent.script ?? "").trim(),
          typeof activeBgEvent.x === "number" ? String(activeBgEvent.x) : "",
          typeof activeBgEvent.y === "number" ? String(activeBgEvent.y) : "",
        ].join("@")
      : "";
    return [
      `talked:${lastTalked}`,
      `object:${lastInteractionObjectIndex}`,
      `stack:${stackDepth}`,
      `await:${awaitingResume}`,
      `tasks:${queuedOverworldTasks}`,
      `stop:${runner?.stop_execution ? 1 : 0}`,
      `busy:${runner?.is_busy ? 1 : 0}`,
      `state:${runner?.state === undefined || runner?.state === null ? "" : String(runner.state)}`,
      `reload:${typeof runner?.pending_reload_map === "string" ? runner.pending_reload_map : ""}`,
      `bg:${activeBgEventFingerprint}`,
      `textlock:${textLockActive ? 1 : 0}`,
      `blocking:${blockingTaskCount}`,
      `blocklock:${blockingMovementLockActive ? 1 : 0}`,
      `activetask:${activeScriptTaskFingerprint}`,
      `queuedtasks:${queuedScriptTaskFingerprint}`,
      `pokepic:${pokepicVisible ? 1 : 0}`,
    ].join("|");
  }

  private async performMove(
    direction: Direction,
    times: number,
    options: { stopOnEvent?: boolean; holdFrames?: number } = {}
  ): Promise<MoveOutcome> {
    await this.ensureReady();
    const normalizedTimes = this.normalizeTimes(times);
    const game = this.getGame();
    const baselineMap = this.readMapIdentity(game);
    const start = this.readPlayerCoords(game);
    let completed = 0;
    let blocked = false;
    let blockReason: string | null = null;
    let stopReason: string | null = null;
    const stopOnEvent = options.stopOnEvent !== false;
    const holdFrames = this.normalizeHoldFrames(options.holdFrames);

    for (let i = 0; i < normalizedTimes; i += 1) {
      if (stopOnEvent) {
        stopReason = this.getStopReason(game, baselineMap);
        // Prompts (YES/NO, etc.) still accept D-pad input. Treat them as navigable rather
        // than an interruption so callers can answer prompt menus.
        if (
          stopReason === "prompt" ||
          stopReason === "menu" ||
          stopReason === "name_entry" ||
          stopReason === "battle"
        ) {
          stopReason = null;
        }
        if (stopReason === "movement_lock") {
          stopReason = this.settleMovementLock(game, baselineMap);
        }
        if (stopReason) {
          break;
        }
      }

      // When a prompt/menu is open, directional input is for UI navigation (not movement).
      // The overworld movement loop won't detect a coordinate change, so we treat the
      // input as a completed step and let state-fingerprinting detect UI changes.
      const promptStatus = promptFromSnapshot(this.lastSnapshot);
      const menuOpen = this.isMenuOpenForSession(game);
      if (
        game.isBattleActive?.() ||
        promptStatus.pending ||
        menuOpen ||
        this.isNameEntryActive(game) ||
        this.isUnownPuzzleInputActive(game)
      ) {
        this.recordAction(formatActionLabel("move", direction));
        this.scheduleKeyPress({
          key: keyForDirection(direction),
          direction,
          holdFrames,
        });
        this.stepFrames(holdFrames + 1);
        completed += 1;
        continue;
      }

      let attemptedTurnRecovery = false;
      while (true) {
        const stepStart = this.readPlayerCoords(game);
        const facingBefore = this.readFacingDirection(game);
        const movementHoldFrames = options.holdFrames === undefined || options.holdFrames === null
          ? Math.max(holdFrames, facingBefore !== direction ? 2 : 1)
          : holdFrames;
        this.clearActiveWarpGuardForIntentionalMove(game, direction);
        this.recordAction(formatActionLabel("move", direction));
        this.scheduleKeyPress({
          key: keyForDirection(direction),
          direction,
          holdFrames: movementHoldFrames,
        });
        this.stepFrames(movementHoldFrames + 1);

        const stepResult = this.waitForMovement(game, stepStart, baselineMap, stopOnEvent);
        if (stepResult.moved) {
          completed += 1;
          if (stepResult.stopReason) {
            stopReason = stepResult.stopReason;
          }
          break;
        }
        if (stepResult.stopReason) {
          stopReason = stepResult.stopReason;
          break;
        }
        const facingAfter = this.readFacingDirection(game);
        const turnedTowardDirection =
          facingBefore !== direction && facingAfter === direction;
        if (!attemptedTurnRecovery && turnedTowardDirection) {
          attemptedTurnRecovery = true;
          continue;
        }
        blocked = true;
        blockReason = stepResult.blockReason ?? null;
        break;
      }
      if (blocked || stopReason) {
        break;
      }
    }

    const end = this.readPlayerCoords(game);
    const map = this.readMapIdentity(game).name;
    this.lastMcpMeta = {
      move_summary: {
        direction,
        requested: normalizedTimes,
        completed,
        start,
        end,
        map,
      },
    };
    this.captureSnapshot();
    return {
      requested: normalizedTimes,
      completed,
      start,
      end,
      map,
      blocked,
      blockReason,
      stopReason,
    };
  }

  private waitForMovement(
    game: Game,
    start: [number, number],
    baselineMap: MapIdentity,
    stopOnEvent: boolean
  ): { moved: boolean; stopReason: string | null; blockReason: string | null } {
    const overworld = game.getOverworld();
    let framesWaited = 0;
    while (this.isMovementActive(overworld)) {
      if (framesWaited >= this.maxFramesPerCall) {
        return { moved: false, stopReason: "timeout", blockReason: null };
      }
      this.stepFrames(1);
      framesWaited += 1;
      if (stopOnEvent) {
        const reason = this.getStopReason(game, baselineMap);
        if (reason === "movement_lock") {
          const settledReason = this.settleMovementLock(game, baselineMap);
          if (settledReason === "movement_lock") {
            const blockReason = this.readBlockReason(overworld);
            if (!this.hasPlayerMoved(game, start) && blockReason) {
              return {
                moved: false,
                stopReason: null,
                blockReason,
              };
            }
          }
          if (settledReason) {
            return {
              moved: this.hasPlayerMoved(game, start),
              stopReason: settledReason,
              blockReason: null,
            };
          }
        }
        if (reason) {
          return {
            moved: this.hasPlayerMoved(game, start),
            stopReason: reason,
            blockReason: null,
          };
        }
      }
    }

    const moved = this.hasPlayerMoved(game, start);
    if (stopOnEvent) {
      if (moved) {
        const postMoveReason = this.settlePostMoveEvents(game, baselineMap);
        if (postMoveReason) {
          return {
            moved: true,
            stopReason: postMoveReason,
            blockReason: null,
          };
        }
      }
      const reason = this.getStopReason(game, baselineMap);
      if (reason === "movement_lock") {
        const settledReason = this.settleMovementLock(game, baselineMap);
        if (settledReason === "movement_lock" && !moved) {
          const blockReason = this.readBlockReason(overworld);
          if (blockReason) {
            return {
              moved: false,
              stopReason: null,
              blockReason,
            };
          }
        }
        if (settledReason) {
          return {
            moved,
            stopReason: settledReason,
            blockReason: null,
          };
        }
      }
      if (reason) {
        return {
          moved,
          stopReason: reason,
          blockReason: null,
        };
      }
    }
    if (!moved) {
      return {
        moved: false,
        stopReason: null,
        blockReason: this.readBlockReason(overworld),
      };
    }
    return { moved: true, stopReason: null, blockReason: null };
  }

  private clearActiveWarpGuardForIntentionalMove(game: Game, direction?: Direction): void {
    const overworld = game.getOverworld() as {
      current_map_name?: string;
      player_x?: number;
      player_y?: number;
      TILES_PER_COLLISION?: number;
      map?: { width?: number; height?: number } | null;
      _active_warp_tile?: [string, number, number] | null;
      _warp_tile_lookup?: Record<
        string,
        Array<{
          target_map_constant?: string;
        }>
      > | null;
    } | null;
    if (!overworld?._active_warp_tile) {
      return;
    }
    const environment = String(getMapEnvironment(overworld.current_map_name ?? "") ?? "")
      .trim()
      .toUpperCase();
    if (environment !== "INDOOR" && environment !== "GATE") {
      return;
    }
    const [mapName, x, y] = overworld._active_warp_tile;
    if (
      mapName === overworld.current_map_name &&
      x === overworld.player_x &&
      y === overworld.player_y
    ) {
      const warpLookup = overworld._warp_tile_lookup ?? null;
      const warpsOnTile =
        warpLookup && typeof warpLookup === "object"
          ? warpLookup[`${x},${y}`] ?? []
          : [];
      const staysIndoors = warpsOnTile.some((warp) => {
        const targetConstant = String(warp?.target_map_constant ?? "").trim();
        if (!targetConstant) {
          return false;
        }
        const targetEnvironment = String(
          getMapEnvironment(mapConstantToName(targetConstant)) ?? ""
        )
          .trim()
          .toUpperCase();
        return targetEnvironment === "INDOOR" || targetEnvironment === "GATE";
      });
      if (staysIndoors) {
        return;
      }
      if (direction && !this.isLeavingMapThroughActiveWarp(overworld, direction, x, y)) {
        return;
      }
      overworld._active_warp_tile = null;
    }
  }

  private isLeavingMapThroughActiveWarp(
    overworld: {
      TILES_PER_COLLISION?: number;
      map?: { width?: number; height?: number } | null;
    },
    direction: Direction,
    x: number,
    y: number
  ): boolean {
    const mapWidth = overworld.map?.width;
    const mapHeight = overworld.map?.height;
    if (
      typeof mapWidth !== "number" ||
      typeof mapHeight !== "number" ||
      !Number.isFinite(mapWidth) ||
      !Number.isFinite(mapHeight)
    ) {
      return true;
    }
    const stride = Math.max(1, Math.trunc(overworld.TILES_PER_COLLISION ?? 2));
    const minTile = Math.max(0, stride - 1);
    const maxX = mapWidth * stride - 1;
    const maxY = mapHeight * stride - 1;
    return (
      (direction === "up" && y <= minTile) ||
      (direction === "down" && y >= maxY) ||
      (direction === "left" && x <= minTile) ||
      (direction === "right" && x >= maxX)
    );
  }

  private settlePostMoveEvents(game: Game, baselineMap: MapIdentity | null): string | null {
    const maxFrames = Math.max(0, Math.min(this.maxFramesPerCall, POST_MOVE_SETTLE_MAX_FRAMES));
    for (let frame = 0; frame < maxFrames; frame += 1) {
      const reason = this.getStopReason(game, baselineMap);
      if (reason === "movement_lock") {
        const settledReason = this.settleMovementLock(game, baselineMap);
        if (settledReason) {
          return settledReason;
        }
      } else if (reason) {
        return reason;
      }
      this.stepFrames(1);
    }

    const finalReason = this.getStopReason(game, baselineMap);
    if (finalReason === "movement_lock") {
      return this.settleMovementLock(game, baselineMap);
    }
    return finalReason;
  }

  private readPlayerCoords(game: Game): [number, number] {
    const state = game.getGameState();
    const best = this.readBestCoords(game);
    if (best) {
      return [best.x, best.y];
    }
    return [state.wram.player_x ?? 0, state.wram.player_y ?? 0];
  }

  private readBestCoords(game: Game): { x: number; y: number } | undefined {
    const state = game.getGameState();
    const overworld = game.getOverworld() as {
      player_x?: number;
      player_y?: number;
      prev_player_x?: number;
      prev_player_y?: number;
      player_object?: {
        x?: number;
        y?: number;
      } | null;
    };
    // Prefer stable overworld status coords before transient sprite-local positions.
    const candidates = [
      { x: state.wram.wXCoord, y: state.wram.wYCoord },
      { x: overworld.player_x, y: overworld.player_y },
      { x: overworld.player_object?.x, y: overworld.player_object?.y },
      { x: overworld.prev_player_x, y: overworld.prev_player_y },
      { x: state.wram.player_x, y: state.wram.player_y },
    ];
    const parsed = candidates
      .map((candidate) => {
        const x = candidate.x;
        const y = candidate.y;
        if (typeof x !== "number" || !Number.isFinite(x) || typeof y !== "number" || !Number.isFinite(y)) {
          return null;
        }
        if (x < 0 || y < 0) {
          return null;
        }
        return { x, y };
      })
      .filter((candidate): candidate is { x: number; y: number } => Boolean(candidate));
    const nonOrigin = parsed.find((candidate) => candidate.x !== 0 || candidate.y !== 0);
    return nonOrigin ?? parsed[0];
  }

  private readBestMapName(game: Game): string | undefined {
    const overworld = game.getOverworld() as {
      current_map_name?: string;
      currentMapName?: string;
      current_map?: { name?: string };
    };
    const candidates = [
      game.getMapName(),
      overworld.current_map?.name,
      overworld.current_map_name,
      overworld.currentMapName,
    ];
    const normalized = candidates
      .map((value) => (typeof value === "string" ? value.trim() : ""))
      .filter((value) => value.length > 0);
    const nonUnknown = normalized.find((value) => value.toLowerCase() !== "unknown");
    return nonUnknown ?? normalized[0] ?? undefined;
  }

  private readFacingDirection(game: Game): McpPlayerContext["facing"] {
    const overworld = game.getOverworld() as {
      player_direction?: unknown;
      player?: { direction?: unknown };
      player_object?: { direction?: unknown } | null;
      _queued_direction?: unknown;
    };
    const candidates = [
      overworld.player_object?.direction,
      overworld.player_direction,
      overworld.player?.direction,
      overworld._queued_direction,
    ];
    for (const candidate of candidates) {
      if (typeof candidate !== "string") {
        continue;
      }
      const normalized = candidate.trim().toLowerCase();
      if (
        normalized === "up" ||
        normalized === "down" ||
        normalized === "left" ||
        normalized === "right"
      ) {
        return normalized;
      }
    }
    return "unknown";
  }

  private readBestFacing(game: Game): Direction | undefined {
    const facing = this.readFacingDirection(game);
    return facing === "unknown" ? undefined : facing;
  }

  private alignMcpHealCounterFacingBeforePress(game: Game, button: Button): void {
    if (button !== "a") {
      return;
    }
    const modal = this.getModalUiState(game);
    if (modal.in_battle || modal.in_dialog || modal.in_menu || modal.prompt_pending) {
      return;
    }
    const coords = this.readBestCoords(game);
    const mapDetails = this.buildSnapshotMapInfo();
    const interactionLane = this.readInteractionLane(coords, this.readBestFacing(game), mapDetails);
    if (
      !interactionLane ||
      interactionLane.hotspot.hotspot_type !== "heal" ||
      interactionLane.lane.facing_aligned
    ) {
      return;
    }
    this.setMcpOverworldFacing(game, interactionLane.lane.facing);
  }

  private runConfirmedScriptedInteraction(
    game: Game,
    target: NonNullable<McpStatusSnapshot["interaction_target"]>
  ): boolean {
    if (!target.script) {
      return false;
    }
    const scriptName = String(target.script).trim();
    if (!scriptName) {
      return false;
    }
    const overworld = game.getOverworld() as unknown as
      | {
          script_runner?: {
            is_busy?: boolean;
            run?: (scriptName: string, options?: { allow_fallthrough?: boolean }) => void;
            last_interaction_object_index?: number | null;
          } | null;
          _bg_event_at?: (tileX: number, tileY: number) => { script?: unknown } | null;
          _handle_bg_event?: (event: { script?: unknown }) => boolean;
          _npc_on_tile?: (tileX: number, tileY: number) => {
            objectIndex?: number;
            x?: number;
            y?: number;
            event?: { script?: unknown; object_type?: unknown } | null;
            facePlayer?: (playerX: number, playerY: number) => void;
            face_player?: (playerX: number, playerY: number) => void;
          } | null;
          _nearest_npc_covering_subtile?: (tileX: number, tileY: number) => {
            objectIndex?: number;
            x?: number;
            y?: number;
            event?: { script?: unknown; object_type?: unknown } | null;
            facePlayer?: (playerX: number, playerY: number) => void;
            face_player?: (playerX: number, playerY: number) => void;
          } | null;
          player_object?: { x?: number; y?: number } | null;
          player_x?: number;
          player_y?: number;
          _play_interaction_sound?: () => void;
        }
      | null
      | undefined;
    const runner = overworld?.script_runner;
    if (!overworld || !runner || runner.is_busy || typeof runner.run !== "function") {
      return false;
    }
    if (target.kind === "bg_event" && typeof overworld._bg_event_at === "function") {
      const bgEvent = overworld._bg_event_at(target.x, target.y);
      if (bgEvent && String(bgEvent.script ?? "").trim() === scriptName) {
        if (typeof overworld._handle_bg_event === "function") {
          return overworld._handle_bg_event(bgEvent);
        }
      }
    }
    const npc =
      (typeof overworld._npc_on_tile === "function"
        ? overworld._npc_on_tile(target.x, target.y)
        : null) ??
      (typeof overworld._nearest_npc_covering_subtile === "function"
        ? overworld._nearest_npc_covering_subtile(target.x, target.y)
        : null);
    const npcScript = String(npc?.event?.script ?? "").trim();
    if (npcScript && npcScript !== scriptName) {
      return false;
    }
    const objectIndex = npc?.objectIndex ?? target.object_index ?? 0;
    const gameState = game.getGameState?.() as { wram?: { last_talked?: number } } | null | undefined;
    if (gameState?.wram) {
      gameState.wram.last_talked = objectIndex;
    }
    runner.last_interaction_object_index = objectIndex || null;
    const playerX = overworld.player_object?.x ?? overworld.player_x ?? target.x;
    const playerY = overworld.player_object?.y ?? overworld.player_y ?? target.y;
    if (npc && typeof npc.facePlayer === "function") {
      npc.facePlayer(playerX, playerY);
    } else if (npc && typeof npc.face_player === "function") {
      npc.face_player(playerX, playerY);
    }
    overworld._play_interaction_sound?.();
    const objectType = String(npc?.event?.object_type ?? "").toUpperCase();
    if (objectType === "OBJECTTYPE_TRAINER") {
      runner.run(scriptName, { allow_fallthrough: false });
    } else {
      runner.run(scriptName);
    }
    return true;
  }

  private setMcpOverworldFacing(game: Game, direction: Direction): void {
    const overworld = game.getOverworld() as {
      player_direction?: Direction;
      player?: { direction?: Direction };
      player_object?: { direction?: Direction; updatePixelPosition?: () => void } | null;
      _queued_direction?: Direction | null;
    };
    overworld.player_direction = direction;
    if (overworld.player) {
      overworld.player.direction = direction;
    }
    if (overworld.player_object) {
      overworld.player_object.direction = direction;
      overworld.player_object.updatePixelPosition?.();
    }
    overworld._queued_direction = null;
  }

  private readInteractionTile(game: Game): { x: number; y: number } | undefined {
    const overworld = game.getOverworld() as
      | {
          get_facing_tile_coords?: () => [number, number];
          _counter_adjusted_tile?: (tileX: number, tileY: number) => [number, number];
        }
      | null
      | undefined;
    if (!overworld || typeof overworld.get_facing_tile_coords !== "function") {
      return undefined;
    }
    const facingTile = overworld.get_facing_tile_coords();
    if (!Array.isArray(facingTile) || facingTile.length < 2) {
      return undefined;
    }
    const [rawX, rawY] = facingTile;
    if (!Number.isFinite(rawX) || !Number.isFinite(rawY)) {
      return undefined;
    }
    const [x, y] =
      typeof overworld._counter_adjusted_tile === "function"
        ? overworld._counter_adjusted_tile(rawX, rawY)
        : [rawX, rawY];
    if (!Number.isFinite(x) || !Number.isFinite(y) || x < 0 || y < 0) {
      return undefined;
    }
    return { x, y };
  }

  private readInteractionTarget(
    game: Game,
    interactionTile: { x: number; y: number } | undefined,
    mapDetails?: McpMapInfoSnapshot
  ): McpStatusSnapshot["interaction_target"] {
    if (!interactionTile) {
      return undefined;
    }
    const overworld = game.getOverworld() as unknown as
      | {
          _npc_on_tile?: (
            tileX: number,
            tileY: number
          ) => { walking?: boolean; jumping?: boolean; event?: { script?: unknown } | null } | null;
          _nearest_npc_covering_subtile?: (
            tileX: number,
            tileY: number
          ) => { walking?: boolean; jumping?: boolean; event?: { script?: unknown } | null } | null;
          _bg_event_at?: (tileX: number, tileY: number) => { event_type?: unknown; script?: unknown } | null;
          _bg_event_allowed_by_flags?: (eventType: string, scriptName: string) => boolean;
          _npc_blueprints?: Map<string, Map<string, [unknown, number]>>;
          current_map_name?: string | null;
          npcs?: Array<{
            x?: unknown;
            y?: unknown;
            walking?: boolean;
            jumping?: boolean;
            event?: { script?: unknown } | null;
          }> | null;
        }
      | null
      | undefined;
    if (!overworld) {
      return undefined;
    }
    const npc =
      (typeof overworld._npc_on_tile === "function"
        ? overworld._npc_on_tile(interactionTile.x, interactionTile.y)
        : null) ??
      (typeof overworld._nearest_npc_covering_subtile === "function"
        ? overworld._nearest_npc_covering_subtile(interactionTile.x, interactionTile.y)
        : null);
    const npcScript = String(npc?.event?.script ?? "").trim();
    const rawObject =
      Array.isArray(overworld.npcs)
        ? overworld.npcs.find((candidate) => {
            const x = Number(candidate?.x);
            const y = Number(candidate?.y);
            return Number.isFinite(x) && Number.isFinite(y) && x === interactionTile.x && y === interactionTile.y;
          }) ?? null
        : null;
    const rawObjectScript = String(rawObject?.event?.script ?? "").trim();
    const hotspotAtInteractionTile =
      mapDetails?.hotspots.find((hotspot) =>
        hotspot.coords.x === interactionTile.x &&
        hotspot.coords.y === interactionTile.y
      ) ?? null;
    if (npc && !npc.walking && !npc.jumping && npcScript) {
      return {
        x: interactionTile.x,
        y: interactionTile.y,
        kind: "npc",
        label: hotspotAtInteractionTile?.label,
        token: hotspotAtInteractionTile?.token,
        hotspot_type: hotspotAtInteractionTile?.type,
        script: npcScript,
      };
    }
    if (rawObject && !rawObject.walking && !rawObject.jumping && rawObjectScript) {
      return {
        x: interactionTile.x,
        y: interactionTile.y,
        kind: hotspotAtInteractionTile?.type === "npc" ? "npc" : "bg_event",
        label: hotspotAtInteractionTile?.label,
        token: hotspotAtInteractionTile?.token,
        hotspot_type: hotspotAtInteractionTile?.type,
        script: rawObjectScript,
      };
    }
    const blueprintTarget =
      hotspotAtInteractionTile &&
      /^npc-(\d+)$/i.test(hotspotAtInteractionTile.id)
        ? this.readBlueprintInteractionTargetByObjectIndex(
            overworld._npc_blueprints,
            typeof overworld.current_map_name === "string" ? overworld.current_map_name : mapDetails?.map ?? null,
            Number.parseInt(hotspotAtInteractionTile.id.slice(4), 10)
          )
        : undefined;
    if (blueprintTarget?.script) {
      return {
        x: interactionTile.x,
        y: interactionTile.y,
        kind: hotspotAtInteractionTile?.type === "npc" ? "npc" : "bg_event",
        label: hotspotAtInteractionTile?.label,
        token: hotspotAtInteractionTile?.token,
        hotspot_type: hotspotAtInteractionTile?.type,
        script: blueprintTarget.script,
      };
    }
    const bgEvent =
      typeof overworld._bg_event_at === "function"
        ? overworld._bg_event_at(interactionTile.x, interactionTile.y)
        : null;
    if (!bgEvent) {
      return this.fallbackInteractionTargetFromHotspot(interactionTile, mapDetails, {
        blueprints: overworld._npc_blueprints,
        mapName: typeof overworld.current_map_name === "string" ? overworld.current_map_name : mapDetails?.map ?? null,
      });
    }
    const eventType = String(bgEvent.event_type ?? "").trim().toUpperCase();
    const scriptName = String(bgEvent.script ?? "").trim();
    const allowed =
      typeof overworld._bg_event_allowed_by_flags === "function"
        ? overworld._bg_event_allowed_by_flags(eventType, scriptName)
        : true;
    if (!allowed) {
      return this.fallbackInteractionTargetFromHotspot(interactionTile, mapDetails, {
        blueprints: overworld._npc_blueprints,
        mapName: typeof overworld.current_map_name === "string" ? overworld.current_map_name : mapDetails?.map ?? null,
      });
    }
    return {
      x: interactionTile.x,
      y: interactionTile.y,
      kind: "bg_event",
      label: hotspotAtInteractionTile?.label,
      token: hotspotAtInteractionTile?.token,
      hotspot_type: hotspotAtInteractionTile?.type,
      script: scriptName,
    };
  }

  private readBlueprintInteractionTargetByObjectIndex(
    blueprints: Map<string, Map<string, [unknown, number]>> | undefined,
    mapName: string | null | undefined,
    objectIndex: number
  ): { script: string; objectIndex: number } | undefined {
    if (!blueprints || !mapName || !Number.isFinite(objectIndex) || objectIndex <= 0) {
      return undefined;
    }
    const blueprint = blueprints.get(mapName);
    if (!blueprint) {
      return undefined;
    }
    for (const [, entry] of blueprint.entries()) {
      const [rawEvent, rawIndex] = Array.isArray(entry) ? entry : [null, null];
      if (rawIndex !== objectIndex || !rawEvent || typeof rawEvent !== "object") {
        continue;
      }
      const script = String((rawEvent as { script?: unknown }).script ?? "").trim();
      if (script) {
        return { script, objectIndex };
      }
    }
    return undefined;
  }

  private readBlueprintInteractionTargetByCoords(
    blueprints: Map<string, Map<string, [unknown, number]>> | undefined,
    mapName: string | null | undefined,
    interactionTile: { x: number; y: number },
    coordStride: number | null | undefined
  ): { script: string; objectIndex: number } | undefined {
    if (!blueprints || !mapName) {
      return undefined;
    }
    const blueprint = blueprints.get(mapName);
    if (!blueprint) {
      return undefined;
    }
    const stride = Number.isFinite(coordStride) && Number(coordStride) > 1 ? Number(coordStride) : 1;
    for (const [, entry] of blueprint.entries()) {
      const [rawEvent, rawIndex] = Array.isArray(entry) ? entry : [null, null];
      if (!rawEvent || typeof rawEvent !== "object" || !Number.isFinite(rawIndex)) {
        continue;
      }
      const event = rawEvent as { x?: unknown; y?: unknown; script?: unknown };
      const eventX = Number(event.x);
      const eventY = Number(event.y);
      if (!Number.isFinite(eventX) || !Number.isFinite(eventY)) {
        continue;
      }
      const matchesTile =
        (eventX === interactionTile.x && eventY === interactionTile.y) ||
        (eventX * stride + Math.max(0, stride - 1) === interactionTile.x &&
          eventY * stride + Math.max(0, stride - 1) === interactionTile.y);
      if (!matchesTile) {
        continue;
      }
      const script = String(event.script ?? "").trim();
      if (script) {
        return { script, objectIndex: Number(rawIndex) };
      }
    }
    return undefined;
  }

  private fallbackInteractionTargetFromHotspot(
    interactionTile: { x: number; y: number },
    mapDetails?: McpMapInfoSnapshot,
    blueprintContext?: {
      blueprints?: Map<string, Map<string, [unknown, number]>>;
      mapName?: string | null;
    }
  ): McpStatusSnapshot["interaction_target"] {
    const hotspotAtInteractionTile =
      mapDetails?.hotspots.find((hotspot) =>
        hotspot.visible &&
        hotspot.interactable &&
        hotspot.coords.x === interactionTile.x &&
        hotspot.coords.y === interactionTile.y
      ) ?? null;
    if (!hotspotAtInteractionTile) {
      return undefined;
    }
    if (hotspotAtInteractionTile.type !== "npc" && hotspotAtInteractionTile.type !== "heal") {
      return undefined;
    }
    const blueprintTarget =
      hotspotAtInteractionTile.type === "heal"
        ? this.readBlueprintInteractionTargetByCoords(
            blueprintContext?.blueprints,
            blueprintContext?.mapName ?? mapDetails?.map ?? null,
            interactionTile,
            mapDetails?.coord_stride
          )
        : undefined;
    return {
      x: interactionTile.x,
      y: interactionTile.y,
      kind: "npc",
      label: hotspotAtInteractionTile.label,
      token: hotspotAtInteractionTile.token,
      hotspot_type: hotspotAtInteractionTile.type,
      script: blueprintTarget?.script,
      object_index: blueprintTarget?.objectIndex,
    };
  }

  private readCurrentHotspot(
    coords: { x: number; y: number } | undefined,
    mapDetails?: McpMapInfoSnapshot
  ): McpStatusSnapshot["current_hotspot"] {
    if (!coords) {
      return undefined;
    }
    const hotspot =
      mapDetails?.hotspots.find((candidate) =>
        candidate.visible &&
        candidate.interactable &&
        candidate.coords.x === coords.x &&
        candidate.coords.y === coords.y
      ) ?? null;
    if (!hotspot) {
      return undefined;
    }
    return {
      x: hotspot.coords.x,
      y: hotspot.coords.y,
      label: hotspot.label,
      token: hotspot.token,
      hotspot_type: hotspot.type,
    };
  }

  private readInteractionSetup(
    coords: { x: number; y: number } | undefined,
    mapDetails: McpMapInfoSnapshot | undefined,
    currentHotspot: McpStatusSnapshot["current_hotspot"] | undefined,
    interactionTarget: McpStatusSnapshot["interaction_target"] | undefined
  ): McpStatusSnapshot["interaction_setup"] {
    if (!coords || !currentHotspot) {
      return undefined;
    }
    const hotspot =
      currentHotspot.label || currentHotspot.token || currentHotspot.hotspot_type
        ? {
            x: currentHotspot.x,
            y: currentHotspot.y,
            label: currentHotspot.label,
            token: currentHotspot.token,
            hotspot_type: currentHotspot.hotspot_type,
          }
        : null;
    if (!hotspot) {
      return undefined;
    }
    if (
      interactionTarget &&
      interactionTarget.x === currentHotspot.x &&
      interactionTarget.y === currentHotspot.y
    ) {
      return { hotspot };
    }
    const hotspotDetails =
      mapDetails?.hotspots.find((candidate) =>
        candidate.visible &&
        candidate.interactable &&
        candidate.coords.x === currentHotspot.x &&
        candidate.coords.y === currentHotspot.y
      ) ?? null;
    const preferLateralNpcRecovery =
      hotspotDetails?.type === "npc" &&
      this.hasRecentNoEffectConfirmAtCurrentCoords(coords, mapDetails?.map ?? undefined);
    const recommendedApproach =
      hotspotDetails?.approach_tiles
        ? this.selectRecommendedApproachTile(hotspotDetails.approach_tiles, {
            coords,
            hotspotType: hotspotDetails.type,
            mapName: mapDetails?.map ?? undefined,
            preferLateralNpcRecovery,
            allowSpentNpcRecovery: hotspotDetails.type === "npc",
          })
        : null;
    return {
      hotspot,
      recommended_approach: recommendedApproach
        ? {
            x: recommendedApproach.coords.x,
            y: recommendedApproach.coords.y,
            facing: recommendedApproach.facing,
          }
        : undefined,
    };
  }

  private readInteractionLane(
    coords: { x: number; y: number } | undefined,
    facing: Direction | undefined,
    mapDetails?: McpMapInfoSnapshot,
    interactionTarget?: McpStatusSnapshot["interaction_target"]
  ): McpStatusSnapshot["interaction_lane"] {
    if (!coords || !mapDetails) {
      return undefined;
    }
    const stride = Math.max(1, mapDetails.coord_stride ?? 1);
    const candidateLanes = mapDetails.hotspots.flatMap((hotspot) =>
      hotspot.visible && hotspot.interactable && hotspot.approach_tiles?.length
        ? hotspot.approach_tiles.map((approachTile) => ({ hotspot, approachTile }))
        : []
    ).filter(({ approachTile }) => approachTile.coords.x === coords.x && approachTile.coords.y === coords.y);
    const interactionTargetLane =
      interactionTarget
        ? candidateLanes.find(
            ({ hotspot }) =>
              hotspot.coords.x === interactionTarget.x && hotspot.coords.y === interactionTarget.y
          ) ?? null
        : null;
    const objectiveLane =
      candidateLanes.find(({ hotspot }) => hotspot.type === "objective") ?? null;
    const npcLane =
      candidateLanes.find(({ hotspot }) => hotspot.type === "npc") ?? null;
    if (
      interactionTarget &&
      isLowAuthorityHotspotType(interactionTarget.hotspot_type) &&
      npcLane &&
      objectiveLane
    ) {
      return undefined;
    }
    const lane =
      (interactionTargetLane && isLowAuthorityHotspotType(interactionTargetLane.hotspot.type) && npcLane
        ? npcLane
        : null) ??
      interactionTargetLane ??
      objectiveLane ??
      npcLane ??
      candidateLanes[0] ??
      null;
    if (!lane) {
      return undefined;
    }
    const targetConfirmed =
      Boolean(
        interactionTarget &&
        lane.hotspot.coords.x === interactionTarget.x &&
        lane.hotspot.coords.y === interactionTarget.y
      );
    const facingAligned = facing === lane.approachTile.facing;
    const moveVector =
      lane.approachTile.facing === "up"
        ? { x: 0, y: -stride }
        : lane.approachTile.facing === "down"
          ? { x: 0, y: stride }
          : lane.approachTile.facing === "left"
            ? { x: -stride, y: 0 }
            : { x: stride, y: 0 };
    const facingMoveLeavesLane =
      coords.x + moveVector.x === lane.hotspot.coords.x &&
      coords.y + moveVector.y === lane.hotspot.coords.y;
    return {
      hotspot: {
        x: lane.hotspot.coords.x,
        y: lane.hotspot.coords.y,
        label: lane.hotspot.label,
        token: lane.hotspot.token,
        hotspot_type: lane.hotspot.type,
      },
      lane: {
        x: lane.approachTile.coords.x,
        y: lane.approachTile.coords.y,
        facing: lane.approachTile.facing,
        facing_aligned: facingAligned,
        facing_move_leaves_lane: facingMoveLeavesLane,
        target_confirmed: targetConfirmed,
      },
    };
  }

  private suppressLowAuthorityInteractionLaneForNpcPivot(
    interactionLane: McpStatusSnapshot["interaction_lane"] | undefined,
    localFocus: McpStatusSnapshot["local_focus"] | undefined
  ): McpStatusSnapshot["interaction_lane"] | undefined {
    if (!interactionLane) {
      return interactionLane;
    }
    if (
      isLowAuthorityHotspotType(interactionLane.hotspot.hotspot_type) &&
      localFocus?.source !== "interaction_lane"
    ) {
      return undefined;
    }
    if (
      localFocus?.source === "interaction_pivot" &&
      localFocus.target.kind === "npc" &&
      isLowAuthorityHotspotType(interactionLane.hotspot.hotspot_type)
    ) {
      return undefined;
    }
    return interactionLane;
  }

  private readLocalFocus(
    coords: { x: number; y: number } | undefined,
    mapDetails: McpMapInfoSnapshot | undefined,
    sceneOwner: NonNullable<NonNullable<McpStatusSnapshot["scene"]>["scene_owner"]> | undefined,
    interactionPivot: McpStatusSnapshot["local_focus"] | undefined,
    currentHotspot: McpStatusSnapshot["current_hotspot"] | undefined,
    interactionSetup: McpStatusSnapshot["interaction_setup"] | undefined,
    interactionLane: McpStatusSnapshot["interaction_lane"] | undefined,
    interactionTarget: McpStatusSnapshot["interaction_target"] | undefined
  ): McpStatusSnapshot["local_focus"] {
    const recentPromptResolutionWithVisibleObjective =
      Boolean(
        coords &&
        mapDetails &&
        currentHotspot?.hotspot_type === "npc" &&
        !sceneOwner &&
        !interactionTarget &&
        this.hasRecentPromptResolution(mapDetails.map ?? undefined) &&
        mapDetails.hotspots.some(
          (hotspot) =>
            hotspot.visible &&
            hotspot.interactable &&
            hotspot.type === "objective" &&
            (hotspot.coords.x !== currentHotspot.x || hotspot.coords.y !== currentHotspot.y)
        )
      );
    const objectiveApproachOverlapWithoutEngineTarget =
      Boolean(
        coords &&
        mapDetails &&
        !sceneOwner &&
        !interactionTarget &&
        mapDetails.hotspots.some(
          (hotspot) =>
            hotspot.visible &&
            hotspot.interactable &&
            hotspot.type === "objective" &&
            hotspot.approach_tiles?.some(
              (approachTile) => approachTile.coords.x === coords.x && approachTile.coords.y === coords.y
            )
        )
      );
    if (sceneOwner) {
      return {
        source: "scene_owner",
        target: {
          kind: sceneOwner.kind,
          x: sceneOwner.x,
          y: sceneOwner.y,
          label: sceneOwner.label,
          token: sceneOwner.token,
          hotspot_type: sceneOwner.hotspot_type,
          script: sceneOwner.script,
        },
      };
    }
    if (interactionPivot && !(objectiveApproachOverlapWithoutEngineTarget && interactionPivot.target.kind === "npc")) {
      return interactionPivot;
    }
    if (recentPromptResolutionWithVisibleObjective) {
      const nearestVisibleObjective =
        mapDetails?.hotspots
          .filter(
            (hotspot) =>
              hotspot.visible &&
              hotspot.interactable &&
              hotspot.type === "objective"
          )
          .sort((left, right) =>
            (Math.abs(left.coords.x - coords!.x) + Math.abs(left.coords.y - coords!.y)) -
              (Math.abs(right.coords.x - coords!.x) + Math.abs(right.coords.y - coords!.y)) ||
            left.coords.x - right.coords.x ||
            left.coords.y - right.coords.y ||
            left.label.localeCompare(right.label)
          )[0] ?? null;
      if (nearestVisibleObjective) {
        return {
          source: "visible_objective",
          target: {
            kind: "bg_event",
            x: nearestVisibleObjective.coords.x,
            y: nearestVisibleObjective.coords.y,
            label: nearestVisibleObjective.label,
            token: nearestVisibleObjective.token,
            hotspot_type: nearestVisibleObjective.type,
          },
        };
      }
    }
    if (currentHotspot && !isLowAuthorityHotspotType(currentHotspot.hotspot_type)) {
      return {
        source: "current_hotspot",
        target: {
          kind: currentHotspot.hotspot_type === "npc" ? "npc" : "bg_event",
          x: currentHotspot.x,
          y: currentHotspot.y,
          label: currentHotspot.label,
          token: currentHotspot.token,
          hotspot_type: currentHotspot.hotspot_type,
        },
      };
    }
    if (
      interactionLane &&
      !isLowAuthorityHotspotType(interactionLane.hotspot.hotspot_type) &&
      !(objectiveApproachOverlapWithoutEngineTarget && interactionLane.hotspot.hotspot_type === "npc")
    ) {
      return {
        source: "interaction_lane",
        target: {
          kind: interactionLane.hotspot.hotspot_type === "npc" ? "npc" : "bg_event",
          x: interactionLane.hotspot.x,
          y: interactionLane.hotspot.y,
          label: interactionLane.hotspot.label,
          token: interactionLane.hotspot.token,
          hotspot_type: interactionLane.hotspot.hotspot_type,
        },
      };
    }
    if (
      interactionSetup &&
      !isLowAuthorityHotspotType(interactionSetup.hotspot.hotspot_type) &&
      !(objectiveApproachOverlapWithoutEngineTarget && interactionSetup.hotspot.hotspot_type === "npc")
    ) {
      return {
        source: "interaction_setup",
        target: {
          kind: interactionSetup.hotspot.hotspot_type === "npc" ? "npc" : "bg_event",
          x: interactionSetup.hotspot.x,
          y: interactionSetup.hotspot.y,
          label: interactionSetup.hotspot.label,
          token: interactionSetup.hotspot.token,
          hotspot_type: interactionSetup.hotspot.hotspot_type,
        },
      };
    }
    if (interactionTarget && !isLowAuthorityHotspotType(interactionTarget.hotspot_type)) {
      return {
        source: "interaction_target",
        target: {
          kind: interactionTarget.kind,
          x: interactionTarget.x,
          y: interactionTarget.y,
          label: interactionTarget.label,
          token: interactionTarget.token,
          hotspot_type: interactionTarget.hotspot_type,
          script: interactionTarget.script,
        },
      };
    }
    return undefined;
  }

  private withLocalFocusApproach(
    coords: { x: number; y: number } | undefined,
    mapDetails: McpMapInfoSnapshot | undefined,
    localFocus: McpStatusSnapshot["local_focus"] | undefined
  ): McpStatusSnapshot["local_focus"] | undefined {
    if (!coords || !mapDetails || !localFocus?.target.x || !localFocus?.target.y) {
      return localFocus;
    }
    const hotspot =
      mapDetails.hotspots.find((candidate) =>
        candidate.visible &&
        candidate.interactable &&
        candidate.coords.x === localFocus.target.x &&
        candidate.coords.y === localFocus.target.y
      ) ?? null;
    if (!hotspot?.approach_tiles?.length) {
      return localFocus;
    }
    const onFocusedHotspot = hotspot.coords.x === coords.x && hotspot.coords.y === coords.y;
    const preferLateralNpcRecovery =
      hotspot.type === "npc" && this.hasRecentNoEffectConfirmAtCurrentCoords(coords, mapDetails?.map ?? undefined);
    const avoidImmediateBacktrackCoords =
      hotspot.type === "npc" && onFocusedHotspot
        ? this.readImmediateBacktrackApproachCoords(coords, mapDetails?.map ?? undefined, mapDetails.coord_stride ?? 1)
        : undefined;
    const recommendedApproach = this.selectRecommendedApproachTile(hotspot.approach_tiles, {
      coords,
      hotspotType: hotspot.type,
      mapName: mapDetails?.map ?? undefined,
      preferLateralNpcRecovery,
      avoidImmediateBacktrackCoords,
      allowSpentNpcRecovery:
        hotspot.type === "npc" &&
        (
          onFocusedHotspot ||
          localFocus.source === "interaction_pivot"
        ),
    });
    if (!recommendedApproach) {
      return localFocus;
    }
    return {
      ...localFocus,
      recommended_approach: {
        x: recommendedApproach.coords.x,
        y: recommendedApproach.coords.y,
        facing: recommendedApproach.facing,
      },
    };
  }

  private suppressStaleInteractionPivot(
    coords: { x: number; y: number } | undefined,
    mapDetails: McpMapInfoSnapshot | undefined,
    interactionLane: McpStatusSnapshot["interaction_lane"] | undefined,
    localFocus: McpStatusSnapshot["local_focus"] | undefined
  ): McpStatusSnapshot["local_focus"] | undefined {
    if (
      !coords ||
      !mapDetails ||
      !interactionLane ||
      localFocus?.source !== "interaction_pivot" ||
      localFocus.target.kind !== "npc" ||
      interactionLane.hotspot.hotspot_type !== "objective" ||
      interactionLane.lane.x !== coords.x ||
      interactionLane.lane.y !== coords.y
    ) {
      return localFocus;
    }
    const focusedNpc =
      mapDetails.hotspots.find((hotspot) =>
        hotspot.visible &&
        hotspot.interactable &&
        hotspot.type === "npc" &&
        hotspot.coords.x === localFocus.target.x &&
        hotspot.coords.y === localFocus.target.y
      ) ?? null;
    if (!focusedNpc?.approach_tiles?.length) {
      return localFocus;
    }
    const distinctInertNpcApproachConfirms = this.countRecentDistinctInertConfirmsOnApproachTiles(
      focusedNpc.approach_tiles,
      mapDetails.map ?? undefined
    );
    if (distinctInertNpcApproachConfirms < 2) {
      return localFocus;
    }
    return {
      source: "interaction_lane",
      target: {
        kind: "bg_event",
        x: interactionLane.hotspot.x,
        y: interactionLane.hotspot.y,
        label: interactionLane.hotspot.label,
        token: interactionLane.hotspot.token,
        hotspot_type: interactionLane.hotspot.hotspot_type,
      },
    };
  }

  private hasRecentNoEffectConfirmAtCurrentCoords(
    coords: { x: number; y: number } | undefined,
    mapName: string | undefined
  ): boolean {
    if (!coords) {
      return false;
    }
    return [...this.actionEvents].reverse().some((event) => {
      if (event.result.changed || event.result.reason !== "no_change") {
        return false;
      }
      if (mapName && event.map && event.map !== mapName) {
        return false;
      }
      if (!event.coords || event.coords.x !== coords.x || event.coords.y !== coords.y) {
        return false;
      }
      return /^(?:press:a:\d+|execute_macro:(?:interact|advance_dialog|mash_a):)/i.test(
        normalizeQuotedToken(event.action)
      );
    });
  }

  private hasRecentPromptResolution(mapName: string | undefined): boolean {
    return [...this.actionEvents]
      .reverse()
      .slice(0, 8)
      .some((event) => {
        if (mapName && event.map && event.map !== mapName) {
          return false;
        }
        const moments = event.moments ?? [];
        return moments.some(
          (moment) =>
            moment.startsWith("prompt_closed:") ||
            moment === "menu_closed" ||
            moment.startsWith("prompt_changed:")
        );
      });
  }

  private recentApproachVisitPenalty(
    approach: { coords: { x: number; y: number } },
    mapName: string | undefined
  ): number {
    const recentEvents = [...this.actionEvents]
      .reverse()
      .filter((event) => !mapName || !event.map || event.map === mapName)
      .slice(0, APPROACH_HISTORY_WINDOW);
    return recentEvents.reduce((penalty, event, index) => {
      if (!event.coords || event.coords.x !== approach.coords.x || event.coords.y !== approach.coords.y) {
        return penalty;
      }
      const recencyWeight = recentEvents.length - index;
      const inertPenalty =
        !event.result.changed &&
        (/^(?:press:a:\d+|execute_macro:(?:interact|advance_dialog|mash_a):)/i.test(
          normalizeQuotedToken(event.action)
        ) ||
          event.result.reason === "blocked")
          ? recencyWeight * 4
          : 0;
      return penalty + recencyWeight + inertPenalty;
    }, 0);
  }

  private readImmediateBacktrackApproachCoords(
    coords: { x: number; y: number } | undefined,
    mapName: string | undefined,
    coordStride: number
  ): { x: number; y: number } | undefined {
    if (!coords) {
      return undefined;
    }
    const latestMove = [...this.actionEvents]
      .reverse()
      .find((event) => {
        if (!event.result.changed) {
          return false;
        }
        if (mapName && event.map && event.map !== mapName) {
          return false;
        }
        if (!event.coords || event.coords.x !== coords.x || event.coords.y !== coords.y) {
          return false;
        }
        return /^move:(up|down|left|right):(\d+)$/i.test(normalizeQuotedToken(event.action));
      });
    if (!latestMove) {
      return undefined;
    }
    const match = normalizeQuotedToken(latestMove.action).match(/^move:(up|down|left|right):(\d+)$/i);
    if (!match || !latestMove.coords) {
      return undefined;
    }
    const direction = match[1].toLowerCase() as Direction;
    const times = Number.parseInt(match[2] ?? "1", 10);
    const stride = Math.max(1, coordStride) * Math.max(1, Number.isFinite(times) ? times : 1);
    switch (direction) {
      case "up":
        return { x: latestMove.coords.x, y: latestMove.coords.y + stride };
      case "down":
        return { x: latestMove.coords.x, y: latestMove.coords.y - stride };
      case "left":
        return { x: latestMove.coords.x + stride, y: latestMove.coords.y };
      case "right":
        return { x: latestMove.coords.x - stride, y: latestMove.coords.y };
    }
  }

  private hasRecentNpcApproachLoop(
    approachTiles: Array<{ coords: { x: number; y: number } }>,
    mapName: string | undefined
  ): boolean {
    if (approachTiles.length === 0) {
      return false;
    }
    const approachKeys = new Set(approachTiles.map((approach) => `${approach.coords.x},${approach.coords.y}`));
    const recentEvents = [...this.actionEvents]
      .reverse()
      .filter((event) => !mapName || !event.map || event.map === mapName)
      .slice(0, APPROACH_HISTORY_WINDOW);
    const visitedApproachKeys = new Set<string>();
    let sawInertSceneEvidence = false;

    for (const event of recentEvents) {
      if (!event.coords) {
        continue;
      }
      const key = `${event.coords.x},${event.coords.y}`;
      if (!approachKeys.has(key)) {
        continue;
      }
      visitedApproachKeys.add(key);
      if (
        (!event.result.changed && event.result.reason === "blocked") ||
        (!event.result.changed &&
          /^(?:press:a:\d+|execute_macro:(?:interact|advance_dialog|mash_a):)/i.test(
            normalizeQuotedToken(event.action)
          ))
      ) {
        sawInertSceneEvidence = true;
      }
    }

    return sawInertSceneEvidence && visitedApproachKeys.size >= Math.min(2, approachTiles.length);
  }

  private hasRecentInertConfirmOnApproachTiles(
    approachTiles: Array<{ coords: { x: number; y: number } }>,
    mapName: string | undefined,
    ignoredCoords?: { x: number; y: number }
  ): boolean {
    if (approachTiles.length === 0) {
      return false;
    }
    const approachKeys = new Set(approachTiles.map((approach) => `${approach.coords.x},${approach.coords.y}`));
    return [...this.actionEvents]
      .reverse()
      .filter((event) => !mapName || !event.map || event.map === mapName)
      .slice(0, APPROACH_HISTORY_WINDOW)
      .some((event) => {
        if (event.result.changed || event.result.reason !== "no_change" || !event.coords) {
          return false;
        }
        if (
          !/^(?:press:a:\d+|execute_macro:(?:interact|advance_dialog|mash_a):)/i.test(
            normalizeQuotedToken(event.action)
          )
        ) {
          return false;
        }
        if (
          ignoredCoords &&
          event.coords.x === ignoredCoords.x &&
          event.coords.y === ignoredCoords.y
        ) {
          return false;
        }
        return approachKeys.has(`${event.coords.x},${event.coords.y}`);
      });
  }

  private countRecentDistinctInertConfirmsOnApproachTiles(
    approachTiles: Array<{ coords: { x: number; y: number } }>,
    mapName: string | undefined
  ): number {
    if (approachTiles.length === 0) {
      return 0;
    }
    const approachKeys = new Set(approachTiles.map((approach) => `${approach.coords.x},${approach.coords.y}`));
    const visited = new Set<string>();
    for (const event of [...this.actionEvents]
      .reverse()
      .filter((event) => !mapName || !event.map || event.map === mapName)
      .slice(0, APPROACH_HISTORY_WINDOW)) {
      if (event.result.changed || event.result.reason !== "no_change" || !event.coords) {
        continue;
      }
      if (
        !/^(?:press:a:\d+|execute_macro:(?:interact|advance_dialog|mash_a):)/i.test(
          normalizeQuotedToken(event.action)
        )
      ) {
        continue;
      }
      const key = `${event.coords.x},${event.coords.y}`;
      if (approachKeys.has(key)) {
        visited.add(key);
      }
    }
    return visited.size;
  }

  private selectRecommendedApproachTile(
    approachTiles: Array<{ coords: { x: number; y: number }; facing: Direction }>,
    context: ApproachRecommendationContext
  ): { coords: { x: number; y: number }; facing: Direction } | null {
    const candidates = approachTiles.filter(
      (approach) => approach.coords.x !== context.coords.x || approach.coords.y !== context.coords.y
    );
    if (!candidates.length) {
      return null;
    }
    const rankedCandidates = candidates.map((approach) => ({
      approach,
      penalty: this.recentApproachVisitPenalty(approach, context.mapName),
    }));
    const hasCleanCandidate = rankedCandidates.some(({ penalty }) => penalty === 0);
    if (
      context.hotspotType === "npc" &&
      (!context.allowSpentNpcRecovery && (!hasCleanCandidate || this.hasRecentNpcApproachLoop(approachTiles, context.mapName)))
    ) {
      return null;
    }
    return (
      [...rankedCandidates].sort((left, right) =>
        (context.avoidImmediateBacktrackCoords
          ? Number(
              left.approach.coords.x === context.avoidImmediateBacktrackCoords.x &&
                left.approach.coords.y === context.avoidImmediateBacktrackCoords.y
            ) -
            Number(
              right.approach.coords.x === context.avoidImmediateBacktrackCoords.x &&
                right.approach.coords.y === context.avoidImmediateBacktrackCoords.y
            )
          : 0) ||
        (context.preferLateralNpcRecovery
          ? Number(left.approach.coords.y !== context.coords.y) -
            Number(right.approach.coords.y !== context.coords.y)
          : 0) ||
        left.penalty - right.penalty ||
        (Math.abs(left.approach.coords.x - context.coords.x) +
          Math.abs(left.approach.coords.y - context.coords.y)) -
          (Math.abs(right.approach.coords.x - context.coords.x) +
            Math.abs(right.approach.coords.y - context.coords.y)) ||
        left.approach.coords.x - right.approach.coords.x ||
        left.approach.coords.y - right.approach.coords.y ||
        left.approach.facing.localeCompare(right.approach.facing)
      )[0]?.approach ?? null
    );
  }

  private readInteractionPivot(
    coords: { x: number; y: number } | undefined,
    mapDetails: McpMapInfoSnapshot | undefined,
    interactionLane: McpStatusSnapshot["interaction_lane"] | undefined,
    interactionTarget: McpStatusSnapshot["interaction_target"] | undefined
  ): McpStatusSnapshot["local_focus"] | undefined {
    if (!coords || !mapDetails) {
      return undefined;
    }
    const recentInertObjectiveInteract = [...this.actionEvents]
      .reverse()
      .find((event) => {
        if (event.result.changed || event.result.reason !== "no_change") {
          return false;
        }
        if (event.map && mapDetails.map && event.map !== mapDetails.map) {
          return false;
        }
        if (!event.coords) {
          return false;
        }
        if (
          !/^(?:press:a:\d+|execute_macro:(?:interact|advance_dialog|mash_a):)/i.test(
            normalizeQuotedToken(event.action)
          )
        ) {
          return false;
        }
        return mapDetails.hotspots.some(
          (hotspot) =>
            hotspot.visible &&
            hotspot.interactable &&
            hotspot.type === "objective" &&
            hotspot.approach_tiles?.some(
              (approachTile) =>
                approachTile.coords.x === event.coords!.x && approachTile.coords.y === event.coords!.y
            )
        );
      });
    const recentInertObjectiveIsLocal =
      recentInertObjectiveInteract?.coords
        ? Math.abs(recentInertObjectiveInteract.coords.x - coords.x) +
            Math.abs(recentInertObjectiveInteract.coords.y - coords.y) <=
          RECENT_INERT_OBJECTIVE_PIVOT_RADIUS
        : false;
    const overlappingNpcLane =
      recentInertObjectiveInteract && recentInertObjectiveIsLocal
        ? mapDetails.hotspots
            .filter(
              (hotspot) =>
                hotspot.visible &&
                hotspot.interactable &&
                hotspot.type === "npc" &&
                hotspot.approach_tiles?.some(
                  (approachTile) => approachTile.coords.x === coords.x && approachTile.coords.y === coords.y
                )
            )
            .sort((left, right) => left.label.localeCompare(right.label))[0] ?? null
        : null;
    const nearestVisibleNpc =
      recentInertObjectiveInteract && recentInertObjectiveIsLocal
        ? mapDetails.hotspots
            .filter((hotspot) => hotspot.visible && hotspot.interactable && hotspot.type === "npc")
            .sort((left, right) =>
              (Math.abs(left.coords.x - coords.x) + Math.abs(left.coords.y - coords.y)) -
                (Math.abs(right.coords.x - coords.x) + Math.abs(right.coords.y - coords.y)) ||
              left.label.localeCompare(right.label)
            )[0] ?? null
        : null;
    const nearestVisibleNpcDistinctInertCount =
      nearestVisibleNpc?.approach_tiles?.length
        ? this.countRecentDistinctInertConfirmsOnApproachTiles(
            nearestVisibleNpc.approach_tiles,
            mapDetails.map ?? undefined
          )
        : 0;
    if (
      recentInertObjectiveInteract &&
      nearestVisibleNpcDistinctInertCount >= 2
    ) {
      return undefined;
    }
    if (overlappingNpcLane) {
      return {
        source: "interaction_pivot",
        target: {
          kind: "npc",
          x: overlappingNpcLane.coords.x,
          y: overlappingNpcLane.coords.y,
          label: overlappingNpcLane.label,
          token: overlappingNpcLane.token,
          hotspot_type: overlappingNpcLane.type,
        },
      };
    }
    if (nearestVisibleNpc) {
      return {
        source: "interaction_pivot",
        target: {
          kind: "npc",
          x: nearestVisibleNpc.coords.x,
          y: nearestVisibleNpc.coords.y,
          label: nearestVisibleNpc.label,
          token: nearestVisibleNpc.token,
          hotspot_type: nearestVisibleNpc.type,
        },
      };
    }
    const currentObjectiveTarget =
      interactionLane?.hotspot.hotspot_type === "objective"
        ? {
            x: interactionLane.hotspot.x,
            y: interactionLane.hotspot.y,
            hotspot_type: interactionLane.hotspot.hotspot_type,
          }
        : interactionTarget?.hotspot_type === "objective"
          ? {
              x: interactionTarget.x,
              y: interactionTarget.y,
              hotspot_type: interactionTarget.hotspot_type,
            }
          : null;
    if (!currentObjectiveTarget) {
      return undefined;
    }
    const recentInertInteract = recentInertObjectiveInteract
      ? recentInertObjectiveInteract.coords?.x === coords.x && recentInertObjectiveInteract.coords?.y === coords.y
        ? recentInertObjectiveInteract
        : null
      : null;
    return undefined;
  }

  private readActiveNpcSceneOwner(
    game: Game,
    mapDetails?: McpMapInfoSnapshot
  ): NonNullable<NonNullable<McpStatusSnapshot["scene"]>["scene_owner"]> | undefined {
    const state = game.getGameState();
    const overworld = game.getOverworld() as unknown as
      | {
          npcs?: Array<{
            objectIndex?: unknown;
            x?: unknown;
            y?: unknown;
            walking?: boolean;
            jumping?: boolean;
            event?: { script?: unknown } | null;
          }> | null;
          script_runner?: {
            last_interaction_object_index?: number | null;
          } | null;
        }
      | null
      | undefined;
    const runnerObjectIndex =
      typeof overworld?.script_runner?.last_interaction_object_index === "number" &&
      Number.isFinite(overworld.script_runner.last_interaction_object_index)
        ? overworld.script_runner.last_interaction_object_index
        : null;
    const lastTalked =
      typeof state.wram.last_talked === "number" && Number.isFinite(state.wram.last_talked)
        ? state.wram.last_talked
        : null;
    const objectIndex = runnerObjectIndex ?? lastTalked;
    if (objectIndex === null || objectIndex <= 0 || !Array.isArray(overworld?.npcs)) {
      return undefined;
    }
    const npc =
      overworld.npcs.find((candidate) =>
        typeof candidate?.objectIndex === "number" &&
        Number.isFinite(candidate.objectIndex) &&
        candidate.objectIndex === objectIndex
      ) ?? null;
    if (!npc) {
      return undefined;
    }
    const x = typeof npc.x === "number" && Number.isFinite(npc.x) ? npc.x : undefined;
    const y = typeof npc.y === "number" && Number.isFinite(npc.y) ? npc.y : undefined;
    const hotspot =
      (x !== undefined && y !== undefined
        ? (mapDetails?.hotspots.find((candidate) =>
            candidate.type === "npc" &&
            candidate.coords.x === x &&
            candidate.coords.y === y
          ) ?? null)
        : null) ??
      (mapDetails?.hotspots.find((candidate) => candidate.id === `npc-${objectIndex}`) ?? null);
    const script = String(npc.event?.script ?? "").trim() || undefined;
    if (!hotspot && x === undefined && y === undefined && !script) {
      return undefined;
    }
    return {
      kind: "npc",
      x,
      y,
      label: hotspot?.label,
      token: hotspot?.token,
      hotspot_type: hotspot?.type,
      script,
    };
  }

  private readMapIdentity(game: Game): MapIdentity {
    const state = game.getGameState();
    return {
      // game.getMapName() can briefly report "Unknown" during map transitions.
      // Prefer our best-effort map resolver so MCP traces stay readable.
      name: this.readBestMapName(game) ?? game.getMapName(),
      group: Number.isFinite(state.wram.wMapGroup) ? state.wram.wMapGroup : null,
      number: Number.isFinite(state.wram.wMapNumber) ? state.wram.wMapNumber : null,
    };
  }

  private hasMapChanged(baseline: MapIdentity, current: MapIdentity): boolean {
    if (baseline.name && current.name && baseline.name !== current.name) {
      return true;
    }
    if (baseline.group !== null && current.group !== null && baseline.group !== current.group) {
      return true;
    }
    if (baseline.number !== null && current.number !== null && baseline.number !== current.number) {
      return true;
    }
    return false;
  }

  private getStopReason(
    game: Game,
    baselineMap: MapIdentity | null,
    options: { ignoreMovementLock?: boolean } = {}
  ): string | null {
    if (game.isBattleActive?.() ?? false) {
      return "battle";
    }
    if (this.isUnownPuzzleInputActive(game)) {
      return null;
    }
    const promptStatus = promptFromSnapshot(this.lastSnapshot);
    const dialogueState = this.getDialogueUiState(game);
    if (promptStatus.pending) {
      return promptStatus.reason ?? "prompt";
    }
    if (dialogueState.yes_no_prompt_open) {
      return "prompt";
    }
    if (this.isMenuOpenForSession(game)) {
      return "menu";
    }
    if (this.isNameEntryActive(game)) {
      return "name_entry";
    }
    if (dialogueState.text_advance_pending) {
      return "dialogue";
    }
    if (dialogueState.input_owned) {
      return "dialogue";
    }
    if (this.lastSnapshot?.dialogue?.length) {
      return "dialogue";
    }
    if (this.isInputCaptureActive(game)) {
      return "prompt";
    }
    if (!options.ignoreMovementLock && this.isMovementLocked(game)) {
      return "movement_lock";
    }
    const scriptBusyReason = this.getScriptBusyReason(game);
    if (scriptBusyReason) {
      return scriptBusyReason;
    }
    if (baselineMap && this.hasMapChanged(baselineMap, this.readMapIdentity(game))) {
      return "map_transition";
    }
    return null;
  }

  private isMenuOpenForSession(game: Game): boolean {
    const snapshot = this.lastSnapshot as { menu?: string[] | null } | null;
    if (snapshot && "menu" in snapshot) {
      return Boolean(snapshot.menu?.length) || isInputOwningSurfaceSnapshot(this.lastSnapshot);
    }
    if (isInputOwningSurfaceSnapshot(this.lastSnapshot)) {
      return true;
    }
    return game.isMenuOpen();
  }

  private isNameEntryActive(game: Game): boolean {
    if (this.lastSnapshot?.info?.some((line) => /^STATE:\s*name_entry/i.test(String(line).trim()))) {
      return true;
    }
    const debugState = game.getDebugStatus?.() as { name_entry?: unknown } | null | undefined;
    return Boolean(debugState?.name_entry);
  }

  private isMovementActive(overworld: ReturnType<Game["getOverworld"]>): boolean {
    const candidate = overworld as { is_moving?: boolean; _turn_frames_remaining?: number; _ledge_jump_active?: boolean };
    return Boolean(
      candidate.is_moving ||
        (candidate._turn_frames_remaining ?? 0) > 0 ||
        candidate._ledge_jump_active
    );
  }

  private hasPlayerMoved(game: Game, start: [number, number]): boolean {
    const [x, y] = this.readPlayerCoords(game);
    return x !== start[0] || y !== start[1];
  }

  private readBlockReason(overworld: ReturnType<Game["getOverworld"]>): string | null {
    const feedback = (overworld as unknown as { _last_block_feedback?: { reason?: string } | null })
      ._last_block_feedback;
    const reason = feedback?.reason;
    if (typeof reason === "string" && reason.length > 0) {
      return reason === "map_edge" ? "terrain" : reason;
    }
    return null;
  }

  private settleMovementLock(game: Game, baselineMap: MapIdentity | null): string | null {
    const maxFrames = Math.max(
      1,
      Math.min(this.maxFramesPerCall, MOVEMENT_LOCK_RECOVERY_MAX_FRAMES)
    );
    for (let frame = 0; frame < maxFrames; frame += 1) {
      const reason = this.getStopReason(game, baselineMap, { ignoreMovementLock: true });
      const scriptBusyReason = reason === "script_runner" || reason === "script_tasks";
      if (reason && !scriptBusyReason) {
        return reason;
      }
      if (!this.isMovementLocked(game)) {
        return reason;
      }
      this.stepFrames(1);
    }
    return this.isMovementLocked(game) ? "movement_lock" : null;
  }

  private isMovementLocked(game: Game): boolean {
    const overworld = game.getOverworld();
    return typeof overworld.player_movement_locked === "function"
      ? overworld.player_movement_locked()
      : false;
  }

  private isInputCaptureActive(game: Game): boolean {
    return Boolean((game.getOverworld() as { input_capture_active?: unknown }).input_capture_active);
  }

  private isUnownPuzzleInputActive(game: Game): boolean {
    const state = game.getGameState?.();
    return Number(state?.wram?.wUnownState ?? 0) !== 0;
  }

  private getDialogueUiState(game: Game): DialogueUiState {
    const dialogue = (game.getOverworld() as {
      dialogue?: {
        active?: boolean;
        visible?: boolean;
        waiting_for_input?: boolean;
        pending_waits?: number;
        pendingWaits?: number;
        _yes_no_prompt?: { selection: number } | null;
      } | null;
    }).dialogue;
    const visible = Boolean(dialogue?.active || dialogue?.visible);
    const waitingForInput = Boolean(dialogue?.waiting_for_input);
    const pendingWaitsRaw = dialogue?.pending_waits ?? dialogue?.pendingWaits ?? 0;
    const pendingWaits = Number.isFinite(pendingWaitsRaw) ? Math.max(0, Number(pendingWaitsRaw)) : 0;
    const yesNoPromptOpen = Boolean(dialogue?._yes_no_prompt);
    const textAdvancePending = waitingForInput && !yesNoPromptOpen;
    const inputOwned = visible || waitingForInput;
    const dialogActive = inputOwned || yesNoPromptOpen;
    const textBoxOpen = dialogActive || pendingWaits > 0;
    return {
      visible,
      waiting_for_input: waitingForInput,
      pending_waits: pendingWaits,
      yes_no_prompt_open: yesNoPromptOpen,
      text_advance_pending: textAdvancePending,
      input_owned: inputOwned,
      dialog_active: dialogActive,
      text_box_open: textBoxOpen,
    };
  }

  private normalizeTimes(times: number): number {
    if (!Number.isFinite(times)) {
      throw new Error("Invalid input count.");
    }
    const normalized = Math.floor(times);
    if (normalized < 1 || normalized > this.maxActionsPerCall) {
      throw new Error("Input count exceeds allowed range.");
    }
    return normalized;
  }

  private normalizeDelayFrames(frames?: number): number {
    if (frames === undefined || frames === null) {
      return 0;
    }
    if (!Number.isFinite(frames)) {
      throw new Error("Invalid delay frame count.");
    }
    const normalized = Math.floor(frames);
    if (normalized < 0 || normalized > this.maxFramesPerCall) {
      throw new Error("Delay frame count exceeds allowed range.");
    }
    return normalized;
  }

  private normalizeHoldFrames(frames?: number): number {
    if (frames === undefined || frames === null) {
      return this.holdFrames;
    }
    if (!Number.isFinite(frames)) {
      throw new Error("Invalid hold frame count.");
    }
    const normalized = Math.floor(frames);
    if (normalized < 1 || normalized > this.maxFramesPerCall) {
      throw new Error("Hold frame count exceeds allowed range.");
    }
    return normalized;
  }

  private normalizeFrames(frames: number): number {
    if (!Number.isFinite(frames)) {
      throw new Error("Invalid frame count.");
    }
    const normalized = Math.floor(frames);
    if (normalized < 1 || normalized > this.maxFramesPerCall) {
      throw new Error("Frame count exceeds allowed range.");
    }
    return normalized;
  }

  reset(): void {
    this.game = null;
    this.gamePromise = null;
    this.ready = false;
    this.frameCounter = 0;
    this.sessionStartedAtMs = Date.now();
    this.actionEventTotal = 0;
    this.autosaveQueue = Promise.resolve();
    this.autosaveLastFrame = -1;
    this.scheduledEvents = [];
    this.actionLog = [];
    this.actionEvents = [];
    this.lastSnapshot = null;
    this.lastMcpMeta = null;
    this.lastActionResult = null;
  }
}

type SessionEntry = {
  session: McpGameSession;
  createdAt: number;
  lastAccessAt: number;
};

const sessions = new Map<string, SessionEntry>();

const pruneSessions = (now: number, maxSessions: number, ttlMs: number): void => {
  for (const [key, entry] of sessions.entries()) {
    if (key === PRIMARY_MCP_SESSION_ID) {
      continue;
    }
    if (now - entry.lastAccessAt >= ttlMs) {
      sessions.delete(key);
    }
  }
  if (sessions.size <= maxSessions) {
    return;
  }
  const ordered = Array.from(sessions.entries()).sort(
    (a, b) => a[1].lastAccessAt - b[1].lastAccessAt
  );
  while (sessions.size > maxSessions && ordered.length) {
    const [key] = ordered.shift() as [string, SessionEntry];
    if (key === PRIMARY_MCP_SESSION_ID) {
      continue;
    }
    sessions.delete(key);
  }
};

export const getMcpSession = (sessionId = PRIMARY_MCP_SESSION_ID): McpGameSession => {
  const settings = getSettings();
  const key = normalizeSessionId(sessionId);
  const now = Date.now();
  pruneSessions(now, settings.mcpMaxSessions, settings.mcpSessionTtlSeconds * 1000);
  const existing = sessions.get(key);
  if (existing) {
    existing.lastAccessAt = now;
    return existing.session;
  }
  const session = new McpGameSession({
    sessionId: key,
    maxActionsPerCall: settings.mcpMaxActionsPerCall,
  });
  sessions.set(key, {
    session,
    createdAt: now,
    lastAccessAt: now,
  });
  pruneSessions(now, settings.mcpMaxSessions, settings.mcpSessionTtlSeconds * 1000);
  return session;
};

export type McpActivitySummary = {
  activeSessions: number;
  sessions: Array<{
    sessionId: string;
    createdAt: number;
    lastAccessAt: number;
    source: "api_skills_mcp";
  }>;
};

export const getMcpActivitySummary = (): McpActivitySummary => {
  const settings = getSettings();
  const now = Date.now();
  pruneSessions(now, settings.mcpMaxSessions, settings.mcpSessionTtlSeconds * 1000);
  const details = Array.from(sessions.entries()).map(([sessionId, entry]) => ({
    sessionId,
    createdAt: entry.createdAt,
    lastAccessAt: entry.lastAccessAt,
    source: "api_skills_mcp" as const,
  }));
  return {
    activeSessions: details.length,
    sessions: details,
  };
};

export const __testing = {
  formatMcpSpeciesLabel,
  isInputOwningSurfaceSnapshot,
  isNonFatalRuntimeSnapshotPersistenceError,
  normalizeIdentityPlayerName,
  resolveSessionPlayerName,
  normalizePlayerGender,
  loadIdentityPlayProfile,
  resolveSessionPlayerGender,
  serializeRuntimeSnapshot,
  normalizeRuntimeSnapshotActionEvents,
  applyRuntimeSnapshot,
  mergeRuntimeSnapshotWithPrevious,
  resolveRuntimeSnapshotSlot,
  clearSessions(): void {
    sessions.clear();
  },
  hasSession(sessionId: string): boolean {
    return sessions.has(sessionId);
  },
  listSessionIds(): string[] {
    return [...sessions.keys()];
  },
  getSessionCount(): number {
    return sessions.size;
  },
};

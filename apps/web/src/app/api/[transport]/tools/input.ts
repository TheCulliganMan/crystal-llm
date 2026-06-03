import * as z from "zod";
import type {
  ActionResult,
  ActionResultWithSnapshot,
  McpPlayerContext,
  McpRecentEventsSnapshot,
  McpStatusSnapshot,
} from "@/app/mcp/session";
import type { TextSnapshotPayload } from "@/app/mcp/text-render";
import {
  MAX_ADVANCE_FRAMES,
  invalidateObserveSnapshotCache,
  McpToolExtra,
  McpToolResponse,
  loadSession,
  resolveSessionId,
  reportSnapshot,
  withRequestIdentity,
} from "./common";
import {
  IncludeSnapshotTextSchema,
  PayloadDetailSchema,
  PayloadFormatSchema,
  normalizePayloadOptions,
  serializeStructuredPayload,
} from "./serialization";
import { recordWebTrainingTurn, type WebTrainingAction } from "@/app/mcp/play-training-recorder";
type ActionCompactContext = {
  m: McpStatusSnapshot["mode"];
  map?: string;
  xy?: [number, number];
  dir: McpPlayerContext["facing"];
  bat?: 1;
  menu?: 1;
  dlg?: 1;
  txt?: 1;
  pr?: 1;
  lock?: 1;
  busy?: 1;
  mv?: 0;
  blk?: string;
  last?: string;
  n?: number;
};

const buildCanonicalContext = (
  sessionId: string | undefined,
  playerContext: McpPlayerContext,
  status: McpStatusSnapshot,
  recentEvents: McpRecentEventsSnapshot,
  actionResult?: ActionResult
) => {
  const compact = buildCompactContext(sessionId, playerContext, status, recentEvents, actionResult);
  return {
    mode: status.mode,
    surface: status.surface
      ? {
          kind: status.surface.kind,
          title: status.surface.title,
          state: status.surface.state,
          phase: status.surface.phase,
          selected: status.surface.selected,
        }
      : undefined,
    map: compact.map,
    coords: compact.xy,
    facing: compact.dir,
    inBattle: compact.bat || undefined,
    inMenu: compact.menu || undefined,
    inDialog: compact.dlg || undefined,
    textBoxOpen: compact.txt || undefined,
    textAdvancePending: status.text_advance_pending || undefined,
    promptPending: compact.pr || undefined,
    movementLocked: compact.lock || undefined,
    scriptBusy: compact.busy || undefined,
    canMove: compact.mv === 0 ? false : status.can_move,
    blockedReason: compact.blk,
    recentSummary: compact.last,
    recentCount: compact.n,
  };
};

const compactBool = (value: boolean | null | undefined): 1 | undefined => (value ? 1 : undefined);

const compactCoords = (
  coords: { x: number; y: number } | null | undefined
): [number, number] | undefined => (coords ? [coords.x, coords.y] : undefined);

const buildCompactContext = (
  sessionId: string | undefined,
  playerContext: McpPlayerContext,
  status: McpStatusSnapshot,
  recentEvents: McpRecentEventsSnapshot,
  actionResult?: ActionResult
): ActionCompactContext => {
  const eventCount = Array.isArray(recentEvents.events) ? recentEvents.events.length : 0;
  const coords = playerContext.coords ?? status.coords ?? undefined;
  const map = playerContext.map ?? status.map ?? undefined;
  return {
    m: status.mode,
    map: map ?? undefined,
    xy: compactCoords(coords),
    dir: playerContext.facing,
    bat: compactBool(status.in_battle),
    menu: compactBool(playerContext.menu_open),
    dlg: compactBool(status.in_dialog || playerContext.dialogue_open),
    txt: compactBool(status.text_box_open ?? status.textbox_open),
    pr: compactBool(status.prompt?.pending),
    lock: compactBool(status.movement_locked),
    busy: compactBool(status.script_busy),
    mv: status.can_move ? undefined : 0,
    blk: status.input_blocked_reason ?? undefined,
    last: recentEvents.recap && recentEvents.recap !== "no_events" ? recentEvents.recap : undefined,
    n: eventCount || undefined,
  };
};

const deriveActionEffect = (actionResult: ActionResult): string => {
  const events = actionResult.events ?? [];
  if (events.some((event) => event.startsWith("moved:"))) return "moved";
  if (events.some((event) => event.includes("mode:overworld->menu"))) return "opened_menu";
  if (events.some((event) => event.includes("mode:menu->overworld"))) return "closed_menu";
  if (events.some((event) => event.includes("prompt_opened"))) return "opened_prompt";
  if (events.some((event) => event.includes("dialog"))) return "advanced_dialogue";
  if (events.some((event) => event.includes("warp"))) return "triggered_warp";
  if (actionResult.reason === "blocked") return "blocked";
  if (actionResult.reason === "menu") return "menu_locked";
  if (actionResult.reason === "busy") return "busy";
  if (actionResult.reason === "no_change") return "no_effect";
  return actionResult.changed ? "changed" : "no_effect";
};

const buildRecommendedApproachPayload = (
  approach:
    | {
        x: number;
        y: number;
        facing: "up" | "down" | "left" | "right";
      }
    | undefined,
  coordStride: number
):
  | {
      coords: [number, number];
      facing: "up" | "down" | "left" | "right";
      setupFrom: [number, number];
    }
  | undefined => {
  if (!approach) {
    return undefined;
  }
  const stride = Math.max(1, coordStride);
  const setupFrom: [number, number] =
    approach.facing === "up"
      ? [approach.x, approach.y + stride]
      : approach.facing === "down"
        ? [approach.x, approach.y - stride]
        : approach.facing === "left"
          ? [approach.x + stride, approach.y]
          : [approach.x - stride, approach.y];
  return {
    coords: [approach.x, approach.y],
    facing: approach.facing,
    setupFrom,
  };
};

const DIRECTION_CONVENTION = {
  coord: "x+ right, x- left, y+ down, y- up",
  move: {
    up: [0, -1],
    down: [0, 1],
    left: [-1, 0],
    right: [1, 0],
  },
} as const;

type EmbeddedObserveState = {
  ow?: {
    g?: unknown;
    o?: unknown;
    p?: unknown;
  };
};

const parseEmbeddedObserveState = (snapshotText: string): EmbeddedObserveState | null => {
  const lines = snapshotText
    .split("\n")
    .map((line) => (line.endsWith("\r") ? line.slice(0, -1) : line))
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    const line = lines[index]!;
    if (!line.startsWith("{") || !line.endsWith("}")) {
      continue;
    }
    try {
      const parsed = JSON.parse(line) as EmbeddedObserveState;
      return parsed && typeof parsed === "object" ? parsed : null;
    } catch {
      continue;
    }
  }
  return null;
};

const isTravelTile = (token: string): boolean =>
  token === "." || token.startsWith("D");

const buildLocalMovementPayload = (snapshotText: string):
  | {
      openDirections: Array<{ direction: "up" | "down" | "left" | "right"; tile: string }>;
      blockedDirections: Array<{ direction: "up" | "down" | "left" | "right"; tile: string }>;
    }
  | undefined => {
  const parsed = parseEmbeddedObserveState(snapshotText);
  const rawGrid = parsed?.ow?.g;
  const rawPlayer = parsed?.ow?.p;
  const rawOrigin = parsed?.ow?.o;
  if (!Array.isArray(rawGrid) || !Array.isArray(rawPlayer) || rawPlayer.length < 2) {
    return undefined;
  }
  const playerX = typeof rawPlayer[0] === "number" ? rawPlayer[0] : undefined;
  const playerY = typeof rawPlayer[1] === "number" ? rawPlayer[1] : undefined;
  if (playerX == null || playerY == null) {
    return undefined;
  }
  const originX = Array.isArray(rawOrigin) && typeof rawOrigin[0] === "number" ? rawOrigin[0] : 0;
  const originY = Array.isArray(rawOrigin) && typeof rawOrigin[1] === "number" ? rawOrigin[1] : 0;
  const grid = rawGrid.filter(Array.isArray) as unknown[][];
  const tileAt = (x: number, y: number): string | undefined => {
    const row = grid[y - originY];
    const tile = row?.[x - originX];
    return typeof tile === "string" ? tile : undefined;
  };
  const directions: Array<{ direction: "up" | "down" | "left" | "right"; x: number; y: number }> = [
    { direction: "up", x: playerX, y: playerY - 1 },
    { direction: "down", x: playerX, y: playerY + 1 },
    { direction: "left", x: playerX - 1, y: playerY },
    { direction: "right", x: playerX + 1, y: playerY },
  ];
  const openDirections: Array<{ direction: "up" | "down" | "left" | "right"; tile: string }> = [];
  const blockedDirections: Array<{ direction: "up" | "down" | "left" | "right"; tile: string }> = [];
  for (const entry of directions) {
    const tile = tileAt(entry.x, entry.y);
    if (!tile) {
      continue;
    }
    if (isTravelTile(tile)) {
      openDirections.push({ direction: entry.direction, tile });
    } else {
      blockedDirections.push({ direction: entry.direction, tile });
    }
  }
  if (openDirections.length === 0 && blockedDirections.length === 0) {
    return undefined;
  }
  return {
    openDirections,
    blockedDirections,
  };
};

const buildTrainingActionResultSnapshot = (actionResult: ActionResult): string => {
  const lines = [
    `ok: ${actionResult.ok ? 1 : 0}`,
    `ch: ${actionResult.changed ? 1 : 0}`,
    `fx: ${deriveActionEffect(actionResult)}`,
  ];
  if (actionResult.reason) {
    lines.push(`rsn: ${actionResult.reason}`);
  }
  const events = actionResult.events ?? [];
  if (events.length) {
    lines.push(`ev: ${events.join(" | ")}`);
  }
  return lines.join("\n");
};

const buildTrainingStatusSnapshot = (status: McpStatusSnapshot): string => {
  const lines = [
    `m: ${status.mode}`,
    `map: ${status.map ?? ""}`,
    `xy: ${status.coords ? `${status.coords.x},${status.coords.y}` : ""}`,
    `dir: ${status.facing ?? ""}`,
    `bat: ${status.in_battle ? 1 : 0}`,
    `menu: ${status.in_menu ? 1 : 0}`,
    `dlg: ${status.in_dialog ? 1 : 0}`,
    `txt: ${status.textbox_open ?? status.text_box_open ? 1 : 0}`,
    `pr: ${status.prompt_pending ? 1 : 0}`,
    `lock: ${status.movement_locked ? 1 : 0}`,
    `busy: ${status.script_busy ? 1 : 0}`,
    `mv: ${status.can_move ? 1 : 0}`,
  ];
  if (status.input_blocked_reason) {
    lines.push(`blk: ${status.input_blocked_reason}`);
  }
  return lines.join("\n");
};

const buildTrainingRecentEventsSnapshot = (recentEvents: McpRecentEventsSnapshot): string => {
  const lines = [`sum: ${recentEvents.recap}`, `n: ${recentEvents.total}`];
  if (recentEvents.truncated) {
    lines.push("tr: 1");
  }
  if (recentEvents.events.length) {
    lines.push(`ev: ${recentEvents.events.map((event) => event.summary).join(" | ")}`);
  }
  return lines.join("\n");
};

const maybeRecordDevTrainingTurn = (input: {
  sessionId: string | undefined;
  baseUrl?: string | undefined;
  rawKey: string;
  action: WebTrainingAction;
  beforeSnapshot: string;
  actionResult: ActionResult;
  afterSnapshot: string;
  statusSnapshot: McpStatusSnapshot;
  recentEventsSnapshot: McpRecentEventsSnapshot;
}): void => {
  if (!input.sessionId) {
    return;
  }
  recordWebTrainingTurn({
    sessionId: input.sessionId,
    baseUrl: input.baseUrl ?? "",
    rawKey: input.rawKey,
    action: input.action,
    beforeSnapshot: input.beforeSnapshot,
    actionResultSnapshot: buildTrainingActionResultSnapshot(input.actionResult),
    afterSnapshot: input.afterSnapshot,
    statusSnapshot: buildTrainingStatusSnapshot(input.statusSnapshot),
    recentEventsSnapshot: buildTrainingRecentEventsSnapshot(input.recentEventsSnapshot),
    responseMeta: {
      action_result: [input.actionResult as Record<string, unknown>],
      observe: [{ snapshot: input.afterSnapshot }],
      status: [input.statusSnapshot as unknown as Record<string, unknown>],
      recent_events: [input.recentEventsSnapshot as unknown as Record<string, unknown>],
    },
  });
};

const buildActionContent = async (
  sessionId: string | undefined,
  actionResult: ActionResult,
  snapshotText: string | undefined,
  playerContext?: McpPlayerContext,
  statusSnapshot?: McpStatusSnapshot,
  recentEventsSnapshot?: McpRecentEventsSnapshot,
  frameId?: number,
  computedAtMs?: number,
  options: {
    format?: "json";
    detail?: "full" | "compact";
    include_snapshot_text?: boolean;
    include_tui_state?: boolean;
    frame_payload?: TextSnapshotPayload | null;
  } = {}
): Promise<McpToolResponse["content"]> => {
  const normalized = normalizePayloadOptions(options);
  const content: McpToolResponse["content"] = [];
  if (normalized.include_snapshot_text) {
    content.push({ type: "text", text: snapshotText ?? "" });
  }

  const payload = {
    context:
      playerContext && statusSnapshot && recentEventsSnapshot
        ? buildCanonicalContext(sessionId, playerContext, statusSnapshot, recentEventsSnapshot, actionResult)
        : undefined,
    recentEvents:
      recentEventsSnapshot && recentEventsSnapshot.total > 0
        ? {
            summary: recentEventsSnapshot.recap,
            total: recentEventsSnapshot.total,
            truncated: recentEventsSnapshot.truncated || undefined,
          }
        : undefined,
    action: {
      ok: actionResult.ok,
      changed: actionResult.changed,
      effect: deriveActionEffect(actionResult),
      reason: actionResult.reason,
      events: actionResult.events,
    },
    tui:
      options.include_tui_state && playerContext && statusSnapshot && recentEventsSnapshot
        ? {
            status: statusSnapshot,
            recent_events: recentEventsSnapshot,
            frame: options.frame_payload
              ? {
                  ctx: buildCompactContext(sessionId, playerContext, statusSnapshot, recentEventsSnapshot, actionResult),
                  view: {
                    viewport: options.frame_payload.viewport,
                    info: options.frame_payload.info,
                    menu: options.frame_payload.menu ?? undefined,
                    prompt: options.frame_payload.prompt ?? undefined,
                    dialogue: options.frame_payload.dialogue ?? undefined,
                    frame: frameId,
                  },
                  surface: statusSnapshot.surface,
                  frame: frameId,
                }
              : undefined,
            frame_id: frameId,
            computed_at_ms: computedAtMs,
          }
        : undefined,
  };

  const serialized = await serializeStructuredPayload(payload);
  content.push({
    type: "text",
    text: serialized.text,
    mimeType: serialized.mimeType,
  });
  return content;
};

const isCompactSnapshotActionResponse = (options: {
  format?: "json";
  detail?: "full" | "compact";
  include_snapshot_text?: boolean;
  include_tui_state?: boolean;
}): boolean =>
  options.include_tui_state !== true &&
  options.include_snapshot_text === true &&
  options.detail === "compact" &&
  options.format !== "json";

type ToolUnavailableState = {
  reason: "battle" | "dialogue" | "name_entry";
  message: string;
};

const DIRECTION_BUTTONS = new Set(["up", "down", "left", "right"]);
const MCP_TUI_TEXT_ADVANCE_SETTLE_FRAMES = 25;

const isAgentVisibleHotspotType = (hotspotType: string | undefined): boolean =>
  hotspotType !== "trigger";

const resolveFieldToolUnavailableState = (
  toolName: string,
  status: McpStatusSnapshot,
  options: { allowBattleNavigation?: boolean; allowPromptNavigation?: boolean; allowDialogueNavigation?: boolean } = {}
): ToolUnavailableState | undefined => {
  if (status.in_battle && !options.allowBattleNavigation) {
    return {
      reason: "battle",
      message: `${toolName} is not available during battle.`,
    };
  }
  const promptNavigationOpen = Boolean(status.prompt_pending || status.surface?.prompt_open);
  if (
    status.in_dialog ||
    status.prompt_pending ||
    status.text_advance_pending ||
    status.textbox_open ||
    status.text_box_open ||
    status.surface?.dialogue_open ||
    status.surface?.prompt_open
  ) {
    if (options.allowDialogueNavigation) {
      return undefined;
    }
    if (options.allowPromptNavigation && promptNavigationOpen) {
      return undefined;
    }
    return {
      reason: "dialogue",
      message: `${toolName} is not available during dialogue.`,
    };
  }
  return undefined;
};

const buildUnavailableToolResponse = async (
  toolName: string,
  unavailable: ToolUnavailableState,
  status: McpStatusSnapshot
): Promise<McpToolResponse> => {
  const serialized = await serializeStructuredPayload({
    available: false,
    error: {
      code: "tool_not_available",
      message: unavailable.message,
      tool: toolName,
      reason: unavailable.reason,
    },
    context: {
      mode: status.mode,
      map: status.map,
      inBattle: status.in_battle || undefined,
      inDialog: status.in_dialog || undefined,
      textBoxOpen: (status.textbox_open ?? status.text_box_open) || undefined,
      textAdvancePending: status.text_advance_pending || undefined,
      promptPending: status.prompt_pending || undefined,
      blockedReason: status.input_blocked_reason ?? undefined,
    },
  });
  return {
    content: [{ type: "text", text: serialized.text, mimeType: serialized.mimeType }],
    isError: true,
  };
};

const isTuiTextAdvancePressAStatus = (status: McpStatusSnapshot): boolean => {
  if (status.in_battle) {
    return false;
  }
  const surfaceKind = String(status.surface?.kind ?? "").trim().toLowerCase();
  if (
    status.mode === "name_entry" ||
    status.input_blocked_reason === "name_entry" ||
    surfaceKind === "name_entry" ||
    surfaceKind === "unown_puzzle" ||
    surfaceKind === "slot_machine"
  ) {
    return false;
  }
  if (surfaceKind === "mart" || status.surface?.title?.trim().toLowerCase() === "mart") {
    return false;
  }
  return Boolean(
    status.in_dialog ||
    status.prompt_pending ||
    status.textbox_open ||
    status.text_box_open ||
    status.text_advance_pending ||
    status.input_blocked_reason === "dialogue" ||
    status.input_blocked_reason === "prompt" ||
    status.surface?.dialogue_open ||
    status.surface?.prompt_open ||
    status.surface?.waiting
  );
};

const isTuiMenuLikeDirectionalStatus = (status: McpStatusSnapshot): boolean => {
  const surfaceKind = String(status.surface?.kind ?? "").trim().toLowerCase();
  return Boolean(
    status.mode === "menu" ||
    status.mode === "title" ||
    status.mode === "main_menu" ||
    status.mode === "continue" ||
    status.mode === "delete_save" ||
    status.mode === "clock_reset" ||
    status.mode === "gender" ||
    status.mode === "name_entry" ||
    status.in_menu ||
    status.menu ||
    status.prompt_pending ||
    status.input_blocked_reason === "prompt" ||
    status.input_blocked_reason === "menu" ||
    status.surface?.menu_open ||
    status.surface?.prompt_open ||
    surfaceKind === "pokegear" ||
    surfaceKind === "slot_machine" ||
    surfaceKind === "fly_to_where"
  );
};

const isTuiDialogueDirectionalNoopStatus = (status: McpStatusSnapshot): boolean => {
  const surfaceKind = String(status.surface?.kind ?? "").trim().toLowerCase();
  const unownActive = Boolean(
    status.unown_puzzle_active ||
    (status.unown_state ?? 0) !== 0 ||
    surfaceKind === "unown_puzzle"
  );
  if (unownActive || isTuiMenuLikeDirectionalStatus(status)) {
    return false;
  }
  return Boolean(
    status.in_dialog ||
    status.textbox_open ||
    status.text_box_open ||
    status.text_advance_pending ||
    status.input_blocked_reason === "dialogue" ||
    status.surface?.dialogue_open ||
    status.surface?.waiting
  );
};

export const RecentEventsSchema = z.object({
  limit: coerceOptionalInt(1, 50),
  format: PayloadFormatSchema,
  detail: PayloadDetailSchema,
});

export const recentEventsHandler = async (
  input: z.infer<typeof RecentEventsSchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  return withRequestIdentity(extra, async () => {
    const resolvedSessionId = resolveSessionId(extra);
    const session = await loadSession(resolvedSessionId, extra);
    const snapshot = await session.recentEvents(input.limit ?? 10);
    normalizePayloadOptions({
      format: input.format,
      detail: input.detail,
    });
    const payload = {
      total: snapshot.total,
      session_started_at_ms: snapshot.session_started_at_ms,
      session_started_at_iso: snapshot.session_started_at_iso,
      time_played_ms: snapshot.time_played_ms,
      summary: snapshot.recap !== "no_events" ? snapshot.recap : undefined,
      truncated: snapshot.truncated || undefined,
      events: snapshot.events.map((event) => ({
        frame: event.frame,
        action: event.action,
        mode: event.mode,
        map: event.map,
        coords: event.coords ? [event.coords.x, event.coords.y] : undefined,
        summary: event.summary,
        ok: event.result.ok,
        changed: event.result.changed,
        reason: event.result.reason,
        events: event.result.events,
      })),
    };
    const serialized = await serializeStructuredPayload(payload);
    return {
      content: [{ type: "text", text: serialized.text, mimeType: serialized.mimeType }],
    };
  });
};

export const JournalSchema = RecentEventsSchema;

export const journalHandler = recentEventsHandler;

const AGENT_STATUS_HOTSPOT_LIMIT = 16;

export const StatusSchema = z.object({
  format: PayloadFormatSchema,
  detail: PayloadDetailSchema,
});

type VisibleStatusOverride = {
  mode: string;
  map: string;
  location: string;
  mapId: string;
  canMove: false;
  blockedReason: string;
  inMenu?: boolean;
  promptPending?: boolean;
  suppressOverworldContext: true;
};

const surfaceLabel = (surface: McpStatusSnapshot["surface"] | undefined): string => {
  if (!surface) {
    return "";
  }
  return [surface.kind, surface.title, surface.state, surface.primary_text]
    .filter((entry): entry is string => Boolean(entry && entry.trim()))
    .join(" ")
    .toUpperCase();
};

const deriveVisibleStatusOverrideFromSurface = (
  surface: McpStatusSnapshot["surface"] | undefined
): VisibleStatusOverride | undefined => {
  if (!surface) {
    return undefined;
  }
  const kind = surface.kind;
  const label = surfaceLabel(surface);
  if (kind === "title" || label.includes("TITLE SCREEN")) {
    return {
      mode: "title",
      map: "TITLE",
      location: "TITLE",
      mapId: "title",
      canMove: false,
      blockedReason: "title_screen",
      suppressOverworldContext: true,
    };
  }
  if (kind === "oak_intro") {
    return {
      mode: "oak_intro",
      map: "OAK INTRO",
      location: "OAK INTRO",
      mapId: "oak_intro",
      canMove: false,
      blockedReason: "oak_intro",
      inMenu: true,
      promptPending: true,
      suppressOverworldContext: true,
    };
  }
  if (kind === "name_entry") {
    return {
      mode: "name_entry",
      map: "NAME ENTRY",
      location: "NAME ENTRY",
      mapId: "name_entry",
      canMove: false,
      blockedReason: "name_entry",
      inMenu: true,
      promptPending: true,
      suppressOverworldContext: true,
    };
  }
  if (kind === "pc" || /\bPC\b/.test(label)) {
    return {
      mode: "menu",
      map: "PC",
      location: surface.title || "PC",
      mapId: "pc",
      canMove: false,
      blockedReason: "pc",
      inMenu: true,
      promptPending: Boolean(surface.prompt_open) || undefined,
      suppressOverworldContext: true,
    };
  }
  return undefined;
};

const deriveVisibleStatusOverride = (snapshotText: string): VisibleStatusOverride | undefined => {
  const lines = snapshotText
    .split(/\r?\n/g)
    .map((line) => line.trim())
    .filter(Boolean);
  const upperLines = lines.map((line) => line.toUpperCase());
  if (upperLines.some((line) => line === "TITLE SCREEN")) {
    return {
      mode: "title",
      map: "TITLE",
      location: "TITLE",
      mapId: "title",
      canMove: false,
      blockedReason: "title_screen",
      suppressOverworldContext: true,
    };
  }
  if (
    upperLines.some((line) => line === "NAME ENTRY") ||
    lines.some((line) => /^STATE:\s*name_entry$/i.test(line))
  ) {
    return {
      mode: "name_entry",
      map: "NAME ENTRY",
      location: "NAME ENTRY",
      mapId: "name_entry",
      canMove: false,
      blockedReason: "name_entry",
      inMenu: true,
      promptPending: true,
      suppressOverworldContext: true,
    };
  }
  if (
    upperLines.some((line) => line === "OAK INTRO" || line === "OAK FINALE") ||
    lines.some((line) => /^STATE:\s*oak_intro$/i.test(line))
  ) {
    return {
      mode: "oak_intro",
      map: "OAK INTRO",
      location: "OAK INTRO",
      mapId: "oak_intro",
      canMove: false,
      blockedReason: "oak_intro",
      inMenu: true,
      promptPending: true,
      suppressOverworldContext: true,
    };
  }
  if (
    upperLines.some((line) => /\bPC\b/.test(line)) &&
    upperLines.some((line) => line.includes("D-PAD=MOVE") || line.includes("A=SELECT") || line.includes("B=BACK"))
  ) {
    return {
      mode: "menu",
      map: "PC",
      location: "PC",
      mapId: "pc",
      canMove: false,
      blockedReason: "pc",
      inMenu: true,
      suppressOverworldContext: true,
    };
  }
  return undefined;
};

export const statusHandler = async (
  input: z.infer<typeof StatusSchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  return withRequestIdentity(extra, async () => {
    const resolvedSessionId = resolveSessionId(extra);
    const session = await loadSession(resolvedSessionId, extra);
    const status = await session.status();
    const observeSnapshot = typeof session.observeText === "function" ? session.observeText() : "";
    const visibleOverride =
      deriveVisibleStatusOverrideFromSurface(status.surface) ?? deriveVisibleStatusOverride(observeSnapshot);
    const coordStride = Math.max(1, status.map_details?.coord_stride ?? 1);
    normalizePayloadOptions({
      format: input.format,
      detail: input.detail,
    });
    const payload = {
      mode: visibleOverride?.mode ?? status.mode,
      surface: status.surface
        ? {
            kind: status.surface.kind,
            title: status.surface.title,
            state: status.surface.state,
            phase: status.surface.phase,
            waiting: status.surface.waiting,
            menuOpen: status.surface.menu_open,
            promptOpen: status.surface.prompt_open,
            dialogueOpen: status.surface.dialogue_open,
            selected: status.surface.selected,
            controls: status.surface.controls,
            primaryText: status.surface.primary_text,
          }
        : undefined,
      map: visibleOverride?.map ?? status.map,
      location: visibleOverride?.location ?? status.location_name,
      mapId: visibleOverride?.mapId ?? status.map_id,
      directionConvention: DIRECTION_CONVENTION,
      coords: visibleOverride ? undefined : status.coords ? [status.coords.x, status.coords.y] : undefined,
      interactionTile: !visibleOverride && status.interaction_tile
        ? [status.interaction_tile.x, status.interaction_tile.y]
        : undefined,
      interactionTarget: !visibleOverride && status.interaction_target && isAgentVisibleHotspotType(status.interaction_target.hotspot_type)
        ? {
            coords: [status.interaction_target.x, status.interaction_target.y],
            kind: status.interaction_target.kind,
            label: status.interaction_target.label,
            token: status.interaction_target.token,
            hotspotType: status.interaction_target.hotspot_type,
            script: status.interaction_target.script,
          }
        : undefined,
      currentHotspot: !visibleOverride && status.current_hotspot && isAgentVisibleHotspotType(status.current_hotspot.hotspot_type)
        ? {
            coords: [status.current_hotspot.x, status.current_hotspot.y],
            label: status.current_hotspot.label,
            token: status.current_hotspot.token,
            hotspotType: status.current_hotspot.hotspot_type,
          }
        : undefined,
      interactionSetup: !visibleOverride && status.interaction_setup && isAgentVisibleHotspotType(status.interaction_setup.hotspot.hotspot_type)
        ? {
            hotspot: {
              coords: [status.interaction_setup.hotspot.x, status.interaction_setup.hotspot.y],
              label: status.interaction_setup.hotspot.label,
              token: status.interaction_setup.hotspot.token,
              hotspotType: status.interaction_setup.hotspot.hotspot_type,
            },
            recommendedApproach: status.interaction_setup.recommended_approach
              ? buildRecommendedApproachPayload(status.interaction_setup.recommended_approach, coordStride)
              : undefined,
          }
        : undefined,
      interactionLane: !visibleOverride && status.interaction_lane && isAgentVisibleHotspotType(status.interaction_lane.hotspot.hotspot_type)
        ? {
            hotspot: {
              coords: [status.interaction_lane.hotspot.x, status.interaction_lane.hotspot.y],
              label: status.interaction_lane.hotspot.label,
              token: status.interaction_lane.hotspot.token,
              hotspotType: status.interaction_lane.hotspot.hotspot_type,
            },
            lane: {
              coords: [status.interaction_lane.lane.x, status.interaction_lane.lane.y],
              facing: status.interaction_lane.lane.facing,
              facingAligned: status.interaction_lane.lane.facing_aligned,
              facingMoveLeavesLane: status.interaction_lane.lane.facing_move_leaves_lane,
              targetConfirmed: status.interaction_lane.lane.target_confirmed,
            },
          }
        : undefined,
      localFocus: !visibleOverride && status.local_focus && isAgentVisibleHotspotType(status.local_focus.target.hotspot_type)
        ? {
            source: status.local_focus.source,
            target: {
              kind: status.local_focus.target.kind,
              coords:
                status.local_focus.target.x !== undefined && status.local_focus.target.y !== undefined
                  ? [status.local_focus.target.x, status.local_focus.target.y]
                  : undefined,
              label: status.local_focus.target.label,
              token: status.local_focus.target.token,
              hotspotType: status.local_focus.target.hotspot_type,
              script: status.local_focus.target.script,
            },
            recommendedApproach: status.local_focus.recommended_approach
              ? buildRecommendedApproachPayload(status.local_focus.recommended_approach, coordStride)
              : undefined,
          }
        : undefined,
      scene: !visibleOverride && status.scene
        ? {
            activeScript: status.scene.active_script,
            owner: status.scene.scene_owner && isAgentVisibleHotspotType(status.scene.scene_owner.hotspot_type)
              ? {
                  kind: status.scene.scene_owner.kind,
                  coords:
                    status.scene.scene_owner.x !== undefined && status.scene.scene_owner.y !== undefined
                      ? [status.scene.scene_owner.x, status.scene.scene_owner.y]
                      : undefined,
                  label: status.scene.scene_owner.label,
                  token: status.scene.scene_owner.token,
                  hotspotType: status.scene.scene_owner.hotspot_type,
                  script: status.scene.scene_owner.script,
                }
              : undefined,
          }
        : undefined,
      facing: visibleOverride ? undefined : status.facing,
      badges: status.badges_count,
      money: status.money,
      momsMoney: status.moms_money,
      momSavingSomeMoney: status.mom_saving_some_money,
      inMenu: visibleOverride?.inMenu || status.in_menu || undefined,
      inBattle: status.in_battle || undefined,
      inDialog: status.in_dialog || undefined,
      textBoxOpen: (status.textbox_open ?? status.text_box_open) || undefined,
      textAdvancePending: status.text_advance_pending || undefined,
      promptPending: visibleOverride?.promptPending || status.prompt_pending || undefined,
      movementLocked: status.movement_locked || undefined,
      scriptBusy: status.script_busy || undefined,
      canMove: visibleOverride?.canMove ?? status.can_move,
      blockedReason: visibleOverride?.blockedReason ?? status.input_blocked_reason ?? undefined,
      engineDebug: status.engine_debug,
      localMovement: visibleOverride ? undefined : buildLocalMovementPayload(observeSnapshot),
      partyCount: status.party_summary?.count,
      flowSummary: status.flow_state?.summary,
      flowNextGoal: status.flow_state?.next_goal?.title,
      flowCompletionTarget: status.flow_state?.completion_target.title,
      audio: status.audio
        ? {
            musicToken: status.audio.musicToken,
            musicRole: status.audio.musicRole,
            musicSource: status.audio.musicSource,
            recentEvents: status.audio.recentEvents.map((event) => ({
              sequence: event.sequence,
              kind: event.kind,
              token: event.token,
              source: event.source,
              role: event.role,
              loop: event.loop,
            })),
          }
        : undefined,
    };
    const serialized = await serializeStructuredPayload(payload);
    return {
      content: [{ type: "text", text: serialized.text, mimeType: serialized.mimeType }],
    };
  });
};

const normalizedDirectionSchema = z.preprocess(
  (value) => (typeof value === "string" ? value.trim().toLowerCase() : value),
  z.enum(["up", "down", "left", "right"])
);

function coerceOptionalInt(min: number, max: number) {
  return z.preprocess((value) => {
    if (typeof value === "string") {
      const trimmed = value.trim();
      if (/^\d+$/.test(trimmed)) {
        return Number(trimmed);
      }
    }
    return value;
  }, z.number().int().min(min).max(max).optional());
}

function coerceInt(min: number, max: number) {
  return z.preprocess((value) => {
    if (typeof value === "string") {
      const trimmed = value.trim();
      if (/^\d+$/.test(trimmed)) {
        return Number(trimmed);
      }
    }
    return value;
  }, z.number().int().min(min).max(max));
}

const normalizedMacroNameSchema = z.preprocess(
  (value) => (typeof value === "string" ? value.trim().toLowerCase() : value),
  z.enum(["advance_dialog", "mash_a", "interact", "approach_target"])
);

const resolveDeterministicCount = (
  values: Array<number | undefined>,
  fallback = 1
): number => {
  const provided = values.filter((value): value is number => typeof value === "number");
  if (!provided.length) {
    return fallback;
  }
  const first = provided[0] ?? fallback;
  if (provided.some((value) => value !== first)) {
    throw new Error("Conflicting repeat counts provided.");
  }
  return first;
};

export const MoveSchema = z.object({
  direction: normalizedDirectionSchema,
  times: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
  steps: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
  count: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
  format: PayloadFormatSchema,
  detail: PayloadDetailSchema,
  include_snapshot_text: IncludeSnapshotTextSchema,
  include_tui_state: z.boolean().optional(),
});

export const moveHandler = async (
  input: z.infer<typeof MoveSchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  return withRequestIdentity(extra, async () => {
    const { direction, times, steps, count, format, detail, include_snapshot_text, include_tui_state } = input;
    const repeats = resolveDeterministicCount([times, steps, count], 1);
    const resolvedSessionId = resolveSessionId(extra);
    const session = await loadSession(resolvedSessionId, extra);
    const compactSnapshotOnly = isCompactSnapshotActionResponse({ format, detail, include_snapshot_text, include_tui_state });
    const beforeSnapshot = compactSnapshotOnly ? "" : session.observeText();
    if (!compactSnapshotOnly) {
      const statusBefore = await session.status();
      const unavailable = resolveFieldToolUnavailableState("move", statusBefore, {
        allowBattleNavigation: true,
        allowPromptNavigation: true,
      });
      if (unavailable) {
        return buildUnavailableToolResponse("move", unavailable, statusBefore);
      }
    }
    invalidateObserveSnapshotCache(resolvedSessionId);
    const action = await session.move(
      direction,
      repeats,
      compactSnapshotOnly ? { settleSnapshot: false } : undefined
    );
    const snapshotText = action.snapshotText ?? session.observeText();
    if (compactSnapshotOnly) {
      return {
        content: await buildActionContent(
          resolvedSessionId,
          action.result,
          snapshotText,
          undefined,
          undefined,
          undefined,
          undefined,
          Date.now(),
          { format, detail, include_snapshot_text }
        ),
      };
    }
    const playerContext = await session.playerContext();
    const statusSnapshot = await session.status();
    const recentEventsSnapshot = await session.recentEvents(5);
    const frameId = session.getFrameCount();
    const computedAtMs = Date.now();
    const framePayload = include_tui_state ? session.observePayload() : undefined;
    await reportSnapshot(resolvedSessionId, session, snapshotText, `move:${direction}`);
    maybeRecordDevTrainingTurn({
      sessionId: resolvedSessionId,
      baseUrl: "",
      rawKey:
        {
          up: "ArrowUp",
          down: "ArrowDown",
          left: "ArrowLeft",
          right: "ArrowRight",
        }[direction],
      action: { type: "move", direction },
      beforeSnapshot,
      actionResult: action.result,
      afterSnapshot: snapshotText,
      statusSnapshot,
      recentEventsSnapshot,
    });
    return {
      content: await buildActionContent(
        resolvedSessionId,
        action.result,
        snapshotText,
        playerContext,
        statusSnapshot,
        recentEventsSnapshot,
        frameId,
        computedAtMs,
        { format, detail, include_snapshot_text, include_tui_state, frame_payload: framePayload }
      ),
    };
  });
};

const normalizedButtonValues = ["a", "b", "start", "select", "up", "down", "left", "right"] as const;
const normalizedButtonSchema = z.preprocess(
  (value) => (typeof value === "string" ? value.trim() : value),
  z
    .union([
      ...normalizedButtonValues.map((button) => z.literal(button)),
      ...normalizedButtonValues.map((button) => z.literal(button.toUpperCase())),
    ])
    .transform((button: string) => button.toLowerCase())
);

export const PressSchema = z.object({
  button: normalizedButtonSchema,
  times: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
  count: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
  format: PayloadFormatSchema,
  detail: PayloadDetailSchema,
  include_snapshot_text: IncludeSnapshotTextSchema,
  include_tui_state: z.boolean().optional(),
});

export const TypeTextSchema = z.object({
  text: z.string().min(1).max(MAX_ADVANCE_FRAMES),
  clear: z.boolean().optional(),
  submit: z.boolean().optional(),
  format: PayloadFormatSchema,
  detail: PayloadDetailSchema,
  include_snapshot_text: IncludeSnapshotTextSchema,
  include_tui_state: z.boolean().optional(),
});

export const pressHandler = async (
  input: z.infer<typeof PressSchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  return withRequestIdentity(extra, async () => {
    const { button, times, count, format, detail, include_snapshot_text, include_tui_state } = input;
    const repeats = resolveDeterministicCount([times, count], 1);
    const resolvedSessionId = resolveSessionId(extra);
    const session = await loadSession(resolvedSessionId, extra);
    const compactSnapshotOnly = isCompactSnapshotActionResponse({ format, detail, include_snapshot_text, include_tui_state });
    const beforeSnapshot = compactSnapshotOnly ? "" : session.observeText();
    const statusBefore = compactSnapshotOnly ? null : await session.status();
    invalidateObserveSnapshotCache(resolvedSessionId);
    const action =
      statusBefore && DIRECTION_BUTTONS.has(button) && isTuiDialogueDirectionalNoopStatus(statusBefore)
        ? {
            result: {
              ok: true,
              changed: false,
              reason: "no_change" as const,
              events: [`noop:${button}:dialogue`],
            },
            snapshotText: beforeSnapshot || session.observeText(),
          }
      : statusBefore && button === "a" && repeats === 1 && isTuiTextAdvancePressAStatus(statusBefore)
        ? await session.executeNamedMacro("advance_dialog", {
            maxPresses: 8,
            settleFrames: statusBefore.instant_mode ? 0 : MCP_TUI_TEXT_ADVANCE_SETTLE_FRAMES,
          })
        : await session.press(
            button,
            repeats,
            compactSnapshotOnly ? { settleSnapshot: false } : undefined
          );
    const snapshotText = action.snapshotText ?? session.observeText();
    if (compactSnapshotOnly) {
      return {
        content: await buildActionContent(
          resolvedSessionId,
          action.result,
          snapshotText,
          undefined,
          undefined,
          undefined,
          undefined,
          Date.now(),
          { format, detail, include_snapshot_text }
        ),
      };
    }
    const playerContext = await session.playerContext();
    const statusSnapshot = await session.status();
    const recentEventsSnapshot = await session.recentEvents(5);
    const frameId = session.getFrameCount();
    const computedAtMs = Date.now();
    const framePayload = include_tui_state ? session.observePayload() : undefined;
    await reportSnapshot(resolvedSessionId, session, snapshotText, `press:${button}`);
    maybeRecordDevTrainingTurn({
      sessionId: resolvedSessionId,
      baseUrl: "",
      rawKey:
        {
          a: "KeyZ",
          b: "KeyX",
          start: "Enter",
          select: "Backspace",
          up: "ArrowUp",
          down: "ArrowDown",
          left: "ArrowLeft",
          right: "ArrowRight",
        }[button],
      action: { type: "press", button },
      beforeSnapshot,
      actionResult: action.result,
      afterSnapshot: snapshotText,
      statusSnapshot,
      recentEventsSnapshot,
    });
    return {
      content: await buildActionContent(
        resolvedSessionId,
        action.result,
        snapshotText,
        playerContext,
        statusSnapshot,
        recentEventsSnapshot,
        frameId,
        computedAtMs,
        { format, detail, include_snapshot_text, include_tui_state, frame_payload: framePayload }
      ),
    };
  });
};

export const typeTextHandler = async (
  input: z.infer<typeof TypeTextSchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  return withRequestIdentity(extra, async () => {
    const { text, clear, submit, format, detail, include_snapshot_text, include_tui_state } = input;
    const resolvedSessionId = resolveSessionId(extra);
    const session = await loadSession(resolvedSessionId, extra);
    const beforeSnapshot = session.observeText();
    invalidateObserveSnapshotCache(resolvedSessionId);
    const action = await session.typeText(text, { clear, submit });
    const playerContext = await session.playerContext();
    const statusSnapshot = await session.status();
    const recentEventsSnapshot = await session.recentEvents(5);
    const snapshotText = action.snapshotText ?? session.observeText();
    const frameId = session.getFrameCount();
    const computedAtMs = Date.now();
    const framePayload = include_tui_state ? session.observePayload() : undefined;
    await reportSnapshot(resolvedSessionId, session, snapshotText, `type_text:${text}`);
    return {
      content: await buildActionContent(
        resolvedSessionId,
        action.result,
        snapshotText,
        playerContext,
        statusSnapshot,
        recentEventsSnapshot,
        frameId,
        computedAtMs,
        { format, detail, include_snapshot_text, include_tui_state, frame_payload: framePayload }
      ),
    };
  });
};

export const HoldButtonSchema = z.object({
  button: normalizedButtonSchema,
  frames: coerceInt(1, MAX_ADVANCE_FRAMES),
  format: PayloadFormatSchema,
  detail: PayloadDetailSchema,
  include_snapshot_text: IncludeSnapshotTextSchema,
  include_tui_state: z.boolean().optional(),
});

export const holdButtonHandler = async (
  input: z.infer<typeof HoldButtonSchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  return withRequestIdentity(extra, async () => {
    const { button, frames, format, detail, include_snapshot_text, include_tui_state } = input;
    const resolvedSessionId = resolveSessionId(extra);
    const session = await loadSession(resolvedSessionId, extra);
    invalidateObserveSnapshotCache(resolvedSessionId);
    const action = await session.holdButton(button, frames);
    const playerContext = await session.playerContext();
    const statusSnapshot = await session.status();
    const recentEventsSnapshot = await session.recentEvents(5);
    const snapshotText = action.snapshotText ?? session.observeText();
    const frameId = session.getFrameCount();
    const computedAtMs = Date.now();
    const framePayload = include_tui_state ? session.observePayload() : undefined;
    await reportSnapshot(resolvedSessionId, session, snapshotText, `hold_button:${button}`);
    return {
      content: await buildActionContent(
        resolvedSessionId,
        action.result,
        snapshotText,
        playerContext,
        statusSnapshot,
        recentEventsSnapshot,
        frameId,
        computedAtMs,
        { format, detail, include_snapshot_text, include_tui_state, frame_payload: framePayload }
      ),
    };
  });
};

export const WaitSchema = z.object({
  frames: coerceInt(1, MAX_ADVANCE_FRAMES),
  format: PayloadFormatSchema,
  detail: PayloadDetailSchema,
  include_snapshot_text: IncludeSnapshotTextSchema,
  include_tui_state: z.boolean().optional(),
});

export const waitHandler = async (
  input: z.infer<typeof WaitSchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  return withRequestIdentity(extra, async () => {
    const { frames, format, detail, include_snapshot_text, include_tui_state } = input;
    const resolvedSessionId = resolveSessionId(extra);
    const session = await loadSession(resolvedSessionId, extra);
    const beforeSnapshot = session.observeText();
    invalidateObserveSnapshotCache(resolvedSessionId);
    const action = await session.executeMacro([{ type: "wait", frames }], {
      delay_frames: 0,
      stop_on_event: false,
    });
    const playerContext = await session.playerContext();
    const statusSnapshot = await session.status();
    const recentEventsSnapshot = await session.recentEvents(5);
    const snapshotText = action.snapshotText ?? session.observeText();
    const frameId = session.getFrameCount();
    const computedAtMs = Date.now();
    const framePayload = include_tui_state ? session.observePayload() : undefined;
    await reportSnapshot(resolvedSessionId, session, snapshotText, `wait:${frames}`);
    maybeRecordDevTrainingTurn({
      sessionId: resolvedSessionId,
      baseUrl: "",
      rawKey: ".",
      action: { type: "wait", frames },
      beforeSnapshot,
      actionResult: action.result,
      afterSnapshot: snapshotText,
      statusSnapshot,
      recentEventsSnapshot,
    });
    return {
      content: await buildActionContent(
        resolvedSessionId,
        action.result,
        snapshotText,
        playerContext,
        statusSnapshot,
        recentEventsSnapshot,
        frameId,
        computedAtMs,
        { format, detail, include_snapshot_text, include_tui_state, frame_payload: framePayload }
      ),
    };
  });
};

const ExecuteMacroActionSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("move"),
    value: normalizedDirectionSchema,
    times: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
    hold_frames: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
    delay_frames: coerceOptionalInt(0, MAX_ADVANCE_FRAMES),
  }),
  z.object({
    type: z.literal("button"),
    value: normalizedButtonSchema,
    times: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
    hold_frames: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
    delay_frames: coerceOptionalInt(0, MAX_ADVANCE_FRAMES),
  }),
]);

const hasFieldMovementAction = (actions: Array<z.infer<typeof ExecuteMacroActionSchema>> | undefined): boolean =>
  Boolean(
    actions?.some((action) =>
      action.type === "move" ||
      (action.type === "button" && DIRECTION_BUTTONS.has(action.value))
    )
  );

export const ExecuteMacroSchema = z.object({
  actions: z.array(ExecuteMacroActionSchema).min(1).optional(),
  macro: normalizedMacroNameSchema.optional(),
  target_token: z.string().trim().min(1).max(8).optional(),
  max_presses: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
  max_steps: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
  max_observes: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
  max_tries: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
  press_a: z.boolean().optional(),
  settle_frames: coerceOptionalInt(0, MAX_ADVANCE_FRAMES),
  delay_frames: coerceOptionalInt(0, MAX_ADVANCE_FRAMES),
  stop_on_event: z.boolean().optional(),
  format: PayloadFormatSchema,
  detail: PayloadDetailSchema,
  include_snapshot_text: IncludeSnapshotTextSchema,
  include_tui_state: z.boolean().optional(),
}).superRefine((value, ctx) => {
  if (!value.actions?.length && !value.macro) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "Either actions or macro must be provided.",
      path: ["actions"],
    });
  }
  if (value.actions?.length && value.macro) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "Provide either actions or macro, not both.",
      path: ["macro"],
    });
  }
  if (value.macro === "approach_target" && !value.target_token) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: "target_token is required when macro=approach_target.",
      path: ["target_token"],
    });
  }
});

const isFieldMacro = (input: z.infer<typeof ExecuteMacroSchema>): boolean =>
  input.macro === "interact" ||
  input.macro === "approach_target" ||
  hasFieldMovementAction(input.actions);

const isDialogueMacro = (input: z.infer<typeof ExecuteMacroSchema>): boolean =>
  input.macro === "advance_dialog" || input.macro === "mash_a";

const resolveDialogueMacroUnavailableState = (
  toolName: string,
  status: McpStatusSnapshot
): ToolUnavailableState | undefined => {
  if (status.mode === "name_entry" || status.input_blocked_reason === "name_entry") {
    return {
      reason: "name_entry",
      message: `${toolName} dialogue macros are not available during name entry.`,
    };
  }
  return undefined;
};

type ViewportMacroSession = {
  observeText: () => string;
  move: (direction: z.infer<typeof normalizedDirectionSchema>, times?: number) => Promise<ActionResultWithSnapshot>;
  press: (button: "a", times?: number) => Promise<ActionResultWithSnapshot>;
  status?: () => Promise<{ map?: string | null }>;
};

type ViewportState = {
  grid: string[][];
  player: { x: number; y: number; facing: z.infer<typeof normalizedDirectionSchema> | "unknown" };
  target: { x: number; y: number; token: string };
};

const DIRECTION_VECTOR: Record<z.infer<typeof normalizedDirectionSchema>, { x: number; y: number }> = {
  up: { x: 0, y: -1 },
  down: { x: 0, y: 1 },
  left: { x: -1, y: 0 },
  right: { x: 1, y: 0 },
};

const CARDINAL_DIRECTIONS: Array<z.infer<typeof normalizedDirectionSchema>> = [
  "up",
  "right",
  "down",
  "left",
];

const tokenFacing = (token: string): z.infer<typeof normalizedDirectionSchema> | "unknown" => {
  if (token.includes("^")) return "up";
  if (token.includes("v")) return "down";
  if (token.includes("<")) return "left";
  if (token.includes(">")) return "right";
  return "unknown";
};

const parseViewportState = (snapshotText: string, targetToken: string): ViewportState | null => {
  const lines = snapshotText.split("\n");
  const gridStart = lines.findIndex((line) => line.startsWith("OVERWORLD"));
  if (gridStart < 0) {
    return null;
  }
  const grid: string[][] = [];
  for (let i = gridStart + 2; i < lines.length; i += 1) {
    const line = lines[i] ?? "";
    if (!/^\d\d /.test(line)) {
      break;
    }
    const parts = line.split(/\s+/).filter(Boolean);
    grid.push(parts.slice(1));
  }
  if (!grid.length) {
    return null;
  }

  const normalizedTarget = normalizeTargetToken(targetToken);
  const matchesTarget = (tile: string): boolean => {
    const normalizedTile = tile.toUpperCase();
    if (normalizedTarget.length === 1) {
      return normalizedTile.startsWith(normalizedTarget);
    }
    return normalizedTile === normalizedTarget || normalizedTile.startsWith(normalizedTarget);
  };

  let player: ViewportState["player"] | null = null;
  const targets: Array<ViewportState["target"]> = [];
  for (let y = 0; y < grid.length; y += 1) {
    for (let x = 0; x < (grid[y]?.length ?? 0); x += 1) {
      const tile = grid[y]?.[x];
      if (!tile) continue;
      if (tile.startsWith("@")) {
        player = { x, y, facing: tokenFacing(tile) };
        continue;
      }
      if (matchesTarget(tile)) {
        targets.push({ x, y, token: tile });
      }
    }
  }
  if (!player || !targets.length) {
    return null;
  }
  const nearest = targets.reduce((best, current) => {
    const bestDistance = Math.abs(best.x - player.x) + Math.abs(best.y - player.y);
    const currentDistance = Math.abs(current.x - player.x) + Math.abs(current.y - player.y);
    if (currentDistance < bestDistance) return current;
    if (currentDistance > bestDistance) return best;
    if (current.y < best.y) return current;
    if (current.y > best.y) return best;
    return current.x < best.x ? current : best;
  });
  return { grid, player, target: nearest };
};

const normalizeTargetToken = (targetToken: string): string => {
  const normalized = targetToken.trim().toUpperCase();
  if (normalized === "DOOR") return "D";
  return normalized;
};

const isWarpTargetToken = (targetToken: string): boolean => {
  const normalized = normalizeTargetToken(targetToken);
  return normalized === "D" || normalized === "W";
};

const manhattanDistance = (
  a: { x: number; y: number },
  b: { x: number; y: number }
): number => Math.abs(a.x - b.x) + Math.abs(a.y - b.y);

const directionTowardTarget = (
  player: { x: number; y: number },
  target: { x: number; y: number }
): z.infer<typeof normalizedDirectionSchema> => {
  const dx = target.x - player.x;
  const dy = target.y - player.y;
  if (Math.abs(dx) >= Math.abs(dy)) {
    return dx >= 0 ? "right" : "left";
  }
  return dy >= 0 ? "down" : "up";
};

const isLikelyBlocked = (tile: string | undefined): boolean => {
  if (!tile) return true;
  if (tile === "#" || tile === "x" || tile === "X") return true;
  if (tile.startsWith("@")) return true;
  if (tile.startsWith("N") || tile.startsWith("D") || tile.startsWith("W") || tile.startsWith("E")) return true;
  return false;
};

const chooseReducingDirection = (
  state: ViewportState
  ,
  options: { allowTargetTile?: boolean } = {}
): z.infer<typeof normalizedDirectionSchema> | null => {
  const dx = state.target.x - state.player.x;
  const dy = state.target.y - state.player.y;
  const primary: Array<z.infer<typeof normalizedDirectionSchema>> = [];
  const secondary: Array<z.infer<typeof normalizedDirectionSchema>> = [];

  if (Math.abs(dx) >= Math.abs(dy)) {
    if (dx !== 0) primary.push(dx > 0 ? "right" : "left");
    if (dy !== 0) primary.push(dy > 0 ? "down" : "up");
  } else {
    if (dy !== 0) primary.push(dy > 0 ? "down" : "up");
    if (dx !== 0) primary.push(dx > 0 ? "right" : "left");
  }

  primary.forEach((direction) => {
    const vector = DIRECTION_VECTOR[direction];
    const x = state.player.x + vector.x;
    const y = state.player.y + vector.y;
    const tile = state.grid[y]?.[x];
    const isTargetTile = x === state.target.x && y === state.target.y;
    if (!isLikelyBlocked(tile) || (options.allowTargetTile && isTargetTile)) {
      secondary.push(direction);
    }
  });

  if (secondary.length) {
    return secondary[0] ?? null;
  }
  return primary[0] ?? null;
};

const isWalkableViewportTile = (
  tile: string | undefined,
  options: { allowTargetTile?: boolean; isTargetTile?: boolean } = {}
): boolean =>
  !isLikelyBlocked(tile) || (options.allowTargetTile === true && options.isTargetTile === true);

const findPathDirection = (
  state: ViewportState,
  options: { allowTargetTile?: boolean; goalMode?: "adjacent" | "occupy" } = {}
): z.infer<typeof normalizedDirectionSchema> | null => {
  const height = state.grid.length;
  const width = Math.max(...state.grid.map((row) => row.length));
  const startKey = `${state.player.x},${state.player.y}`;
  const queue: Array<{ x: number; y: number }> = [{ x: state.player.x, y: state.player.y }];
  const previous = new Map<string, { x: number; y: number; direction: z.infer<typeof normalizedDirectionSchema> | null }>();
  previous.set(startKey, { x: state.player.x, y: state.player.y, direction: null });

  const isGoal = (x: number, y: number): boolean =>
    options.goalMode === "occupy"
      ? x === state.target.x && y === state.target.y
      : Math.abs(x - state.target.x) + Math.abs(y - state.target.y) === 1;

  while (queue.length > 0) {
    const current = queue.shift();
    if (!current) {
      break;
    }
    if (!(current.x === state.player.x && current.y === state.player.y) && isGoal(current.x, current.y)) {
      let cursorKey = `${current.x},${current.y}`;
      let cursor = previous.get(cursorKey);
      while (cursor && cursor.direction === null && cursorKey !== startKey) {
        cursorKey = `${cursor.x},${cursor.y}`;
        cursor = previous.get(cursorKey);
      }
      while (cursor && cursorKey !== startKey) {
        const parentKey = `${cursor.x},${cursor.y}`;
        const parent = previous.get(parentKey);
        if (!parent || parentKey === startKey) {
          return cursor.direction;
        }
        cursorKey = parentKey;
        cursor = parent;
      }
    }

    for (const direction of CARDINAL_DIRECTIONS) {
      const vector = DIRECTION_VECTOR[direction];
      const nextX = current.x + vector.x;
      const nextY = current.y + vector.y;
      if (nextX < 0 || nextY < 0 || nextY >= height || nextX >= width) {
        continue;
      }
      const tile = state.grid[nextY]?.[nextX];
      if (
        !isWalkableViewportTile(tile, {
          allowTargetTile: options.allowTargetTile,
          isTargetTile: nextX === state.target.x && nextY === state.target.y,
        })
      ) {
        continue;
      }
      const nextKey = `${nextX},${nextY}`;
      if (previous.has(nextKey)) {
        continue;
      }
      previous.set(nextKey, {
        x: current.x,
        y: current.y,
        direction: current.x === state.player.x && current.y === state.player.y ? direction : previous.get(`${current.x},${current.y}`)?.direction ?? direction,
      });
      queue.push({ x: nextX, y: nextY });
    }
  }

  return null;
};

const buildApproachMacroFallbackResult = (
  events: string[],
  reason: ActionResult["reason"] = "no_change"
): ActionResultWithSnapshot => ({
  result: {
    ok: false,
    changed: false,
    reason,
    events,
  },
});

export const runViewportApproachMacro = async (
  session: ViewportMacroSession,
  options: {
    targetToken: string;
    maxSteps?: number;
    maxObserves?: number;
    maxTries?: number;
    pressA?: boolean;
  }
): Promise<ActionResultWithSnapshot> => {
  const maxSteps = Math.max(1, Math.min(MAX_ADVANCE_FRAMES, Math.trunc(options.maxSteps ?? 40)));
  const maxObserves = Math.max(1, Math.min(MAX_ADVANCE_FRAMES, Math.trunc(options.maxObserves ?? maxSteps * 2)));
  const maxTries = Math.max(1, Math.min(MAX_ADVANCE_FRAMES, Math.trunc(options.maxTries ?? 4)));
  const pressA = options.pressA ?? true;
  let stepsTaken = 0;
  let observesUsed = 0;
  let lastAction: ActionResultWithSnapshot | null = null;
  let currentMap = (await session.status?.())?.map ?? null;
  const targetIsWarp = isWarpTargetToken(options.targetToken);

  const observeState = (): ViewportState | null => {
    if (observesUsed >= maxObserves) {
      return null;
    }
    observesUsed += 1;
    return parseViewportState(session.observeText(), options.targetToken);
  };

  while (stepsTaken < maxSteps) {
    const state = observeState();
    if (!state) {
      return lastAction ?? buildApproachMacroFallbackResult(["observe_limit_or_target_missing"]);
    }

    const distance = manhattanDistance(state.player, state.target);
    if (!targetIsWarp && distance === 1) {
      const desiredFacing = directionTowardTarget(state.player, state.target);
      for (let attempt = 0; attempt < maxTries; attempt += 1) {
        const refreshed = attempt === 0 ? state : observeState();
        if (!refreshed) {
          return lastAction ?? buildApproachMacroFallbackResult(["observe_limit_during_interaction"]);
        }
        if (manhattanDistance(refreshed.player, refreshed.target) !== 1) {
          break;
        }
        const desired = directionTowardTarget(refreshed.player, refreshed.target);
        if (refreshed.player.facing !== desired) {
          const direction = attempt === 0 ? desiredFacing : CARDINAL_DIRECTIONS[(attempt - 1) % CARDINAL_DIRECTIONS.length] ?? desired;
          lastAction = await session.move(direction, 1);
          stepsTaken += 1;
          if (stepsTaken >= maxSteps) {
            break;
          }
          continue;
        }
        if (!pressA) {
          return lastAction ?? {
            result: { ok: true, changed: false, events: ["adjacent_and_facing"] },
            snapshotText: session.observeText(),
          };
        }
        lastAction = await session.press("a", 1);
        return lastAction;
      }
      if (stepsTaken >= maxSteps) {
        break;
      }
      continue;
    }

    const direction =
      findPathDirection(state, {
        allowTargetTile: targetIsWarp,
        goalMode: targetIsWarp ? "occupy" : "adjacent",
      }) ??
      chooseReducingDirection(state, { allowTargetTile: targetIsWarp });
    if (!direction) {
      return lastAction ?? buildApproachMacroFallbackResult(["no_reducing_direction"]);
    }
    lastAction = await session.move(direction, 1);
    stepsTaken += 1;
    if (targetIsWarp) {
      const nextMap = (await session.status?.())?.map ?? currentMap;
      if (currentMap && nextMap && nextMap !== currentMap) {
        return lastAction;
      }
      currentMap = nextMap;
    }
  }

  return (
    lastAction ??
    buildApproachMacroFallbackResult(["max_steps_reached_without_adjacent_or_interact"])
  );
};

export const executeMacroHandler = async (
  input: z.infer<typeof ExecuteMacroSchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  return withRequestIdentity(extra, async () => {
    const resolvedSessionId = resolveSessionId(extra);
    const session = await loadSession(resolvedSessionId, extra);
    if (isFieldMacro(input) || isDialogueMacro(input)) {
      const statusBefore = await session.status();
      const dialogueUnavailable = isDialogueMacro(input)
        ? resolveDialogueMacroUnavailableState("execute_macro", statusBefore)
        : undefined;
      if (dialogueUnavailable) {
        return buildUnavailableToolResponse("execute_macro", dialogueUnavailable, statusBefore);
      }
      const allowLowLevelNavigation = Boolean(input.actions?.length && hasFieldMovementAction(input.actions));
      const unavailable = resolveFieldToolUnavailableState("execute_macro", statusBefore, {
        allowBattleNavigation: false,
        allowDialogueNavigation: isDialogueMacro(input),
        allowPromptNavigation: isDialogueMacro(input) || allowLowLevelNavigation,
      });
      if (unavailable) {
        return buildUnavailableToolResponse("execute_macro", unavailable, statusBefore);
      }
    }
    invalidateObserveSnapshotCache(resolvedSessionId);
    const action = input.macro
      ? input.macro === "interact"
        ? await session.executeMacro(
            buildInteractMicroAdjustActions(input.max_presses ?? 5, input.settle_frames ?? 0),
            {
              delay_frames: input.delay_frames ?? 0,
              stop_on_event: input.stop_on_event ?? true,
            }
          )
        : input.macro === "approach_target"
          ? await runViewportApproachMacro(session, {
              targetToken: input.target_token ?? "D",
              maxSteps: input.max_steps,
              maxObserves: input.max_observes,
              maxTries: input.max_tries,
              pressA: input.press_a,
            })
        : await session.executeNamedMacro(input.macro, {
            maxPresses: input.max_presses,
            settleFrames: input.settle_frames,
          })
      : await session.executeMacro(input.actions ?? [], {
          delay_frames: input.delay_frames,
          stop_on_event: input.stop_on_event,
        });
    const playerContext = await session.playerContext();
    const statusSnapshot = await session.status();
    const recentEventsSnapshot = await session.recentEvents(5);
    const snapshotText = action.snapshotText ?? session.observeText();
    const frameId = session.getFrameCount();
    const computedAtMs = Date.now();
    const framePayload = input.include_tui_state ? session.observePayload() : undefined;
    await reportSnapshot(resolvedSessionId, session, snapshotText, "execute_macro");
    return {
      content: await buildActionContent(
        resolvedSessionId,
        action.result,
        snapshotText,
        playerContext,
        statusSnapshot,
        recentEventsSnapshot,
        frameId,
        computedAtMs,
        {
          format: input.format,
          detail: input.detail,
          include_snapshot_text: input.include_snapshot_text,
          include_tui_state: input.include_tui_state,
          frame_payload: framePayload,
        }
      ),
    };
  });
};

const INTERACT_MICRO_ADJUST_DIRECTIONS: Array<z.infer<typeof normalizedDirectionSchema>> = [
  "up",
  "down",
  "right",
  "left",
];

export const buildInteractMicroAdjustActions = (
  maxPresses: number,
  settleFrames: number
): Array<z.infer<typeof ExecuteMacroActionSchema>> => {
  const attempts = Math.max(1, Math.min(MAX_ADVANCE_FRAMES, Math.trunc(maxPresses)));
  const normalizedSettle = Math.max(0, Math.min(MAX_ADVANCE_FRAMES, Math.trunc(settleFrames)));
  const actions: Array<z.infer<typeof ExecuteMacroActionSchema>> = [];
  for (let i = 0; i < attempts; i += 1) {
    if (i > 0) {
      actions.push({
        type: "move",
        value: INTERACT_MICRO_ADJUST_DIRECTIONS[(i - 1) % INTERACT_MICRO_ADJUST_DIRECTIONS.length] ?? "up",
        times: 1,
        hold_frames: 1,
        delay_frames: 0,
      });
    }
    actions.push({
      type: "button",
      value: "a",
      times: 1,
      hold_frames: 1,
      delay_frames: normalizedSettle,
    });
  }
  return actions;
};

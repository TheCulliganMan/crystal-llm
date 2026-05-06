import { normalizeSessionId, SESSION_ID_REGEX } from "@/app/mcp/session-guards";
import { ensureArenaRunForSession, reportArenaEvent, reportArenaSnapshot } from "@/arena/runtime/telemetry";
import { getSettings } from "@pokecrystal/core/core/config";
import type { Json } from "@/lib/supabase/types";
import type {
  McpPlayerContext,
  McpRecentEventsSnapshot,
  McpStatusSnapshot,
} from "@/app/mcp/session";
import { resolveIdentityFromExtra } from "./identity";
import { runWithMcpIdentityContext } from "@pokecrystal/core/core/mcp-identity-context.server";

export type HeaderBag = Headers | Record<string, string | string[] | undefined> | undefined;

export type McpToolContent = {
  type: string;
  text?: string;
  data?: string;
  mimeType?: string;
};

export type McpToolResponse = {
  content: McpToolContent[];
  isError?: boolean;
};

export const MAX_ADVANCE_FRAMES = getSettings().mcpMaxActionsPerCall ?? 25;

export type McpToolExtra = {
  requestInfo?: { headers?: Record<string, string | string[] | undefined> };
  rawInput?: unknown;
};

export type McpToolHandler<T = unknown> = (
  input: T,
  extra?: McpToolExtra
) => Promise<McpToolResponse>;

export type ObserveSnapshotCacheEntry = {
  sessionId: string;
  frameCounter: number;
  cacheVersion: number;
  snapshotText: string;
  playerContext: McpPlayerContext;
  statusSnapshot: McpStatusSnapshot;
  recentEventsSnapshot: McpRecentEventsSnapshot;
  frameId: number;
  computedAtMs: number;
};

const SESSION_HEADER_KEYS = [
  "mcp-session-id",
  "x-mcp-session",
  "x-pokecrystal-session",
  "x-session-id",
];
const SESSION_MODE_HEADER_KEYS = [
  "x-pokecrystal-session-mode",
  "x-mcp-session-mode",
];
const TRAINING_STRING_LIMIT = 4_000;
const TRAINING_OBJECT_KEYS_LIMIT = 40;
const TRAINING_ARRAY_LIMIT = 20;
const TRAINING_DEPTH_LIMIT = 4;
const TRAINING_CONTENT_LIMIT = 8;
const SENSITIVE_KEY_PATTERN = /token|secret|password|authorization|cookie|api[-_]?key/i;
const observeSnapshotCache = new Map<string, ObserveSnapshotCacheEntry>();
const observeCacheVersions = new Map<string, number>();

type SanitizeState = {
  depth: number;
  seen: WeakSet<object>;
};

const truncateText = (value: string, maxLength = TRAINING_STRING_LIMIT): string =>
  value.length <= maxLength ? value : `${value.slice(0, maxLength)}...`;

const sanitizeTrainingValue = (value: unknown, state: SanitizeState): Json => {
  if (value === null || value === undefined) {
    return null;
  }
  if (typeof value === "string") {
    return truncateText(value);
  }
  if (typeof value === "boolean") {
    return value;
  }
  if (typeof value === "number") {
    return Number.isFinite(value) ? value : String(value);
  }
  if (typeof value === "bigint") {
    return value.toString();
  }
  if (Array.isArray(value)) {
    if (state.depth >= TRAINING_DEPTH_LIMIT) {
      return "[array_truncated]";
    }
    return value
      .slice(0, TRAINING_ARRAY_LIMIT)
      .map((entry) => sanitizeTrainingValue(entry, { depth: state.depth + 1, seen: state.seen }));
  }
  if (typeof value === "object") {
    if (state.depth >= TRAINING_DEPTH_LIMIT) {
      return "[object_truncated]";
    }
    if (state.seen.has(value as object)) {
      return "[circular]";
    }
    state.seen.add(value as object);
    const entries = Object.entries(value as Record<string, unknown>);
    const out: Record<string, Json> = {};
    for (const [rawKey, rawValue] of entries.slice(0, TRAINING_OBJECT_KEYS_LIMIT)) {
      const key = truncateText(rawKey, 80);
      if (SENSITIVE_KEY_PATTERN.test(rawKey)) {
        out[key] = "[redacted]";
        continue;
      }
      if (rawKey === "data" && typeof rawValue === "string") {
        out[key] = `[omitted:${rawValue.length} chars]`;
        continue;
      }
      out[key] = sanitizeTrainingValue(rawValue, { depth: state.depth + 1, seen: state.seen });
    }
    if (entries.length > TRAINING_OBJECT_KEYS_LIMIT) {
      out.__truncated_keys = entries.length - TRAINING_OBJECT_KEYS_LIMIT;
    }
    return out;
  }
  return String(value);
};

const sanitizeForTraining = (value: unknown): Json =>
  sanitizeTrainingValue(value, { depth: 0, seen: new WeakSet<object>() });

const sanitizeError = (error: unknown): Json => {
  if (error instanceof Error) {
    return {
      name: truncateText(error.name, 120),
      message: truncateText(error.message, 400),
      stack: error.stack ? truncateText(error.stack, 1_600) : null,
    };
  }
  return sanitizeForTraining(error);
};

const summarizeToolResponse = (response: McpToolResponse): Json => {
  const content = Array.isArray(response.content) ? response.content : [];
  const preview = content.slice(0, TRAINING_CONTENT_LIMIT).map((entry) => ({
    type: entry.type,
    text: typeof entry.text === "string" ? truncateText(entry.text) : undefined,
    mime_type: typeof entry.mimeType === "string" ? truncateText(entry.mimeType, 120) : undefined,
    data_bytes: typeof entry.data === "string" ? entry.data.length : undefined,
  }));
  const metadata: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(response as Record<string, unknown>)) {
    if (key === "content" || key === "isError") {
      continue;
    }
    metadata[key] = value;
  }
  return {
    is_error: Boolean(response.isError),
    content_count: content.length,
    content_preview: sanitizeForTraining(preview),
    metadata: Object.keys(metadata).length ? sanitizeForTraining(metadata) : null,
  };
};

const resolveTelemetrySessionId = (extra?: McpToolExtra): string | undefined => {
  const sessionId = sessionIdFromHeaders(extra?.requestInfo?.headers);
  if (!sessionId || !SESSION_ID_REGEX.test(sessionId)) {
    return undefined;
  }
  return normalizeSessionId(sessionId);
};

const safeReportArenaEvent = async (
  report: Parameters<typeof reportArenaEvent>[0]
): Promise<void> => {
  try {
    await reportArenaEvent(report);
  } catch (error) {
    void error;
  }
};

export const readHeaderValue = (headers: HeaderBag, name: string): string | undefined => {
  if (!headers) {
    return undefined;
  }
  if (headers instanceof Headers) {
    return headers.get(name) ?? headers.get(name.toLowerCase()) ?? undefined;
  }
  const lowerHeaders = headers as Record<string, string | string[] | undefined>;
  const direct = headers[name] ?? lowerHeaders[name.toLowerCase()];
  if (Array.isArray(direct)) {
    return direct[0];
  }
  return direct as string | undefined;
};

export const sessionIdFromHeaders = (headers: HeaderBag): string | undefined => {
  for (const key of SESSION_HEADER_KEYS) {
    const value = readHeaderValue(headers, key);
    if (value && value.trim()) {
      return value.trim();
    }
  }
  return undefined;
};

export const resolveSessionId = (extra?: McpToolExtra): string | undefined => {
  const headerSession = sessionIdFromHeaders(extra?.requestInfo?.headers);
  if (!headerSession) {
    return undefined;
  }
  if (!SESSION_ID_REGEX.test(headerSession)) {
    throw new Error("Invalid session id.");
  }
  return headerSession;
};

export const resolveSessionMode = (
  extra?: McpToolExtra
): "automation" | "interactive" | undefined => {
  for (const key of SESSION_MODE_HEADER_KEYS) {
    const value = readHeaderValue(extra?.requestInfo?.headers, key);
    if (!value) {
      continue;
    }
    const normalized = value.trim().toLowerCase();
    if (normalized === "interactive") {
      return "interactive";
    }
    if (normalized === "automation") {
      return "automation";
    }
  }
  return undefined;
};

export const loadSession = async (sessionId?: string, extra?: McpToolExtra) => {
  const { getMcpSession } = await import("@/app/mcp/session");
  const normalizedSessionId = normalizeSessionId(sessionId);
  const session = getMcpSession(normalizedSessionId);
  const sessionMode = resolveSessionMode(extra);
  const sessionWithMode = session as { setInteractiveMode?: (interactive: boolean) => void };
  sessionWithMode.setInteractiveMode?.(sessionMode === "interactive");
  await session.ensureReady();
  try {
    await ensureArenaRunForSession(normalizedSessionId);
  } catch (error) {
    void error;
  }
  return session;
};

export const reportSnapshot = async (
  sessionId: string | undefined,
  session: Awaited<ReturnType<typeof loadSession>>,
  text: string,
  action: string
) => {
  try {
    const normalizedSessionId = normalizeSessionId(sessionId);
    await reportArenaSnapshot({
      sessionId: normalizedSessionId,
      text,
      payload: session.observePayload() as Json,
      frame: session.getFrameCount(),
      action,
    });
  } catch (error) {
    void error;
  }
};

export const runToolWithTelemetry = async <T>(
  toolName: string,
  input: T,
  handler: McpToolHandler<T>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  const sessionId = resolveTelemetrySessionId(extra);
  const startedAtMs = Date.now();
  const inputPayload = extra?.rawInput !== undefined ? extra.rawInput : input;
  if (sessionId) {
    await safeReportArenaEvent({
      sessionId,
      label: "tool_call",
      action: `tool:${toolName}`,
      text: `${toolName} call`,
      payload: {
        tool: toolName,
        phase: "call",
        at: new Date(startedAtMs).toISOString(),
        input: sanitizeForTraining(inputPayload),
      },
    });
  }
  try {
    const response = await handler(input, extra);
    if (sessionId) {
      await safeReportArenaEvent({
        sessionId,
        label: response.isError ? "tool_result_error" : "tool_result",
        action: `tool:${toolName}`,
        text: `${toolName} result`,
        payload: {
          tool: toolName,
          phase: "result",
          duration_ms: Date.now() - startedAtMs,
          response: summarizeToolResponse(response),
        },
      });
    }
    return response;
  } catch (error) {
    if (sessionId) {
      await safeReportArenaEvent({
        sessionId,
        label: "tool_exception",
        action: `tool:${toolName}`,
        text: `${toolName} exception`,
        payload: {
          tool: toolName,
          phase: "exception",
          duration_ms: Date.now() - startedAtMs,
          error: sanitizeError(error),
        },
      });
    }
    throw error;
  }
};

export const getObserveSnapshotCache = (
  sessionId: string | undefined,
  frameCounter: number
): ObserveSnapshotCacheEntry | undefined => {
  const key = normalizeSessionId(sessionId);
  const entry = observeSnapshotCache.get(key);
  if (!entry) {
    return undefined;
  }
  const currentVersion = observeCacheVersions.get(key) ?? 0;
  if (entry.cacheVersion !== currentVersion || entry.frameCounter !== frameCounter) {
    return undefined;
  }
  return entry;
};

export const setObserveSnapshotCache = (
  sessionId: string | undefined,
  entry: Omit<ObserveSnapshotCacheEntry, "sessionId" | "cacheVersion">
): ObserveSnapshotCacheEntry => {
  const key = normalizeSessionId(sessionId);
  const cacheVersion = observeCacheVersions.get(key) ?? 0;
  const payload: ObserveSnapshotCacheEntry = {
    sessionId: key,
    cacheVersion,
    ...entry,
  };
  observeSnapshotCache.set(key, payload);
  return payload;
};

export const invalidateObserveSnapshotCache = (sessionId: string | undefined): void => {
  const key = normalizeSessionId(sessionId);
  observeSnapshotCache.delete(key);
  observeCacheVersions.set(key, (observeCacheVersions.get(key) ?? 0) + 1);
};

export const withRequestIdentity = async <T>(
  extra: McpToolExtra | undefined,
  fn: () => Promise<T>
): Promise<T> => {
  const identity = resolveIdentityFromExtra(extra);
  if (!identity) {
    return fn();
  }
  return runWithMcpIdentityContext(identity, fn);
};

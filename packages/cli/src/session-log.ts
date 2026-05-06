import fs from "node:fs";
import path from "node:path";
import type { CliOptions, ToolContent, ToolResult } from "./types";

export const DEFAULT_SESSION_LOG_DIR = "/tmp";
export const SESSION_LOG_MAX_LINES = 10_000;
const SESSION_LOG_TRIM_SLACK_LINES = 500;

const LOG_TEXT_LIMIT = 4_000;
const LOG_STACK_LIMIT = 2_000;
const LOG_ARRAY_LIMIT = 50;
const LOG_OBJECT_KEYS_LIMIT = 80;
const LOG_DEPTH_LIMIT = 6;
const SENSITIVE_KEY_PATTERN = /token|secret|password|authorization|cookie|api[-_]?key/i;

type JsonLogValue =
  | string
  | number
  | boolean
  | null
  | JsonLogValue[]
  | { [key: string]: JsonLogValue };

type LogSanitizeState = {
  depth: number;
  seen: WeakSet<object>;
};

type SessionLoggableOptions = Pick<
  CliOptions,
  | "command"
  | "transport"
  | "baseUrl"
  | "sessionId"
  | "sessionLogEnabled"
  | "sessionLogDir"
  | "sessionLogFile"
>;

export type SessionLogger = {
  enabled: boolean;
  filePath?: string;
  write: (event: string, payload?: Record<string, unknown>) => void;
};

const preparedLogFiles = new Set<string>();
const logLineCounts = new Map<string, number>();

const truncateText = (value: string, maxLength = LOG_TEXT_LIMIT): string =>
  value.length <= maxLength ? value : `${value.slice(0, maxLength)}...`;

const sanitizePathSegment = (value: string): string => {
  const sanitized = value
    .trim()
    .replace(/[^a-zA-Z0-9._-]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .slice(0, 80);
  return sanitized || "session";
};

export const resolveSessionLogFile = (
  options: Pick<SessionLoggableOptions, "command" | "sessionId" | "sessionLogDir" | "sessionLogFile">,
  _startedAtMs = Date.now(),
): string | undefined => {
  const explicitFile = options.sessionLogFile?.trim();
  if (explicitFile) {
    return path.resolve(explicitFile);
  }
  const logDir = options.sessionLogDir?.trim();
  if (!logDir) {
    return undefined;
  }
  const session = sanitizePathSegment(options.sessionId);
  return path.join(path.resolve(logDir), `pokecrystal-${session}.jsonl`);
};

export const withResolvedSessionLogFile = <T extends SessionLoggableOptions>(
  options: T,
  startedAtMs = Date.now(),
): T => {
  if (options.sessionLogEnabled === false) {
    return options;
  }
  const sessionLogDir = options.sessionLogDir?.trim() || DEFAULT_SESSION_LOG_DIR;
  const sessionLogFile = resolveSessionLogFile(
    {
      command: options.command,
      sessionId: options.sessionId,
      sessionLogDir,
      sessionLogFile: options.sessionLogFile,
    },
    startedAtMs,
  );
  return {
    ...options,
    sessionLogEnabled: true,
    sessionLogDir,
    sessionLogFile,
  };
};

const sanitizeForLog = (value: unknown, state: LogSanitizeState): JsonLogValue => {
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
  if (value instanceof Error) {
    return {
      name: truncateText(value.name, 120),
      message: truncateText(value.message, 800),
      stack: value.stack ? truncateText(value.stack, LOG_STACK_LIMIT) : null,
    };
  }
  if (Array.isArray(value)) {
    if (state.depth >= LOG_DEPTH_LIMIT) {
      return "[array_truncated]";
    }
    const items = value
      .slice(0, LOG_ARRAY_LIMIT)
      .map((entry) => sanitizeForLog(entry, { depth: state.depth + 1, seen: state.seen }));
    if (value.length > LOG_ARRAY_LIMIT) {
      items.push(`[truncated:${value.length - LOG_ARRAY_LIMIT}]`);
    }
    return items;
  }
  if (typeof value === "object") {
    if (state.depth >= LOG_DEPTH_LIMIT) {
      return "[object_truncated]";
    }
    if (state.seen.has(value)) {
      return "[circular]";
    }
    state.seen.add(value);
    const out: Record<string, JsonLogValue> = {};
    const entries = Object.entries(value as Record<string, unknown>);
    for (const [key, rawValue] of entries.slice(0, LOG_OBJECT_KEYS_LIMIT)) {
      const safeKey = truncateText(key, 120);
      if (SENSITIVE_KEY_PATTERN.test(key)) {
        out[safeKey] = "[redacted]";
        continue;
      }
      if (key === "data" && typeof rawValue === "string") {
        out[safeKey] = `[omitted:${rawValue.length} chars]`;
        continue;
      }
      out[safeKey] = sanitizeForLog(rawValue, { depth: state.depth + 1, seen: state.seen });
    }
    if (entries.length > LOG_OBJECT_KEYS_LIMIT) {
      out.__truncated_keys = entries.length - LOG_OBJECT_KEYS_LIMIT;
    }
    return out;
  }
  return String(value);
};

export const sanitizeSessionLogValue = (value: unknown): JsonLogValue =>
  sanitizeForLog(value, { depth: 0, seen: new WeakSet<object>() });

const prepareLogFile = (filePath: string): void => {
  if (preparedLogFiles.has(filePath)) {
    return;
  }
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  if (fs.existsSync(filePath)) {
    const raw = fs.readFileSync(filePath, "utf8");
    const lines = raw.split(/\r?\n/);
    if (lines[lines.length - 1] === "") {
      lines.pop();
    }
    logLineCounts.set(filePath, lines.length);
  } else {
    logLineCounts.set(filePath, 0);
  }
  preparedLogFiles.add(filePath);
};

const trimLogFileToLineLimit = (filePath: string): void => {
  const raw = fs.readFileSync(filePath, "utf8");
  const lines = raw.split(/\r?\n/);
  if (lines[lines.length - 1] === "") {
    lines.pop();
  }
  if (lines.length <= SESSION_LOG_MAX_LINES) {
    return;
  }
  fs.writeFileSync(
    filePath,
    `${lines.slice(-SESSION_LOG_MAX_LINES).join("\n")}\n`,
    "utf8",
  );
};

export const createSessionLogger = (options: Pick<SessionLoggableOptions, "sessionId" | "sessionLogEnabled" | "sessionLogFile">): SessionLogger => {
  const filePath = options.sessionLogEnabled === false ? undefined : options.sessionLogFile?.trim();
  if (!filePath) {
    return {
      enabled: false,
      write: () => undefined,
    };
  }
  const resolvedFilePath = path.resolve(filePath);
  return {
    enabled: true,
    filePath: resolvedFilePath,
    write: (event, payload = {}) => {
      prepareLogFile(resolvedFilePath);
      const timestampMs = Date.now();
      const sanitizedPayload = sanitizeSessionLogValue(payload) as Record<string, JsonLogValue>;
      const entry = {
        timestamp_ms: timestampMs,
        timestamp_iso: new Date(timestampMs).toISOString(),
        session_id: options.sessionId,
        event,
        ...sanitizedPayload,
      };
      fs.appendFileSync(resolvedFilePath, `${JSON.stringify(entry)}\n`, "utf8");
      const lineCount = (logLineCounts.get(resolvedFilePath) ?? 0) + 1;
      logLineCounts.set(resolvedFilePath, lineCount);
      if (lineCount > SESSION_LOG_MAX_LINES + SESSION_LOG_TRIM_SLACK_LINES) {
        trimLogFileToLineLimit(resolvedFilePath);
        logLineCounts.set(resolvedFilePath, SESSION_LOG_MAX_LINES);
      }
    },
  };
};

const summarizeContent = (
  entry: ToolContent,
  maxTextLength: number,
): Record<string, JsonLogValue> => {
  if (entry.type === "text") {
    const text = entry.text ?? "";
    return {
      type: "text",
      text: truncateText(text, maxTextLength),
      text_length: text.length,
    };
  }
  if (entry.type === "image" || entry.type === "audio") {
    return {
      type: entry.type,
      mime_type: entry.mimeType ?? null,
      data_length: entry.data?.length ?? 0,
    };
  }
  return { type: entry.type };
};

export const summarizeToolResult = (
  result: ToolResult | undefined,
  options: { maxTextLength?: number } = {},
): Record<string, JsonLogValue> => {
  const content = result?.content ?? [];
  const maxTextLength = options.maxTextLength ?? LOG_TEXT_LIMIT;
  return {
    is_error: Boolean(result?.isError),
    content_count: content.length,
    content: content.map((entry) => summarizeContent(entry, maxTextLength)),
  };
};

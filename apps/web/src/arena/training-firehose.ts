import { SESSION_ID_REGEX } from "@/app/mcp/session-guards";
import type { Json, Tables } from "@/lib/supabase/types";

export const FIREHOSE_LIMIT_DEFAULT = 500;
export const FIREHOSE_LIMIT_MAX = 2000;

export type FirehoseQuery = {
  afterId: number;
  limit: number;
  runId?: string;
  label?: string;
};

export type FirehoseRow = Pick<
  Tables<"arena_run_events">,
  "id" | "run_id" | "frame" | "label" | "payload" | "created_at"
>;

export type FirehoseRecord = FirehoseRow & {
  session_id: string | null;
};

const parseIntParam = (
  value: string | null,
  fallback: number,
  field: string,
  options: { min?: number; max?: number } = {}
): number => {
  if (value === null) {
    return fallback;
  }
  const trimmed = value.trim();
  if (!trimmed) {
    return fallback;
  }
  if (!/^\d+$/.test(trimmed)) {
    throw new Error(`Invalid ${field}.`);
  }
  const parsed = Number(trimmed);
  if (!Number.isSafeInteger(parsed)) {
    throw new Error(`Invalid ${field}.`);
  }
  if (options.min !== undefined && parsed < options.min) {
    throw new Error(`Invalid ${field}.`);
  }
  if (options.max !== undefined && parsed > options.max) {
    throw new Error(`Invalid ${field}.`);
  }
  return parsed;
};

const parseStringParam = (value: string | null): string | undefined => {
  if (value === null) {
    return undefined;
  }
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
};

const pickParam = (params: URLSearchParams, keys: string[]): string | null => {
  for (const key of keys) {
    const value = params.get(key);
    if (value !== null) {
      return value;
    }
  }
  return null;
};

const extractSessionId = (payload: Json | null): string | null => {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    return null;
  }
  const candidate = (payload as Record<string, Json>).session_id;
  if (typeof candidate !== "string") {
    return null;
  }
  const trimmed = candidate.trim();
  if (!trimmed || !SESSION_ID_REGEX.test(trimmed)) {
    return null;
  }
  return trimmed;
};

export const parseFirehoseQuery = (url: URL): FirehoseQuery => {
  const params = url.searchParams;
  const afterParam = pickParam(params, ["after_id", "cursor", "since_id"]);
  const afterId = parseIntParam(afterParam, 0, "after_id", { min: 0 });
  const limit = parseIntParam(params.get("limit"), FIREHOSE_LIMIT_DEFAULT, "limit", {
    min: 1,
    max: FIREHOSE_LIMIT_MAX,
  });
  const runId = parseStringParam(params.get("run_id"));
  const label = parseStringParam(params.get("label"));
  return { afterId, limit, runId, label };
};

export const buildFirehoseRecord = (row: FirehoseRow): FirehoseRecord => ({
  ...row,
  session_id: extractSessionId(row.payload),
});

export const encodeFirehoseRecords = (records: FirehoseRecord[]): string => {
  if (!records.length) {
    return "";
  }
  return `${records.map((record) => JSON.stringify(record)).join("\n")}\n`;
};

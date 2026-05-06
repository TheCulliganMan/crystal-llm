import type { ArenaRun } from "@/arena/types";
import { extractSessionIdFromRun } from "@/arena/utils";
import { createSupabaseServiceRoleClient } from "@/lib/supabase/server";

export const MAX_WATCH_SESSION_LIMIT = 27;
export const DEFAULT_WATCH_SESSION_LIMIT = MAX_WATCH_SESSION_LIMIT;

const ACTIVE_RUN_STATUSES = ["queued", "running"] as const;
const ARENA_RUN_SELECT_WITH_AGENT = "*, agent:arena_agents(*)";

const isTableMissing = (message: string) =>
  message.includes('relation "public.arena_runs" does not exist') ||
  message.includes("does not exist");

const prioritizeWatchRuns = (runs: ArenaRun[]): ArenaRun[] => {
  const seen = new Set<string>();
  const merged: ArenaRun[] = [];
  for (const run of runs) {
    const key = extractSessionIdFromRun(run) ?? run.id;
    if (!seen.has(key)) {
      seen.add(key);
      merged.push(run);
    }
  }
  return merged;
};

const parseTimestamp = (value: string | null | undefined): number => {
  if (!value) {
    return 0;
  }
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : 0;
};

const getRunSortTimestamp = (run: ArenaRun): number => {
  return Math.max(
    parseTimestamp(run.updated_at),
    parseTimestamp(run.started_at),
    parseTimestamp(run.created_at)
  );
};

type ArenaClient = NonNullable<ReturnType<typeof createSupabaseServiceRoleClient>>;

const buildQuery = (client: ArenaClient, includeAgent: boolean) => {
  const query = includeAgent
    ? client.from("arena_runs").select(ARENA_RUN_SELECT_WITH_AGENT)
    : client.from("arena_runs").select("*");
  return query.in("status", [...ACTIVE_RUN_STATUSES]).order("created_at", { ascending: false });
};

const fetchRuns = async (
  client: ArenaClient,
  includeAgent: boolean,
  limit: number
) => {
  const query = buildQuery(client, includeAgent);
  return query.limit(limit);
};

export const clampWatchSessionLimit = (limit: number): number => {
  if (!Number.isFinite(limit)) {
    return DEFAULT_WATCH_SESSION_LIMIT;
  }
  return Math.min(MAX_WATCH_SESSION_LIMIT, Math.max(1, Math.floor(limit)));
};

export type WatchRunResolution =
  | { ok: true; runs: ArenaRun[]; warning?: string }
  | { ok: false; runs: ArenaRun[]; error: string };

export const resolveWatchRuns = async (limitInput: number): Promise<WatchRunResolution> => {
  const limit = clampWatchSessionLimit(limitInput);
  try {
    const supabase = createSupabaseServiceRoleClient();
    if (!supabase) {
      return {
        ok: true,
        runs: [],
      };
    }

    const primaryWithAgentResult = await fetchRuns(supabase, true, limit);
    let primaryRuns = (primaryWithAgentResult.data ?? []) as ArenaRun[];
    let primaryError = primaryWithAgentResult.error;

    if (primaryError?.code === "PGRST200") {
      const primaryWithoutAgentResult = await fetchRuns(supabase, false, limit);
      primaryRuns = (primaryWithoutAgentResult.data ?? []) as ArenaRun[];
      primaryError = primaryWithoutAgentResult.error;
    }
    if (primaryError) {
      const objectError = primaryError as { message?: string; details?: string };
      const message = objectError.message ?? objectError.details ?? "Failed to fetch active runs.";
      if (isTableMissing(message)) {
        return {
          ok: true,
          runs: [],
          warning: message,
        };
      }
      return { ok: false, error: message, runs: [] };
    }

    return {
      ok: true,
      runs: prioritizeWatchRuns(primaryRuns)
        .sort((left, right) => getRunSortTimestamp(right) - getRunSortTimestamp(left))
        .slice(0, limit),
    };
  } catch (error) {
    const objectError = error as { message?: string; details?: string } | undefined;
    const message =
      error instanceof Error ? error.message : objectError?.message ?? objectError?.details ?? String(error);
    return { ok: false, error: message, runs: [] };
  }
};

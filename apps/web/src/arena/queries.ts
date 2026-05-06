import type { SupabaseClient } from "@supabase/supabase-js";
import { createSupabaseServerClient } from "@/lib/supabase/server";
import type { Database } from "@/lib/supabase/types";
import type {
  ArenaAgent,
  ArenaAgentRow,
  ArenaLeaderboardRow,
  ArenaProfile,
  ArenaRun,
  ArenaRunRow,
} from "./types";

type ArenaRunWithAgent = ArenaRunRow & { agent: ArenaAgentRow };

const fetchRunsWithAgentFallback = async (
  client: SupabaseClient<Database>,
  buildQuery: (selectClause: string) => Promise<any>
): Promise<ArenaRun[]> => {
  const withAgent = (await buildQuery("*, agent:arena_agents(*)")) as {
    data: ArenaRun[] | null;
    error: { code?: string } | null;
  };

  if (!withAgent.error) {
    return (withAgent.data ?? []) as ArenaRun[];
  }
  if (withAgent.error.code !== "PGRST200") {
    throw withAgent.error;
  }

  const fallback = (await buildQuery("*")) as {
    data: ArenaRun[] | null;
    error: unknown;
  };
  if (fallback.error) {
    throw fallback.error;
  }
  return (fallback.data ?? []) as ArenaRun[];
};

const withClient = <T>(
  fn: (client: SupabaseClient<Database>) => Promise<T>,
  fallback: T
): Promise<T> => {
  const supabase = createSupabaseServerClient();
  if (!supabase) {
    return Promise.resolve(fallback);
  }
  return fn(supabase);
};

export const getCurrentUser = async () => {
  const supabase = createSupabaseServerClient();
  if (!supabase) {
    return null;
  }
  const {
    data: { user },
  } = await supabase.auth.getUser();
  return user;
};

export const fetchProfile = async (userId: string): Promise<ArenaProfile | null> =>
  withClient(async (client) => {
    const { data } = await client
      .from("arena_profiles")
      .select("*")
      .eq("id", userId)
      .maybeSingle();
    return (data ?? null) as ArenaProfile | null;
  }, null);

export const fetchOwnedAgents = async (userId: string): Promise<ArenaAgentRow[]> =>
  withClient(async (client) => {
    const { data, error } = await client
      .from("arena_agents")
      .select("*")
      .eq("owner_id", userId)
      .order("created_at", { ascending: false });
    if (error) {
      throw error;
    }
    return (data ?? []) as ArenaAgentRow[];
  }, []);

export const fetchRecentRuns = async (limit = 10): Promise<ArenaRun[]> =>
  withClient(async (client) => {
    return fetchRunsWithAgentFallback(client, async (selectClause) =>
      await client
        .from("arena_runs")
        .select(selectClause)
        .order("created_at", { ascending: false })
        .limit(limit)
    );
  }, []);

export const fetchActiveRuns = async (limit = 12): Promise<ArenaRun[]> =>
  withClient(async (client) => {
    return fetchRunsWithAgentFallback(client, async (selectClause) =>
      await client
        .from("arena_runs")
        .select(selectClause)
        .in("status", ["queued", "running"])
        .order("created_at", { ascending: false })
        .limit(limit)
    );
  }, []);

export const fetchRunById = async (runId: string): Promise<ArenaRun | null> =>
  withClient(async (client) => {
    const { data, error } = await client
      .from("arena_runs")
      .select("*, agent:arena_agents(*)")
      .eq("id", runId)
      .maybeSingle();
    if (error) {
      throw error;
    }
    if (!data) return null;
    return data as ArenaRun;
  }, null);

export const fetchLeaderboard = async (
  limit = 15
): Promise<{ leaderboard: ArenaLeaderboardRow[]; agents: ArenaAgentRow[] }> =>
  withClient(async (client) => {
    const { data: leaderboard, error } = await client
      .from("arena_leaderboard")
      .select("*")
      .order("max_badges", { ascending: false })
      .order("best_duration", { ascending: true })
      .limit(limit);
    if (error) {
      throw error;
    }
    const typedLeaderboard = (leaderboard ?? []) as ArenaLeaderboardRow[];
    const ids = typedLeaderboard.map((row) => row.agent_id).filter(Boolean) as string[];
    const { data: agents } = ids.length
      ? await client.from("arena_agents").select("*").in("id", ids)
      : { data: [] };
    return {
      leaderboard: typedLeaderboard,
      agents: (agents ?? []) as ArenaAgentRow[],
    };
  }, { leaderboard: [], agents: [] });

export const fetchPublicAgents = async (limit = 12): Promise<ArenaAgentRow[]> =>
  withClient(async (client) => {
    const { data, error } = await client
      .from("arena_agents")
      .select("*")
      .eq("visibility", "public")
      .order("created_at", { ascending: false })
      .limit(limit);
    if (error) {
      throw error;
    }
    return (data ?? []) as ArenaAgentRow[];
  }, []);

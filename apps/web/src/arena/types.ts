import type { Database, Tables } from "@/lib/supabase/types";

export type ArenaProfile = Tables<"arena_profiles">;
export type ArenaAgentRow = Tables<"arena_agents">;
export type ArenaRunRow = Tables<"arena_runs">;
export type ArenaRunEventRow = Tables<"arena_run_events">;
export type ArenaLeaderboardRow = Database["public"]["Views"]["arena_leaderboard"]["Row"];
export type KrabbyClawArenaRatingRow = Tables<"krabbyclaw_arena_ratings">;
export type KrabbyClawArenaMatchRow = Tables<"krabbyclaw_arena_matches">;
export type KrabbyClawArenaLeaderboardRow = Database["public"]["Views"]["krabbyclaw_arena_leaderboard"]["Row"];

export type ArenaAgent = ArenaAgentRow & {
  owner_handle?: string;
  latest_run?: ArenaRunRow | null;
  leaderboard?: ArenaLeaderboardRow | null;
};

export type ArenaRun = ArenaRunRow & {
  agent?: ArenaAgentRow;
};

export const AGENT_RUNTIME_LABELS: Record<string, string> = {
  "mcp-http": "Arena Live",
  "mcp-stdio": "Arena Live",
  "langgraph-worker": "Arena Worker",
};

export const RUN_STATUS_LABELS: Record<ArenaRunRow["status"], string> = {
  queued: "Queued",
  running: "Running",
  completed: "Completed",
  failed: "Failed",
  cancelled: "Cancelled",
};

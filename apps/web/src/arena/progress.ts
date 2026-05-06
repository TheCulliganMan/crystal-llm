import type { ArenaRun } from "@/arena/types";
import { extractCommandCount, extractStepCount } from "@/arena/run-metrics";

export type ArenaProgressRow = {
  agentId: string;
  agentName: string;
  status: string;
  badges: number | null;
  frames: number | null;
  steps: number | null;
  instructions: number | null;
  updatedAt: string | null;
};

export const buildLatestProgressRows = (
  recentRunsFeed: ArenaRun[],
  activeRuns: ArenaRun[]
): ArenaProgressRow[] => {
  const latestProgressByAgent = new Map<string, ArenaRun>();

  for (const run of recentRunsFeed) {
    if (!run.agent_id || latestProgressByAgent.has(run.agent_id)) {
      continue;
    }
    latestProgressByAgent.set(run.agent_id, run);
  }

  for (const run of activeRuns) {
    if (!run.agent_id) {
      continue;
    }
    latestProgressByAgent.set(run.agent_id, run);
  }

  return Array.from(latestProgressByAgent.values())
    .map((run) => ({
      agentId: run.agent_id,
      agentName: run.agent?.name ?? "Unknown agent",
      status: run.status,
      badges: run.badge_count ?? null,
      frames: run.frame_count ?? null,
      steps: extractStepCount(run.metrics),
      instructions: extractCommandCount(run.metrics),
      updatedAt: run.updated_at ?? run.created_at ?? null,
    }))
    .sort((left, right) => {
      const leftTime = left.updatedAt ? Date.parse(left.updatedAt) : 0;
      const rightTime = right.updatedAt ? Date.parse(right.updatedAt) : 0;
      return rightTime - leftTime;
    });
};

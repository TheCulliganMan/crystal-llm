import type { SupabaseClient } from "@supabase/supabase-js";
import { createSupabaseServiceRoleClient } from "@/lib/supabase/server";
import type { Database, Tables, Json } from "@/lib/supabase/types";

type RunRow = Tables<"arena_runs">;
type ArenaSnapshotPayload = Json | null;
type SnapshotResponse = { text?: string; payload?: ArenaSnapshotPayload };

const siteUrl =
  process.env.NEXT_PUBLIC_SITE_URL ??
  (process.env.VERCEL_URL ? `https://${process.env.VERCEL_URL}` : undefined);

const SNAPSHOT_URL =
  process.env.ARENA_SNAPSHOT_URL ??
  (siteUrl ? `${siteUrl}/api/arena/snapshot` : "http://localhost:3000/api/arena/snapshot");

export class ArenaOrchestrator {
  constructor(private readonly maxConcurrent: number = 2) {}

  async poll(): Promise<void> {
    const supabase = createSupabaseServiceRoleClient();
    if (!supabase) {
      throw new Error("Supabase service role is not configured.");
    }
    const { data: queued, error } = await supabase
      .from("arena_runs")
      .select("*")
      .eq("status", "queued")
      .order("created_at", { ascending: true })
      .limit(this.maxConcurrent);
    if (error) {
      throw error;
    }
    for (const run of queued ?? []) {
      await this.processRun(supabase, run as RunRow);
    }
  }

  private async processRun(client: SupabaseClient<Database>, run: RunRow) {
    const startedAt = new Date().toISOString();
    await client.from("arena_runs").update({ status: "running", started_at: startedAt }).eq("id", run.id);

    const snapshot = await this.fetchSnapshot();
    const payload = snapshot?.payload;
    if (payload && typeof payload === "object") {
      await client.from("arena_run_events").insert({
        run_id: run.id,
        label: "mcp_snapshot",
        payload,
      });
    }

    const finishedAt = new Date().toISOString();
    await client
      .from("arena_runs")
      .update({
        status: "completed",
        finished_at: finishedAt,
        badge_count: run.badge_count ?? 0,
        frame_count: this.extractFrameFromSnapshot(snapshot?.payload) ?? run.frame_count,
        metrics: {
          ...(run.metrics as Record<string, unknown>),
          last_snapshot_text: snapshot?.text,
        },
      })
      .eq("id", run.id);
  }

  private async fetchSnapshot(): Promise<SnapshotResponse | null> {
    try {
      const token = process.env.POKECRYSTAL_ARENA_SNAPSHOT_TOKEN;
      const headers = token ? { Authorization: `Bearer ${token}` } : undefined;
      const response = await fetch(SNAPSHOT_URL, { cache: "no-store", headers });
      if (!response.ok) {
        throw new Error(`Snapshot request failed: ${response.status}`);
      }
      const json = (await response.json()) as SnapshotResponse;
      return json;
    } catch (error) {
      console.error("[arena-orchestrator] snapshot fetch failed", error);
      return null;
    }
  }

  private extractFrameFromSnapshot(payload: SnapshotResponse["payload"]): number | undefined {
    if (!payload || typeof payload !== "object") {
      return undefined;
    }
    const frameCandidate = (payload as { frame?: unknown }).frame;
    if (typeof frameCandidate === "number" && Number.isFinite(frameCandidate)) {
      return frameCandidate;
    }
    return undefined;
  }
}

export const runOrchestratorOnce = async () => {
  const orchestrator = new ArenaOrchestrator();
  await orchestrator.poll();
};

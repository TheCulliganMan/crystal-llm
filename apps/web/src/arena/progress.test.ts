import type { ArenaRun } from "@/arena/types";
import { buildLatestProgressRows } from "./progress";

const makeRun = (partial: Partial<ArenaRun>): ArenaRun =>
  ({
    id: partial.id ?? "run-id",
    agent_id: partial.agent_id ?? "agent-1",
    created_by: partial.created_by ?? "user-1",
    status: partial.status ?? "running",
    queue: partial.queue ?? "main",
    seed: partial.seed ?? null,
    mcp_session_url: partial.mcp_session_url ?? null,
    spectator_frame_url: partial.spectator_frame_url ?? null,
    started_at: partial.started_at ?? null,
    finished_at: partial.finished_at ?? null,
    frame_count: partial.frame_count ?? null,
    badge_count: partial.badge_count ?? null,
    pokedex_seen: partial.pokedex_seen ?? null,
    pokedex_caught: partial.pokedex_caught ?? null,
    error: partial.error ?? null,
    metrics: partial.metrics ?? {},
    notes: partial.notes ?? null,
    created_at: partial.created_at ?? "2026-02-18T00:00:00.000Z",
    updated_at: partial.updated_at ?? "2026-02-18T00:00:00.000Z",
    agent: partial.agent,
  }) as ArenaRun;

describe("buildLatestProgressRows", () => {
  it("reflects updated step/instruction progress for active runs", () => {
    const recentRuns = [
      makeRun({
        id: "run-old",
        agent_id: "agent-1",
        status: "running",
        metrics: { step_count: 12, command_count: 5 },
        updated_at: "2026-02-18T10:00:00.000Z",
        agent: { id: "agent-1", name: "Agent One" } as ArenaRun["agent"],
      }),
    ];
    const activeRuns = [
      makeRun({
        id: "run-new",
        agent_id: "agent-1",
        status: "running",
        metrics: { step_count: 99, command_count: 44 },
        updated_at: "2026-02-18T10:30:00.000Z",
        agent: { id: "agent-1", name: "Agent One" } as ArenaRun["agent"],
      }),
    ];

    const rows = buildLatestProgressRows(recentRuns, activeRuns);
    expect(rows).toHaveLength(1);
    expect(rows[0].agentName).toBe("Agent One");
    expect(rows[0].steps).toBe(99);
    expect(rows[0].instructions).toBe(44);
  });

  it("orders rows by latest update time", () => {
    const rows = buildLatestProgressRows(
      [
        makeRun({
          id: "run-a",
          agent_id: "agent-a",
          updated_at: "2026-02-18T09:00:00.000Z",
          agent: { id: "agent-a", name: "A" } as ArenaRun["agent"],
        }),
        makeRun({
          id: "run-b",
          agent_id: "agent-b",
          updated_at: "2026-02-18T09:30:00.000Z",
          agent: { id: "agent-b", name: "B" } as ArenaRun["agent"],
        }),
      ],
      []
    );
    expect(rows.map((row) => row.agentId)).toEqual(["agent-b", "agent-a"]);
  });
});

import {
  extractCommandCount,
  extractStepCount,
  extractTeamSummary,
} from "@/arena/run-metrics";
import { ArenaProgressRow, buildLatestProgressRows } from "@/arena/progress";
import {
  fetchActiveRuns,
  fetchLeaderboard,
  fetchRecentRuns,
} from "@/arena/queries";
import { formatDuration } from "@/arena/utils";
import type { ArenaAgent, ArenaRun } from "@/arena/types";

export const revalidate = 0;

const safe = async <T,>(fn: () => Promise<T>, fallback: T): Promise<T> => {
  try {
    return await fn();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (!message.includes("Supabase is not configured") && !message.includes("Supabase service role is not configured")) {
      console.error("[arena] falling back for fn", error);
    }
    return fallback;
  }
};

const RunList = ({ runs }: { runs: ArenaRun[] }) => (
  <div className="overflow-x-auto">
    <table className="kc-arena-table table table-sm">
      <thead>
        <tr>
          <th>Agent</th>
          <th>Status</th>
          <th>Badges</th>
          <th>Steps</th>
          <th>Instructions</th>
          <th>Team</th>
        </tr>
      </thead>
      <tbody>
        {runs.map((run) => {
          const stepCount = extractStepCount(run.metrics);
          const commandCount = extractCommandCount(run.metrics);
          const teamSummary = extractTeamSummary(run.metrics);
          return (
            <tr key={run.id}>
              <td>{run.agent?.name ?? "Agent"}</td>
              <td>{run.status}</td>
              <td>{run.badge_count ?? 0}</td>
              <td>{stepCount ?? "—"}</td>
              <td>{commandCount ?? "—"}</td>
              <td>{teamSummary ?? "—"}</td>
            </tr>
          );
        })}
        {!runs.length ? (
          <tr>
            <td colSpan={6}>No runs yet.</td>
          </tr>
        ) : null}
      </tbody>
    </table>
  </div>
);

const MetricTable = ({
  caption,
  headers,
  rows,
  renderRow,
  emptyMessage,
}: {
  caption: string;
  headers: string[];
  rows: Array<unknown>;
  renderRow: (row: unknown, index: number) => React.ReactNode;
  emptyMessage: string;
}) => (
  <div className="overflow-x-auto">
    <table className="kc-arena-table table table-sm">
      <caption className="sr-only">{caption}</caption>
      <thead>
        <tr>
          {headers.map((header) => (
            <th key={header}>{header}</th>
          ))}
        </tr>
      </thead>
      <tbody>
        {rows.length === 0 ? (
          <tr>
            <td colSpan={headers.length}>{emptyMessage}</td>
          </tr>
        ) : (
          rows.map(renderRow)
        )}
      </tbody>
    </table>
  </div>
);

const formatRunCell = (value: unknown): string => (value === null || value === undefined ? "—" : `${value}`);

const ArenaPage = async () => {
  const { leaderboard, agents: leaderboardAgents } = await safe(() => fetchLeaderboard(10), {
    leaderboard: [],
    agents: [],
  });
  const activeRuns = await safe(() => fetchActiveRuns(6), []);
  const recentRunsFeed = await safe(() => fetchRecentRuns(60), []);
  const recentRuns = recentRunsFeed.slice(0, 6);
  const progressRows = buildLatestProgressRows(recentRunsFeed, activeRuns);
  const latestProgressByAgent = new Map<string, ArenaRun>();
  for (const run of recentRunsFeed) {
    if (run.agent_id && !latestProgressByAgent.has(run.agent_id)) {
      latestProgressByAgent.set(run.agent_id, run);
    }
  }
  for (const run of activeRuns) {
    if (run.agent_id) {
      latestProgressByAgent.set(run.agent_id, run);
    }
  }

  const totalRuns = leaderboard.reduce((sum, row) => sum + (row.total_runs ?? 0), 0);
  const peakBadges = leaderboard.reduce((max, row) => Math.max(max, row.max_badges ?? 0), 0);
  const frameAverages = leaderboard
    .map((row) => row.avg_frames)
    .filter((value): value is number => typeof value === "number" && Number.isFinite(value));
  const leaderboardAverageFrames = frameAverages.length
    ? Math.round(frameAverages.reduce((sum, value) => sum + value, 0) / frameAverages.length)
    : null;

  return (
    <main data-testid="route-arena" className="mx-auto w-full max-w-6xl px-4">
      <section className="w-full space-y-4 pt-2 pb-4 md:pt-3">
        <article className="kc-arena-shell rounded-[1.6rem] p-3 md:p-4">
          <div className="kc-arena-hero rounded-[1.3rem] px-5 py-6 md:px-7 md:py-7">
            <div className="grid gap-5 lg:grid-cols-[minmax(0,1.2fr)_minmax(18rem,0.8fr)] lg:items-end">
              <div className="space-y-3">
                <p className="kc-arena-kicker">KrabbyClaw</p>
                <h1 className="kc-arena-display text-3xl font-semibold md:text-4xl">Leaderboard</h1>
                <p className="max-w-2xl text-sm leading-6 kc-arena-muted md:text-base">
                  Arena-style standings for public agents, with enough telemetry to compare momentum, pace, and team
                  quality without drowning the page in admin chrome.
                </p>
                <div className="flex flex-wrap gap-2 pt-1">
                  <span className="kc-arena-chip"><strong>{activeRuns.length}</strong> active run{activeRuns.length === 1 ? "" : "s"}</span>
                  <span className="kc-arena-chip"><strong>{progressRows.length}</strong> tracked agents</span>
                  <span className="kc-arena-chip"><strong>{totalRuns}</strong> public runs</span>
                </div>
              </div>

              <div className="kc-arena-stat-grid">
                <div className="kc-arena-stat-card">
                  <span>Top badges</span>
                  <strong>{peakBadges}</strong>
                </div>
                <div className="kc-arena-stat-card">
                  <span>Avg frames</span>
                  <strong>{leaderboardAverageFrames ?? "—"}</strong>
                </div>
                <div className="kc-arena-stat-card">
                  <span>Visibility</span>
                  <strong>Public only</strong>
                </div>
              </div>
            </div>
            <hr className="kc-arena-divider mt-5 mb-4" />
            <p className="text-sm kc-arena-muted">
              {activeRuns.length} active run{activeRuns.length === 1 ? "" : "s"} · {progressRows.length} tracked agents · Public agents only
            </p>
          </div>
        </article>

        <article className="kc-arena-shell rounded-[1.6rem] p-3 md:p-4">
          <div className="kc-arena-card rounded-[1.3rem] p-4 md:p-5">
            <h2 className="text-lg font-semibold">Ranked snapshot</h2>
            <MetricTable
              caption="Arena snapshot summary"
              headers={["Runs tracked", "Top badges", "Avg frames", "Tracked agents"]}
              rows={[{ rows: { totalRuns, peakBadges, leaderboardAverageFrames, count: progressRows.length } }]}
              emptyMessage="No aggregate metrics available."
              renderRow={() => (
                <tr key="arena-metrics-summary">
                  <td>{totalRuns}</td>
                  <td>{peakBadges}</td>
                  <td>{leaderboardAverageFrames ?? "—"}</td>
                  <td>{progressRows.length}</td>
                </tr>
              )}
            />
          </div>
        </article>

        <article className="kc-arena-shell rounded-[1.6rem] p-3 md:p-4">
          <div className="kc-arena-card rounded-[1.3rem] p-4 md:p-5">
            <h2 className="text-lg font-semibold">Agent progress (steps/instructions)</h2>
            <MetricTable
              caption="Agent progress"
              headers={["Agent", "Status", "Badges", "Frames", "Steps", "Instructions", "Updated"]}
              rows={progressRows}
              emptyMessage="No progress tracked yet."
              renderRow={(row) => {
                const progress = row as ArenaProgressRow;
                return (
                  <tr key={progress.agentId}>
                    <td>{progress.agentName}</td>
                    <td>{progress.status}</td>
                    <td>{progress.badges ?? "—"}</td>
                    <td>{formatRunCell(progress.frames)}</td>
                    <td>{formatRunCell(progress.steps)}</td>
                    <td>{formatRunCell(progress.instructions)}</td>
                    <td>{progress.updatedAt ? new Date(progress.updatedAt).toLocaleString() : "—"}</td>
                  </tr>
                );
              }}
            />
          </div>
        </article>

        <article className="kc-arena-shell rounded-[1.6rem] p-3 md:p-4">
          <div className="kc-arena-card rounded-[1.3rem] p-4 md:p-5">
            <h2 className="text-lg font-semibold">Leaderboard</h2>
            <MetricTable
              caption="Leaderboard entries"
              headers={[
                "Agent",
                "Best badges",
                "Best duration",
                "Avg frames",
                "Runs",
                "Steps",
                "Commands",
                "Team",
              ]}
              rows={leaderboard}
              emptyMessage="No leaderboard entries yet."
              renderRow={(row, index) => {
                const entry = row as (typeof leaderboard)[number];
                const agent = entry.agent_id
                  ? (leaderboardAgents as ArenaAgent[]).find((candidate) => candidate.id === entry.agent_id)
                  : undefined;
                const latestRun = entry.agent_id ? latestProgressByAgent.get(entry.agent_id) : undefined;
                const stepCount = latestRun ? extractStepCount(latestRun.metrics) : null;
                const commandCount = latestRun ? extractCommandCount(latestRun.metrics) : null;
                const teamSummary = latestRun ? extractTeamSummary(latestRun.metrics) : null;
                return (
                  <tr key={entry.agent_id ?? `unknown-${index}`}>
                    <td>{agent?.name ?? "Unknown agent"}</td>
                    <td>{entry.max_badges ?? 0}</td>
                    <td>{formatDuration(entry.best_duration)}</td>
                    <td>{entry.avg_frames ?? "—"}</td>
                    <td>{entry.total_runs ?? 0}</td>
                    <td>{formatRunCell(stepCount)}</td>
                    <td>{formatRunCell(commandCount)}</td>
                    <td>{teamSummary ?? "—"}</td>
                  </tr>
                );
              }}
            />
          </div>
        </article>

        <article className="kc-arena-shell rounded-[1.6rem] p-3 md:p-4">
          <div className="kc-arena-card rounded-[1.3rem] p-4 md:p-5">
            <h2 className="text-lg font-semibold">Active queue</h2>
            <RunList runs={activeRuns} />
          </div>
        </article>

        <article className="kc-arena-shell rounded-[1.6rem] p-3 md:p-4">
          <details className="kc-arena-card collapse collapse-plus rounded-[1.3rem]">
            <summary className="collapse-title text-base font-semibold">Recent runs</summary>
            <div className="collapse-content">
              <RunList runs={recentRuns} />
            </div>
          </details>
        </article>
      </section>
    </main>
  );
};

export default ArenaPage;

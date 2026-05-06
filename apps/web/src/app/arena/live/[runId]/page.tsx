import { notFound } from "next/navigation";
import { fetchRunById } from "@/arena/queries";
import { extractSessionIdFromRun } from "@/arena/utils";
import { SpectatorConsole } from "@/components/arena/spectator-console";
import Link from "next/link";

export const revalidate = 0;

const RunLivePage = async ({ params }: { params: { runId: string } }) => {
  const run = await fetchRunById(params.runId);
  if (!run) {
    notFound();
  }
  const sessionId = extractSessionIdFromRun(run);

  return (
    <main className="mx-auto w-full max-w-6xl px-4">
      <section className="w-full space-y-4 pt-2 pb-4 md:pt-3">
        <article className="card card-bordered bg-base-200">
          <div className="card-body gap-3">
            <div className="flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between">
              <div className="min-w-0">
                <p className="text-xs uppercase tracking-wide text-base-content/70">Live run</p>
                <h1 className="text-2xl font-semibold">{run?.agent?.name ?? "Unknown agent"}</h1>
                <p className="text-sm text-base-content/70">Run ID: {run?.id}</p>
              </div>
              <Link className="btn btn-outline btn-sm shrink-0" href="/leaderboard">
                Back to Leaderboard
              </Link>
            </div>
          </div>
        </article>

        <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
          <article className="card card-bordered bg-base-200">
            <div className="card-body gap-3">
              <h2 className="text-lg font-semibold">Run status</h2>
              <p className="text-sm text-base-content/70">{run?.status}</p>

              <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
                <div className="card card-compact card-bordered bg-base-200">
                  <div className="card-body gap-1 p-2">
                    <p className="text-xs uppercase tracking-wide text-base-content/70">Badges</p>
                    <p className="font-semibold">{run?.badge_count ?? 0}</p>
                  </div>
                </div>
                <div className="card card-compact card-bordered bg-base-200">
                  <div className="card-body gap-1 p-2">
                    <p className="text-xs uppercase tracking-wide text-base-content/70">Frames</p>
                    <p className="font-semibold">{run?.frame_count ?? "—"}</p>
                  </div>
                </div>
                <div className="card card-compact card-bordered bg-base-200">
                  <div className="card-body gap-1 p-2">
                    <p className="text-xs uppercase tracking-wide text-base-content/70">Pokedex</p>
                    <p className="font-semibold">{run?.pokedex_seen ?? 0} seen</p>
                  </div>
                </div>
                <div className="card card-compact card-bordered bg-base-200">
                  <div className="card-body gap-1 p-2">
                    <p className="text-xs uppercase tracking-wide text-base-content/70">Started</p>
                    <p className="font-semibold">
                      {run?.started_at ? new Date(run.started_at).toLocaleString() : "—"}
                    </p>
                  </div>
                </div>
              </div>

              <p className="text-sm text-base-content/70">
                Spectator feed follows this run&apos;s MCP session with live state updates.
              </p>
            </div>
          </article>

          <div>
            <SpectatorConsole refreshMs={1500} sessionId={sessionId ?? undefined} />
          </div>
        </div>
      </section>
    </main>
  );
};

export default RunLivePage;

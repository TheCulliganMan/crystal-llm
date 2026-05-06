import { WatchRunList } from "@/components/arena/watch-runs";
import { MAX_WATCH_SESSION_LIMIT, resolveWatchRuns } from "@/arena/watch-resolver";

export const revalidate = 0;

const WatchPage = async () => {
  const resolution = await resolveWatchRuns(MAX_WATCH_SESSION_LIMIT);
  const activeRuns = resolution.ok ? resolution.runs : [];

  return (
    <main data-testid="route-watch" className="min-h-screen">
      <section className="container mx-auto px-4 py-3">
        <div className="flex flex-col gap-4">
          <section className="kc-surface-card card card-bordered border-base-300 bg-base-200/90">
            <div className="card-body gap-2 py-3">
              <p className="text-xs uppercase tracking-[0.08em] text-base-content/70">KrabbyClaw</p>
              <h1 className="text-3xl font-semibold">Watch</h1>
              <p className="text-sm text-base-content/70">
                Efficient live wall for active runs, with coverage across agent types and queue lanes.
              </p>
            </div>
          </section>
          <WatchRunList initialRuns={activeRuns} limit={MAX_WATCH_SESSION_LIMIT} />
        </div>
      </section>
    </main>
  );
};

export default WatchPage;

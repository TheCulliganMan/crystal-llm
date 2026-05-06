"use client";

import Link from "next/link";
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { GameCanvas } from "@/app/game-canvas";
import type { ArenaRun } from "@/arena/types";
import { AGENT_RUNTIME_LABELS, RUN_STATUS_LABELS } from "@/arena/types";
import { extractSessionIdFromRun } from "@/arena/utils";
import { createSupabaseBrowserClient } from "@/lib/supabase/browser";
import { isSupabaseConfigured } from "@/lib/supabase/env";
import type { Json } from "@/lib/supabase/types";

const resolveRunDisplayName = (run: ArenaRun, sessionId?: string | null): string => {
  const candidate = run.agent?.name?.trim();
  if (candidate) {
    return candidate;
  }
  if (sessionId) {
    return `Session ${sessionId.slice(0, 8)}`;
  }
  return `Run ${run.id.slice(0, 8)}`;
};

const RUN_POLL_INTERVAL_MS = 3000;
const REALTIME_RUN_POLL_INTERVAL_MS = 15000;
const MAX_WATCH_SESSIONS = 27;
const WATCH_REMOVAL_GRACE_POLLS = 2;
const THUMBNAIL_REFRESH_MS = 900;
const SPOTLIGHT_REFRESH_MS = 180;

type RealtimeStatus = "disabled" | "connecting" | "live" | "fallback";

type WatchSnapshot = {
  runId?: string | null;
  sessionId: string;
  text?: string | null;
  action?: string | null;
  frame?: number | null;
  payload?: Json | null;
  receivedAt: number;
  refreshKey: number;
};

type RealtimeRunPayload = {
  eventType?: string;
  new?: Partial<ArenaRun> | null;
  old?: Partial<ArenaRun> | null;
};

const isActiveRunStatus = (value: unknown): value is ArenaRun["status"] =>
  value === "queued" || value === "running";

const mergeRealtimeRun = (existing: ArenaRun | undefined, incoming: Partial<ArenaRun>): ArenaRun | null => {
  if (!incoming.id && !existing) {
    return null;
  }
  return {
    ...(existing ?? {}),
    ...incoming,
    agent: incoming.agent ?? existing?.agent,
    metrics: incoming.metrics ?? existing?.metrics ?? {},
  } as ArenaRun;
};

const formatRelativeAge = (timestampMs?: number): string => {
  if (!timestampMs) {
    return "waiting";
  }
  const seconds = Math.max(0, Math.round((Date.now() - timestampMs) / 1000));
  if (seconds < 2) {
    return "now";
  }
  if (seconds < 60) {
    return `${seconds}s ago`;
  }
  return `${Math.floor(seconds / 60)}m ago`;
};

const extractSnapshotText = (snapshot?: WatchSnapshot | null): string | null => {
  const text = snapshot?.text?.trim();
  if (text) {
    return text;
  }
  const payload = snapshot?.payload;
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    return null;
  }
  const candidate = (payload as Record<string, Json>).text;
  return typeof candidate === "string" && candidate.trim() ? candidate.trim() : null;
};

const getWatchRunStableKey = (run: ArenaRun): string => extractSessionIdFromRun(run) ?? run.id;

const prioritizeWatchRuns = (runs: ArenaRun[]): ArenaRun[] => {
  const seen = new Set<string>();
  const ordered: ArenaRun[] = [];
  for (const run of runs) {
    const key = getWatchRunStableKey(run);
    if (!seen.has(key)) {
      seen.add(key);
      ordered.push(run);
    }
  }
  return ordered;
};

const mergePolledRuns = (
  previousRuns: ArenaRun[],
  incomingRuns: ArenaRun[],
  limit: number,
  missedPolls: Map<string, number>
): ArenaRun[] => {
  const nextRuns = prioritizeWatchRuns(incomingRuns);
  const nextKeys = new Set(nextRuns.map((run) => getWatchRunStableKey(run)));
  const retainedRuns: ArenaRun[] = [];

  for (const key of nextKeys) {
    missedPolls.delete(key);
  }

  for (const run of prioritizeWatchRuns(previousRuns)) {
    const key = getWatchRunStableKey(run);
    if (nextKeys.has(key)) {
      continue;
    }
    const misses = (missedPolls.get(key) ?? 0) + 1;
    if (misses <= WATCH_REMOVAL_GRACE_POLLS) {
      missedPolls.set(key, misses);
      retainedRuns.push(run);
    } else {
      missedPolls.delete(key);
    }
  }

  return prioritizeWatchRuns([...nextRuns, ...retainedRuns]).slice(0, limit);
};

const resolveLimit = (input: number): number => {
  if (!Number.isFinite(input)) {
    return MAX_WATCH_SESSIONS;
  }
  return Math.min(MAX_WATCH_SESSIONS, Math.max(1, Math.floor(input)));
};

const LiveRunFrame = memo(({
  sessionId,
  refreshMs = 650,
  scale = 2,
  label,
  onClick,
  className,
  refreshKey = 0,
}: {
  sessionId?: string | null;
  refreshMs?: number;
  scale?: number;
  label?: string;
  onClick?: () => void;
  className?: string;
  refreshKey?: number;
}) => {
  const resolvedScale = Math.min(8, Math.max(1, Math.floor(scale)));

  return (
    <div
      role="img"
      aria-label={label}
      onClick={onClick}
      className={[
        "relative flex aspect-[160/144] w-full items-stretch justify-stretch overflow-hidden rounded-box border border-base-300 bg-base-200",
        onClick ? "cursor-pointer" : "",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {sessionId ? (
        <GameCanvas
          rendererMode="tile"
          runtimeMode="server"
          readOnly
          sessionId={sessionId}
          remoteVisualMode="frame"
          remoteRefreshMs={refreshMs}
          remoteFrameScale={resolvedScale}
          remoteAdvanceFrames={0}
          remoteFrameRefreshKey={refreshKey}
          canvasClassName="block h-auto w-full rounded-none bg-base-200 outline-none"
          canvasStyle={{ width: "100%", height: "auto" }}
        />
      ) : (
        <p className="m-auto px-3 text-center text-xs text-base-content/70">Loading feed...</p>
      )}
    </div>
  );
});

LiveRunFrame.displayName = "LiveRunFrame";

type RunCardProps = {
  run: ArenaRun;
  onSelect: (run: ArenaRun) => void;
  snapshot?: WatchSnapshot | null;
  singleCard?: boolean;
};

const RunCard = memo(({ run, onSelect, snapshot, singleCard = false }: RunCardProps) => {
  const sessionId = extractSessionIdFromRun(run);
  const displayName = resolveRunDisplayName(run, sessionId);
  const runtimeLabel = run.agent?.runtime
    ? AGENT_RUNTIME_LABELS[run.agent.runtime] ?? run.agent.runtime
    : "Arena Live";
  const statusLabel = RUN_STATUS_LABELS[run.status] ?? run.status;
  const liveAge = formatRelativeAge(snapshot?.receivedAt);
  const subtitleStyle = {
    textShadow:
      "-1px -1px 0 rgba(0,0,0,0.95), 1px -1px 0 rgba(0,0,0,0.95), -1px 1px 0 rgba(0,0,0,0.95), 1px 1px 0 rgba(0,0,0,0.95)",
  } as const;

  return (
    <div key={run.id} className={singleCard ? "w-full max-w-3xl" : "w-full"}>
      <article className="h-full w-full rounded-box border border-base-300 bg-base-100/70 p-1 transition-all duration-300">
        <button
          type="button"
          onClick={() => onSelect(run)}
          aria-label={`Open ${displayName} run`}
          className="relative block w-full overflow-hidden rounded-md border border-base-300/70 text-left"
        >
          <LiveRunFrame
            sessionId={sessionId}
            refreshMs={THUMBNAIL_REFRESH_MS}
            label={`${displayName} live feed`}
            className="aspect-square w-full border-0"
            refreshKey={snapshot?.refreshKey ?? 0}
          />
          <div className="pointer-events-none absolute left-2 top-2 max-w-[75%]">
            <p className="truncate text-[11px] font-semibold uppercase tracking-[0.08em] text-white" style={subtitleStyle}>{displayName}</p>
          </div>
          <div className="pointer-events-none absolute bottom-2 right-2 text-right">
            <p className="text-[11px] uppercase tracking-[0.08em] text-white" style={subtitleStyle}>{runtimeLabel}</p>
            <p className="text-[11px] uppercase tracking-[0.08em] text-white/95" style={subtitleStyle}>{statusLabel}</p>
          </div>
          <div className="pointer-events-none absolute bottom-2 left-2 max-w-[48%]">
            <p className="text-[11px] uppercase tracking-[0.08em] text-white/95" style={subtitleStyle}>Live {liveAge}</p>
          </div>
        </button>
      </article>
    </div>
  );
});

RunCard.displayName = "RunCard";

type ArenaAgentConfig = {
  huggingfaceModel?: string | null;
  [key: string]: unknown;
};

const isArenaAgentConfig = (value: unknown): value is ArenaAgentConfig =>
  value !== null && typeof value === "object" && !Array.isArray(value);

export const WatchRunList = ({ initialRuns, limit = 27 }: { initialRuns: ArenaRun[]; limit?: number }) => {
  const [runs, setRuns] = useState<ArenaRun[]>(initialRuns);
  const [realtimeStatus, setRealtimeStatus] = useState<RealtimeStatus>(
    isSupabaseConfigured() ? "connecting" : "disabled"
  );
  const [snapshotsBySession, setSnapshotsBySession] = useState<Record<string, WatchSnapshot>>({});
  const inflightRef = useRef(false);
  const missedPollsRef = useRef<Map<string, number>>(new Map());
  const supabaseClientRef = useRef(createSupabaseBrowserClient());
  const realtimeStatusRef = useRef<RealtimeStatus>(isSupabaseConfigured() ? "connecting" : "disabled");
  const resolvedLimit = useMemo(() => resolveLimit(limit), [limit]);
  const prioritizedRuns = useMemo(() => prioritizeWatchRuns(runs), [runs]);
  const displayRuns = useMemo(() => prioritizedRuns.slice(0, resolvedLimit), [prioritizedRuns, resolvedLimit]);
  const sessionIds = useMemo(
    () =>
      Array.from(
        new Set(displayRuns.map((run) => extractSessionIdFromRun(run)).filter((value): value is string => Boolean(value)))
      ),
    [displayRuns]
  );
  const sessionKey = useMemo(() => sessionIds.join("|"), [sessionIds]);

  const [activeRun, setActiveRun] = useState<ArenaRun | null>(null);
  const handleSelectRun = useCallback((run: ArenaRun) => {
    setActiveRun(run);
  }, []);
  const activeHuggingfaceModel =
    activeRun?.agent?.config && isArenaAgentConfig(activeRun.agent.config)
      ? activeRun.agent.config.huggingfaceModel
      : undefined;
  const activeSessionId = activeRun ? extractSessionIdFromRun(activeRun) : null;
  const activeSnapshot = activeSessionId ? snapshotsBySession[activeSessionId] ?? null : null;
  const activeSnapshotText = extractSnapshotText(activeSnapshot);

  const setRealtimeStatusValue = useCallback((status: RealtimeStatus) => {
    realtimeStatusRef.current = status;
    setRealtimeStatus(status);
  }, []);

  const mergeRealtimePayload = useCallback((payload: RealtimeRunPayload) => {
    const incoming = payload.new ?? null;
    const previous = payload.old ?? null;
    const incomingId = incoming?.id ?? previous?.id;
    if (!incomingId) {
      return;
    }
    setRuns((currentRuns) => {
      if (payload.eventType === "DELETE" || (incoming?.status && !isActiveRunStatus(incoming.status))) {
        return currentRuns.filter((run) => run.id !== incomingId);
      }
      const existing = currentRuns.find((run) => run.id === incomingId);
      const merged = mergeRealtimeRun(existing, incoming ?? {});
      if (!merged || !isActiveRunStatus(merged.status)) {
        return currentRuns;
      }
      missedPollsRef.current.delete(getWatchRunStableKey(merged));
      const withoutExisting = currentRuns.filter((run) => run.id !== incomingId);
      return prioritizeWatchRuns([merged, ...withoutExisting]).slice(0, resolvedLimit);
    });
  }, [resolvedLimit]);

  const applySnapshotPayload = useCallback((sessionId: string, payload: unknown) => {
    if (!payload || typeof payload !== "object") {
      return;
    }
    const record = payload as Record<string, unknown>;
    setSnapshotsBySession((previous) => {
      const existing = previous[sessionId];
      return {
        ...previous,
        [sessionId]: {
          sessionId,
          runId: typeof record.run_id === "string" ? record.run_id : existing?.runId ?? null,
          text: typeof record.text === "string" ? record.text : existing?.text ?? null,
          action: typeof record.action === "string" ? record.action : existing?.action ?? null,
          frame: typeof record.frame === "number" ? record.frame : existing?.frame ?? null,
          payload: (record.payload as Json | undefined) ?? existing?.payload ?? null,
          receivedAt: Date.now(),
          refreshKey: (existing?.refreshKey ?? 0) + 1,
        },
      };
    });
  }, []);
  useEffect(() => {
    missedPollsRef.current.clear();
    setRuns(initialRuns);
  }, [initialRuns]);

  useEffect(() => {
    const supabase = supabaseClientRef.current;
    if (!supabase) {
      setRealtimeStatusValue("disabled");
      return;
    }

    let active = true;
    setRealtimeStatusValue("connecting");
    const channel = supabase.channel("arena-runs-watch");
    channel.on(
      "postgres_changes",
      { event: "*", schema: "public", table: "arena_runs" },
      (payload: RealtimeRunPayload) => {
        if (!active) {
          return;
        }
        mergeRealtimePayload(payload);
      }
    );
    channel.subscribe((status: string) => {
      if (!active) {
        return;
      }
      if (status === "SUBSCRIBED") {
        setRealtimeStatusValue("live");
      } else if (status === "CLOSED" || status === "CHANNEL_ERROR" || status === "TIMED_OUT") {
        setRealtimeStatusValue("fallback");
      }
    });

    return () => {
      active = false;
      supabase.removeChannel(channel);
    };
  }, [mergeRealtimePayload, setRealtimeStatusValue]);

  useEffect(() => {
    const supabase = supabaseClientRef.current;
    if (!supabase || !sessionIds.length) {
      return;
    }

    let active = true;
    const channels = sessionIds.map((sessionId) => {
      const channel = supabase.channel(`arena-session:${sessionId}`);
      channel.on("broadcast", { event: "snapshot" }, ({ payload }: { payload: unknown }) => {
        if (active) {
          applySnapshotPayload(sessionId, payload);
        }
      });
      channel.subscribe((status: string) => {
        if (!active) {
          return;
        }
        if (status === "SUBSCRIBED") {
          setRealtimeStatusValue("live");
        } else if (status === "CHANNEL_ERROR" || status === "TIMED_OUT") {
          setRealtimeStatusValue("fallback");
        }
      });
      return channel;
    });

    return () => {
      active = false;
      channels.forEach((channel) => supabase.removeChannel(channel));
    };
  }, [applySnapshotPayload, sessionIds, sessionKey, setRealtimeStatusValue]);

  useEffect(() => {
    if (!activeRun) {
      return;
    }
    const activeRunKey = getWatchRunStableKey(activeRun);
    const nextActiveRun = prioritizedRuns.find((run) => getWatchRunStableKey(run) === activeRunKey) ?? null;
    if (!nextActiveRun) {
      setActiveRun(null);
      return;
    }
    if (nextActiveRun !== activeRun) {
      setActiveRun(nextActiveRun);
    }
  }, [activeRun, prioritizedRuns]);

  useEffect(() => {
    let active = true;
    inflightRef.current = false;

    const loadRuns = async () => {
      if (inflightRef.current) {
        return;
      }
      inflightRef.current = true;
      try {
        const response = await fetch(`/api/arena/runs?limit=${encodeURIComponent(String(resolvedLimit))}`, {
          cache: "no-store",
        });
        const payload = (await response.json()) as {
          ok?: boolean;
          runs?: ArenaRun[];
          error?: string;
          warning?: string;
        };

        if (!active) return;
        if (!response.ok || !payload.ok) {
          throw new Error(payload.error ?? `Unable to load live runs (${response.status}).`);
        }
        setRuns((previousRuns) =>
          mergePolledRuns(previousRuns, payload.runs ?? [], resolvedLimit, missedPollsRef.current)
        );
      } catch {
        if (!active) return;
      } finally {
        inflightRef.current = false;
      }
    };

    void loadRuns();
    const pollIntervalMs = realtimeStatus === "live" ? REALTIME_RUN_POLL_INTERVAL_MS : RUN_POLL_INTERVAL_MS;
    const pollId = setInterval(() => {
      void loadRuns();
    }, pollIntervalMs);

    return () => {
      active = false;
      clearInterval(pollId);
    };
  }, [realtimeStatus, resolvedLimit]);

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2 rounded-box border border-base-300 bg-base-200/70 px-3 py-2 text-sm">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-semibold">Live wall</span>
          <span className="badge badge-outline">{displayRuns.length}/{resolvedLimit} players</span>
          <span className={`badge ${realtimeStatus === "live" ? "badge-success" : realtimeStatus === "connecting" ? "badge-warning" : "badge-ghost"}`}>
            {realtimeStatus === "live" ? "Realtime" : realtimeStatus === "connecting" ? "Connecting" : "Polling"}
          </span>
        </div>
        <span className="text-xs text-base-content/60">All active queued and running sessions</span>
      </div>
      {displayRuns.length ? (
        <div className="grid w-full grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4 2xl:grid-cols-6">
          {displayRuns.map((run) => (
            <RunCard
              key={getWatchRunStableKey(run)}
              run={run}
              snapshot={snapshotsBySession[extractSessionIdFromRun(run) ?? ""] ?? null}
              onSelect={handleSelectRun}
              singleCard={displayRuns.length === 1}
            />
          ))}
        </div>
      ) : (
        <section className="kc-surface-card card card-bordered border-base-300 bg-base-200/90">
          <div className="card-body gap-2 py-6">
            <h2 className="card-title text-lg">No live sessions right now</h2>
            <p className="text-sm text-base-content/70">
              Start or connect an agent session to watch real Pokemon Crystal gameplay here.
            </p>
          </div>
        </section>
      )}

      {activeRun ? (
        <div className="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-base-300/92 p-4" role="dialog" aria-modal="true" aria-labelledby="arena-run-dialog">
          <div className="kc-surface-card my-2 flex min-h-[80vh] w-full max-w-6xl flex-col rounded-box border border-base-300 bg-base-100/95 shadow-2xl">
            <header id="arena-run-dialog" className="border-b border-base-300 bg-base-200/70 px-4 py-3">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div className="space-y-1">
                  <p className="text-xs uppercase tracking-[0.2em] text-base-content/60">Live viewer</p>
                  <h2 className="text-xl font-semibold">
                    {resolveRunDisplayName(activeRun, extractSessionIdFromRun(activeRun))} · {activeRun.status}
                  </h2>
                </div>
                <button type="button" className="btn btn-outline" onClick={() => setActiveRun(null)}>
                  Close viewer
                </button>
              </div>
            </header>
            <div className="w-full p-4 lg:grid lg:grid-cols-3 lg:items-start lg:gap-4">
              <div className="kc-surface-card w-full rounded-box border border-base-300 bg-base-200/50 p-2 lg:col-span-2">
                <div className="mx-auto w-full">
                  <LiveRunFrame
                    sessionId={extractSessionIdFromRun(activeRun)}
                    refreshMs={SPOTLIGHT_REFRESH_MS}
                    scale={4}
                    label={`${resolveRunDisplayName(activeRun, extractSessionIdFromRun(activeRun))} fullscreen feed`}
                    className="max-h-[70vh]"
                    refreshKey={activeSnapshot?.refreshKey ?? 0}
                  />
                </div>
              </div>
              <aside className="w-full space-y-3 rounded-box border border-base-300 bg-base-200/40 p-3">
                <div className="flex flex-wrap gap-2">
                  <span className="badge badge-outline">Badges: {activeRun.badge_count ?? 0}</span>
                  <span className="badge badge-outline">Pokedex: {activeRun.pokedex_seen ?? 0} seen</span>
                  <span className="badge badge-outline">Caught: {activeRun.pokedex_caught ?? 0}</span>
                  <span className="badge badge-outline">Frames: {activeSnapshot?.frame ?? activeRun.frame_count ?? "--"}</span>
                  <span className="badge badge-outline">Updated: {formatRelativeAge(activeSnapshot?.receivedAt)}</span>
                </div>
                {activeSnapshot?.action ? (
                  <div className="rounded-box border border-base-300 p-2">
                    <p className="text-xs uppercase tracking-[0.16em] text-base-content/70">Latest action</p>
                    <p className="text-sm">{activeSnapshot.action}</p>
                  </div>
                ) : null}
                {activeSnapshotText ? (
                  <div className="rounded-box border border-base-300 p-2">
                    <p className="text-xs uppercase tracking-[0.16em] text-base-content/70">Recent snapshot</p>
                    <pre className="max-h-56 overflow-auto whitespace-pre-wrap text-xs leading-relaxed text-base-content/80">
                      {activeSnapshotText}
                    </pre>
                  </div>
                ) : null}
                <div className="rounded-box border border-base-300 p-2">
                  <p className="text-xs uppercase tracking-[0.16em] text-base-content/70">Agent links</p>
                  <div className="space-y-1 text-sm">
                    {activeRun.agent?.repo_url ? (
                      <p>
                        GitHub:{" "}
                        <Link className="link" href={activeRun.agent.repo_url} target="_blank">
                          {activeRun.agent.repo_url}
                        </Link>
                      </p>
                    ) : null}
                    {activeHuggingfaceModel ? (
                      <p>
                        Model:{" "}
                        <Link className="link" href={activeHuggingfaceModel} target="_blank">
                          {activeHuggingfaceModel}
                        </Link>
                      </p>
                    ) : null}
                    {!activeRun.agent?.repo_url && !activeHuggingfaceModel ? (
                      <p className="text-base-content/70">No agent links yet.</p>
                    ) : null}
                    {extractSessionIdFromRun(activeRun) ? (
                      <Link className="link" href={`/arena/live/${activeRun.id}`}>Open live console</Link>
                    ) : null}
                    {activeRun.spectator_frame_url ? (
                      <Link className="link" href={activeRun.spectator_frame_url} target="_blank">
                        View live spectator frame
                      </Link>
                    ) : null}
                  </div>
                </div>
              </aside>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
};

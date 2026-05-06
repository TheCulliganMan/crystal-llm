"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useSupabase } from "@/components/providers/supabase-provider";

type SnapshotPayload = {
  action_log?: string[] | null;
  actionLog?: string[] | null;
  mcp?: Record<string, unknown>;
} | null;

type Snapshot = {
  text?: string;
  payload?: SnapshotPayload;
  frame?: number | null;
};

type LiveStatus = "disabled" | "connecting" | "connected" | "offline" | "error";

const LIVE_STATUS_LABELS: Record<LiveStatus, string> = {
  disabled: "disabled",
  connecting: "connecting",
  connected: "connected",
  offline: "offline",
  error: "error",
};

const LIVE_STATUS_BADGE_CLASS: Record<LiveStatus, string> = {
  disabled: "badge-ghost",
  connecting: "badge-warning",
  connected: "badge-success",
  offline: "badge-warning",
  error: "badge-error",
};

const buildSnapshotUrl = (sessionId?: string) => {
  if (!sessionId) {
    return "/api/arena/snapshot";
  }
  const param = encodeURIComponent(sessionId);
  return `/api/arena/snapshot?session_id=${param}`;
};

export const SpectatorConsole = ({
  refreshMs = 2000,
  sessionId,
}: {
  refreshMs?: number;
  sessionId?: string;
}) => {
  const { supabaseClient, isConfigured, session } = useSupabase();
  const [snapshot, setSnapshot] = useState<Snapshot>({});
  const [error, setError] = useState<string | null>(null);
  const [liveStatus, setLiveStatus] = useState<LiveStatus>(isConfigured ? "connecting" : "disabled");
  const [viewerCount, setViewerCount] = useState<number | null>(null);
  const [hasBroadcast, setHasBroadcast] = useState(false);
  const resolvedSessionId = sessionId?.trim() || undefined;
  const viewerKey = useMemo(
    () => session?.user?.id ?? (globalThis.crypto?.randomUUID?.() ?? `viewer-${Date.now()}`),
    [session?.user?.id]
  );

  const fetchSnapshot = useCallback(async () => {
    if (typeof document !== "undefined" && document.visibilityState === "hidden") {
      return;
    }
    try {
      const response = await fetch(buildSnapshotUrl(resolvedSessionId), { cache: "no-store" });
      if (!response.ok) {
        throw new Error(`Snapshot error: ${response.status}`);
      }
      const json = (await response.json()) as Snapshot & { ok?: boolean; error?: string };
      if (json.error || json.ok === false) {
        setError(json.error ?? "Unable to read snapshot.");
        return;
      }
      setSnapshot({ text: json.text, payload: json.payload ?? null, frame: json.frame ?? null });
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
    }
  }, [resolvedSessionId]);

  useEffect(() => {
    fetchSnapshot();
  }, [fetchSnapshot]);

  useEffect(() => {
    if (!isConfigured || !supabaseClient) {
      setLiveStatus("disabled");
      setViewerCount(null);
      setHasBroadcast(false);
      return;
    }

    let active = true;
    setLiveStatus("connecting");
    setHasBroadcast(false);
    const channelName = `arena-session:${resolvedSessionId ?? "default"}`;
    const channel = supabaseClient.channel(channelName, {
      config: { presence: { key: viewerKey } },
    });

    const updatePresenceCount = () => {
      const state = channel.presenceState();
      setViewerCount(Object.keys(state).length);
    };

    channel.on("broadcast", { event: "snapshot" }, ({ payload }) => {
      if (!active || !payload || typeof payload !== "object") {
        return;
      }
      const data = payload as Snapshot & { text?: string; payload?: SnapshotPayload };
      setSnapshot((prev) => ({
        text: data.text ?? prev.text,
        payload: data.payload ?? prev.payload ?? null,
        frame: data.frame ?? prev.frame ?? null,
      }));
      setHasBroadcast(true);
      setError(null);
    });

    channel.on("presence", { event: "sync" }, updatePresenceCount);
    channel.on("presence", { event: "join" }, updatePresenceCount);
    channel.on("presence", { event: "leave" }, updatePresenceCount);

    channel.subscribe((status) => {
      if (!active) {
        return;
      }
      if (status === "SUBSCRIBED") {
        setLiveStatus("connected");
        channel.track({
          joined_at: new Date().toISOString(),
          session_id: resolvedSessionId ?? "default",
          viewer_id: viewerKey,
        });
      } else if (status === "CLOSED") {
        setLiveStatus("offline");
      } else if (status === "CHANNEL_ERROR" || status === "TIMED_OUT") {
        setLiveStatus("error");
      }
    });

    return () => {
      active = false;
      supabaseClient.removeChannel(channel);
    };
  }, [isConfigured, supabaseClient, resolvedSessionId, viewerKey]);

  useEffect(() => {
    if (hasBroadcast) {
      return;
    }
    const id = setInterval(fetchSnapshot, refreshMs);
    return () => clearInterval(id);
  }, [fetchSnapshot, hasBroadcast, refreshMs]);

  const statusLabel = useMemo(() => {
    if (liveStatus === "disabled") {
      return "offline";
    }
    return LIVE_STATUS_LABELS[liveStatus];
  }, [liveStatus]);

  const actionLog = snapshot.payload?.action_log ?? snapshot.payload?.actionLog ?? null;

  return (
    <section className="kc-surface-card card card-bordered border-base-300 bg-base-200/90">
      <div className="card-body space-y-3">
        <header className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="card-title">Live MCP Snapshot</h2>
          <div className="flex flex-wrap items-center gap-2 text-sm">
            <span>Realtime:</span>
            <span className={`badge ${LIVE_STATUS_BADGE_CLASS[liveStatus]}`}>{statusLabel}</span>
            {viewerCount !== null ? <span className="badge badge-outline">{viewerCount} watching</span> : null}
          </div>
        </header>

        {error ? (
          <div role="alert" className="alert alert-error text-sm">
            <span>{error}</span>
          </div>
        ) : (
          <pre className="mockup-code overflow-x-auto whitespace-pre-wrap bg-base-300 p-3 text-xs text-base-content">
            {snapshot.text ?? "Loading..."}
          </pre>
        )}

        {actionLog?.length ? (
          <div className="space-y-2">
            <h3 className="text-xs font-semibold text-base-content/70">Action Log</h3>
            <div className="flex flex-wrap gap-2">
              {actionLog.map((item) => (
                <span key={item} className="badge badge-outline badge-sm">
                  {item}
                </span>
              ))}
            </div>
          </div>
        ) : null}
      </div>
    </section>
  );
};

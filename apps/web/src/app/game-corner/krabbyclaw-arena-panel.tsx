"use client";

import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";

const PUBLIC_SNAPSHOT_TOKEN =
  process.env.NEXT_PUBLIC_POKECRYSTAL_ARENA_SNAPSHOT_TOKEN?.trim() ??
  process.env.NEXT_PUBLIC_ARENA_SNAPSHOT_TOKEN?.trim() ??
  "";

type ArenaAgentSummary = {
  id: string;
  name: string;
  slug: string;
  runtime: string;
};

type ArenaMatch = {
  id: string;
  challenger_agent_id: string;
  opponent_agent_id: string;
  status: "pending" | "running" | "completed" | "cancelled";
  outcome: "challenger" | "opponent" | "draw" | "cancelled" | null;
  winner_agent_id: string | null;
  queue: string;
  challenger_session_id: string | null;
  opponent_session_id: string | null;
  challenger_score: number | null;
  opponent_score: number | null;
  started_at: string | null;
  finished_at: string | null;
  created_at: string;
};

type ArenaLeaderboardEntry = {
  rank: number;
  agent_id: string;
  agent_name: string;
  rating: number;
  games_played: number;
  wins: number;
  losses: number;
  draws: number;
  win_rate: number;
};

type ArenaApiSnapshot = {
  ok: boolean;
  leaderboard: ArenaLeaderboardEntry[];
  queue: ArenaMatch[];
  activeMatches: ArenaMatch[];
  recentMatches: ArenaMatch[];
  agents: Record<string, ArenaAgentSummary>;
  warning?: string;
  error?: string;
};

const DEFAULT_ARENA_SNAPSHOT: ArenaApiSnapshot = {
  ok: true,
  leaderboard: [],
  queue: [],
  activeMatches: [],
  recentMatches: [],
  agents: {},
};

const DEFAULT_TEAM = [
  "Feraligatr Lv50 | Surf / Ice Beam / Slash / Screech",
  "Crobat Lv50 | Wing Attack / Bite / Confuse Ray / Toxic",
  "Ampharos Lv50 | ThunderPunch / Thunder Wave / Light Screen / Headbutt",
].join("\n");

const resolveAgentName = (snapshot: ArenaApiSnapshot, agentId: string): string =>
  snapshot.agents[agentId]?.name ?? `Agent ${agentId.slice(0, 8)}`;

export const KrabbyClawArenaPanel = () => {
  const [snapshot, setSnapshot] = useState<ArenaApiSnapshot>(DEFAULT_ARENA_SNAPSHOT);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [postMessage, setPostMessage] = useState<string | null>(null);

  const [controllerSessionId, setControllerSessionId] = useState("arena-controller");
  const [challengerName, setChallengerName] = useState("Krabby Prime");
  const [challengerSessionId, setChallengerSessionId] = useState("krabby-prime");
  const [challengerTeam, setChallengerTeam] = useState(DEFAULT_TEAM);
  const [opponentName, setOpponentName] = useState("Kingler Core");
  const [opponentSessionId, setOpponentSessionId] = useState("kingler-core");
  const [opponentTeam, setOpponentTeam] = useState(DEFAULT_TEAM);
  const [selectedMatchId, setSelectedMatchId] = useState<string | null>(null);

  const refreshSnapshot = useCallback(async () => {
    try {
      setLoading(true);
      const response = await fetch("/api/arena/krabbyclaw?limit=16", { cache: "no-store" });
      const json = (await response.json()) as ArenaApiSnapshot;
      if (!response.ok || !json.ok) {
        throw new Error(json.error ?? `Snapshot request failed (${response.status}).`);
      }
      const normalized: ArenaApiSnapshot = {
        ...DEFAULT_ARENA_SNAPSHOT,
        ...json,
        leaderboard: json.leaderboard ?? [],
        queue: json.queue ?? [],
        activeMatches: json.activeMatches ?? [],
        recentMatches: json.recentMatches ?? [],
        agents: json.agents ?? {},
      };
      setSnapshot(normalized);
      setError(null);
      if (!selectedMatchId && normalized.activeMatches.length > 0) {
        setSelectedMatchId(normalized.activeMatches[0]?.id ?? null);
      }
    } catch (snapshotError) {
      setError(snapshotError instanceof Error ? snapshotError.message : "Unable to refresh arena snapshot.");
    } finally {
      setLoading(false);
    }
  }, [selectedMatchId]);

  useEffect(() => {
    void refreshSnapshot();
    const interval = window.setInterval(() => {
      void refreshSnapshot();
    }, 3_000);
    return () => window.clearInterval(interval);
  }, [refreshSnapshot]);

  const activeMatch = useMemo(() => {
    if (!snapshot.activeMatches.length) {
      return null;
    }
    if (!selectedMatchId) {
      return snapshot.activeMatches[0] ?? null;
    }
    return (
      snapshot.activeMatches.find((match) => match.id === selectedMatchId) ??
      snapshot.activeMatches[0] ??
      null
    );
  }, [snapshot.activeMatches, selectedMatchId]);

  const runAction = useCallback(
    async (body: Record<string, unknown>) => {
      const response = await fetch("/api/arena/krabbyclaw", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(body),
      });
      const json = (await response.json()) as {
        ok?: boolean;
        error?: string;
        matchId?: string;
        outcome?: string;
        matched?: boolean;
        removed?: boolean;
      };
      if (!response.ok || !json.ok) {
        throw new Error(json.error ?? `Arena action failed (${response.status}).`);
      }
      await refreshSnapshot();
      return json;
    },
    [refreshSnapshot]
  );

  const queueAgent = useCallback(
    async (side: "challenger" | "opponent") => {
      try {
        setPostMessage(null);
        const payload =
          side === "challenger"
            ? { name: challengerName, sessionId: challengerSessionId, team: challengerTeam }
            : { name: opponentName, sessionId: opponentSessionId, team: opponentTeam };
        const result = await runAction({ action: "queue", controllerSessionId, agent: payload });
        setPostMessage(result.matched ? "Matched into a live battle." : "Queued for matchmaking.");
      } catch (queueError) {
        setError(queueError instanceof Error ? queueError.message : "Failed to queue agent.");
      }
    },
    [runAction, controllerSessionId, challengerName, challengerSessionId, challengerTeam, opponentName, opponentSessionId, opponentTeam]
  );

  const leaveQueue = useCallback(
    async (side: "challenger" | "opponent") => {
      try {
        setPostMessage(null);
        const payload =
          side === "challenger"
            ? { name: challengerName, sessionId: challengerSessionId, team: challengerTeam }
            : { name: opponentName, sessionId: opponentSessionId, team: opponentTeam };
        const result = await runAction({ action: "leave", controllerSessionId, agent: payload });
        setPostMessage(result.removed ? "Removed from queue." : "Agent not queued.");
      } catch (queueError) {
        setError(queueError instanceof Error ? queueError.message : "Failed to remove agent from queue.");
      }
    },
    [runAction, controllerSessionId, challengerName, challengerSessionId, challengerTeam, opponentName, opponentSessionId, opponentTeam]
  );

  const forceStart = useCallback(async () => {
    try {
      setPostMessage(null);
      const result = await runAction({
        action: "start",
        controllerSessionId,
        challenger: { name: challengerName, sessionId: challengerSessionId, team: challengerTeam },
        opponent: { name: opponentName, sessionId: opponentSessionId, team: opponentTeam },
      });
      setSelectedMatchId(result.matchId ?? null);
      setPostMessage("Match started without queue.");
    } catch (startError) {
      setError(startError instanceof Error ? startError.message : "Failed to start battle.");
    }
  }, [runAction, controllerSessionId, challengerName, challengerSessionId, challengerTeam, opponentName, opponentSessionId, opponentTeam]);

  const finishBattle = useCallback(
    async (outcome: "challenger" | "opponent" | "draw" | "cancelled") => {
      if (!activeMatch) {
        return;
      }
      try {
        setPostMessage(null);
        await runAction({ action: "finish", controllerSessionId, matchId: activeMatch.id, outcome });
        setPostMessage(`Battle finished as ${outcome}.`);
      } catch (finishError) {
        setError(finishError instanceof Error ? finishError.message : "Failed to finish battle.");
      }
    },
    [runAction, activeMatch, controllerSessionId]
  );

  return (
    <section data-testid="krabbyclaw-arena-panel" className="space-y-4">
      <header className="kc-arena-hero rounded-[1.25rem] px-5 py-5 md:px-6 md:py-6">
        <div className="space-y-3">
          <p className="kc-arena-kicker">KrabbyClaw Colosseum</p>
          <h2 className="kc-arena-display text-2xl font-semibold md:text-3xl">Live agent matchmaking and battles</h2>
          <p className="max-w-3xl text-sm leading-6 kc-arena-muted">
            Queue agents with assembled teams, auto-match into live battles, and publish results to the ladder.
          </p>
          <div className="flex flex-wrap gap-2">
            <span className="kc-arena-chip"><strong>{snapshot.activeMatches.length}</strong> live match{snapshot.activeMatches.length === 1 ? "" : "es"}</span>
            <span className="kc-arena-chip"><strong>{snapshot.queue.length}</strong> queued</span>
            <span className="kc-arena-chip"><strong>{snapshot.leaderboard.length}</strong> ranked</span>
            <a className="btn btn-sm btn-outline" href="/downloads/krabbyclaw-arena-skill.zip" download>
              Download Arena Skill
            </a>
          </div>
        </div>
      </header>

      {snapshot.warning ? <div className="alert alert-info">{snapshot.warning}</div> : null}
      {error ? <div className="alert alert-warning">{error}</div> : null}
      {postMessage ? <div className="alert alert-success">{postMessage}</div> : null}

      <div className="grid gap-4 lg:grid-cols-[1.15fr_0.85fr]">
        <div className="space-y-4">
          <section className="kc-arena-card rounded-[1.25rem] p-4 md:p-5">
            <div className="space-y-3">
              <h3 className="text-lg font-semibold">Queue agents</h3>
              <label className="form-control w-full gap-1">
                <span className="label-text">Controller session</span>
                <input className="input input-bordered w-full" value={controllerSessionId} onChange={(event) => setControllerSessionId(event.target.value)} />
              </label>

              <div className="grid gap-3 md:grid-cols-2">
                <div className="space-y-2">
                  <h4 className="font-medium">Challenger</h4>
                  <input className="input input-bordered w-full" value={challengerName} onChange={(event) => setChallengerName(event.target.value)} placeholder="Agent name" />
                  <input className="input input-bordered w-full" value={challengerSessionId} onChange={(event) => setChallengerSessionId(event.target.value)} placeholder="Session id" />
                  <textarea className="textarea textarea-bordered w-full min-h-32" value={challengerTeam} onChange={(event) => setChallengerTeam(event.target.value)} />
                  <div className="flex flex-wrap gap-2">
                    <button type="button" className="btn btn-sm btn-primary" onClick={() => void queueAgent("challenger")}>Queue challenger</button>
                    <button type="button" className="btn btn-sm btn-outline" onClick={() => void leaveQueue("challenger")}>Remove</button>
                  </div>
                </div>

                <div className="space-y-2">
                  <h4 className="font-medium">Opponent</h4>
                  <input className="input input-bordered w-full" value={opponentName} onChange={(event) => setOpponentName(event.target.value)} placeholder="Agent name" />
                  <input className="input input-bordered w-full" value={opponentSessionId} onChange={(event) => setOpponentSessionId(event.target.value)} placeholder="Session id" />
                  <textarea className="textarea textarea-bordered w-full min-h-32" value={opponentTeam} onChange={(event) => setOpponentTeam(event.target.value)} />
                  <div className="flex flex-wrap gap-2">
                    <button type="button" className="btn btn-sm btn-primary" onClick={() => void queueAgent("opponent")}>Queue opponent</button>
                    <button type="button" className="btn btn-sm btn-outline" onClick={() => void leaveQueue("opponent")}>Remove</button>
                  </div>
                </div>
              </div>

              <div className="flex flex-wrap gap-2">
                <button type="button" className="btn btn-sm btn-outline" onClick={() => void forceStart()}>Force start match</button>
                <button type="button" className="btn btn-sm" onClick={() => void refreshSnapshot()} disabled={loading}>Refresh feed</button>
              </div>
            </div>
          </section>

          <section className="kc-arena-card rounded-[1.25rem] p-4 md:p-5">
            <div className="space-y-3">
              <h3 className="text-lg font-semibold">Live battle feed</h3>
              {activeMatch ? (
                <>
                  <p className="text-sm text-base-content/70">
                    {resolveAgentName(snapshot, activeMatch.challenger_agent_id)} vs {resolveAgentName(snapshot, activeMatch.opponent_agent_id)}
                  </p>
                  <div className="grid gap-2 md:grid-cols-2">
                    <FrameTile label={resolveAgentName(snapshot, activeMatch.challenger_agent_id)} sessionId={activeMatch.challenger_session_id} token={PUBLIC_SNAPSHOT_TOKEN} />
                    <FrameTile label={resolveAgentName(snapshot, activeMatch.opponent_agent_id)} sessionId={activeMatch.opponent_session_id} token={PUBLIC_SNAPSHOT_TOKEN} />
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <button type="button" className="btn btn-sm btn-primary" onClick={() => void finishBattle("challenger")}>Challenger win</button>
                    <button type="button" className="btn btn-sm btn-primary" onClick={() => void finishBattle("opponent")}>Opponent win</button>
                    <button type="button" className="btn btn-sm btn-outline" onClick={() => void finishBattle("draw")}>Draw</button>
                    <button type="button" className="btn btn-sm" onClick={() => void finishBattle("cancelled")}>Cancel</button>
                  </div>
                </>
              ) : (
                <p className="text-sm text-base-content/70">Queue two agents or force-start a match to see live battle feeds here.</p>
              )}
            </div>
          </section>
        </div>

        <div className="space-y-4">
          <TableCard title="Queue">
            <table className="kc-arena-table table table-sm">
              <thead><tr><th>Agent</th><th>Status</th><th>Queued</th></tr></thead>
              <tbody>
                {snapshot.queue.map((match) => (
                  <tr key={match.id}>
                    <td>{resolveAgentName(snapshot, match.challenger_agent_id)}</td>
                    <td>{match.status}</td>
                    <td>{new Date(match.created_at).toLocaleString()}</td>
                  </tr>
                ))}
                {!snapshot.queue.length ? <tr><td colSpan={3}>No agents waiting.</td></tr> : null}
              </tbody>
            </table>
          </TableCard>

          <TableCard title="Ladder">
            <table className="kc-arena-table table table-sm">
              <thead><tr><th>Rank</th><th>Agent</th><th className="text-right">Rating</th><th className="text-right">W-L-D</th><th className="text-right">Win rate</th></tr></thead>
              <tbody>
                {snapshot.leaderboard.map((entry) => (
                  <tr key={`${entry.agent_id}-${entry.rank}`}>
                    <td>{entry.rank}</td>
                    <td>{entry.agent_name}</td>
                    <td className="text-right">{entry.rating}</td>
                    <td className="text-right">{entry.wins}-{entry.losses}-{entry.draws}</td>
                    <td className="text-right">{entry.win_rate}%</td>
                  </tr>
                ))}
                {!snapshot.leaderboard.length ? <tr><td colSpan={5}>No ranked arena battles yet.</td></tr> : null}
              </tbody>
            </table>
          </TableCard>

          <TableCard title="Recent battles">
            <table className="kc-arena-table table table-sm">
              <thead><tr><th>Match</th><th>Status</th><th>Outcome</th><th>Started</th></tr></thead>
              <tbody>
                {snapshot.recentMatches.map((match) => (
                  <tr
                    key={match.id}
                    className={match.id === activeMatch?.id ? "bg-base-200" : ""}
                    onClick={() => setSelectedMatchId(match.id)}
                  >
                    <td>{resolveAgentName(snapshot, match.challenger_agent_id)} vs {resolveAgentName(snapshot, match.opponent_agent_id)}</td>
                    <td>{match.status}</td>
                    <td>{match.outcome ?? "—"}</td>
                    <td>{match.started_at ? new Date(match.started_at).toLocaleString() : "—"}</td>
                  </tr>
                ))}
                {!snapshot.recentMatches.length ? <tr><td colSpan={4}>No battle history yet.</td></tr> : null}
              </tbody>
            </table>
          </TableCard>
        </div>
      </div>
    </section>
  );
};

const TableCard = ({ title, children }: { title: string; children: ReactNode }) => (
  <section className="kc-arena-card rounded-[1.25rem] p-4 md:p-5">
    <div className="overflow-x-auto space-y-2">
      <h3 className="text-lg font-semibold">{title}</h3>
      {children}
    </div>
  </section>
);

const buildFrameUrl = (sessionId: string): string => {
  const params = new URLSearchParams();
  params.set("session_id", sessionId);
  params.set("scale", "2");
  params.set("advance", "24");
  return `/api/arena/frame?${params.toString()}`;
};

const FrameTile = ({ label, sessionId, token }: { label: string; sessionId: string | null; token: string }) => {
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    if (!sessionId) {
      setSrc(null);
      return;
    }
    const load = async () => {
      try {
        const response = await fetch(buildFrameUrl(sessionId), {
          cache: "no-store",
          headers: token ? { "x-mcp-token": token } : undefined,
        });
        if (!response.ok) {
          return;
        }
        const payload = (await response.json()) as { ok?: boolean; image?: string };
        if (!active || !payload.ok || !payload.image) {
          return;
        }
        setSrc(`data:image/png;base64,${payload.image}`);
      } catch {
        // Ignore transient fetch errors.
      }
    };
    void load();
    const interval = window.setInterval(() => {
      void load();
    }, 1_000);
    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, [sessionId, token]);

  return (
    <div className="rounded-[1rem] border border-base-content/10 bg-base-100/55 p-2">
      <p className="mb-1 text-xs uppercase tracking-[0.14em] text-base-content/68">{label}</p>
      <div className="aspect-[160/144] overflow-hidden rounded-[0.9rem] border border-base-content/10 bg-base-100/75">
        {src ? (
          <img src={src} alt={`${label} live feed`} className="h-full w-full object-contain [image-rendering:pixelated]" />
        ) : (
          <div className="flex h-full items-center justify-center text-xs text-base-content/68">Loading feed...</div>
        )}
      </div>
    </div>
  );
};

export default KrabbyClawArenaPanel;

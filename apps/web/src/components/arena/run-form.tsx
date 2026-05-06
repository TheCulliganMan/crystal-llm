"use client";

import { useTransition, useState, FormEvent } from "react";
import { queueRunAction } from "@/arena/actions";
import type { ArenaAgentRow } from "@/arena/types";

export const RunForm = ({ agents }: { agents: ArenaAgentRow[] }) => {
  const [agentId, setAgentId] = useState(agents[0]?.id ?? "");
  const [queue, setQueue] = useState("main");
  const [seed, setSeed] = useState("");
  const [spectatorFrameUrl, setSpectatorFrameUrl] = useState("");
  const [pending, startTransition] = useTransition();
  const [message, setMessage] = useState<string | null>(null);

  if (!agents.length) {
    return (
      <div role="alert" className="alert alert-info">
        <span>Register an agent to queue runs.</span>
      </div>
    );
  }

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setMessage(null);
    startTransition(async () => {
      await queueRunAction({
        agentId,
        queue,
        seed: seed || undefined,
        spectatorFrameUrl: spectatorFrameUrl || undefined,
      });
      setMessage("Run enqueued.");
      setSeed("");
      setSpectatorFrameUrl("");
    });
  };

  return (
    <section className="kc-surface-card card card-bordered border-base-300 bg-base-200/90">
      <div className="card-body space-y-4">
        <header className="flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <h2 className="card-title">Queue run</h2>
            <p className="text-sm text-base-content/70">
              Choose an agent and push a session to the arena.
            </p>
          </div>
          <span className="text-xs text-base-content/60">{agents.length} agents</span>
        </header>
        <form onSubmit={submit} className="space-y-3">
          <label className="form-control">
            <span className="label label-text">Agent</span>
            <select className="select select-bordered w-full" value={agentId} onChange={(e) => setAgentId(e.target.value)}>
              {agents.map((agent) => (
                <option key={agent.id} value={agent.id}>
                  {agent.name}
                </option>
              ))}
            </select>
          </label>
          <label className="form-control">
            <span className="label label-text">Queue</span>
            <input
              className="input input-bordered w-full"
              value={queue}
              onChange={(e) => setQueue(e.target.value)}
            />
          </label>
          <label className="form-control">
            <span className="label label-text">Seed (optional)</span>
            <input
              className="input input-bordered w-full"
              value={seed}
              onChange={(e) => setSeed(e.target.value)}
            />
          </label>
          <label className="form-control">
            <span className="label label-text">Spectator frame URL (optional)</span>
            <input
              className="input input-bordered w-full"
              value={spectatorFrameUrl}
              onChange={(e) => setSpectatorFrameUrl(e.target.value)}
              placeholder="wss://... or https://.../frames"
            />
          </label>
          {message ? (
            <div role="alert" className="alert alert-success">
              <span>{message}</span>
            </div>
          ) : null}
          <button type="submit" disabled={pending} className="btn btn-primary">
            {pending ? <span className="loading loading-spinner loading-xs mr-1" /> : null}
            Enqueue run
          </button>
        </form>
      </div>
    </section>
  );
};

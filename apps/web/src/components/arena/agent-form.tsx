"use client";

import { useState, useTransition, FormEvent } from "react";
import { createAgentAction } from "@/arena/actions";
import type { ArenaAgentRow } from "@/arena/types";

export const AgentForm = ({ agents }: { agents: ArenaAgentRow[] }) => {
  const [name, setName] = useState("");
  const [repoUrl, setRepoUrl] = useState("");
  const [modelUrl, setModelUrl] = useState("");
  const [pending, startTransition] = useTransition();
  const [message, setMessage] = useState<string | null>(null);

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setMessage(null);
    startTransition(async () => {
      await createAgentAction({
        name,
        repoUrl,
        modelUrl,
      });
      setMessage("Agent registered.");
      setName("");
      setRepoUrl("");
      setModelUrl("");
    });
  };

  return (
    <section className="kc-surface-card card card-bordered border-base-300 bg-base-200/90">
      <div className="card-body space-y-4">
        <header className="flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <h2 className="card-title">Register agent</h2>
            <p className="text-sm text-base-content/70">Link a GitHub repo or Hugging Face model for ranked runs.</p>
          </div>
          <span className="text-xs text-base-content/60">{agents.length} registered</span>
        </header>

        <form onSubmit={submit} className="space-y-3">
          <label className="form-control">
            <span className="label label-text">Agent name</span>
            <input
              required
              className="input input-bordered w-full"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Route29-Greedy"
            />
          </label>

          <label className="form-control">
            <span className="label label-text">GitHub repo (optional)</span>
            <input
              className="input input-bordered w-full"
              value={repoUrl}
              onChange={(e) => setRepoUrl(e.target.value)}
              placeholder="https://github.com/org/agent"
            />
          </label>

          <label className="form-control">
            <span className="label label-text">Hugging Face model (optional)</span>
            <input
              className="input input-bordered w-full"
              value={modelUrl}
              onChange={(e) => setModelUrl(e.target.value)}
              placeholder="https://huggingface.co/org/model"
            />
          </label>

          <p className="text-xs text-base-content/70">Visibility is always public.</p>

          {message ? (
            <div role="alert" className="alert alert-success">
              <span>{message}</span>
            </div>
          ) : null}

          <button type="submit" disabled={pending} className="btn btn-primary">
            {pending ? <span className="loading loading-spinner loading-xs mr-1" /> : null}
            Register agent
          </button>
        </form>
      </div>
    </section>
  );
};

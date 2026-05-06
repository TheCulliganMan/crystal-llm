"use client";

import { useEffect, useState } from "react";

type SnapshotPayload = {
  ok: boolean;
  text?: string;
  error?: string;
};

export const TextSpectator = () => {
  const [snapshot, setSnapshot] = useState<string>("Initializing arena MCP session...");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setInterval> | null = null;

    const fetchSnapshot = async () => {
      if (typeof document !== "undefined" && document.visibilityState === "hidden") {
        return;
      }
      try {
        const response = await fetch("/api/arena/snapshot", { cache: "no-store" });
        const payload = (await response.json()) as SnapshotPayload;
        if (!active) return;
        if (!payload.ok) {
          const message = payload.error ?? "Unable to read snapshot.";
          setError((prev) => (prev === message ? prev : message));
          return;
        }
        setError((prev) => (prev === null ? prev : null));
        const nextText = payload.text ?? "(no frame)";
        setSnapshot((prev) => (prev === nextText ? prev : nextText));
      } catch (err) {
        if (!active) return;
        const message = err instanceof Error ? err.message : "Failed to fetch snapshot.";
        setError((prev) => (prev === message ? prev : message));
      }
    };

    fetchSnapshot();
    timer = setInterval(fetchSnapshot, 1500);

    return () => {
      active = false;
      if (timer) clearInterval(timer);
    };
  }, []);

  return (
    <section className="kc-surface-card card card-bordered border-base-300 bg-base-200/90">
      <div className="card-body gap-3">
        <header className="mb-1 flex items-center justify-between gap-3">
          <p className="text-xs font-medium uppercase tracking-wide text-base-content/60">Live text</p>
        </header>
        {error ? <p className="text-xs text-error">{error}</p> : null}
        <pre className="mockup-code overflow-x-auto whitespace-pre-wrap bg-black/70 p-3 text-xs text-base-content">
          {snapshot}
        </pre>
      </div>
    </section>
  );
};

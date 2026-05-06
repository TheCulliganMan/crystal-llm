"use client";

import React, { useEffect, useState, useTransition } from "react";
import { Spawn } from "@pokecrystal/core/engine/world/maps";
import type { Game } from "./game";
import {
  buildVisualDebugSnapshot,
  DEBUG_SCENE_OPTIONS,
  DEBUG_SPAWN_OPTIONS,
  parseVisualDebugScript,
  runVisualDebugScript,
} from "./visual-debug";

const DEFAULT_SCRIPT = JSON.stringify(
  [
    "start",
    { wait: 20 },
    "a",
    { wait: 20 },
    { loop: { count: 3, steps: ["down", { wait: 12 }] } },
  ],
  null,
  2
);

type VisualDebugPanelProps = {
  game: Game | null;
};

export const VisualDebugPanel = React.memo(({ game }: VisualDebugPanelProps) => {
  const [selectedScene, setSelectedScene] = useState(DEBUG_SCENE_OPTIONS[0]?.value ?? "title");
  const [selectedSpawn, setSelectedSpawn] = useState<Spawn>(Spawn.NEW_BARK);
  const [scriptText, setScriptText] = useState(DEFAULT_SCRIPT);
  const [statusText, setStatusText] = useState("Attach to a live game session to inspect runtime state.");
  const [errorText, setErrorText] = useState<string | null>(null);
  const [snapshotText, setSnapshotText] = useState("{}");
  const [isPending, startTransition] = useTransition();

  const refreshSnapshot = () => {
    if (!game) {
      setSnapshotText("{}");
      return;
    }
    const snapshot = buildVisualDebugSnapshot(game);
    setSnapshotText(JSON.stringify(snapshot, null, 2));
  };

  useEffect(() => {
    refreshSnapshot();
    if (!game) {
      return;
    }
    const timerId = window.setInterval(() => {
      refreshSnapshot();
    }, 500);
    return () => window.clearInterval(timerId);
  }, [game]);

  const runAction = (action: () => Promise<void>) => {
    startTransition(() => {
      void (async () => {
        try {
          setErrorText(null);
          await action();
          refreshSnapshot();
        } catch (error) {
          setErrorText(error instanceof Error ? error.message : String(error));
        }
      })();
    });
  };

  return (
    <div className="space-y-4" data-testid="visual-debug-panel">
      <div className="grid gap-3 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
        <section className="card border border-base-300 bg-base-200/60">
          <div className="card-body gap-3">
            <div>
              <p className="text-[0.65rem] uppercase tracking-[0.2em] text-base-content/70">Scene Control</p>
              <h3 className="text-lg font-semibold">Jump runtime state</h3>
            </div>
            <label className="form-control gap-1.5">
              <span className="label-text text-xs uppercase tracking-[0.18em] text-base-content/60">Scene</span>
              <select
                className="select select-bordered"
                value={selectedScene}
                onChange={(event) => setSelectedScene(event.target.value as typeof selectedScene)}
              >
                {DEBUG_SCENE_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </label>
            <button
              type="button"
              className="btn btn-primary"
              disabled={!game || isPending}
              onClick={() =>
                runAction(async () => {
                  if (!game) {
                    return;
                  }
                  await game.debugJumpToScene(selectedScene);
                  setStatusText(`Jumped to ${selectedScene}.`);
                })
              }
            >
              {isPending ? <span className="loading loading-spinner loading-xs mr-1" /> : null}
              Jump To Scene
            </button>

            <label className="form-control gap-1.5">
              <span className="label-text text-xs uppercase tracking-[0.18em] text-base-content/60">Spawn</span>
              <select
                className="select select-bordered"
                value={String(selectedSpawn)}
                onChange={(event) => setSelectedSpawn(Number(event.target.value) as Spawn)}
              >
                {DEBUG_SPAWN_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </label>
            <button
              type="button"
              className="btn btn-outline"
              disabled={!game || isPending}
              onClick={() =>
                runAction(async () => {
                  if (!game) {
                    return;
                  }
                  await game.debugJumpToSpawn(selectedSpawn);
                  setStatusText(`Jumped to spawn ${selectedSpawn}.`);
                })
              }
            >
              {isPending ? <span className="loading loading-spinner loading-xs mr-1" /> : null}
              Jump To Spawn
            </button>

            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                className="btn btn-sm btn-outline"
                disabled={!game || isPending}
                onClick={() =>
                  runAction(async () => {
                    game?.tick();
                    setStatusText("Advanced 1 frame.");
                  })
                }
              >
                Step 1 Frame
              </button>
              <button
                type="button"
                className="btn btn-sm btn-outline"
                disabled={!game || isPending}
                onClick={() =>
                  runAction(async () => {
                    for (let i = 0; i < 10; i += 1) {
                      game?.tick();
                    }
                    setStatusText("Advanced 10 frames.");
                  })
                }
              >
                Step 10 Frames
              </button>
              <button type="button" className="btn btn-sm btn-ghost" onClick={refreshSnapshot}>
                Refresh
              </button>
            </div>
          </div>
        </section>

        <section className="card border border-base-300 bg-base-200/60">
          <div className="card-body gap-3">
            <div>
              <p className="text-[0.65rem] uppercase tracking-[0.2em] text-base-content/70">Script Driver</p>
              <h3 className="text-lg font-semibold">Play through flows</h3>
            </div>
            <textarea
              className="textarea textarea-bordered min-h-52 font-mono text-xs"
              value={scriptText}
              onChange={(event) => setScriptText(event.target.value)}
              spellCheck={false}
            />
            <p className="text-xs text-base-content/70">
              JSON array using tokens like <code>{'"a"'}</code>, <code>{'"up"'}</code>, <code>{`{"wait":12}`}</code>,{" "}
              <code>{`{"loop":{"count":3,"steps":[...]}}`}</code>.
            </p>
            <button
              type="button"
              className="btn btn-secondary"
              disabled={!game || isPending}
              onClick={() =>
                runAction(async () => {
                  if (!game) {
                    return;
                  }
                  const script = parseVisualDebugScript(scriptText);
                  const result = await runVisualDebugScript(game, script);
                  setStatusText(
                    result.complete
                      ? `Script complete in ${result.frames} frames with ${result.events} events.`
                      : `Script stopped after ${result.frames} frames (${result.waiting_reason ?? "pending"}).`
                  );
                })
              }
            >
              {isPending ? <span className="loading loading-spinner loading-xs mr-1" /> : null}
              Run Script
            </button>
          </div>
        </section>
      </div>

      <section className="card border border-base-300 bg-base-200/60">
        <div className="card-body gap-2.5">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div>
              <p className="text-[0.65rem] uppercase tracking-[0.2em] text-base-content/70">Live State</p>
              <h3 className="text-lg font-semibold">Debug snapshot</h3>
            </div>
            <span className="badge badge-outline">{isPending ? "Busy" : "Ready"}</span>
          </div>
          <p className="text-sm text-base-content/80">{statusText}</p>
          {errorText ? <p className="text-sm text-error">{errorText}</p> : null}
          <pre
            data-testid="visual-debug-snapshot"
            className="mockup-code m-0 max-h-[26rem] overflow-auto whitespace-pre-wrap bg-base-100 text-xs text-base-content/75"
          >
            {snapshotText}
          </pre>
        </div>
      </section>
    </div>
  );
});

VisualDebugPanel.displayName = "VisualDebugPanel";

export default VisualDebugPanel;

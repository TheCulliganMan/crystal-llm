"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import type { AudioTestEntry, AudioTestGroup, AudioTestStats } from "./catalog";

type AudioFilter = AudioTestGroup | "all";

type AudioTestClientProps = {
  entries: AudioTestEntry[];
  stats: AudioTestStats;
};

const FILTERS: Array<{ id: AudioFilter; label: string }> = [
  { id: "all", label: "All" },
  { id: "music", label: "Music" },
  { id: "sfx", label: "SFX" },
  { id: "cry", label: "Cries" },
];

const GROUP_LABELS: Record<AudioTestGroup, string> = {
  music: "Music",
  sfx: "SFX",
  cry: "Cry",
};

const GROUP_BADGE_CLASSES: Record<AudioTestGroup, string> = {
  music: "border-emerald-400/40 bg-emerald-500/10 text-emerald-100",
  sfx: "border-amber-400/40 bg-amber-500/10 text-amber-100",
  cry: "border-rose-400/40 bg-rose-500/10 text-rose-100",
};

const clampVolume = (value: number): number =>
  Number.isFinite(value) ? Math.max(0, Math.min(1, value)) : 0.6;

const supportsDirectPcm = (): boolean => {
  if (typeof window === "undefined") {
    return false;
  }
  const audioWindow = window as Window & {
    AudioContext?: typeof AudioContext;
    webkitAudioContext?: typeof AudioContext;
    AudioWorkletNode?: typeof AudioWorkletNode;
  };
  const ContextCtor = audioWindow.AudioContext ?? audioWindow.webkitAudioContext;
  if (!ContextCtor) {
    return false;
  }
  return "audioWorklet" in ContextCtor.prototype || typeof audioWindow.AudioWorkletNode === "function";
};

const searchTextFor = (entry: AudioTestEntry): string =>
  `${entry.group} ${entry.token} ${entry.title} ${entry.source} ${entry.stem} ${entry.detail}`.toLowerCase();

export function AudioTestClient({ entries, stats }: AudioTestClientProps) {
  const engineRef = useRef<AudioEngine | null>(null);
  const initialVolumeRef = useRef(0.6);
  const initialMutedRef = useRef(false);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<AudioFilter>("all");
  const [volume, setVolume] = useState(initialVolumeRef.current);
  const [muted, setMuted] = useState(initialMutedRef.current);
  const [ready, setReady] = useState(false);
  const [pcmSupported, setPcmSupported] = useState<boolean | null>(null);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [status, setStatus] = useState("Idle");

  useEffect(() => {
    setPcmSupported(supportsDirectPcm());
    const engine = new AudioEngine({
      playbackBackend: "direct-pcm",
      masterVolume: initialVolumeRef.current,
      muted: initialMutedRef.current,
    });
    for (const entry of entries) {
      if (entry.group === "music") {
        engine.loadMusic(entry.token, entry.source);
      } else {
        engine.loadSound(entry.token, entry.source);
      }
    }
    engineRef.current = engine;
    setReady(true);
    return () => {
      engine.dispose();
      if (engineRef.current === engine) {
        engineRef.current = null;
      }
    };
  }, [entries]);

  useEffect(() => {
    engineRef.current?.setMasterVolume(volume);
  }, [volume]);

  useEffect(() => {
    engineRef.current?.setMuted(muted);
  }, [muted]);

  const filteredEntries = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return entries.filter((entry) => {
      if (filter !== "all" && entry.group !== filter) {
        return false;
      }
      if (!normalizedQuery) {
        return true;
      }
      return searchTextFor(entry).includes(normalizedQuery);
    });
  }, [entries, filter, query]);

  const activeEntry = activeId
    ? entries.find((entry) => entry.id === activeId) ?? null
    : null;

  const playEntry = (entry: AudioTestEntry): void => {
    const engine = engineRef.current;
    if (!engine) {
      setStatus("Audio unavailable");
      return;
    }
    try {
      engine.setMasterVolume(volume);
      if (muted) {
        engine.setMuted(false);
        setMuted(false);
      }
      engine.unlock();
      if (entry.group === "music") {
        engine.loadMusic(entry.token, entry.source);
        engine.playMusic(entry.token, "audio-test");
      } else {
        engine.loadSound(entry.token, entry.source);
        engine.playSound(entry.token);
      }
      setActiveId(entry.id);
      setStatus(entry.token);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Playback failed");
    }
  };

  const stopMusic = (): void => {
    engineRef.current?.stopMusic();
    if (activeEntry?.group === "music") {
      setActiveId(null);
    }
    setStatus("Music stopped");
  };

  const stopAll = (): void => {
    engineRef.current?.channelsOff();
    setActiveId(null);
    setStatus("Stopped");
  };

  const playRelative = (delta: number): void => {
    if (filteredEntries.length === 0) {
      return;
    }
    const currentIndex = activeId
      ? filteredEntries.findIndex((entry) => entry.id === activeId)
      : -1;
    const nextIndex = currentIndex < 0
      ? (delta < 0 ? filteredEntries.length - 1 : 0)
      : (currentIndex + delta + filteredEntries.length) % filteredEntries.length;
    playEntry(filteredEntries[nextIndex]);
  };

  const volumePercent = Math.round(volume * 100);
  const supportLabel = pcmSupported === null ? "Checking" : pcmSupported ? "Ready" : "Unavailable";

  return (
    <div data-testid="audio-test-client" className="flex min-h-full flex-col bg-zinc-950 text-zinc-100">
      <section className="border-b border-zinc-800 bg-zinc-950 px-4 py-4 sm:px-6">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
          <div className="min-w-0">
            <p className="text-xs font-semibold uppercase tracking-wide text-emerald-300">Direct PCM</p>
            <h1 className="mt-1 text-3xl font-semibold tracking-normal text-white">Audio Test</h1>
            <div className="mt-3 flex flex-wrap gap-2 text-xs text-zinc-300">
              <span className="rounded border border-zinc-700 px-2 py-1">{stats.music} music</span>
              <span className="rounded border border-zinc-700 px-2 py-1">{stats.sfx} SFX</span>
              <span className="rounded border border-zinc-700 px-2 py-1">{stats.cry} cries</span>
              <span className="rounded border border-zinc-700 px-2 py-1">{entries.length} total</span>
            </div>
          </div>

          <div className="grid gap-3 lg:grid-cols-[minmax(18rem,28rem)_auto_auto] lg:items-end">
            <label className="block min-w-0">
              <span className="mb-1 block text-xs font-medium uppercase tracking-wide text-zinc-400">Search</span>
              <input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                className="h-10 w-full rounded border border-zinc-700 bg-zinc-900 px-3 text-sm text-zinc-100 outline-none transition focus:border-emerald-400"
                aria-label="Search audio"
                type="search"
              />
            </label>

            <div>
              <span className="mb-1 block text-xs font-medium uppercase tracking-wide text-zinc-400">Group</span>
              <div className="grid grid-cols-4 overflow-hidden rounded border border-zinc-700">
                {FILTERS.map((item) => (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => setFilter(item.id)}
                    className={`h-10 min-w-16 px-3 text-sm transition ${
                      filter === item.id
                        ? "bg-emerald-500 text-zinc-950"
                        : "bg-zinc-900 text-zinc-200 hover:bg-zinc-800"
                    }`}
                  >
                    {item.label}
                  </button>
                ))}
              </div>
            </div>

            <div className="grid grid-cols-[auto_auto] gap-2">
              <button
                type="button"
                onClick={() => playRelative(-1)}
                disabled={!ready || filteredEntries.length === 0}
                className="h-10 rounded border border-zinc-700 bg-zinc-900 px-3 text-sm text-zinc-100 transition hover:border-zinc-500 disabled:cursor-not-allowed disabled:opacity-40"
              >
                Previous
              </button>
              <button
                type="button"
                onClick={() => playRelative(1)}
                disabled={!ready || filteredEntries.length === 0}
                className="h-10 rounded border border-emerald-400/60 bg-emerald-500 px-3 text-sm font-semibold text-zinc-950 transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:opacity-40"
              >
                Next
              </button>
            </div>
          </div>
        </div>
      </section>

      <section className="border-b border-zinc-800 bg-zinc-900/70 px-4 py-3 sm:px-6">
        <div className="grid gap-3 lg:grid-cols-[minmax(16rem,24rem)_auto_auto_1fr] lg:items-center">
          <label className="grid grid-cols-[auto_1fr_auto] items-center gap-3 text-sm text-zinc-200">
            <span className="font-medium">Volume</span>
            <input
              type="range"
              min="0"
              max="100"
              value={volumePercent}
              onChange={(event) => setVolume(clampVolume(Number(event.target.value) / 100))}
              className="h-8 min-w-32 accent-emerald-400"
              aria-label="Master volume"
            />
            <span className="w-10 text-right font-mono text-xs text-zinc-300">{volumePercent}%</span>
          </label>

          <label className="flex h-10 items-center gap-2 rounded border border-zinc-700 bg-zinc-950 px-3 text-sm text-zinc-200">
            <input
              type="checkbox"
              checked={muted}
              onChange={(event) => setMuted(event.target.checked)}
              className="h-4 w-4 accent-emerald-400"
            />
            Mute
          </label>

          <div className="grid grid-cols-2 gap-2">
            <button
              type="button"
              onClick={stopMusic}
              className="h-10 rounded border border-zinc-700 bg-zinc-950 px-3 text-sm text-zinc-100 transition hover:border-zinc-500"
            >
              Stop Music
            </button>
            <button
              type="button"
              onClick={stopAll}
              className="h-10 rounded border border-red-400/50 bg-red-500/10 px-3 text-sm text-red-100 transition hover:bg-red-500/20"
            >
              Stop All
            </button>
          </div>

          <div className="min-w-0 rounded border border-zinc-700 bg-zinc-950 px-3 py-2 text-sm">
            <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
              <span className="text-zinc-400">PCM</span>
              <span className={pcmSupported ? "text-emerald-300" : "text-amber-200"}>{supportLabel}</span>
              <span className="text-zinc-400">Status</span>
              <span className="min-w-0 truncate font-mono text-zinc-100" aria-live="polite">{status}</span>
              {activeEntry ? (
                <>
                  <span className="text-zinc-400">Active</span>
                  <span className="min-w-0 truncate font-mono text-zinc-100">{activeEntry.token}</span>
                </>
              ) : null}
            </div>
          </div>
        </div>
      </section>

      <section className="min-h-0 flex-1 overflow-hidden">
        <div className="h-full overflow-auto">
          <table className="min-w-[920px] table-fixed border-collapse text-left text-sm">
            <thead className="sticky top-0 z-10 border-b border-zinc-800 bg-zinc-950 text-xs uppercase tracking-wide text-zinc-400">
              <tr>
                <th className="w-20 px-4 py-3 font-semibold sm:px-6">Play</th>
                <th className="w-24 px-3 py-3 font-semibold">Type</th>
                <th className="w-72 px-3 py-3 font-semibold">Token</th>
                <th className="w-72 px-3 py-3 font-semibold">Name</th>
                <th className="w-80 px-3 py-3 font-semibold">Program</th>
                <th className="w-96 px-3 py-3 font-semibold">Detail</th>
              </tr>
            </thead>
            <tbody>
              {filteredEntries.map((entry) => {
                const active = activeId === entry.id;
                return (
                  <tr
                    key={entry.id}
                    data-testid={`audio-test-row-${entry.id}`}
                    className={`border-b border-zinc-900 transition ${
                      active ? "bg-emerald-500/10" : "bg-zinc-950 hover:bg-zinc-900/80"
                    }`}
                  >
                    <td className="px-4 py-2 sm:px-6">
                      <button
                        type="button"
                        onClick={() => playEntry(entry)}
                        disabled={!ready}
                        className="h-9 w-14 rounded border border-zinc-700 bg-zinc-900 text-sm font-medium text-zinc-100 transition hover:border-emerald-400 hover:text-emerald-100 disabled:cursor-not-allowed disabled:opacity-40"
                      >
                        Play
                      </button>
                    </td>
                    <td className="px-3 py-2">
                      <span className={`inline-flex h-7 min-w-16 items-center justify-center rounded border px-2 text-xs font-semibold ${GROUP_BADGE_CLASSES[entry.group]}`}>
                        {GROUP_LABELS[entry.group]}
                      </span>
                    </td>
                    <td className="truncate px-3 py-2 font-mono text-xs text-zinc-200">{entry.token}</td>
                    <td className="truncate px-3 py-2 text-zinc-100">{entry.title}</td>
                    <td className="truncate px-3 py-2 font-mono text-xs text-zinc-300">{entry.source}</td>
                    <td className="truncate px-3 py-2 text-zinc-300">{entry.detail}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>

          {filteredEntries.length === 0 ? (
            <div className="px-6 py-10 text-sm text-zinc-400">No matches</div>
          ) : null}
        </div>
      </section>
    </div>
  );
}

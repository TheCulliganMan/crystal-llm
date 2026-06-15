import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { parseJsonText } from "./client";
import type { ToolResult } from "./types";

export type TuiAudioPlaybackEvent = {
  sequence: number;
  kind: "music" | "sfx" | "cry" | "other";
  token: string;
  source: string;
  role?: string;
  loop?: boolean;
};

export type TuiAudioPlaybackSnapshot = {
  musicToken?: string | null;
  musicRole?: string;
  musicSource?: string | null;
  recentEvents?: TuiAudioPlaybackEvent[];
};

export type TuiAudioPlayerInput = {
  source: string;
  pcm: Int16Array;
  sampleRate: number;
  loopStartSample?: number | null;
  loopEndSample?: number | null;
  token: string;
  kind: TuiAudioPlaybackEvent["kind"];
  loop: boolean;
};

export type TuiAudioPlayerHandle = {
  kill?: () => void;
};

export type TuiSoundController = {
  setEnabled: (enabled: boolean) => void;
  isEnabled: () => boolean;
  syncSnapshot: (snapshot?: TuiAudioPlaybackSnapshot | null) => void;
  close?: () => void;
};

const repoRoot = path.resolve(__dirname, "..", "..", "..");
const PCM_STREAM_CHUNK_MS = 80;

export type TuiPcmClip = {
  pcm: Int16Array;
  sampleRate: number;
  loopStartSample?: number | null;
  loopEndSample?: number | null;
};

type PcmToolchain = {
  compileAsmAudioProgramToPcmJson: (
    audioRoot: string,
    kind: "music" | "sfx" | "cry",
    stem: string,
    token: string,
  ) => string | null;
  renderPcmClipFromJson: (jsonText: string) => TuiPcmClip;
  getDisassemblyRoot: () => string;
};

let cachedPcmToolchain: PcmToolchain | null = null;

const loadPcmToolchain = (): PcmToolchain => {
  if (cachedPcmToolchain) {
    return cachedPcmToolchain;
  }
  // Dynamic require keeps the CLI package build decoupled from core source rootDir rules.
  // The published CLI depends on @pokecrystal/core at runtime.
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const pcm = require("@pokecrystal/core/audio-export/pcm-clip") as Pick<
    PcmToolchain,
    "compileAsmAudioProgramToPcmJson" | "renderPcmClipFromJson"
  >;
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const paths = require("@pokecrystal/core/core/paths") as Pick<PcmToolchain, "getDisassemblyRoot">;
  cachedPcmToolchain = {
    compileAsmAudioProgramToPcmJson: pcm.compileAsmAudioProgramToPcmJson,
    renderPcmClipFromJson: pcm.renderPcmClipFromJson,
    getDisassemblyRoot: paths.getDisassemblyRoot,
  };
  return cachedPcmToolchain;
};

const candidateAudioRoots = (): string[] => [
  ...(process.env.POKECRYSTAL_CLI_AUDIO_ROOT ? [process.env.POKECRYSTAL_CLI_AUDIO_ROOT] : []),
  path.join(repoRoot, "apps", "web", "assets", "audio"),
  path.join(repoRoot, "apps", "web", "public", "assets", "audio"),
  path.join(repoRoot, "apps", "web", ".next-desktop", "assets", "audio"),
];

const fileExists = (filePath: string): boolean => {
  try {
    return fs.statSync(filePath).isFile();
  } catch {
    return false;
  }
};

const sourceCandidates = (source: string): string[] => {
  const normalized = source.trim();
  if (!normalized) {
    return [];
  }
  const apiAudio = normalized.match(/^\/api\/audio\/(.+)$/);
  const assetAudio = normalized.match(/^\/assets\/audio\/(.+)$/);
  const rootRelativeAudio = normalized.match(/^\/audio\/(.+)$/);
  const relative =
    apiAudio?.[1] ??
    assetAudio?.[1] ??
    rootRelativeAudio?.[1] ??
    normalized.replace(/^assets\/audio\//, "");
  if (relative === normalized && path.isAbsolute(normalized)) {
    return [normalized];
  }
  return candidateAudioRoots().map((root) => path.join(root, relative));
};

export const resolveTuiAudioSourcePath = (source: string): string | null =>
  sourceCandidates(source).find(fileExists) ?? null;

const commandExists = (command: string): boolean => {
  const result = spawnSync("sh", ["-c", `command -v ${JSON.stringify(command)} >/dev/null 2>&1`], {
    stdio: "ignore",
  });
  return result.status === 0;
};

const resolvePcmPlayerCommand = (): string[] | null => {
  const configured = process.env.POKECRYSTAL_CLI_PCM_PLAYER?.trim();
  if (configured) {
    return configured.split(/\s+/);
  }
  const candidates = [
    ["ffplay", "-f", "s16le", "-ar", "{sampleRate}", "-ac", "2", "-nodisp", "-autoexit", "-loglevel", "quiet", "-"],
    ["play", "-q", "-t", "s16", "-r", "{sampleRate}", "-c", "2", "-"],
    ["aplay", "-q", "-f", "S16_LE", "-r", "{sampleRate}", "-c", "2"],
    ["paplay", "--raw", "--rate={sampleRate}", "--channels=2", "--format=s16le"],
  ];
  return candidates.find((candidate) => commandExists(candidate[0] ?? "")) ?? null;
};

const spawnSinglePcmPlayer = (
  sampleRate: number,
  playerCommand: string[] | null,
): ChildProcess | null => {
  const [command, ...baseArgs] = playerCommand ?? [];
  if (!command) {
    return null;
  }
  const args = baseArgs.map((arg) => arg.replace(/\{sampleRate\}/g, String(sampleRate)));
  try {
    const child = spawn(command, args, {
      detached: false,
      stdio: ["pipe", "ignore", "ignore"],
    });
    child.once("error", () => undefined);
    return child;
  } catch {
    return null;
  }
};

const pcmFramesToBuffer = (pcm: Int16Array, startFrame: number, frameCount: number): Buffer => {
  const frames = Math.max(0, frameCount);
  const buffer = Buffer.alloc(frames * 4);
  for (let frame = 0; frame < frames; frame += 1) {
    const sampleIndex = (startFrame + frame) * 2;
    buffer.writeInt16LE(pcm[sampleIndex] ?? 0, frame * 4);
    buffer.writeInt16LE(pcm[sampleIndex + 1] ?? 0, frame * 4 + 2);
  }
  return buffer;
};

const spawnPcmStreamPlayer = (
  input: TuiAudioPlayerInput,
  playerCommand: string[] | null,
): TuiAudioPlayerHandle | null => {
  const child = spawnSinglePcmPlayer(input.sampleRate, playerCommand);
  if (!child?.stdin) {
    return null;
  }
  let stopped = false;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let cursor = 0;
  const totalFrames = Math.floor(input.pcm.length / 2);
  const loopStart = Math.max(0, Math.min(totalFrames, input.loopStartSample ?? 0));
  const loopEnd = Math.max(loopStart + 1, Math.min(totalFrames, input.loopEndSample ?? totalFrames));
  const chunkFrames = Math.max(1, Math.round((input.sampleRate * PCM_STREAM_CHUNK_MS) / 1000));
  child.once("exit", () => {
    stopped = true;
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
  });
  child.once("error", () => {
    stopped = true;
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
  });

  const schedule = (): void => {
    if (stopped) {
      return;
    }
    timer = setTimeout(writeChunk, PCM_STREAM_CHUNK_MS);
  };

  const writeChunk = (): void => {
    timer = null;
    if (stopped || totalFrames <= 0) {
      return;
    }
    let remaining = chunkFrames;
    const buffers: Buffer[] = [];
    while (remaining > 0 && !stopped) {
      if (cursor >= totalFrames) {
        if (!input.loop) {
          child.stdin?.end();
          stopped = true;
          break;
        }
        cursor = loopStart;
      }
      const end = input.loop ? loopEnd : totalFrames;
      const take = Math.min(remaining, Math.max(0, end - cursor));
      if (take <= 0) {
        if (!input.loop) {
          child.stdin?.end();
          stopped = true;
          break;
        }
        cursor = loopStart;
        continue;
      }
      buffers.push(pcmFramesToBuffer(input.pcm, cursor, take));
      cursor += take;
      remaining -= take;
      if (input.loop && cursor >= loopEnd) {
        cursor = loopStart;
      }
    }
    if (buffers.length === 0 || stopped) {
      return;
    }
    const chunk = Buffer.concat(buffers);
    if (!child.stdin?.write(chunk)) {
      child.stdin?.once("drain", schedule);
      return;
    }
    schedule();
  };

  writeChunk();
  return {
    kill: () => {
      stopped = true;
      if (timer) {
        clearTimeout(timer);
        timer = null;
      }
      child.kill();
    },
  };
};

const maxSequence = (snapshot?: TuiAudioPlaybackSnapshot | null): number =>
  Math.max(0, ...(snapshot?.recentEvents ?? []).map((event) => event.sequence).filter(Number.isFinite));

const isPlaybackEvent = (value: unknown): value is TuiAudioPlaybackEvent => {
  if (!value || typeof value !== "object") {
    return false;
  }
  const event = value as Partial<TuiAudioPlaybackEvent>;
  return (
    typeof event.sequence === "number" &&
    typeof event.token === "string" &&
    typeof event.source === "string" &&
    (event.kind === "music" || event.kind === "sfx" || event.kind === "cry" || event.kind === "other")
  );
};

export const extractTuiAudioPlaybackSnapshot = (result?: ToolResult): TuiAudioPlaybackSnapshot | null => {
  const payload = parseJsonText(result?.content).find((entry) => entry.audio && typeof entry.audio === "object");
  const audio = payload?.audio as Record<string, unknown> | undefined;
  if (!audio) {
    return null;
  }
  const recentEvents = Array.isArray(audio.recentEvents)
    ? audio.recentEvents.filter(isPlaybackEvent)
    : [];
  return {
    musicToken: typeof audio.musicToken === "string" ? audio.musicToken : null,
    musicRole: typeof audio.musicRole === "string" ? audio.musicRole : undefined,
    musicSource: typeof audio.musicSource === "string" ? audio.musicSource : null,
    recentEvents,
  };
};

const normalizeStem = (value: string): string =>
  value.replace(/[^A-Za-z0-9]/g, "").toLowerCase();

const parseSourceStem = (event: TuiAudioPlaybackEvent): { kind: "music" | "sfx" | "cry"; stem: string } | null => {
  const source = event.source.trim();
  const direct = source.match(/\/api\/audio\/pcm\/(music|sfx|cries)\/([^/.]+)(?:\.json)?(?:\/ch\d+\.pcm)?$/);
  if (direct) {
    return {
      kind: direct[1] === "cries" ? "cry" : direct[1] as "music" | "sfx",
      stem: normalizeStem(direct[2]),
    };
  }
  const legacy = source.match(/\/api\/audio\/(?:(sfx|cries)\/)?([^/.]+)\.(?:mp3|wav|pcm|json)$/);
  if (legacy) {
    const group = legacy[1];
    return {
      kind: group === "cries" ? "cry" : group === "sfx" ? "sfx" : event.kind === "music" ? "music" : event.kind === "cry" ? "cry" : "sfx",
      stem: normalizeStem(legacy[2]),
    };
  }
  if (event.kind === "music" || event.kind === "sfx" || event.kind === "cry") {
    return {
      kind: event.kind === "cry" ? "cry" : event.kind,
      stem: normalizeStem(path.basename(source, path.extname(source)) || event.token),
    };
  }
  return null;
};

const createPcmClipResolver = (): ((event: TuiAudioPlaybackEvent) => TuiPcmClip | null) => {
  const cache = new Map<string, TuiPcmClip | null>();
  return (event) => {
    const parsed = parseSourceStem(event);
    if (!parsed) {
      return null;
    }
    const cacheKey = `${parsed.kind}:${parsed.stem}`;
    if (cache.has(cacheKey)) {
      return cache.get(cacheKey) ?? null;
    }
    try {
      const toolchain = loadPcmToolchain();
      const audioRoot = path.join(toolchain.getDisassemblyRoot(), "audio");
      const audioJson = toolchain.compileAsmAudioProgramToPcmJson(
        audioRoot,
        parsed.kind,
        parsed.stem,
        event.token,
      );
      const clip = audioJson ? toolchain.renderPcmClipFromJson(audioJson) : null;
      cache.set(cacheKey, clip);
      return clip;
    } catch {
      cache.set(cacheKey, null);
      return null;
    }
  };
};

export const createTuiSoundController = (options: {
  stdout?: Pick<NodeJS.WriteStream, "write">;
  enabled?: boolean;
  playerCommand?: string[] | null;
  player?: (input: TuiAudioPlayerInput) => TuiAudioPlayerHandle | null | undefined;
  pcmResolver?: (event: TuiAudioPlaybackEvent) => TuiPcmClip | null | undefined;
} = {}): TuiSoundController => {
  let enabled = options.enabled ?? false;
  let lastEventSequence = 0;
  let activeMusicToken: string | null = null;
  let activeMusicSource: string | null = null;
  let activeMusicHandle: TuiAudioPlayerHandle | null = null;
  const stdout = options.stdout;
  const playerCommand = options.playerCommand === undefined ? resolvePcmPlayerCommand() : options.playerCommand;
  const resolvePcmClip = options.pcmResolver ?? createPcmClipResolver();
  const player =
    options.player ??
    ((input: TuiAudioPlayerInput): TuiAudioPlayerHandle | null =>
      spawnPcmStreamPlayer(input, playerCommand));

  const ringBell = (): void => {
    stdout?.write?.("\u0007");
  };

  const stopMusic = (): void => {
    activeMusicHandle?.kill?.();
    activeMusicHandle = null;
    activeMusicToken = null;
    activeMusicSource = null;
  };

  const playSource = (event: TuiAudioPlaybackEvent, loop: boolean): TuiAudioPlayerHandle | null => {
    const clip = resolvePcmClip(event);
    if (!clip) {
      ringBell();
      return null;
    }
    const handle = player({
      source: event.source,
      pcm: clip.pcm,
      sampleRate: clip.sampleRate,
      loopStartSample: clip.loopStartSample,
      loopEndSample: clip.loopEndSample,
      token: event.token,
      kind: event.kind,
      loop,
    }) ?? null;
    if (!handle) {
      ringBell();
    }
    return handle;
  };

  const syncMusicState = (snapshot: TuiAudioPlaybackSnapshot): void => {
    const token = snapshot.musicToken ?? null;
    const source = snapshot.musicSource ?? null;
    if (!token || !source) {
      stopMusic();
      return;
    }
    if (activeMusicToken === token && activeMusicSource === source) {
      return;
    }
    stopMusic();
    activeMusicHandle = playSource({
      sequence: lastEventSequence,
      kind: "music",
      token,
      source,
      role: snapshot.musicRole,
      loop: true,
    }, true);
    activeMusicToken = activeMusicHandle ? token : null;
    activeMusicSource = activeMusicHandle ? source : null;
  };

  const playNewEvents = (snapshot: TuiAudioPlaybackSnapshot): void => {
    const events = [...(snapshot.recentEvents ?? [])]
      .filter((event) => event.sequence > lastEventSequence)
      .sort((left, right) => left.sequence - right.sequence);
    for (const event of events) {
      lastEventSequence = Math.max(lastEventSequence, event.sequence);
      if (event.kind === "music") {
        stopMusic();
        activeMusicHandle = playSource(event, event.loop ?? true);
        activeMusicToken = activeMusicHandle ? event.token : null;
        activeMusicSource = activeMusicHandle ? event.source : null;
        continue;
      }
      playSource(event, false);
    }
  };

  return {
    setEnabled: (next) => {
      enabled = next;
      if (!enabled) {
        stopMusic();
      }
    },
    isEnabled: () => enabled,
    syncSnapshot: (snapshot) => {
      if (!snapshot) {
        return;
      }
      if (!enabled) {
        lastEventSequence = Math.max(lastEventSequence, maxSequence(snapshot));
        stopMusic();
        return;
      }
      playNewEvents(snapshot);
      syncMusicState(snapshot);
    },
    close: stopMusic,
  };
};

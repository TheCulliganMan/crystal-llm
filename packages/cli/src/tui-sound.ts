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
  filePath: string;
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

const candidateAudioRoots = (): string[] => [
  ...(process.env.POKECRYSTAL_CLI_AUDIO_ROOT ? [process.env.POKECRYSTAL_CLI_AUDIO_ROOT] : []),
  path.join(repoRoot, "apps", "web", "assets", "audio"),
  path.join(repoRoot, "apps", "web", "public", "assets", "audio"),
  path.join(repoRoot, "apps", "web", ".next-electron", "standalone", "apps", "web", "assets", "audio"),
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

const resolvePlayerCommand = (): string[] | null => {
  const configured = process.env.POKECRYSTAL_CLI_AUDIO_PLAYER?.trim();
  if (configured) {
    return configured.split(/\s+/);
  }
  const candidates =
    process.platform === "darwin"
      ? [["afplay"]]
      : [
          ["mpg123", "-q"],
          ["mpv", "--no-video", "--really-quiet", "--no-terminal"],
          ["ffplay", "-nodisp", "-autoexit", "-loglevel", "quiet"],
          ["play", "-q"],
          ["paplay"],
        ];
  return candidates.find((candidate) => commandExists(candidate[0] ?? "")) ?? null;
};

const spawnSingleAudioPlayer = (
  filePath: string,
  playerCommand: string[] | null,
): ChildProcess | null => {
  const [command, ...baseArgs] = playerCommand ?? [];
  if (!command) {
    return null;
  }
  const args = baseArgs.map((arg) => (arg === "{file}" ? filePath : arg));
  if (!baseArgs.includes("{file}")) {
    args.push(filePath);
  }
  try {
    const child = spawn(command, args, {
      detached: false,
      stdio: "ignore",
    });
    child.once("error", () => undefined);
    return child;
  } catch {
    return null;
  }
};

const spawnAudioPlayer = (
  input: TuiAudioPlayerInput,
  playerCommand: string[] | null,
): TuiAudioPlayerHandle | null => {
  const startChild = (): ChildProcess | null => spawnSingleAudioPlayer(input.filePath, playerCommand);
  if (!input.loop) {
    const child = startChild();
    return child ? { kill: () => child.kill() } : null;
  }

  let stopped = false;
  let currentChild: ChildProcess | null = null;
  let restartTimer: ReturnType<typeof setTimeout> | null = null;

  const startLoop = (): void => {
    if (stopped) {
      return;
    }
    const child = startChild();
    if (!child) {
      return;
    }
    currentChild = child;
    let handled = false;
    const restart = (): void => {
      if (handled || stopped) {
        return;
      }
      handled = true;
      restartTimer = setTimeout(() => {
        restartTimer = null;
        startLoop();
      }, 25);
    };
    child.once("exit", restart);
    child.once("error", restart);
  };

  startLoop();
  if (!currentChild) {
    return null;
  }
  return {
    kill: () => {
      stopped = true;
      if (restartTimer) {
        clearTimeout(restartTimer);
        restartTimer = null;
      }
      currentChild?.kill();
      currentChild = null;
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

export const createTuiSoundController = (options: {
  stdout?: Pick<NodeJS.WriteStream, "write">;
  enabled?: boolean;
  playerCommand?: string[] | null;
  player?: (input: TuiAudioPlayerInput) => TuiAudioPlayerHandle | null | undefined;
} = {}): TuiSoundController => {
  let enabled = options.enabled ?? false;
  let lastEventSequence = 0;
  let activeMusicToken: string | null = null;
  let activeMusicSource: string | null = null;
  let activeMusicHandle: TuiAudioPlayerHandle | null = null;
  const stdout = options.stdout;
  const playerCommand = options.playerCommand === undefined ? resolvePlayerCommand() : options.playerCommand;
  const player =
    options.player ??
    ((input: TuiAudioPlayerInput): TuiAudioPlayerHandle | null =>
      spawnAudioPlayer(input, playerCommand));

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
    const filePath = resolveTuiAudioSourcePath(event.source);
    if (!filePath) {
      ringBell();
      return null;
    }
    const handle = player({
      filePath,
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

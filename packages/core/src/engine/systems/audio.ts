import * as fs from "fs";
import * as path from "path";
import { getAssetPath, getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { DISASSEMBLY_MUSIC_ALIASES, DISASSEMBLY_SFX_ALIASES } from "./audio-aliases";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import {
  pcmBytesToInt16,
  type PcmAudioManifest,
  type PcmClipManifest,
  type PcmMusicTrackManifest,
} from "@pokecrystal/core/audio-export/pcm-clip";

const FRAME_MS = GB_FRAME_DURATION_MS;

type DisassemblyAliases = {
  music: Record<string, string>;
  sfx: Record<string, string>;
  sfxPriority: Record<string, number>;
};

type SfxReleaseState = {
  released: boolean;
  release: () => void;
};

type CryTableEntry = { cry?: string | null };
type BattlePanDirection = "left" | "right" | "player" | "enemy" | "center";
type BattleSoundOptions = {
  tracks?: number | null;
  duration?: number | null;
  panning?: string | null;
  pitch?: number | null;
};
type ResolvedBattleSoundOptions = {
  tracks: number | null;
  duration: number | null;
  panning: BattlePanDirection | null;
  pitch: number | null;
};
type PendingSound = {
  token: string;
  source: string;
  options: ResolvedBattleSoundOptions;
  priorityClass?: "none" | "priority" | "cry";
};
type SoundChannelCategory = "sfx" | "cry" | "other";
type ManifestKind = "music" | "sfx" | "cry";

export interface MusicStemManifest {
  channel: number;
  path: string;
  loop?: boolean;
  pan?: [boolean, boolean];
}

export interface MusicTrackManifest {
  kind: "music";
  token: string;
  mixedPath: string;
  channelCount: number;
  loop?: boolean;
  loopStartFrame?: number | null;
  loopStartSeconds?: number | null;
  stems: MusicStemManifest[];
}

export interface SoundCueManifest {
  kind: "sfx" | "cry";
  token: string;
  assetPath: string;
  ownedChannels: number[];
  durationFrames?: number | null;
  priorityClass?: "none" | "priority" | "cry";
}

export interface AudioBundleManifest {
  music: Record<string, string>;
  sounds: Record<string, string>;
}

export interface ActiveChannelState {
  channel: number;
  ownerToken: string;
  category: SoundChannelCategory | "music";
  role?: string;
}

export interface AudioPlaybackEvent {
  sequence: number;
  kind: "music" | "sfx" | "cry" | "other";
  token: string;
  source: string;
  role?: string;
  loop?: boolean;
}

export interface AudioPlaybackSnapshot {
  musicToken: string | null;
  musicRole: string;
  musicSource: string | null;
  musicFrame: number;
  fadedVolume: number;
  activeChannels: ActiveChannelState[];
  recentEvents: AudioPlaybackEvent[];
}

type AnyAudioManifest = MusicTrackManifest | SoundCueManifest | PcmAudioManifest;

type AudioPlaybackBackendKind = "auto" | "direct-pcm" | "html";

type AudioHandle = {
  src: string;
  loop: boolean;
  volume: number;
  muted: boolean;
  paused: boolean;
  ended: boolean;
  currentTime: number;
  playbackRate?: number;
  play: () => Promise<void> | void;
  pause: () => void;
  addEventListener: (event: string, listener: () => void, options?: { once?: boolean }) => void;
};

type MusicPlaybackState = {
  source: string;
  stems: Map<number, AudioHandle>;
  mixed: AudioHandle | null;
  manifest: MusicTrackManifest | PcmMusicTrackManifest | null;
  frameCursor: number;
  gain: number;
  role: string;
  token: string;
};

export type PcmBackendVoice = {
  id: number;
  kind: "music" | "sfx" | "cry" | "other";
  token: string;
  source: string;
  pcm: Int16Array;
  sampleRate: number;
  loop: boolean;
  loopStartSample: number | null;
  loopEndSample: number | null;
  volume: number;
  muted: boolean;
  pan: number | null;
  playbackRate: number;
};

export interface PcmAudioPlaybackBackend {
  playVoice(voice: PcmBackendVoice, onEnded?: () => void): void;
  stopVoice(id: number): void;
  updateVoice(id: number, patch: Partial<Pick<PcmBackendVoice, "volume" | "muted" | "pan" | "playbackRate">>): void;
  resume(): Promise<void>;
  dispose(): void;
}

const PCM_WORKLET_SOURCE = `
class PokeCrystalPcmProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.voices = new Map();
    this.port.onmessage = (event) => {
      const message = event.data || {};
      if (message.type === "play") {
        const voice = message.voice;
        this.voices.set(voice.id, {
          ...voice,
          pcm: new Int16Array(voice.pcm),
          cursor: 0
        });
        return;
      }
      if (message.type === "stop") {
        this.voices.delete(message.id);
        return;
      }
      if (message.type === "update") {
        const voice = this.voices.get(message.id);
        if (voice) {
          Object.assign(voice, message.patch || {});
        }
        return;
      }
      if (message.type === "clear") {
        this.voices.clear();
      }
    };
  }

  process(_inputs, outputs) {
    const output = outputs[0];
    const left = output[0];
    const right = output[1] || output[0];
    left.fill(0);
    if (right !== left) {
      right.fill(0);
    }
    const ended = [];
    for (const voice of this.voices.values()) {
      if (voice.muted || voice.volume <= 0 || voice.pcm.length < 2) {
        continue;
      }
      const frames = Math.floor(voice.pcm.length / 2);
      const loopStart = Math.max(0, voice.loopStartSample == null ? 0 : voice.loopStartSample);
      const loopEnd = Math.max(loopStart + 1, Math.min(frames, voice.loopEndSample == null ? frames : voice.loopEndSample));
      const rate = Math.max(0.05, voice.playbackRate || 1) * (voice.sampleRate / sampleRate);
      const pan = Math.max(-1, Math.min(1, voice.pan == null ? 0 : voice.pan));
      const leftGain = pan <= 0 ? 1 : 1 - pan;
      const rightGain = pan >= 0 ? 1 : 1 + pan;
      for (let i = 0; i < left.length; i += 1) {
        let frame = Math.floor(voice.cursor);
        if (frame >= frames) {
          if (voice.loop && loopStart < loopEnd) {
            voice.cursor = loopStart + ((voice.cursor - loopStart) % (loopEnd - loopStart));
            frame = Math.floor(voice.cursor);
          } else {
            ended.push(voice.id);
            break;
          }
        }
        const base = frame * 2;
        left[i] += (voice.pcm[base] / 32768) * voice.volume * leftGain;
        right[i] += (voice.pcm[base + 1] / 32768) * voice.volume * rightGain;
        voice.cursor += rate;
        if (voice.loop && voice.cursor >= loopEnd && loopStart < loopEnd) {
          voice.cursor = loopStart + ((voice.cursor - loopStart) % (loopEnd - loopStart));
        }
      }
    }
    for (const id of ended) {
      this.voices.delete(id);
      this.port.postMessage({ type: "ended", id });
    }
    return true;
  }
}
registerProcessor("pokecrystal-pcm", PokeCrystalPcmProcessor);
`;

let nextPcmVoiceId = 1;

class PcmAudioHandle implements AudioHandle {
  public readonly src: string;
  public loop: boolean;
  public ended = false;
  public paused = true;
  public currentTime = 0;
  private volumeValue: number;
  private mutedValue: boolean;
  private playbackRateValue: number;
  private readonly listeners = new Map<string, Array<{ listener: () => void; once: boolean }>>();

  constructor(
    private readonly backend: PcmAudioPlaybackBackend,
    private readonly voice: PcmBackendVoice,
  ) {
    this.src = voice.source;
    this.loop = voice.loop;
    this.volumeValue = voice.volume;
    this.mutedValue = voice.muted;
    this.playbackRateValue = voice.playbackRate;
  }

  get id(): number {
    return this.voice.id;
  }

  get volume(): number {
    return this.volumeValue;
  }

  set volume(value: number) {
    this.volumeValue = Math.max(0, Math.min(1, value));
    this.voice.volume = this.volumeValue;
    this.backend.updateVoice(this.id, { volume: this.volumeValue });
  }

  get muted(): boolean {
    return this.mutedValue;
  }

  set muted(value: boolean) {
    this.mutedValue = Boolean(value);
    this.voice.muted = this.mutedValue;
    this.backend.updateVoice(this.id, { muted: this.mutedValue });
  }

  get playbackRate(): number {
    return this.playbackRateValue;
  }

  set playbackRate(value: number) {
    this.playbackRateValue = Number.isFinite(value) ? Math.max(0.05, Math.min(4, value)) : 1;
    this.voice.playbackRate = this.playbackRateValue;
    this.backend.updateVoice(this.id, { playbackRate: this.playbackRateValue });
  }

  play(): Promise<void> {
    this.paused = false;
    this.ended = false;
    this.backend.playVoice(
      {
        ...this.voice,
        volume: this.volumeValue,
        muted: this.mutedValue,
        playbackRate: this.playbackRateValue,
      },
      () => this.finish(),
    );
    return this.backend.resume().catch(() => undefined);
  }

  pause(): void {
    if (this.paused && this.ended) {
      return;
    }
    this.paused = true;
    this.backend.stopVoice(this.id);
    this.emit("pause");
  }

  addEventListener(event: string, listener: () => void, options?: { once?: boolean }): void {
    const entries = this.listeners.get(event) ?? [];
    entries.push({ listener, once: Boolean(options?.once) });
    this.listeners.set(event, entries);
  }

  private finish(): void {
    if (this.ended) {
      return;
    }
    this.ended = true;
    this.paused = true;
    this.emit("ended");
  }

  private emit(event: string): void {
    const entries = this.listeners.get(event) ?? [];
    const next: Array<{ listener: () => void; once: boolean }> = [];
    for (const entry of entries) {
      entry.listener();
      if (!entry.once) {
        next.push(entry);
      }
    }
    this.listeners.set(event, next);
  }
}

class BrowserPcmAudioBackend implements PcmAudioPlaybackBackend {
  private context: AudioContext | null = null;
  private node: AudioWorkletNode | null = null;
  private initPromise: Promise<AudioWorkletNode | null> | null = null;
  private workletUrl: string | null = null;
  private readonly endedCallbacks = new Map<number, () => void>();

  static isSupported(): boolean {
    if (typeof window === "undefined" || typeof Blob === "undefined" || typeof URL === "undefined") {
      return false;
    }
    const ContextCtor = (window as Window & { AudioContext?: typeof AudioContext; webkitAudioContext?: typeof AudioContext }).AudioContext
      ?? (window as Window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!ContextCtor) {
      return false;
    }
    return "audioWorklet" in ContextCtor.prototype ||
      typeof (window as Window & { AudioWorkletNode?: typeof AudioWorkletNode }).AudioWorkletNode === "function";
  }

  playVoice(voice: PcmBackendVoice, onEnded?: () => void): void {
    if (onEnded) {
      this.endedCallbacks.set(voice.id, onEnded);
    }
    void this.ensureNode().then((node) => {
      if (!node) {
        return;
      }
      const pcm = new Int16Array(voice.pcm);
      node.port.postMessage(
        {
          type: "play",
          voice: {
            ...voice,
            pcm: pcm.buffer,
          },
        },
        [pcm.buffer],
      );
    });
  }

  stopVoice(id: number): void {
    this.endedCallbacks.delete(id);
    this.node?.port.postMessage({ type: "stop", id });
  }

  updateVoice(id: number, patch: Partial<Pick<PcmBackendVoice, "volume" | "muted" | "pan" | "playbackRate">>): void {
    this.node?.port.postMessage({ type: "update", id, patch });
  }

  async resume(): Promise<void> {
    const node = await this.ensureNode();
    if (!node || !this.context) {
      return;
    }
    if (this.context.state === "suspended") {
      await this.context.resume();
    }
  }

  dispose(): void {
    this.node?.port.postMessage({ type: "clear" });
    this.node?.disconnect();
    this.node = null;
    this.initPromise = null;
    this.endedCallbacks.clear();
    if (this.workletUrl) {
      URL.revokeObjectURL(this.workletUrl);
      this.workletUrl = null;
    }
  }

  private async ensureNode(): Promise<AudioWorkletNode | null> {
    if (this.node) {
      return this.node;
    }
    if (this.initPromise) {
      return this.initPromise;
    }
    this.initPromise = this.createNode();
    return this.initPromise;
  }

  private async createNode(): Promise<AudioWorkletNode | null> {
    if (typeof window === "undefined") {
      return null;
    }
    const ContextCtor = (window as Window & { AudioContext?: typeof AudioContext; webkitAudioContext?: typeof AudioContext }).AudioContext
      ?? (window as Window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!ContextCtor) {
      return null;
    }
    const context = this.context ?? new ContextCtor();
    this.context = context;
    if (!context.audioWorklet) {
      return null;
    }
    try {
      const blob = new Blob([PCM_WORKLET_SOURCE], { type: "text/javascript" });
      this.workletUrl = URL.createObjectURL(blob);
      await context.audioWorklet.addModule(this.workletUrl);
      const node = new AudioWorkletNode(context, "pokecrystal-pcm", {
        numberOfInputs: 0,
        numberOfOutputs: 1,
        outputChannelCount: [2],
      });
      node.port.onmessage = (event) => {
        const message = event.data as { type?: string; id?: number };
        if (message.type === "ended" && typeof message.id === "number") {
          const callback = this.endedCallbacks.get(message.id);
          this.endedCallbacks.delete(message.id);
          callback?.();
        }
      };
      node.connect(context.destination);
      this.node = node;
      return node;
    } catch {
      return null;
    }
  }
}

const MENU_SOUND_ALIASES: Record<string, string> = {
  menu_cursor: "SFX_MENU",
  menu_option: "SFX_READ_TEXT_2",
  menu_cancel: "SFX_READ_TEXT_2",
};

// ASM: audio/sfx.asm Sfx_Fanfare* uses sfx_priority_on/off to mute music.
const PRIORITY_SFX_TOKENS = new Set([
  "SFX_FANFARE",
  "SFX_FANFARE_2",
  "SFX_CAUGHT_MON",
  "SFX_LEVEL_UP",
  "SFX_REGISTER_PHONE_NUMBER",
  "SFX_PRESENT",
  "SFX_1ST_PLACE",
  "SFX_2ND_PLACE",
  "SFX_3RD_PLACE",
  "SFX_GET_EGG",
  "SFX_GET_EGG_UNUSED",
  "SFX_GET_TM",
  "SFX_GET_BADGE",
  "SFX_GET_TRADEMON",
  "SFX_EVOLVED",
]);
const PRIORITY_SFX_PREFIXES = ["SFX_DEX_FANFARE_", "SFX_GET_"];

let cachedAliases: DisassemblyAliases | null = null;
let cachedCryTable: Record<string, CryTableEntry> | null = null;

const stripComment = (line: string): string => line.split(";", 1)[0].trim();

const normalizePointerLabel = (label: string): string => {
  let suffix = label;
  if (suffix.toLowerCase().startsWith("music_")) {
    suffix = suffix.slice("music_".length);
  }
  return suffix.replace(/[^A-Za-z0-9]/g, "").toLowerCase();
};

const normalizeSfxLabel = (label: string): string => {
  let suffix = label;
  if (suffix.toLowerCase().startsWith("sfx_")) {
    suffix = suffix.slice("sfx_".length);
  }
  return suffix.replace(/[^A-Za-z0-9]/g, "").toLowerCase();
};

const parseConstants = (contents: string, prefix: string): string[] => {
  const regex = new RegExp(`^const\\s+(${prefix}[A-Z0-9_]+)\\b`);
  const entries: string[] = [];
  for (const rawLine of contents.split(/\r?\n/)) {
    const line = stripComment(rawLine);
    if (!line) {
      continue;
    }
    const match = line.match(regex);
    if (match) {
      entries.push(match[1]);
    }
  }
  return entries;
};

const parsePointerLabels = (contents: string, prefix: string): string[] => {
  const regex = new RegExp(`^dba\\s+(${prefix}[A-Za-z0-9_]+)\\b`);
  const entries: string[] = [];
  for (const rawLine of contents.split(/\r?\n/)) {
    const line = stripComment(rawLine);
    if (!line) {
      continue;
    }
    const match = line.match(regex);
    if (match) {
      entries.push(match[1]);
    }
  }
  return entries;
};

const buildSfxPriorityMap = (constants: string[]): Record<string, number> => {
  const priorities: Record<string, number> = {};
  constants.forEach((name, index) => {
    priorities[name] = index;
  });
  return priorities;
};

const loadStaticAliases = (): DisassemblyAliases => {
  const sfx: Record<string, string> = { ...DISASSEMBLY_SFX_ALIASES };
  return {
    music: { ...DISASSEMBLY_MUSIC_ALIASES },
    sfx,
    sfxPriority: buildSfxPriorityMap(Object.keys(sfx)),
  };
};

const loadDisassemblyAliases = (): DisassemblyAliases | null => {
  if (cachedAliases) {
    return cachedAliases;
  }
  if (typeof window !== "undefined") {
    cachedAliases = loadStaticAliases();
    return cachedAliases;
  }
  try {
    // ASM: constants/music_constants.asm, constants/sfx_constants.asm,
    // audio/music_pointers.asm, audio/sfx_pointers.asm
    const disasmRoot = getDisassemblyRoot();
    const musicConstantsPath = path.join(disasmRoot, "constants", "music_constants.asm");
    const sfxConstantsPath = path.join(disasmRoot, "constants", "sfx_constants.asm");
    const musicPointersPath = path.join(disasmRoot, "audio", "music_pointers.asm");
    const sfxPointersPath = path.join(disasmRoot, "audio", "sfx_pointers.asm");
    const musicConstants = parseConstants(fs.readFileSync(musicConstantsPath, "utf8"), "MUSIC_");
    const sfxConstants = parseConstants(fs.readFileSync(sfxConstantsPath, "utf8"), "SFX_");
    const musicPointers = parsePointerLabels(fs.readFileSync(musicPointersPath, "utf8"), "Music_");
    const sfxPointers = parsePointerLabels(fs.readFileSync(sfxPointersPath, "utf8"), "Sfx_");
    if (musicConstants.length !== musicPointers.length) {
      throw new Error("Music constants do not match pointer table length.");
    }
    if (sfxConstants.length !== sfxPointers.length) {
      throw new Error("SFX constants do not match pointer table length.");
    }
    const music: Record<string, string> = {};
    const sfx: Record<string, string> = {};
    musicConstants.forEach((name, index) => {
      music[name] = normalizePointerLabel(musicPointers[index]);
    });
    sfxConstants.forEach((name, index) => {
      sfx[name] = `sfx/${normalizeSfxLabel(sfxPointers[index])}`;
    });
    const sfxPriority = buildSfxPriorityMap(sfxConstants);
    cachedAliases = { music, sfx, sfxPriority };
    return cachedAliases;
  } catch {
    cachedAliases = loadStaticAliases();
    return cachedAliases;
  }
};

const loadCryTable = (): Record<string, CryTableEntry> | null => {
  if (cachedCryTable) {
    return cachedCryTable;
  }
  try {
    const tablePath = getAssetPath("data", "pokemon_cries.json");
    const raw = fs.readFileSync(tablePath, "utf8");
    cachedCryTable = JSON.parse(raw) as Record<string, CryTableEntry>;
    return cachedCryTable;
  } catch {
    return null;
  }
};

const slugifyToken = (token: string): string =>
  token.replace(/[^A-Za-z0-9]/g, "").toLowerCase();

const resolveCryBase = (species: string): string => {
  const cryTable = loadCryTable();
  const entry = cryTable?.[species]?.cry ?? `CRY_${species}`;
  return entry.replace(/^CRY_/, "").toLowerCase();
};

const normalizePriorityToken = (token: string): string => {
  const trimmed = token.trim();
  if (!trimmed) {
    return "";
  }
  const alias = MENU_SOUND_ALIASES[trimmed.toLowerCase()];
  return (alias ?? trimmed).trim().toUpperCase();
};

const normalizeSfxToken = (token: string): string => {
  const trimmed = token.trim();
  if (!trimmed) {
    return "";
  }
  const alias = MENU_SOUND_ALIASES[trimmed.toLowerCase()];
  return (alias ?? trimmed).trim();
};

const isPrioritySoundToken = (token: string): boolean => {
  const normalized = normalizePriorityToken(token);
  if (!normalized) {
    return false;
  }
  // ASM: audio/engine.asm::_PlayCry sets wSFXPriority to mute music during cries.
  if (getSoundChannelCategory(normalized) === "cry") {
    return true;
  }
  return (
    getSoundChannelCategory(normalized) === "sfx" &&
    (PRIORITY_SFX_TOKENS.has(normalized) ||
      PRIORITY_SFX_PREFIXES.some((prefix) => normalized.startsWith(prefix)))
  );
};

const getSoundChannelCategory = (token: string): SoundChannelCategory => {
  const normalized = normalizePriorityToken(token);
  if (normalized.startsWith("SFX_")) {
    return "sfx";
  }
  if (normalized.startsWith("CRY_") || normalized.endsWith("_CRY")) {
    return "cry";
  }
  return "other";
};

const isSfxChannelToken = (token: string): boolean => getSoundChannelCategory(token) === "sfx";

const joinUrl = (base: string, parts: string[]): string => {
  const cleanedBase = base.replace(/\/+$/, "");
  const cleanedParts = parts.map((part) => part.replace(/^\/+/, ""));
  return [cleanedBase, ...cleanedParts].join("/");
};

const getAudioAssetPath = (...parts: string[]): string => {
  if (typeof window === "undefined") {
    return getAssetPath("audio", ...parts);
  }
  const base = process.env.NEXT_PUBLIC_AUDIO_BASE || "/api/audio";
  return joinUrl(base, parts);
};

const resolveMusicAsset = (token: string, _directPcm: boolean): string | null => {
  const upper = token.toUpperCase();
  const aliases = loadDisassemblyAliases();
  if (upper.startsWith("MUSIC_")) {
    const mapped = aliases?.music[upper] ?? slugifyToken(upper.replace(/^MUSIC_/, ""));
    return getAudioAssetPath("pcm", "music", `${mapped}.json`);
  }
  const normalized = token.trim().toLowerCase();
  if (!normalized) {
    return null;
  }
  return getAudioAssetPath("pcm", "music", `${normalized}.json`);
};

const resolveSoundAsset = (token: string, directPcm: boolean): string | null => {
  const alias = MENU_SOUND_ALIASES[token.toLowerCase()];
  if (alias) {
    return resolveSoundAsset(alias, directPcm);
  }
  const upper = token.toUpperCase();
  const aliases = loadDisassemblyAliases();
  if (upper.startsWith("SFX_")) {
    const mapped = aliases?.sfx[upper] ?? `sfx/${slugifyToken(upper.replace(/^SFX_/, ""))}`;
    return getAudioAssetPath("pcm", "sfx", `${path.basename(mapped)}.json`);
  }
  if (upper.startsWith("CRY_")) {
    const base = resolveCryBase(upper.replace(/^CRY_/, ""));
    return getAudioAssetPath("pcm", "cries", `${base}.json`);
  }
  if (upper.endsWith("_CRY")) {
    const base = upper.replace(/_CRY$/, "").toLowerCase();
    return getAudioAssetPath("pcm", "cries", `${base}.json`);
  }
  return null;
};

export class AudioEngine {
  public readonly sounds: Record<string, string> = {};
  public readonly music: Record<string, string> = {};
  public masterVolume: number;
  public muted: boolean;
  private currentMusic: AudioHandle | null = null;
  private currentMusicState: MusicPlaybackState | null = null;
  private currentMusicName: string | null = null;
  private currentMusicRole: string = "general";
  private mapMusicName: string | null = null;
  private musicMutedByPriority = false;
  private priorityMuteCount = 0;
  private fadeState:
    | {
        active: boolean;
        durationFrames: number;
        remainingFrames: number;
        startVolume: number;
      }
    | null = null;
  private activeSounds = new Map<string, AudioHandle[]>();
  private activeSoundReleases = new Map<AudioHandle, SfxReleaseState>();
  private activeSoundTimeouts = new Map<AudioHandle, ReturnType<typeof setTimeout>>();
  private activeSoundFrames = new Map<AudioHandle, number>();
  private activeSoundTokens = new Map<AudioHandle, string>();
  private activeSoundChannels = new Map<AudioHandle, number[]>();
  private currentSfxPriority: number | null = null;
  private pendingSounds: PendingSound[] = [];
  private currentAudioContext: AudioContext | null = null;
  private pendingGraphTeardowns = new Map<AudioHandle, () => void>();
  private readonly playbackBackendKind: AudioPlaybackBackendKind;
  private readonly pcmBackend: PcmAudioPlaybackBackend | null;
  private musicMutedByController = false;
  private suppressedMusicChannels = new Set<number>();
  private pendingMusicAfterFade: { token: string; role: string } | null = null;
  private pendingMusicRequestId = 0;
  private playbackEventSequence = 0;
  private recentPlaybackEvents: AudioPlaybackEvent[] = [];

  constructor(options?: {
    masterVolume?: number;
    muted?: boolean;
    playbackBackend?: AudioPlaybackBackendKind;
    pcmBackend?: PcmAudioPlaybackBackend | null;
  }) {
    this.masterVolume = options?.masterVolume ?? 1;
    this.muted = options?.muted ?? false;
    this.playbackBackendKind = options?.playbackBackend ?? "auto";
    this.pcmBackend =
      options?.pcmBackend === undefined
        ? (this._shouldUseDirectPcm() ? new BrowserPcmAudioBackend() : null)
        : options.pcmBackend;
  }

  loadSound(name: string, filePath: string): void {
    this.sounds[name] = filePath;
  }

  loadMusic(name: string, filePath: string): void {
    this.music[name] = filePath;
  }

  private _shouldUseDirectPcm(): boolean {
    if (this.playbackBackendKind === "html") {
      return false;
    }
    if (this.playbackBackendKind === "direct-pcm") {
      return true;
    }
    return BrowserPcmAudioBackend.isSupported();
  }

  private _usesDirectPcm(): boolean {
    return this.pcmBackend !== null;
  }

  playSound(name: string, options: BattleSoundOptions = {}): void {
    const token = normalizeSfxToken(String(name));
    if (!token) {
      return;
    }
    const source = this._resolveSource(token, this.sounds);
    if (!source || this.muted) {
      return;
    }
    const normalized = this._normalizeBattleSoundOptions(options);
    if (this._isManifestSource(source)) {
      void this._playSoundManifest(token, source, normalized);
      return;
    }
    this._playSoundSource(token, source, true, normalized);
  }

  play_sound(name: string, options: BattleSoundOptions = {}): void {
    this.playSound(name, options);
  }

  private _resolveBattlePan(panning: BattlePanDirection | null, tracks: number | null): number | null {
    const normalized = panning?.trim().toLowerCase();
    if (normalized === "left" || normalized === "player") {
      return -1;
    }
    if (normalized === "right" || normalized === "enemy") {
      return 1;
    }
    if (normalized === "center") {
      return 0;
    }
    if (tracks === null || tracks === undefined) {
      return null;
    }
    return (tracks & 1) === 0 ? -1 : 1;
  }

  private _normalizeBattleSoundOptions(options: BattleSoundOptions): ResolvedBattleSoundOptions {
    const tracks = options.tracks === undefined || options.tracks === null
      ? null
      : Math.max(0, Math.floor(options.tracks)) & 0x03;
    const duration = options.duration === undefined || options.duration === null
      ? null
      : Math.max(0, Math.floor(options.duration));
    const pitch = options.pitch === undefined || options.pitch === null
      ? null
      : options.pitch;
    const rawPanning = (options.panning ?? "").toString().trim().toLowerCase();
    const panning: BattlePanDirection | null = rawPanning && ["left", "right", "player", "enemy", "center"].includes(rawPanning)
      ? (rawPanning as BattlePanDirection)
      : null;
    return {
      tracks,
      duration,
      pitch,
      panning,
    };
  }

  private _soundDurationMs(durationFrames: number | null): number | null {
    if (durationFrames === null || durationFrames <= 0) {
      return null;
    }
    // ASM duration is measured in frames for battle animation commands.
    const durationMs = Math.ceil(durationFrames * FRAME_MS);
    return durationMs > 0 ? durationMs : null;
  }

  private _durationMsToFrames(durationMs: number): number {
    if (!Number.isFinite(durationMs) || durationMs <= 0) {
      return 0;
    }
    return Math.max(1, Math.ceil(durationMs / FRAME_MS));
  }

  private _setTimeout(callback: () => void, delayMs: number): ReturnType<typeof setTimeout> | null {
    const schedule =
      typeof window !== "undefined" && typeof window.setTimeout === "function"
        ? window.setTimeout
        : (typeof setTimeout === "function" ? setTimeout : null);
    if (!schedule) {
      return null;
    }
    return schedule(callback, delayMs) as ReturnType<typeof setTimeout>;
  }

  private _clearTimeout(handle: ReturnType<typeof setTimeout>): void {
    const clear =
      typeof window !== "undefined" && typeof window.clearTimeout === "function"
        ? window.clearTimeout
        : (typeof clearTimeout === "function" ? clearTimeout : null);
    if (!clear) {
      return;
    }
    clear(handle as unknown as number);
  }

  private _toPlaybackRate(pitch: number | null): number | null {
    if (pitch === null || !Number.isFinite(pitch)) {
      return null;
    }
    const rounded = Math.trunc(pitch);
    const semitone = Math.max(-24, Math.min(24, rounded));
    const playbackRate = 2 ** (semitone / 12);
    if (!Number.isFinite(playbackRate)) {
      return null;
    }
    return Math.max(0.2, Math.min(3.5, playbackRate));
  }

  private _getAudioContext(): AudioContext | null {
    if (this.currentAudioContext) {
      return this.currentAudioContext;
    }
    if (typeof window === "undefined") {
      return null;
    }
    const ContextCtor = (window as Window & { AudioContext?: typeof AudioContext; webkitAudioContext?: typeof AudioContext }).AudioContext
      ?? (window as Window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!ContextCtor) {
      return null;
    }
    const context = new ContextCtor();
    this.currentAudioContext = context;
    return context;
  }

  private _disposeSoundGraph(audio: AudioHandle): void {
    const teardown = this.pendingGraphTeardowns.get(audio);
    if (!teardown) {
      return;
    }
    teardown();
    this.pendingGraphTeardowns.delete(audio);
  }

  private _attachSoundGraph(audio: AudioHandle, panning: number | null): (() => void) | null {
    if (panning === null || typeof window === "undefined") {
      return null;
    }
    const context = this._getAudioContext();
    if (!context || !("createStereoPanner" in context) || typeof context.createStereoPanner !== "function") {
      return null;
    }
    try {
      const source = context.createMediaElementSource(audio as HTMLAudioElement);
      const panner = context.createStereoPanner();
      const gain = context.createGain();
      panner.pan.value = panning;
      source.connect(panner).connect(gain).connect(context.destination);
      return () => {
        try {
          source.disconnect();
          panner.disconnect();
          gain.disconnect();
        } catch {
          // ignore
        }
      };
    } catch {
      return null;
    }
  }

  private _applyBattleSoundOptions(audio: AudioHandle, options: ResolvedBattleSoundOptions): void {
    const playbackRate = this._toPlaybackRate(options.pitch ?? null);
    if (playbackRate !== null) {
      audio.playbackRate = playbackRate;
    }
    const pan = this._resolveBattlePan(options.panning, options.tracks);
    if (pan === null) {
      return;
    }
    const teardown = this._attachSoundGraph(audio, pan);
    if (teardown) {
      this.pendingGraphTeardowns.set(audio, teardown);
    }
  }

  setMuted(muted: boolean): void {
    const nextMuted = Boolean(muted);
    if (this.muted === nextMuted) {
      return;
    }
    this.muted = nextMuted;
    this._syncMuteState();
    if (!nextMuted) {
      this.unlock();
    }
  }

  setMasterVolume(volume: number): void {
    if (!Number.isFinite(volume)) {
      return;
    }
    const nextVolume = Math.max(0, Math.min(1, volume));
    if (this.masterVolume === nextVolume) {
      return;
    }
    this.masterVolume = nextVolume;
    this._applyMusicState();
    for (const sounds of this.activeSounds.values()) {
      for (const audio of sounds) {
        audio.volume = nextVolume;
      }
    }
  }

  set_master_volume(volume: number): void {
    this.setMasterVolume(volume);
  }

  unlock(): void {
    if (this.muted) {
      return;
    }
    const context = this.currentAudioContext;
    if (context?.state === "suspended" && typeof context.resume === "function") {
      void context.resume().catch(() => null);
    }
    void this.pcmBackend?.resume().catch(() => null);
    if (this.currentMusicState) {
      this._resumeMusicState();
    } else if (this.currentMusicName) {
      this.playMusic(this.currentMusicName, this.currentMusicRole);
    } else if (this.mapMusicName) {
      this.playMusic(this.mapMusicName, "map");
    }
    this._flushPendingSounds();
  }

  playMusic(name: string, role: string = "general"): void {
    const token = String(name).trim();
    if (!token) {
      return;
    }
    if (this._isSilenceToken(token)) {
      this.stopMusic();
      return;
    }
    this.currentMusicName = token;
    this.currentMusicRole = role;
    if (role === "map") {
      this.mapMusicName = token;
    }
    const source = this._resolveSource(token, this.music);
    if (!source) {
      return;
    }
    this._recordPlaybackEvent({
      kind: "music",
      token,
      source,
      role,
      loop: true,
    });
    this.fadeState = null;
    this.pendingMusicAfterFade = null;
    this._stopMusicElement();
    if (this._isManifestSource(source)) {
      const requestId = ++this.pendingMusicRequestId;
      void this._playMusicManifest(token, source, role, requestId);
      return;
    }
    const audio = this._createAudio(source, true);
    if (!audio) {
      return;
    }
    this.currentMusic = audio;
    this.currentMusicState = {
      source,
      stems: new Map(),
      mixed: audio,
      manifest: null,
      frameCursor: 0,
      gain: 1,
      role,
      token,
    };
    this._applyMusicState();
    this._playAudioHandle(audio);
  }

  play_music(name: string, role?: string | { role?: string }): void {
    const resolvedRole =
      typeof role === "string" ? role : role?.role ?? "general";
    this.playMusic(name, resolvedRole);
  }

  stopMusic(): void {
    this.fadeState = null;
    this.pendingMusicAfterFade = null;
    this._stopMusicElement();
    this.currentMusicName = null;
    this.currentMusicRole = "general";
  }

  stop_music(): void {
    this.stopMusic();
  }

  clearMapMusic(): void {
    this.mapMusicName = null;
  }

  sfxChannelsOff(): void {
    this._stopSfxChannels();
  }

  channelsOff(): void {
    this.stopMusic();
    this.sfxChannelsOff();
  }

  dispose(): void {
    this.channelsOff();
    this.pendingSounds = [];
    this.activeSoundReleases.clear();
    this.activeSoundTimeouts.clear();
    this.activeSoundFrames.clear();
    this.activeSoundTokens.clear();
    this.activeSoundChannels.clear();
    this.pendingGraphTeardowns.clear();
    this.priorityMuteCount = 0;
    this.musicMutedByPriority = false;
    this.suppressedMusicChannels.clear();
    this.pcmBackend?.dispose();
    this._syncMuteState();
  }

  fadeToMusicFrames(name: string, durationFrames: number, role: string = "general"): void {
    const token = String(name ?? "").trim();
    if (!token) {
      return;
    }
    this.pendingMusicAfterFade = { token, role };
    this.fadeOutMusicFrames(durationFrames);
  }

  fadeOutMusic(durationMs: number): void {
    if (!this.currentMusicState) {
      return;
    }
    const duration = this._durationMsToFrames(durationMs);
    if (duration === 0) {
      this._finishFadeTransition();
      return;
    }
    this.fadeState = {
      active: true,
      durationFrames: duration,
      remainingFrames: duration,
      startVolume: this.currentMusicState.gain,
    };
  }

  fadeOutMusicFrames(durationFrames: number): void {
    if (!this.currentMusicState) {
      return;
    }
    const duration = Math.max(0, Math.trunc(durationFrames));
    if (duration === 0) {
      this._finishFadeTransition();
      return;
    }
    this.fadeState = {
      active: true,
      durationFrames: duration,
      remainingFrames: duration,
      startVolume: this.currentMusicState.gain,
    };
  }

  fade_out_music(durationMs: number): void {
    this.fadeOutMusic(durationMs);
  }

  restartMapMusic(): void {
    const target = this.mapMusicName ?? this.currentMusicName;
    if (!target) {
      return;
    }
    this.playMusic(target, "map");
  }

  restart_map_music(): void {
    this.restartMapMusic();
  }

  hasTemporaryMusicOverride(): boolean {
    const token = String(this.currentMusicName ?? "").trim();
    if (!token || this._isSilenceToken(token)) {
      return false;
    }
    return this.currentMusicRole !== "map";
  }

  isSoundPlaying(_name?: string): boolean {
    this._pruneEndedSounds();
    if (!_name) {
      return this._hasActiveSfxChannelSound();
    }
    const token = normalizeSfxToken(String(_name));
    if ((this.activeSounds.get(token) ?? []).length > 0) {
      return true;
    }
    const normalizedToken = normalizePriorityToken(token);
    return (normalizedToken !== token && (this.activeSounds.get(normalizedToken) ?? []).length > 0);
  }

  is_sound_playing(name?: string): boolean {
    return this.isSoundPlaying(name);
  }

  getActiveSoundIds(): string[] {
    this._pruneEndedSounds();
    return Array.from(this.activeSounds.entries())
      .filter(([token, sounds]) => isSfxChannelToken(token) && sounds.length > 0)
      .map(([token]) => token);
  }

  get_active_sound_ids(): string[] {
    return this.getActiveSoundIds();
  }

  getPlaybackSnapshot(): AudioPlaybackSnapshot {
    const activeChannels: ActiveChannelState[] = [];
    if (this.currentMusicState) {
      for (const channel of this.currentMusicState.stems.keys()) {
        activeChannels.push({
          channel,
          ownerToken: this.currentMusicState.token,
          category: "music",
          role: this.currentMusicState.role,
        });
      }
      if (this.currentMusicState.stems.size === 0 && this.currentMusicState.mixed) {
        activeChannels.push({
          channel: 0,
          ownerToken: this.currentMusicState.token,
          category: "music",
          role: this.currentMusicState.role,
        });
      }
    }
    for (const [audio, channels] of this.activeSoundChannels.entries()) {
      const token = this.activeSoundTokens.get(audio);
      if (!token) {
        continue;
      }
      const category = getSoundChannelCategory(token);
      for (const channel of channels) {
        activeChannels.push({ channel, ownerToken: token, category });
      }
    }
    return {
      musicToken: this.currentMusicName,
      musicRole: this.currentMusicRole,
      musicSource:
        this.currentMusicState?.source ??
        (this.currentMusicName ? this._resolveSource(this.currentMusicName, this.music) : null),
      musicFrame: this.currentMusicState?.frameCursor ?? 0,
      fadedVolume: this.currentMusicState?.gain ?? 0,
      activeChannels,
      recentEvents: this.recentPlaybackEvents.slice(),
    };
  }

  update(): void {
    this._tickSoundDurations();
    if (this.currentMusicState) {
      this.currentMusicState.frameCursor += 1;
    }
    if (!this.fadeState?.active || !this.currentMusicState) {
      return;
    }
    this.fadeState.remainingFrames -= 1;
    const remaining = Math.max(0, this.fadeState.remainingFrames);
    const progress = 1 - remaining / this.fadeState.durationFrames;
    const volume = Math.max(0, this.fadeState.startVolume * (1 - progress));
    this.currentMusicState.gain = volume;
    this._applyMusicState();
    if (remaining <= 0) {
      this._finishFadeTransition();
    }
  }

  private _recordPlaybackEvent(event: Omit<AudioPlaybackEvent, "sequence">): void {
    this.playbackEventSequence += 1;
    this.recentPlaybackEvents.push({
      sequence: this.playbackEventSequence,
      ...event,
    });
    if (this.recentPlaybackEvents.length > 32) {
      this.recentPlaybackEvents = this.recentPlaybackEvents.slice(-32);
    }
  }

  private _stopMusicElement(): void {
    const state = this.currentMusicState;
    const audios = state
      ? [...state.stems.values(), ...(state.mixed ? [state.mixed] : [])]
      : (this.currentMusic ? [this.currentMusic] : []);
    for (const audio of audios) {
      audio.pause();
      try {
        audio.currentTime = 0;
      } catch {
        // Some browsers block setting currentTime until metadata is loaded.
      }
    }
    this.currentMusic = null;
    this.currentMusicState = null;
  }

  private _stopSfxChannels(): void {
    for (const [token, sounds] of Array.from(this.activeSounds.entries())) {
      if (!this._isSfxChannelSoundToken(token)) {
        continue;
      }
      for (const audio of sounds) {
        this._disposeSoundGraph(audio);
        this._releaseActiveSound(token, audio);
        try {
          audio.pause();
          audio.currentTime = 0;
        } catch {
          // ignore
        }
      }
    }
    this.pendingSounds = [];
  }

  private _isSilenceToken(token: string): boolean {
    return token.trim().toUpperCase() === "MUSIC_NONE";
  }

  private _resolveSource(token: string, table: Record<string, string>): string | null {
    if (table[token]) {
      return table[token];
    }
    const upper = token.toUpperCase();
    if (table[upper]) {
      return table[upper];
    }
    const lower = token.toLowerCase();
    if (table[lower]) {
      return table[lower];
    }
    const resolved = table === this.music
      ? resolveMusicAsset(token, this._usesDirectPcm())
      : resolveSoundAsset(token, this._usesDirectPcm());
    if (resolved) {
      return resolved;
    }
    if (this._looksLikePath(token)) {
      return token;
    }
    return null;
  }

  canResolveMusicToken(token: string): boolean {
    return this._resolveSource(token, this.music) !== null || this._isSilenceToken(token);
  }

  canResolveSoundToken(token: string): boolean {
    return this._resolveSource(token, this.sounds) !== null;
  }

  private _looksLikePath(token: string): boolean {
    if (token.includes("/")) {
      return true;
    }
    return /\.[a-z0-9]+$/i.test(token);
  }

  private _createAudio(source: string, loop: boolean): AudioHandle | null {
    if (typeof window === "undefined" || typeof window.Audio !== "function") {
      return null;
    }
    const audio = new window.Audio(source);
    audio.loop = loop;
    audio.volume = this.masterVolume;
    audio.muted = this.muted;
    return audio;
  }

  private _playAudioHandle(audio: AudioHandle): void {
    const result = audio.play();
    if (result && typeof result.then === "function") {
      void result.catch(() => null);
    }
  }

  private _playSoundSource(
    token: string,
    source: string,
    allowRetry: boolean,
    options: ResolvedBattleSoundOptions,
    ownedChannels: number[] = [],
    priorityClass?: "none" | "priority" | "cry",
  ): void {
    if (!this._canPlaySfx(token)) {
      return;
    }
    this._recordPlaybackEvent({
      kind: getSoundChannelCategory(token),
      token,
      source,
      loop: false,
    });
    const audio = this._createAudio(source, false);
    if (!audio) {
      return;
    }
    this._applyBattleSoundOptions(audio, options);
    const trackActive = () => {
      if (ownedChannels.length > 0) {
        this.activeSoundChannels.set(audio, ownedChannels);
      }
      const releasePriority = this._trackPrioritySound(token, ownedChannels, priorityClass);
      this._scheduleSoundDuration(token, audio, options.duration);
      const active = this.activeSounds.get(token) ?? [];
      active.push(audio);
      this.activeSounds.set(token, active);
      this.activeSoundReleases.set(audio, { released: false, release: releasePriority });
      const cleanup = () => {
        this._releaseActiveSound(token, audio);
      };
      audio.addEventListener("ended", cleanup, { once: true });
      audio.addEventListener("pause", cleanup, { once: true });
    };
    const handleRejection = (error: unknown) => {
      if (allowRetry && this._isAutoplayBlocked(error)) {
        this.pendingSounds.push({ token, source, options, priorityClass });
        this._disposeSoundGraph(audio);
      }
    };
    const playResult = audio.play();
    if (playResult && typeof playResult.then === "function") {
      void playResult.then(trackActive).catch(handleRejection);
    } else {
      trackActive();
    }
  }

  private _flushPendingSounds(): void {
    if (this.pendingSounds.length === 0 || this.muted) {
      return;
    }
    const queued = this.pendingSounds;
    this.pendingSounds = [];
    for (const entry of queued) {
      this._playSoundSource(
        entry.token,
        entry.source,
        true,
        entry.options,
        [],
        entry.priorityClass,
      );
    }
  }

  private _isAutoplayBlocked(error: unknown): boolean {
    if (!error || typeof error !== "object") {
      return false;
    }
    const name = (error as { name?: string }).name ?? "";
    return name === "NotAllowedError" || name === "NotSupportedError";
  }

  private _syncMuteState(): void {
    if (this.currentMusicState) {
      const globalMute = this.muted || this.musicMutedByPriority || this.musicMutedByController;
      if (this.currentMusicState.stems.size > 0) {
        for (const [channel, audio] of this.currentMusicState.stems.entries()) {
          audio.muted = globalMute || this.suppressedMusicChannels.has(channel);
        }
      }
      if (this.currentMusicState.mixed) {
        this.currentMusicState.mixed.muted = globalMute || this.suppressedMusicChannels.size > 0;
      }
    } else if (this.currentMusic) {
      this.currentMusic.muted = this.muted || this.musicMutedByPriority || this.musicMutedByController;
    }
    for (const sounds of this.activeSounds.values()) {
      for (const audio of sounds) {
        audio.muted = this.muted;
      }
    }
  }

  setMusicMutedByController(muted: boolean): void {
    this.musicMutedByController = Boolean(muted);
    this._syncMuteState();
  }

  private _canPlaySfx(token: string): boolean {
    // ASM: home/audio.asm::PlaySFX calls CheckSFX and uses wCurSFX priority ordering.
    if (!isSfxChannelToken(token)) {
      return true;
    }
    const priority = this._getSfxPriority(token);
    if (priority === null) {
      return true;
    }
    const hasActiveSfx = this._hasActiveSfx();
    if (!hasActiveSfx) {
      this.currentSfxPriority = priority;
      return true;
    }
    if (this.currentSfxPriority === null || this.currentSfxPriority >= priority) {
      this.sfxChannelsOff();
      this.currentSfxPriority = priority;
      return true;
    }
    return false;
  }

  private _getSfxPriority(token: string): number | null {
    const normalized = normalizePriorityToken(token);
    if (!normalized.startsWith("SFX_")) {
      return null;
    }
    const aliases = loadDisassemblyAliases();
    return aliases?.sfxPriority[normalized] ?? null;
  }

  private _hasActiveSfx(): boolean {
    this._pruneEndedSounds();
    for (const [token, sounds] of Array.from(this.activeSounds.entries())) {
      if (isSfxChannelToken(token) && sounds.length > 0) {
        return true;
      }
    }
    return false;
  }

  private _hasActiveSfxChannelSound(): boolean {
    this._pruneEndedSounds();
    // ASM: WaitSFX/CheckSFX sample channels 5-8. Pokemon cries are authored on
    // channels 5/6/8, so script waits and channel clears include them.
    for (const [token, sounds] of Array.from(this.activeSounds.entries())) {
      if (this._isSfxChannelSoundToken(token) && sounds.length > 0) {
        return true;
      }
    }
    return false;
  }

  private _isSfxChannelSoundToken(token: string): boolean {
    const category = getSoundChannelCategory(token);
    return category === "sfx" || category === "cry";
  }

  private _releaseActiveSound(token: string, audio: AudioHandle): void {
    this._clearTimedSoundState(audio);
    const entries = this.activeSounds.get(token) ?? [];
    const next = entries.filter((entry) => entry !== audio);
    if (next.length > 0) {
      this.activeSounds.set(token, next);
    } else {
      this.activeSounds.delete(token);
    }
    const releaseState = this.activeSoundReleases.get(audio);
    if (!releaseState || releaseState.released) {
      this.activeSoundReleases.delete(audio);
      this._disposeSoundGraph(audio);
      return;
    }
    releaseState.released = true;
    releaseState.release();
    this.activeSoundReleases.delete(audio);
    this._disposeSoundGraph(audio);
    this._recomputeSuppressedMusicChannels();
  }

  private _pruneEndedSounds(): void {
    for (const [token, sounds] of Array.from(this.activeSounds.entries())) {
      for (const audio of sounds) {
        if (audio.ended || audio.paused) {
          this._releaseActiveSound(token, audio);
        }
      }
    }
  }

  private _clearTimedSoundState(audio: AudioHandle): void {
    this.activeSoundFrames.delete(audio);
    this.activeSoundTokens.delete(audio);
    this.activeSoundChannels.delete(audio);
    const timeoutId = this.activeSoundTimeouts.get(audio);
    if (timeoutId !== undefined) {
      this._clearTimeout(timeoutId);
    }
    this.activeSoundTimeouts.delete(audio);
  }

  private _resolveSoundToken(audio: AudioHandle): string | null {
    const directToken = this.activeSoundTokens.get(audio);
    if (directToken) {
      return directToken;
    }
    for (const [token, sounds] of Array.from(this.activeSounds.entries())) {
      if (sounds.includes(audio)) {
        return token;
      }
    }
    return null;
  }

  private _releaseSoundByElement(audio: AudioHandle): void {
    const token = this._resolveSoundToken(audio);
    if (!token) {
      this._clearTimedSoundState(audio);
      this._disposeSoundGraph(audio);
      try {
        audio.pause();
        audio.currentTime = 0;
      } catch {
        // ignore
      }
      return;
    }
    this._releaseActiveSound(token, audio);
  }

  private _tickSoundDurations(): void {
    if (this.activeSoundFrames.size === 0) {
      return;
    }
    const expired = new Set<AudioHandle>();
    for (const [audio, framesLeft] of Array.from(this.activeSoundFrames.entries())) {
      const next = framesLeft - 1;
      if (next <= 0) {
        this.activeSoundFrames.delete(audio);
        expired.add(audio);
      } else {
        this.activeSoundFrames.set(audio, next);
      }
    }
    for (const audio of expired) {
      this._releaseSoundByElement(audio);
    }
  }

  private _scheduleSoundDuration(token: string, audio: AudioHandle, durationFrames: number | null): void {
    const duration = Math.max(0, Math.floor(durationFrames ?? 0));
    this.activeSoundTokens.set(audio, token);
    if (duration <= 0) {
      return;
    }
    this.activeSoundFrames.set(audio, duration);
    const durationMs = this._soundDurationMs(duration);
    if (durationMs === null) {
      return;
    }
    const timeout = this._setTimeout(() => {
      this._releaseSoundByElement(audio);
    }, durationMs);
    if (timeout === null) {
      return;
    }
    this.activeSoundTimeouts.set(audio, timeout);
  }

  private _trackPrioritySound(
    token: string,
    ownedChannels: number[] = [],
    priorityClass?: "none" | "priority" | "cry",
  ): () => void {
    if (ownedChannels.length > 0) {
      this._recomputeSuppressedMusicChannels();
    }
    const shouldMuteMusic =
      priorityClass === "cry"
        ? true
        : priorityClass === "priority"
          ? true
          : priorityClass === "none"
            ? false
            : isPrioritySoundToken(token);
    if (!shouldMuteMusic) {
      return () => undefined;
    }
    this._updatePriorityMute(1);
    let released = false;
    return () => {
      if (released) {
        return;
      }
      released = true;
      this._updatePriorityMute(-1);
    };
  }

  private _updatePriorityMute(delta: number): void {
    this.priorityMuteCount = Math.max(0, this.priorityMuteCount + delta);
    const shouldMute = this.priorityMuteCount > 0;
    if (this.musicMutedByPriority === shouldMute) {
      return;
    }
    this.musicMutedByPriority = shouldMute;
    this._syncMuteState();
  }

  private _isManifestSource(source: string): boolean {
    return source.toLowerCase().endsWith(".json");
  }

  private async _loadManifest<T extends AnyAudioManifest>(source: string): Promise<T | null> {
    try {
      if (typeof fetch === "function") {
        const response = await fetch(source, { cache: "force-cache" });
        if (!response.ok) {
          return null;
        }
        return (await response.json()) as T;
      }
      if (typeof window === "undefined") {
        const raw = fs.readFileSync(source, "utf8");
        return JSON.parse(raw) as T;
      }
    } catch {
      return null;
    }
    return null;
  }

  private _isPcmMusicManifest(manifest: AnyAudioManifest | null): manifest is PcmMusicTrackManifest {
    return Boolean(
      manifest &&
      manifest.kind === "music" &&
      Array.isArray((manifest as PcmMusicTrackManifest).stems) &&
      (manifest as Partial<MusicTrackManifest>).mixedPath === undefined &&
      (manifest as PcmMusicTrackManifest).stems.every((stem) => typeof stem.path === "string" && stem.bitsPerSample === 16),
    );
  }

  private _isPcmClipManifest(manifest: AnyAudioManifest | null): manifest is PcmClipManifest {
    return Boolean(
      manifest &&
      (manifest.kind === "sfx" || manifest.kind === "cry") &&
      typeof (manifest as PcmClipManifest).path === "string" &&
      (manifest as PcmClipManifest).bitsPerSample === 16,
    );
  }

  private _resolveManifestPath(manifestSource: string, assetPath: string): string {
    if (/^(https?:)?\/\//.test(assetPath) || assetPath.startsWith("/")) {
      return assetPath;
    }
    if (/^(https?:)?\/\//.test(manifestSource)) {
      return new URL(assetPath, manifestSource).toString();
    }
    const slash = manifestSource.lastIndexOf("/");
    return slash >= 0 ? `${manifestSource.slice(0, slash + 1)}${assetPath}` : assetPath;
  }

  private async _loadPcmBytes(source: string): Promise<Int16Array | null> {
    try {
      if (typeof fetch === "function") {
        const response = await fetch(source, { cache: "force-cache" });
        if (!response.ok) {
          return null;
        }
        return pcmBytesToInt16(await response.arrayBuffer());
      }
      if (typeof window === "undefined") {
        const raw = fs.readFileSync(source);
        return pcmBytesToInt16(raw.buffer.slice(raw.byteOffset, raw.byteOffset + raw.byteLength));
      }
    } catch {
      return null;
    }
    return null;
  }

  private _createPcmHandle(args: {
    kind: PcmBackendVoice["kind"];
    token: string;
    source: string;
    pcm: Int16Array;
    sampleRate: number;
    loop: boolean;
    loopStartSample: number | null;
    loopEndSample: number | null;
    volume: number;
    muted: boolean;
    pan: number | null;
    playbackRate: number;
  }): AudioHandle | null {
    if (!this.pcmBackend) {
      return null;
    }
    return new PcmAudioHandle(this.pcmBackend, {
      id: nextPcmVoiceId++,
      ...args,
    });
  }

  private async _playPcmMusicManifest(
    token: string,
    source: string,
    role: string,
    manifest: PcmMusicTrackManifest,
  ): Promise<void> {
    if (!this.pcmBackend) {
      return;
    }
    const stems = new Map<number, AudioHandle>();
    for (const stem of manifest.stems) {
      const path = this._resolveManifestPath(source, stem.path);
      const pcm = await this._loadPcmBytes(path);
      if (!pcm) {
        continue;
      }
      const handle = this._createPcmHandle({
        kind: "music",
        token,
        source: path,
        pcm,
        sampleRate: stem.sampleRate,
        loop: true,
        loopStartSample: stem.loopStartSample,
        loopEndSample: stem.loopEndSample,
        volume: this.masterVolume,
        muted: this.muted,
        pan: null,
        playbackRate: 1,
      });
      if (handle) {
        stems.set(stem.channel, handle);
      }
    }
    if (stems.size === 0) {
      return;
    }
    if (this.currentMusicState) {
      const currentState = this.currentMusicState;
      for (const audio of currentState.stems.values()) {
        audio.pause();
      }
      currentState.mixed?.pause();
    }
    this.currentMusic = stems.values().next().value ?? null;
    this.currentMusicState = {
      source,
      stems,
      mixed: null,
      manifest,
      frameCursor: 0,
      gain: 1,
      role,
      token,
    };
    this._applyMusicState();
    for (const audio of stems.values()) {
      this._playAudioHandle(audio);
    }
  }

  private async _playMusicManifest(token: string, source: string, role: string, requestId: number): Promise<void> {
    const manifest = await this._loadManifest<MusicTrackManifest | PcmMusicTrackManifest>(source);
    if (requestId !== this.pendingMusicRequestId || this.currentMusicName !== token) {
      return;
    }
    if (!manifest) {
      if (this.currentMusicName === token) {
        this.currentMusicName = null;
        this.currentMusicRole = "general";
      }
      if (role === "map" && this.mapMusicName === token) {
        this.mapMusicName = null;
      }
      return;
    }
    if (this._isPcmMusicManifest(manifest)) {
      await this._playPcmMusicManifest(token, source, role, manifest);
      return;
    }
    if (this.currentMusicState) {
      const currentState = this.currentMusicState;
      for (const audio of currentState.stems.values()) {
        audio.pause();
      }
      currentState.mixed?.pause();
    }
    const stems = new Map<number, AudioHandle>();
    for (const stem of manifest.stems) {
      const audio = this._createAudio(stem.path, stem.loop ?? manifest.loop ?? true);
      if (audio) {
        stems.set(stem.channel, audio);
      }
    }
    let mixed: AudioHandle | null = null;
    if (stems.size === 0) {
      mixed = this._createAudio(manifest.mixedPath, manifest.loop ?? true);
    }
    this.currentMusic = stems.values().next().value ?? mixed;
    this.currentMusicState = {
      source,
      stems,
      mixed,
      manifest,
      frameCursor: 0,
      gain: 1,
      role,
      token,
    };
    this._applyMusicState();
    const audios = stems.size > 0 ? [...stems.values()] : (mixed ? [mixed] : []);
    for (const audio of audios) {
      this._playAudioHandle(audio);
    }
  }

  private async _playSoundManifest(
    token: string,
    source: string,
    options: ResolvedBattleSoundOptions,
  ): Promise<void> {
    const manifest = await this._loadManifest<SoundCueManifest | PcmClipManifest>(source);
    if (!manifest) {
      return;
    }
    if (this._isPcmClipManifest(manifest)) {
      const path = this._resolveManifestPath(source, manifest.path);
      const pcm = await this._loadPcmBytes(path);
      if (!pcm || !this._canPlaySfx(token)) {
        return;
      }
      const pan = this._resolveBattlePan(options.panning, options.tracks);
      const playbackRate = this._toPlaybackRate(options.pitch ?? null) ?? 1;
      const audio = this._createPcmHandle({
        kind: manifest.kind,
        token,
        source,
        pcm,
        sampleRate: manifest.sampleRate,
        loop: false,
        loopStartSample: null,
        loopEndSample: null,
        volume: this.masterVolume,
        muted: this.muted,
        pan,
        playbackRate,
      });
      if (!audio) {
        return;
      }
      this._recordPlaybackEvent({
        kind: getSoundChannelCategory(token),
        token,
        source,
        loop: false,
      });
      const ownedChannels = manifest.ownedChannels ?? [];
      if (ownedChannels.length > 0) {
        this.activeSoundChannels.set(audio, ownedChannels);
      }
      const priorityClass = manifest.priorityClass === "none" ? undefined : manifest.priorityClass;
      const releasePriority = this._trackPrioritySound(token, ownedChannels, priorityClass);
      this._scheduleSoundDuration(token, audio, options.duration ?? manifest.durationFrames ?? null);
      const active = this.activeSounds.get(token) ?? [];
      active.push(audio);
      this.activeSounds.set(token, active);
      this.activeSoundReleases.set(audio, { released: false, release: releasePriority });
      const cleanup = () => {
        this._releaseActiveSound(token, audio);
      };
      audio.addEventListener("ended", cleanup, { once: true });
      audio.addEventListener("pause", cleanup, { once: true });
      this._playAudioHandle(audio);
      return;
    }
    const assetPath = manifest.assetPath;
    if (!assetPath) {
      return;
    }
    this._playSoundSource(
      token,
      assetPath,
      true,
      {
        ...options,
        duration: options.duration ?? manifest?.durationFrames ?? null,
      },
      manifest?.ownedChannels ?? [],
      manifest?.priorityClass,
    );
  }

  private _applyMusicState(): void {
    const state = this.currentMusicState;
    if (!state) {
      return;
    }
    const applyVolume = (audio: AudioHandle) => {
      audio.volume = Math.max(0, Math.min(1, this.masterVolume * state.gain));
    };
    for (const audio of state.stems.values()) {
      applyVolume(audio);
    }
    if (state.mixed) {
      applyVolume(state.mixed);
    }
    this._syncMuteState();
  }

  private _resumeMusicState(): void {
    const state = this.currentMusicState;
    if (!state) {
      return;
    }
    const resumeAt = Math.max(0, state.frameCursor * FRAME_MS) / 1000;
    const audios = state.stems.size > 0 ? [...state.stems.values()] : (state.mixed ? [state.mixed] : []);
    for (const audio of audios) {
      if (audio.paused) {
        try {
          audio.currentTime = resumeAt;
        } catch {
          // ignore if media has not loaded enough metadata yet
        }
        this._playAudioHandle(audio);
      }
    }
  }

  private _finishFadeTransition(): void {
    const next = this.pendingMusicAfterFade;
    this.stopMusic();
    if (!next || this._isSilenceToken(next.token)) {
      return;
    }
    this.playMusic(next.token, next.role);
  }

  private _recomputeSuppressedMusicChannels(): void {
    const next = new Set<number>();
    for (const [audio, channels] of this.activeSoundChannels.entries()) {
      if (!this.activeSoundReleases.has(audio)) {
        continue;
      }
      for (const channel of channels) {
        if (channel >= 5 && channel <= 8) {
          next.add(channel - 4);
        } else if (channel >= 1 && channel <= 4) {
          next.add(channel);
        }
      }
    }
    this.suppressedMusicChannels = next;
    this._syncMuteState();
  }

}

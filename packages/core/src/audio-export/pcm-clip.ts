import * as fs from "fs";
import * as path from "path";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { WavConverter } from "./converter";
import { AsmAudioParser, DrumkitParser, WaveSampleParser, type ParsedMusicData } from "./parsers";
import type { NoiseNote } from "./schemas";
import { buildAsmAudioProgram, type AsmAudioProgramKind } from "./asm-programs";

export type PcmClipKind = "music" | "sfx" | "cry";
export type PcmPriorityClass = "none" | "priority" | "cry";

export type PcmClip = {
  kind: PcmClipKind;
  token: string;
  pcm: Int16Array;
  sampleRate: number;
  durationFrames: number;
  loopStartSample: number | null;
  loopEndSample: number | null;
  ownedChannels: number[];
  priorityClass: PcmPriorityClass;
  sourceKey?: string;
};

export type PcmClipManifest = {
  kind: PcmClipKind;
  token: string;
  path: string;
  sampleRate: number;
  channels: 2;
  bitsPerSample: 16;
  durationFrames: number;
  loopStartSample: number | null;
  loopEndSample: number | null;
  ownedChannels: number[];
  priorityClass: PcmPriorityClass;
};

export type PcmMusicStemManifest = PcmClipManifest & {
  kind: "music";
  channel: number;
};

export type PcmMusicTrackManifest = {
  kind: "music";
  token: string;
  sampleRate: number;
  channelCount: number;
  durationFrames: number;
  loopStartSample: number | null;
  loopEndSample: number | null;
  stems: PcmMusicStemManifest[];
};

export type PcmAudioManifest = PcmMusicTrackManifest | PcmClipManifest;

export type PcmRenderContext = {
  drumkits: Record<number, Record<number, NoiseNote[]>>;
  waveSamples: Record<number, number[]>;
  waveInstrumentMap?: Record<number, number>;
};

export type PcmJsonAudioProgram = {
  format: "pokecrystal.audio-program.json";
  version: 1;
  kind: PcmClipKind;
  token: string;
  musicData: ParsedMusicData;
  context: PcmRenderContext;
};

type RenderClipOptions = {
  kind: PcmClipKind;
  token: string;
  musicData: ParsedMusicData;
  context: PcmRenderContext;
  soloChannel?: number | null;
  priorityClass?: PcmPriorityClass;
  ownedChannels?: number[];
};

export function inferPcmPriorityClass(token: string, kind: PcmClipKind | "cries" | "sfx"): PcmPriorityClass {
  if (kind === "cry" || kind === "cries") {
    return "cry";
  }
  const normalized = String(token ?? "").trim().toUpperCase();
  if (!normalized.startsWith("SFX_")) {
    return "none";
  }
  if (
    normalized.startsWith("SFX_DEX_FANFARE_") ||
    normalized.startsWith("SFX_GET_") ||
    [
      "SFX_FANFARE",
      "SFX_FANFARE_2",
      "SFX_CAUGHT_MON",
      "SFX_LEVEL_UP",
      "SFX_REGISTER_PHONE_NUMBER",
      "SFX_PRESENT",
      "SFX_1ST_PLACE",
      "SFX_2ND_PLACE",
      "SFX_3RD_PLACE",
      "SFX_EVOLVED",
    ].includes(normalized)
  ) {
    return "priority";
  }
  return "none";
}

export function renderPcmClip({
  kind,
  token,
  musicData,
  context,
  soloChannel = null,
  priorityClass,
  ownedChannels,
}: RenderClipOptions): PcmClip {
  const converter = new WavConverter(
    musicData,
    context.drumkits,
    context.waveSamples,
    {
      waveInstrumentMap: context.waveInstrumentMap,
      loopedMusicExportSeconds: null,
      soloChannel,
    },
  );
  const rendered = converter.convert("pcm");
  const loopSamplesByChannel = rendered.metadata.loopSamplesByChannel ?? {};
  const primaryLoopChannel = soloChannel
    ?? Object.values(musicData.channels)
      .map((entry) => entry.number)
      .find((value): value is number => typeof value === "number")
    ?? null;
  const totalSamples = Math.floor(rendered.stereo.length / 2);
  const rawLoopStart = primaryLoopChannel == null ? null : loopSamplesByChannel[primaryLoopChannel] ?? null;
  const loopStartSample =
    kind === "music" && typeof rawLoopStart === "number" && rawLoopStart >= 0 && rawLoopStart < totalSamples
      ? rawLoopStart
      : null;
  const loopEndSample = loopStartSample == null ? null : totalSamples;
  const durationFrames = Math.max(
    1,
    Math.round((rendered.metadata.durationSeconds * 1000) / GB_FRAME_DURATION_MS),
  );
  return {
    kind,
    token,
    pcm: rendered.stereo,
    sampleRate: rendered.sampleRate,
    durationFrames,
    loopStartSample,
    loopEndSample,
    ownedChannels: ownedChannels ?? Object.values(musicData.channels)
      .map((entry) => entry.number)
      .filter((value): value is number => typeof value === "number"),
    priorityClass: priorityClass ?? inferPcmPriorityClass(token, kind),
  };
}

export function renderPcmMusicStems(
  token: string,
  musicData: ParsedMusicData,
  context: PcmRenderContext,
): PcmClip[] {
  return Object.values(musicData.channels)
    .map((entry) => entry.number)
    .filter((value): value is number => typeof value === "number")
    .sort((left, right) => left - right)
    .map((channel) => renderPcmClip({
      kind: "music",
      token,
      musicData,
      context,
      soloChannel: channel,
      ownedChannels: [channel],
      priorityClass: "none",
    }));
}

export function pcmClipToManifest(clip: PcmClip, path: string): PcmClipManifest {
  return {
    kind: clip.kind,
    token: clip.token,
    path,
    sampleRate: clip.sampleRate,
    channels: 2,
    bitsPerSample: 16,
    durationFrames: clip.durationFrames,
    loopStartSample: clip.loopStartSample,
    loopEndSample: clip.loopEndSample,
    ownedChannels: clip.ownedChannels,
    priorityClass: clip.priorityClass,
  };
}

export function pcmClipToBytes(clip: Pick<PcmClip, "pcm">): Uint8Array {
  const bytes = new Uint8Array(clip.pcm.length * 2);
  const view = new DataView(bytes.buffer);
  for (let i = 0; i < clip.pcm.length; i += 1) {
    view.setInt16(i * 2, clip.pcm[i], true);
  }
  return bytes;
}

export function pcmBytesToInt16(bytes: ArrayBuffer): Int16Array {
  const view = new DataView(bytes);
  const samples = new Int16Array(Math.floor(bytes.byteLength / 2));
  for (let offset = 0; offset + 1 < bytes.byteLength; offset += 2) {
    samples[offset / 2] = view.getInt16(offset, true);
  }
  return samples;
}

export function loadPcmRenderContext(audioRoot: string): PcmRenderContext {
  const drumkitsText = fs.readFileSync(path.join(audioRoot, "drumkits.asm"), "utf8");
  const waveSamplesText = fs.readFileSync(path.join(audioRoot, "wave_samples.asm"), "utf8");
  const drumkits = new DrumkitParser().parseFromText(drumkitsText);
  const waveSampleParser = new WaveSampleParser();
  const waveSamples = waveSampleParser.parseFromText(waveSamplesText);
  return {
    drumkits,
    waveSamples,
    waveInstrumentMap: waveSampleParser.instrumentMap,
  };
}

export function compileAsmAudioProgramToPcmJson(
  audioRoot: string,
  kind: AsmAudioProgramKind,
  stem: string,
  token: string,
): string | null {
  const program = buildAsmAudioProgram(audioRoot, kind, stem);
  if (!program) {
    return null;
  }
  const musicData = new AsmAudioParser(program.source).parse();
  const context = loadPcmRenderContext(audioRoot);
  const payload: PcmJsonAudioProgram = {
    format: "pokecrystal.audio-program.json",
    version: 1,
    kind: kind === "cry" ? "cry" : kind,
    token,
    musicData,
    context,
  };
  return JSON.stringify(payload);
}

const parsePcmJsonAudioProgram = (jsonText: string): PcmJsonAudioProgram => {
  const payload = JSON.parse(jsonText) as Partial<PcmJsonAudioProgram>;
  if (
    payload.format !== "pokecrystal.audio-program.json" ||
    payload.version !== 1 ||
    (payload.kind !== "music" && payload.kind !== "sfx" && payload.kind !== "cry") ||
    typeof payload.token !== "string" ||
    !payload.musicData ||
    !payload.context
  ) {
    throw new Error("Invalid PCM audio program JSON");
  }
  return payload as PcmJsonAudioProgram;
};

export function renderPcmClipFromJson(jsonText: string, options?: { soloChannel?: number | null; ownedChannels?: number[] }): PcmClip {
  const payload = parsePcmJsonAudioProgram(jsonText);
  return renderPcmClip({
    kind: payload.kind,
    token: payload.token,
    musicData: payload.musicData,
    context: payload.context,
    soloChannel: options?.soloChannel,
    ownedChannels: options?.ownedChannels,
    priorityClass: inferPcmPriorityClass(payload.token, payload.kind),
  });
}

export function renderPcmMusicStemsFromJson(jsonText: string): PcmClip[] {
  const payload = parsePcmJsonAudioProgram(jsonText);
  if (payload.kind !== "music") {
    throw new Error("PCM music stems require a music audio program JSON");
  }
  return renderPcmMusicStems(payload.token, payload.musicData, payload.context);
}

export function renderPcmClipFromAsm(
  audioRoot: string,
  kind: AsmAudioProgramKind,
  stem: string,
  token: string,
): PcmClip | null {
  const jsonText = compileAsmAudioProgramToPcmJson(audioRoot, kind, stem, token);
  return jsonText ? renderPcmClipFromJson(jsonText) : null;
}

export function renderPcmMusicStemsFromAsm(
  audioRoot: string,
  stem: string,
  token: string,
): PcmClip[] | null {
  const jsonText = compileAsmAudioProgramToPcmJson(audioRoot, "music", stem, token);
  return jsonText ? renderPcmMusicStemsFromJson(jsonText) : null;
}

import { NextResponse } from "next/server";
import path from "node:path";
import fs from "node:fs/promises";
import { createReadStream } from "node:fs";
import type { Stats } from "node:fs";
import { Readable } from "node:stream";
import { AsmAudioParser, DrumkitParser, WaveSampleParser } from "@pokecrystal/core/audio-export/parsers";
import { WavConverter } from "@pokecrystal/core/audio-export/converter";
import { buildAsmAudioProgram, type AsmAudioProgramKind } from "@pokecrystal/core/audio-export/asm-programs";
import {
  pcmClipToBytes,
  pcmClipToManifest,
  renderPcmClipFromAsm,
  renderPcmMusicStemsFromAsm,
  type PcmClip,
  type PcmMusicTrackManifest,
} from "@pokecrystal/core/audio-export/pcm-clip";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";

const DIRECT_AUDIO_ROOT_CANDIDATES = [
  path.resolve(process.cwd(), "public", "assets", "audio"),
  path.resolve(process.cwd(), "assets", "audio"),
];

const DIRECT_CONTENT_TYPES: Record<string, string> = {
  ".json": "application/json; charset=utf-8",
  ".mp3": "audio/mpeg",
  ".mid": "audio/midi",
  ".midi": "audio/midi",
  ".wav": "audio/wav",
};

const SYNTHESIZED_CACHE_CONTROL = "public, max-age=31536000, immutable";
const EXPORT_INFINITE_LOOP_REPEAT_LIMIT = 2;
const synthesizedAudioCache = new Map<string, Uint8Array>();
const synthesizedPcmClipCache = new Map<string, PcmClip>();
const synthesizedPcmMusicStemCache = new Map<string, PcmClip[]>();

type BundledAudioFile = {
  root: string;
  path: string;
};

type SynthesizedAudio = {
  body: Uint8Array;
  contentType: string;
  cacheKey: string;
};

type PcmRoutePayload =
  | {
      body: Uint8Array;
      contentType: string;
      cacheKey: string;
    }
  | {
      json: unknown;
      cacheKey: string;
    };

export const runtime = "nodejs";

const resolveDisassemblyAudioRoot = (): string =>
  path.join(
    process.env.POKECRYSTAL_DISASSEMBLY_ROOT
      ? path.resolve(process.env.POKECRYSTAL_DISASSEMBLY_ROOT)
      : getDisassemblyRoot(),
    "audio",
  );

function isSynthesisAllowed(parts: string[]): boolean {
  const ext = path.extname(parts.at(-1) ?? "").toLowerCase();
  if (ext === ".mid" || ext === ".midi") {
    return parts.length === 1;
  }
  if (ext === ".mp3" || ext === ".wav") {
    return parts.length === 2 && (parts[0] === "sfx" || parts[0] === "cries");
  }
  return false;
}

function resolveSafePath(root: string, parts: string[]): string | null {
  const resolved = path.resolve(root, ...parts);
  if (resolved === root || resolved.startsWith(`${root}${path.sep}`)) {
    return resolved;
  }
  return null;
}

async function findBundledAudioFile(parts: string[]): Promise<BundledAudioFile | null> {
  for (const root of DIRECT_AUDIO_ROOT_CANDIDATES) {
    const safePath = resolveSafePath(root, parts);
    if (!safePath) {
      continue;
    }
    try {
      const stat = await fs.stat(safePath);
      if (stat.isFile()) {
        return { root, path: safePath };
      }
    } catch {
      // keep checking other candidates
    }
  }
  return null;
}

async function buildSynthProgram(root: string, parts: string[]): Promise<{ cacheKey: string; source: string } | null> {
  const ext = path.extname(parts.at(-1) ?? "").toLowerCase();
  if (ext !== ".mid" && ext !== ".midi" && ext !== ".mp3" && ext !== ".wav") {
    return null;
  }

  const stem = path.basename(parts.at(-1) ?? "", ext);
  if (!stem) {
    return null;
  }

  let kind: AsmAudioProgramKind | null = null;
  if (parts.length === 1) {
    kind = "music";
  } else if (parts.length === 2 && parts[0] === "sfx") {
    kind = "sfx";
  } else if (parts.length === 2 && parts[0] === "cries") {
    kind = "cry";
  }

  if (!kind) {
    return null;
  }

  return buildAsmAudioProgram(root, kind, stem);
}

function createWavFromStereo16(interleavedStereo: Int16Array, sampleRate: number): Uint8Array {
  const channels = 2;
  const bitsPerSample = 16;
  const byteRate = sampleRate * channels * (bitsPerSample / 8);
  const blockAlign = channels * (bitsPerSample / 8);
  const dataBytes = interleavedStereo.length * 2;
  const buffer = new ArrayBuffer(44 + dataBytes);
  const view = new DataView(buffer);

  writeAscii(view, 0, "RIFF");
  view.setUint32(4, 36 + dataBytes, true);
  writeAscii(view, 8, "WAVE");
  writeAscii(view, 12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, channels, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, byteRate, true);
  view.setUint16(32, blockAlign, true);
  view.setUint16(34, bitsPerSample, true);
  writeAscii(view, 36, "data");
  view.setUint32(40, dataBytes, true);

  let offset = 44;
  for (let i = 0; i < interleavedStereo.length; i += 1) {
    view.setInt16(offset, interleavedStereo[i], true);
    offset += 2;
  }

  return new Uint8Array(buffer);
}

function writeAscii(view: DataView, offset: number, text: string): void {
  for (let i = 0; i < text.length; i += 1) {
    view.setUint8(offset + i, text.charCodeAt(i));
  }
}

async function synthesizeBundledAsmAudio(parts: string[]): Promise<SynthesizedAudio | null> {
  const synthRoots = [resolveDisassemblyAudioRoot()];
  for (const root of synthRoots) {
    const program = await buildSynthProgram(root, parts);
    if (!program) {
      continue;
    }

    const ext = path.extname(parts.at(-1) ?? "").toLowerCase();
    const synthFormat = ext === ".mid" || ext === ".midi" ? "midi" : "pcm";
    const contentType = synthFormat === "midi" ? "audio/midi" : "audio/wav";
    const cacheKey = `${program.cacheKey}:${synthFormat}`;
    const cached = synthesizedAudioCache.get(cacheKey);
    if (cached) {
      return {
        body: cached,
        contentType,
        cacheKey,
      };
    }

    const [drumkitsText, waveSamplesText] = await Promise.all([
      fs.readFile(path.join(root, "drumkits.asm"), "utf8"),
      fs.readFile(path.join(root, "wave_samples.asm"), "utf8"),
    ]);
    const musicData = new AsmAudioParser(program.source).parse();
    const drumkits = new DrumkitParser().parseFromText(drumkitsText);
    const waveSampleParser = new WaveSampleParser();
    const waveSamples = waveSampleParser.parseFromText(waveSamplesText);
    const converter = new WavConverter(
      musicData,
      drumkits,
      waveSamples,
      {
        waveInstrumentMap: waveSampleParser.instrumentMap,
        infiniteLoopRepeatLimit: EXPORT_INFINITE_LOOP_REPEAT_LIMIT,
      },
    );
    const rendered = converter.convert(synthFormat);
    const body = synthFormat === "midi"
      ? rendered.midiBytes ?? new Uint8Array()
      : createWavFromStereo16(rendered.stereo, rendered.sampleRate);
    synthesizedAudioCache.set(cacheKey, body);
    return {
      body,
      contentType,
      cacheKey,
    };
  }
  return null;
}

const pcmAudioPath = (...parts: string[]): string =>
  `/api/audio/${parts.map((part) => encodeURIComponent(part)).join("/")}`;

const parsePcmStem = (filename: string, expectedExt: ".json" | ".pcm"): string | null => {
  if (path.extname(filename).toLowerCase() !== expectedExt) {
    return null;
  }
  const stem = path.basename(filename, expectedExt);
  return stem && !stem.includes("/") && !stem.includes("\\") ? stem : null;
};

const loadPcmMusicStems = (audioRoot: string, stem: string, token: string): PcmClip[] | null => {
  const cacheKey = `music:${audioRoot}:${stem}`;
  const cached = synthesizedPcmMusicStemCache.get(cacheKey);
  if (cached) {
    return cached;
  }
  const rendered = renderPcmMusicStemsFromAsm(audioRoot, stem, token);
  if (!rendered) {
    return null;
  }
  synthesizedPcmMusicStemCache.set(cacheKey, rendered);
  return rendered;
};

const loadPcmClip = (
  audioRoot: string,
  kind: "sfx" | "cry",
  stem: string,
  token: string,
): PcmClip | null => {
  const cacheKey = `${kind}:${audioRoot}:${stem}`;
  const cached = synthesizedPcmClipCache.get(cacheKey);
  if (cached) {
    return cached;
  }
  const rendered = renderPcmClipFromAsm(audioRoot, kind, stem, token);
  if (!rendered) {
    return null;
  }
  synthesizedPcmClipCache.set(cacheKey, rendered);
  return rendered;
};

function buildPcmMusicManifest(stem: string, token: string, clips: PcmClip[]): PcmMusicTrackManifest | null {
  const first = clips[0];
  if (!first) {
    return null;
  }
  const stems = clips.map((clip) => {
    const channel = clip.ownedChannels[0] ?? 0;
    return {
      ...pcmClipToManifest(clip, pcmAudioPath("pcm", "music", stem, `ch${channel}.pcm`)),
      kind: "music" as const,
      channel,
    };
  });
  return {
    kind: "music",
    token,
    sampleRate: first.sampleRate,
    channelCount: stems.length,
    durationFrames: Math.max(...clips.map((clip) => clip.durationFrames)),
    loopStartSample: first.loopStartSample,
    loopEndSample: first.loopEndSample,
    stems,
  };
}

function synthesizePcmRoute(parts: string[]): PcmRoutePayload | null {
  if (parts[0] !== "pcm") {
    return null;
  }
  const audioRoot = resolveDisassemblyAudioRoot();
  const group = parts[1];

  if (group === "music") {
    if (parts.length === 3) {
      const stem = parsePcmStem(parts[2], ".json");
      if (!stem) {
        return null;
      }
      const clips = loadPcmMusicStems(audioRoot, stem, stem);
      const manifest = clips ? buildPcmMusicManifest(stem, stem, clips) : null;
      return manifest ? { json: manifest, cacheKey: `pcm:music:${stem}:manifest` } : null;
    }
    if (parts.length === 4) {
      const stem = parts[2];
      const channelMatch = parts[3].match(/^ch(\d+)\.pcm$/i);
      if (!stem || !channelMatch) {
        return null;
      }
      const channel = Number(channelMatch[1]);
      const clips = loadPcmMusicStems(audioRoot, stem, stem);
      const clip = clips?.find((entry) => entry.ownedChannels.includes(channel));
      return clip
        ? {
            body: pcmClipToBytes(clip),
            contentType: "application/octet-stream",
            cacheKey: `pcm:music:${stem}:ch${channel}`,
          }
        : null;
    }
    return null;
  }

  if (group !== "sfx" && group !== "cries") {
    return null;
  }
  if (parts.length !== 3) {
    return null;
  }
  const ext = path.extname(parts[2]).toLowerCase();
  if (ext !== ".json" && ext !== ".pcm") {
    return null;
  }
  const stem = parsePcmStem(parts[2], ext as ".json" | ".pcm");
  if (!stem) {
    return null;
  }
  const kind = group === "cries" ? "cry" : "sfx";
  const token = kind === "cry" ? `CRY_${stem.toUpperCase()}` : `SFX_${stem.toUpperCase()}`;
  const clip = loadPcmClip(audioRoot, kind, stem, token);
  if (!clip) {
    return null;
  }
  if (ext === ".json") {
    return {
      json: pcmClipToManifest(clip, pcmAudioPath("pcm", group, `${stem}.pcm`)),
      cacheKey: `pcm:${group}:${stem}:manifest`,
    };
  }
  return {
    body: pcmClipToBytes(clip),
    contentType: "application/octet-stream",
    cacheKey: `pcm:${group}:${stem}:raw`,
  };
}

export const buildAudioEtag = (stat: Pick<Stats, "mtimeMs" | "size">): string =>
  `"${Math.trunc(stat.mtimeMs).toString(16)}-${stat.size.toString(16)}"`;

export const isFreshAudioRequest = (request: Request, etag: string): boolean =>
  request.headers.get("if-none-match") === etag;

export async function GET(
  request: Request,
  { params }: { params: Promise<{ path?: string[] }> },
): Promise<Response> {
  const { path: parts = [] } = await params;
  if (parts[0] === "pcm") {
    try {
      const pcm = synthesizePcmRoute(parts);
      if (!pcm) {
        return new NextResponse("Not found", { status: 404 });
      }
      if ("json" in pcm) {
        const body = JSON.stringify(pcm.json);
        return new NextResponse(body, {
          status: 200,
          headers: {
            "Content-Type": "application/json; charset=utf-8",
            "Content-Length": String(Buffer.byteLength(body)),
            "Cache-Control": SYNTHESIZED_CACHE_CONTROL,
            ETag: `"pcm-${Buffer.from(pcm.cacheKey).toString("base64url")}"`,
          },
        });
      }
      const responseBodyBytes = Uint8Array.from(pcm.body);
      const responseBody = new Blob([responseBodyBytes as unknown as BlobPart], {
        type: pcm.contentType,
      });
      return new NextResponse(responseBody, {
        status: 200,
        headers: {
          "Content-Type": pcm.contentType,
          "Content-Length": String(pcm.body.byteLength),
          "Cache-Control": SYNTHESIZED_CACHE_CONTROL,
          ETag: `"pcm-${Buffer.from(pcm.cacheKey).toString("base64url")}-${pcm.body.byteLength.toString(16)}"`,
        },
      });
    } catch {
      return new NextResponse("Not found", { status: 404 });
    }
  }

  const bundledFile = await findBundledAudioFile(parts);

  if (bundledFile) {
    try {
      const stat = await fs.stat(bundledFile.path);
      const etag = buildAudioEtag(stat);
      if (isFreshAudioRequest(request, etag)) {
        return new NextResponse(null, {
          status: 304,
          headers: {
            ETag: etag,
            "Cache-Control": SYNTHESIZED_CACHE_CONTROL,
          },
        });
      }

      const ext = path.extname(bundledFile.path).toLowerCase();
      const contentType = DIRECT_CONTENT_TYPES[ext] ?? "application/octet-stream";
      const stream = Readable.toWeb(createReadStream(bundledFile.path)) as ReadableStream;
      return new NextResponse(stream, {
        status: 200,
        headers: {
          "Content-Type": contentType,
          "Content-Length": String(stat.size),
          ETag: etag,
          "Cache-Control": SYNTHESIZED_CACHE_CONTROL,
        },
      });
    } catch {
      return new NextResponse("Not found", { status: 404 });
    }
  }

  if (!isSynthesisAllowed(parts)) {
    return new NextResponse("Not found", { status: 404 });
  }

  try {
    const synthesized = await synthesizeBundledAsmAudio(parts);
    if (!synthesized) {
      return new NextResponse("Not found", { status: 404 });
    }
    const responseBodyBytes = Uint8Array.from(synthesized.body);
    const responseBody = new Blob([responseBodyBytes as unknown as BlobPart], {
      type: synthesized.contentType,
    });
    return new NextResponse(responseBody, {
      status: 200,
      headers: {
        "Content-Type": synthesized.contentType,
        "Content-Length": String(synthesized.body.byteLength),
        "Cache-Control": SYNTHESIZED_CACHE_CONTROL,
        ETag: `"synth-${Buffer.from(synthesized.cacheKey).toString("base64url")}-${synthesized.body.byteLength.toString(16)}"`,
      },
    });
  } catch {
    return new NextResponse("Not found", { status: 404 });
  }
}

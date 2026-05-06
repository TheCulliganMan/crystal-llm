import { NextResponse } from "next/server";
import path from "node:path";
import fs from "node:fs/promises";
import { createReadStream } from "node:fs";
import type { Stats } from "node:fs";
import { Readable } from "node:stream";
import { AsmAudioParser, DrumkitParser, WaveSampleParser } from "@pokecrystal/core/audio-export/parsers";
import { WavConverter } from "@pokecrystal/core/audio-export/converter";
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

type BundledAudioFile = {
  root: string;
  path: string;
};

type SynthesizedAudio = {
  body: Uint8Array;
  contentType: string;
  cacheKey: string;
};

export const runtime = "nodejs";

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

const normalizeAsmSlug = (value: string): string =>
  value.replace(/[^A-Za-z0-9]/g, "").toLowerCase();

const normalizeStandaloneLocalLabels = (sourceText: string): string =>
  sourceText.replace(/^(\s*)(\.[A-Za-z0-9_]+)\s*$/gm, "$1$2:");

function extractAsmProgram(sourceText: string, entryLabel: string): string | null {
  const lines = normalizeStandaloneLocalLabels(sourceText).split(/\r?\n/);
  const labelIndex = new Map<string, number>();
  for (let i = 0; i < lines.length; i += 1) {
    const match = lines[i].match(/^([A-Za-z0-9_.]+):\s*$/);
    if (match) {
      labelIndex.set(match[1], i);
    }
  }

  const readBlock = (label: string): string[] | null => {
    const start = labelIndex.get(label);
    if (start === undefined) {
      return null;
    }
    let end = lines.length;
    for (let i = start + 1; i < lines.length; i += 1) {
      if (/^[A-Za-z0-9_]+:\s*$/.test(lines[i])) {
        end = i;
        break;
      }
    }
    return lines.slice(start, end);
  };

  const queue = [entryLabel];
  const seen = new Set<string>();
  const blocks: string[] = [];

  while (queue.length > 0) {
    const label = queue.shift();
    if (!label || seen.has(label)) {
      continue;
    }
    seen.add(label);
    const block = readBlock(label);
    if (!block) {
      continue;
    }
    blocks.push(block.join("\n"));
    const blockText = block.join("\n");

    for (const match of blockText.matchAll(/^\s*channel\s+\d+\s*,\s*([A-Za-z0-9_.]+)/gm)) {
      queue.push(match[1]);
    }

    const owner = label.startsWith(".") ? null : label;
    for (const match of blockText.matchAll(/^\s*sound_call\s+([A-Za-z0-9_.]+)/gm)) {
      const raw = match[1];
      queue.push(raw.startsWith(".") && owner ? `${owner}${raw}` : raw);
    }
  }

  return blocks.length > 0 ? blocks.join("\n\n") : null;
}

async function loadAsmCollectionSource(
  root: string,
  collectionFile: string,
  requestStem: string,
): Promise<string | null> {
  const filePath = path.join(root, collectionFile);
  let sourceText: string;
  try {
    sourceText = normalizeStandaloneLocalLabels(await fs.readFile(filePath, "utf8"));
  } catch {
    return null;
  }
  const requestedSlug = normalizeAsmSlug(requestStem);
  const labels = Array.from(sourceText.matchAll(/^([A-Za-z0-9_]+):\s*$/gm))
    .map((match) => match[1]);
  const entryLabel = labels.find((label) => normalizeAsmSlug(label.replace(/^(Sfx|Cry)_/, "")) === requestedSlug);
  if (!entryLabel) {
    return null;
  }
  return extractAsmProgram(sourceText, entryLabel);
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

  if (parts.length === 1) {
    const musicPath = path.join(root, "music", `${stem}.asm`);
    try {
      const source = await fs.readFile(musicPath, "utf8");
      const normalizedSource = normalizeStandaloneLocalLabels(source);
      return {
        cacheKey: `music:${musicPath}`,
        source: normalizedSource,
      };
    } catch {
      return null;
    }
  }

  if (parts.length === 2 && parts[0] === "sfx") {
    const source = await loadAsmCollectionSource(root, "sfx.asm", stem)
      ?? await loadAsmCollectionSource(root, "sfx_crystal.asm", stem);
    if (!source) {
      return null;
    }
    return {
      cacheKey: `sfx:${stem}`,
      source,
    };
  }

  if (parts.length === 2 && parts[0] === "cries") {
    const source = await loadAsmCollectionSource(root, "cries.asm", stem);
    if (!source) {
      return null;
    }
    return {
      cacheKey: `cry:${stem}`,
      source,
    };
  }

  return null;
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
  const synthRoots = [path.join(getDisassemblyRoot(), "audio")];
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

export const buildAudioEtag = (stat: Pick<Stats, "mtimeMs" | "size">): string =>
  `"${Math.trunc(stat.mtimeMs).toString(16)}-${stat.size.toString(16)}"`;

export const isFreshAudioRequest = (request: Request, etag: string): boolean =>
  request.headers.get("if-none-match") === etag;

export async function GET(
  request: Request,
  { params }: { params: Promise<{ path?: string[] }> },
): Promise<Response> {
  const { path: parts = [] } = await params;
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

import { NextResponse } from "next/server";
import path from "node:path";
import fs from "node:fs/promises";
import { createReadStream } from "node:fs";
import type { Stats } from "node:fs";
import { Readable } from "node:stream";
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
  ".mid": "audio/midi",
  ".midi": "audio/midi",
  ".wav": "audio/wav",
};

const SYNTHESIZED_CACHE_CONTROL = "public, max-age=31536000, immutable";
const synthesizedPcmClipCache = new Map<string, PcmClip>();
const synthesizedPcmMusicStemCache = new Map<string, PcmClip[]>();

type BundledAudioFile = {
  root: string;
  path: string;
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
  if (parts.some((part) => path.extname(part).toLowerCase() === ".mp3")) {
    return new NextResponse("Not found", { status: 404 });
  }
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
  return new NextResponse("Not found", { status: 404 });
}

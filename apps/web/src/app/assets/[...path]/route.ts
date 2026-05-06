import { NextResponse } from "next/server";
import path from "node:path";
import fs from "node:fs/promises";
import { createReadStream, existsSync } from "node:fs";
import { Readable } from "node:stream";

const IMMUTABLE_CACHE_CONTROL = "public, max-age=31536000, immutable";

const CONTENT_TYPES: Record<string, string> = {
  ".json": "application/json; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
  ".asm": "text/plain; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".mjs": "application/javascript; charset=utf-8",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".webp": "image/webp",
  ".svg": "image/svg+xml; charset=utf-8",
  ".ico": "image/x-icon",
  ".mp3": "audio/mpeg",
  ".wav": "audio/wav",
  ".mid": "audio/midi",
  ".midi": "audio/midi",
  ".pal": "application/octet-stream",
  ".gbcpal": "application/octet-stream",
  ".2bpp": "application/octet-stream",
  ".1bpp": "application/octet-stream",
  ".lz": "application/octet-stream",
  ".bin": "application/octet-stream",
  ".tilemap": "application/octet-stream",
  ".attrmap": "application/octet-stream",
  ".rle": "application/octet-stream",
  ".mk": "text/plain; charset=utf-8",
};

export const runtime = "nodejs";

const resolveAssetsRoot = (): string => {
  const cwd = process.cwd();
  const candidates = [
    path.resolve(cwd, "assets"),
    path.resolve(cwd, "apps", "web", "assets"),
  ];
  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return candidate;
    }
  }
  return candidates[0];
};

const ASSETS_ROOT = resolveAssetsRoot();

const resolveSafePath = (parts: string[]): string | null => {
  const resolved = path.resolve(ASSETS_ROOT, ...parts);
  if (resolved === ASSETS_ROOT || resolved.startsWith(`${ASSETS_ROOT}${path.sep}`)) {
    return resolved;
  }
  return null;
};

const contentTypeFor = (filePath: string): string =>
  CONTENT_TYPES[path.extname(filePath).toLowerCase()] ?? "application/octet-stream";

async function serve(
  _request: Request,
  params: Promise<{ path: string[] }>,
  includeBody: boolean,
): Promise<Response> {
  const { path: parts } = await params;
  const safePath = resolveSafePath(parts);
  if (!safePath) {
    return NextResponse.json({ error: "Not found" }, { status: 404 });
  }

  let stat;
  try {
    stat = await fs.stat(safePath);
  } catch {
    return NextResponse.json({ error: "Not found" }, { status: 404 });
  }

  if (!stat.isFile()) {
    return NextResponse.json({ error: "Not found" }, { status: 404 });
  }

  const headers = new Headers({
    "Content-Type": contentTypeFor(safePath),
    "Cache-Control": IMMUTABLE_CACHE_CONTROL,
    "Content-Length": String(stat.size),
  });

  if (!includeBody) {
    return new Response(null, { status: 200, headers });
  }

  const stream = createReadStream(safePath);
  return new Response(Readable.toWeb(stream) as ReadableStream, {
    status: 200,
    headers,
  });
}

export async function GET(
  request: Request,
  context: { params: Promise<{ path: string[] }> },
): Promise<Response> {
  return serve(request, context.params, true);
}

export async function HEAD(
  request: Request,
  context: { params: Promise<{ path: string[] }> },
): Promise<Response> {
  return serve(request, context.params, false);
}

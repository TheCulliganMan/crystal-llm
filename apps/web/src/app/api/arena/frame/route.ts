import { NextResponse } from "next/server";
import { isRequestAuthorized, SESSION_ID_REGEX } from "@/app/mcp/session-guards";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const noStoreHeaders = {
  "Cache-Control": "no-store, max-age=0, must-revalidate",
};
const MCP_INSTANT_MODE = true;

const jsonError = (status: number, message: string) =>
  NextResponse.json({ ok: false, error: message }, { status, headers: noStoreHeaders });

const resolveSessionId = (request: Request): string | undefined => {
  const url = new URL(request.url);
  const keys = ["session_id", "session", "user"];
  for (const key of keys) {
    const value = url.searchParams.get(key);
    if (!value) {
      continue;
    }
    const trimmed = value.trim();
    if (!trimmed) {
      continue;
    }
    if (!SESSION_ID_REGEX.test(trimmed)) {
      throw new Error("Invalid session id.");
    }
    return trimmed;
  }
  return undefined;
};

const resolveScale = (request: Request): number => {
  const url = new URL(request.url);
  const raw = url.searchParams.get("scale");
  if (!raw) {
    return 2;
  }
  const parsed = Number(raw);
  if (!Number.isFinite(parsed)) {
    throw new Error("Scale must be a number.");
  }
  const normalized = Math.floor(parsed);
  if (normalized < 1 || normalized > 8) {
    throw new Error("Scale must be between 1 and 8.");
  }
  return normalized;
};

const resolveAdvanceFrames = (request: Request): number => {
  const url = new URL(request.url);
  const raw = url.searchParams.get("advance");
  if (!raw) {
    return 0;
  }
  const parsed = Number(raw);
  if (!Number.isFinite(parsed)) {
    throw new Error("Advance must be a number.");
  }
  const normalized = Math.floor(parsed);
  const maxAdvance = MCP_INSTANT_MODE ? 3600 : 120;
  if (normalized < 0 || normalized > maxAdvance) {
    throw new Error(`Advance must be between 0 and ${maxAdvance}.`);
  }
  return normalized;
};

const resolveInstantMode = (request: Request): boolean | undefined => {
  const url = new URL(request.url);
  const raw = url.searchParams.get("instant");
  if (raw === null) {
    return undefined;
  }
  const normalized = raw.trim().toLowerCase();
  if (normalized === "1" || normalized === "true" || normalized === "on") {
    return true;
  }
  if (normalized === "0" || normalized === "false" || normalized === "off") {
    return false;
  }
  throw new Error("Instant must be true or false.");
};

export async function GET(request: Request) {
  try {
    const token = process.env.POKECRYSTAL_ARENA_SNAPSHOT_TOKEN ?? "";
    if (!isRequestAuthorized(request, token)) {
      return new NextResponse("Unauthorized", { status: 401, headers: noStoreHeaders });
    }

    let sessionId: string | undefined;
    let scale: number;
    let advanceFrames: number;
    let instantMode: boolean | undefined;
    try {
      sessionId = resolveSessionId(request);
      scale = resolveScale(request);
      advanceFrames = resolveAdvanceFrames(request);
      instantMode = resolveInstantMode(request);
    } catch (error) {
      return jsonError(400, error instanceof Error ? error.message : "Invalid request.");
    }

    // Load the MCP runtime lazily so Next's build-time route analysis does not
    // have to evaluate the full session stack for this dynamic API route.
    const { getMcpSession } = await import("@/app/mcp/session");
    const session = getMcpSession(sessionId);
    if (instantMode !== undefined) {
      session.setInstantMode(instantMode);
    }
    if (advanceFrames > 0) {
      await session.advanceFrames(advanceFrames);
    } else {
      await session.ensureReady();
    }
    const image = await session.observeTilemapImage({ scale });
    return NextResponse.json(
      {
        ok: true,
        image: image.data,
        width: image.width,
        height: image.height,
        frame: session.getFrameCount(),
      },
      { headers: noStoreHeaders }
    );
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unable to render frame.";
    try {
      return jsonError(500, message);
    } catch {
      return new NextResponse("Internal Server Error", {
        status: 500,
        headers: noStoreHeaders,
      });
    }
  }
}

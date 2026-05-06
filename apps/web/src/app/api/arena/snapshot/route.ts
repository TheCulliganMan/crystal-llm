import { NextResponse } from "next/server";
import { isRequestAuthorized, SESSION_ID_REGEX } from "@/app/mcp/session-guards";
export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const noStoreHeaders = {
  "Cache-Control": "no-store, max-age=0, must-revalidate",
};

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

export async function GET(request: Request) {
  const token = process.env.POKECRYSTAL_ARENA_SNAPSHOT_TOKEN ?? "";
  if (!isRequestAuthorized(request, token)) {
    return new NextResponse("Unauthorized", { status: 401 });
  }
  try {
    const { getMcpSession } = await import("@/app/mcp/session");
    const sessionId = resolveSessionId(request);
    const session = getMcpSession(sessionId);
    await session.ensureReady();
    const payload = session.observePayload();
    const text = session.observeText();
    return NextResponse.json(
      { ok: true, payload, text, map: payload?.map ?? null, flow_state: payload?.flow_state ?? null },
      { headers: noStoreHeaders }
    );
  } catch (error) {
    console.error("[arena snapshot]", error);
    const message = error instanceof Error ? error.message : "Unknown error";
    const status = message.includes("Invalid session id") ? 400 : 500;
    return NextResponse.json(
      { ok: false, error: message },
      { status, headers: noStoreHeaders }
    );
  }
}

import { NextResponse } from "next/server";
import { isRequestAuthorized, SESSION_ID_REGEX } from "@/app/mcp/session-guards";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const noStoreHeaders = {
  "Cache-Control": "no-store, max-age=0, must-revalidate",
};

type InputPayload = {
  session_id?: unknown;
  key?: unknown;
  direction?: unknown;
  button?: unknown;
  is_press?: unknown;
  instant?: unknown;
};

const jsonError = (status: number, message: string) =>
  NextResponse.json({ ok: false, error: message }, { status, headers: noStoreHeaders });

const parseBoolean = (value: unknown, name: string): boolean => {
  if (value === true || value === false) {
    return value;
  }
  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    if (normalized === "1" || normalized === "true" || normalized === "on") {
      return true;
    }
    if (normalized === "0" || normalized === "false" || normalized === "off") {
      return false;
    }
  }
  throw new Error(`${name} must be true or false.`);
};

const parseOptionalString = (value: unknown, name: string): string | null => {
  if (value === undefined || value === null) {
    return null;
  }
  if (typeof value !== "string") {
    throw new Error(`${name} must be a string.`);
  }
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
};

export async function POST(request: Request) {
  try {
    const token = process.env.POKECRYSTAL_ARENA_SNAPSHOT_TOKEN ?? "";
    if (!isRequestAuthorized(request, token)) {
      return new NextResponse("Unauthorized", { status: 401, headers: noStoreHeaders });
    }

    const payload = (await request.json()) as InputPayload;
    const sessionId = parseOptionalString(payload.session_id, "session_id");
    if (sessionId && !SESSION_ID_REGEX.test(sessionId)) {
      return jsonError(400, "Invalid session id.");
    }
    const key = parseOptionalString(payload.key, "key");
    if (!key) {
      return jsonError(400, "key is required.");
    }
    const instant = payload.instant === undefined ? undefined : parseBoolean(payload.instant, "instant");
    const { getMcpSession } = await import("@/app/mcp/session");
    const session = getMcpSession(sessionId ?? undefined);
    if (instant !== undefined) {
      session.setInstantMode(instant);
    }
    await session.postInputEvent({
      key,
      direction: parseOptionalString(payload.direction, "direction"),
      button: parseOptionalString(payload.button, "button"),
      isPress: parseBoolean(payload.is_press, "is_press"),
    });
    return NextResponse.json({ ok: true }, { headers: noStoreHeaders });
  } catch (error) {
    return jsonError(400, error instanceof Error ? error.message : "Invalid input.");
  }
}

import { NextResponse } from "next/server";
import { clampWatchSessionLimit, DEFAULT_WATCH_SESSION_LIMIT, resolveWatchRuns } from "@/arena/watch-resolver";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const noStoreHeaders = {
  "Cache-Control": "no-store, max-age=0, must-revalidate",
};

export async function GET(request: Request) {
  const url = new URL(request.url);
  const limitRaw = Number(url.searchParams.get("limit") ?? DEFAULT_WATCH_SESSION_LIMIT.toString());
  const limit = clampWatchSessionLimit(limitRaw);
  const resolution = await resolveWatchRuns(limit);
  if (!resolution.ok) {
    return NextResponse.json(
      { ok: false, error: resolution.error, runs: [] },
      { status: 500, headers: noStoreHeaders }
    );
  }
  return NextResponse.json(
    { ok: true, runs: resolution.runs, warning: resolution.warning },
    { headers: noStoreHeaders }
  );
}

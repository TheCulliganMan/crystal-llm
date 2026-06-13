import { NextResponse } from "next/server";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const removed = () =>
  NextResponse.json(
    { ok: false, error: "Progress endpoint removed." },
    { status: 404, headers: { "Cache-Control": "no-store, max-age=0, must-revalidate" } }
  );

export const GET = removed;
export const POST = removed;

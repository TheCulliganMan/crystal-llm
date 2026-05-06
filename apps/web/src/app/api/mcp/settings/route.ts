import { NextResponse } from "next/server";

export const runtime = "nodejs";
const MCP_INSTANT_MODE = true;
const STATIC_SETTINGS_HEADERS = {
  "Cache-Control": "public, max-age=300, s-maxage=300, stale-while-revalidate=600",
} as const;

export async function GET() {
  return NextResponse.json({ mcpInstantMode: MCP_INSTANT_MODE }, { headers: STATIC_SETTINGS_HEADERS });
}

export async function POST() {
  return NextResponse.json(
    { ok: false, error: "MCP instant mode is always enabled." },
    { status: 400 }
  );
}

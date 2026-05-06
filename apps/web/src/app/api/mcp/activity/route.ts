import { NextResponse } from "next/server";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const noStoreHeaders = {
  "Cache-Control": "no-store, max-age=0, must-revalidate",
};

export async function GET() {
  try {
    // Defer MCP session runtime loading until request handling so Next/Vercel
    // build analysis does not evaluate the full ASM-backed session stack.
    const { getMcpActivitySummary } = await import("@/app/mcp/session");
    const summary = getMcpActivitySummary();
    return NextResponse.json(
      {
        ok: true,
        apiSkillsMcpCount: summary.activeSessions,
      },
      { headers: noStoreHeaders }
    );
  } catch (_error) {
    return NextResponse.json(
      {
        ok: true,
        apiSkillsMcpCount: 0,
      },
      { headers: noStoreHeaders }
    );
  }
}

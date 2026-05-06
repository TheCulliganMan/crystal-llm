import { NextResponse } from "next/server";
import { createClient } from "@supabase/supabase-js";
import { buildSessionSecret, claimSessionOwnership, SESSION_ID_REGEX } from "@/app/mcp/session-guards";
import { identityTokenFromHeaders, parseIdentityToken } from "@/app/api/[transport]/tools/identity";
import { getSupabaseServiceRoleConfig } from "@/lib/supabase/env";
import type { Database } from "@/lib/supabase/types";
import type { Json } from "@/lib/supabase/types";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const noStoreHeaders = {
  "Cache-Control": "no-store, max-age=0, must-revalidate",
};

const resolveSessionId = (request: Request): string => {
  const url = new URL(request.url);
  const value = url.searchParams.get("session_id")?.trim() ?? "";
  if (!value || !SESSION_ID_REGEX.test(value)) {
    throw new Error("Valid session_id query parameter is required.");
  }
  return value;
};

const resolveRunOwnerPlayerId = async (sessionId: string): Promise<string | null> => {
  const config = getSupabaseServiceRoleConfig();
  if (!config) {
    return null;
  }
  try {
    const supabase = createClient<Database>(config.url, config.serviceRoleKey, {
      auth: {
        autoRefreshToken: false,
        persistSession: false,
      },
    });
    const { data, error } = await supabase
      .from("arena_runs")
      .select("metrics")
      .contains("metrics", { session_id: sessionId })
      .order("updated_at", { ascending: false })
      .limit(1)
      .maybeSingle();
    if (error) {
      return null;
    }
    const metrics = data?.metrics;
    if (!metrics || typeof metrics !== "object" || Array.isArray(metrics)) {
      return null;
    }
    const ownerPlayerId = (metrics as Record<string, Json>).owner_player_id;
    if (typeof ownerPlayerId !== "string") {
      return null;
    }
    const normalized = ownerPlayerId.trim();
    return normalized || null;
  } catch {
    return null;
  }
};

export async function GET(request: Request) {
  try {
    let sessionId: string;
    try {
      sessionId = resolveSessionId(request);
    } catch (error) {
      return NextResponse.json(
        { ok: false, error: error instanceof Error ? error.message : "Invalid session id." },
        { status: 400, headers: noStoreHeaders }
      );
    }

    const token = identityTokenFromHeaders(request.headers);
    if (!token) {
      return NextResponse.json(
        { ok: false, error: "Missing identity token." },
        { status: 401, headers: noStoreHeaders }
      );
    }
    const claims = parseIdentityToken(token);
    if (!claims) {
      return NextResponse.json(
        { ok: false, error: "Invalid identity token." },
        { status: 401, headers: noStoreHeaders }
      );
    }

    const runOwnerPlayerId = await resolveRunOwnerPlayerId(sessionId);
    if (runOwnerPlayerId && runOwnerPlayerId !== claims.playerId) {
      return NextResponse.json(
        { ok: false, error: "Session is already bound to a different identity." },
        { status: 403, headers: noStoreHeaders }
      );
    }

    if (!claimSessionOwnership(sessionId, claims.playerId)) {
      return NextResponse.json(
        { ok: false, error: "Session is already owned by a different identity." },
        { status: 403, headers: noStoreHeaders }
      );
    }

    const sessionSecret = buildSessionSecret(sessionId, claims.playerId);
    return NextResponse.json(
      {
        ok: true,
        sessionId,
        playerId: claims.playerId,
        sessionSecret,
        note: "Save this secret and send it on every MCP/tools/play request for this session.",
      },
      { headers: noStoreHeaders }
    );
  } catch (error) {
    console.error("[arena/session-secret] unexpected failure", error);
    return NextResponse.json(
      {
        ok: false,
        error: error instanceof Error ? error.message : "Unexpected session secret failure.",
      },
      { status: 500, headers: noStoreHeaders }
    );
  }
}

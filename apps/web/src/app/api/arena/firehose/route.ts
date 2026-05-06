import { NextResponse } from "next/server";
import { isRequestAuthorized } from "@/app/mcp/session-guards";
import { buildFirehoseRecord, encodeFirehoseRecords, parseFirehoseQuery } from "@/arena/training-firehose";
import type { FirehoseRow } from "@/arena/training-firehose";
import { createSupabaseServiceRoleClient } from "@/lib/supabase/server";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const noStoreHeaders = {
  "Cache-Control": "no-store, max-age=0, must-revalidate",
};

const jsonError = (status: number, message: string) =>
  NextResponse.json({ ok: false, error: message }, { status, headers: noStoreHeaders });

export async function GET(request: Request) {
  const token = process.env.POKECRYSTAL_TRAINING_FIREHOSE_TOKEN ?? "";
  if (!isRequestAuthorized(request, token)) {
    return new NextResponse("Unauthorized", { status: 401, headers: noStoreHeaders });
  }

  let query;
  try {
    query = parseFirehoseQuery(new URL(request.url));
  } catch (error) {
    return jsonError(400, error instanceof Error ? error.message : "Invalid request.");
  }

  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    return jsonError(503, "Supabase service role is not configured.");
  }

  let supabaseQuery = supabase
    .from("arena_run_events")
    .select("id, run_id, frame, label, payload, created_at")
    .order("id", { ascending: true })
    .limit(query.limit);

  if (query.afterId > 0) {
    supabaseQuery = supabaseQuery.gt("id", query.afterId);
  }
  if (query.runId) {
    supabaseQuery = supabaseQuery.eq("run_id", query.runId);
  }
  if (query.label) {
    supabaseQuery = supabaseQuery.eq("label", query.label);
  }

  const { data, error } = await supabaseQuery;
  if (error) {
    return jsonError(500, error.message);
  }

  const rows = (data ?? []) as FirehoseRow[];
  const records = rows.map(buildFirehoseRecord);
  const body = encodeFirehoseRecords(records);

  const headers = new Headers(noStoreHeaders);
  headers.set("Content-Type", "application/x-ndjson; charset=utf-8");
  headers.set("X-Result-Count", String(records.length));
  const nextCursor = records.length ? records[records.length - 1].id : query.afterId;
  headers.set("X-Next-Cursor", String(nextCursor));
  return new Response(body, { headers });
}

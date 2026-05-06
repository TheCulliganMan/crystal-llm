import { NextResponse } from "next/server";
import { isRequestAuthorized, SESSION_ID_REGEX, verifySessionSecret } from "@/app/mcp/session-guards";
import { hasValidIdentityToken } from "@/app/api/[transport]/tools/identity";
import { createSupabaseServiceRoleClient } from "@/lib/supabase/server";
import { slugifyAgentName } from "@/arena/utils";
import type { Json, Tables, TablesInsert, TablesUpdate } from "@/lib/supabase/types";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const noStoreHeaders = {
  "Cache-Control": "no-store, max-age=0, must-revalidate",
};

const STATUS_VALUES = new Set(["queued", "running", "completed", "failed", "cancelled"]);
const UUID_LIKE = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

type ProgressPayload = {
  sessionId: string;
  agentName: string;
  status?: "queued" | "running" | "completed" | "failed" | "cancelled";
  queue?: string;
  frameCount?: number;
  badgeCount?: number;
  pokedexSeen?: number;
  pokedexCaught?: number;
  stepCount?: number;
  instructionCount?: number;
  runtime?: string;
  repoUrl?: string | null;
  modelUrl?: string | null;
  note?: string | null;
  error?: string | null;
  flowState?: {
    summary?: string;
    nextGoal?: string | null;
    completionTarget?: string | null;
    completedIds?: string[];
  } | null;
};

const clampNonNegativeInt = (value: unknown): number | null => {
  if (value === null || value === undefined || value === "") {
    return null;
  }
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return null;
  }
  return Math.max(0, Math.trunc(parsed));
};

const parsePayload = (raw: unknown): ProgressPayload => {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error("Request body must be a JSON object.");
  }
  const body = raw as Record<string, unknown>;
  const sessionId = String(body.sessionId ?? body.session_id ?? "").trim();
  const agentName = String(body.agentName ?? body.agent_name ?? "").trim();
  const statusValue = body.status === undefined ? undefined : String(body.status).trim().toLowerCase();
  const status = statusValue && STATUS_VALUES.has(statusValue)
    ? (statusValue as ProgressPayload["status"])
    : undefined;
  if (!sessionId || !SESSION_ID_REGEX.test(sessionId)) {
    throw new Error("sessionId is required and must match the MCP session format.");
  }
  if (!agentName || agentName.length > 80) {
    throw new Error("agentName is required and must be 1-80 characters.");
  }
  if (statusValue && !status) {
    throw new Error("status must be queued, running, completed, failed, or cancelled.");
  }
  return {
    sessionId,
    agentName,
    status,
    queue: String(body.queue ?? "main").trim() || "main",
    frameCount: clampNonNegativeInt(body.frameCount ?? body.frame_count) ?? undefined,
    badgeCount: clampNonNegativeInt(body.badgeCount ?? body.badge_count) ?? undefined,
    pokedexSeen: clampNonNegativeInt(body.pokedexSeen ?? body.pokedex_seen) ?? undefined,
    pokedexCaught: clampNonNegativeInt(body.pokedexCaught ?? body.pokedex_caught) ?? undefined,
    stepCount: clampNonNegativeInt(body.stepCount ?? body.step_count) ?? undefined,
    instructionCount: clampNonNegativeInt(body.instructionCount ?? body.instruction_count) ?? undefined,
    runtime: String(body.runtime ?? "mcp-http").trim() || "mcp-http",
    repoUrl: body.repoUrl === undefined ? null : String(body.repoUrl ?? "").trim() || null,
    modelUrl: body.modelUrl === undefined ? null : String(body.modelUrl ?? "").trim() || null,
    note: body.note === undefined ? null : String(body.note ?? "").trim() || null,
    error: body.error === undefined ? null : String(body.error ?? "").trim() || null,
    flowState:
      body.flowState && typeof body.flowState === "object"
        ? {
            summary:
              typeof (body.flowState as Record<string, unknown>).summary === "string"
                ? String((body.flowState as Record<string, unknown>).summary).trim()
                : undefined,
            nextGoal:
              typeof (body.flowState as Record<string, unknown>).nextGoal === "string"
                ? String((body.flowState as Record<string, unknown>).nextGoal).trim()
                : null,
            completionTarget:
              typeof (body.flowState as Record<string, unknown>).completionTarget === "string"
                ? String((body.flowState as Record<string, unknown>).completionTarget).trim()
                : null,
            completedIds: Array.isArray((body.flowState as Record<string, unknown>).completedIds)
              ? ((body.flowState as Record<string, unknown>).completedIds as unknown[])
                  .filter((value): value is string => typeof value === "string" && value.trim().length > 0)
                  .map((value) => value.trim())
              : [],
          }
        : body.flow_state && typeof body.flow_state === "object"
          ? {
              summary:
                typeof (body.flow_state as Record<string, unknown>).summary === "string"
                  ? String((body.flow_state as Record<string, unknown>).summary).trim()
                  : undefined,
              nextGoal:
                typeof (body.flow_state as Record<string, unknown>).nextGoal === "string"
                  ? String((body.flow_state as Record<string, unknown>).nextGoal).trim()
                  : null,
              completionTarget:
                typeof (body.flow_state as Record<string, unknown>).completionTarget === "string"
                  ? String((body.flow_state as Record<string, unknown>).completionTarget).trim()
                  : null,
              completedIds: Array.isArray((body.flow_state as Record<string, unknown>).completedIds)
                ? ((body.flow_state as Record<string, unknown>).completedIds as unknown[])
                    .filter((value): value is string => typeof value === "string" && value.trim().length > 0)
                    .map((value) => value.trim())
                : [],
            }
          : null,
  };
};

const resolveSystemOwnerId = async (): Promise<string | null> => {
  const configured = (process.env.POKECRYSTAL_MCP_SYSTEM_USER_ID ?? "").trim();
  if (UUID_LIKE.test(configured)) {
    return configured;
  }
  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    return null;
  }
  const { data, error } = await supabase
    .from("arena_agents")
    .select("owner_id")
    .limit(1)
    .maybeSingle();
  if (error) {
    return null;
  }
  const owner = data?.owner_id ?? null;
  return owner && UUID_LIKE.test(owner) ? owner : null;
};

const mergeMetricCount = (metrics: Record<string, Json>, key: string, incoming?: number): void => {
  if (incoming === undefined) {
    return;
  }
  const existingRaw = metrics[key];
  const existing = typeof existingRaw === "number" && Number.isFinite(existingRaw) ? Math.trunc(existingRaw) : 0;
  metrics[key] = Math.max(existing, incoming);
};

const buildProgressEventPayload = (
  payload: ProgressPayload,
  status: NonNullable<ProgressPayload["status"]> | Tables<"arena_runs">["status"],
  created: boolean
): Json => ({
  session_id: payload.sessionId,
  created,
  agent_name: payload.agentName,
  status,
  queue: payload.queue ?? "main",
  frame_count: payload.frameCount ?? null,
  badge_count: payload.badgeCount ?? null,
  pokedex_seen: payload.pokedexSeen ?? null,
  pokedex_caught: payload.pokedexCaught ?? null,
  step_count: payload.stepCount ?? null,
  instruction_count: payload.instructionCount ?? null,
  runtime: payload.runtime ?? "mcp-http",
  note: payload.note ?? null,
  error: payload.error ?? null,
  flow_state: payload.flowState ?? null,
});

export async function POST(request: Request) {
  const configuredToken = (
    process.env.POKECRYSTAL_ARENA_PROGRESS_TOKEN ??
    process.env.POKECRYSTAL_ARENA_SNAPSHOT_TOKEN ??
    process.env.POKECRYSTAL_MCP_TOKEN ??
    ""
  ).trim();

  const tokenProtected = configuredToken.length > 0;
  const staticAuthorized = tokenProtected ? isRequestAuthorized(request, configuredToken) : false;
  const identityAuthorized = hasValidIdentityToken(request.headers);

  if (tokenProtected && !staticAuthorized && !identityAuthorized) {
    return NextResponse.json({ ok: false, error: "Unauthorized" }, { status: 401, headers: noStoreHeaders });
  }

  let payload: ProgressPayload;
  try {
    payload = parsePayload(await request.json());
  } catch (error) {
    return NextResponse.json(
      { ok: false, error: error instanceof Error ? error.message : "Invalid request body." },
      { status: 400, headers: noStoreHeaders }
    );
  }
  const secretCheck = verifySessionSecret(request, payload.sessionId);
  if (!secretCheck.ok) {
    return NextResponse.json(
      { ok: false, error: secretCheck.message ?? "Unauthorized" },
      { status: secretCheck.status, headers: noStoreHeaders }
    );
  }

  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    return NextResponse.json(
      { ok: false, error: "Supabase service role is not configured." },
      { status: 503, headers: noStoreHeaders }
    );
  }

  const ownerId = await resolveSystemOwnerId();
  if (!ownerId) {
    return NextResponse.json(
      { ok: false, error: "No system owner is configured for arena progress ingest." },
      { status: 503, headers: noStoreHeaders }
    );
  }

  const now = new Date().toISOString();
  const agentInsert: TablesInsert<"arena_agents"> = {
    owner_id: ownerId,
    name: payload.agentName,
    slug: slugifyAgentName(payload.agentName),
    runtime: payload.runtime ?? "mcp-http",
    visibility: "public",
    repo_url: payload.repoUrl ?? null,
    config: {
      huggingfaceModel: payload.modelUrl ?? null,
      progress_api: true,
    },
    updated_at: now,
  };

  const { data: agent, error: agentError } = await supabase
    .from("arena_agents")
    .upsert(agentInsert, { onConflict: "owner_id,name" })
    .select("id,name")
    .maybeSingle();
  if (agentError || !agent?.id) {
    return NextResponse.json(
      { ok: false, error: `Failed to upsert agent: ${agentError?.message ?? "Unknown error"}` },
      { status: 500, headers: noStoreHeaders }
    );
  }

  const { data: existingRunData, error: existingRunError } = await supabase
    .from("arena_runs")
    .select("*")
    .eq("agent_id", agent.id)
    .contains("metrics", { session_id: payload.sessionId })
    .order("updated_at", { ascending: false })
    .limit(1)
    .maybeSingle();
  if (existingRunError) {
    return NextResponse.json(
      { ok: false, error: `Failed to query run progress: ${existingRunError.message}` },
      { status: 500, headers: noStoreHeaders }
    );
  }
  const existingRun = (existingRunData ?? null) as Tables<"arena_runs"> | null;

  const status = payload.status ?? (existingRun ? existingRun.status : "running");
  const metrics = existingRun?.metrics && typeof existingRun.metrics === "object" && !Array.isArray(existingRun.metrics)
    ? ({ ...(existingRun.metrics as Record<string, Json>) })
    : {};
  const ownerPlayerId =
    typeof metrics.owner_player_id === "string" ? metrics.owner_player_id.trim() : "";
  if (ownerPlayerId && secretCheck.playerId && ownerPlayerId !== secretCheck.playerId) {
    return NextResponse.json(
      { ok: false, error: "Session is already bound to a different identity." },
      { status: 403, headers: noStoreHeaders }
    );
  }
  metrics.session_id = payload.sessionId;
  if (secretCheck.playerId) {
    metrics.owner_player_id = secretCheck.playerId;
  }
  metrics.agent_name = payload.agentName;
  mergeMetricCount(metrics, "step_count", payload.stepCount);
  mergeMetricCount(metrics, "steps_taken", payload.stepCount);
  mergeMetricCount(metrics, "command_count", payload.instructionCount);
  mergeMetricCount(metrics, "commands", payload.instructionCount);
  if (payload.flowState) {
    metrics.flow_state = payload.flowState as Json;
    if (payload.flowState.nextGoal) {
      metrics.flow_next_goal = payload.flowState.nextGoal;
    }
    if (payload.flowState.completionTarget) {
      metrics.flow_completion_target = payload.flowState.completionTarget;
    }
    if (payload.flowState.summary) {
      metrics.flow_summary = payload.flowState.summary;
    }
    if (payload.flowState.completedIds?.length) {
      metrics.flow_completed_ids = payload.flowState.completedIds;
    }
  }

  const frameCount = payload.frameCount ?? existingRun?.frame_count ?? null;
  const badgeCount = payload.badgeCount ?? existingRun?.badge_count ?? null;
  const pokedexSeen = payload.pokedexSeen ?? existingRun?.pokedex_seen ?? null;
  const pokedexCaught = payload.pokedexCaught ?? existingRun?.pokedex_caught ?? null;

  if (!existingRun) {
    const insert: TablesInsert<"arena_runs"> = {
      agent_id: agent.id,
      created_by: ownerId,
      status,
      queue: payload.queue ?? "main",
      started_at: status === "queued" ? null : now,
      finished_at: status === "completed" || status === "failed" || status === "cancelled" ? now : null,
      frame_count: frameCount,
      badge_count: badgeCount,
      pokedex_seen: pokedexSeen,
      pokedex_caught: pokedexCaught,
      mcp_session_url: `/api/mcp?session_id=${encodeURIComponent(payload.sessionId)}`,
      error: payload.error ?? null,
      notes: payload.note ?? null,
      metrics,
      updated_at: now,
    };
    const { data, error } = await supabase.from("arena_runs").insert(insert).select("id,status").maybeSingle();
    if (error || !data?.id) {
      return NextResponse.json(
        { ok: false, error: `Failed to insert run progress: ${error?.message ?? "Unknown error"}` },
        { status: 500, headers: noStoreHeaders }
      );
    }
    const { error: eventError } = await supabase.from("arena_run_events").insert({
      run_id: data.id,
      frame: frameCount,
      label: "progress_update",
      payload: buildProgressEventPayload(payload, status, true),
    });
    if (eventError) {
      console.warn("[arena-progress] failed to insert run event", eventError);
    }
    return NextResponse.json(
      { ok: true, created: true, agentId: agent.id, agentName: agent.name, runId: data.id, status: data.status },
      { headers: noStoreHeaders }
    );
  }

  const updates: TablesUpdate<"arena_runs"> = {
    status,
    queue: payload.queue ?? existingRun.queue,
    started_at: existingRun.started_at ?? (status === "queued" ? null : now),
    finished_at:
      status === "completed" || status === "failed" || status === "cancelled"
        ? (existingRun.finished_at ?? now)
        : null,
    frame_count: frameCount,
    badge_count: badgeCount,
    pokedex_seen: pokedexSeen,
    pokedex_caught: pokedexCaught,
    error: payload.error ?? existingRun.error,
    notes: payload.note ?? existingRun.notes,
    metrics,
    updated_at: now,
  };

  const { error: updateError } = await supabase.from("arena_runs").update(updates).eq("id", existingRun.id);
  if (updateError) {
    return NextResponse.json(
      { ok: false, error: `Failed to update run progress: ${updateError.message}` },
      { status: 500, headers: noStoreHeaders }
    );
  }
  const { error: eventError } = await supabase.from("arena_run_events").insert({
    run_id: existingRun.id,
    frame: frameCount,
    label: "progress_update",
    payload: buildProgressEventPayload(payload, status, false),
  });
  if (eventError) {
    console.warn("[arena-progress] failed to insert run event", eventError);
  }
  return NextResponse.json(
    { ok: true, created: false, agentId: agent.id, agentName: agent.name, runId: existingRun.id, status },
    { headers: noStoreHeaders }
  );
}

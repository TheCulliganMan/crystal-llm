import "server-only";

import { randomUUID } from "crypto";
import type { RealtimeChannel, SupabaseClient } from "@supabase/supabase-js";
import { createSupabaseServiceRoleClient } from "@/lib/supabase/server";
import type { Database, Json, Tables, TablesInsert, TablesUpdate } from "@/lib/supabase/types";
import { extractSessionIdFromMetrics, extractSessionIdFromUrl } from "@/arena/utils";
import { SESSION_ID_REGEX } from "@/app/mcp/session-guards";

type RunRow = Tables<"arena_runs">;

type SnapshotReport = {
  sessionId?: string | null;
  text: string;
  payload?: Json | null;
  frame?: number | null;
  action?: string | null;
};

type ArenaEventReport = {
  sessionId?: string | null;
  label: string;
  payload?: Json | null;
  frame?: number | null;
  action?: string | null;
  text?: string | null;
};

type MetricsShape = Record<string, Json>;

const ACTIVE_STATUSES: RunRow["status"][] = ["queued", "running"];
const channelCache = new Map<string, RealtimeChannel>();
const MCP_SYSTEM_USER_ID = (process.env.POKECRYSTAL_MCP_SYSTEM_USER_ID ?? "").trim();
const MCP_SYSTEM_EMAIL = (process.env.POKECRYSTAL_MCP_SYSTEM_EMAIL ?? "mcp-session@pokecrystal.local").trim();
const MCP_SYSTEM_PASSWORD = (process.env.POKECRYSTAL_MCP_SYSTEM_PASSWORD ?? "").trim();
const MCP_AGENT_ID = (process.env.POKECRYSTAL_MCP_AGENT_ID ?? "").trim();
const MCP_AGENT_NAME = (process.env.POKECRYSTAL_MCP_AGENT_NAME ?? "MCP Session").trim();
const MCP_AGENT_SLUG = (process.env.POKECRYSTAL_MCP_AGENT_SLUG ?? "mcp-session").trim();
const MCP_AGENT_DESCRIPTION = (process.env.POKECRYSTAL_MCP_AGENT_DESCRIPTION ?? "Autotracked MCP session").trim();
const MCP_QUEUE = (process.env.POKECRYSTAL_MCP_QUEUE ?? "mcp").trim();
const MCP_SESSION_URL_BASE =
  process.env.NEXT_PUBLIC_MCP_ENTRYPOINT ??
  process.env.POKECRYSTAL_MCP_URL ??
  "/api/mcp";
const ACTIVE_RUN_SCAN_LIMIT = 200;
const SESSION_ENSURE_COOLDOWN_MS = 60_000;
const sessionEnsureCache = new Map<string, number>();
const sessionEnsureInflight = new Map<string, Promise<RunRow | null>>();
let cachedSystemUserId: string | null = MCP_SYSTEM_USER_ID || null;
let cachedAgentId: string | null = MCP_AGENT_ID || null;
let arenaTelemetryDisabled = false;
let arenaTelemetryDisableReason: string | null = null;

type AdminUser = {
  id: string;
  email?: string | null;
};

type AuthAdminApi = {
  listUsers: (params?: { page?: number; perPage?: number }) => Promise<{
    data?: { users?: AdminUser[] } | null;
    error?: { message?: string } | null;
  }>;
  createUser: (params: {
    email: string;
    password: string;
    email_confirm?: boolean;
    user_metadata?: Record<string, unknown>;
    app_metadata?: Record<string, unknown>;
  }) => Promise<{
    data?: { user?: AdminUser } | null;
    error?: { message?: string } | null;
  }>;
};

const getErrorCode = (error: unknown): string | null => {
  if (!error || typeof error !== "object") {
    return null;
  }
  const code = (error as { code?: unknown }).code;
  return typeof code === "string" && code.trim() ? code : null;
};

const isMissingArenaSchemaError = (error: unknown): boolean => getErrorCode(error) === "PGRST205";
const isDuplicateKeyError = (error: unknown): boolean => getErrorCode(error) === "23505";

const disableArenaTelemetry = (reason: string, error?: unknown): void => {
  if (arenaTelemetryDisabled) {
    return;
  }
  arenaTelemetryDisabled = true;
  arenaTelemetryDisableReason = reason;
  if (error) {
    console.debug("[arena-telemetry] disabled", reason, error);
    return;
  }
  console.debug("[arena-telemetry] disabled", reason);
};

const logTelemetryWarning = (context: string, error: unknown): void => {
  if (isMissingArenaSchemaError(error)) {
    disableArenaTelemetry(`${context}: missing Supabase relation (PGRST205)`, error);
    return;
  }
  console.warn(`[arena-telemetry] ${context}`, error);
};

const getAuthAdmin = (client: SupabaseClient<Database>): AuthAdminApi | null => {
  const admin = (client as { auth?: { admin?: AuthAdminApi } }).auth?.admin;
  return admin ?? null;
};

const findAdminUserByEmail = async (
  admin: AuthAdminApi,
  email: string
): Promise<AdminUser | null> => {
  const normalizedEmail = email.toLowerCase();
  let page = 1;
  while (page <= 10) {
    const { data, error } = await admin.listUsers({ page, perPage: 200 });
    if (error) {
      logTelemetryWarning("failed to list users", error);
      return null;
    }
    const users = data?.users ?? [];
    const existing = users.find(
      (user) => (user.email ?? "").toLowerCase() === normalizedEmail
    );
    if (existing) {
      return existing;
    }
    if (users.length < 200) {
      return null;
    }
    page += 1;
  }
  return null;
};

const resolveSystemUserId = async (
  client: SupabaseClient<Database>
): Promise<string | null> => {
  if (cachedSystemUserId) {
    return cachedSystemUserId;
  }
  if (!MCP_SYSTEM_EMAIL) {
    return null;
  }
  const admin = getAuthAdmin(client);
  if (!admin) {
    return null;
  }
  const normalizedEmail = MCP_SYSTEM_EMAIL.toLowerCase();
  const existing = await findAdminUserByEmail(admin, normalizedEmail);
  if (existing?.id) {
    cachedSystemUserId = existing.id;
    return cachedSystemUserId;
  }
  const password = MCP_SYSTEM_PASSWORD || randomUUID();
  const { data: created, error: createError } = await admin.createUser({
    email: normalizedEmail,
    password,
    email_confirm: true,
    user_metadata: { role: "mcp-system" },
    app_metadata: { role: "mcp-system" },
  });
  if (createError) {
    const recovered = await findAdminUserByEmail(admin, normalizedEmail);
    if (recovered?.id) {
      cachedSystemUserId = recovered.id;
      return cachedSystemUserId;
    }
    return null;
  }
  cachedSystemUserId = created?.user?.id ?? null;
  return cachedSystemUserId;
};

const resolveMcpAgentId = async (
  client: SupabaseClient<Database>,
  ownerId: string
): Promise<string | null> => {
  if (cachedAgentId) {
    return cachedAgentId;
  }
  if (!ownerId) {
    return null;
  }
  const slug = MCP_AGENT_SLUG || "mcp-session";
  const name = MCP_AGENT_NAME || "MCP Session";
  const { data: existing, error: existingError } = await client
    .from("arena_agents")
    .select("id")
    .eq("owner_id", ownerId)
    .eq("slug", slug)
    .maybeSingle();
  if (!existingError && existing?.id) {
    cachedAgentId = existing.id;
    return cachedAgentId;
  }
  const now = new Date().toISOString();
  const insert: TablesInsert<"arena_agents"> = {
    owner_id: ownerId,
    name,
    slug,
    description: MCP_AGENT_DESCRIPTION || null,
    repo_url: null,
    mcp_endpoint: null,
    runtime: "mcp-http",
    visibility: "public",
    config: { mcp_session_agent: true },
    updated_at: now,
  };
  const { data, error } = await client
    .from("arena_agents")
    .upsert(insert, { onConflict: "owner_id,name" })
    .select("id")
    .maybeSingle();
  if (error) {
    logTelemetryWarning("failed to upsert MCP agent", error);
    return null;
  }
  cachedAgentId = data?.id ?? null;
  return cachedAgentId;
};

const buildSessionUrl = (sessionId: string): string => {
  const base = (MCP_SESSION_URL_BASE || "/api/mcp").trim() || "/api/mcp";
  const connector = base.includes("?") ? "&" : "?";
  return `${base}${connector}session_id=${encodeURIComponent(sessionId)}`;
};

const normalizeMetrics = (value: Json | null | undefined): MetricsShape => {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  return { ...(value as MetricsShape) };
};

const matchesSession = (run: RunRow, sessionId: string): boolean => {
  const metricsSessionId = extractSessionIdFromMetrics(run.metrics);
  if (metricsSessionId && metricsSessionId === sessionId) {
    return true;
  }
  const urlSessionId = extractSessionIdFromUrl(run.mcp_session_url);
  return urlSessionId === sessionId;
};

const findRunForSession = async (
  client: SupabaseClient<Database>,
  sessionId: string
): Promise<RunRow | null> => {
  const { data, error } = await client
    .from("arena_runs")
    .select("*")
    .in("status", ACTIVE_STATUSES)
    .order("created_at", { ascending: true })
    .limit(ACTIVE_RUN_SCAN_LIMIT);
  if (error) {
    logTelemetryWarning("failed to list active runs", error);
    return null;
  }
  const matches = (data ?? []).filter((run) => matchesSession(run as RunRow, sessionId)) as RunRow[];
  if (!matches.length) {
    return null;
  }
  const running = matches.find((run) => run.status === "running");
  return running ?? matches[0];
};

const findOrEnsureRunForSession = async (
  client: SupabaseClient<Database>,
  sessionId: string
): Promise<RunRow | null> => {
  const existing = await findRunForSession(client, sessionId);
  if (existing) {
    return existing;
  }
  return ensureArenaRunForSession(sessionId);
};

const buildEventPayload = (
  sessionId: string,
  report: Pick<ArenaEventReport, "payload" | "action" | "frame" | "text">
): Json => ({
  text: report.text ?? null,
  payload: report.payload ?? null,
  action: report.action ?? null,
  frame: report.frame ?? null,
  session_id: sessionId,
});

const normalizeEventLabel = (label: string): string => {
  const trimmed = label.trim().toLowerCase();
  if (!trimmed) {
    return "event";
  }
  return trimmed.length > 80 ? trimmed.slice(0, 80) : trimmed;
};

const insertRunEvent = async (
  client: SupabaseClient<Database>,
  runId: string,
  label: string,
  frame: number | null,
  payload: Json
): Promise<void> => {
  const { error } = await client.from("arena_run_events").insert({
    run_id: runId,
    frame,
    label,
    payload,
  });
  if (error) {
    logTelemetryWarning("failed to insert run event", error);
  }
};

export const ensureArenaRunForSession = async (
  sessionId: string,
  options: { sessionUrl?: string | null } = {}
): Promise<RunRow | null> => {
  if (arenaTelemetryDisabled) {
    return null;
  }
  const normalizedSessionId = sessionId.trim();
  if (!normalizedSessionId || !SESSION_ID_REGEX.test(normalizedSessionId)) {
    return null;
  }
  const now = Date.now();
  const lastAttempt = sessionEnsureCache.get(normalizedSessionId);
  if (lastAttempt && now - lastAttempt < SESSION_ENSURE_COOLDOWN_MS) {
    return null;
  }
  const inflight = sessionEnsureInflight.get(normalizedSessionId);
  if (inflight) {
    return inflight;
  }
  const task = (async () => {
    const supabase = createSupabaseServiceRoleClient();
    if (!supabase) {
      return null;
    }
    const existing = await findRunForSession(supabase, normalizedSessionId);
    if (existing) {
      const updates: TablesUpdate<"arena_runs"> = {};
      if (!existing.mcp_session_url) {
        updates.mcp_session_url = options.sessionUrl ?? buildSessionUrl(normalizedSessionId);
      }
      const metrics = normalizeMetrics(existing.metrics);
      if (metrics.session_id !== normalizedSessionId) {
        metrics.session_id = normalizedSessionId;
        updates.metrics = metrics;
      }
      if (Object.keys(updates).length) {
        updates.updated_at = new Date().toISOString();
        const { error } = await supabase.from("arena_runs").update(updates).eq("id", existing.id);
        if (error) {
          logTelemetryWarning("failed to update MCP session metadata", error);
        }
      }
      return existing;
    }
    const ownerId = await resolveSystemUserId(supabase);
    if (!ownerId) {
      return null;
    }
    const agentId = await resolveMcpAgentId(supabase, ownerId);
    if (!agentId) {
      return null;
    }
    const nowIso = new Date().toISOString();
    const insert: TablesInsert<"arena_runs"> = {
      agent_id: agentId,
      created_by: ownerId,
      status: "running",
      queue: MCP_QUEUE || "mcp",
      started_at: nowIso,
      mcp_session_url: options.sessionUrl ?? buildSessionUrl(normalizedSessionId),
      metrics: { session_id: normalizedSessionId },
      updated_at: nowIso,
    };
    const { data, error } = await supabase
      .from("arena_runs")
      .insert(insert)
      .select("*")
      .maybeSingle();
    if (error) {
      if (isDuplicateKeyError(error)) {
        const recovered = await findRunForSession(supabase, normalizedSessionId);
        if (recovered) {
          return recovered;
        }
      }
      logTelemetryWarning("failed to insert MCP run", error);
      return null;
    }
    return data as RunRow;
  })();
  sessionEnsureInflight.set(normalizedSessionId, task);
  try {
    return await task;
  } finally {
    sessionEnsureInflight.delete(normalizedSessionId);
    sessionEnsureCache.set(normalizedSessionId, Date.now());
  }
};

const getBroadcastChannel = (
  client: SupabaseClient<Database>,
  channelName: string
): RealtimeChannel => {
  const existing = channelCache.get(channelName);
  if (existing && existing.state !== "closed") {
    return existing;
  }
  const channel = client.channel(channelName, {
    config: { broadcast: { ack: false } },
  });
  channel.subscribe((status) => {
    if (status === "CLOSED" || status === "CHANNEL_ERROR" || status === "TIMED_OUT") {
      channelCache.delete(channelName);
    }
  });
  channelCache.set(channelName, channel);
  return channel;
};

const broadcastSnapshot = async (
  client: SupabaseClient<Database>,
  sessionId: string,
  payload: Json
): Promise<void> => {
  const channel = getBroadcastChannel(client, `arena-session:${sessionId}`);
  try {
    await channel.send({ type: "broadcast", event: "snapshot", payload });
  } catch (error) {
    console.warn("[arena-telemetry] broadcast failed", error);
  }
};

export const reportArenaSnapshot = async (report: SnapshotReport): Promise<void> => {
  if (arenaTelemetryDisabled) {
    return;
  }
  const sessionId = report.sessionId?.trim();
  if (!sessionId || !SESSION_ID_REGEX.test(sessionId)) {
    return;
  }
  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    return;
  }
  const run = await findOrEnsureRunForSession(supabase, sessionId);
  if (!run) {
    return;
  }
  const now = new Date().toISOString();
  const metrics = normalizeMetrics(run.metrics);
  metrics.session_id = sessionId;
  metrics.last_snapshot_text = report.text;
  metrics.last_snapshot_at = now;
  if (report.action) {
    metrics.last_action = report.action;
    metrics.last_action_at = now;
  }
  const updates: TablesUpdate<"arena_runs"> = {
    status: run.status === "queued" ? "running" : run.status,
    started_at: run.started_at ?? now,
    frame_count: report.frame ?? run.frame_count,
    metrics,
    updated_at: now,
  };
  const { error: updateError } = await supabase.from("arena_runs").update(updates).eq("id", run.id);
  if (updateError) {
    logTelemetryWarning("failed to update run", updateError);
  }
  const eventPayload: Json = {
    text: report.text,
    payload: report.payload ?? null,
    action: report.action ?? null,
    frame: report.frame ?? null,
    session_id: sessionId,
  };
  await insertRunEvent(supabase, run.id, "snapshot", report.frame ?? null, eventPayload);
  await broadcastSnapshot(supabase, sessionId, {
    run_id: run.id,
    ...eventPayload,
  });
};

export const reportArenaEvent = async (report: ArenaEventReport): Promise<void> => {
  if (arenaTelemetryDisabled) {
    return;
  }
  const sessionId = report.sessionId?.trim();
  if (!sessionId || !SESSION_ID_REGEX.test(sessionId)) {
    return;
  }
  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    return;
  }
  const run = await findOrEnsureRunForSession(supabase, sessionId);
  if (!run) {
    return;
  }
  const payload = buildEventPayload(sessionId, report);
  await insertRunEvent(
    supabase,
    run.id,
    normalizeEventLabel(report.label),
    report.frame ?? null,
    payload
  );
};

export const __testing = {
  resetTelemetryState(): void {
    arenaTelemetryDisabled = false;
    arenaTelemetryDisableReason = null;
  },
  isTelemetryDisabled(): boolean {
    return arenaTelemetryDisabled;
  },
  telemetryDisableReason(): string | null {
    return arenaTelemetryDisableReason;
  },
};

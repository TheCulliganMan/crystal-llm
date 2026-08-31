import { NextResponse } from "next/server";
import { DEFAULT_ARENA_ELO } from "@/arena/elo";
import { createSupabaseServerClient, createSupabaseServiceRoleClient } from "@/lib/supabase/server";
import { isSupabaseConfigured, isSupabaseServiceRoleConfigured } from "@/lib/supabase/env";
import type { Database, Json, Tables, TablesInsert } from "@/lib/supabase/types";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const noStoreHeaders = {
  "Cache-Control": "no-store, max-age=0, must-revalidate",
};

type MatchMode = Database["public"]["Enums"]["matchmaking_mode"];
type MatchOutcome = "local" | "remote" | "draw" | "cancelled";

type CompleteMatchPayload = {
  action?: "complete";
  channelName?: string;
  peerUserId?: string;
  mode?: MatchMode;
  modpackId?: string;
  outcome?: MatchOutcome;
  metadata?: Record<string, Json>;
};

type ArenaProfile = Pick<
  Tables<"arena_profiles">,
  | "id"
  | "handle"
  | "display_name"
  | "link_battle_wins"
  | "link_battle_losses"
  | "link_battle_rating"
  | "total_trades"
>;

const jsonError = (status: number, message: string) =>
  NextResponse.json({ ok: false, error: message }, { status, headers: noStoreHeaders });

const readString = (value: unknown): string => (typeof value === "string" ? value.trim() : "");

const isMatchMode = (value: unknown): value is MatchMode =>
  value === "battle" || value === "trade" || value === "time_capsule";

const isMatchOutcome = (value: unknown): value is MatchOutcome =>
  value === "local" || value === "remote" || value === "draw" || value === "cancelled";

const readMetadata = (value: unknown): Record<string, Json> => {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  return value as Record<string, Json>;
};

const defaultHandle = (userId: string): string => `trainer-${userId.replace(/-/g, "").slice(0, 10)}`;

const ensureProfile = async (
  supabase: NonNullable<ReturnType<typeof createSupabaseServiceRoleClient>>,
  userId: string,
): Promise<ArenaProfile> => {
  const { data: existing, error: readError } = await supabase
    .from("arena_profiles")
    .select("id, handle, display_name, link_battle_wins, link_battle_losses, link_battle_rating, total_trades")
    .eq("id", userId)
    .maybeSingle();

  if (readError) {
    throw new Error(readError.message);
  }
  if (existing) {
    return existing as ArenaProfile;
  }

  const payload: TablesInsert<"arena_profiles"> = {
    id: userId,
    handle: defaultHandle(userId),
    display_name: null,
    link_battle_wins: 0,
    link_battle_losses: 0,
    link_battle_rating: DEFAULT_ARENA_ELO,
    total_trades: 0,
  };

  const { data: created, error: createError } = await supabase
    .from("arena_profiles")
    .insert(payload)
    .select("id, handle, display_name, link_battle_wins, link_battle_losses, link_battle_rating, total_trades")
    .single();

  if (createError) {
    throw new Error(createError.message);
  }
  return created as ArenaProfile;
};

export async function GET(request: Request) {
  if (!isSupabaseServiceRoleConfigured()) {
    return jsonError(503, "Supabase service role is not configured.");
  }
  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    return jsonError(503, "Supabase client is unavailable.");
  }

  const url = new URL(request.url);
  const limit = Math.min(50, Math.max(1, Number(url.searchParams.get("limit") ?? 10)));
  const { data, error } = await supabase
    .from("multiplayer_leaderboard")
    .select("*")
    .order("rank", { ascending: true })
    .limit(limit);

  if (error) {
    return jsonError(500, error.message);
  }

  return NextResponse.json({ ok: true, leaderboard: data ?? [] }, { headers: noStoreHeaders });
}

export async function POST(request: Request) {
  if (!isSupabaseConfigured() || !isSupabaseServiceRoleConfigured()) {
    return jsonError(503, "Supabase is not configured.");
  }

  const authClient = createSupabaseServerClient();
  const supabase = createSupabaseServiceRoleClient();
  if (!authClient || !supabase) {
    return jsonError(503, "Supabase client is unavailable.");
  }

  const {
    data: { user },
  } = await authClient.auth.getUser();
  if (!user) {
    return jsonError(401, "Not signed in.");
  }

  let payload: CompleteMatchPayload = {};
  try {
    payload = (await request.json()) as CompleteMatchPayload;
  } catch {
    return jsonError(400, "Invalid JSON body.");
  }

  const channelName = readString(payload.channelName);
  const peerUserId = readString(payload.peerUserId);
  const mode = payload.mode;
  const modpackId = readString(payload.modpackId) || "core-modular";
  const outcome = payload.outcome;
  if (!channelName) {
    return jsonError(400, "Missing channelName.");
  }
  if (!peerUserId || peerUserId === user.id) {
    return jsonError(400, "peerUserId must identify another player.");
  }
  if (!isMatchMode(mode)) {
    return jsonError(400, "Invalid match mode.");
  }
  if (!isMatchOutcome(outcome)) {
    return jsonError(400, "Invalid match outcome.");
  }

  try {
    await Promise.all([
      ensureProfile(supabase, user.id),
      ensureProfile(supabase, peerUserId),
    ]);
    const { data: settlement, error: settlementError } = await supabase.rpc(
      "report_multiplayer_match",
      {
        report_channel_name: channelName,
        report_user_id: user.id,
        report_peer_user_id: peerUserId,
        report_mode: mode,
        report_modpack_id: modpackId,
        report_outcome: outcome,
        report_metadata: readMetadata(payload.metadata),
      },
    );
    if (settlementError) {
      const lower = settlementError.message.toLowerCase();
      const status = lower.includes("participants") ? 403 : lower.includes("mismatch") || lower.includes("conflict") ? 409 : 500;
      return jsonError(status, settlementError.message);
    }

    return NextResponse.json({ ok: true, settlement }, { headers: noStoreHeaders });
  } catch (error) {
    return jsonError(500, error instanceof Error ? error.message : "Failed to complete match.");
  }
}

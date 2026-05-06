import { NextResponse } from "next/server";
import { applyEloRating, DEFAULT_ARENA_ELO, type EloOutcome } from "@/arena/elo";
import { createSupabaseServerClient, createSupabaseServiceRoleClient } from "@/lib/supabase/server";
import { isSupabaseConfigured, isSupabaseServiceRoleConfigured } from "@/lib/supabase/env";
import type { Database, Json, Tables, TablesInsert, TablesUpdate } from "@/lib/supabase/types";

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

const updateProfile = async (
  supabase: NonNullable<ReturnType<typeof createSupabaseServiceRoleClient>>,
  userId: string,
  patch: TablesUpdate<"arena_profiles">,
) => {
  const { error } = await supabase.from("arena_profiles").update(patch).eq("id", userId);
  if (error) {
    throw new Error(error.message);
  }
};

const resolveEloOutcome = (outcome: MatchOutcome): EloOutcome | null => {
  if (outcome === "local") return "a";
  if (outcome === "remote") return "b";
  if (outcome === "draw") return "draw";
  return null;
};

const applyRankedStats = async (
  supabase: NonNullable<ReturnType<typeof createSupabaseServiceRoleClient>>,
  localProfile: ArenaProfile,
  peerProfile: ArenaProfile,
  mode: MatchMode,
  outcome: MatchOutcome,
) => {
  if (mode === "trade") {
    if (outcome !== "cancelled") {
      await Promise.all([
        updateProfile(supabase, localProfile.id, {
          total_trades: (localProfile.total_trades ?? 0) + 1,
        }),
        updateProfile(supabase, peerProfile.id, {
          total_trades: (peerProfile.total_trades ?? 0) + 1,
        }),
      ]);
    }
    return null;
  }

  if (mode !== "battle") {
    return null;
  }

  const eloOutcome = resolveEloOutcome(outcome);
  if (!eloOutcome) {
    return null;
  }

  const elo = applyEloRating(
    localProfile.link_battle_rating ?? DEFAULT_ARENA_ELO,
    peerProfile.link_battle_rating ?? DEFAULT_ARENA_ELO,
    eloOutcome,
  );

  const localWon = outcome === "local";
  const peerWon = outcome === "remote";
  await Promise.all([
    updateProfile(supabase, localProfile.id, {
      link_battle_wins: (localProfile.link_battle_wins ?? 0) + (localWon ? 1 : 0),
      link_battle_losses: (localProfile.link_battle_losses ?? 0) + (peerWon ? 1 : 0),
      link_battle_rating: elo.nextRatingA,
    }),
    updateProfile(supabase, peerProfile.id, {
      link_battle_wins: (peerProfile.link_battle_wins ?? 0) + (peerWon ? 1 : 0),
      link_battle_losses: (peerProfile.link_battle_losses ?? 0) + (localWon ? 1 : 0),
      link_battle_rating: elo.nextRatingB,
    }),
  ]);

  return elo;
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
    const [localProfile, peerProfile] = await Promise.all([
      ensureProfile(supabase, user.id),
      ensureProfile(supabase, peerUserId),
    ]);
    const { data: existing, error: existingError } = await supabase
      .from("matches")
      .select("id, status, player1_id, player2_id, started_at")
      .eq("channel_name", channelName)
      .maybeSingle();
    if (existingError) {
      throw new Error(existingError.message);
    }

    if (existing?.status === "completed") {
      return NextResponse.json(
        { ok: true, match: existing, duplicate: true },
        { headers: noStoreHeaders },
      );
    }

    const elo = await applyRankedStats(supabase, localProfile, peerProfile, mode, outcome);
    const winner =
      outcome === "local"
        ? user.id
        : outcome === "remote"
          ? peerUserId
          : null;
    const now = new Date().toISOString();
    const result: Record<string, Json> = {
      outcome,
      winner,
      reported_by: user.id,
      peer_user_id: peerUserId,
      metadata: readMetadata(payload.metadata),
    };
    if (elo) {
      result.rating = {
        localBefore: elo.ratingA,
        localAfter: elo.nextRatingA,
        peerBefore: elo.ratingB,
        peerAfter: elo.nextRatingB,
      };
    }

    const matchPatch: TablesUpdate<"matches"> = {
      status: outcome === "cancelled" ? "cancelled" : "completed",
      result: result as Json,
      completed_at: now,
      started_at: existing?.started_at ?? now,
    };

    let match: Tables<"matches"> | null = null;
    if (existing) {
      const participantIds = [existing.player1_id, existing.player2_id];
      if (!participantIds.includes(user.id) || !participantIds.includes(peerUserId)) {
        return jsonError(403, "Only match participants can complete this match.");
      }
      const { data: updated, error: updateError } = await supabase
        .from("matches")
        .update(matchPatch)
        .eq("id", existing.id)
        .select("*")
        .single();
      if (updateError) {
        throw new Error(updateError.message);
      }
      match = updated as Tables<"matches">;
    } else {
      const insertPayload: TablesInsert<"matches"> = {
        player1_id: user.id,
        player2_id: peerUserId,
        mode,
        channel_name: channelName,
        ...matchPatch,
      };
      const { data: created, error: insertError } = await supabase
        .from("matches")
        .insert(insertPayload)
        .select("*")
        .single();
      if (insertError) {
        throw new Error(insertError.message);
      }
      match = created as Tables<"matches">;
    }

    return NextResponse.json({ ok: true, match, result }, { headers: noStoreHeaders });
  } catch (error) {
    return jsonError(500, error instanceof Error ? error.message : "Failed to complete match.");
  }
}

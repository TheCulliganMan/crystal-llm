import { NextResponse } from "next/server";
import { createSupabaseServerClient, createSupabaseServiceRoleClient } from "@/lib/supabase/server";
import { isSupabaseConfigured, isSupabaseServiceRoleConfigured } from "@/lib/supabase/env";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const CHANNELS = new Set(["local", "trade", "whisper"]);
const noStoreHeaders = { "Cache-Control": "no-store, max-age=0, must-revalidate" };
const jsonError = (status: number, error: string) =>
  NextResponse.json({ ok: false, error }, { status, headers: noStoreHeaders });
const readString = (value: unknown): string => typeof value === "string" ? value.trim() : "";

export async function POST(request: Request) {
  if (!isSupabaseConfigured() || !isSupabaseServiceRoleConfigured()) {
    return jsonError(503, "Chat moderation is unavailable.");
  }
  const authClient = createSupabaseServerClient();
  const serviceClient = createSupabaseServiceRoleClient();
  if (!authClient || !serviceClient) {
    return jsonError(503, "Chat moderation is unavailable.");
  }
  const { data: { user } } = await authClient.auth.getUser();
  if (!user) {
    return jsonError(401, "Sign in to report a message.");
  }

  let body: Record<string, unknown>;
  try {
    body = await request.json() as Record<string, unknown>;
  } catch {
    return jsonError(400, "Invalid JSON body.");
  }
  const messageId = readString(body.messageId);
  const reportedUserId = readString(body.reportedUserId);
  const playerName = readString(body.playerName);
  const channel = readString(body.channel);
  const text = readString(body.text);
  if (!messageId || !reportedUserId || reportedUserId === user.id) {
    return jsonError(400, "Invalid reported message.");
  }
  if (!playerName || playerName.length > 32 || !CHANNELS.has(channel) || !text || text.length > 240) {
    return jsonError(400, "Invalid report details.");
  }

  const database = serviceClient as any;
  const { error } = await database.from("multiplayer_chat_reports").insert({
    reporter_user_id: user.id,
    reported_user_id: reportedUserId,
    message_id: messageId,
    player_name: playerName,
    channel,
    message_text: text,
  });
  if (error) {
    if (String(error.code) === "23505") {
      return NextResponse.json({ ok: true, duplicate: true }, { headers: noStoreHeaders });
    }
    return jsonError(500, error.message ?? "Failed to report message.");
  }
  return NextResponse.json({ ok: true }, { headers: noStoreHeaders });
}

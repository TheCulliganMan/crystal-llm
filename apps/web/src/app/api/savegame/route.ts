import { NextResponse } from "next/server";
import { createSupabaseServerClient } from "@/lib/supabase/server";
import { isSupabaseConfigured } from "@/lib/supabase/env";
import type { Database, Json } from "@/lib/supabase/types";
import { normalizeSaveSnapshot } from "@pokecrystal/core/core/save";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const noStoreHeaders = {
  "Cache-Control": "no-store, max-age=0, must-revalidate",
};

const jsonError = (status: number, message: string) =>
  NextResponse.json({ ok: false, error: message }, { status, headers: noStoreHeaders });

const requireSupabase = () => {
  if (!isSupabaseConfigured()) {
    return null;
  }
  return createSupabaseServerClient();
};

export async function GET(request: Request) {
  const supabase = requireSupabase();
  if (!supabase) {
    return jsonError(503, "Supabase is not configured.");
  }

  const url = new URL(request.url);
  const slot = url.searchParams.get("slot");
  if (!slot) {
    return jsonError(400, "Missing save slot.");
  }

  const {
    data: { user },
  } = await supabase.auth.getUser();
  if (!user) {
    return jsonError(401, "Not signed in.");
  }

  const { data, error } = await supabase
    .from("game_saves")
    .select("payload, updated_at")
    .eq("user_id", user.id)
    .eq("slot", slot)
    .maybeSingle();

  if (error) {
    return jsonError(500, error.message);
  }
  if (!data?.payload) {
    return new NextResponse(null, { status: 204, headers: noStoreHeaders });
  }

  const updatedAt = data.updated_at ?? null;
  return NextResponse.json(
    {
      ok: true,
      payload: data.payload,
      updated_at: updatedAt,
      saved_at: updatedAt,
    },
    { headers: noStoreHeaders }
  );
}

export async function POST(request: Request) {
  const supabase = requireSupabase();
  if (!supabase) {
    return jsonError(503, "Supabase is not configured.");
  }

  let body: { slot?: string; payload?: Json; saved_at?: string } = {};
  try {
    body = (await request.json()) as typeof body;
  } catch (error) {
    return jsonError(400, "Invalid JSON body.");
  }

  const slot = typeof body.slot === "string" ? body.slot : null;
  if (!slot) {
    return jsonError(400, "Missing save slot.");
  }

  const payload = body.payload ?? null;
  if (!payload) {
    return jsonError(400, "Missing save payload.");
  }
  let normalizedPayload: Record<string, unknown>;
  try {
    normalizedPayload = normalizeSaveSnapshot(payload, `supabase:${slot}`);
  } catch (error) {
    return jsonError(
      400,
      error instanceof Error ? error.message : "Invalid save payload."
    );
  }

  const {
    data: { user },
  } = await supabase.auth.getUser();
  if (!user) {
    return jsonError(401, "Not signed in.");
  }

  const savePayload: Database["public"]["Tables"]["game_saves"]["Insert"] = {
    user_id: user.id,
    slot,
    payload: normalizedPayload as Json,
    updated_at: new Date().toISOString(),
  };

  const { data, error } = await supabase
    .from("game_saves")
    .upsert(savePayload, { onConflict: "user_id,slot" })
    .select("updated_at")
    .single();

  if (error) {
    return jsonError(500, error.message);
  }

  const updatedAt = data?.updated_at ?? savePayload.updated_at ?? null;
  return NextResponse.json(
    {
      ok: true,
      updated_at: updatedAt,
      saved_at: body.saved_at ?? updatedAt,
    },
    { headers: noStoreHeaders }
  );
}

export async function DELETE(request: Request) {
  const supabase = requireSupabase();
  if (!supabase) {
    return jsonError(503, "Supabase is not configured.");
  }

  const url = new URL(request.url);
  const slot = url.searchParams.get("slot");
  if (!slot) {
    return jsonError(400, "Missing save slot.");
  }

  const {
    data: { user },
  } = await supabase.auth.getUser();
  if (!user) {
    return jsonError(401, "Not signed in.");
  }

  const { error } = await supabase
    .from("game_saves")
    .delete()
    .eq("user_id", user.id)
    .eq("slot", slot);

  if (error) {
    return jsonError(500, error.message);
  }

  return NextResponse.json({ ok: true }, { headers: noStoreHeaders });
}

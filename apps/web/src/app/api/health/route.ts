import { NextResponse } from "next/server";
import {
  isSupabaseConfigured,
  isSupabaseServiceRoleConfigured,
} from "@/lib/supabase/env";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

export async function GET() {
  const publicClient = isSupabaseConfigured();
  const serviceRole = isSupabaseServiceRoleConfigured();
  const ready = publicClient && serviceRole;

  return NextResponse.json(
    {
      ok: ready,
      service: "pokecrystal-multiplayer",
      worldId: process.env.NEXT_PUBLIC_POKECRYSTAL_WORLD_ID?.trim() || "main",
      modpackId: process.env.NEXT_PUBLIC_POKECRYSTAL_MODPACK_ID?.trim() || "core-modular",
      multiplayer: { publicClient, serviceRole },
    },
    {
      status: ready ? 200 : 503,
      headers: { "Cache-Control": "no-store, max-age=0" },
    },
  );
}

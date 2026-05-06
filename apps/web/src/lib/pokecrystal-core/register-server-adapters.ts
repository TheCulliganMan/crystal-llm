import { setIdentityCloudSaveAdapter } from "@pokecrystal/core/adapters";
import type { Database, Json } from "@/lib/supabase/types";
import { createSupabaseServiceRoleClient } from "@/lib/supabase/server";

setIdentityCloudSaveAdapter({
  loadForIdentity: async (slot, playerId) => {
    const supabase = createSupabaseServiceRoleClient();
    if (!supabase) {
      return null;
    }
    const { data, error } = await supabase
      .from("game_saves")
      .select("payload")
      .eq("user_id", playerId)
      .eq("slot", slot)
      .maybeSingle();
    if (error) {
      throw new Error(`[save] Supabase identity load failed for ${playerId}:${slot}`);
    }
    return (data?.payload as Record<string, unknown> | null) ?? null;
  },
  saveForIdentity: async (slot, playerId, snapshot) => {
    const supabase = createSupabaseServiceRoleClient();
    if (!supabase) {
      return;
    }
    const payload: Database["public"]["Tables"]["game_saves"]["Insert"] = {
      user_id: playerId,
      slot,
      payload: snapshot as Json,
      updated_at: new Date().toISOString(),
    };
    const { error } = await supabase
      .from("game_saves")
      .upsert(payload, { onConflict: "user_id,slot" });
    if (error) {
      throw new Error(`[save] Supabase identity save failed for ${playerId}:${slot}`);
    }
  },
  deleteForIdentity: async (slot, playerId) => {
    const supabase = createSupabaseServiceRoleClient();
    if (!supabase) {
      return false;
    }
    const { error, count } = await supabase
      .from("game_saves")
      .delete({ count: "exact" })
      .eq("user_id", playerId)
      .eq("slot", slot);
    if (error) {
      throw new Error(`[save] Supabase identity delete failed for ${playerId}:${slot}`);
    }
    return (count ?? 0) > 0;
  },
});

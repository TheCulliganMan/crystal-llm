import {
  setBrowserCloudSaveAdapter,
  setMultiplayerClientFactory,
} from "@pokecrystal/core/adapters";
import { createSupabaseBrowserClient } from "@/lib/supabase/browser";
import { isSupabaseConfigured } from "@/lib/supabase/env";

type BrowserLocation = Pick<Location, "pathname" | "protocol">;

export const isDesktopBrowserLocation = (location: BrowserLocation): boolean =>
  location.pathname.startsWith("/desktop") || location.protocol === "zero:";

const isDesktopRoute = (): boolean =>
  typeof window !== "undefined" && isDesktopBrowserLocation(window.location);

setBrowserCloudSaveAdapter({
  isConfigured: () => isSupabaseConfigured() && !isDesktopRoute(),
  load: async (slot) => {
    const response = await fetch(`/api/savegame?slot=${encodeURIComponent(slot)}`, {
      cache: "no-store",
    });
    if (response.status === 401 || response.status === 404 || !response.ok) {
      return null;
    }
    const body = (await response.json().catch(() => null)) as {
      payload?: Record<string, unknown>;
      updated_at?: string | null;
      saved_at?: string | null;
    } | null;
    if (!body?.payload) {
      return null;
    }
    return {
      payload: body.payload,
      updated_at: body.updated_at ?? null,
      saved_at: body.saved_at ?? body.updated_at ?? null,
    };
  },
  save: async (slot, snapshot, savedAt) => {
    const response = await fetch("/api/savegame", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ slot, payload: snapshot, saved_at: savedAt }),
    });
    if (response.status === 401) {
      return false;
    }
    return response.ok;
  },
  delete: async (slot) => {
    const response = await fetch(`/api/savegame?slot=${encodeURIComponent(slot)}`, {
      method: "DELETE",
    });
    if (response.status === 401 || response.status === 404) {
      return false;
    }
    return response.ok;
  },
});

setMultiplayerClientFactory(() => createSupabaseBrowserClient() as any);

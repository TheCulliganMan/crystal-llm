import { createClient } from "@supabase/supabase-js";
import type { Database } from "./types";
import { getSupabasePublicConfig } from "./env";

let browserClient: ReturnType<typeof createClient<Database>> | null = null;

export const createSupabaseBrowserClient = () => {
  const config = getSupabasePublicConfig();
  if (!config) {
    browserClient = null;
    return null;
  }
  if (browserClient) {
    return browserClient;
  }
  browserClient = createClient<Database>(config.url, config.anonKey, {
    auth: {
      persistSession: true,
      autoRefreshToken: true,
      detectSessionInUrl: true,
    },
  });
  return browserClient;
};

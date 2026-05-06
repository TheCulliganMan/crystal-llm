import "server-only";

import { createServerClient, type CookieOptions } from "@supabase/ssr";
import { cookies } from "next/headers";
import { cache } from "react";
import type { Database } from "./types";
import { getSupabasePublicConfig, getSupabaseServiceRoleConfig } from "./env";

const resolveCookieAdapter = () => {
  if (process.env.NEXT_PHASE === "phase-production-build") {
    return {
      async getAll() {
        return [];
      },
      async setAll() {},
    };
  }
  try {
    const store = cookies();
    return {
      async getAll() {
        const cookieStore = await store;
        return cookieStore.getAll().map(({ name, value }) => ({ name, value }));
      },
      async setAll(cookieList: { name: string; value: string; options: CookieOptions }[]) {
        try {
          const cookieStore = await store;
          if (typeof (cookieStore as { set?: unknown }).set !== "function") {
            throw new Error("cookie store is read-only");
          }
          for (const { name, value, options } of cookieList) {
            (cookieStore as { set: (payload: { name: string; value: string } & CookieOptions) => void }).set({
              name,
              value,
              ...options,
            });
          }
        } catch (error) {
          // Server Components can expose read-only cookies; ignore refresh attempts.
          console.warn("[supabase] unable to set cookies in this context", error);
        }
      },
    };
  } catch (error) {
    // Build-time rendering can lack request storage; fall back to no-op cookies.
    console.warn("[supabase] cookies unavailable, falling back to empty adapter", error);
    return {
      async getAll() {
        return [];
      },
      async setAll() {},
    };
  }
};

export const createSupabaseServerClient = cache(() => {
  const config = getSupabasePublicConfig();
  if (!config) {
    return null;
  }
  return createServerClient<Database>(config.url, config.anonKey, {
    cookies: resolveCookieAdapter(),
  });
});

export const createSupabaseServiceRoleClient = cache(() => {
  const config = getSupabaseServiceRoleConfig();
  if (!config) {
    return null;
  }
  return createServerClient<Database>(config.url, config.serviceRoleKey, {
    cookies: resolveCookieAdapter(),
  });
});

import { NextResponse, type NextRequest } from "next/server";
import type { Database } from "./types";
import { getSupabasePublicConfig } from "./env";

type SupabaseCookie = {
  name: string;
  value: string;
  options: {
    domain?: string;
    expires?: Date;
    httpOnly?: boolean;
    maxAge?: number;
    path?: string;
    sameSite?: "lax" | "strict" | "none" | boolean;
    secure?: boolean;
  };
};

export const updateSupabaseSession = async (request: NextRequest) => {
  const config = getSupabasePublicConfig();
  if (!config) {
    return NextResponse.next({ request });
  }

  try {
    const { createServerClient } = await import("@supabase/ssr");
    let response = NextResponse.next({ request });
    const supabase = createServerClient<Database>(config.url, config.anonKey, {
      cookies: {
        getAll() {
          return request.cookies.getAll();
        },
        setAll(cookiesToSet: SupabaseCookie[]) {
          cookiesToSet.forEach(({ name, value }) => {
            request.cookies.set(name, value);
          });
          response = NextResponse.next({ request });
          cookiesToSet.forEach(({ name, value, options }) => {
            response.cookies.set(name, value, options);
          });
        },
      },
    });

    await supabase.auth.getUser();

    return response;
  } catch (error) {
    console.warn("[middleware] failed to refresh supabase session; continuing request", error);
    return NextResponse.next({ request });
  }
};

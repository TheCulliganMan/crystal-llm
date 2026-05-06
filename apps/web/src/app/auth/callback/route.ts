import { NextResponse, type NextRequest } from "next/server";
import { createSupabaseServerClient } from "@/lib/supabase/server";
import { isSupabaseConfigured } from "@/lib/supabase/env";
import { buildRecoveryRedirect, resolvePostAuthRedirect } from "@/lib/supabase/urls";

const redirectWithError = (request: NextRequest, message: string) => {
  const url = new URL("/auth/error", request.nextUrl.origin);
  url.searchParams.set("message", message);
  return NextResponse.redirect(url);
};

export const GET = async (request: NextRequest) => {
  if (!isSupabaseConfigured()) {
    return redirectWithError(request, "Supabase is not configured.");
  }

  const code = request.nextUrl.searchParams.get("code");
  if (!code) {
    return redirectWithError(request, "Missing auth code.");
  }

  const supabase = createSupabaseServerClient();
  if (!supabase) {
    return redirectWithError(request, "Supabase client is unavailable.");
  }

  const { error } = await supabase.auth.exchangeCodeForSession(code);
  if (error) {
    return redirectWithError(request, `Auth exchange failed: ${error.message}`);
  }

  const next = request.nextUrl.searchParams.get("next");
  const type = request.nextUrl.searchParams.get("type");

  if (type === "recovery" || type === "invite") {
    return NextResponse.redirect(buildRecoveryRedirect(request.nextUrl.origin, next));
  }

  return NextResponse.redirect(resolvePostAuthRedirect(request.nextUrl.origin, next));
};

import type { NextRequest } from "next/server";
import { updateSupabaseSession } from "@/lib/supabase/middleware";

export const proxy = (request: NextRequest) => updateSupabaseSession(request);

export const config = {
  matcher: [
    "/api/savegame/:path*",
    "/((?!api/|_next/static|_next/image|favicon.ico|robots.txt|sitemap.xml|assets/|disassembly/|downloads/|gfx/|index.html|.*\\.(?:svg|png|jpg|jpeg|gif|webp|ico|txt|xml|json|map|css|js|mjs|mp3|wav|mid|midi|zip|pdf|webmanifest|woff|woff2|ttf|otf)$).*)",
  ],
};

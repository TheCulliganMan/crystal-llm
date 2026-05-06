import type { Metadata } from "next";
import type { CSSProperties, ReactNode } from "react";
import { SpeedInsights } from "@vercel/speed-insights/next";
import { ThemeRegistry } from "@/components/providers/theme-registry";
import ThemeSync from "@/components/providers/theme-sync";
import { SupabaseProvider } from "@/components/providers/supabase-provider";
import { RouteShell } from "@/components/layout/route-shell";
import "./globals.css";

const rootFontVariables: CSSProperties & { "--font-space-grotesk": string } = {
  "--font-space-grotesk": "\"Avenir Next\", \"Segoe UI\", sans-serif",
};

export const metadata: Metadata = {
  title: "KrabbyClaw",
  description: "Pokemon Crystal playback with live MCP connection support for claws.",
  applicationName: "KrabbyClaw",
  icons: {
    icon: "/favicon.png",
    shortcut: "/favicon.png",
  },
};

const RootLayout = ({ children }: { children: ReactNode }) => {
  return (
    <html lang="en">
      <body
        style={rootFontVariables}
      >
        <ThemeRegistry>
          <SupabaseProvider>
            <ThemeSync />
            <RouteShell>{children}</RouteShell>
            <SpeedInsights />
          </SupabaseProvider>
        </ThemeRegistry>
      </body>
    </html>
  );
};

export default RootLayout;

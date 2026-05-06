import { SESSION_ID_REGEX } from "@/app/mcp/session-guards";
import type { Json } from "@/lib/supabase/types";

export const slugifyAgentName = (value: string): string =>
  value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 80) || "agent";

const SESSION_QUERY_KEYS = ["session_id", "session", "user"];

export const extractSessionIdFromUrl = (rawUrl?: string | null): string | null => {
  if (!rawUrl) {
    return null;
  }
  try {
    const parsed = new URL(rawUrl, "http://localhost");
    for (const key of SESSION_QUERY_KEYS) {
      const value = parsed.searchParams.get(key)?.trim();
      if (value && SESSION_ID_REGEX.test(value)) {
        return value;
      }
    }
  } catch {
    return null;
  }
  return null;
};

export const extractSessionIdFromMetrics = (metrics: Json | null | undefined): string | null => {
  if (!metrics || typeof metrics !== "object" || Array.isArray(metrics)) {
    return null;
  }
  const value = (metrics as Record<string, Json>).session_id;
  if (typeof value === "string" && SESSION_ID_REGEX.test(value)) {
    return value;
  }
  return null;
};

export const extractSessionIdFromRun = (run: {
  mcp_session_url?: string | null;
  metrics?: Json | null;
}): string | null => {
  return extractSessionIdFromMetrics(run.metrics) ?? extractSessionIdFromUrl(run.mcp_session_url);
};

export const formatDuration = (duration: string | null): string => {
  if (!duration) return "—";
  // PostgreSQL interval comes back as ISO-ish; leave as-is for now.
  return duration.replace("00:00:", "");
};

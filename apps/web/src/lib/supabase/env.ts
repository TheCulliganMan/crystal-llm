export type SupabasePublicConfig = {
  url: string;
  anonKey: string;
};

export type SupabaseServiceRoleConfig = {
  url: string;
  serviceRoleKey: string;
};

const resolveSupabaseUrl = (): string | null => {
  const url =
    process.env.NEXT_PUBLIC_SUPABASE_URL ??
    process.env.SUPABASE_URL ??
    null;
  if (!url || !url.trim()) {
    return null;
  }
  return url.trim();
};

export const getSupabasePublicConfig = (): SupabasePublicConfig | null => {
  const url = resolveSupabaseUrl();
  const anonKey = process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY;
  if (!url || !anonKey) {
    return null;
  }
  return { url, anonKey };
};

export const getSupabaseServiceRoleConfig = (): SupabaseServiceRoleConfig | null => {
  const url = resolveSupabaseUrl();
  const serviceRoleKey = process.env.SUPABASE_SERVICE_ROLE_KEY;
  if (!url || !serviceRoleKey) {
    return null;
  }
  return { url, serviceRoleKey };
};

export const isSupabaseConfigured = (): boolean => Boolean(getSupabasePublicConfig());
export const isSupabaseServiceRoleConfigured = (): boolean => Boolean(getSupabaseServiceRoleConfig());

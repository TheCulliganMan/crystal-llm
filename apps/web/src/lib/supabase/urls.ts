const DEFAULT_REDIRECT_PATH = "/";

const hasProtocol = (value: string) => /^[a-zA-Z][a-zA-Z\\d+.-]*:/.test(value);

export const sanitizeNextPath = (value?: string | null): string | null => {
  if (!value) {
    return null;
  }
  const trimmed = value.trim();
  if (!trimmed.startsWith("/") || trimmed.startsWith("//")) {
    return null;
  }
  if (hasProtocol(trimmed)) {
    return null;
  }
  return trimmed;
};

export const buildAuthCallbackUrl = (origin: string, nextPath?: string | null): string => {
  const url = new URL("/auth/callback", origin);
  const safeNext = sanitizeNextPath(nextPath);
  if (safeNext) {
    url.searchParams.set("next", safeNext);
  }
  return url.toString();
};

export const resolvePostAuthRedirect = (
  origin: string,
  nextPath?: string | null,
  fallback: string = DEFAULT_REDIRECT_PATH
): URL => {
  const safeNext = sanitizeNextPath(nextPath) ?? sanitizeNextPath(fallback) ?? DEFAULT_REDIRECT_PATH;
  return new URL(safeNext, origin);
};

export const buildRecoveryRedirect = (origin: string, nextPath?: string | null): URL => {
  const url = new URL("/auth/update-password", origin);
  const safeNext = sanitizeNextPath(nextPath);
  if (safeNext) {
    url.searchParams.set("next", safeNext);
  }
  return url;
};

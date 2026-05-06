import { createHash, createHmac, timingSafeEqual } from "crypto";
import { PRIMARY_MCP_SESSION_ID } from "./session-id";

export const DEFAULT_SESSION_ID = PRIMARY_MCP_SESSION_ID;
export const SESSION_ID_REGEX = /^[a-zA-Z0-9_-]{1,64}$/;

const FALLBACK_SESSION_SECRET = "pokecrystal-dev-session-secret";
const FALLBACK_IDENTITY_SECRET = "pokecrystal-dev-identity-secret";
const IDENTITY_TOKEN_PREFIX = "pcid";
const IDENTITY_VERSION = "1";
const SESSION_SECRET_HEADER_KEYS = [
  "x-session-secret",
  "x-pokecrystal-session-secret",
  "mcp-session-secret",
] as const;

const sessionOwnerById = new Map<string, string>();

export const normalizeSessionId = (sessionId?: string): string => {
  const trimmed = sessionId?.trim();
  if (!trimmed) {
    return DEFAULT_SESSION_ID;
  }
  if (!SESSION_ID_REGEX.test(trimmed)) {
    throw new Error("Invalid session id.");
  }
  return trimmed;
};

export const claimSessionOwnership = (sessionId: string, playerId: string): boolean => {
  const normalizedSessionId = normalizeSessionId(sessionId);
  const normalizedPlayerId = playerId.trim();
  if (!normalizedPlayerId) {
    throw new Error("playerId is required.");
  }
  const existingOwner = sessionOwnerById.get(normalizedSessionId);
  if (!existingOwner) {
    sessionOwnerById.set(normalizedSessionId, normalizedPlayerId);
    return true;
  }
  return existingOwner === normalizedPlayerId;
};

export const clearSessionOwnershipClaims = (): void => {
  sessionOwnerById.clear();
};

const secureEquals = (left: string, right: string): boolean => {
  const leftHash = createHash("sha256").update(left).digest();
  const rightHash = createHash("sha256").update(right).digest();
  return timingSafeEqual(leftHash, rightHash);
};

export const isRequestAuthorized = (
  request: Request,
  token?: string | null
): boolean => {
  const expected = (token ?? "").trim();
  if (!expected) {
    return true;
  }
  const header =
    request.headers.get("authorization") ??
    request.headers.get("x-pokecrystal-token") ??
    request.headers.get("x-mcp-token") ??
    "";
  if (!header) {
    return false;
  }
  const match = header.trim();
  if (/^bearer\s+/i.test(match)) {
    return secureEquals(match.slice(7).trim(), expected);
  }
  return secureEquals(match, expected);
};

const resolveSecretWithProductionGuard = (
  values: Array<string | undefined>,
  fallback: string,
  productionMessage: string
): string => {
  const first = values.map((value) => value?.trim()).find((value) => Boolean(value));
  if (first) {
    return first;
  }
  if (process.env.NODE_ENV === "production") {
    throw new Error(productionMessage);
  }
  return fallback;
};

const resolveSessionSecretKey = (): string =>
  resolveSecretWithProductionGuard(
    [
      process.env.POKECRYSTAL_SESSION_SECRET,
      process.env.POKECRYSTAL_IDENTITY_SECRET,
      process.env.SUPABASE_SERVICE_ROLE_KEY,
    ],
    FALLBACK_SESSION_SECRET,
    "Missing POKECRYSTAL_SESSION_SECRET (or equivalent) in production."
  );

const resolveIdentitySecret = (): string =>
  resolveSecretWithProductionGuard(
    [
      process.env.POKECRYSTAL_IDENTITY_SECRET,
      process.env.NEXTAUTH_SECRET,
      process.env.SUPABASE_SERVICE_ROLE_KEY,
    ],
    FALLBACK_IDENTITY_SECRET,
    "Missing POKECRYSTAL_IDENTITY_SECRET (or equivalent) in production."
  );

const identityTokenFromHeaders = (headers: Headers): string | null => {
  const raw =
    headers.get("authorization") ??
    headers.get("x-pokecrystal-token") ??
    "";
  const value = raw.trim();
  if (!value) {
    return null;
  }
  if (/^bearer\s+/i.test(value)) {
    const token = value.slice(7).trim();
    return token || null;
  }
  return value;
};

type IdentityClaims = {
  playerId: string;
};

const parseIdentityToken = (token: string): IdentityClaims | null => {
  const trimmed = token.trim();
  if (!trimmed) {
    return null;
  }
  const [prefix, version, payloadB64, signature] = trimmed.split(".");
  if (prefix !== IDENTITY_TOKEN_PREFIX || version !== IDENTITY_VERSION || !payloadB64 || !signature) {
    return null;
  }
  const expected = createHmac("sha256", resolveIdentitySecret())
    .update(`${version}.${payloadB64}`)
    .digest("base64url");
  if (!secureEquals(signature, expected)) {
    return null;
  }
  let decoded: unknown;
  try {
    decoded = JSON.parse(Buffer.from(payloadB64, "base64url").toString("utf8"));
  } catch {
    return null;
  }
  if (!decoded || typeof decoded !== "object") {
    return null;
  }
  const candidate = decoded as Record<string, unknown>;
  const playerId = typeof candidate.playerId === "string" ? candidate.playerId.trim() : "";
  if (!playerId || playerId.length > 128) {
    return null;
  }
  return { playerId };
};

const readSessionSecretFromRequest = (request: Request): string | null => {
  for (const headerKey of SESSION_SECRET_HEADER_KEYS) {
    const value = request.headers.get(headerKey);
    if (value && value.trim()) {
      return value.trim();
    }
  }
  const url = new URL(request.url);
  const queryValue = url.searchParams.get("session_secret")?.trim();
  return queryValue || null;
};

export const isSessionSecretRequired = (): boolean => {
  const raw = (process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET ?? "true").trim().toLowerCase();
  return raw !== "0" && raw !== "false" && raw !== "off";
};

export const buildSessionSecret = (sessionId: string, playerId: string): string => {
  const input = `${sessionId}:${playerId}`;
  return createHmac("sha256", resolveSessionSecretKey()).update(input).digest("base64url");
};

export type SessionSecretCheck = {
  ok: boolean;
  status: number;
  message?: string;
  playerId?: string;
};

export const verifySessionSecret = (
  request: Request,
  sessionId?: string | null
): SessionSecretCheck => {
  if (!isSessionSecretRequired()) {
    return { ok: true, status: 200 };
  }

  const staticToken = (process.env.POKECRYSTAL_MCP_TOKEN ?? "").trim();
  if (staticToken && isRequestAuthorized(request, staticToken)) {
    return { ok: true, status: 200 };
  }

  const normalizedSessionId = sessionId?.trim();
  if (!normalizedSessionId) {
    return {
      ok: false,
      status: 400,
      message: "Session id is required when session-secret auth is enabled.",
    };
  }
  if (!SESSION_ID_REGEX.test(normalizedSessionId)) {
    return {
      ok: false,
      status: 400,
      message: "Invalid session id.",
    };
  }

  const identityToken = identityTokenFromHeaders(request.headers);
  if (!identityToken) {
    return {
      ok: false,
      status: 401,
      message: "Missing identity token.",
    };
  }
  const claims = parseIdentityToken(identityToken);
  if (!claims) {
    return {
      ok: false,
      status: 401,
      message: "Invalid identity token.",
    };
  }
  const provided = readSessionSecretFromRequest(request);
  if (!provided) {
    return {
      ok: false,
      status: 401,
      message: "Missing session secret.",
      playerId: claims.playerId,
    };
  }
  const expected = buildSessionSecret(normalizedSessionId, claims.playerId);
  if (!secureEquals(provided, expected)) {
    return {
      ok: false,
      status: 401,
      message: "Invalid session secret.",
      playerId: claims.playerId,
    };
  }
  if (!claimSessionOwnership(normalizedSessionId, claims.playerId)) {
    return {
      ok: false,
      status: 403,
      message: "Session is owned by a different identity.",
      playerId: claims.playerId,
    };
  }
  return { ok: true, status: 200, playerId: claims.playerId };
};

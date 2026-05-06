import { createHash, createHmac, randomUUID, timingSafeEqual } from "crypto";
import * as z from "zod";
import { createSupabaseServiceRoleClient } from "@/lib/supabase/server";
import type { McpToolExtra, McpToolResponse } from "./common";

type IdentityClaims = {
  v: 1;
  playerId: string;
  name: string | null;
  iat: number;
};

type SaveSlotSummary = {
  slot: string;
  updatedAt: string;
};

type SupabaseLikeError = {
  message?: string | null;
  code?: string | null;
};

const IDENTITY_TOKEN_PREFIX = "pcid";
const IDENTITY_VERSION = 1;
const FALLBACK_IDENTITY_SECRET = "pokecrystal-dev-identity-secret";
const MCP_PLAYER_EMAIL_DOMAIN = (process.env.POKECRYSTAL_MCP_IDENTITY_EMAIL_DOMAIN ?? "mcp.pokecrystal.local").trim();
const MAX_NAME_LENGTH = 24;

const toBase64Url = (value: string): string =>
  Buffer.from(value, "utf8").toString("base64url");

const fromBase64Url = (value: string): string =>
  Buffer.from(value, "base64url").toString("utf8");

const resolveIdentitySecret = (): string => {
  return (
    process.env.POKECRYSTAL_IDENTITY_SECRET?.trim() ||
    process.env.NEXTAUTH_SECRET?.trim() ||
    process.env.SUPABASE_SERVICE_ROLE_KEY?.trim() ||
    FALLBACK_IDENTITY_SECRET
  );
};

const normalizeIdentityName = (name?: string | null): string | null => {
  const trimmed = typeof name === "string" ? name.trim() : "";
  if (!trimmed) {
    return null;
  }
  if (trimmed.length > MAX_NAME_LENGTH) {
    throw new Error(`Identity name must be ${MAX_NAME_LENGTH} characters or fewer.`);
  }
  return trimmed;
};

const secureEquals = (left: string, right: string): boolean => {
  const leftHash = createHash("sha256").update(left).digest();
  const rightHash = createHash("sha256").update(right).digest();
  return timingSafeEqual(leftHash, rightHash);
};

const signPayload = (payloadB64: string): string => {
  const body = `${IDENTITY_VERSION}.${payloadB64}`;
  return createHmac("sha256", resolveIdentitySecret()).update(body).digest("base64url");
};

const buildIdentityToken = (claims: IdentityClaims): string => {
  const payload = toBase64Url(JSON.stringify(claims));
  const signature = signPayload(payload);
  return `${IDENTITY_TOKEN_PREFIX}.${IDENTITY_VERSION}.${payload}.${signature}`;
};

export const parseIdentityToken = (token: string): IdentityClaims | null => {
  const trimmed = token.trim();
  if (!trimmed) {
    return null;
  }
  const [prefix, version, payloadB64, signature] = trimmed.split(".");
  if (prefix !== IDENTITY_TOKEN_PREFIX || version !== String(IDENTITY_VERSION) || !payloadB64 || !signature) {
    return null;
  }
  const expected = signPayload(payloadB64);
  if (!secureEquals(signature, expected)) {
    return null;
  }
  let decoded: unknown;
  try {
    decoded = JSON.parse(fromBase64Url(payloadB64));
  } catch {
    return null;
  }
  const parsed = z
    .object({
      v: z.literal(1),
      playerId: z.string().min(1).max(128),
      name: z.string().max(MAX_NAME_LENGTH).nullable(),
      iat: z.number().int().nonnegative(),
    })
    .safeParse(decoded);
  if (!parsed.success) {
    return null;
  }
  return parsed.data;
};

export const hasValidIdentityToken = (
  headers?: Headers | Record<string, string | string[] | undefined>
): boolean => {
  const token = identityTokenFromHeaders(headers);
  if (!token) {
    return false;
  }
  return Boolean(parseIdentityToken(token));
};

export const identityTokenFromHeaders = (
  headers?: Headers | Record<string, string | string[] | undefined>
): string | null => {
  const raw =
    readHeaderValue(headers, "authorization") ??
    readHeaderValue(headers, "x-pokecrystal-token") ??
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

export const resolveIdentityFromExtra = (
  extra?: McpToolExtra
): { playerId: string; name: string | null; token: string } | null => {
  const token = identityTokenFromHeaders(extra?.requestInfo?.headers);
  if (!token) {
    return null;
  }
  const claims = parseIdentityToken(token);
  if (!claims) {
    return null;
  }
  return {
    playerId: claims.playerId,
    name: claims.name,
    token,
  };
};

const ensureMcpIdentityUser = async (
  playerId: string,
  name: string | null
): Promise<void> => {
  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    return;
  }
  const admin = (supabase as { auth?: { admin?: unknown } }).auth?.admin as
    | {
        createUser: (input: {
          email: string;
          password: string;
          email_confirm: boolean;
          user_metadata?: Record<string, unknown>;
          app_metadata?: Record<string, unknown>;
        }) => Promise<{ error?: { message?: string } | null }>;
      }
    | undefined;
  if (!admin?.createUser) {
    return;
  }
  const email = `mcp-${playerId}@${MCP_PLAYER_EMAIL_DOMAIN}`;
  const response = await admin.createUser({
    email,
    password: randomUUID(),
    email_confirm: true,
    user_metadata: {
      role: "mcp-player",
      name,
      mcp_player_id: playerId,
    },
    app_metadata: {
      role: "mcp-player",
      mcp_player_id: playerId,
    },
  });
  if (response.error && !response.error.message?.toLowerCase().includes("already")) {
    throw new Error(response.error.message || "Failed to create MCP identity user.");
  }
};

const listSaveSlotsForIdentity = async (playerId: string): Promise<SaveSlotSummary[]> => {
  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    return [];
  }
  const { data, error } = await supabase
    .from("game_saves")
    .select("slot,updated_at")
    .eq("user_id", playerId)
    .order("updated_at", { ascending: false });
  if (error) {
    if (isMissingGameSavesTableError(error)) {
      return [];
    }
    throw new Error(error.message);
  }
  return (data ?? []).map((row) => ({
    slot: row.slot,
    updatedAt: row.updated_at,
  }));
};

const isMissingGameSavesTableError = (error: SupabaseLikeError): boolean => {
  const code = String(error.code ?? "").toUpperCase();
  const message = String(error.message ?? "").toLowerCase();
  if (code === "PGRST205" || code === "42P01") {
    return true;
  }
  return (
    message.includes("could not find the table 'public.game_saves'") ||
    message.includes('relation "game_saves" does not exist')
  );
};

export const RegisterIdentitySchema = z.object({
  name: z.string().max(MAX_NAME_LENGTH).optional(),
});

export const registerIdentityHandler = async (
  input: z.infer<typeof RegisterIdentitySchema>
): Promise<McpToolResponse> => {
  const playerId = randomUUID();
  const name = normalizeIdentityName(input.name ?? null);
  await ensureMcpIdentityUser(playerId, name);
  const token = buildIdentityToken({
    v: 1,
    playerId,
    name,
    iat: Date.now(),
  });

  return {
    content: [
      {
        type: "text",
        text: JSON.stringify({ playerId, token }, null, 2),
      },
    ],
  };
};

export const WhoAmISchema = z.object({});

export const whoAmIHandler = async (
  _input: z.infer<typeof WhoAmISchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  const identity = resolveIdentityFromExtra(extra);
  if (!identity) {
    return {
      isError: true,
      content: [
        {
          type: "text",
          text: JSON.stringify(
            {
              error: "No valid identity token found. Call register_identity first and send Authorization: Bearer <token> or x-pokecrystal-token.",
            },
            null,
            2
          ),
        },
      ],
    };
  }
  const slots = await listSaveSlotsForIdentity(identity.playerId);
  return {
    content: [
      {
        type: "text",
        text: JSON.stringify(
          {
            playerId: identity.playerId,
            name: identity.name,
            saveSlots: {
              count: slots.length,
              slots,
            },
          },
          null,
          2
        ),
      },
    ],
  };
};
const readHeaderValue = (
  headers: Headers | Record<string, string | string[] | undefined> | undefined,
  name: string
): string | undefined => {
  if (!headers) {
    return undefined;
  }
  if (headers instanceof Headers) {
    return headers.get(name) ?? headers.get(name.toLowerCase()) ?? undefined;
  }
  const direct = headers[name] ?? (headers as Record<string, string | string[] | undefined>)[name.toLowerCase()];
  if (Array.isArray(direct)) {
    return direct[0];
  }
  return direct as string | undefined;
};

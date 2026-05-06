import { NextResponse } from "next/server";
import {
  isRequestAuthorized,
  SESSION_ID_REGEX,
  verifySessionSecret,
} from "@/app/mcp/session-guards";
import { hasValidIdentityToken } from "@/app/api/[transport]/tools/identity";
import { getMcpToolDefinition } from "@/app/api/[transport]/tools/registry";
import { runToolWithTelemetry } from "@/app/api/[transport]/tools/common";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const noStoreHeaders = {
  "Cache-Control": "no-store, max-age=0, must-revalidate",
};
const SESSION_QUERY_KEYS = ["session_id", "session"];
const SESSION_HEADER_KEYS = [
  "mcp-session-id",
  "x-mcp-session",
  "x-pokecrystal-session",
  "x-session-id",
];

type ApiToolRequest = {
  name?: unknown;
  tool?: unknown;
  arguments?: unknown;
  input?: unknown;
  session_id?: unknown;
  session?: unknown;
  method?: unknown;
  params?: {
    name?: unknown;
    tool?: unknown;
    arguments?: unknown;
    input?: unknown;
    session_id?: unknown;
    session?: unknown;
  } | null;
};

const REGISTER_IDENTITY_TOOL = "register_identity";

const resolveSessionFromHeaders = (request: Request): string | undefined => {
  for (const key of SESSION_HEADER_KEYS) {
    const value = request.headers.get(key);
    if (value && value.trim()) {
      return value.trim();
    }
  }
  return undefined;
};

const resolveSessionFromQuery = (request: Request): string | undefined => {
  const url = new URL(request.url);
  for (const key of SESSION_QUERY_KEYS) {
    const value = url.searchParams.get(key);
    if (value && value.trim()) {
      return value.trim();
    }
  }
  return undefined;
};

const resolveSessionFromBody = (payload: ApiToolRequest): string | undefined => {
  const sessionIdValue =
    typeof payload.session_id === "string" && payload.session_id.trim()
      ? payload.session_id
      : typeof payload.session === "string" && payload.session.trim()
        ? payload.session
        : typeof payload.params?.session_id === "string" && payload.params.session_id.trim()
          ? payload.params.session_id
          : typeof payload.params?.session === "string" && payload.params.session.trim()
            ? payload.params.session
        : undefined;
  return sessionIdValue?.trim();
};

const buildHeaderRecord = (
  request: Request,
  sessionId?: string
): Record<string, string> => {
  const headers: Record<string, string> = {};
  for (const [key, value] of request.headers.entries()) {
    headers[key] = value;
  }
  if (sessionId) {
    headers["mcp-session-id"] = sessionId;
    headers["x-mcp-session"] = sessionId;
  }
  return headers;
};

const isAuthorized = (request: Request): boolean => {
  const token = process.env.POKECRYSTAL_MCP_TOKEN ?? "";
  const tokenProtected = token.trim().length > 0;
  if (tokenProtected) {
    const staticAuthorized = isRequestAuthorized(request, token);
    const identityAuthorized = hasValidIdentityToken(request.headers);
    return staticAuthorized || identityAuthorized;
  }
  return isRequestAuthorized(request, token);
};

export async function POST(request: Request) {
  let payload: ApiToolRequest;
  try {
    payload = (await request.json()) as ApiToolRequest;
  } catch {
    return NextResponse.json(
      { ok: false, error: "Invalid JSON body." },
      { status: 400, headers: noStoreHeaders }
    );
  }

  const toolName =
    (typeof payload.tool === "string" && payload.tool.trim()) ||
    (typeof payload.name === "string" && payload.name.trim()) ||
    (typeof payload.params?.tool === "string" && payload.params.tool.trim()) ||
    (typeof payload.params?.name === "string" && payload.params.name.trim()) ||
    "";
  if (!toolName) {
    return NextResponse.json(
      { ok: false, error: "Missing tool name. Provide 'tool' or 'name'." },
      { status: 400, headers: noStoreHeaders }
    );
  }
  const isIdentityBootstrap = toolName === REGISTER_IDENTITY_TOOL;
  if (!isAuthorized(request)) {
    return NextResponse.json({ ok: false, error: "Unauthorized" }, { status: 401, headers: noStoreHeaders });
  }

  const definition = getMcpToolDefinition(toolName);
  if (!definition) {
    return NextResponse.json(
      { ok: false, error: `Unknown tool: ${toolName}` },
      { status: 404, headers: noStoreHeaders }
    );
  }

  const sessionId =
    resolveSessionFromBody(payload) ??
    resolveSessionFromQuery(request) ??
    resolveSessionFromHeaders(request);
  if (sessionId && !SESSION_ID_REGEX.test(sessionId)) {
    return NextResponse.json(
      { ok: false, error: "Invalid session id." },
      { status: 400, headers: noStoreHeaders }
    );
  }
  if (!isIdentityBootstrap) {
    const secretCheck = verifySessionSecret(request, sessionId);
    if (!secretCheck.ok) {
      return NextResponse.json(
        { ok: false, error: secretCheck.message ?? "Unauthorized" },
        { status: secretCheck.status, headers: noStoreHeaders }
      );
    }
  }
  const rawInput =
    payload.input !== undefined
      ? payload.input
      : payload.arguments !== undefined
        ? payload.arguments
        : payload.params?.input !== undefined
          ? payload.params.input
          : payload.params?.arguments !== undefined
            ? payload.params.arguments
        : {};
  const parsed = definition.inputSchema.safeParse(rawInput);
  if (!parsed.success) {
    return NextResponse.json(
      {
        ok: false,
        error: "Invalid tool arguments.",
        issues: parsed.error.issues,
      },
      { status: 400, headers: noStoreHeaders }
    );
  }

  try {
    const result = await runToolWithTelemetry(toolName, parsed.data, definition.handler, {
      requestInfo: {
        headers: buildHeaderRecord(request, sessionId),
      },
      rawInput,
    });
    return NextResponse.json(
      {
        ok: !result.isError,
        tool: toolName,
        result,
      },
      { headers: noStoreHeaders }
    );
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    const status = message === "Invalid session id." ? 400 : 500;
    return NextResponse.json(
      {
        ok: false,
        error: message,
      },
      { status, headers: noStoreHeaders }
    );
  }
}

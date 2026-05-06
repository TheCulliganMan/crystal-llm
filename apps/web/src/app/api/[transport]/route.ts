import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { WebStandardStreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/webStandardStreamableHttp.js";
import { ensureArenaRunForSession } from "@/arena/runtime/telemetry";
import {
  isRequestAuthorized,
  SESSION_ID_REGEX,
  verifySessionSecret,
} from "@/app/mcp/session-guards";
import { hasValidIdentityToken } from "./tools/identity";
import { registerTools } from "./tools";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const SESSION_QUERY_KEYS = ["session_id", "session"];
const USER_QUERY_KEYS = ["user"];
const SESSION_HEADER_KEYS = [
  "mcp-session-id",
  "x-mcp-session",
  "x-pokecrystal-session",
  "x-session-id",
];
const MCP_ACCEPT_HEADER = "application/json, text/event-stream";
const MCP_SERVER_INFO = { name: "pokecrystal-mcp", version: "1.0.0" };
const REGISTER_IDENTITY_TOOL = "register_identity";
const parseEncodedQuery = (
  key: string,
  value: string | null
): { sessionId?: string; source?: "session" | "user" } | undefined => {
  if (value && value.trim()) {
    return undefined;
  }
  const equalsIndex = key.indexOf("=");
  if (equalsIndex <= 0) {
    return undefined;
  }
  const rawKey = key.slice(0, equalsIndex).trim();
  const rawValue = key.slice(equalsIndex + 1).trim();
  if (!rawKey || !rawValue) {
    return undefined;
  }
  if (SESSION_QUERY_KEYS.includes(rawKey)) {
    return { sessionId: rawValue, source: "session" };
  }
  if (USER_QUERY_KEYS.includes(rawKey)) {
    return { sessionId: rawValue, source: "user" };
  }
  return undefined;
};

const sessionIdFromQuery = (
  request: Request
): { sessionId?: string; source?: "session" | "user" } | undefined => {
  const url = new URL(request.url);
  for (const key of SESSION_QUERY_KEYS) {
    const value = url.searchParams.get(key);
    if (value && value.trim()) {
      return { sessionId: value.trim(), source: "session" };
    }
  }
  for (const key of USER_QUERY_KEYS) {
    const value = url.searchParams.get(key);
    if (value && value.trim()) {
      return { sessionId: value.trim(), source: "user" };
    }
  }
  for (const [key, value] of url.searchParams.entries()) {
    const parsed = parseEncodedQuery(key, value);
    if (parsed?.sessionId) {
      return parsed;
    }
  }
  return undefined;
};

const sessionIdFromHeaders = (request: Request): string | undefined => {
  for (const key of SESSION_HEADER_KEYS) {
    const value = request.headers.get(key);
    if (value && value.trim()) {
      return value.trim();
    }
  }
  return undefined;
};

const ensureMcpAcceptHeader = (request: Request): Request => {
  const headers = new Headers(request.headers);
  headers.set("accept", MCP_ACCEPT_HEADER);
  return new Request(request, { headers });
};

const isRegisterIdentityBootstrapRequest = async (request: Request): Promise<boolean> => {
  if (request.method !== "POST") {
    return false;
  }
  const contentType = request.headers.get("content-type")?.toLowerCase() ?? "";
  if (!contentType.includes("application/json")) {
    return false;
  }
  let payload: unknown;
  try {
    payload = await request.clone().json();
  } catch {
    return false;
  }
  if (!payload || typeof payload !== "object") {
    return false;
  }
  const rpcPayload = payload as { method?: unknown; params?: unknown };
  if (rpcPayload.method !== "tools/call") {
    return false;
  }
  const params = rpcPayload.params;
  if (!params || typeof params !== "object") {
    return false;
  }
  const toolName = (params as { name?: unknown }).name;
  return typeof toolName === "string" && toolName.trim() === REGISTER_IDENTITY_TOOL;
};

const buildMcpServer = () => {
  const serverInstance = new McpServer(MCP_SERVER_INFO);
  registerTools(serverInstance);
  return serverInstance;
};

const handleMcpRequest = async (request: Request): Promise<Response> => {
  const server = buildMcpServer();
  const transport = new WebStandardStreamableHTTPServerTransport({
    sessionIdGenerator: undefined,
  });
  await server.connect(transport);
  return transport.handleRequest(request);
};

const primeSseResponse = (response: Response): Response => {
  const body = response.body;
  if (!body) {
    return response;
  }
  const reader = body.getReader();
  const encoder = new TextEncoder();
  const stream = new ReadableStream({
    start(controller) {
      controller.enqueue(encoder.encode(":ok\n\n"));
      const pump = (): void => {
        reader.read().then(({ done, value }) => {
          if (done) {
            controller.close();
            return;
          }
          if (value) {
            controller.enqueue(value);
          }
          pump();
        }).catch((error) => {
          controller.error(error);
        });
      };
      pump();
    },
    cancel() {
      reader.cancel().catch?.(() => undefined);
    },
  });
  const headers = new Headers(response.headers);
  return new Response(stream, {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
};

const shouldPrimeSse = (request: Request, response: Response): boolean => {
  if (request.method !== "GET") {
    return false;
  }
  const accept = request.headers.get("accept") ?? "";
  if (!accept.includes("text/event-stream")) {
    return false;
  }
  const contentType = response.headers.get("content-type") ?? "";
  if (!contentType.includes("text/event-stream")) {
    return false;
  }
  return true;
};

const ensureUtf8ContentType = (response: Response): Response => {
  const headers = new Headers(response.headers);
  const contentType = headers.get("content-type") ?? "";
  if (
    contentType &&
    !/charset=/i.test(contentType) &&
    (contentType.includes("text/") ||
      contentType.includes("application/json") ||
      contentType.includes("text/event-stream"))
  ) {
    headers.set("content-type", `${contentType}; charset=utf-8`);
  }
  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
};

const finalizeMcpResponse = (request: Request, response: Response): Response => {
  const primed = shouldPrimeSse(request, response) ? primeSseResponse(response) : response;
  return ensureUtf8ContentType(primed);
};

const withAuth = async (request: Request) => {
  const bootstrapRequest = await isRegisterIdentityBootstrapRequest(request);
  const token = process.env.POKECRYSTAL_MCP_TOKEN ?? "";
  const tokenProtected = token.trim().length > 0;
  if (tokenProtected) {
    const staticAuthorized = isRequestAuthorized(request, token);
    const identityAuthorized = hasValidIdentityToken(request.headers);
    if (!staticAuthorized && !identityAuthorized) {
      return new Response("Unauthorized", { status: 401 });
    }
  }
  if (!tokenProtected && !isRequestAuthorized(request, token)) {
    return new Response("Unauthorized", { status: 401 });
  }
  const normalizedRequest = ensureMcpAcceptHeader(request);
  const url = new URL(normalizedRequest.url);
  const sessionLookup = sessionIdFromQuery(normalizedRequest);
  const headerSessionId = sessionIdFromHeaders(normalizedRequest);
  const trackedSessionId = sessionLookup?.sessionId ?? headerSessionId;
  if (!bootstrapRequest) {
    const sessionSecretCheck = verifySessionSecret(normalizedRequest, trackedSessionId);
    if (!sessionSecretCheck.ok) {
      return new Response(sessionSecretCheck.message ?? "Unauthorized", { status: sessionSecretCheck.status });
    }
  }
  if (!sessionLookup?.sessionId) {
    if (headerSessionId && SESSION_ID_REGEX.test(headerSessionId)) {
      void ensureArenaRunForSession(headerSessionId).catch(() => undefined);
    }
    const response = await handleMcpRequest(normalizedRequest);
    return finalizeMcpResponse(normalizedRequest, response);
  }
  if (!SESSION_ID_REGEX.test(sessionLookup.sessionId)) {
    if (sessionLookup.source === "user") {
      const response = await handleMcpRequest(normalizedRequest);
      return finalizeMcpResponse(normalizedRequest, response);
    }
    return new Response("Invalid session id.", { status: 400 });
  }
  if (trackedSessionId && SESSION_ID_REGEX.test(trackedSessionId)) {
    void ensureArenaRunForSession(trackedSessionId).catch(() => undefined);
  }
  const existingHeader = normalizedRequest.headers.get("mcp-session-id");
  if (existingHeader && existingHeader.trim()) {
    const response = await handleMcpRequest(normalizedRequest);
    return finalizeMcpResponse(normalizedRequest, response);
  }
  const headers = new Headers(normalizedRequest.headers);
  headers.set("mcp-session-id", sessionLookup.sessionId);
  headers.set("x-mcp-session", sessionLookup.sessionId);
  const nextRequest = new Request(normalizedRequest, { headers });
  const response = await handleMcpRequest(nextRequest);
  return finalizeMcpResponse(normalizedRequest, response);
};

export { withAuth as GET, withAuth as POST };

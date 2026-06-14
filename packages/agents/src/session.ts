import fs from "node:fs/promises";
import path from "node:path";

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import { MCPClient } from "@mastra/mcp";
import { createTool, type Tool } from "@mastra/core/tools";
import { z } from "zod";

import {
  observationSchema,
  sessionAuthSchema,
  sessionConfigSchema,
  statusSchema,
  type Observation,
  type SessionAuth,
  type SessionConfig,
  type Status,
} from "./types.js";
import {
  emitAgentMcpCall,
  emitAgentMcpResult,
  emitAgentStreamEvent,
  shouldEmitAgentStreamEvents,
} from "./stream-events.js";

type McpTextPart = { type: "text"; text: string };
type McpImagePart = { type: "image"; data: string; mimeType?: string; mime_type?: string };
export type McpToolResult = { content?: Array<McpTextPart | McpImagePart> };
type KrabbyClawToolset = Record<string, Tool<any, any, any, any>>;
type KrabbyClawNamespacedTools = Record<string, Tool<any, any, any, any>>;
type DirectMcpClient = {
  client: Client;
  transport: StreamableHTTPClientTransport;
};

const PLAYER_TOOL_NAMES = [
  "observe",
  "map_info",
  "route_render",
  "flow_state",
  "move",
  "press",
  "type_text",
  "hold_button",
  "status",
  "recent_events",
] as const;

const PLAYER_TOOL_ALLOWLIST = new Set([
  ...PLAYER_TOOL_NAMES,
  ...PLAYER_TOOL_NAMES.map(name => `krabbyclaw_${name}`),
]);
const ACTION_TOOL_NAMES = new Set(["move", "press", "hold_button"]);

function normalizeKrabbyClawToolName(name: string): string | null {
  const normalized = name.startsWith("krabbyclaw_") ? name.slice("krabbyclaw_".length) : name;
  return PLAYER_TOOL_NAMES.includes(normalized as (typeof PLAYER_TOOL_NAMES)[number])
    ? normalized
    : null;
}

export function prepareVisibleActionToolInput(
  name: string,
  args: Record<string, unknown>,
): { args: Record<string, unknown>; reason: string | null } {
  const normalizedName = normalizeKrabbyClawToolName(name);
  if (!normalizedName || !ACTION_TOOL_NAMES.has(normalizedName)) {
    return { args, reason: null };
  }
  const reason = typeof args.reason === "string" ? args.reason.trim() : "";
  const { reason: _reason, ...mcpArgs } = args;
  return {
    args: mcpArgs,
    reason: reason || null,
  };
}

const emitVisibleActionReason = (reason: string | null): void => {
  if (reason && shouldEmitAgentStreamEvents()) {
    emitAgentStreamEvent({ type: "thinking-delta", text: reason, source: "player" });
  }
};

const visibleReasonField = z.string().min(1).describe(
  "Visible trainer rationale for this action, grounded in the latest MCP state.",
);

const withVisibleActionReasonSchema = (
  name: string,
  inputSchema: unknown,
): unknown => {
  const normalizedName = normalizeKrabbyClawToolName(name);
  if (!normalizedName || !ACTION_TOOL_NAMES.has(normalizedName) || !(inputSchema instanceof z.ZodObject)) {
    return inputSchema;
  }
  return inputSchema.extend({ reason: visibleReasonField });
};

const DIRECT_PLAYER_TOOL_ALLOWLIST = new Set(
  [...PLAYER_TOOL_ALLOWLIST].map(name => name.replace(/^krabbyclaw_/, "")),
);

const toDirectPlayerToolName = (name: string): string =>
  name.startsWith("krabbyclaw_") ? name.slice("krabbyclaw_".length) : name;

export function mapMcpToolsToPlayerTools(tools: KrabbyClawToolset): KrabbyClawNamespacedTools {
  return Object.fromEntries(
    Object.entries(tools)
      .filter(([name]) => {
        const directName = toDirectPlayerToolName(name);
        return PLAYER_TOOL_ALLOWLIST.has(name) || DIRECT_PLAYER_TOOL_ALLOWLIST.has(directName);
      })
      .map(([name, tool]) => [name, instrumentTool(name, tool)]),
  );
}

export function mapMcpToolsToDirectPlayerTools(tools: KrabbyClawToolset): KrabbyClawToolset {
  return Object.fromEntries(
    Object.entries(tools)
      .map(([name, tool]) => [toDirectPlayerToolName(name), tool] as const)
      .filter(([name]) => DIRECT_PLAYER_TOOL_ALLOWLIST.has(name)),
  );
}

function getSessionAuthCacheDir(): string {
  const override = process.env.POKECRYSTAL_AGENT_SESSION_DIR?.trim();
  if (override) {
    return path.resolve(override);
  }
  return path.resolve(process.cwd(), "packages/agents/.session-auth");
}

function getSessionAuthCachePath(sessionId: string): string {
  return path.join(getSessionAuthCacheDir(), `${sessionId}.json`);
}

function buildSessionUrl(baseUrl: string, sessionId: string, route: string): URL {
  return new URL(`${route}?session_id=${encodeURIComponent(sessionId)}`, baseUrl);
}

const summarizeToolResult = (result: unknown): string => {
  if (result && typeof result === "object" && Array.isArray((result as McpToolResult).content)) {
    return extractTextParts(result as McpToolResult).join(" ").slice(0, 240);
  }
  if (typeof result === "string") {
    return result.slice(0, 240);
  }
  try {
    return JSON.stringify(result).slice(0, 240);
  } catch {
    return String(result).slice(0, 240);
  }
};

const instrumentTool = (name: string, tool: Tool<any, any, any, any>): Tool<any, any, any, any> => {
  if (!tool.execute) {
    return tool;
  }
  const originalExecute = tool.execute.bind(tool);
  return {
    ...tool,
    inputSchema: withVisibleActionReasonSchema(name, tool.inputSchema) as typeof tool.inputSchema,
    execute: async (args: Record<string, unknown>, context: unknown) => {
      const prepared = prepareVisibleActionToolInput(name, args ?? {});
      emitVisibleActionReason(prepared.reason);
      emitAgentMcpCall(name, prepared.args);
      const result = await originalExecute(prepared.args, context as any);
      emitAgentMcpResult(name, summarizeToolResult(result));
      return result;
    },
  } as Tool<any, any, any, any>;
};

export function buildMcpUrl(baseUrl: string, sessionId: string): URL {
  return buildSessionUrl(baseUrl, sessionId, "/api/mcp");
}

export function buildToolsUrl(baseUrl: string, sessionId: string): URL {
  return buildSessionUrl(baseUrl, sessionId, "/api/mcp/tools");
}

export function deriveIdentity(config: SessionConfig): Pick<SessionAuth, "agentId" | "identityName"> {
  const agentId = config.agentId ?? config.sessionId;
  const identityName = config.identityName ?? `trainer-${config.sessionId}`;
  return { agentId, identityName };
}

export function createDirectMcpSessionAuth(configInput: SessionConfig): SessionAuth {
  const config = sessionConfigSchema.parse(configInput);
  if (!config.mcpUrl) {
    throw new Error("Direct MCP session auth requires mcpUrl.");
  }
  const { agentId, identityName } = deriveIdentity(config);
  return sessionAuthSchema.parse({
    baseUrl: config.baseUrl,
    mcpUrl: config.mcpUrl,
    sessionId: config.sessionId,
    agentId,
    identityName,
    token: "direct-mcp",
    sessionSecret: "direct-mcp",
  });
}

export function extractTextParts(result: McpToolResult): string[] {
  return (result.content ?? [])
    .filter((part): part is McpTextPart => part?.type === "text" && typeof part.text === "string")
    .map(part => part.text);
}

export function extractFirstImagePart(result: McpToolResult): { data: string; mimeType: string } | undefined {
  const image = (result.content ?? []).find((part): part is McpImagePart =>
    Boolean(part?.type === "image" && typeof part.data === "string" && part.data.length > 0),
  );
  if (!image) {
    return undefined;
  }
  const mimeType = typeof image.mimeType === "string"
    ? image.mimeType
    : typeof image.mime_type === "string"
      ? image.mime_type
      : "image/png";
  return { data: image.data, mimeType };
}

export function parseFirstJsonText<T>(result: McpToolResult, parser: (value: unknown) => T): T {
  const [firstText] = extractTextParts(result);
  if (!firstText) {
    throw new Error("Expected MCP tool result to include at least one text payload.");
  }
  return parser(JSON.parse(firstText));
}

export function parseObservation(result: McpToolResult): Observation {
  const texts = extractTextParts(result);
  const snapshotText = texts.find(text => text.trim().startsWith("{"));
  const snapshot = snapshotText ? observationSchema.shape.snapshot.parse(JSON.parse(snapshotText)) : undefined;
  const image = extractFirstImagePart(result);

  return observationSchema.parse({
    summaryText: texts[0] ?? "",
    snapshot,
    image,
    rawTexts: texts,
  });
}

async function fetchJson(url: URL | string, init?: RequestInit): Promise<unknown> {
  const response = await fetch(url, init);
  if (!response.ok) {
    throw new Error(`Request failed with ${response.status} ${response.statusText}`);
  }
  return response.json();
}

export async function readSessionAuthCache(sessionId: string): Promise<SessionAuth | null> {
  try {
    const raw = await fs.readFile(getSessionAuthCachePath(sessionId), "utf8");
    return sessionAuthSchema.parse(JSON.parse(raw));
  } catch {
    return null;
  }
}

async function writeCachedSessionAuth(auth: SessionAuth): Promise<void> {
  await fs.mkdir(getSessionAuthCacheDir(), { recursive: true });
  await fs.writeFile(getSessionAuthCachePath(auth.sessionId), JSON.stringify(auth, null, 2), "utf8");
}

async function refreshSessionSecret(auth: SessionAuth): Promise<SessionAuth> {
  const secretJson = (await fetchJson(
    new URL(`/api/arena/session-secret?session_id=${encodeURIComponent(auth.sessionId)}`, auth.baseUrl),
    {
      headers: {
        Authorization: `Bearer ${auth.token}`,
      },
    },
  )) as { sessionSecret?: string };

  if (!secretJson.sessionSecret) {
    throw new Error("Session secret response did not include sessionSecret.");
  }

  return sessionAuthSchema.parse({
    ...auth,
    sessionSecret: secretJson.sessionSecret,
  });
}

export async function bootstrapSession(configInput: SessionConfig): Promise<SessionAuth> {
  const config = sessionConfigSchema.parse(configInput);
  const { agentId, identityName } = deriveIdentity(config);
  const cachedAuth = await readSessionAuthCache(config.sessionId);

  if (
    cachedAuth &&
    cachedAuth.baseUrl === config.baseUrl &&
    cachedAuth.agentId === agentId &&
    cachedAuth.identityName === identityName
  ) {
    try {
      const refreshed = await refreshSessionSecret(cachedAuth);
      await writeCachedSessionAuth(refreshed);
      return refreshed;
    } catch {
      return cachedAuth;
    }
  }

  const toolsUrl = buildToolsUrl(config.baseUrl, config.sessionId);

  const registerResponse = (await fetchJson(toolsUrl, {
    method: "POST",
    headers: {
      accept: "application/json",
      "content-type": "application/json",
    },
    body: JSON.stringify({
      name: "register_identity",
      arguments: {
        agentId,
        identityName,
      },
    }),
  })) as {
    result?: { content?: Array<{ text?: string }> };
  };

  const registerText = registerResponse.result?.content?.[0]?.text;
  if (!registerText) {
    throw new Error("register_identity did not return an identity token.");
  }

  const token = JSON.parse(registerText).token as string | undefined;
  if (!token) {
    throw new Error("register_identity response did not include a token.");
  }

  const auth = sessionAuthSchema.parse({
    baseUrl: config.baseUrl,
    sessionId: config.sessionId,
    agentId,
    identityName,
    token,
    sessionSecret: (await refreshSessionSecret({
      baseUrl: config.baseUrl,
      sessionId: config.sessionId,
      agentId,
      identityName,
      token,
      sessionSecret: "pending-refresh",
    })).sessionSecret,
  });

  await writeCachedSessionAuth(auth);

  return auth;
}

export class KrabbyClawSession {
  private mcpClient: MCPClient | null = null;
  private directMcpClientPromise: Promise<DirectMcpClient> | null = null;
  private playerToolsPromise: Promise<KrabbyClawNamespacedTools> | null = null;
  private directToolsetPromise: Promise<KrabbyClawToolset> | null = null;

  constructor(readonly auth: SessionAuth) {}

  private ensureClient(): MCPClient {
    if (!this.mcpClient) {
      this.mcpClient = new MCPClient({
        id: `krabbyclaw:${this.auth.sessionId}`,
        servers: {
          krabbyclaw: {
            url: this.auth.mcpUrl ? new URL(this.auth.mcpUrl) : buildMcpUrl(this.auth.baseUrl, this.auth.sessionId),
            requestInit: this.auth.mcpUrl
              ? undefined
              : {
                  headers: {
                    Authorization: `Bearer ${this.auth.token}`,
                    "x-session-secret": this.auth.sessionSecret,
                  },
                },
          },
        },
      });
    }
    return this.mcpClient;
  }

  private async toolset(): Promise<KrabbyClawToolset> {
    if (!this.playerToolsPromise) {
      if (this.auth.mcpUrl) {
        this.playerToolsPromise = Promise.resolve(this.createDirectPlayerTools());
        return this.playerToolsPromise;
      }
      this.playerToolsPromise = this.ensureClient()
        .listTools()
        .then(tools => {
          const filtered = mapMcpToolsToPlayerTools(tools);
          if (Object.keys(filtered).length === 0) {
            throw new Error("KrabbyClaw MCP server did not expose player tools.");
          }
          return filtered;
        });
    }
    return this.playerToolsPromise;
  }

  async listPlayerTools(): Promise<KrabbyClawNamespacedTools> {
    return this.toolset();
  }

  private async directToolset(): Promise<KrabbyClawToolset> {
    if (!this.auth.mcpUrl) {
      throw new Error("Direct MCP toolset requested for non-direct session.");
    }
    if (!this.directToolsetPromise) {
      this.directToolsetPromise = this.ensureClient()
        .listTools()
        .then(tools => {
          const listedToolNames = Object.keys(tools);
          const directTools = mapMcpToolsToDirectPlayerTools(tools);
          if (Object.keys(directTools).length === 0) {
            throw new Error(
              `Direct KrabbyClaw MCP server did not expose tools. Listed tools: ${listedToolNames.join(", ") || "(none)"}.`,
            );
          }
          return directTools;
        });
    }
    return this.directToolsetPromise;
  }

  private async directClient(): Promise<DirectMcpClient> {
    if (!this.auth.mcpUrl) {
      throw new Error("Direct MCP client requested for non-direct session.");
    }
    if (!this.directMcpClientPromise) {
      this.directMcpClientPromise = (async () => {
        const client = new Client({ name: "pokecrystal-agent-direct", version: "1.0.0" });
        const transport = new StreamableHTTPClientTransport(new URL(this.auth.mcpUrl as string), {
          requestInit: {
            headers: {
              accept: "application/json, text/event-stream",
            },
          },
        });
        await client.connect(transport);
        return { client, transport };
      })();
    }
    return this.directMcpClientPromise;
  }

  private createDirectPlayerTools(): KrabbyClawToolset {
    const textTool = (
      id: string,
      description: string,
      inputSchema: z.ZodTypeAny,
    ): Tool<any, any, any, any> =>
      createTool({
        id,
        description,
        inputSchema,
        execute: async (input: unknown) => {
          const prepared = prepareVisibleActionToolInput(
            id,
            input && typeof input === "object" ? input as Record<string, unknown> : {},
          );
          emitVisibleActionReason(prepared.reason);
          const result = await this.executeTool(id, prepared.args);
          return extractTextParts(result).join("\n");
        },
      });

    const emptySchema = z.object({});
    const reasonSchema = z.object({ reason: visibleReasonField });
    const normalizedButtonSchema = z.preprocess(
      value => typeof value === "string" ? value.trim().toLowerCase() : value,
      z.enum(["a", "b", "start", "select", "up", "down", "left", "right"]),
    );
    const buttonSchema = z.object({
      button: normalizedButtonSchema,
    });
    const directionSchema = z.object({
      direction: z.enum(["up", "down", "left", "right"]),
      steps: z.number().int().min(1).max(10).optional(),
    });
    const typeTextSchema = z.object({
      text: z.string().min(1).max(32),
      clear: z.boolean().optional(),
      submit: z.boolean().optional(),
    });
    return {
      observe: textTool("observe", "Observe the current game state and visible screen text.", emptySchema),
      map_info: textTool("map_info", "Get local map, hotspot, and routing information.", emptySchema),
      route_render: textTool("route_render", "Get a full current-map navigation schematic when viewport evidence is insufficient.", emptySchema),
      flow_state: textTool("flow_state", "Get spoiler-safe story-flow progress and next goal.", emptySchema),
      move: textTool("move", "Send a d-pad input; in menus, name entry, and time entry this moves the cursor or adjusts the selected value.", directionSchema.merge(reasonSchema)),
      press: textTool("press", "Press a Game Boy button once; A selects/confirms, B cancels/deletes, and Start accepts END on name entry.", buttonSchema.merge(reasonSchema)),
      type_text: textTool("type_text", "Send literal text input to text-entry surfaces such as name entry. For full names, prefer clear:true and submit:true.", typeTextSchema),
      hold_button: textTool(
        "hold_button",
        "Hold a Game Boy button for a bounded number of frames.",
        buttonSchema.extend({ frames: z.number().int().min(1).max(25) }).merge(reasonSchema),
      ),
      status: textTool("status", "Get structured current game state.", emptySchema),
      recent_events: textTool("recent_events", "Get recent gameplay actions and outcomes.", z.object({ limit: z.number().int().min(1).max(100).optional() })),
    };
  }

  private async executeTool(name: string, args: Record<string, unknown>): Promise<McpToolResult> {
    emitAgentMcpCall(name, args);
    let result: McpToolResult;
    if (this.auth.mcpUrl) {
      const { client } = await this.directClient();
      result = (await client.callTool({ name, arguments: args })) as McpToolResult;
    } else {
      const response = (await fetchJson(buildToolsUrl(this.auth.baseUrl, this.auth.sessionId), {
        method: "POST",
        headers: {
          accept: "application/json",
          "content-type": "application/json",
          Authorization: `Bearer ${this.auth.token}`,
          "x-session-secret": this.auth.sessionSecret,
        },
        body: JSON.stringify({
          name,
          arguments: args,
        }),
      })) as {
        ok?: boolean;
        result?: McpToolResult;
      };

      if (!response.result) {
        throw new Error(`KrabbyClaw tool '${name}' did not return a result payload.`);
      }
      result = response.result;
    }

    emitAgentMcpResult(name, extractTextParts(result).join(" ").slice(0, 240));
    return result;
  }

  private async executeTextTool(name: string, args: Record<string, unknown>): Promise<string> {
    return extractTextParts(await this.executeTool(name, args)).join("\n");
  }

  async status(): Promise<Status> {
    return parseFirstJsonText(await this.executeTool("status", {}), value => statusSchema.parse(value));
  }

  async observe(options: { includeImage?: boolean; imageScale?: number } = {}): Promise<Observation> {
    return parseObservation(await this.executeTool("observe", {
      ...(options.includeImage ? { include_image: true } : {}),
      ...(options.imageScale ? { image_scale: options.imageScale } : {}),
    }));
  }

  async move(direction: "up" | "down" | "left" | "right", steps = 1): Promise<string> {
    return this.executeTextTool("move", { direction, steps });
  }

  async press(button: "A" | "B" | "Start" | "Select" | "Up" | "Down" | "Left" | "Right"): Promise<string> {
    return this.executeTextTool("press", { button: button.toLowerCase() });
  }

  async typeText(text: string, options: { clear?: boolean; submit?: boolean } = {}): Promise<string> {
    return this.executeTextTool("type_text", { text, ...options });
  }

  async holdButton(
    button: "A" | "B" | "Start" | "Select" | "Up" | "Down" | "Left" | "Right",
    frames: number,
  ): Promise<string> {
    return this.executeTextTool("hold_button", { button: button.toLowerCase(), frames });
  }

  async mapInfo(): Promise<string> {
    return this.executeTextTool("map_info", {});
  }

  async routeRender(): Promise<string> {
    return this.executeTextTool("route_render", {});
  }

  async flowState(): Promise<string> {
    return this.executeTextTool("flow_state", {});
  }

  async recentEvents(): Promise<string> {
    return this.executeTextTool("recent_events", {});
  }

  async disconnect(): Promise<void> {
    const client = this.mcpClient;
    this.mcpClient = null;
    this.playerToolsPromise = null;
    this.directToolsetPromise = null;
    const directClient = this.directMcpClientPromise;
    this.directMcpClientPromise = null;
    if (client) {
      await client.disconnect();
    }
    if (directClient) {
      const { client: mcpClient, transport } = await directClient;
      await transport.close();
      await mcpClient.close();
    }
  }
}

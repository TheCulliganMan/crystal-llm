import { resolveTransport } from "./runtime";
import { createSessionLogger, summarizeToolResult, type SessionLogger } from "./session-log";
import type { CliOptions, ToolContent, ToolEnvelope, ToolResult } from "./types";

type FetchLike = typeof fetch;

type SessionSecretResponse = {
  sessionSecret?: string;
};

export class PokecrystalToolsClient {
  private readonly toolsUrl: string;
  private readonly baseUrl: string;
  private readonly transport: "local" | "http";
  private readonly sessionId: string;
  private readonly sessionMode?: "automation" | "interactive";
  private readonly token?: string;
  private readonly sessionSecret?: string;
  private readonly fetchImpl: FetchLike;
  private readonly sessionLogger: SessionLogger;
  private readonly localExecutor?: (
    options: Pick<CliOptions, "sessionId" | "sessionMode" | "token" | "sessionSecret">,
    name: string,
    input?: Record<string, unknown>
  ) => Promise<ToolResult>;

  constructor(
    options: Pick<
      CliOptions,
      | "transport"
      | "toolsUrl"
      | "baseUrl"
      | "sessionId"
      | "sessionMode"
      | "token"
      | "sessionSecret"
      | "sessionLogEnabled"
      | "sessionLogFile"
    > & {
      fetchImpl?: FetchLike;
      localExecutor?: (
        options: Pick<CliOptions, "sessionId" | "sessionMode" | "token" | "sessionSecret">,
        name: string,
        input?: Record<string, unknown>
      ) => Promise<ToolResult>;
    }
  ) {
    this.transport = resolveTransport(options);
    this.toolsUrl = options.toolsUrl ?? "";
    this.baseUrl = options.baseUrl;
    this.sessionId = options.sessionId;
    this.sessionMode = options.sessionMode;
    this.token = options.token;
    this.sessionSecret = options.sessionSecret;
    this.fetchImpl = options.fetchImpl ?? fetch;
    this.sessionLogger = createSessionLogger(options);
    this.localExecutor = options.localExecutor;
  }

  private buildHeaders(): Record<string, string> {
    const headers: Record<string, string> = {
      accept: "application/json",
      "content-type": "application/json",
    };
    if (this.token) {
      headers.Authorization = `Bearer ${this.token}`;
    }
    if (this.sessionSecret) {
      headers["x-session-secret"] = this.sessionSecret;
    }
    if (this.sessionMode) {
      headers["x-pokecrystal-session-mode"] = this.sessionMode;
    }
    return headers;
  }

  async callTool(name: string, input: Record<string, unknown> = {}): Promise<ToolResult> {
    const startedAtMs = Date.now();
    this.sessionLogger.write("tool_call", {
      tool: name,
      transport: this.transport,
      input,
    });
    try {
      if (this.transport === "local") {
        const { callLocalTool } = await import("./local-runtime");
        const result = await (this.localExecutor ?? callLocalTool)(
          {
            sessionId: this.sessionId,
            sessionMode: this.sessionMode,
            token: this.token,
            sessionSecret: this.sessionSecret,
          },
          name,
          input
        );
        this.sessionLogger.write("tool_result", {
          tool: name,
          duration_ms: Date.now() - startedAtMs,
          result: summarizeToolResult(result),
        });
        return result;
      }
      const url = new URL(this.toolsUrl);
      url.searchParams.set("session_id", this.sessionId);
      const response = await this.fetchImpl(url.toString(), {
        method: "POST",
        headers: this.buildHeaders(),
        body: JSON.stringify({
          tool: name,
          input,
          session_id: this.sessionId,
        }),
      });
      const payload = (await response.json()) as ToolEnvelope;
      if (!response.ok || payload.ok === false) {
        throw new Error(payload.error ?? `Tool call failed (${response.status}).`);
      }
      const result = payload.result ?? { content: [] };
      this.sessionLogger.write("tool_result", {
        tool: name,
        duration_ms: Date.now() - startedAtMs,
        result: summarizeToolResult(result),
      });
      return result;
    } catch (error) {
      this.sessionLogger.write("tool_error", {
        tool: name,
        duration_ms: Date.now() - startedAtMs,
        error,
      });
      throw error;
    }
  }

  async registerIdentity(input: {
    agentId?: string;
    identityName?: string;
  }): Promise<{ tool: ToolResult; sessionSecret?: string }> {
    const tool = await this.callTool("register_identity", {
      name: input.identityName,
    });
    const payload = parseJsonText(tool.content).find(
      (entry) => typeof entry.token === "string" && typeof entry.playerId === "string"
    );
    if (!payload?.token || !payload?.playerId) {
      return { tool };
    }
    const sessionSecret =
      this.transport === "local"
        ? await this.buildLocalSessionSecret(payload.playerId)
        : await this.issueSessionSecret(payload.token);
    return { tool, sessionSecret };
  }

  private async buildLocalSessionSecret(playerId: string): Promise<string> {
    const { buildLocalSessionSecret } = await import("./local-runtime");
    return buildLocalSessionSecret(this.sessionId, playerId);
  }

  async issueSessionSecret(token: string): Promise<string | undefined> {
    const url = new URL("/api/arena/session-secret", this.baseUrl);
    url.searchParams.set("session_id", this.sessionId);
    const response = await this.fetchImpl(url.toString(), {
      headers: {
        accept: "application/json",
        Authorization: `Bearer ${token}`,
      },
    });
    if (!response.ok) {
      return undefined;
    }
    const payload = (await response.json()) as SessionSecretResponse;
    return typeof payload.sessionSecret === "string" ? payload.sessionSecret : undefined;
  }
}

export const extractText = (content?: ToolContent[]): string => {
  const textBlocks = (content ?? [])
    .map((entry) => (entry.type === "text" ? entry.text ?? "" : ""))
    .filter((entry) => entry.trim().length > 0);
  return textBlocks.join("\n\n");
};

export const parseJsonText = (content?: ToolContent[]): Array<Record<string, any>> =>
  (content ?? [])
    .filter((entry) => entry.type === "text" && typeof entry.text === "string")
    .map((entry) => entry.text as string)
    .map((entry) => {
      try {
        return JSON.parse(entry) as Record<string, any>;
      } catch {
        return null;
      }
    })
    .filter((entry): entry is Record<string, any> => Boolean(entry));

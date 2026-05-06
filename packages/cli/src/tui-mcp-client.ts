import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import { callLocalTool } from "./local-runtime";
import type { CliOptions, ToolResult } from "./types";

export type TuiMcpClient = {
  callTool: (name: string, args?: Record<string, unknown>) => Promise<ToolResult>;
  close: () => Promise<void>;
};

export const createTuiMcpClient = async (
  url: string,
  options: { fetchImpl?: typeof fetch } = {},
): Promise<TuiMcpClient> => {
  const client = new Client({ name: "pokecrystal-ink-tui", version: "1.0.0" });
  const transport = new StreamableHTTPClientTransport(new URL(url), {
    fetch: options.fetchImpl,
    requestInit: {
      headers: {
        accept: "application/json, text/event-stream",
      },
    },
  });
  await client.connect(transport);
  return {
    callTool: async (name, args = {}) => {
      const result = await client.callTool({ name, arguments: args }) as {
        content?: Array<Record<string, any>>;
        isError?: boolean;
      };
      return {
        content: result.content?.map((entry: Record<string, any>) => {
          if (entry.type === "text") {
            return { type: "text", text: entry.text };
          }
          if (entry.type === "image") {
            return { type: "image", data: entry.data, mimeType: entry.mimeType };
          }
          if (entry.type === "audio") {
            return { type: "audio", data: entry.data, mimeType: entry.mimeType };
          }
          return { type: entry.type };
        }),
        isError: typeof result.isError === "boolean" ? result.isError : undefined,
      };
    },
    close: async () => {
      await transport.close();
      await client.close();
    },
  };
};

export const createDirectLocalTuiMcpClient = (
  options: Pick<CliOptions, "sessionId" | "sessionMode" | "token" | "sessionSecret">,
): TuiMcpClient => ({
  callTool: (name, args = {}) =>
    callLocalTool(
      {
        sessionId: options.sessionId,
        sessionMode: options.sessionMode,
        token: options.token,
        sessionSecret: options.sessionSecret,
      },
      name,
      args,
    ),
  close: async () => undefined,
});

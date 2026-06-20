import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { PokecrystalToolsClient } from "./client";
import { getLocalMcpToolDefinitions } from "./local-runtime";
import type { CliOptions, ToolResult } from "./types";

type McpProxyOptions = CliOptions & {
  fetchImpl?: typeof fetch;
};

const buildContent = (result: ToolResult): NonNullable<ToolResult["content"]> =>
  result.content ?? [];

export const createMcpProxyServer = (options: McpProxyOptions): McpServer => {
  const server = new McpServer({
    name: "pokecrystal-cli",
    version: "1.0.0",
  });
  const client = new PokecrystalToolsClient({
    ...options,
    fetchImpl: options.fetchImpl,
  });

  for (const definition of getLocalMcpToolDefinitions()) {
    if (!definition.name) {
      continue;
    }
    server.registerTool(
      definition.name,
      {
        title: definition.title,
        description: definition.description,
        inputSchema: definition.inputSchema,
      } as Parameters<McpServer["registerTool"]>[1],
      (async (input: unknown) => {
        const parsed = definition.inputSchema.safeParse(input ?? {});
        if (!parsed.success) {
          const issue = parsed.error.issues[0];
          const pathText = issue?.path?.length ? ` (${issue.path.join(".")})` : "";
          return {
            content: [
              {
                type: "text",
                text: `Invalid tool arguments${pathText}: ${issue?.message ?? "schema validation failed"}`,
              },
            ],
            isError: true,
          };
        }
        const result = await client.callTool(definition.name!, parsed.data);
        return {
          content: buildContent(result),
          isError: result.isError,
        };
      }) as Parameters<McpServer["registerTool"]>[2]
    );
  }

  return server;
};

export const runMcpProxyServer = async (options: McpProxyOptions): Promise<void> => {
  const server = createMcpProxyServer(options);
  await server.connect(new StdioServerTransport());
};

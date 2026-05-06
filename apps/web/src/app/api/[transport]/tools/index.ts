import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runToolWithTelemetry } from "./common";
import { MCP_TOOL_DEFINITIONS } from "./registry";

export function registerTools(server: McpServer) {
  for (const definition of MCP_TOOL_DEFINITIONS) {
    server.registerTool(
      definition.name,
      {
        title: definition.title,
        description: definition.description,
        inputSchema: definition.inputSchema,
      },
      (async (input: unknown, extra: unknown) =>
        runToolWithTelemetry(
          definition.name,
          input,
          definition.handler,
          extra as Parameters<typeof definition.handler>[1]
        )) as Parameters<McpServer["registerTool"]>[2]
    );
  }
}

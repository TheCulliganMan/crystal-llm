import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { registerTools } from "./index";
import { MAX_ADVANCE_FRAMES } from "./common";
import { getMcpToolDefinition, MCP_TOOL_DEFINITIONS } from "./registry";

describe("registerTools", () => {
  it("publishes execute_macro limits from runtime settings", () => {
    const registerTool = jest.fn();
    const server = { registerTool } as unknown as McpServer;
    registerTools(server);

    const executeMacroCall = registerTool.mock.calls.find((call) => call[0] === "execute_macro");
    expect(executeMacroCall).toBeDefined();
    const metadata = executeMacroCall?.[1] as { description?: string } | undefined;
    expect(metadata?.description).toContain(`times?: 1-${MAX_ADVANCE_FRAMES}`);
    expect(metadata?.description).toContain(`max_presses?: 1-${MAX_ADVANCE_FRAMES}`);
    expect(metadata?.description).toContain(`"interact"`);
    expect(metadata?.description).toContain(`"approach_target"`);
    expect(metadata?.description).not.toContain(`type: "wait"`);
  });

  it("registers every tool from the shared registry", () => {
    const registerTool = jest.fn();
    const server = { registerTool } as unknown as McpServer;
    registerTools(server);
    expect(registerTool).toHaveBeenCalledTimes(MCP_TOOL_DEFINITIONS.length);
    expect(registerTool.mock.calls.map((call) => call[0])).toEqual(
      MCP_TOOL_DEFINITIONS.map((definition) => definition.name)
    );
  });

  it("accepts optional training metadata on tool input schemas", () => {
    const definition = getMcpToolDefinition("move");
    const parsed = definition?.inputSchema.safeParse({
      direction: "up",
      reasoning: "heading toward the door",
      goal: "enter next map",
    });
    expect(parsed?.success).toBe(true);
  });

  it("does not publish wait on the MCP tool surface", () => {
    expect(MCP_TOOL_DEFINITIONS.map((definition) => definition.name)).not.toContain("wait");
    expect(getMcpToolDefinition("wait")).toBeUndefined();
  });
});

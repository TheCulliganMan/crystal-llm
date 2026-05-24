import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { serializeMessage } from "@modelcontextprotocol/sdk/shared/stdio.js";
import type { JSONRPCMessage } from "@modelcontextprotocol/sdk/types.js";
import { PassThrough } from "node:stream";
import { createMcpProxyServer } from "../mcp-server";
import { createFakeFetch, createJsonResponse } from "./test-helpers";

const waitForMessage = (
  messages: JSONRPCMessage[],
  predicate: (message: JSONRPCMessage) => boolean
): Promise<JSONRPCMessage> =>
  new Promise((resolve) => {
    const timer = setInterval(() => {
      const found = messages.find(predicate);
      if (found) {
        clearInterval(timer);
        resolve(found);
      }
    }, 5);
  });

const hasMessageId = (message: JSONRPCMessage, id: number): boolean => "id" in message && message.id === id;

describe("MCP stdio e2e", () => {
  it("speaks JSON-RPC over stdio and proxies a tool call", async () => {
    const { fetch, calls } = createFakeFetch({
      "/api/mcp/tools": (_url, _init, body) => {
        const toolName = body && typeof body === "object" ? (body as { tool?: string }).tool ?? "" : "";
        if (toolName === "register_identity") {
          return createJsonResponse({
            ok: true,
            result: {
              content: [
                {
                  type: "text",
                  text: JSON.stringify({ token: "token-xyz" }),
                },
              ],
            },
          });
        }
        return createJsonResponse({ ok: false, error: `unexpected tool ${toolName}` }, 500);
      },
      "/api/arena/session-secret": () =>
        createJsonResponse({
          sessionSecret: "secret-xyz",
        }),
    });

    const serverInput = new PassThrough();
    const serverOutput = new PassThrough();
    const messages: JSONRPCMessage[] = [];
    let buffer = "";
    serverOutput.on("data", (chunk: Buffer | string) => {
      buffer += Buffer.isBuffer(chunk) ? chunk.toString("utf8") : String(chunk);
      let index = buffer.indexOf("\n");
      while (index !== -1) {
        const line = buffer.slice(0, index).trim();
        buffer = buffer.slice(index + 1);
        if (line) {
          messages.push(JSON.parse(line) as JSONRPCMessage);
        }
        index = buffer.indexOf("\n");
      }
    });

    const server = createMcpProxyServer({
      command: "mcp",
      baseUrl: "http://localhost:3000",
      toolsUrl: "http://localhost:3000/api/mcp/tools",
      sessionId: "session-1",
      fetchImpl: fetch,
    } as any);
    await server.connect(new StdioServerTransport(serverInput, serverOutput));

    serverInput.write(
      serializeMessage({
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test-client", version: "1.0.0" },
        },
      })
    );

    const initializeResponse = (await waitForMessage(messages, (message) => hasMessageId(message, 1) && "result" in message)) as {
      result: { protocolVersion: string };
    } & JSONRPCMessage;
    expect(initializeResponse.result.protocolVersion).toBeTruthy();

    serverInput.write(
      serializeMessage({
        jsonrpc: "2.0",
        method: "initialized",
        params: {},
      })
    );

    serverInput.write(
      serializeMessage({
        jsonrpc: "2.0",
        id: 2,
        method: "tools/list",
        params: {},
      })
    );
    const toolsListResponse = (await waitForMessage(messages, (message) => hasMessageId(message, 2) && "result" in message)) as {
      result: { tools?: Array<{ name: string; inputSchema?: unknown }> };
    } & JSONRPCMessage;
    const toolNames = toolsListResponse.result.tools?.map((tool: { name: string }) => tool.name);
    expect(toolNames).toContain("register_identity");
    expect(toolNames).toContain("route_render");
    expect(toolNames).not.toContain("wait");
    expect(JSON.stringify(toolsListResponse.result.tools?.find((tool) => tool.name === "route_render")?.inputSchema)).not.toContain("map_name");
    expect(JSON.stringify(toolsListResponse.result.tools)).not.toContain("toon");
    expect(JSON.stringify(toolsListResponse.result.tools?.find((tool) => tool.name === "status")?.inputSchema)).toContain("json");
    expect(JSON.stringify(toolsListResponse.result.tools?.find((tool) => tool.name === "type_text")?.inputSchema)).toContain("clear");
    expect(JSON.stringify(toolsListResponse.result.tools?.find((tool) => tool.name === "type_text")?.inputSchema)).toContain("submit");

    serverInput.write(
      serializeMessage({
        jsonrpc: "2.0",
        id: 3,
        method: "tools/call",
        params: {
          name: "register_identity",
          arguments: {
            agentId: "oak-lab-runner",
            identityName: "trainer-2",
          },
        },
      })
    );

    const toolResponse = (await waitForMessage(messages, (message) => hasMessageId(message, 3) && "result" in message)) as {
      result: { content?: Array<{ type: string; text?: string }> };
    } & JSONRPCMessage;
    expect(toolResponse.result.content?.[0]?.text).toContain("token-xyz");
    expect(calls.length).toBeGreaterThanOrEqual(1);

    await server.close();
  });

  it("proxies one type_text call with clear and submit for name entry", async () => {
    const { fetch, calls } = createFakeFetch({
      "/api/mcp/tools": (_url, _init, body) => {
        const payload = body as { tool?: string; input?: Record<string, unknown> } | null;
        if (payload?.tool !== "type_text") {
          return createJsonResponse({ ok: false, error: `unexpected tool ${payload?.tool ?? ""}` }, 500);
        }
        return createJsonResponse({
          ok: true,
          result: {
            content: [
              {
                type: "text",
                text: JSON.stringify({ action: { ok: true, changed: true, events: ["deleted", "typed:do", "submitted"] } }),
              },
            ],
          },
        });
      },
    });
    const serverInput = new PassThrough();
    const serverOutput = new PassThrough();
    const messages: JSONRPCMessage[] = [];
    let buffer = "";
    serverOutput.on("data", (chunk: Buffer | string) => {
      buffer += Buffer.isBuffer(chunk) ? chunk.toString("utf8") : String(chunk);
      let index = buffer.indexOf("\n");
      while (index !== -1) {
        const line = buffer.slice(0, index).trim();
        buffer = buffer.slice(index + 1);
        if (line) {
          messages.push(JSON.parse(line) as JSONRPCMessage);
        }
        index = buffer.indexOf("\n");
      }
    });

    const server = createMcpProxyServer({
      command: "mcp",
      baseUrl: "http://localhost:3000",
      toolsUrl: "http://localhost:3000/api/mcp/tools",
      sessionId: "session-name-entry-type-text",
      fetchImpl: fetch,
    } as any);
    await server.connect(new StdioServerTransport(serverInput, serverOutput));

    serverInput.write(
      serializeMessage({
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test-client", version: "1.0.0" },
        },
      })
    );
    await waitForMessage(messages, (message) => hasMessageId(message, 1) && "result" in message);
    serverInput.write(serializeMessage({ jsonrpc: "2.0", method: "initialized", params: {} }));

    serverInput.write(
      serializeMessage({
        jsonrpc: "2.0",
        id: 2,
        method: "tools/call",
        params: {
          name: "type_text",
          arguments: {
            text: "do",
            clear: true,
            submit: true,
          },
        },
      })
    );

    const response = (await waitForMessage(messages, (message) => hasMessageId(message, 2) && "result" in message)) as {
      result: { content?: Array<{ type: string; text?: string }> };
    } & JSONRPCMessage;
    expect(response.result.content?.[0]?.text).toContain("submitted");
    expect(calls.at(-1)?.body).toMatchObject({
      tool: "type_text",
      input: { text: "do", clear: true, submit: true },
    });
    await server.close();
  });

  it("normalizes directional hold_button input before proxying", async () => {
    const { fetch, calls } = createFakeFetch({
      "/api/mcp/tools": (_url, _init, body) => {
        const payload = body as { tool?: string; input?: Record<string, unknown> } | null;
        if (payload?.tool !== "hold_button") {
          return createJsonResponse({ ok: false, error: `unexpected tool ${payload?.tool ?? ""}` }, 500);
        }
        return createJsonResponse({
          ok: true,
          result: {
            content: [
              {
                type: "text",
                text: JSON.stringify({ action: { ok: true, changed: true, events: ["held:left:3"] } }),
              },
            ],
          },
        });
      },
    });
    const serverInput = new PassThrough();
    const serverOutput = new PassThrough();
    const messages: JSONRPCMessage[] = [];
    let buffer = "";
    serverOutput.on("data", (chunk: Buffer | string) => {
      buffer += Buffer.isBuffer(chunk) ? chunk.toString("utf8") : String(chunk);
      let index = buffer.indexOf("\n");
      while (index !== -1) {
        const line = buffer.slice(0, index).trim();
        buffer = buffer.slice(index + 1);
        if (line) {
          messages.push(JSON.parse(line) as JSONRPCMessage);
        }
        index = buffer.indexOf("\n");
      }
    });

    const server = createMcpProxyServer({
      command: "mcp",
      baseUrl: "http://localhost:3000",
      toolsUrl: "http://localhost:3000/api/mcp/tools",
      sessionId: "session-hold-direction",
      fetchImpl: fetch,
    } as any);
    await server.connect(new StdioServerTransport(serverInput, serverOutput));

    serverInput.write(serializeMessage({
      jsonrpc: "2.0",
      id: 1,
      method: "initialize",
      params: {
        protocolVersion: "2024-11-05",
        capabilities: {},
        clientInfo: { name: "test-client", version: "1.0.0" },
      },
    }));
    await waitForMessage(messages, (message) => hasMessageId(message, 1) && "result" in message);
    serverInput.write(serializeMessage({ jsonrpc: "2.0", method: "initialized", params: {} }));

    serverInput.write(serializeMessage({
      jsonrpc: "2.0",
      id: 2,
      method: "tools/call",
      params: {
        name: "hold_button",
        arguments: {
          button: " Left ",
          frames: 3,
        },
      },
    }));

    await waitForMessage(messages, (message) => hasMessageId(message, 2) && "result" in message);
    expect(calls.at(-1)?.body).toMatchObject({
      tool: "hold_button",
      input: { button: "left", frames: 3 },
    });
    await server.close();
  });
});

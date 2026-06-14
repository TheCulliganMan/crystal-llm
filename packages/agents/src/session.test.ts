import fs from "node:fs";
import { z } from "zod";

import {
  bootstrapSession,
  deriveIdentity,
  extractTextParts,
  KrabbyClawSession,
  mapMcpToolsToDirectPlayerTools,
  mapMcpToolsToPlayerTools,
  parseFirstJsonText,
  parseObservation,
  prepareVisibleActionToolInput,
  type McpToolResult,
} from "./session.js";

describe("session helpers", () => {
  const originalSessionDir = process.env.POKECRYSTAL_AGENT_SESSION_DIR;

  beforeEach(() => {
    process.env.POKECRYSTAL_AGENT_SESSION_DIR = `${process.cwd()}/packages/agents/.tmp-session-auth-tests`;
    fs.rmSync(process.env.POKECRYSTAL_AGENT_SESSION_DIR, { recursive: true, force: true });
  });

  afterEach(() => {
    jest.restoreAllMocks();
    process.env.POKECRYSTAL_AGENT_SESSION_DIR = originalSessionDir;
  });

  it("derives default identity values from the session id", () => {
    expect(
      deriveIdentity({
        baseUrl: "http://127.0.0.1:3000",
        sessionId: "run-123",
      }),
    ).toEqual({
      agentId: "run-123",
      identityName: "trainer-run-123",
    });
  });

  it("extracts text parts and parses json payloads", () => {
    const result: McpToolResult = {
      content: [
        { type: "text", text: '{"mode":"overworld","map":"PlayersHouse2F","mapId":"24:7","coords":[3,3],"facing":"down","badges":0,"canMove":true,"partyCount":0,"flowSummary":"x","flowNextGoal":"Starter","flowCompletionTarget":"Beat Mt. Silver"}' },
      ],
    };

    expect(extractTextParts(result)).toHaveLength(1);
    expect(parseFirstJsonText(result, (value: unknown) => value)).toMatchObject({
      mode: "overworld",
      map: "PlayersHouse2F",
    });
  });

  it("parses observations with both summary and snapshot text", () => {
    const observation = parseObservation({
      content: [
        { type: "text", text: "OVERWORLD\nPos: (3,3)" },
        { type: "image", data: "pngbase64", mimeType: "image/png" },
        {
          type: "text",
          text: '{"flow_state":{"sum":"Next goal","done":0,"total":22,"next":"Starter","target":"Beat Mt. Silver"},"view":{"focus":"overworld"}}',
        },
      ],
    });

    expect(observation.summaryText).toContain("OVERWORLD");
    expect(observation.snapshot?.flow_state?.next).toBe("Starter");
    expect(observation.image).toEqual({ data: "pngbase64", mimeType: "image/png" });
  });

  it("separates visible action reasons from MCP tool arguments", () => {
    expect(
      prepareVisibleActionToolInput("press", {
        button: "a",
        reason: "  The starter ball is directly ahead, so I need to press A.  ",
      }),
    ).toEqual({
      args: { button: "a" },
      reason: "The starter ball is directly ahead, so I need to press A.",
    });

    expect(
      prepareVisibleActionToolInput("status", {
        reason: "Checking state",
      }),
    ).toEqual({
      args: { reason: "Checking state" },
      reason: null,
    });
  });

  it("normalizes direct linked-agent button casing before MCP calls", async () => {
    const session = new KrabbyClawSession({
      baseUrl: "http://127.0.0.1",
      mcpUrl: "http://127.0.0.1/mcp",
      sessionId: "direct-button-case",
      agentId: "agent",
      identityName: "trainer",
      token: "direct-mcp",
      sessionSecret: "direct-mcp",
    });
    const tools = await session.listPlayerTools();
    expect(tools.wait).toBeUndefined();

    const pressSchema = tools.press.inputSchema as z.ZodTypeAny;
    const holdButtonSchema = tools.hold_button.inputSchema as z.ZodTypeAny;

    expect(pressSchema.safeParse({ button: " A ", reason: "Confirm the current prompt." })).toMatchObject({
      success: true,
      data: { button: "a" },
    });
    expect(holdButtonSchema.safeParse({
      button: " Select ",
      frames: 2,
      reason: "Open select-bound action.",
    })).toMatchObject({
      success: true,
      data: { button: "select", frames: 2 },
    });
    await session.disconnect();
  });

  it("accepts both namespaced and direct MCP gameplay tool names", () => {
    const status = { execute: jest.fn() };
    const move = { execute: jest.fn() };
    const routeRender = { execute: jest.fn() };
    const registerIdentity = { execute: jest.fn() };
    const tools = {
      status,
      krabbyclaw_move: move,
      route_render: routeRender,
      register_identity: registerIdentity,
    };

    expect(Object.keys(mapMcpToolsToPlayerTools(tools as any)).sort()).toEqual(["krabbyclaw_move", "route_render", "status"]);
    expect(mapMcpToolsToDirectPlayerTools(tools as any)).toEqual({
      status,
      move,
      route_render: routeRender,
    });
  });

  it("bootstraps a token and session secret through the HTTP bootstrap flow", async () => {
    const fetchMock = jest
      .spyOn(global, "fetch")
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          result: {
            content: [{ text: JSON.stringify({ token: "token-123" }) }],
          },
        }),
      } as Response)
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          sessionSecret: "secret-456",
        }),
      } as Response);

    const auth = await bootstrapSession({
      baseUrl: "http://127.0.0.1:3000",
      sessionId: "run-123",
    });

    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(auth.agentId).toBe("run-123");
    expect(auth.identityName).toBe("trainer-run-123");
    expect(auth.token).toBe("token-123");
    expect(auth.sessionSecret).toBe("secret-456");
  });

  it("reuses cached session auth on restart and only refreshes the session secret", async () => {
    const firstFetchMock = jest
      .spyOn(global, "fetch")
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          result: {
            content: [{ text: JSON.stringify({ token: "token-123" }) }],
          },
        }),
      } as Response)
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          sessionSecret: "secret-456",
        }),
      } as Response);

    const firstAuth = await bootstrapSession({
      baseUrl: "http://127.0.0.1:3000",
      sessionId: "restartable-run",
    });

    expect(firstFetchMock).toHaveBeenCalledTimes(2);
    jest.restoreAllMocks();

    const secondFetchMock = jest
      .spyOn(global, "fetch")
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          sessionSecret: "secret-789",
        }),
      } as Response);

    const resumedAuth = await bootstrapSession({
      baseUrl: "http://127.0.0.1:3000",
      sessionId: "restartable-run",
    });

    expect(secondFetchMock).toHaveBeenCalledTimes(1);
    expect(resumedAuth.token).toBe(firstAuth.token);
    expect(resumedAuth.agentId).toBe(firstAuth.agentId);
    expect(resumedAuth.sessionSecret).toBe("secret-789");
  });

  it("falls back to cached auth when session-secret refresh is forbidden during restart", async () => {
    const firstFetchMock = jest
      .spyOn(global, "fetch")
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          result: {
            content: [{ text: JSON.stringify({ token: "token-123" }) }],
          },
        }),
      } as Response)
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          sessionSecret: "secret-456",
        }),
      } as Response);

    const firstAuth = await bootstrapSession({
      baseUrl: "http://127.0.0.1:3000",
      sessionId: "resume-forbidden-run",
    });

    expect(firstFetchMock).toHaveBeenCalledTimes(2);
    jest.restoreAllMocks();

    const forbiddenFetchMock = jest
      .spyOn(global, "fetch")
      .mockResolvedValueOnce({
        ok: false,
        status: 403,
        statusText: "Forbidden",
      } as Response);

    const resumedAuth = await bootstrapSession({
      baseUrl: "http://127.0.0.1:3000",
      sessionId: "resume-forbidden-run",
    });

    expect(forbiddenFetchMock).toHaveBeenCalledTimes(1);
    expect(resumedAuth).toEqual(firstAuth);
  });
});

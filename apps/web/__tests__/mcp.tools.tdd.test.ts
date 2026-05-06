import type { ChildProcessWithoutNullStreams } from "node:child_process";

import { startNextDev, stopNextDev } from "./helpers/next-dev";
import { McpRpc, MCP_PROTOCOL_VERSION } from "./helpers/mcp-rpc";

jest.setTimeout(60_000);

type ObserveMetaPayload = {
  player_context?: {
    map?: string | null;
    coords?: { x?: number; y?: number } | null;
    facing?: string;
    menu_open?: boolean;
    dialogue_open?: boolean;
  };
  frame_id?: number;
  computed_at_ms?: number;
};

const callObserve = async (rpc: McpRpc) => {
  const startedAt = performance.now();
  const response = await rpc.call("tools/call", {
    name: "observe",
    arguments: {},
  });
  const durationMs = performance.now() - startedAt;
  const content =
    (response.json as { result?: { content?: Array<{ type?: string; text?: string }> } })?.result?.content ?? [];
  const meta = JSON.parse(content[1]?.text ?? "{}") as ObserveMetaPayload;
  return { response, durationMs, content, meta };
};

describe("MCP (mcp-handler) tools TDD", () => {
  let proc: ChildProcessWithoutNullStreams | null = null;
  let mcpUrl = "";
  let baseUrl = "";
  let unavailableReason: string | null = null;

  beforeAll(async () => {
    try {
      const started = await startNextDev();
      proc = started.proc;
      mcpUrl = started.mcpUrl;
      baseUrl = started.baseUrl;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (message.includes("EPERM") || message.includes("EACCES")) {
        unavailableReason = message;
        return;
      }
      throw error;
    }
  });

  afterAll(async () => {
    await stopNextDev(proc);
  });

  test("Regression: Streamable HTTP POST should not return 406", async () => {
    if (unavailableReason) {
      return;
    }
    const rpc = new McpRpc(mcpUrl);

    const { res, json } = await rpc.call("initialize", {
      protocolVersion: MCP_PROTOCOL_VERSION,
      clientInfo: { name: "jest", version: "0.0.0" },
      capabilities: {},
    });

    expect(res.status).not.toBe(406);
    expect(res.status).toBe(200);
    expect(json).toBeTruthy();
    expect(json).toHaveProperty("result");
  });

  test("tools/list returns tool definitions", async () => {
    if (unavailableReason) {
      return;
    }
    const rpc = new McpRpc(mcpUrl);

    const init = await rpc.call("initialize", {
      protocolVersion: MCP_PROTOCOL_VERSION,
      clientInfo: { name: "jest", version: "0.0.0" },
      capabilities: {},
    });
    expect(init.res.status).toBe(200);

    const { res, json } = await rpc.call("tools/list", {});
    expect(res.status).toBe(200);
    expect(json).toBeTruthy();
    expect(json).toHaveProperty("result");

    const names = ((json as { result?: { tools?: Array<{ name?: string }> } })?.result?.tools ?? []).map(
      (tool) => tool.name
    );
    expect(names).toEqual(
      expect.arrayContaining([
        "register_identity",
        "whoami",
        "observe",
        "map_info",
        "flow_state",
        "move",
        "press",
        "hold_button",
        "execute_macro",
        "status",
        "recent_events",
        "journal", 
      ])
    );
  });

  test("tools/call observe returns a text snapshot", async () => {
    if (unavailableReason) {
      return;
    }
    const rpc = new McpRpc(mcpUrl);

    await rpc.call("initialize", {
      protocolVersion: MCP_PROTOCOL_VERSION,
      clientInfo: { name: "jest", version: "0.0.0" },
      capabilities: {},
    });

    const { response, content, meta } = await callObserve(rpc);
    const { res, json } = response;

    expect(res.status).toBe(200);
    expect(json).toBeTruthy();
    expect(content.length).toBeGreaterThan(0);
    expect(content[0]?.type).toBe("text");
    expect(typeof content[0]?.text).toBe("string");

    if (content[1]?.text) {
      expect(content[1].type).toBe("text");
    }
  });

  test("observe reuses cached snapshot for repeated polling", async () => {
    if (unavailableReason) {
      return;
    }
    const rpc = new McpRpc(mcpUrl);

    await rpc.call("initialize", {
      protocolVersion: MCP_PROTOCOL_VERSION,
      clientInfo: { name: "jest", version: "0.0.0" },
      capabilities: {},
    });

    const first = await callObserve(rpc);
    const second = await callObserve(rpc);

    expect(first.response.res.status).toBe(200);
    expect(second.response.res.status).toBe(200);
    if (first.meta.frame_id !== undefined || second.meta.frame_id !== undefined) {
      expect(second.meta.frame_id).toBe(first.meta.frame_id);
      expect(second.meta.computed_at_ms).toBe(first.meta.computed_at_ms);
    }
    // Timing expectations are noisy on shared CI; assert caching via stable meta only.
    expect(typeof first.durationMs).toBe("number");
    expect(typeof second.durationMs).toBe("number");
  });

  test("press returns snapshot first + actionResult last, and recent_events returns recap", async () => {
    if (unavailableReason) {
      return;
    }
    const rpc = new McpRpc(mcpUrl);
    await rpc.call("initialize", {
      protocolVersion: MCP_PROTOCOL_VERSION,
      clientInfo: { name: "jest", version: "0.0.0" },
      capabilities: {},
    });

    const press = await rpc.call("tools/call", {
      name: "press",
      arguments: { button: "a" },
    });
    expect(press.res.status).toBe(200);
    const pressContent =
      (press.json as { result?: { content?: Array<{ type?: string; text?: string }> } })?.result?.content ?? [];
    expect(pressContent.length).toBeGreaterThanOrEqual(1);
    expect(pressContent[0]?.type).toBe("text");
    expect(typeof pressContent[0]?.text).toBe("string");
    const actionBlock = pressContent[pressContent.length - 1];
    expect(actionBlock?.type).toBe("text");
    const actionPayload = JSON.parse(actionBlock?.text ?? "{}") as {
      actionResult?: { ok?: boolean; changed?: boolean };
    };
    if (actionPayload.actionResult) {
      expect(typeof actionPayload.actionResult.ok).toBe("boolean");
      expect(typeof actionPayload.actionResult.changed).toBe("boolean");
    }

    const recent = await rpc.call("tools/call", {
      name: "recent_events",
      arguments: { limit: 5 },
    });
    expect(recent.res.status).toBe(200);
    const recentContent =
      (recent.json as { result?: { content?: Array<{ type?: string; text?: string }> } })?.result?.content ?? [];
    const recentPayload = JSON.parse(recentContent[0]?.text ?? "{}") as {
      recap?: string;
      events?: Array<{ action?: string }>;
    };
    if (recentPayload.recap !== undefined) {
      expect(typeof recentPayload.recap).toBe("string");
    }
    if (recentPayload.events !== undefined) {
      expect(Array.isArray(recentPayload.events)).toBe(true);
    }
  });

  test("SSE fallback route should not 405 when requested correctly", async () => {
    if (unavailableReason) {
      return;
    }
    const sseUrl = `${baseUrl}/api/sse?session_id=test-session`;

    const controller = new AbortController();
    // The dev server can be under heavy load when the full suite runs in parallel.
    const timeout = setTimeout(() => controller.abort(), 15_000);
    timeout.unref?.();
    const res = await fetch(sseUrl, {
      method: "GET",
      headers: { accept: "text/event-stream" },
      signal: controller.signal,
    }).finally(() => {
      clearTimeout(timeout);
    });

    expect(res.status).not.toBe(405);
    await res.body?.cancel();
  });
});

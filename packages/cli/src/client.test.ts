import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { extractText, parseJsonText, PokecrystalToolsClient } from "./client";

describe("PokecrystalToolsClient", () => {
  it("posts tools calls with session and auth headers", async () => {
    const fetchImpl = jest.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        ok: true,
        result: {
          content: [{ type: "text", text: "hello" }],
        },
      }),
    });
    const client = new PokecrystalToolsClient({
      transport: "http",
      baseUrl: "http://localhost:3000",
      toolsUrl: "http://localhost:3000/api/mcp/tools",
      sessionId: "session-123",
      sessionMode: "interactive",
      token: "token-1",
      sessionSecret: "secret-1",
      fetchImpl: fetchImpl as typeof fetch,
    });

    const result = await client.callTool("observe", { include_image: false });

    expect(result.content?.[0]?.text).toBe("hello");
    expect(fetchImpl).toHaveBeenCalledWith(
      "http://localhost:3000/api/mcp/tools?session_id=session-123",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          Authorization: "Bearer token-1",
          "x-session-secret": "secret-1",
          "x-pokecrystal-session-mode": "interactive",
        }),
        body: JSON.stringify({
          tool: "observe",
          input: { include_image: false },
          session_id: "session-123",
        }),
      })
    );
  });

  it("calls the local executor in local mode", async () => {
    const localExecutor = jest.fn().mockResolvedValue({
      content: [{ type: "text", text: "local hello" }],
    });
    const client = new PokecrystalToolsClient({
      transport: "local",
      baseUrl: "",
      sessionId: "local-session",
      sessionMode: "interactive",
      localExecutor,
    });

    const result = await client.callTool("observe", { include_image: false });

    expect(result.content?.[0]?.text).toBe("local hello");
    expect(localExecutor).toHaveBeenCalledWith(
      {
        sessionId: "local-session",
        sessionMode: "interactive",
        token: undefined,
        sessionSecret: undefined,
      },
      "observe",
      { include_image: false }
    );
  });

  it("writes session JSONL for tool calls when a log file is configured", async () => {
    const logDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-client-log-"));
    const logFile = path.join(logDir, "session.jsonl");
    const localExecutor = jest.fn().mockResolvedValue({
      content: [
        { type: "text", text: "{\"mode\":\"battle\"}" },
        { type: "image", data: "abc123", mimeType: "image/png" },
      ],
    });
    const client = new PokecrystalToolsClient({
      transport: "local",
      baseUrl: "",
      sessionId: "local-session",
      sessionMode: "interactive",
      sessionLogEnabled: true,
      sessionLogFile: logFile,
      localExecutor,
    });

    await client.callTool("observe", { include_image: true });

    const entries = fs.readFileSync(logFile, "utf8").trim().split("\n").map((line) => JSON.parse(line));
    expect(entries.map((entry) => entry.event)).toEqual(["tool_call", "tool_result"]);
    expect(entries[0]).toMatchObject({
      session_id: "local-session",
      tool: "observe",
      input: { include_image: true },
    });
    expect(entries[1].result.content).toEqual([
      { type: "text", text: "{\"mode\":\"battle\"}", text_length: 17 },
      { type: "image", mime_type: "image/png", data_length: 6 },
    ]);
  });

  it("parses JSON text helper blocks", () => {
    const content = [
      { type: "text", text: "snapshot" },
      { type: "text", text: "{\"status\":{\"mode\":\"overworld\"}}" },
    ];

    expect(extractText(content)).toContain("snapshot");
    expect(parseJsonText(content)).toEqual([{ status: { mode: "overworld" } }]);
  });
});

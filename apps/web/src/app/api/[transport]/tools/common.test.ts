const mockReportArenaEvent = jest.fn();

jest.mock("@/arena/runtime/telemetry", () => ({
  ensureArenaRunForSession: jest.fn(),
  reportArenaSnapshot: jest.fn(),
  reportArenaEvent: (...args: unknown[]) => mockReportArenaEvent(...args),
}));

jest.mock("@pokecrystal/core/core/config", () => ({
  getSettings: () => ({ mcpMaxActionsPerCall: 25 }),
}));

jest.mock("./identity", () => ({
  resolveIdentityFromExtra: jest.fn(() => null),
}));

jest.mock("@pokecrystal/core/core/mcp-identity-context.server", () => ({
  runWithMcpIdentityContext: jest.fn((_identity: unknown, fn: () => unknown) => fn()),
}));

const mockEnsureReady = jest.fn().mockResolvedValue(undefined);
const mockSetInteractiveMode = jest.fn();
const mockSetInstantMode = jest.fn();
const mockGetMcpSession = jest.fn(() => ({
  ensureReady: mockEnsureReady,
  setInteractiveMode: mockSetInteractiveMode,
  setInstantMode: mockSetInstantMode,
}));

jest.mock("@/app/mcp/session", () => ({
  getMcpSession: (...args: unknown[]) => mockGetMcpSession(...args),
}));

import { loadSession, resolveInstantMode, resolveSessionMode, runToolWithTelemetry } from "./common";

describe("runToolWithTelemetry", () => {
  beforeEach(() => {
    mockReportArenaEvent.mockReset();
    mockEnsureReady.mockClear();
    mockSetInteractiveMode.mockClear();
    mockSetInstantMode.mockClear();
    mockGetMcpSession.mockClear();
  });

  it("records tool call/result events for valid sessions", async () => {
    const handler = jest.fn().mockResolvedValue({
      content: [{ type: "text", text: "ok" }],
    });

    await runToolWithTelemetry(
      "move",
      { direction: "up", reasoning: "walk north" },
      handler,
      {
        requestInfo: {
          headers: { "mcp-session-id": "session-1" },
        },
      }
    );

    expect(handler).toHaveBeenCalledTimes(1);
    expect(mockReportArenaEvent).toHaveBeenCalledTimes(2);
    expect(mockReportArenaEvent.mock.calls[0][0]).toMatchObject({
      sessionId: "session-1",
      label: "tool_call",
      action: "tool:move",
      payload: expect.objectContaining({
        phase: "call",
        input: expect.objectContaining({
          direction: "up",
          reasoning: "walk north",
        }),
      }),
    });
    expect(mockReportArenaEvent.mock.calls[1][0]).toMatchObject({
      sessionId: "session-1",
      label: "tool_result",
      action: "tool:move",
      payload: expect.objectContaining({
        phase: "result",
        response: expect.objectContaining({
          is_error: false,
          content_count: 1,
        }),
      }),
    });
  });

  it("redacts sensitive keys and omits binary content from training payloads", async () => {
    const handler = jest.fn().mockResolvedValue({
      content: [{ type: "image", data: "a".repeat(256), mimeType: "image/png" }],
    });

    await runToolWithTelemetry(
      "observe",
      { token: "secret-token", api_key: "abc123", nested: { password: "pw" } },
      handler,
      {
        requestInfo: {
          headers: { "mcp-session-id": "session-2" },
        },
      }
    );

    const callEvent = mockReportArenaEvent.mock.calls[0][0];
    expect(callEvent.payload.input).toMatchObject({
      token: "[redacted]",
      api_key: "[redacted]",
      nested: { password: "[redacted]" },
    });

    const resultEvent = mockReportArenaEvent.mock.calls[1][0];
    expect(resultEvent.payload.response).toMatchObject({
      content_preview: [{ data_bytes: 256, mime_type: "image/png", type: "image" }],
    });
  });

  it("records exceptions and rethrows handler errors", async () => {
    const handler = jest.fn().mockRejectedValue(new Error("boom"));

    await expect(
      runToolWithTelemetry(
        "press",
        { button: "a" },
        handler,
        {
          requestInfo: {
            headers: { "mcp-session-id": "session-3" },
          },
        }
      )
    ).rejects.toThrow("boom");

    expect(mockReportArenaEvent).toHaveBeenCalledTimes(2);
    expect(mockReportArenaEvent.mock.calls[1][0]).toMatchObject({
      sessionId: "session-3",
      label: "tool_exception",
      action: "tool:press",
      payload: expect.objectContaining({
        phase: "exception",
        error: expect.objectContaining({
          message: "boom",
        }),
      }),
    });
  });

  it("parses interactive session mode headers for playable CLI calls", () => {
    expect(
      resolveSessionMode({
        requestInfo: {
          headers: { "x-pokecrystal-session-mode": "interactive" },
        },
      })
    ).toBe("interactive");
  });

  it("parses instant mode headers for desktop playback calls", () => {
    expect(
      resolveInstantMode({
        requestInfo: {
          headers: { "x-pokecrystal-instant-mode": "0" },
        },
      })
    ).toBe(false);
    expect(
      resolveInstantMode({
        requestInfo: {
          headers: { "x-mcp-instant-mode": "true" },
        },
      })
    ).toBe(true);
  });

  it("loads sessions in interactive mode when the play header is present", async () => {
    await loadSession("session-play", {
      requestInfo: {
        headers: {
          "x-pokecrystal-session-mode": "interactive",
          "x-pokecrystal-instant-mode": "0",
        },
      },
    });

    expect(mockGetMcpSession).toHaveBeenCalledWith("session-play");
    expect(mockSetInstantMode).toHaveBeenCalledWith(false);
    expect(mockSetInteractiveMode).toHaveBeenCalledWith(true);
    expect(mockEnsureReady).toHaveBeenCalledTimes(1);
  });
});

type MockToolDefinition = {
  inputSchema: {
    safeParse: jest.Mock;
  };
  handler: jest.Mock;
};

const mockGetMcpToolDefinition = jest.fn();
const mockHasValidIdentityToken = jest.fn();
const mockVerifySessionSecret = jest.fn();
const mockIsRequestAuthorized = jest.fn();
const mockRunToolWithTelemetry = jest.fn();

const loadRoute = async () => {
  jest.resetModules();
  jest.doMock("@/app/api/[transport]/tools/registry", () => ({
    getMcpToolDefinition: mockGetMcpToolDefinition,
  }));
  jest.doMock("@/app/mcp/session-guards", () => ({
    isRequestAuthorized: mockIsRequestAuthorized,
    SESSION_ID_REGEX: /^[a-zA-Z0-9_-]{1,64}$/,
    verifySessionSecret: mockVerifySessionSecret,
  }));
  jest.doMock("@/app/api/[transport]/tools/identity", () => ({
    hasValidIdentityToken: mockHasValidIdentityToken,
  }));
  jest.doMock("@/app/api/[transport]/tools/common", () => ({
    runToolWithTelemetry: (...args: unknown[]) => mockRunToolWithTelemetry(...args),
  }));
  return await import("./route");
};

describe("mcp tools API", () => {
  const originalToken = process.env.POKECRYSTAL_MCP_TOKEN;

  beforeEach(() => {
    mockGetMcpToolDefinition.mockReset();
    mockHasValidIdentityToken.mockReset();
    mockVerifySessionSecret.mockReset();
    mockIsRequestAuthorized.mockReset();
    mockRunToolWithTelemetry.mockReset();
    mockHasValidIdentityToken.mockReturnValue(false);
    mockVerifySessionSecret.mockReturnValue({ ok: true, status: 200 });
    mockIsRequestAuthorized.mockReturnValue(true);
    mockRunToolWithTelemetry.mockImplementation(
      async (_toolName: string, parsedInput: unknown, handler: (input: unknown, extra?: unknown) => Promise<unknown>, extra?: unknown) =>
        handler(parsedInput, extra)
    );
    delete process.env.POKECRYSTAL_MCP_TOKEN;
  });

  afterAll(() => {
    if (originalToken === undefined) {
      delete process.env.POKECRYSTAL_MCP_TOKEN;
      return;
    }
    process.env.POKECRYSTAL_MCP_TOKEN = originalToken;
  });

  it("invokes the shared tool handler with parsed input and session headers", async () => {
    const definition: MockToolDefinition = {
      inputSchema: {
        safeParse: jest.fn().mockReturnValue({
          success: true,
          data: { direction: "down" },
        }),
      },
      handler: jest.fn().mockResolvedValue({
        content: [{ type: "text", text: "ok" }],
      }),
    };
    mockGetMcpToolDefinition.mockReturnValue(definition);

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/mcp/tools?session_id=test-session", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          tool: "move",
          input: { direction: "down" },
        }),
      })
    );

    expect(response.status).toBe(200);
    const payload = await response.json();
    expect(payload.ok).toBe(true);
    expect(payload.tool).toBe("move");
    expect(payload.result.content[0]).toEqual({ type: "text", text: "ok" });
    expect(definition.handler).toHaveBeenCalledWith(
      { direction: "down" },
      expect.objectContaining({
        requestInfo: expect.objectContaining({
          headers: expect.objectContaining({
            "mcp-session-id": "test-session",
            "x-mcp-session": "test-session",
          }),
        }),
      })
    );
  });

  it("accepts name/arguments aliases and body session_id", async () => {
    const definition: MockToolDefinition = {
      inputSchema: {
        safeParse: jest.fn().mockReturnValue({
          success: true,
          data: {},
        }),
      },
      handler: jest.fn().mockResolvedValue({
        content: [{ type: "text", text: "ok" }],
      }),
    };
    mockGetMcpToolDefinition.mockReturnValue(definition);

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/mcp/tools", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          name: "status",
          arguments: {},
          session_id: "body-session",
        }),
      })
    );

    expect(response.status).toBe(200);
    expect(definition.handler).toHaveBeenCalledWith(
      {},
      expect.objectContaining({
        requestInfo: expect.objectContaining({
          headers: expect.objectContaining({
            "mcp-session-id": "body-session",
          }),
        }),
      })
    );
  });

  it("accepts JSON-RPC tools/call payloads used by the downloadable skill", async () => {
    const definition: MockToolDefinition = {
      inputSchema: {
        safeParse: jest.fn().mockReturnValue({
          success: true,
          data: { agentId: "local-review", identityName: "trainer-1" },
        }),
      },
      handler: jest.fn().mockResolvedValue({
        content: [{ type: "text", text: "ok" }],
      }),
    };
    mockGetMcpToolDefinition.mockReturnValue(definition);

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/mcp/tools?session_id=test-session", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          jsonrpc: "2.0",
          id: 1,
          method: "tools/call",
          params: {
            name: "register_identity",
            arguments: { agentId: "local-review", identityName: "trainer-1" },
          },
        }),
      })
    );

    expect(response.status).toBe(200);
    const payload = await response.json();
    expect(payload.ok).toBe(true);
    expect(payload.tool).toBe("register_identity");
    expect(definition.handler).toHaveBeenCalledWith(
      { agentId: "local-review", identityName: "trainer-1" },
      expect.objectContaining({
        requestInfo: expect.objectContaining({
          headers: expect.objectContaining({
            "mcp-session-id": "test-session",
          }),
        }),
      })
    );
  });

  it("returns validation errors when tool arguments fail schema parse", async () => {
    const definition: MockToolDefinition = {
      inputSchema: {
        safeParse: jest.fn().mockReturnValue({
          success: false,
          error: {
            issues: [{ path: ["direction"], message: "Required" }],
          },
        }),
      },
      handler: jest.fn(),
    };
    mockGetMcpToolDefinition.mockReturnValue(definition);

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/mcp/tools", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          tool: "move",
          input: {},
        }),
      })
    );

    expect(response.status).toBe(400);
    const payload = await response.json();
    expect(payload.ok).toBe(false);
    expect(payload.error).toBe("Invalid tool arguments.");
    expect(definition.handler).not.toHaveBeenCalled();
  });

  it("returns 401 when static MCP token auth fails", async () => {
    process.env.POKECRYSTAL_MCP_TOKEN = "secret-token";
    mockIsRequestAuthorized.mockReturnValue(false);
    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/mcp/tools", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ tool: "status", input: {} }),
      })
    );
    expect(response.status).toBe(401);
    const payload = await response.json();
    expect(payload.ok).toBe(false);
    expect(payload.error).toBe("Unauthorized");
  });

  it("allows register_identity bootstrap when static token is not configured", async () => {
    const definition: MockToolDefinition = {
      inputSchema: {
        safeParse: jest.fn().mockReturnValue({
          success: true,
          data: { name: "Bootstrap" },
        }),
      },
      handler: jest.fn().mockResolvedValue({
        content: [{ type: "text", text: "{\"playerId\":\"p1\",\"token\":\"t1\"}" }],
      }),
    };
    mockGetMcpToolDefinition.mockReturnValue(definition);
    mockIsRequestAuthorized.mockReturnValue(true);
    mockHasValidIdentityToken.mockReturnValue(false);

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/mcp/tools?session_id=bootstrap-session", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          tool: "register_identity",
          arguments: { name: "Bootstrap" },
        }),
      })
    );

    expect(response.status).toBe(200);
    const payload = await response.json();
    expect(payload.ok).toBe(true);
    expect(payload.tool).toBe("register_identity");
    expect(mockVerifySessionSecret).not.toHaveBeenCalled();
  });

  it("rejects register_identity bootstrap when static MCP token auth fails", async () => {
    process.env.POKECRYSTAL_MCP_TOKEN = "secret-token";
    const definition: MockToolDefinition = {
      inputSchema: {
        safeParse: jest.fn().mockReturnValue({
          success: true,
          data: { name: "Bootstrap" },
        }),
      },
      handler: jest.fn().mockResolvedValue({
        content: [{ type: "text", text: "ok" }],
      }),
    };
    mockGetMcpToolDefinition.mockReturnValue(definition);
    mockIsRequestAuthorized.mockReturnValue(false);
    mockHasValidIdentityToken.mockReturnValue(false);

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/mcp/tools?session_id=bootstrap-session", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          tool: "register_identity",
          arguments: { name: "Bootstrap" },
        }),
      })
    );

    expect(response.status).toBe(401);
    const payload = await response.json();
    expect(payload.ok).toBe(false);
    expect(payload.error).toBe("Unauthorized");
    expect(mockVerifySessionSecret).not.toHaveBeenCalled();
  });

});

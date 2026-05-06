const mockIdentityTokenFromHeaders = jest.fn();
const mockParseIdentityToken = jest.fn();
const mockBuildSessionSecret = jest.fn();
const mockClaimSessionOwnership = jest.fn();
const mockGetSupabaseServiceRoleConfig = jest.fn();
const mockCreateClient = jest.fn();

const buildSupabase = () => {
  const maybeSingle = jest.fn();
  const from = jest.fn(() => ({
    select: jest.fn(() => ({
      contains: jest.fn(() => ({
        order: jest.fn(() => ({
          limit: jest.fn(() => ({
            maybeSingle,
          })),
        })),
      })),
    })),
  }));
  return {
    client: { from },
    mocks: { maybeSingle },
  };
};

const loadRoute = async () => {
  jest.resetModules();
  jest.doMock("@/app/api/[transport]/tools/identity", () => ({
    identityTokenFromHeaders: mockIdentityTokenFromHeaders,
    parseIdentityToken: mockParseIdentityToken,
  }));
  jest.doMock("@/lib/supabase/env", () => ({
    getSupabaseServiceRoleConfig: mockGetSupabaseServiceRoleConfig,
  }));
  jest.doMock("@supabase/supabase-js", () => ({
    createClient: mockCreateClient,
  }));
  jest.doMock("@/app/mcp/session-guards", () => ({
    buildSessionSecret: mockBuildSessionSecret,
    claimSessionOwnership: mockClaimSessionOwnership,
    SESSION_ID_REGEX: /^[a-zA-Z0-9_-]{1,64}$/,
  }));
  return await import("./route");
};

describe("arena session-secret API", () => {
  beforeEach(() => {
    mockIdentityTokenFromHeaders.mockReset();
    mockParseIdentityToken.mockReset();
    mockBuildSessionSecret.mockReset();
    mockClaimSessionOwnership.mockReset();
    mockGetSupabaseServiceRoleConfig.mockReset();
    mockCreateClient.mockReset();
    mockIdentityTokenFromHeaders.mockReturnValue("token");
    mockParseIdentityToken.mockReturnValue({ playerId: "player-1" });
    mockClaimSessionOwnership.mockReturnValue(true);
    mockBuildSessionSecret.mockReturnValue("secret-1");
    const { client, mocks } = buildSupabase();
    mockGetSupabaseServiceRoleConfig.mockReturnValue({
      url: "https://example.supabase.co",
      serviceRoleKey: "service-role-key",
    });
    mockCreateClient.mockReturnValue(client);
    mocks.maybeSingle.mockResolvedValue({ data: null, error: null });
  });

  it("issues session secret for valid identity and session", async () => {
    const { GET } = await loadRoute();
    const response = await GET(
      new Request("http://localhost/api/arena/session-secret?session_id=run-123")
    );
    expect(response.status).toBe(200);
    const payload = await response.json();
    expect(payload.ok).toBe(true);
    expect(payload.sessionSecret).toBe("secret-1");
    expect(mockBuildSessionSecret).toHaveBeenCalledWith("run-123", "player-1");
  });

  it("rejects missing identity tokens", async () => {
    mockIdentityTokenFromHeaders.mockReturnValue(null);
    const { GET } = await loadRoute();
    const response = await GET(
      new Request("http://localhost/api/arena/session-secret?session_id=run-123")
    );
    expect(response.status).toBe(401);
  });

  it("rejects session ids already claimed by another identity", async () => {
    mockClaimSessionOwnership.mockReturnValue(false);
    const { GET } = await loadRoute();
    const response = await GET(
      new Request("http://localhost/api/arena/session-secret?session_id=run-123")
    );
    expect(response.status).toBe(403);
    const payload = await response.json();
    expect(payload.ok).toBe(false);
    expect(payload.error).toContain("already owned");
  });

  it("rejects secret issuance when an existing run is bound to another identity", async () => {
    const { client, mocks } = buildSupabase();
    mockCreateClient.mockReturnValue(client);
    mocks.maybeSingle.mockResolvedValue({
      data: { metrics: { session_id: "run-123", owner_player_id: "player-2" } },
      error: null,
    });

    const { GET } = await loadRoute();
    const response = await GET(
      new Request("http://localhost/api/arena/session-secret?session_id=run-123")
    );

    expect(response.status).toBe(403);
    const payload = await response.json();
    expect(payload.ok).toBe(false);
    expect(payload.error).toContain("bound");
  });

  it("falls back to local ownership checks when the service role client throws", async () => {
    mockCreateClient.mockImplementation(() => {
      throw new Error("missing service role config");
    });

    const { GET } = await loadRoute();
    const response = await GET(
      new Request("http://localhost/api/arena/session-secret?session_id=run-123")
    );

    expect(response.status).toBe(200);
    const payload = await response.json();
    expect(payload.ok).toBe(true);
    expect(payload.sessionSecret).toBe("secret-1");
  });
});

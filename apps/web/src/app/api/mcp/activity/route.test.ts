const mockGetMcpActivitySummary = jest.fn();

const loadRoute = async () => {
  jest.resetModules();
  jest.doMock("@/app/mcp/session", () => ({
    getMcpActivitySummary: mockGetMcpActivitySummary,
  }));
  return await import("@/app/api/mcp/activity/route");
};

describe("mcp activity API", () => {
  beforeEach(() => {
    mockGetMcpActivitySummary.mockReset();
  });

  it("returns active api/mcp session count", async () => {
    mockGetMcpActivitySummary.mockReturnValue({
      activeSessions: 7,
      sessions: [],
    });

    const { GET } = await loadRoute();
    const response = await GET();
    expect(response.status).toBe(200);
    const payload = await response.json();
    expect(payload).toEqual({
      ok: true,
      apiSkillsMcpCount: 7,
    });
  });

  it("falls back to zero when summary fetch fails", async () => {
    mockGetMcpActivitySummary.mockImplementation(() => {
      throw new Error("boom");
    });

    const { GET } = await loadRoute();
    const response = await GET();
    expect(response.status).toBe(200);
    const payload = await response.json();
    expect(payload).toEqual({
      ok: true,
      apiSkillsMcpCount: 0,
    });
  });
});

const mockIsRequestAuthorized = jest.fn();
const mockGetMcpSession = jest.fn();

const SESSION_ID_REGEX = /^[a-zA-Z0-9_-]{1,64}$/;

const loadRoute = async () => {
  jest.resetModules();
  jest.doMock("@/app/mcp/session-guards", () => ({
    isRequestAuthorized: mockIsRequestAuthorized,
    SESSION_ID_REGEX,
  }));
  jest.doMock("@/app/mcp/session", () => ({
    getMcpSession: mockGetMcpSession,
  }));
  return await import("./route");
};

describe("arena snapshot route", () => {
  beforeEach(() => {
    mockIsRequestAuthorized.mockReset();
    mockGetMcpSession.mockReset();
    mockIsRequestAuthorized.mockReturnValue(true);
  });

  it("returns top-level map and flow_state alongside payload and text", async () => {
    mockGetMcpSession.mockReturnValue({
      ensureReady: jest.fn().mockResolvedValue(undefined),
      observePayload: jest.fn(() => ({
        map: { map: "NewBarkTown", map_id: "1:1", warps: [], hotspots: [] },
        flow_state: {
          completion_target: { id: "mt-silver", title: "Beat Mt. Silver" },
          summary: "Next goal: Zephyr Badge",
          completed_count: 1,
          total_count: 21,
          completed: [],
          available: [],
          remaining: [],
          remaining_path: [],
        },
      })),
      observeText: jest.fn(() => "OVERWORLD"),
    });

    const { GET } = await loadRoute();
    const response = await GET(new Request("http://localhost/api/arena/snapshot?session_id=test-session"));
    const payload = await response.json();

    expect(response.status).toBe(200);
    expect(payload.ok).toBe(true);
    expect(payload.text).toBe("OVERWORLD");
    expect(payload.map).toEqual(expect.objectContaining({ map: "NewBarkTown" }));
    expect(payload.flow_state).toEqual(
      expect.objectContaining({
        completion_target: { id: "mt-silver", title: "Beat Mt. Silver" },
      })
    );
  });
});

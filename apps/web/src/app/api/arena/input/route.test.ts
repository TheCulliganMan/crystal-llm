const mockGetMcpSession = jest.fn();
const mockIsRequestAuthorized = jest.fn();
const SESSION_ID_REGEX = /^[a-zA-Z0-9_-]{1,64}$/;

const loadRoute = async () => {
  jest.resetModules();
  jest.doMock("@/app/mcp/session", () => ({
    getMcpSession: mockGetMcpSession,
  }));
  jest.doMock("@/app/mcp/session-guards", () => ({
    isRequestAuthorized: mockIsRequestAuthorized,
    SESSION_ID_REGEX,
  }));
  return await import("@/app/api/arena/input/route");
};

describe("arena input API", () => {
  beforeEach(() => {
    mockGetMcpSession.mockReset();
    mockIsRequestAuthorized.mockReset();
    mockIsRequestAuthorized.mockReturnValue(true);
  });

  it("posts raw input into the MCP session and applies instant mode", async () => {
    const setInstantMode = jest.fn();
    const postInputEvent = jest.fn(async () => undefined);
    mockGetMcpSession.mockReturnValue({ setInstantMode, postInputEvent });

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/arena/input", {
        method: "POST",
        body: JSON.stringify({
          session_id: "desktop-input-session",
          key: "ArrowDown",
          direction: "down",
          is_press: true,
          instant: false,
        }),
      })
    );

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ ok: true });
    expect(mockGetMcpSession).toHaveBeenCalledWith("desktop-input-session");
    expect(setInstantMode).toHaveBeenCalledWith(false);
    expect(postInputEvent).toHaveBeenCalledWith({
      key: "ArrowDown",
      direction: "down",
      button: null,
      isPress: true,
    });
  });

  it("rejects invalid session ids", async () => {
    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/arena/input", {
        method: "POST",
        body: JSON.stringify({ session_id: "../bad", key: "ArrowDown", is_press: true }),
      })
    );

    expect(response.status).toBe(400);
    expect(mockGetMcpSession).not.toHaveBeenCalled();
  });
});

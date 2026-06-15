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
  return await import("@/app/api/arena/frame/route");
};

describe("arena frame API", () => {
  beforeEach(() => {
    mockGetMcpSession.mockReset();
    mockIsRequestAuthorized.mockReset();
    mockIsRequestAuthorized.mockReturnValue(true);
  });

  it("returns a tilemap frame payload and advances frames by default", async () => {
    const ensureReady = jest.fn(async () => undefined);
    const advanceFrames = jest.fn(async () => undefined);
    const setInstantMode = jest.fn();
    const observeTilemapImage = jest.fn(async () => ({
      data: "dGVzdA==",
      width: 160,
      height: 144,
    }));
    const getFrameCount = jest.fn(() => 42);
    const reset = jest.fn();
    mockGetMcpSession.mockReturnValue({
      ensureReady,
      advanceFrames,
      setInstantMode,
      observeTilemapImage,
      getFrameCount,
      reset,
    });

    const { GET } = await loadRoute();
    const response = await GET(
      new Request("http://localhost/api/arena/frame?session_id=test-session&scale=3")
    );
    expect(response.status).toBe(200);
    const payload = await response.json();
    expect(payload).toEqual({
      ok: true,
      image: "dGVzdA==",
      width: 160,
        height: 144,
        frame: 42,
      });
    expect(mockGetMcpSession).toHaveBeenCalledWith("test-session");
    expect(observeTilemapImage).toHaveBeenCalledWith({ scale: 3 });
    expect(advanceFrames).not.toHaveBeenCalled();
    expect(ensureReady).toHaveBeenCalled();
    expect(setInstantMode).not.toHaveBeenCalled();
  });

  it("applies the requested instant mode before advancing remote frames", async () => {
    const ensureReady = jest.fn(async () => undefined);
    const advanceFrames = jest.fn(async () => undefined);
    const setInstantMode = jest.fn();
    const observeTilemapImage = jest.fn(async () => ({
      data: "dGVzdA==",
      width: 160,
      height: 144,
    }));
    const getFrameCount = jest.fn(() => 43);
    mockGetMcpSession.mockReturnValue({
      ensureReady,
      advanceFrames,
      setInstantMode,
      observeTilemapImage,
      getFrameCount,
    });

    const { GET } = await loadRoute();
    const response = await GET(
      new Request("http://localhost/api/arena/frame?session_id=test-session&advance=1&instant=0")
    );
    expect(response.status).toBe(200);
    expect(setInstantMode).toHaveBeenCalledWith(false);
    expect(advanceFrames).toHaveBeenCalledWith(1);
    expect(ensureReady).not.toHaveBeenCalled();
  });

  it("uses ensureReady when advance=0", async () => {
    const ensureReady = jest.fn(async () => undefined);
    const advanceFrames = jest.fn(async () => undefined);
    const observeTilemapImage = jest.fn(async () => ({
      data: "dGVzdA==",
      width: 160,
      height: 144,
    }));
    const getFrameCount = jest.fn(() => 7);
    mockGetMcpSession.mockReturnValue({
      ensureReady,
      advanceFrames,
      observeTilemapImage,
      getFrameCount,
    });

    const { GET } = await loadRoute();
    const response = await GET(
      new Request("http://localhost/api/arena/frame?session_id=test-session&advance=0")
    );
    expect(response.status).toBe(200);
    expect(ensureReady).toHaveBeenCalled();
    expect(advanceFrames).not.toHaveBeenCalled();
  });

  it("returns a 500 without resetting or double-advancing after a frame error", async () => {
    const ensureReady = jest.fn(async () => undefined);
    const advanceFrames = jest.fn(async () => undefined);
    const observeTilemapImage = jest
      .fn()
      .mockRejectedValueOnce(new Error("boom"));
    const reset = jest.fn();
    mockGetMcpSession.mockReturnValue({
      ensureReady,
      advanceFrames,
      observeTilemapImage,
      getFrameCount: jest.fn(() => 12),
      reset,
    });

    const { GET } = await loadRoute();
    const response = await GET(
      new Request("http://localhost/api/arena/frame?session_id=test-session&advance=2")
    );
    expect(response.status).toBe(500);
    await expect(response.json()).resolves.toEqual({
      ok: false,
      error: "boom",
    });
    expect(reset).not.toHaveBeenCalled();
    expect(observeTilemapImage).toHaveBeenCalledTimes(1);
    expect(advanceFrames).toHaveBeenCalledTimes(1);
    expect(advanceFrames).toHaveBeenCalledWith(2);
  });

  it("rejects invalid scale values", async () => {
    const { GET } = await loadRoute();
    const response = await GET(new Request("http://localhost/api/arena/frame?scale=0"));
    expect(response.status).toBe(400);
    const payload = await response.json();
    expect(payload.ok).toBe(false);
  });

  it("returns a 500 response when authorization throws", async () => {
    mockIsRequestAuthorized.mockImplementation(() => {
      throw new Error("auth crash");
    });
    const { GET } = await loadRoute();
    const response = await GET(
      new Request("http://localhost/api/arena/frame?session_id=test-session&scale=2")
    );
    expect(response.status).toBe(500);
    const payload = await response.json();
    expect(payload.ok).toBe(false);
    expect(payload.error).toBe("auth crash");
  });
});

describe("arena frame API (integration)", () => {
  it("returns a tilemap frame payload for a real session", async () => {
    const originalOffscreen = (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas })
      .OffscreenCanvas;
    delete (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas;
    jest.resetModules();
    try {
      const { GET } = await import("@/app/api/arena/frame/route");
      const response = await GET(
        new Request(
          "http://localhost/api/arena/frame?session_id=integration-session&scale=2&advance=39"
        )
      );
      const payload = await response.json();
      if (response.status !== 200) {
        console.error("arena frame payload error", payload);
      }
      expect(response.status).toBe(200);
      expect(payload.ok).toBe(true);
      expect(typeof payload.image).toBe("string");
      expect(payload.width).toBeGreaterThan(0);
      expect(payload.height).toBeGreaterThan(0);
    } finally {
      if (originalOffscreen) {
        (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas =
          originalOffscreen;
      }
    }
  }, 30_000);
});

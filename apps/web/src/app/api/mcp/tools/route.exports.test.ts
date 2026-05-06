describe("api/mcp/tools route exports", () => {
  it("defines runtime and dynamic directly on the route file", async () => {
    const mod = await import("./route");
    expect(mod.runtime).toBe("nodejs");
    expect(mod.dynamic).toBe("force-dynamic");
    expect(typeof mod.POST).toBe("function");
  });
});

test("backend has no api.ts <-> web-backend.ts circular dependency", async () => {
  const madgeImport = await import("madge");
  const madge = (madgeImport as any).default ?? (madgeImport as any);

  const result = await madge("../../packages/core/src/backend", { fileExtensions: ["ts", "tsx"] });
  const cycles: string[][] = result.circular();

  const hasApiWebCycle = cycles.some(
    (cycle) =>
      cycle.length === 2 &&
      ((cycle[0] === "api.ts" && cycle[1] === "web-backend.ts") ||
        (cycle[0] === "web-backend.ts" && cycle[1] === "api.ts"))
  );

  expect(hasApiWebCycle).toBe(false);
});

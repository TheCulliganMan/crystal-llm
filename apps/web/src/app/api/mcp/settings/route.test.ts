import { GET } from "./route";

describe("mcp settings route", () => {
  it("returns cache-friendly headers for static settings", async () => {
    const response = await GET();
    expect(response.headers.get("cache-control")).toBe(
      "public, max-age=300, s-maxage=300, stale-while-revalidate=600"
    );
  });
});

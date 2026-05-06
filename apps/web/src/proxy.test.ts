import { config } from "./proxy";

describe("proxy matcher", () => {
  it("excludes static and downloadable assets from auth middleware", () => {
    const [savegameMatcher, pageMatcher] = config.matcher;

    expect(savegameMatcher).toBe("/api/savegame/:path*");
    expect(pageMatcher).toContain("api/");
    expect(pageMatcher).toContain("downloads/");
    expect(pageMatcher).toContain("assets/");
    expect(pageMatcher).toContain("disassembly/");
    expect(pageMatcher).toContain("gfx/");
    expect(pageMatcher).toContain("zip");
    expect(pageMatcher).toContain("mp3");
    expect(pageMatcher).toContain("webmanifest");
  });
});

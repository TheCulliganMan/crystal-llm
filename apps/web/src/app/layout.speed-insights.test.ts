import fs from "node:fs";
import path from "node:path";

describe("RootLayout build safety", () => {
  it("includes SpeedInsights in the root layout", () => {
    const layoutPath = path.join(process.cwd(), "src/app/layout.tsx");
    const source = fs.readFileSync(layoutPath, "utf8");

    expect(source).toContain('from "@vercel/speed-insights/next"');
    expect(source).toMatch(/<SpeedInsights\s*\/>/);
    expect(source).not.toContain('export const dynamic = "force-dynamic"');
    expect(source).toContain("RouteShell");
    expect(source).not.toContain("drawer-toggle");
  });
});

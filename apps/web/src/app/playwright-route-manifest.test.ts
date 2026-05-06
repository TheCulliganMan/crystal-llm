import fs from "node:fs";
import path from "node:path";

type RouteManifestEntry = {
  declaredPath: string;
  path: string;
  expectedPath: string;
  slug: string;
  readyTestId: string;
  includeInVisual: boolean;
};

const manifestPath = path.resolve(process.cwd(), "scripts", "playwright-route-manifest.json");

describe("playwright route manifest", () => {
  it("keeps slugs and requested paths unique while targeting stable route-ready markers", () => {
    const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8")) as RouteManifestEntry[];

    expect(manifest.length).toBeGreaterThan(0);
    expect(new Set(manifest.map((entry) => entry.slug)).size).toBe(manifest.length);
    expect(new Set(manifest.map((entry) => entry.path)).size).toBe(manifest.length);

    for (const entry of manifest) {
      expect(entry.declaredPath).toMatch(/^\//);
      expect(entry.path).toMatch(/^\//);
      expect(entry.expectedPath).toMatch(/^\//);
      expect(entry.readyTestId).toMatch(/^[a-z0-9-]+$/);
    }
  });

  it("keeps visual coverage focused on the deterministic non-canvas surfaces", () => {
    const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8")) as RouteManifestEntry[];
    const visualSlugs = manifest.filter((entry) => entry.includeInVisual).map((entry) => entry.slug);

    expect(visualSlugs).toEqual([
      "game-corner-arena-mcp-skill",
      "game-corner-progress-tracker",
      "arena",
      "leaderboard",
      "mcp",
      "watch",
    ]);
  });
});

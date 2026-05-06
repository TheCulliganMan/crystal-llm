import {
  MAIN_NAV_ITEMS,
  canonicalizeNavigationPath,
  resolveTopBarLabel,
} from "@/components/layout/navigation-config";

describe("navigation-config", () => {
  it("uses a single nav definition for shared routes", () => {
    expect(MAIN_NAV_ITEMS).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ label: "Play", href: "/" }),
        expect.objectContaining({ label: "Leaderboard", href: "/leaderboard" }),
      ])
    );
    expect(MAIN_NAV_ITEMS).toHaveLength(5);
  });

  it("normalizes arena and leaderboard paths to leaderboard", () => {
    expect(canonicalizeNavigationPath("/arena/live/demo")).toBe("/leaderboard");
    expect(canonicalizeNavigationPath("/leaderboard/history")).toBe("/leaderboard");
    expect(canonicalizeNavigationPath("/watch")).toBe("/watch");
  });

  it("resolves top bar labels from normalized routes", () => {
    expect(resolveTopBarLabel("/arena/live/demo")).toBe("Leaderboard");
    expect(resolveTopBarLabel("/mcp/tools")).toBe("Connect");
    expect(resolveTopBarLabel("/unknown")).toBe("Play");
  });
});

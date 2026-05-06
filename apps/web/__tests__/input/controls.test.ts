import { buttonKeys, mapKeyToDirection } from "@/input/controls";

describe("input/controls exports", () => {
  it("exposes key helpers for menu input handling", () => {
    expect(buttonKeys("a").length).toBeGreaterThan(0);
    expect(mapKeyToDirection("ArrowUp")).toBe("up");
  });
});

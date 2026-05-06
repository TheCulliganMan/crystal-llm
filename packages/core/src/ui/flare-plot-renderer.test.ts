import { FlarePlotRenderer } from "./flare-plot-renderer";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";

describe("FlarePlotRenderer", () => {
  it("records frame metrics and limits buffer size", () => {
    const renderer = new FlarePlotRenderer(3);
    renderer.recordFrame("title", 10);
    renderer.recordFrame("overworld", 20);
    renderer.recordFrame("oak_intro", 15);
    renderer.recordFrame("intro", 30);

    const metrics = renderer.getMetrics();
    expect(metrics.length).toBe(3);
    expect(metrics[0].label).toBe("overworld");
    expect(metrics[2].label).toBe("intro");
  });

  it("renders a plot without throwing", () => {
    const renderer = new FlarePlotRenderer(10);
    renderer.recordFrame("title", 16);
    renderer.recordFrame("title", 33);

    const surface = new gameEngine.Surface(100, 50);
    renderer.render(surface, 0, 0, 100, 50);
    // If it doesn't throw, we consider it a success for the basic test
    expect(true).toBe(true);
  });
});

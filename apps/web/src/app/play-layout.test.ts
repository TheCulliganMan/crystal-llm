import { computeFullscreenCanvasLayout, GAMEBOY_ASPECT_RATIO } from "./play-layout";

describe("computeFullscreenCanvasLayout", () => {
  it("keeps the game frame within bounds from mobile to giga screens", () => {
    const viewports: Array<{ width: number; height: number }> = [];

    for (let width = 240; width <= 1280; width += 80) {
      for (let height = 240; height <= 1024; height += 56) {
        viewports.push({ width, height });
      }
    }

    viewports.push(
      { width: 200, height: 320 },
      { width: 320, height: 200 },
      { width: 1024, height: 320 },
      { width: 320, height: 1024 },
      { width: 2560, height: 1440 },
      { width: 3440, height: 1440 },
      { width: 5120, height: 1440 },
      { width: 3840, height: 2160 },
      { width: 7680, height: 4320 }
    );

    expect(viewports.length).toBeGreaterThanOrEqual(50);

    for (const viewport of viewports) {
      const layout = computeFullscreenCanvasLayout({
        viewportWidth: viewport.width,
        viewportHeight: viewport.height,
      });

      const availableWidth = Math.max(1, viewport.width - layout.shellPaddingX * 2 - layout.framePadding * 2);
      const availableHeight = Math.max(1, viewport.height - layout.shellPaddingY * 2 - layout.framePadding * 2);

      expect(layout.frameWidth).toBeGreaterThan(0);
      expect(layout.frameHeight).toBeGreaterThan(0);
      expect(layout.frameWidth).toBeLessThanOrEqual(Math.floor(availableWidth));
      expect(layout.frameHeight).toBeLessThanOrEqual(Math.floor(availableHeight));

      const computedAspect = layout.frameWidth / layout.frameHeight;
      expect(Math.abs(computedAspect - GAMEBOY_ASPECT_RATIO)).toBeLessThan(0.02);
    }
  });

  it("falls back to sane defaults for invalid viewport values", () => {
    const layout = computeFullscreenCanvasLayout({
      viewportWidth: Number.NaN,
      viewportHeight: -1,
    });

    expect(layout.frameWidth).toBeGreaterThan(0);
    expect(layout.frameHeight).toBeGreaterThan(0);
    expect(layout.shellPaddingX).toBeGreaterThan(0);
    expect(layout.shellPaddingY).toBeGreaterThan(0);
  });
});

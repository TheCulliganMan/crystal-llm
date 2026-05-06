describe("game-engine headless blit", () => {
  it("blits pixels between headless surfaces", async () => {
    const originalOffscreen = (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas })
      .OffscreenCanvas;
    delete (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas;
    try {
      await jest.isolateModulesAsync(async () => {
        const { Surface } = await import("@pokecrystal/core/ui/game-engine");
        const source = new Surface(2, 2);
        source.fill([255, 0, 0, 255]);
        const dest = new Surface(2, 2);
        dest.fill([0, 0, 0, 255]);
        dest.blit(source, [0, 0]);
        const data = dest.getImageData().data as Uint8ClampedArray;
        expect(data[0]).toBe(255);
        expect(data[1]).toBe(0);
        expect(data[2]).toBe(0);
        expect(data[3]).toBe(255);
      });
    } finally {
      if (originalOffscreen) {
        (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas =
          originalOffscreen;
      }
    }
  });

  it("preserves destination pixels for fully transparent source pixels", async () => {
    const originalOffscreen = (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas })
      .OffscreenCanvas;
    delete (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas;
    try {
      await jest.isolateModulesAsync(async () => {
        const { Surface } = await import("@pokecrystal/core/ui/game-engine");
        const source = new Surface(2, 2);
        source.fill([0, 0, 0, 0]);
        const dest = new Surface(2, 2);
        dest.fill([10, 20, 30, 255]);
        dest.blit(source, [0, 0]);
        const data = dest.getImageData().data as Uint8ClampedArray;
        expect(data[0]).toBe(10);
        expect(data[1]).toBe(20);
        expect(data[2]).toBe(30);
        expect(data[3]).toBe(255);
      });
    } finally {
      if (originalOffscreen) {
        (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas =
          originalOffscreen;
      }
    }
  });

  it("blits pixels with explicit destination coordinates", async () => {
    const originalOffscreen = (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas })
      .OffscreenCanvas;
    delete (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas;
    try {
      await jest.isolateModulesAsync(async () => {
        const { Surface } = await import("@pokecrystal/core/ui/game-engine");
        const source = new Surface(1, 1);
        source.fill([0, 255, 0, 255]);
        const dest = new Surface(2, 2);
        dest.fill([0, 0, 0, 255]);
        dest.blitAt(source, 1, 1);
        const data = dest.getImageData().data as Uint8ClampedArray;
        const bottomRightPixel = ((1 * 2) + 1) * 4;
        expect(data[bottomRightPixel]).toBe(0);
        expect(data[bottomRightPixel + 1]).toBe(255);
        expect(data[bottomRightPixel + 2]).toBe(0);
        expect(data[bottomRightPixel + 3]).toBe(255);
      });
    } finally {
      if (originalOffscreen) {
        (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas =
          originalOffscreen;
      }
    }
  });

  it("restores global alpha after scoped draw operations", async () => {
    const originalOffscreen = (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas })
      .OffscreenCanvas;
    delete (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas;
    try {
      await jest.isolateModulesAsync(async () => {
        const { Surface } = await import("@pokecrystal/core/ui/game-engine");
        const overlay = new Surface(1, 1);
        overlay.fill([0, 0, 0, 255]);
        const source = new Surface(1, 1);
        source.fill([255, 0, 0, 255]);
        const dest = new Surface(1, 1);
        dest.fill([255, 255, 255, 255]);
        const ctx = dest.getContext();

        ctx.save();
        ctx.globalAlpha = 0.5;
        ctx.drawImage(overlay.getCanvasImageSource()!, 0, 0);
        ctx.restore();
        dest.blit(source, [0, 0]);

        const data = dest.getImageData().data as Uint8ClampedArray;
        expect(ctx.globalAlpha).toBe(1);
        expect(data[0]).toBe(255);
        expect(data[1]).toBe(0);
        expect(data[2]).toBe(0);
        expect(data[3]).toBe(255);
      });
    } finally {
      if (originalOffscreen) {
        (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas =
          originalOffscreen;
      }
    }
  });
});

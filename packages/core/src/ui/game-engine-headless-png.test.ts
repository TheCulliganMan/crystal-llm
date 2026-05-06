import type { Surface } from "@pokecrystal/core/ui/game-engine";

const hasVisibleColor = (data: Uint8ClampedArray): boolean => {
  for (let i = 0; i < data.length; i += 4) {
    if (data[i] !== 0 || data[i + 1] !== 0 || data[i + 2] !== 0) {
      return true;
    }
  }
  return false;
};

describe("game-engine headless PNG decoding", () => {
  const samples = [
    { label: "grayscale", parts: ["gfx", "tilesets", "johto.png"] },
    { label: "indexed", parts: ["gfx", "pokemon", "abra", "back.png"] },
    { label: "rgba", parts: ["gfx", "emotes", "bolt.png"] },
  ];

  it.each(samples)("decodes %s PNGs into pixels", async ({ parts }) => {
    const originalOffscreen = (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas })
      .OffscreenCanvas;
    delete (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas;
    try {
      await jest.isolateModulesAsync(async () => {
        const { getAssetPath } = await import("@pokecrystal/core/core/paths");
        const { gameEngine } = await import("@pokecrystal/core/ui/game-engine");
        const assetPath = getAssetPath(...parts);
        const surface = (await gameEngine.image.load(assetPath)) as Surface;
        const data = surface.getImageData().data as Uint8ClampedArray;
        expect(hasVisibleColor(data)).toBe(true);
      });
    } finally {
      if (originalOffscreen) {
        (globalThis as { OffscreenCanvas?: typeof OffscreenCanvas }).OffscreenCanvas =
          originalOffscreen;
      }
    }
  });
});

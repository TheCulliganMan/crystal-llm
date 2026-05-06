import fs from "fs";
import { Game } from "./game";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { listAssetFilesBySuffixes } from "@pokecrystal/core/core/asset-manifest";
import { prefetchFiles } from "@/shims/fs-browser";

jest.mock("@pokecrystal/core/core/asset-manifest", () => ({
  listAssetFilesBySuffixes: jest.fn(),
}));

jest.mock("@/shims/fs-browser", () => ({
  prefetchFiles: jest.fn(async () => undefined),
}));

describe("Game preload background warmup", () => {
  const originalWindow = (globalThis as { window?: unknown }).window;
  const originalPrefetchMode = process.env.NEXT_PUBLIC_CORE_DATA_PREFETCH_MODE;

  afterEach(() => {
    jest.restoreAllMocks();
    (globalThis as { window?: unknown }).window = originalWindow;
    if (originalPrefetchMode === undefined) {
      delete process.env.NEXT_PUBLIC_CORE_DATA_PREFETCH_MODE;
    } else {
      process.env.NEXT_PUBLIC_CORE_DATA_PREFETCH_MODE = originalPrefetchMode;
    }
  });

  it("does not block preload completion on deferred binary prefetch", async () => {
    process.env.NEXT_PUBLIC_CORE_DATA_PREFETCH_MODE = "deferred";
    (globalThis as { window?: unknown }).window = {} as unknown;

    const preloadMock = jest.spyOn(gameEngine.image, "preload").mockResolvedValue(undefined as never);
    jest.spyOn(fs, "readdirSync").mockImplementation((pathArg: fs.PathLike) => {
      const path = String(pathArg);
      if (path.includes("/gfx/sprites") || path.includes("/gfx/tilesets")) {
        throw new Error("directory listing unavailable");
      }
      return [] as unknown as ReturnType<typeof fs.readdirSync>;
    });

    (listAssetFilesBySuffixes as jest.MockedFunction<typeof listAssetFilesBySuffixes>).mockImplementation(
      (suffixes, options) => {
        const prefixes = options?.prefixes ?? [];
        if (suffixes.includes(".png")) {
          return ["/assets/gfx/sprites/youngster.png"];
        }
        if (prefixes.includes("/assets/gfx/battle_anims")) {
          return ["/assets/gfx/battle_anims/smoke.2bpp"];
        }
        if (prefixes.includes("/assets/data")) {
          return ["/assets/data/battle_anim_bundle.json"];
        }
        return [];
      }
    );

    let resolvePrefetch: (() => void) | null = null;
    const pendingPrefetch = new Promise<void>((resolve) => {
      resolvePrefetch = resolve;
    });
    (prefetchFiles as jest.MockedFunction<typeof prefetchFiles>).mockReturnValue(pendingPrefetch);

    const onProgress = jest.fn();
    const preloadPromise = (
      Game as unknown as {
        preload_core_assets: (
          onProgress?: (completed: number, total: number, label?: string) => void
        ) => Promise<void>;
      }
    ).preload_core_assets(onProgress);

    const completedState = await Promise.race([
      preloadPromise.then(() => "resolved" as const),
      new Promise<"timeout">((resolve) => setTimeout(() => resolve("timeout"), 50)),
    ]);

    expect(completedState).toBe("resolved");
    expect(prefetchFiles).toHaveBeenCalledWith(
      ["/assets/data/battle_anim_bundle.json", "/assets/gfx/battle_anims/smoke.2bpp"],
      expect.objectContaining({
        ignoreMissing: true,
        concurrency: 8,
      })
    );
    expect(preloadMock).toHaveBeenCalled();
    const finalProgress = onProgress.mock.calls.at(-1);
    expect(finalProgress?.[0]).toBeGreaterThan(preloadMock.mock.calls.length);
    expect(finalProgress?.[1]).toBeGreaterThan(preloadMock.mock.calls.length);

    resolvePrefetch?.();
    await pendingPrefetch;
  });
});

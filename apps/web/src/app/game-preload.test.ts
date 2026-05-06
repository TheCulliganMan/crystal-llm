import fs from "fs";
import { Game } from "./game";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { listAssetFilesBySuffixes } from "@pokecrystal/core/core/asset-manifest";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import * as assetReader from "@pokecrystal/core/core/asset-reader";
import * as dataLoader from "@pokecrystal/core/core/data-loader";
import * as storyEventCommon from "@pokecrystal/core/engine/world/story-events/common";

jest.mock("@pokecrystal/core/core/asset-manifest", () => ({
  listAssetFilesBySuffixes: jest.fn(),
  assetExists: jest.fn(() => false),
}));

jest.mock("@/shims/fs-browser", () => ({
  prefetchFiles: jest.fn(async () => undefined),
}));

describe("Game preload assets", () => {
  const originalWindow = (globalThis as { window?: unknown }).window;

  afterEach(() => {
    jest.restoreAllMocks();
    (globalThis as { window?: unknown }).window = originalWindow;
    Game.reset_preload_state_for_tests();
  });

  it("preloads sprite sheets from manifest in browser fallback mode", async () => {
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
      (suffixes) => {
        if (suffixes.includes(".png")) {
          return ["/assets/gfx/sprites/youngster.png"];
        }
        return [];
      }
    );

    await (Game as unknown as { preload_core_assets: () => Promise<void> }).preload_core_assets();

    expect(preloadMock).toHaveBeenCalledWith("/assets/gfx/sprites/youngster.png");
    for (const emote of ["bolt", "fish", "happy", "heart", "question", "sad", "shock", "sleep"]) {
      expect(preloadMock).toHaveBeenCalledWith(getAssetPath("gfx", "emotes", `${emote}.png`));
    }
    expect(preloadMock).toHaveBeenCalledWith(getAssetPath("gfx", "trainer_card", "chris_card.png"));
    expect(preloadMock).toHaveBeenCalledWith(getAssetPath("gfx", "trainer_card", "kris_card.png"));
    for (const introAsset of [
      "unowns",
      "background",
      "suicune_run",
      "pulse",
      "crystal_unowns",
      "pichu_wooper",
      "suicune_close",
      "suicune_jump",
      "suicune_back",
      "unown_back",
      "grass1",
      "grass2",
      "grass3",
      "grass4",
    ]) {
      expect(preloadMock).toHaveBeenCalledWith(getAssetPath("gfx", "intro", `${introAsset}.png`));
    }
  });

  it("keeps reporting progress through Unown puzzle warmup assets", async () => {
    (globalThis as { window?: unknown }).window = {} as unknown;

    const preloadMock = jest.spyOn(gameEngine.image, "preload").mockResolvedValue(undefined as never);
    jest.spyOn(fs, "readdirSync").mockImplementation((pathArg: fs.PathLike) => {
      const path = String(pathArg);
      if (path.includes("/gfx/sprites") || path.includes("/gfx/tilesets")) {
        throw new Error("directory listing unavailable");
      }
      return [] as unknown as ReturnType<typeof fs.readdirSync>;
    });
    jest.spyOn(fs.promises, "readFile").mockResolvedValue(Buffer.from("ok"));
    jest.spyOn(assetReader, "readJsonAsset").mockResolvedValue({} as never);
    (listAssetFilesBySuffixes as jest.MockedFunction<typeof listAssetFilesBySuffixes>).mockImplementation(
      (suffixes) => {
        if (suffixes.includes(".png")) {
          return ["/assets/gfx/sprites/youngster.png"];
        }
        return [];
      }
    );

    const onProgress = jest.fn();
    await (
      Game as unknown as {
        preload_core_assets: (
          onProgress?: (completed: number, total: number, label?: string) => void
        ) => Promise<void>;
      }
    ).preload_core_assets(onProgress);

    const finalProgress = onProgress.mock.calls.at(-1);
    expect(finalProgress?.[0]).toBe(finalProgress?.[1]);
    expect(finalProgress?.[1]).toBeGreaterThan(preloadMock.mock.calls.length);
    expect(fs.promises.readFile).toHaveBeenCalled();
    expect(assetReader.readJsonAsset).toHaveBeenCalled();
  });

  it("prefetches battle animation binaries from the browser manifest during blocking preload", async () => {
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
          return ["/assets/gfx/battle_anims/smoke.2bpp", "/assets/gfx/battle_anims/battle_anims.pal"];
        }
        if (prefixes.includes("/assets/data")) {
          return ["/assets/data/battle_anim_bundle.json"];
        }
        return [];
      }
    );
    const { prefetchFiles } = require("@/shims/fs-browser") as typeof import("@/shims/fs-browser");

    await (Game as unknown as { preload_core_assets: () => Promise<void> }).preload_core_assets();

    expect(preloadMock).toHaveBeenCalled();
    expect(prefetchFiles).toHaveBeenCalledWith(
      [
        "/assets/data/battle_anim_bundle.json",
        "/assets/gfx/battle_anims/smoke.2bpp",
        "/assets/gfx/battle_anims/battle_anims.pal",
      ],
      expect.objectContaining({
        ignoreMissing: true,
        concurrency: 8,
      })
    );
  });

  it("primes story-event runtime assets during auto preload boot", async () => {
    jest.resetModules();

    const preloadCoreData = jest.fn(async () => undefined);
    const primeStoryAssets = jest.fn(async () => undefined);

    jest.doMock("@pokecrystal/core/core/data-loader", () => ({
      DataLoader: class DataLoader {},
      preloadCoreDataAssets: preloadCoreData,
    }));
    jest.doMock("@pokecrystal/core/engine/world/story-events/common", () => ({
      primeStoryEventRuntimeAssets: primeStoryAssets,
    }));
    jest.doMock("@pokecrystal/core/core/save", () => ({
      hasSaveGame: async () => false,
      loadGame: async () => null,
      saveGame: async () => undefined,
      saveGameWithHistory: async () => undefined,
      deleteSaveGame: async () => undefined,
      SaveFileNotFoundError: class SaveFileNotFoundError extends Error {},
    }));
    jest.doMock("@pokecrystal/core/engine/world/overworld/overworld", () => ({
      OverworldEngine: class OverworldEngine {
        script_runner = null;
        async init_assets(): Promise<void> {}
      },
    }));
    jest.doMock("@pokecrystal/core/engine/world/overworld/overworld-tileset", () => ({
      OverworldTileset: class OverworldTileset {},
    }));
    jest.doMock("@pokecrystal/core/engine/systems/audio", () => ({
      AudioEngine: class AudioEngine {},
    }));
    jest.doMock("@pokecrystal/core/ui/overlays/battle-ui", () => ({
      begin_battle: jest.fn(),
      create_battle_ui: jest.fn(() => ({})),
      end_battle: jest.fn(),
      set_game_state: jest.fn(),
      set_audio_engine: jest.fn(),
    }));
    jest.doMock("@pokecrystal/core/ui/menus/menu-state", () => ({
      MenuState: class MenuState {},
    }));
    jest.doMock("@pokecrystal/core/engine/world/whiteout", () => ({
      WhiteoutManager: class WhiteoutManager {},
    }));

    let IsolatedGame: typeof import("./game").Game;
    jest.isolateModules(() => {
      ({ Game: IsolatedGame } = require("./game") as typeof import("./game"));
    });

    const prepareUi = jest
      .spyOn(IsolatedGame as unknown as { prepare_ui: (ui: unknown) => Promise<void> }, "prepare_ui")
      .mockResolvedValue(undefined);
    const preloadCoreAssets = jest
      .spyOn(IsolatedGame as unknown as { preload_core_assets: () => Promise<void> }, "preload_core_assets")
      .mockResolvedValue(undefined);
    const initSpy = jest
      .spyOn(IsolatedGame.prototype as unknown as { init: () => Promise<void> }, "init")
      .mockResolvedValue(undefined);
    const bootScreensSpy = jest
      .spyOn(
        IsolatedGame.prototype as unknown as { initializeBootScreens: () => Promise<void> },
        "initializeBootScreens",
      )
      .mockResolvedValue(undefined);

    const ui = {
      font: {
        renderText: jest.fn(),
        font_tiles: {},
      },
    } as never;
    await IsolatedGame!.create(ui, { preloadMode: "auto" });

    expect(prepareUi).toHaveBeenCalledTimes(1);
    expect(preloadCoreAssets).toHaveBeenCalledTimes(1);
    expect(preloadCoreData).toHaveBeenCalledWith("core", expect.any(Object));
    expect(primeStoryAssets).toHaveBeenCalledTimes(1);
    expect(initSpy).toHaveBeenCalledTimes(1);
    expect(bootScreensSpy).toHaveBeenCalledTimes(1);
  });
});

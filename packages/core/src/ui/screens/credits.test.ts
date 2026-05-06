/** @jest-environment jsdom */

import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { CreditsGraphics, CreditsPlayer } from "@pokecrystal/core/ui/screens/credits";
import * as creditsData from "@pokecrystal/core/ui/screens/credits-data";

describe("CreditsPlayer rendering optimizations", () => {
  it("loads inline credit string data from ASM labels", () => {
    const constants = creditsData.loadCreditConstantIndices();
    const strings = creditsData.loadCreditsStrings();
    const tiles = creditsData.loadCreditsStringTiles();

    expect(strings[constants.SATOSHI_TAJIRI]).toBe("   SATOSHI TAJIRI");
    expect(strings[constants.STAFF]).toBe(
      "      #MON\n  CRYSTAL VERSION\n       STAFF"
    );
    expect(strings[constants.PLANNING]).toBe(
      " SPECIAL PRODUCTION\n      PLANNING\n & DEVELOPMENT DEPT."
    );

    expect(tiles[constants.SATOSHI_TAJIRI]).toHaveLength(1);
    expect(tiles[constants.SATOSHI_TAJIRI][0]).toHaveLength("   SATOSHI TAJIRI".length);
    expect(tiles[constants.STAFF]).toHaveLength(3);
    expect(tiles[constants.PLANNING]).toHaveLength(3);
  });

  it("starts credits music with an explicit credits role", () => {
    const playMusic = jest.fn();
    const skipMusicFrames = jest.fn();
    const loadCreditsScriptSpy = jest
      .spyOn(creditsData, "loadCreditsScript")
      .mockReturnValue([{ kind: "music", byteLength: 1 } as any]);
    const loadCreditConstantIndicesSpy = jest
      .spyOn(creditsData, "loadCreditConstantIndices")
      .mockReturnValue({});
    const loadCreditsStringTilesSpy = jest
      .spyOn(creditsData, "loadCreditsStringTiles")
      .mockReturnValue([]);
    const player = new (CreditsPlayer as any)(
      {},
      { playMusic, skipMusicFrames },
      false,
      {
        paletteSets: [[[0, 0, 0, 0]]],
      }
    ) as {
      [key: string]: unknown;
      script: Array<{ kind: string; byteLength: number }>;
      scriptIndex: number;
      consumedBytes: number;
      finished: boolean;
      stepParse: () => void;
    };
    player.script = [{ kind: "music", byteLength: 1 }];
    player.scriptIndex = 0;
    player.consumedBytes = 0;
    player.finished = false;

    player.stepParse();

    expect(playMusic).toHaveBeenNthCalledWith(1, "MUSIC_NONE", "credits");
    expect(skipMusicFrames).toHaveBeenCalledWith(1);
    expect(playMusic).toHaveBeenNthCalledWith(2, "MUSIC_CREDITS", "credits");

    loadCreditsScriptSpy.mockRestore();
    loadCreditConstantIndicesSpy.mockRestore();
    loadCreditsStringTilesSpy.mockRestore();
  });

  it("reuses cached border frame composites for identical frame state", () => {
    const blank = new gameEngine.Surface(
      CreditsGraphics.MON_FRAME_SIZE,
      CreditsGraphics.MON_FRAME_SIZE
    );
    blank.fill([20, 20, 20, 255]);

    const topFrame = new gameEngine.Surface(
      CreditsGraphics.MON_FRAME_SIZE,
      CreditsGraphics.MON_FRAME_SIZE
    );
    topFrame.fill([50, 60, 70, 255]);

    const bottomFrame = new gameEngine.Surface(
      CreditsGraphics.MON_FRAME_SIZE,
      CreditsGraphics.MON_FRAME_SIZE
    );
    bottomFrame.fill([80, 90, 100, 255]);

    const getMonFrame = jest.fn((_: number, frameIndex: number) =>
      frameIndex === 1 ? topFrame : bottomFrame
    );
    const graphics = {
      getBlankFrame: jest.fn(() => blank),
      getMonFrame,
    } as unknown as CreditsGraphics;

    const player = Object.create(CreditsPlayer.prototype) as {
      [key: string]: unknown;
      getBorderFrame: () => InstanceType<typeof gameEngine.Surface>;
    };
    player.graphics = graphics;
    player.sceneIndex = 0;
    player.borderFrameTop = [0, 1];
    player.borderFrameBottom = [0, 2];
    player.borderFrameCompositeCache = new Map();

    const first = player.getBorderFrame();
    const second = player.getBorderFrame();

    expect(second).toBe(first);
    expect(getMonFrame).toHaveBeenCalledTimes(2);
  });

  it("scrolls only configured LY bands instead of the whole frame", () => {
    const width = CreditsPlayer.SCREEN_WIDTH_TILES * CreditsPlayer.TILE_SIZE;
    const height = CreditsPlayer.SCREEN_HEIGHT_TILES * CreditsPlayer.TILE_SIZE;
    const source = {
      get_size: () => [width, height] as [number, number],
    } as unknown as InstanceType<typeof gameEngine.Surface>;

    const blit = jest.fn();
    const scrolledSurface = {
      blit,
    } as unknown as InstanceType<typeof gameEngine.Surface>;

    const player = Object.create(CreditsPlayer.prototype) as {
      [key: string]: unknown;
      applyLineScroll: (surface: InstanceType<typeof gameEngine.Surface>) => InstanceType<typeof gameEngine.Surface>;
    };
    player.lyOverride = 2;
    player.scrolledSurface = scrolledSurface;

    const scrolled = player.applyLineScroll(source);
    expect(scrolled).toBe(scrolledSurface);
    // 1 full-frame copy + (2 bands * 8 rows * 2 wrapped blits per row)
    expect(blit).toHaveBeenCalledTimes(33);
    expect(blit).toHaveBeenNthCalledWith(1, source, [0, 0]);
  });

  it("uses GB frame timing for fallback post-credits fade", () => {
    const fadeOutMusic = jest.fn();
    const player = Object.create(CreditsPlayer.prototype) as {
      [key: string]: unknown;
      markScriptComplete: () => void;
    };
    player.audioEngine = { fadeOutMusic };
    player.scriptComplete = false;
    player.jumptableIndex = 0;
    player.timer = 7;
    player.scriptIndex = 1;
    player.script = [1, 2, 3];

    player.markScriptComplete();

    expect(fadeOutMusic).toHaveBeenCalledWith(CreditsPlayer.POST_CREDITS_FADE_FRAMES * GB_FRAME_DURATION_MS);
  });
});

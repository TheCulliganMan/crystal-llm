import { IntroSequence } from "./intro-sequence";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { HeadlessCanvas } from "@pokecrystal/core/ui/headless-canvas";

type MockAudioEngine = {
  playSound: jest.Mock;
  playMusic: jest.Mock;
  sfxChannelsOff: jest.Mock;
  fadeOutMusic: jest.Mock;
  stopMusic: jest.Mock;
  channelsOff: jest.Mock;
  channelsOn: jest.Mock;
};

type Bounds = {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
};

const createIntroHarness = () => {
  const audioEngine: MockAudioEngine = {
    playSound: jest.fn(),
    playMusic: jest.fn(),
    sfxChannelsOff: jest.fn(),
    fadeOutMusic: jest.fn(),
    stopMusic: jest.fn(),
    channelsOff: jest.fn(),
    channelsOn: jest.fn(),
  };
  return {
    sequence: new IntroSequence(audioEngine as unknown as AudioEngine),
    audioEngine,
  };
};

const createIntroSequence = () => createIntroHarness().sequence;

const renderIntroFrame = (sequence: IntroSequence): HeadlessCanvas => {
  const canvas = new HeadlessCanvas(160, 144);
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    throw new Error("Failed to create headless intro render context.");
  }
  sequence.draw(ctx as unknown as CanvasRenderingContext2D);
  return canvas;
};

const findBounds = (
  canvas: HeadlessCanvas,
  predicate: (r: number, g: number, b: number, a: number) => boolean
): Bounds | null => {
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    throw new Error("Failed to read headless intro render context.");
  }
  const image = ctx.getImageData(0, 0, canvas.width, canvas.height);
  let minX = canvas.width;
  let minY = canvas.height;
  let maxX = -1;
  let maxY = -1;
  for (let y = 0; y < canvas.height; y += 1) {
    for (let x = 0; x < canvas.width; x += 1) {
      const index = (y * canvas.width + x) * 4;
      const r = image.data[index] ?? 0;
      const g = image.data[index + 1] ?? 0;
      const b = image.data[index + 2] ?? 0;
      const a = image.data[index + 3] ?? 0;
      if (!predicate(r, g, b, a)) {
        continue;
      }
      minX = Math.min(minX, x);
      minY = Math.min(minY, y);
      maxX = Math.max(maxX, x);
      maxY = Math.max(maxY, y);
    }
  }
  if (maxX < minX || maxY < minY) {
    return null;
  }
  return { minX, minY, maxX, maxY };
};

const boundsOverlap = (a: Bounds, b: Bounds): boolean =>
  a.minX <= b.maxX &&
  a.maxX >= b.minX &&
  a.minY <= b.maxY &&
  a.maxY >= b.minY;

describe("IntroSequence scene parity fixes", () => {
  it("accepts select as an intro skip input like ASM PAD_BUTTONS", () => {
    const sequence = createIntroSequence();

    expect(
      sequence.handleInput({
        type: "keydown",
        key: "Backspace",
      })
    ).toBe(true);
  });

  it("stops intro music with MUSIC_NONE when skipping", () => {
    const { sequence, audioEngine } = createIntroHarness();

    expect(
      sequence.handleInput({
        type: "keydown",
        key: "Backspace",
      })
    ).toBe(true);
    expect(audioEngine.playMusic).toHaveBeenCalledWith("MUSIC_NONE", "intro");
  });

  it("scene 1 clears sprites before setup", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      sprites: unknown[];
      spawnSprite: (objectName: string, x: number, y: number) => unknown;
      introScene1: () => boolean;
    };

    state.spawnSprite("SPRITE_ANIM_OBJ_INTRO_WOOPER", 1, 2);
    expect(state.sprites).toHaveLength(1);
    expect(state.introScene1()).toBe(true);
    expect(state.sprites).toHaveLength(0);
  });

  it("clearTilemap truly clears transparent layers instead of leaving stale pixels behind", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      bgMap1: {
        set_at: (pos: [number, number], color: [number, number, number, number]) => void;
        get_at: (pos: [number, number]) => [number, number, number, number];
      };
      clearTilemap: () => void;
    };

    state.bgMap1.set_at([9 * 8, 12 * 8], [12, 200, 34, 255]);
    expect(state.bgMap1.get_at([9 * 8, 12 * 8])[3]).toBe(255);

    state.clearTilemap();

    expect(state.bgMap1.get_at([9 * 8, 12 * 8])[3]).toBe(0);
  });

  it("scene 4 uses current frame counter for end check", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      sceneFrameCounter: number;
      perspectiveScrollBg: () => void;
      introScene4: () => boolean;
    };
    const perspectiveSpy = jest.fn();
    state.perspectiveScrollBg = perspectiveSpy;

    state.sceneFrameCounter = 0x7f;
    expect(state.introScene4()).toBe(false);

    state.sceneFrameCounter = 0x80;
    expect(state.introScene4()).toBe(true);
    expect(state.perspectiveScrollBg).toHaveBeenCalledTimes(2);
  });

  it("scene 6 keeps the second pulse in place and retargets the third pulse to the lower-left unown glyph", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      sceneFrameCounter: number;
      sprites: Array<{ x: number; y: number }>;
      introScene6: () => boolean;
      clearSprites: () => void;
    };
    state.clearSprites();
    state.sceneFrameCounter = 0x20;
    expect(state.introScene6()).toBe(false);
    const firstGroup = state.sprites[0];
    expect(firstGroup).toMatchObject({ x: 15 * 8, y: 7 * 8 });

    state.clearSprites();
    state.sceneFrameCounter = 0x60;
    expect(state.introScene6()).toBe(false);
    const secondGroup = state.sprites[0];
    expect(secondGroup).toMatchObject({ x: 5 * 8, y: 14 * 8 });
    const thirdPulseFrame = renderIntroFrame(sequence);
    const pulseBounds = findBounds(
      thirdPulseFrame,
      (r, g, b, a) => a > 0 && r >= 0xf8 && g === 0 && b >= 0xf8
    );
    expect(pulseBounds).not.toBeNull();
    const targetBounds: Bounds = {
      minX: 3 * 8,
      minY: 11 * 8,
      maxX: 7 * 8 - 1,
      maxY: 14 * 8 - 1,
    };
    expect(boundsOverlap(pulseBounds as Bounds, targetBounds as Bounds)).toBe(true);
  });

  it("scene 7 and scene 13 spawn sprites with swapped coordinates fixed", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      sprites: Array<{ x: number; y: number; object_name: string }>;
      introScene7: () => boolean;
      clearSprites: () => void;
      introScene13: () => boolean;
    };

    state.introScene7();
    expect(state.sprites[0]).toMatchObject({ x: 27 * 8, y: 13 * 8 + 4 });

    state.clearSprites();
    state.introScene13();
    expect((sequence as unknown as { audioEngine: MockAudioEngine }).audioEngine.playMusic)
      .toHaveBeenCalledWith("MUSIC_CRYSTAL_OPENING", "intro");
    const suicune = state.sprites[0];
    expect(suicune).toMatchObject({
      x: 13 * 8 + 4,
      y: 11 * 8,
      object_name: "SPRITE_ANIM_OBJ_INTRO_SUICUNE",
    });
  });

  it("scene 13 restores the ASM scroll origin before Suicune runs through the forest", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      hSCX: number;
      hSCY: number;
      introScene13: () => boolean;
    };

    state.hSCX = 0x35;
    state.hSCY = 0x4a;

    expect(state.introScene13()).toBe(true);
    expect(state.hSCX).toBe(0);
    expect(state.hSCY).toBe(0);
  });

  it("intro unown pulse uses exact ASM frame durations", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      sprites: Array<{ current_oam_set: string | null; frameset_name: string | null }>;
      spawnSprite: (objectName: string, x: number, y: number) => {
        frameset_name: string | null;
        current_oam_set: string | null;
      };
      updateSpriteAnimations: () => void;
    };

    const sprite = state.spawnSprite("SPRITE_ANIM_OBJ_INTRO_UNOWN", 0, 0);
    sprite.frameset_name = "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_1";

    const seen: string[] = [];
    for (let i = 0; i < 13; i += 1) {
      state.updateSpriteAnimations();
      seen.push(state.sprites[0]?.current_oam_set ?? "deleted");
    }

    expect(seen).toEqual([
      "SPRITE_ANIM_OAMSET_INTRO_UNOWN_1",
      "SPRITE_ANIM_OAMSET_INTRO_UNOWN_1",
      "SPRITE_ANIM_OAMSET_INTRO_UNOWN_1",
      "SPRITE_ANIM_OAMSET_INTRO_UNOWN_2",
      "SPRITE_ANIM_OAMSET_INTRO_UNOWN_2",
      "SPRITE_ANIM_OAMSET_INTRO_UNOWN_2",
      "SPRITE_ANIM_OAMSET_INTRO_UNOWN_3",
      "SPRITE_ANIM_OAMSET_INTRO_UNOWN_3",
      "SPRITE_ANIM_OAMSET_INTRO_UNOWN_3",
      "SPRITE_ANIM_OAMSET_INTRO_UNOWN_3",
      "SPRITE_ANIM_OAMSET_INTRO_UNOWN_3",
      "SPRITE_ANIM_OAMSET_INTRO_UNOWN_3",
      "SPRITE_ANIM_OAMSET_INTRO_UNOWN_3",
    ]);

    state.updateSpriteAnimations();
    expect(state.sprites).toHaveLength(0);
  });

  it("intro unown motion honors VAR1 amplitude without fallback", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      spawnSprite: (objectName: string, x: number, y: number) => {
        anim_function: string | null;
        var1: number;
        jumptable_index: number;
        x_offset: number;
        y_offset: number;
      };
      applySpriteAnimFunctions: () => void;
    };

    const sprite = state.spawnSprite("SPRITE_ANIM_OBJ_INTRO_UNOWN", 0, 0);
    sprite.anim_function = "SPRITE_ANIM_FUNC_INTRO_UNOWN";
    sprite.var1 = 0;
    sprite.jumptable_index = 0;
    sprite.x_offset = 99;
    sprite.y_offset = 99;

    state.applySpriteAnimFunctions();

    expect(sprite.x_offset).toBe(0);
    expect(sprite.y_offset).toBe(0);
    expect(sprite.jumptable_index).toBe(3);
  });

  it("spawns intro unown pulses with ASM frame order and zero starting distance", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      initUnownAnim: (x: number, y: number) => void;
      sprites: Array<{
        x: number;
        y: number;
        frameset_name: string | null;
        var1: number;
        jumptable_index: number;
      }>;
    };

    state.initUnownAnim(11 * 8, 11 * 8);

    expect(
      state.sprites.map(({ x, y, frameset_name, var1, jumptable_index }) => ({
        x,
        y,
        frameset_name,
        var1,
        jumptable_index,
      }))
    ).toEqual([
      // var1 = fixed direction angle; jumptable_index = distance, starts at 0 (pulse from center)
      {
        x: 11 * 8,
        y: 11 * 8,
        frameset_name: "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_4",
        var1: 0x08,
        jumptable_index: 0x00,
      },
      {
        x: 11 * 8,
        y: 11 * 8,
        frameset_name: "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_3",
        var1: 0x18,
        jumptable_index: 0x00,
      },
      {
        x: 11 * 8,
        y: 11 * 8,
        frameset_name: "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_1",
        var1: 0x28,
        jumptable_index: 0x00,
      },
      {
        x: 11 * 8,
        y: 11 * 8,
        frameset_name: "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_2",
        var1: 0x38,
        jumptable_index: 0x00,
      },
    ]);
  });

  it("scene 9 applies palette stripes across full 32-tile rows", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      introScene3: () => boolean;
      introScene9: () => boolean;
      activeTilemaps: Map<unknown, string>;
      activeTilemapAttrmaps: Map<unknown, Uint8Array>;
      bgMap0: unknown;
      introGraphics: { attrmaps: Record<string, number[]> };
    };

    state.introScene3();
    expect(state.introScene9()).toBe(true);

    const adjusted = state.activeTilemapAttrmaps.get(state.bgMap0);
    expect(adjusted).toBeDefined();
    const row12Col25 = adjusted?.[12 * 32 + 25] ?? -1;
    const row17Col31 = adjusted?.[17 * 32 + 31] ?? -1;
    expect(row12Col25 & 0x7).toBe(2);
    expect(row17Col31 & 0x7).toBe(3);
  });

  it("scene 9 keeps the last tree scroll as the global SCX offset", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      introScene3: () => boolean;
      introScene9: () => boolean;
      treeScrollOffset: number;
      hSCX: number;
    };

    state.introScene3();
    state.treeScrollOffset = 0x13;
    expect(state.introScene9()).toBe(true);
    expect(state.hSCX).toBe(0x13);
  });

  it("scene 10 spawns Wooper and Pichu with corrected coordinate order", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      sceneFrameCounter: number;
      clearSprites: () => void;
      introScene10: () => boolean;
      sprites: Array<{ x: number; y: number; object_name: string }>;
    };
    state.clearSprites();
    state.sceneFrameCounter = 0x20;
    state.introScene10();
    expect(state.sprites).toHaveLength(1);
    expect(state.sprites[0]).toMatchObject({
      object_name: "SPRITE_ANIM_OBJ_INTRO_WOOPER",
      x: 6 * 8,
      y: 22 * 8,
    });

    state.clearSprites();
    state.sceneFrameCounter = 0x40;
    state.introScene10();
    expect(state.sprites).toHaveLength(1);
    expect(state.sprites[0]).toMatchObject({
      object_name: "SPRITE_ANIM_OBJ_INTRO_PICHU",
      x: 16 * 8,
      y: 21 * 8 + 1,
    });
  });

  it("scene 11 clears leftover scroll so the post-Pichu/Wooper unowns do not start shifted left", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      hSCX: number;
      hSCY: number;
      globalAnimXOffset: number;
      introScene11: () => boolean;
    };

    state.hSCX = 0x13;
    state.hSCY = 0x22;
    state.globalAnimXOffset = 0x40;

    expect(state.introScene11()).toBe(true);
    expect(state.hSCX).toBe(0);
    expect(state.hSCY).toBe(0);
    expect(state.globalAnimXOffset).toBe(0);
  });

  it("scene 11 unowns BG layer renders with no horizontal offset in the composed frame", () => {
    // Pixel-level proof that hSCX=0 after scene 11 means the tilemap draws at x=0.
    // The existing state test confirms hSCX is reset; this test confirms it propagates
    // correctly through drawWrappedLayer to the final composed output.
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      hSCX: number;
      bgMap0: { canvas: HeadlessCanvas };
      activeTilemaps: Map<unknown, string>;
    };

    // Simulate state as it exists after introScene11() runs: scroll cleared, bgMap0
    // registered as a wrapped tilemap ("unowns").
    state.hSCX = 0;
    state.activeTilemaps.set(state.bgMap0, "unowns");

    // Paint an opaque red pixel at tile origin (0, 0) of the bgMap0 surface.
    const bgCtx = state.bgMap0.canvas.getContext("2d");
    if (!bgCtx) {
      throw new Error("HeadlessCanvas context unavailable");
    }
    bgCtx.fillStyle = "rgba(255,0,0,255)";
    bgCtx.fillRect(0, 0, 1, 1);

    const composed = renderIntroFrame(sequence);

    // The pixel must land at screen (0, 0), not shifted right by any residual scroll.
    const bounds = findBounds(composed, (_r, _g, _b, a) => a > 0);
    expect(bounds).not.toBeNull();
    expect(bounds!.minX).toBe(0);
  });

  it("scene 15 and scene 19 spawn corrected sprites", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      sprites: Array<{ x: number; y: number; object_name: string; gfx_name?: string }>;
      introScene15: () => boolean;
      introScene19: () => boolean;
    };

    state.introScene15();
    expect(state.sprites).toHaveLength(2);
    expect(state.sprites[0]).toMatchObject({
      object_name: "SPRITE_ANIM_OBJ_INTRO_UNOWN_F",
      x: 5 * 8,
      y: 8 * 8,
    });
    expect(state.sprites[1]).toMatchObject({
      object_name: "SPRITE_ANIM_OBJ_INTRO_SUICUNE_AWAY",
      x: 0,
      y: 12 * 8,
      gfx_name: "suicune_jump",
    });

    state.sprites = [];
    state.introScene19();
    expect(state.sprites).toHaveLength(1);
    expect(state.sprites[0]).toMatchObject({
      object_name: "SPRITE_ANIM_OBJ_INTRO_SUICUNE_AWAY",
      x: 0,
      y: 12 * 8,
    });
  });

  it("scene 10 ends only on frame 0xc0", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      sceneFrameCounter: number;
      introScene10: () => boolean;
    };

    state.sceneFrameCounter = 0xbf;
    expect(state.introScene10()).toBe(false);

    state.sceneFrameCounter = 0xc0;
    expect(state.introScene10()).toBe(true);
  });

  it("scene 14 clears sprites when offset drops below 0x88", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      sceneFrameCounter: number;
      globalAnimXOffset: number;
      sprites: unknown[];
      clearSprites: () => void;
      introScene14: () => boolean;
      spawnSprite: (objectName: string, x: number, y: number) => unknown;
    };

    state.globalAnimXOffset = 0x87;
    state.sceneFrameCounter = 0x60;
    state.spawnSprite("SPRITE_ANIM_OBJ_INTRO_SUICUNE", 1, 2);
    expect(state.introScene14()).toBe(false);
    expect(state.sprites).toHaveLength(0);
  });

  it("scene 20 scrolls only before frame 0x28 and skips 0x28-0x3f", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      sceneFrameCounter: number;
      hSCY: number;
      sprites: unknown[];
      introScene20: () => boolean;
    };
    state.hSCY = 0;

    state.sceneFrameCounter = 0x26;
    state.introScene20();
    expect(state.hSCY).toBe(1);

    state.sceneFrameCounter = 0x27;
    state.introScene20();
    expect(state.hSCY).toBe(2);

    state.sceneFrameCounter = 0x28;
    state.introScene20();
    expect(state.hSCY).toBe(2);
  });

  it("scene 20 performs the colored suicune swap every active frame", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      sceneFrameCounter: number;
      introScene20: () => boolean;
      coloredSuicuneFrameSwap: () => void;
    };
    const swapSpy = jest.spyOn(state, "coloredSuicuneFrameSwap");

    state.sceneFrameCounter = 0x20;
    expect(state.introScene20()).toBe(false);
    expect(swapSpy).toHaveBeenCalledTimes(1);

    state.sceneFrameCounter = 0x50;
    expect(state.introScene20()).toBe(false);
    expect(swapSpy).toHaveBeenCalledTimes(2);
  });

  it("scene 21 performs the colored suicune swap once before advancing", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      introScene21: () => boolean;
      coloredSuicuneFrameSwap: () => void;
      sceneFrameCounter: number;
      sceneTimer: number;
    };
    const swapSpy = jest.spyOn(state, "coloredSuicuneFrameSwap");

    expect(state.introScene21()).toBe(true);
    expect(swapSpy).toHaveBeenCalledTimes(1);
    expect(state.sceneFrameCounter).toBe(0);
    expect(state.sceneTimer).toBe(0);
  });

  it("perspective scrolling advances the tree layer on even scene frames like ASM", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      sceneFrameCounter: number;
      treeScrollOffset: number;
      grassScrollOffset: number;
      bgMap0: unknown;
      activeTilemaps: Map<unknown, string>;
      getBackgroundSource: () => { canvas: CanvasImageSource };
      getSurfaceContext: (surface: unknown) => { drawImage: (...args: unknown[]) => void };
      perspectiveScrollBg: () => void;
    };
    state.getBackgroundSource = () => ({ canvas: {} as CanvasImageSource });
    state.getSurfaceContext = () => ({ drawImage: () => {} });

    state.sceneFrameCounter = 0;
    state.treeScrollOffset = 0;
    state.grassScrollOffset = 0;
    state.perspectiveScrollBg();
    expect(state.treeScrollOffset).toBe(1);
    expect(state.grassScrollOffset).toBe(2);

    state.sceneFrameCounter = 1;
    state.perspectiveScrollBg();
    expect(state.treeScrollOffset).toBe(1);
    expect(state.grassScrollOffset).toBe(4);
  });

  it("wrapped blit offsets cover the viewport when scrolled", () => {
    const computeWrappedBlitOffsets = (
      IntroSequence as unknown as {
        computeWrappedBlitOffsets: (scroll: number, surfaceSize: number, viewportSize: number) => number[];
      }
    ).computeWrappedBlitOffsets;

    expect(computeWrappedBlitOffsets(0, 256, 160)).toEqual([0]);
    expect(computeWrappedBlitOffsets(120, 256, 160)).toEqual([-120, 136]);
    expect(computeWrappedBlitOffsets(248, 256, 144)).toEqual([-248, 8]);
  });

  it("wraps any active intro tilemap surface so SCX/SCY transitions stay hardware-faithful", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      bgMap0: InstanceType<typeof gameEngine.Surface>;
      activeTilemaps: Map<InstanceType<typeof gameEngine.Surface>, string>;
      shouldWrapSurface: (surface: InstanceType<typeof gameEngine.Surface>) => boolean;
    };

    state.activeTilemaps.set(state.bgMap0, "background");
    expect(state.shouldWrapSurface(state.bgMap0)).toBe(true);

    state.activeTilemaps.set(state.bgMap0, "suicune_back");
    expect(state.shouldWrapSurface(state.bgMap0)).toBe(true);
  });

  it("clearBgPalettes applies black BG palette overrides without clearing active layers", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      bgMap0: unknown;
      bgMap1: unknown;
      activeTilemaps: Map<unknown, string>;
      introGraphics: {
        paletteOverrides: Record<string, Record<number, [number, number, number][]>>;
      };
      clearBgPalettes: () => void;
      redrawDisplayedBgLayers: () => void;
    };
    state.activeTilemaps.set(state.bgMap0, "crystal_unowns");
    state.activeTilemaps.set(state.bgMap1, "background");
    const redrawSpy = jest.spyOn(state, "redrawDisplayedBgLayers");

    state.clearBgPalettes();

    expect(state.activeTilemaps.get(state.bgMap0)).toBe("crystal_unowns");
    expect(state.activeTilemaps.get(state.bgMap1)).toBe("background");
    expect(state.introGraphics.paletteOverrides.crystal_unowns?.[0]).toEqual([
      [0, 0, 0],
      [0, 0, 0],
      [0, 0, 0],
      [0, 0, 0],
    ]);
    expect(state.introGraphics.paletteOverrides.background?.[7]).toEqual([
      [0, 0, 0],
      [0, 0, 0],
      [0, 0, 0],
      [0, 0, 0],
    ]);
    expect(redrawSpy).toHaveBeenCalledTimes(1);
  });

  it("clearBgPalettes targets the shared suicune palette bank for suicune backgrounds", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      bgMap0: unknown;
      activeTilemaps: Map<unknown, string>;
      introGraphics: {
        paletteOverrides: Record<string, Record<number, [number, number, number][]>>;
      };
      clearBgPalettes: () => void;
    };

    state.activeTilemaps.set(state.bgMap0, "suicune_back");
    state.clearBgPalettes();

    expect(state.introGraphics.paletteOverrides.suicune?.[0]).toEqual([
      [0, 0, 0],
      [0, 0, 0],
      [0, 0, 0],
      [0, 0, 0],
    ]);
    expect(state.introGraphics.paletteOverrides.suicune_back).toBeUndefined();
  });

  it("scene 24 fade only touches active BG palette overrides", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      bgMap0: unknown;
      activeTilemaps: Map<unknown, string>;
      introGraphics: {
        paletteOverrides: Record<string, Record<number, [number, number, number][]>>;
        objPaletteOverrides: Record<string, Record<number, [number, number, number][]>>;
      };
      applyScene24PaletteFade: (fadeIndex: number) => void;
      redrawDisplayedBgLayers: () => void;
    };
    state.activeTilemaps.set(state.bgMap0, "crystal_unowns");
    const redrawSpy = jest.spyOn(state, "redrawDisplayedBgLayers");

    state.applyScene24PaletteFade(0);

    expect(Object.keys(state.introGraphics.paletteOverrides)).toContain("crystal_unowns");
    expect(Object.keys(state.introGraphics.objPaletteOverrides)).toHaveLength(0);
    expect(redrawSpy).toHaveBeenCalledTimes(1);
  });

  it("scene 24 fade routes suicune backgrounds through the shared suicune palette bank", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      bgMap0: unknown;
      activeTilemaps: Map<unknown, string>;
      introGraphics: {
        paletteOverrides: Record<string, Record<number, [number, number, number][]>>;
      };
      applyScene24PaletteFade: (fadeIndex: number) => void;
    };

    state.activeTilemaps.set(state.bgMap0, "suicune_back");
    state.applyScene24PaletteFade(0);

    expect(Object.keys(state.introGraphics.paletteOverrides)).toContain("suicune");
    expect(Object.keys(state.introGraphics.paletteOverrides)).not.toContain("suicune_back");
  });

  it("renders suicune_back low tiles with the orange background atlas and high tiles with the unown bank", () => {
    // ASM Scene19 composes the orange reveal from the suicune_back tilemap plus
    // a second tile bank for Unown. In the TypeScript atlas split, low tile IDs
    // stay on the suicune_back gfx and high IDs are remapped into the unown bank.
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      bgMap0: unknown;
      introGraphics: {
        tilemaps: Record<string, Uint8Array>;
        attrmaps: Record<string, Uint8Array>;
        getTile: jest.Mock;
      };
      renderTilemap: (
        name: string,
        surface: unknown,
        store: boolean,
        tilemapOverride?: Uint8Array | null,
        attrmapOverride?: Uint8Array | null,
        tileCount?: number
      ) => void;
    };
    state.introGraphics.tilemaps.suicune_back = new Uint8Array(20 * 18);
    state.introGraphics.attrmaps.suicune_back = new Uint8Array(20 * 18);
    state.introGraphics.getTile = jest.fn(() => {
      const tile = new gameEngine.Surface(8, 8);
      return tile;
    }) as unknown as jest.Mock;

    // Low tile (0x01): orange suicune_back background atlas
    const lowTilemap = new Uint8Array(20 * 18);
    lowTilemap[0] = 0x01;
    const attrmap = new Uint8Array(20 * 18);
    state.renderTilemap("suicune_back", state.bgMap0, false, lowTilemap, attrmap, 1);
    expect(state.introGraphics.getTile).toHaveBeenCalledWith(
      "suicune_back",
      0x01,
      0,
      0,
      false,
      0,
      undefined,
      "offset"
    );

    // High tile (0x80): remapped into the unown bank with a 0x80 shift
    (state.introGraphics.getTile as jest.Mock).mockClear();
    const highTilemap = new Uint8Array(20 * 18);
    highTilemap[0] = 0x80;
    state.renderTilemap("suicune_back", state.bgMap0, false, highTilemap, attrmap, 1);
    expect(state.introGraphics.getTile).toHaveBeenCalledWith(
      "unowns",
      0x80,
      0,
      0,
      false,
      0x80,
      "suicune",
      "offset"
    );
  });

  it("renders suicune_close through signed tile addressing so the far-right close-up tiles stay mapped", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      bgMap0: unknown;
      introGraphics: {
        tilemaps: Record<string, Uint8Array>;
        attrmaps: Record<string, Uint8Array>;
        getTile: jest.Mock;
      };
      renderTilemap: (
        name: string,
        surface: unknown,
        store: boolean,
        tilemapOverride?: Uint8Array | null,
        attrmapOverride?: Uint8Array | null,
        tileCount?: number
      ) => void;
    };
    state.introGraphics.tilemaps.suicune_close = new Uint8Array(20 * 18);
    state.introGraphics.attrmaps.suicune_close = new Uint8Array(20 * 18);
    state.introGraphics.getTile = jest.fn(() => new gameEngine.Surface(8, 8)) as unknown as jest.Mock;

    const tilemap = new Uint8Array(20 * 18);
    tilemap[0] = 0x00;
    const attrmap = new Uint8Array(20 * 18);

    state.renderTilemap("suicune_close", state.bgMap0, false, tilemap, attrmap, 1);

    expect(state.introGraphics.getTile).toHaveBeenCalledWith(
      "suicune_close",
      0x00,
      0,
      0,
      false,
      0x80,
      undefined,
      "signed"
    );
  });

  it("scene 28 compares actions against pre-decrement frame value", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      sceneFrameCounter: number;
      nextSceneFrameCounter: number | null;
      clearBgPalettes: () => void;
      introScene28: () => boolean;
    };
    const clearSpy = jest.spyOn(state, "clearBgPalettes");
    state.sceneFrameCounter = 0x18;
    expect(state.introScene28()).toBe(false);
    expect(clearSpy).toHaveBeenCalledTimes(1);
    expect(state.nextSceneFrameCounter).toBe(0x17);

    clearSpy.mockClear();
    state.sceneFrameCounter = 0x17;
    expect(state.introScene28()).toBe(false);
    expect(clearSpy).not.toHaveBeenCalled();

    state.sceneFrameCounter = 0x00;
    expect(state.introScene28()).toBe(true);
  });

  it("inserts the ASM hold frames for clear-BG and scene handoff delays", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      getSceneDelayFrames: (index: number) => number;
    };

    expect(state.getSceneDelayFrames(0)).toBe(2);
    expect(state.getSceneDelayFrames(2)).toBe(2);
    expect(state.getSceneDelayFrames(6)).toBe(2);
    expect(state.getSceneDelayFrames(8)).toBe(6);
    expect(state.getSceneDelayFrames(18)).toBe(2);
    expect(state.getSceneDelayFrames(20)).toBe(3);
  });

  it("holds scene 9 and scene 21 for their ASM delay budgets before advancing", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      introScene3: () => boolean;
      jumptableIndex: number;
      sceneFrameCounter: number;
      sceneDelayFrames: number;
    };

    state.introScene3();
    state.jumptableIndex = 8;
    state.sceneFrameCounter = 0;
    sequence.update();
    expect(state.jumptableIndex).toBe(8);
    expect(state.sceneDelayFrames).toBe(6);
    for (let i = 0; i < 6; i += 1) {
      sequence.update();
    }
    expect(state.jumptableIndex).toBe(9);

    state.jumptableIndex = 20;
    state.sceneFrameCounter = 0;
    state.sceneDelayFrames = 0;
    sequence.update();
    expect(state.jumptableIndex).toBe(20);
    expect(state.sceneDelayFrames).toBe(3);
    for (let i = 0; i < 3; i += 1) {
      sequence.update();
    }
    expect(state.jumptableIndex).toBe(21);
  });

  it("completes the full intro sequence within the expected frame budget", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      jumptableIndex: number;
      sceneFrameCounter: number;
      perspectiveScrollBg: () => void;
    };
    state.perspectiveScrollBg = jest.fn();

    let finished = false;
    for (let frame = 0; frame < 2400; frame += 1) {
      finished = sequence.update();
      if (finished) {
        break;
      }
    }

    if (!finished) {
      throw new Error(
        `Intro stalled at scene ${state.jumptableIndex} frame ${state.sceneFrameCounter}`
      );
    }
    expect(finished).toBe(true);
  });

  it("reports intro scene status for debug and text snapshot consumers", () => {
    const sequence = createIntroSequence();
    const state = sequence as unknown as {
      jumptableIndex: number;
      sceneFrameCounter: number;
      sprites: unknown[];
      hSCX: number;
      hSCY: number;
    };
    state.jumptableIndex = 6;
    state.sceneFrameCounter = 64;
    state.sprites = [{}, {}, {}];
    state.hSCX = 12;
    state.hSCY = 34;

    const debugState = sequence.getDebugState();
    const snapshot = sequence.getTextSnapshot();

    expect(debugState).toMatchObject({
      sceneIndex: 6,
      sceneFrameCounter: 64,
      spriteCount: 3,
    });
    expect(snapshot.viewportTitle).toBe("Intro");
    expect(snapshot.viewportLines).toEqual(expect.arrayContaining(["CRYSTAL INTRO"]));
    expect(snapshot.infoLines).toEqual(
      expect.arrayContaining(["STATE: intro", "SCENE INDEX: 7/28", "SCENE FRAME: 64", "SPRITES: 3", "A/START/SELECT/B=Skip intro"])
    );
    expect(snapshot.promptLines).toBeNull();
  });
});

/** @jest-environment node */
import { HealMachineAnimator } from "./heal-machine-animation";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";

const POKECENTER_POSITIONS: Array<[number, number]> = [
  [24, 18],
  [24, 22],
  [30, 16],
  [30, 24],
  [35, 16],
  [35, 24],
  [40, 16],
  [40, 24],
];

const ELMS_LAB_POSITIONS: Array<[number, number]> = POKECENTER_POSITIONS.map(
  ([x, y]) => [x + 16, y + 32]
);

const HALL_OF_FAME_POSITIONS: Array<[number, number]> = [
  [52, 65],
  [52, 70],
  [51, 61],
  [51, 74],
  [49, 57],
  [49, 77],
];

const expectedHealMachinePalette: Array<[number, number, number]> = [
  [gbc5To8(31), gbc5To8(31), gbc5To8(31)],
  [gbc5To8(31), gbc5To8(19), gbc5To8(10)],
  [gbc5To8(31), gbc5To8(7), gbc5To8(1)],
  [gbc5To8(0), gbc5To8(0), gbc5To8(0)],
];

const mockHealMachineTiles = (animator: HealMachineAnimator): jest.SpyInstance => {
  const tiles = [new gameEngine.Surface(8, 8), new gameEngine.Surface(8, 8)];
  return jest.spyOn(animator as any, "loadHealMachineTilesForPalette").mockReturnValue(tiles);
};

const computeSpritePositions = (
  animator: HealMachineAnimator,
  animationType: number
): Array<[number, number]> => {
  const computeHealMachineSprites = (animator as any).computeHealMachineSprites.bind(animator);
  return computeHealMachineSprites(animationType, 0, 0, 0).map(
    (sprite: { x: number; y: number }) => [sprite.x, sprite.y]
  );
};

describe("HealMachineAnimator timing", () => {
  it("delays at exact 60 fps cadence when no overworld is provided", () => {
    const animator = new HealMachineAnimator();
    const delaySpy = jest.spyOn(gameEngine.time, "delay").mockImplementation(() => {});

    animator.play(null, 1, null);

    expect(delaySpy).toHaveBeenCalledTimes(110);
    for (const [ms] of delaySpy.mock.calls) {
      expect(ms).toBeCloseTo(GB_FRAME_DURATION_MS, 6);
    }

    delaySpy.mockRestore();
  });

  it("uses requestAnimationFrame pacing for async playback when available", async () => {
    const animator = new HealMachineAnimator();
    const delaySpy = jest.spyOn(gameEngine.time, "delay").mockImplementation(() => {});
    const globalScope = globalThis as typeof globalThis & {
      requestAnimationFrame?: (callback: (timestamp: number) => void) => number;
    };
    const previousRaf = globalScope.requestAnimationFrame;
    let rafCalls = 0;
    Object.defineProperty(globalScope, "requestAnimationFrame", {
      configurable: true,
      writable: true,
      value: (callback: (timestamp: number) => void): number => {
        rafCalls += 1;
        callback(rafCalls * GB_FRAME_DURATION_MS);
        return rafCalls;
      },
    });

    try {
      await animator.playAsync(null, 1, null);
      expect(delaySpy).not.toHaveBeenCalled();
      expect(rafCalls).toBe(110);
    } finally {
      delaySpy.mockRestore();
      if (previousRaf === undefined) {
        delete globalScope.requestAnimationFrame;
      } else {
        Object.defineProperty(globalScope, "requestAnimationFrame", {
          configurable: true,
          writable: true,
          value: previousRaf,
        });
      }
    }
  });

  it("rotates palettes on the flash cadence", () => {
    const animator = new HealMachineAnimator();
    const timeline = (animator as any).computeHealMachineTimeline(1, 0);
    const paletteRotationIndex = (animator as any).paletteRotationIndex.bind(animator);
    const start = timeline.flashStart;

    expect(paletteRotationIndex(start, timeline)).toBe(1);
    expect(paletteRotationIndex(start + 9, timeline)).toBe(1);
    expect(paletteRotationIndex(start + 10, timeline)).toBe(2);
    expect(paletteRotationIndex(start + 20, timeline)).toBe(3);
    expect(paletteRotationIndex(start + 30, timeline)).toBe(0);
    expect(paletteRotationIndex(start + timeline.flashDuration, timeline)).toBe(0);
  });

  it("matches ASM frame timelines for zero, one, three, and six party slots", () => {
    const animator = new HealMachineAnimator();
    const timelineFor = (slots: number, type = 0) =>
      (animator as any).computeHealMachineTimeline(slots, type);

    expect(timelineFor(0).totalFrames).toBe(0);
    expect(timelineFor(0).events).toEqual([]);
    expect(timelineFor(1).spawnFrames).toEqual([0]);
    expect(timelineFor(1).flashFrames).toEqual([30, 40, 50, 60, 70, 80, 90, 100]);
    expect(timelineFor(1).totalFrames).toBe(110);
    expect(timelineFor(3).spawnFrames).toEqual([0, 30, 60]);
    expect(timelineFor(3).totalFrames).toBe(170);
    expect(timelineFor(6).spawnFrames).toEqual([0, 30, 60, 90, 120, 150]);
    expect(timelineFor(6).flashFrames).toEqual([180, 190, 200, 210, 220, 230, 240, 250]);
    expect(timelineFor(6).totalFrames).toBe(260);
  });

  it("matches Hall of Fame ASM timing before and after WaitSFX", () => {
    const animator = new HealMachineAnimator();
    const timeline = (animator as any).computeHealMachineTimeline(6, 2);
    const events = timeline.events as Array<{ frame: number; kind: string; payload: unknown }>;

    expect(events.filter((event) => event.kind === "static_sprite")).toHaveLength(0);
    expect(events.filter((event) => event.kind === "spawn_ball").map((event) => event.frame)).toEqual([
      0, 30, 60, 90, 120, 150,
    ]);
    expect(events.find((event) => event.payload === "SFX_GAME_FREAK_LOGO_GS")).toEqual({
      frame: 180,
      kind: "sfx",
      payload: "SFX_GAME_FREAK_LOGO_GS",
    });
    expect(events.filter((event) => event.kind === "flash_toggle").map((event) => event.frame)).toEqual([
      180, 190, 200, 210, 220, 230, 240, 250,
    ]);
    expect(events.at(-1)).toEqual({
      frame: 260,
      kind: "wait_sfx",
      payload: { waitFor: "SFX_GAME_FREAK_LOGO_GS", thenPlay: "SFX_BOOT_PC" },
    });
    expect(timeline.totalFrames).toBe(261);
  });

  it("throws on invalid heal machine type values to mirror ASM jump-table assumptions", () => {
    const animator = new HealMachineAnimator();

    expect(() => animator.play("HEALMACHINE_BAD", 1, null)).toThrow(
      "Unknown heal machine animation type 'HEALMACHINE_BAD'."
    );
    expect(() => animator.play("3", 1, null)).toThrow(
      "Heal machine animation type '3' is out of range; expected 0..2."
    );
  });
  it("matches Pokecenter ASM sequence ordering (load, static balls, party balls, heal music, flashes)", () => {
    const animator = new HealMachineAnimator();
    const timeline = (animator as any).computeHealMachineTimeline(3, 0);
    const events = timeline.events as Array<{ frame: number; kind: string; payload: unknown }>;

    expect(events[0]).toEqual({ frame: 0, kind: "load_gfx", payload: "heal_machine" });
    expect(events.filter((event) => event.kind === "static_sprite").map((event) => event.frame)).toEqual([0, 0]);
    expect(events.filter((event) => event.kind === "spawn_ball").map((event) => event.frame)).toEqual([0, 30, 60]);
    expect(events.filter((event) => event.kind === "sfx").map((event) => event.frame)).toEqual([0, 30, 60]);
    expect(events.find((event) => event.kind === "music")).toEqual({
      frame: 90,
      kind: "music",
      payload: "MUSIC_HEAL",
    });
    expect(events.filter((event) => event.kind === "flash_toggle").map((event) => event.frame)).toEqual([
      90, 100, 110, 120, 130, 140, 150, 160,
    ]);
    expect(timeline.totalFrames).toBe(170);
  });
  it("draws the overworld before blitting the heal machine sprites", () => {
    const animator = new HealMachineAnimator();
    const screen = new gameEngine.Surface(160, 144);
    const sequence: string[] = [];
    const blitSpy = jest.spyOn(screen, "blit").mockImplementation(() => {
      sequence.push("blit");
    });
    const delaySpy = jest.spyOn(gameEngine.time, "delay").mockImplementation(() => {});
    const overworld = {
      ui: { screen, update: jest.fn(), font: { renderText: jest.fn() } },
      update: jest.fn(),
      draw: jest.fn(() => {
        sequence.push("draw");
      }),
      audio_engine: {
        playMusic: jest.fn(),
        playSound: jest.fn(),
      },
    };

    animator.play(null, 1, overworld as Parameters<HealMachineAnimator["play"]>[2]);

    expect(sequence).toContain("blit");
    expect(sequence[0]).toBe("draw");

    blitSpy.mockRestore();
    delaySpy.mockRestore();
  });

  it("waits for the hall of fame sfx before playing the boot pc sfx", () => {
    const animator = new HealMachineAnimator();
    const delaySpy = jest.spyOn(gameEngine.time, "delay").mockImplementation(() => {});
    const sequence: string[] = [];
    let remainingChecks = 2;
    const audioEngine = {
      playMusic: jest.fn(),
      playSound: jest.fn((name: string) => {
        sequence.push(`play:${name}`);
      }),
      isSoundPlaying: jest.fn(() => {
        sequence.push("check");
        if (remainingChecks > 0) {
          remainingChecks -= 1;
          return true;
        }
        return false;
      }),
    };
    const hallOfFame = String(HealMachineAnimator.HEAL_MACHINE_TYPE_MAP.HEALMACHINE_HALL_OF_FAME);
    const baseTimeline = (animator as any).computeHealMachineTimeline(1, 2);

    animator.play(hallOfFame, 1, { audio_engine: audioEngine });

    expect(delaySpy).toHaveBeenCalledTimes(baseTimeline.totalFrames + 2);
    expect(baseTimeline.totalFrames).toBe(111);
    expect(sequence.lastIndexOf("check")).toBeLessThan(sequence.indexOf("play:SFX_BOOT_PC"));

    delaySpy.mockRestore();
  });

  it("recolors grayscale tiles against the heal machine palette", () => {
    const animator = new HealMachineAnimator();
    class MockSurface {
      private readonly data: Uint8ClampedArray;
      constructor(public readonly width: number, public readonly height: number) {
        this.data = new Uint8ClampedArray(width * height * 4);
      }

      get_width(): number {
        return this.width;
      }

      get_height(): number {
        return this.height;
      }

      get_size(): [number, number] {
        return [this.width, this.height];
      }

      get_at([x, y]: [number, number]): [number, number, number, number] {
        const index = (y * this.width + x) * 4;
        return [
          this.data[index] ?? 0,
          this.data[index + 1] ?? 0,
          this.data[index + 2] ?? 0,
          this.data[index + 3] ?? 0,
        ];
      }

      set_at([x, y]: [number, number], color: [number, number, number, number]): void {
        const index = (y * this.width + x) * 4;
        this.data[index] = color[0];
        this.data[index + 1] = color[1];
        this.data[index + 2] = color[2];
        this.data[index + 3] = color[3];
      }
    }

    const originalSurface = gameEngine.Surface;
    gameEngine.Surface = MockSurface as unknown as typeof gameEngine.Surface;

    try {
      const tile = new (gameEngine.Surface as unknown as typeof MockSurface)(8, 8);
      tile.set_at([0, 0], [170, 170, 170, 255]);
      expect(tile.get_at([0, 0])).toEqual([170, 170, 170, 255]);

      const palette: Array<[number, number, number]> = expectedHealMachinePalette;

      const animatorClass = HealMachineAnimator as unknown as {
        healMachineTiles: MockSurface[] | null;
        healMachinePalette: Array<[number, number, number]> | null;
        healMachineTilesByPalette: Map<number, MockSurface[]>;
      };
      const previousTiles = animatorClass.healMachineTiles;
      const previousPalette = animatorClass.healMachinePalette;
      const previousCache = animatorClass.healMachineTilesByPalette;

      animatorClass.healMachineTiles = [tile];
      animatorClass.healMachinePalette = palette;
      animatorClass.healMachineTilesByPalette = new Map();

      const recolored = (animator as any).loadHealMachineTilesForPalette(0);
      const [r, g, b, a] = recolored[0].get_at([0, 0]);
      expect([r, g, b, a]).toEqual([palette[1][0], palette[1][1], palette[1][2], 255]);

      animatorClass.healMachineTiles = previousTiles;
      animatorClass.healMachinePalette = previousPalette;
      animatorClass.healMachineTilesByPalette = previousCache;
    } finally {
      gameEngine.Surface = originalSurface;
    }
  });

  it("loads the ASM heal machine OBJ palette", () => {
    const animator = new HealMachineAnimator();
    const animatorClass = HealMachineAnimator as unknown as {
      healMachinePalette: Array<[number, number, number]> | null;
    };
    const previousPalette = animatorClass.healMachinePalette;
    animatorClass.healMachinePalette = null;

    try {
      expect((animator as any).loadHealMachinePalette()).toEqual(expectedHealMachinePalette);
    } finally {
      animatorClass.healMachinePalette = previousPalette;
    }
  });

  it("rotates the OBJ palette in the same order as FlashPalettes", () => {
    const animator = new HealMachineAnimator();
    const rotatePalette = (animator as any).rotatePalette.bind(animator);

    expect(rotatePalette(expectedHealMachinePalette, 1)).toEqual([
      expectedHealMachinePalette[1],
      expectedHealMachinePalette[2],
      expectedHealMachinePalette[3],
      expectedHealMachinePalette[0],
    ]);
    expect(rotatePalette(expectedHealMachinePalette, 2)).toEqual([
      expectedHealMachinePalette[2],
      expectedHealMachinePalette[3],
      expectedHealMachinePalette[0],
      expectedHealMachinePalette[1],
    ]);
    expect(rotatePalette(expectedHealMachinePalette, 3)).toEqual([
      expectedHealMachinePalette[3],
      expectedHealMachinePalette[0],
      expectedHealMachinePalette[1],
      expectedHealMachinePalette[2],
    ]);
    expect(rotatePalette(expectedHealMachinePalette, 4)).toEqual(expectedHealMachinePalette);
  });

  it("keeps transparent OBJ palette index 0 transparent after rotation", () => {
    const animator = new HealMachineAnimator();
    const transparentTile = new gameEngine.Surface(8, 8);
    transparentTile.set_at([0, 0], [
      expectedHealMachinePalette[0][0],
      expectedHealMachinePalette[0][1],
      expectedHealMachinePalette[0][2],
      0,
    ]);
    transparentTile.set_at([1, 0], [
      expectedHealMachinePalette[1][0],
      expectedHealMachinePalette[1][1],
      expectedHealMachinePalette[1][2],
      255,
    ]);

    const animatorClass = HealMachineAnimator as unknown as {
      healMachineTiles: Array<typeof transparentTile> | null;
      healMachinePalette: Array<[number, number, number]> | null;
      healMachineTilesByPalette: Map<number, Array<typeof transparentTile>>;
    };
    const previousTiles = animatorClass.healMachineTiles;
    const previousPalette = animatorClass.healMachinePalette;
    const previousCache = animatorClass.healMachineTilesByPalette;

    animatorClass.healMachineTiles = [transparentTile];
    animatorClass.healMachinePalette = expectedHealMachinePalette;
    animatorClass.healMachineTilesByPalette = new Map();

    try {
      expect((animator as any).loadHealMachineTilesForPalette(1)[0].get_at([0, 0])[3]).toBe(0);
      expect((animator as any).loadHealMachineTilesForPalette(1)[0].get_at([1, 0])).toEqual([
        expectedHealMachinePalette[2][0],
        expectedHealMachinePalette[2][1],
        expectedHealMachinePalette[2][2],
        255,
      ]);
    } finally {
      animatorClass.healMachineTiles = previousTiles;
      animatorClass.healMachinePalette = previousPalette;
      animatorClass.healMachineTilesByPalette = previousCache;
    }
  });
});

describe("HealMachineAnimator ASM layout", () => {
  it("anchors the pokecenter heal machine at the ASM top-left position", () => {
    const animator = new HealMachineAnimator();
    const resolveOamAnchor = (animator as any).resolveOamAnchor.bind(animator);

    expect(resolveOamAnchor(0)).toEqual([24, 16]);
  });

  it("positions the first pokeball sprite at the ASM coordinates", () => {
    const animator = new HealMachineAnimator();
    mockHealMachineTiles(animator);
    const sprites = computeSpritePositions(animator, 0);

    expect(sprites[2]).toEqual([30, 16]);
  });

  it("keeps the static left-side heal machine tiles rendered at the ASM coordinates", () => {
    const animator = new HealMachineAnimator();
    mockHealMachineTiles(animator);
    const sprites = computeSpritePositions(animator, 0);

    expect(sprites[0]).toEqual([24, 18]);
    expect(sprites[1]).toEqual([24, 22]);
  });

  it("matches all Pokecenter OAM sprite coordinates from .PC_ElmsLab_OAM", () => {
    const animator = new HealMachineAnimator();
    mockHealMachineTiles(animator);

    expect(computeSpritePositions(animator, 0)).toEqual(POKECENTER_POSITIONS);
  });

  it("matches all Elm's Lab coordinates by applying ASM bcpixel 2,4", () => {
    const animator = new HealMachineAnimator();
    mockHealMachineTiles(animator);

    expect(computeSpritePositions(animator, 1)).toEqual(ELMS_LAB_POSITIONS);
    expect(computeSpritePositions(animator, 1)).toEqual(
      computeSpritePositions(animator, 0).map(([x, y]) => [x + 16, y + 32])
    );
  });

  it("matches all Hall of Fame OAM sprite coordinates and has no static machine tiles", () => {
    const animator = new HealMachineAnimator();
    mockHealMachineTiles(animator);

    expect((HealMachineAnimator as any).HEAL_MACHINE_STATIC_SPRITES[2]).toBe(0);
    expect(computeSpritePositions(animator, 2)).toEqual(HALL_OF_FAME_POSITIONS);
  });

  it("renders the static machine tiles before the first spawned party ball on frame 0", () => {
    const animator = new HealMachineAnimator();
    const screen = new gameEngine.Surface(160, 144);
    const blitSpy = jest.spyOn(screen, "blit");
    const delaySpy = jest.spyOn(gameEngine.time, "delay").mockImplementation(() => {});
    const overworld = {
      ui: { screen, update: jest.fn(), font: { renderText: jest.fn() } },
      update: jest.fn(),
      draw: jest.fn(),
      audio_engine: {
        playMusic: jest.fn(),
        playSound: jest.fn(),
      },
    };

    animator.play(null, 1, overworld as Parameters<HealMachineAnimator["play"]>[2]);

    const blitPositions = blitSpy.mock.calls
      .map(([, position]) => position)
      .filter((value): value is [number, number] => Array.isArray(value) && value.length === 2);

    expect(blitPositions).toEqual(
      expect.arrayContaining([
        [24, 16],
        [24, 18],
        [30, 16],
      ])
    );

    delaySpy.mockRestore();
    blitSpy.mockRestore();
  });

  it("matches the ASM frame count for three party slots", () => {
    const animator = new HealMachineAnimator();
    const timeline = (animator as any).computeHealMachineTimeline(3, 0);

    expect(timeline.totalFrames).toBe(170);
  });
});

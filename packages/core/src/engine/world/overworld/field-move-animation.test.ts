import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import {
  bind_tile_animation_timer,
  FieldMoveAnimationController,
  FieldMoveAnimationLibrary,
  HEADBUTT_SHAKE_FRAMES,
  WhirlpoolTileAnimation,
} from "./field-move-animation";
import { NpcPaletteManager } from "./palette";

const collectOpaqueColors = (surface: { getImageData: () => { data: Uint8ClampedArray } }): Set<string> => {
  const colors = new Set<string>();
  const data = surface.getImageData().data;
  for (let i = 0; i < data.length; i += 4) {
    if (data[i + 3] === 0) {
      continue;
    }
    colors.add(`${data[i]},${data[i + 1]},${data[i + 2]}`);
  }
  return colors;
};

describe("FieldMoveAnimationLibrary headbutt animation", () => {
  it("matches the ASM headbutt frame cadence", () => {
    const library = new FieldMoveAnimationLibrary();
    const definition = library.get("headbutt", "headbutt");
    const frames = definition.frames;

    expect(frames).toHaveLength(HEADBUTT_SHAKE_FRAMES);
    const surfaces = frames.map((frame) => frame.layers[0]?.resolved_surface() ?? null);
    const unique = new Set(surfaces.filter((surface): surface is NonNullable<typeof surface> => Boolean(surface)));

    expect(unique.size).toBe(3);
    expect(surfaces[0]?.get_size()).toEqual([16, 16]);
    expect(surfaces[0]).toBe(surfaces[1]);
    expect(surfaces[1]).not.toBe(surfaces[2]);
    expect(surfaces[2]).toBe(surfaces[3]);
    expect(surfaces[5]).toBe(surfaces[0]);
  });

  it("uses PAL_OW_TREE colours for day and night headbutt frames", () => {
    const paletteManager = new NpcPaletteManager();
    const dayExpected = new Set(
      paletteManager.palette(6, "day").map(([r, g, b]) => `${r},${g},${b}`)
    );
    const nightExpected = new Set(
      paletteManager.palette(6, "nite").map(([r, g, b]) => `${r},${g},${b}`)
    );

    const dayLibrary = new FieldMoveAnimationLibrary();
    const dayFrame = dayLibrary.get("headbutt", "headbutt").frames[0];
    const daySurface = dayFrame.layers[0]!.resolved_surface();
    const dayColors = collectOpaqueColors(daySurface);
    expect(dayColors.size).toBeGreaterThan(0);
    dayColors.forEach((color) => {
      expect(dayExpected).toContain(color);
    });

    const nightLibrary = new FieldMoveAnimationLibrary() as FieldMoveAnimationLibrary & {
      set_time_of_day?: (time_of_day: string) => void;
    };
    nightLibrary.set_time_of_day?.("nite");
    const nightFrame = nightLibrary.get("headbutt", "headbutt").frames[0];
    const nightSurface = nightFrame.layers[0]!.resolved_surface();
    const nightColors = collectOpaqueColors(nightSurface);
    expect(nightColors.size).toBeGreaterThan(0);
    nightColors.forEach((color) => {
      expect(nightExpected).toContain(color);
    });
  });
});

describe("FieldMoveAnimationLibrary fly animation", () => {
  it("uses ASM fly frame counts including the exit-timer tick", () => {
    const library = new FieldMoveAnimationLibrary();

    expect(library.get("fly", "from").duration).toBe(129);
    expect(library.get("fly", "to").duration).toBe(65);
  });

  it("renders fly with the exported mon icon and first-tick leaf timing", () => {
    const library = new FieldMoveAnimationLibrary();
    const firstFrame = library.get("fly", "from").frames[0];
    const layerSizes = firstFrame.layers.map((layer) => layer.resolved_surface().get_size());

    expect(firstFrame.layers).toHaveLength(2);
    expect(layerSizes).toEqual([
      [8, 8],
      [16, 16],
    ]);
  });
});

describe("WhirlpoolTileAnimation parity guards", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("cycles through all four ASM whirlpool frames from wTileAnimationTimer", () => {
    const gameState = createInitialGameState();
    const animation = new WhirlpoolTileAnimation();
    const frames: string[] = [];

    bind_tile_animation_timer(gameState);
    for (let timer = 0; timer < 4; timer += 1) {
      gameState.wram.wTileAnimationTimer = timer;
      frames.push(Array.from(animation.current_surface().getImageData().data).join(","));
    }

    expect(new Set(frames).size).toBe(4);
    gameState.wram.wTileAnimationTimer = 4;
    expect(Array.from(animation.current_surface().getImageData().data).join(",")).toBe(frames[0]);
  });

  it("throws when whirlpool composition would rely on stub fallback tiles", () => {
    jest
      .spyOn(
        WhirlpoolTileAnimation as unknown as {
          load_animated_tiles: () => Map<number, InstanceType<typeof gameEngine.Surface>[]>;
        },
        "load_animated_tiles"
      )
      .mockReturnValue(new Map());
    jest
      .spyOn(
        WhirlpoolTileAnimation as unknown as { load_whirlpool_layout: () => number[] },
        "load_whirlpool_layout"
      )
      .mockReturnValue([5]);
    jest
      .spyOn(
        WhirlpoolTileAnimation as unknown as {
          load_tileset_tiles: (_tileset: string) => InstanceType<typeof gameEngine.Surface>[];
        },
        "load_tileset_tiles"
      )
      .mockReturnValue([new gameEngine.Surface(8, 8)]);

    expect(() => new WhirlpoolTileAnimation()).toThrow(
      "Whirlpool tileset 'johto' did not load enough base tiles for ASM-faithful composition."
    );
  });
});

describe("FieldMoveAnimationController draw", () => {
  it("draws active players in world-y order with translated screen coordinates", () => {
    const controller = new FieldMoveAnimationController(null);
    const near = { world_x: 64, world_y: 16, draw: jest.fn() };
    const far = { world_x: 48, world_y: 32, draw: jest.fn() };
    (controller as unknown as { players: Map<string, unknown> }).players = new Map([
      ["far", far],
      ["near", near],
    ]);
    const screen = new gameEngine.Surface(160, 144);

    controller.draw(screen, 8, 4, [2, 6]);

    expect(near.draw).toHaveBeenCalledWith(screen, 58, 18);
    expect(far.draw).toHaveBeenCalledWith(screen, 42, 34);
    expect(near.draw.mock.invocationCallOrder[0]).toBeLessThan(far.draw.mock.invocationCallOrder[0]);
  });
});

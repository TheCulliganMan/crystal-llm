import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { NpcSpriteCache } from "./npc-sprites";

describe("NpcSpriteCache", () => {
  let originalLoadSync: typeof gameEngine.image.loadSync | undefined;

  beforeEach(() => {
    originalLoadSync = gameEngine.image.loadSync;
  });

  afterEach(() => {
    if (originalLoadSync) {
      gameEngine.image.loadSync = originalLoadSync;
    } else {
      delete gameEngine.image.loadSync;
    }
  });

  it("uses the visible frame for every direction when static sprite sheets are padded with blank frames", () => {
    const sheet = new gameEngine.Surface(16, 96);
    sheet.fill([255, 255, 255, 255]);
    sheet.fill([0, 0, 0, 255], { x: 6, y: 6, width: 4, height: 4 });
    gameEngine.image.loadSync = jest.fn(() => sheet);

    const animations = new NpcSpriteCache().instantiate("POKE_BALL", null);

    expect(animations.down.currentFrame.get_at([7, 7])).toEqual([0, 0, 0, 255]);
    expect(animations.up.currentFrame.get_at([7, 7])).toEqual([0, 0, 0, 255]);
    expect(animations.left.currentFrame.get_at([7, 7])).toEqual([0, 0, 0, 255]);
    expect(animations.right.currentFrame.get_at([7, 7])).toEqual([0, 0, 0, 255]);
  });

  it("loads Pokemon overworld icon sprite ids from gfx/icons", () => {
    const sheet = new gameEngine.Surface(16, 32);
    sheet.fill([255, 255, 255, 255]);
    sheet.fill([0, 0, 0, 255], { x: 5, y: 5, width: 6, height: 6 });
    sheet.fill([85, 85, 85, 255], { x: 4, y: 20, width: 8, height: 6 });
    gameEngine.image.loadSync = jest.fn(() => sheet);

    const animations = new NpcSpriteCache().instantiate("ICON_MOTH", 8);

    expect(gameEngine.image.loadSync).toHaveBeenCalledWith(
      expect.stringContaining("gfx/icons/moth.png")
    );
    expect(animations.down.frames).toHaveLength(2);
    expect(animations.up.frames).toHaveLength(2);
    expect(animations.left.frames).toHaveLength(2);
    expect(animations.right.frames).toHaveLength(2);
    expect(animations.down.currentFrame.get_size()).toEqual([16, 16]);
  });

  it("composes the symmetric Big Snorlax doll as a full 32x32 overworld sprite", () => {
    const animations = new NpcSpriteCache().instantiate("BIG_SNORLAX", 1, "day");

    expect(animations.down.currentFrame.get_size()).toEqual([32, 32]);
    expect(animations.up.currentFrame.get_size()).toEqual([32, 32]);
    expect(animations.left.currentFrame.get_size()).toEqual([32, 32]);
    expect(animations.right.currentFrame.get_size()).toEqual([32, 32]);
  });

  it("applies Burned Tower legendary beast object palettes to their overworld sprites", () => {
    const cache = new NpcSpriteCache();

    const expectations: Array<[string, number, [number, number, number]]> = [
      ["RAIKOU", 11, [123, 82, 24]],
      ["ENTEI", 8, [255, 57, 8]],
      ["SUICUNE", 9, [82, 74, 255]],
    ];

    for (const [spriteId, paletteId, expectedAccent] of expectations) {
      const frame = cache.instantiate(spriteId, paletteId, "day").down.currentFrame;
      const image = frame.getImageData();
      const colors = new Set<string>();
      for (let offset = 0; offset < image.data.length; offset += 4) {
        if (image.data[offset + 3] === 0) {
          continue;
        }
        colors.add([
          image.data[offset],
          image.data[offset + 1],
          image.data[offset + 2],
        ].join(","));
      }

      expect(colors).toContain(expectedAccent.join(","));
    }
  });
});

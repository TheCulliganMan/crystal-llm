import fs from "fs";
import { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { Surface } from "@pokecrystal/core/ui/surface";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { reset_deferred_image_preloads_for_test } from "@pokecrystal/core/ui/deferred-assets";

class TestUI extends BaseUI {
  protected createScreenSurface(): Surface {
    return new Surface(this.screenWidth, this.screenHeight);
  }

  public update(): void {
    // No-op for palette tests.
  }
}

const parsePaletteFile = (raw: string): [number, number, number][] => {
  const colors: [number, number, number][] = [];
  for (const line of raw.split(/\r?\n/)) {
    const stripped = line.split(";", 1)[0].trim();
    if (!stripped.toUpperCase().startsWith("RGB")) {
      continue;
    }
    const parts = stripped.replace("RGB", "").replace(/,/g, " ").trim().split(/\s+/);
    if (parts.length !== 3) {
      throw new Error(`Malformed RGB entry '${stripped}'`);
    }
    const r = gbc5To8(Number(parts[0]), "textbox palette r");
    const g = gbc5To8(Number(parts[1]), "textbox palette g");
    const b = gbc5To8(Number(parts[2]), "textbox palette b");
    colors.push([r, g, b]);
    if (colors.length === 4) {
      break;
    }
  }
  if (colors.length !== 4) {
    throw new Error(`Textbox palette should contain 4 colors, got ${colors.length}`);
  }
  return colors;
};

describe("BaseUI textbox palette", () => {
  it("loads the palette file when running under node", () => {
    const ui = new TestUI();
    const palette = ui.get_context_palette("textbox");
    const path = getAssetPath("gfx", "stats", "party_menu_bg.pal");
    const raw = fs.readFileSync(path, "utf-8");
    const expected = parsePaletteFile(raw);
    expect(palette).toEqual(expected);
  });

  it("falls back to the baked palette in the browser when reads fail", () => {
    const globalAny = globalThis as { window?: unknown };
    const originalWindow = globalAny.window;
    Object.defineProperty(globalAny, "window", {
      value: {},
      configurable: true,
      writable: true,
    });

    const readSpy = jest
      .spyOn(fs, "readFileSync")
      .mockImplementation(() => {
        throw new Error("read failed");
      });

    const ui = new TestUI();
    const palette = ui.get_context_palette("textbox");
    const expected = [
      [31, 31, 31],
      [17, 19, 31],
      [14, 16, 31],
      [0, 0, 0],
    ].map(([r, g, b]) => [
      gbc5To8(r, "textbox palette r"),
      gbc5To8(g, "textbox palette g"),
      gbc5To8(b, "textbox palette b"),
    ]);

    expect(palette).toEqual(expected);

    readSpy.mockRestore();
    if (originalWindow === undefined) {
      delete globalAny.window;
    } else {
      Object.defineProperty(globalAny, "window", {
        value: originalWindow,
        configurable: true,
        writable: true,
      });
    }
  });
});

describe("BaseUI pokemon front frame count", () => {
  let originalLoadSync: typeof gameEngine.image.loadSync | undefined;

  beforeEach(() => {
    originalLoadSync = gameEngine.image.loadSync;
  });

  afterEach(() => {
    gameEngine.image.loadSync = originalLoadSync;
  });

  it("returns the frame count from front sprite sheets", () => {
    const ui = new TestUI();
    gameEngine.image.loadSync = jest.fn(() => new Surface(56, 112));

    const count = ui.get_pokemon_frame_count("pikachu", "pokemon_front");

    expect(count).toBe(2);
  });

  it("falls back to one frame for non-front sprite types", () => {
    const ui = new TestUI();
    const loadSpy = jest.fn(() => new Surface(56, 112));
    gameEngine.image.loadSync = loadSpy;

    const count = ui.get_pokemon_frame_count("pikachu", "pokemon_back");

    expect(count).toBe(1);
    expect(loadSpy).not.toHaveBeenCalled();
  });

  it("throws when front sprite sheets are not square-tiled", () => {
    const ui = new TestUI();
    gameEngine.image.loadSync = jest.fn(() => new Surface(56, 50));

    expect(() => ui.get_pokemon_frame_count("pikachu", "pokemon_front")).toThrow(
      "Pokemon front sprite sheet height must be a multiple of its width.",
    );
  });

  it("throws when the front sprite sheet is missing", () => {
    const ui = new TestUI();
    gameEngine.image.loadSync = jest.fn(() => null);

    expect(() => ui.get_pokemon_frame_count("missingmon", "pokemon_front")).toThrow(
      "Missing pokemon front sprite:",
    );
  });

  it("preserves opaque black pixels when source sprite already uses alpha transparency", () => {
    const ui = new TestUI();
    const sprite = new Surface(56, 56);
    sprite.fill([0, 0, 0, 0]);
    sprite.set_at([10, 10], [0, 0, 0, 255]);
    sprite.set_at([11, 10], [120, 240, 120, 255]);
    gameEngine.image.loadSync = jest.fn(() => sprite);

    const frame = ui.getPokemonFrontSurface("testmon", 0);

    expect(frame).not.toBeNull();
    expect(frame!.get_at([10, 10])).toEqual([0, 0, 0, 255]);
    expect(frame!.get_at([11, 10])).toEqual([120, 240, 120, 255]);
  });

  it("preserves enclosed white pixels when applying color-key transparency", () => {
    const ui = new TestUI();
    const sprite = new Surface(56, 56);
    sprite.fill([255, 255, 255, 255]);
    for (let x = 20; x <= 35; x += 1) {
      sprite.set_at([x, 20], [0, 0, 0, 255]);
      sprite.set_at([x, 35], [0, 0, 0, 255]);
    }
    for (let y = 20; y <= 35; y += 1) {
      sprite.set_at([20, y], [0, 0, 0, 255]);
      sprite.set_at([35, y], [0, 0, 0, 255]);
    }
    gameEngine.image.loadSync = jest.fn(() => sprite);

    const frame = ui.getPokemonFrontSurface("testmon", 0);

    expect(frame).not.toBeNull();
    expect(frame!.get_at([0, 0])[3]).toBe(0);
    expect(frame!.get_at([20, 20])).toEqual([0, 0, 0, 255]);
    expect(frame!.get_at([28, 28])).toEqual([255, 255, 255, 255]);
  });

  it("pads 5x5 frontpics like PadFrontpic with one blank row on top and two blank columns on the left", () => {
    const ui = new TestUI();
    const sprite = new Surface(40, 40);
    sprite.fill([0, 0, 0, 0]);
    sprite.set_at([0, 0], [12, 200, 34, 255]);
    gameEngine.image.loadSync = jest.fn(() => sprite);

    const frame = ui.getPokemonFrontSurface("testmon", 0);

    expect(frame).not.toBeNull();
    expect(frame!.get_at([15, 7])[3]).toBe(0);
    expect(frame!.get_at([16, 8])).toEqual([12, 200, 34, 255]);
  });

  it("pads 6x6 frontpics like PadFrontpic with no top blank row and one blank column on the left", () => {
    const ui = new TestUI();
    const sprite = new Surface(48, 48);
    sprite.fill([0, 0, 0, 0]);
    sprite.set_at([0, 0], [220, 50, 80, 255]);
    gameEngine.image.loadSync = jest.fn(() => sprite);

    const frame = ui.getPokemonFrontSurface("testmon", 0);

    expect(frame).not.toBeNull();
    expect(frame!.get_at([7, 0])[3]).toBe(0);
    expect(frame!.get_at([8, 0])).toEqual([220, 50, 80, 255]);
  });
});

describe("BaseUI sprite asset guards", () => {
  let originalLoadSync: typeof gameEngine.image.loadSync | undefined;
  let originalPreload: typeof gameEngine.image.preload | undefined;

  beforeEach(() => {
    originalLoadSync = gameEngine.image.loadSync;
    originalPreload = gameEngine.image.preload;
    reset_deferred_image_preloads_for_test();
  });

  afterEach(() => {
    gameEngine.image.loadSync = originalLoadSync;
    gameEngine.image.preload = originalPreload!;
    reset_deferred_image_preloads_for_test();
  });

  it("queues a browser preload for manifest-backed front sprites before they are synchronously available", async () => {
    const ui = new TestUI();
    const sprite = new Surface(56, 56);
    let loaded = false;
    gameEngine.image.loadSync = jest.fn(() => (loaded ? sprite : null));
    gameEngine.image.preload = jest.fn(async () => {
      loaded = true;
      return sprite;
    });

    expect(ui.getPokemonFrontSurface("cyndaquil")).toBeNull();
    expect(gameEngine.image.preload).toHaveBeenCalledWith(
      expect.stringContaining("/assets/gfx/pokemon/cyndaquil/front.png")
    );

    await Promise.resolve();
    await Promise.resolve();

    expect(ui.getPokemonFrontSurface("cyndaquil")).not.toBeNull();
  });

  it("queues a browser preload for manifest-backed back sprites before they are synchronously available", async () => {
    const ui = new TestUI();
    const sprite = new Surface(48, 48);
    let loaded = false;
    gameEngine.image.loadSync = jest.fn(() => (loaded ? sprite : null));
    gameEngine.image.preload = jest.fn(async () => {
      loaded = true;
      return sprite;
    });

    expect(ui.getPokemonBackSurface("cyndaquil")).toBeNull();
    expect(gameEngine.image.preload).toHaveBeenCalledWith(
      expect.stringContaining("/assets/gfx/pokemon/cyndaquil/back.png")
    );

    await Promise.resolve();
    await Promise.resolve();

    expect(ui.getPokemonBackSurface("cyndaquil")).not.toBeNull();
  });

  it("throws when the pokemon front sprite is missing", () => {
    const ui = new TestUI();
    gameEngine.image.loadSync = jest.fn(() => null);

    expect(() => ui.getPokemonFrontSurface("missingmon")).toThrow("Missing pokemon front sprite:");
  });

  it("surfaces a better browser error when a manifest-backed sprite is unexpectedly unavailable", () => {
    const ui = new TestUI();
    const globalAny = globalThis as { window?: unknown };
    const originalWindow = globalAny.window;
    Object.defineProperty(globalAny, "window", {
      value: {},
      configurable: true,
      writable: true,
    });
    gameEngine.image.loadSync = jest.fn(() => null);
    gameEngine.image.preload = undefined as typeof gameEngine.image.preload;

    expect(() => ui.getPokemonFrontSurface("totodile")).toThrow(
      "restart `npm run dev` and hard-refresh the page",
    );

    if (originalWindow === undefined) {
      delete globalAny.window;
    } else {
      Object.defineProperty(globalAny, "window", {
        value: originalWindow,
        configurable: true,
        writable: true,
      });
    }
  });

  it("surfaces a better browser error when a manifest-backed back sprite is unexpectedly unavailable", () => {
    const ui = new TestUI();
    const globalAny = globalThis as { window?: unknown };
    const originalWindow = globalAny.window;
    Object.defineProperty(globalAny, "window", {
      value: {},
      configurable: true,
      writable: true,
    });
    gameEngine.image.loadSync = jest.fn(() => null);
    gameEngine.image.preload = undefined as typeof gameEngine.image.preload;

    expect(() => ui.getPokemonBackSurface("totodile")).toThrow(
      "restart `npm run dev` and hard-refresh the page",
    );

    if (originalWindow === undefined) {
      delete globalAny.window;
    } else {
      Object.defineProperty(globalAny, "window", {
        value: originalWindow,
        configurable: true,
        writable: true,
      });
    }
  });

  it("throws when the pokemon back sprite is missing", () => {
    const ui = new TestUI();
    gameEngine.image.loadSync = jest.fn(() => null);

    expect(() => ui.getPokemonBackSurface("missingmon")).toThrow("Missing pokemon back sprite:");
  });

  it("throws when a trainer sprite cannot be loaded", () => {
    const ui = new TestUI();
    gameEngine.image.loadSync = jest.fn(() => null);

    expect(() => ui.get_sprite_surface("missingtrainer", "trainer")).toThrow(
      "Sprite asset not found for",
    );
  });

  it("queues a browser preload for manifest-backed trainer sprites before they are synchronously available", async () => {
    const ui = new TestUI();
    const sprite = new Surface(56, 56);
    let loaded = false;
    gameEngine.image.loadSync = jest.fn(() => (loaded ? sprite : null));
    gameEngine.image.preload = jest.fn(async () => {
      loaded = true;
      return sprite;
    });

    expect(ui.get_sprite_surface("falkner", "trainer")).toBeNull();
    expect(gameEngine.image.preload).toHaveBeenCalledWith(
      expect.stringContaining("/assets/gfx/trainers/falkner.png")
    );

    await Promise.resolve();
    await Promise.resolve();

    expect(ui.get_sprite_surface("falkner", "trainer")).not.toBeNull();
  });

  it("queues a browser preload for manifest-backed player back sprites before they are synchronously available", async () => {
    const ui = new TestUI();
    const sprite = new Surface(48, 48);
    let loaded = false;
    gameEngine.image.loadSync = jest.fn(() => (loaded ? sprite : null));
    gameEngine.image.preload = jest.fn(async () => {
      loaded = true;
      return sprite;
    });

    expect(ui.get_sprite_surface("chris_back", "player_back")).toBeNull();
    expect(gameEngine.image.preload).toHaveBeenCalledWith(
      expect.stringContaining("/assets/gfx/player/chris_back.png")
    );

    await Promise.resolve();
    await Promise.resolve();

    expect(ui.get_sprite_surface("chris_back", "player_back")).not.toBeNull();
  });
});

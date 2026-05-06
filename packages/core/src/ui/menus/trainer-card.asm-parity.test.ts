import fs from "fs";
import type { GameState } from "@pokecrystal/core/core/state";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { Surface } from "@pokecrystal/core/ui/surface";
import { SPACE_TILE, _CHAR_MAP } from "@pokecrystal/core/ui/tilemap-surface";
import { BitmapFont } from "@pokecrystal/core/ui/text/bitmap-font";
import { PNG } from "pngjs";
import { TrainerCardScreen } from "./trainer-card";
import {
  clockColonCoords,
  pokedexValueOrigin,
  smallColonTile,
  trainerCardPortraitTilePixels,
} from "./trainer-card-layout";

type BGMapWrite = (
  name: string,
  width: number,
  height: number,
  tiles: number[],
  attrs: number[],
  options?: { origin_x?: number; origin_y?: number }
) => void;

type ExtendedGameState = GameState & {
  bg_map_sync: { is_busy: boolean };
  write_bg_map_with_wait: BGMapWrite;
};

type LastBGWrite = {
  width: number;
  height: number;
  tiles: number[];
};

const TILE_TO_CHAR = new Map<number, string>();
Object.entries(_CHAR_MAP).forEach(([char, tile]) => {
  if (!TILE_TO_CHAR.has(tile)) {
    TILE_TO_CHAR.set(tile, char);
  }
});

const rowText = (tiles: number[]): string => {
  return tiles
    .map((tile) => TILE_TO_CHAR.get(tile) ?? `\\x${tile.toString(16).padStart(2, "0")}`)
    .join("");
};

const keyDown = (code: string) => ({ type: "keydown", code });

const decodePngPortraitTiles = (stem: string): number[][] => {
  const png = PNG.sync.read(fs.readFileSync(getAssetPath("gfx", "trainer_card", `${stem}.png`)));
  const tiles: number[][] = [];
  for (let top = 0; top < png.height; top += 8) {
    for (let left = 0; left < png.width; left += 8) {
      const pixels: number[] = [];
      for (let row = 0; row < 8; row += 1) {
        for (let col = 0; col < 8; col += 1) {
          const offset = ((top + row) * png.width + (left + col)) * 4;
          const luminance = png.data[offset] ?? 255;
          pixels.push(Math.max(0, Math.min(3, Math.round((255 - luminance) / 85))));
        }
      }
      tiles.push(pixels);
    }
  }
  return tiles;
};

let fontProxy: {
  paletteVariants: BitmapFont["paletteVariants"];
  fontTiles: BitmapFont["fontTiles"];
  font_tiles: BitmapFont["fontTiles"];
};

const buildHarness = () => {
  const gameState = createInitialGameState();
  let lastWrite: LastBGWrite | null = null;
  const writeBG = jest.fn(
    (_name: string, width: number, height: number, tiles: number[], _attrs: number[]) => {
      lastWrite = {
        width,
        height,
        tiles: [...tiles],
      };
    }
  );
  const extended = gameState as ExtendedGameState;
  extended.bg_map_sync = { is_busy: false };
  extended.write_bg_map_with_wait = writeBG;
  gameState.sram.player_name = "KRIS";
  const ui = {
    screen: new Surface(160, 144),
    font: fontProxy as never,
  };
  const screen = new TrainerCardScreen(ui, gameState);

  const tileAt = (x: number, y: number): number => {
    if (!lastWrite) {
      throw new Error("Trainer card draw did not produce a BG map write.");
    }
    if (x < 0 || y < 0 || x >= lastWrite.width || y >= lastWrite.height) {
      throw new Error(`Tile lookup (${x},${y}) outside ${lastWrite.width}x${lastWrite.height}`);
    }
    return lastWrite.tiles[y * lastWrite.width + x];
  };

  return {
    gameState,
    screen,
    setFrame(frame: number) {
      gameState.frame_counter = frame;
    },
    tileAt,
  };
};

describe("trainer card ASM parity", () => {
  let debugSpy: jest.SpyInstance;

  beforeAll(async () => {
    const font = new BitmapFont();
    await font.load();
    fontProxy = {
      paletteVariants: font.paletteVariants.bind(font),
      fontTiles: font.fontTiles,
      font_tiles: font.fontTiles,
    };
    debugSpy = jest.spyOn(console, "debug").mockImplementation(() => undefined);
  });

  afterAll(() => {
    debugSpy.mockRestore();
  });

  it("formats money like PrintNum with PRINTNUM_MONEY (right-aligned, no zero padding)", () => {
    const { gameState, screen, setFrame, tileAt } = buildHarness();
    gameState.sram.money = 3_000;

    setFrame(1);
    screen.draw();

    const moneyTiles = Array.from({ length: 7 }, (_unused, index) => tileAt(7 + index, 6));
    expect(rowText(moneyTiles)).toBe("  \u00a53000");
  });

  it("toggles the play-time colon only on absolute VBlank % 32 frames", () => {
    const { screen, setFrame, tileAt } = buildHarness();
    const [colonX, colonY] = clockColonCoords();

    setFrame(0);
    screen.draw();
    expect(tileAt(colonX, colonY)).toBe(SPACE_TILE);

    setFrame(1);
    screen.draw();
    expect(tileAt(colonX, colonY)).toBe(SPACE_TILE);

    setFrame(32);
    screen.draw();
    expect(tileAt(colonX, colonY)).toBe(smallColonTile());

    setFrame(64);
    screen.draw();
    expect(tileAt(colonX, colonY)).toBe(SPACE_TILE);
  });

  it("advances badge animation only on absolute VBlank % 8 frames", () => {
    const { screen, setFrame } = buildHarness();
    screen.handleInput(keyDown("ArrowRight"));

    setFrame(0);
    screen.draw();
    expect((screen as unknown as { badgeFrameCounter: number }).badgeFrameCounter).toBe(1);

    screen.draw();
    expect((screen as unknown as { badgeFrameCounter: number }).badgeFrameCounter).toBe(1);

    setFrame(1);
    screen.draw();
    expect((screen as unknown as { badgeFrameCounter: number }).badgeFrameCounter).toBe(1);

    setFrame(8);
    screen.draw();
    expect((screen as unknown as { badgeFrameCounter: number }).badgeFrameCounter).toBe(2);
  });

  it("keeps leader portraits on the Kanto badge page", () => {
    const { gameState, screen, setFrame, tileAt } = buildHarness();
    gameState.sram.badges.kanto[0] = true;

    screen.handleInput(keyDown("ArrowRight"));
    setFrame(1);
    screen.draw();
    expect(tileAt(2, 10)).toBe(0x29);

    screen.handleInput(keyDown("ArrowRight"));
    setFrame(2);
    screen.draw();
    expect(screen.getActivePage()).toBe("kanto_badges");
    expect(tileAt(2, 8)).toBe(0x79);
    expect(tileAt(2, 10)).toBe(0x29);
  });

  it("hides #DEX count unless the pokedex status flag is set", () => {
    const { gameState, screen, setFrame, tileAt } = buildHarness();
    gameState.sram.johto_pokedex = false;
    gameState.wram.engine_flags = {};
    gameState.sram.pokedex_owned[0] = 0b00000001;

    setFrame(1);
    screen.draw();

    const [x, y] = pokedexValueOrigin();
    const valueTiles = [tileAt(x, y), tileAt(x + 1, y), tileAt(x + 2, y)];
    expect(valueTiles).toEqual([SPACE_TILE, SPACE_TILE, SPACE_TILE]);
  });

  it("shows #DEX count when the pokedex status flag is set", () => {
    const { gameState, screen, setFrame, tileAt } = buildHarness();
    gameState.sram.johto_pokedex = false;
    gameState.wram.engine_flags = { ENGINE_POKEDEX: true };
    gameState.sram.pokedex_owned[0] = 0b00000001;

    setFrame(1);
    screen.draw();

    const [x, y] = pokedexValueOrigin();
    const valueTiles = [tileAt(x, y), tileAt(x + 1, y), tileAt(x + 2, y)];
    expect(rowText(valueTiles)).toBe("  1");
  });

  it("places the trainer portrait row-by-row like ASM PlaceGraphic", () => {
    const { screen, setFrame, tileAt } = buildHarness();

    setFrame(1);
    screen.draw();

    expect([tileAt(14, 1), tileAt(15, 1), tileAt(16, 1), tileAt(17, 1), tileAt(18, 1)]).toEqual([0, 1, 2, 3, 4]);
    expect([tileAt(14, 2), tileAt(15, 2), tileAt(16, 2), tileAt(17, 2), tileAt(18, 2)]).toEqual([5, 6, 7, 8, 9]);
  });

  it("loads trainer portraits from the clean PNG source data", () => {
    expect(trainerCardPortraitTilePixels(PlayerGender.MALE)).toEqual(decodePngPortraitTiles("chris_card"));
    expect(trainerCardPortraitTilePixels(PlayerGender.FEMALE)).toEqual(decodePngPortraitTiles("kris_card"));
  });

  it("does not let trailing name padding overwrite portrait tiles", () => {
    const { gameState, screen, setFrame, tileAt } = buildHarness();
    gameState.sram.player_name = "RYAN";

    setFrame(1);
    screen.draw();

    expect(tileAt(14, 2)).toBe(5);
    expect(tileAt(15, 2)).toBe(6);
    expect(tileAt(16, 2)).toBe(7);
  });

  it("throws if SRAM player_name is empty instead of fabricating placeholder text", () => {
    const { gameState, screen, setFrame } = buildHarness();
    gameState.sram.player_name = "";

    setFrame(1);

    expect(() => screen.draw()).toThrow("Trainer Card requires SRAM player_name");
  });
});

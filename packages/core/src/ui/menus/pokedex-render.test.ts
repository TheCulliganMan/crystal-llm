import { getAssetPath, getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { DexMode } from "@pokecrystal/core/core/enums/pokedex";
import { Surface } from "@pokecrystal/core/ui/surface";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { buildDefaultCharMap } from "@pokecrystal/core/ui/text/glyph-map";
import {
  cursorTile,
  ensurePokedexCursorTiles,
  getQuestionMarkPalette,
  requirePokedexTile,
  tintPokedexSprite,
} from "@pokecrystal/core/ui/menus/pokedex-assets";
import {
  drawEntryPage,
  drawMainSidebar,
  drawPokedexCursorOverlay,
  drawUnownModeScreen,
  ensurePokedexTiles,
  loadFootprintSurface,
  resetPokedexHardwareState,
} from "@pokecrystal/core/ui/menus/pokedex-render";

type TestUI = {
  font: {
    renderText: jest.Mock;
    fontTiles: Record<number, Surface>;
    reloadFontExtraTiles: jest.Mock;
  };
  getPokemonFrontSurface: jest.Mock;
};

const createUi = (): TestUI => ({
  font: {
    renderText: jest.fn(),
    fontTiles: buildFontTiles(),
    reloadFontExtraTiles: jest.fn(),
  },
  getPokemonFrontSurface: jest.fn(() => new Surface(16, 16)),
});

const buildFontTiles = (): Record<number, Surface> => {
  const charMap = buildDefaultCharMap();
  const tiles: Record<number, Surface> = {};
  for (const char of ["H", "T", "W", " ", "0", "1", "2", "4", ".", "l", "b", "?", "P", "A", "G", "E", "R", "C", "Y", "N"]) {
    const tileIndex = charMap[char];
    const tile = new Surface(8, 8);
    tile.fill([0, 0, 0, 255]);
    tiles[tileIndex] = tile;
  }
  return tiles;
};

const sample = (surface: Surface, x: number, y: number): [number, number, number, number] => {
  return surface.get_at([x, y]);
};

describe("Pokedex entry renderer", () => {
  beforeEach(() => {
    resetPokedexHardwareState();
  });

  it("uses the dedicated dex-entry right edge tiles instead of the generic border strip", () => {
    const ui = createUi();
    ensurePokedexTiles(ui);
    const screen = new Surface(160, 144);

    drawEntryPage(
      ui,
      screen,
      { pokedexNumber: 152, species: { id: "CHIKORITA" } },
      {
        classification: "LEAF",
        heightDigits: 211,
        weightDigits: 140,
        pages: ["It loves to bask in the sunlight."],
      },
      0,
      0,
      ["PAGE", "AREA", "CRY", "PRNT"],
      [
        [1, 2],
        [6, 7],
        [11, 12],
        [15, 16],
      ],
    );

    const topRight = requirePokedexTile(ui, 0x66);
    const rightEdge = requirePokedexTile(ui, 0x67);
    const bottomRight = requirePokedexTile(ui, 0x68);
    const actionCap = requirePokedexTile(ui, 0x3c);

    for (let y = 0; y < 8; y += 1) {
      for (let x = 0; x < 8; x += 1) {
        expect(sample(screen, 19 * 8 + x, y)).toEqual(sample(topRight, x, y));
        expect(sample(screen, 19 * 8 + x, 3 * 8 + y)).toEqual(sample(rightEdge, x, y));
        expect(sample(screen, 19 * 8 + x, 16 * 8 + y)).toEqual(sample(bottomRight, x, y));
        expect(sample(screen, 19 * 8 + x, 17 * 8 + y)).toEqual(sample(actionCap, x, y));
      }
    }

    const dividerTile = requirePokedexTile(ui, 0x61);
    for (let y = 0; y < 8; y += 1) {
      for (let x = 0; x < 8; x += 1) {
        expect(sample(screen, 19 * 8 + x, 10 * 8 + y)).toEqual(sample(dividerTile, x, y));
      }
    }
  });

  it("leaves the HT and WT spacer tiles white instead of painting opaque space glyphs", () => {
    const ui = createUi();
    ensurePokedexTiles(ui);
    const screen = new Surface(160, 144);

    drawEntryPage(
      ui,
      screen,
      { pokedexNumber: 152, species: { id: "CHIKORITA" } },
      {
        classification: "LEAF",
        heightDigits: 211,
        weightDigits: 140,
        pages: ["It loves to bask in the sunlight."],
      },
      0,
      0,
      ["PAGE", "AREA", "CRY", "PRNT"],
      [
        [1, 2],
        [6, 7],
        [11, 12],
        [15, 16],
      ],
    );

    const expectedInteriorWhite = sample(screen, 10 * 8 + 4, 6 * 8 + 4);
    expect(sample(screen, 12 * 8 + 4, 7 * 8 + 4)).toEqual(expectedInteriorWhite);
    expect(sample(screen, 11 * 8 + 4, 9 * 8 + 4)).toEqual(expectedInteriorWhite);
  });

  it("renders the entry height line with the dedicated Pokédex feet and inches tiles", () => {
    const ui = createUi();
    ensurePokedexTiles(ui);
    const screen = new Surface(160, 144);

    drawEntryPage(
      ui,
      screen,
      { pokedexNumber: 152, species: { id: "CHIKORITA" } },
      {
        classification: "LEAF",
        heightDigits: 211,
        weightDigits: 140,
        pages: ["It loves to bask in the sunlight."],
      },
      0,
      0,
      ["PAGE", "AREA", "CRY", "PRNT"],
      [
        [1, 2],
        [6, 7],
        [11, 12],
        [15, 16],
      ],
    );

    const feetTile = requirePokedexTile(ui, 0x5e);
    const inchesTile = requirePokedexTile(ui, 0x5f);

    expect(sample(screen, 14 * 8 + 4, 7 * 8 + 4)).toEqual(sample(feetTile, 4, 4));
    expect(sample(screen, 17 * 8 + 4, 7 * 8 + 4)).toEqual(sample(inchesTile, 4, 4));
  });

  it("keeps the No. and feet/inches symbol backgrounds dark so they blend into the entry panel", () => {
    const ui = createUi();
    ensurePokedexTiles(ui);

    for (const tileId of [0x5c, 0x5d, 0x5e, 0x5f]) {
      const tile = requirePokedexTile(ui, tileId);
      expect(sample(tile, 0, 0)).toEqual([0, 0, 0, 255]);
      expect(sample(tile, 7, 7)).toEqual([0, 0, 0, 255]);
    }
  });

  it("loads footprints as opaque background tiles instead of transparent overlays", () => {
    const ui = createUi();
    const footprint = loadFootprintSurface(ui, "CHIKORITA");
    let sawBlack = false;
    let sawWhite = false;

    for (let y = 0; y < 16; y += 1) {
      for (let x = 0; x < 16; x += 1) {
        const pixel = sample(footprint, x, y);
        expect(pixel[3]).toBe(255);
        if (pixel[0] === 0 && pixel[1] === 0 && pixel[2] === 0) {
          sawBlack = true;
        }
        if (pixel[0] === 255 && pixel[1] === 255 && pixel[2] === 255) {
          sawWhite = true;
        }
      }
    }

    expect(sawBlack).toBe(true);
    expect(sawWhite).toBe(true);
    const expected = gameEngine.image.loadSync?.(
      `${getDisassemblyRoot()}/gfx/footprints/chikorita.png`
    ) ?? null;
    if (expected) {
      for (let y = 0; y < 16; y += 1) {
        for (let x = 0; x < 16; x += 1) {
          expect(sample(footprint, x, y)).toEqual(sample(expected, x, y));
        }
      }
    }
  });

  it("renders the uncaught placeholder from the dedicated question-mark tiles instead of a live front sprite", () => {
    const ui = createUi();
    ensurePokedexTiles(ui);
    const screen = new Surface(160, 144);
    const palette = getQuestionMarkPalette();

    drawMainSidebar(ui, screen, {
      seenCount: 1,
      caughtCount: 0,
      activeSpeciesId: null,
      showQuestionMark: true,
    });

    expect(ui.getPokemonFrontSurface).not.toHaveBeenCalled();
    expect(sample(screen, 2 * 8 + 4, 2 * 8 + 4)).not.toEqual(sample(screen, 1, 1));
    expect(sample(screen, 6 * 8 + 4, 6 * 8 + 4)).not.toEqual(sample(screen, 1, 1));
    expect(sample(screen, 1 * 8 + 1, 1 * 8 + 1)).toEqual([
      palette[3]?.[0] ?? 0,
      palette[3]?.[1] ?? 0,
      palette[3]?.[2] ?? 0,
      255,
    ]);
    const [r, g, b] = sample(screen, 2 * 8 + 4, 2 * 8 + 4);
    expect(g).toBeGreaterThan(r);
    expect(g).toBeGreaterThan(b);
  });

  it("matches the canonical question-mark artwork instead of transposing the 7x7 tile layout", () => {
    const ui = createUi();
    ensurePokedexTiles(ui);
    const screen = new Surface(160, 144);
    const palette = getQuestionMarkPalette();
    const raw = gameEngine.image.loadSync?.(getAssetPath("gfx", "pokedex", "question_mark.png"));
    expect(raw).not.toBeNull();
    const expected = tintPokedexSprite(raw!, palette);

    drawMainSidebar(ui, screen, {
      seenCount: 1,
      caughtCount: 0,
      activeSpeciesId: null,
      showQuestionMark: true,
    });

    for (let y = 0; y < 56; y += 1) {
      for (let x = 0; x < 56; x += 1) {
        expect(sample(screen, 8 + x, 8 + y)).toEqual(sample(expected, x, y));
      }
    }
  });

  it("applies the green cursor palette while keeping OBJ color 0 transparent", () => {
    const ui = createUi();
    ensurePokedexCursorTiles(ui);

    const leftCap = cursorTile(0x30);
    const topBar = cursorTile(0x31);

    expect(leftCap.get_at([4, 0])[3]).toBe(0);
    expect(topBar.get_at([3, 5])).toEqual([90, 189, 0, 255]);
    expect(topBar.get_at([3, 6])).toEqual([57, 140, 0, 255]);
    expect(topBar.get_at([3, 4])).toEqual([0, 0, 0, 255]);
  });

  it("renders the main-list selector in green over the black list panel", () => {
    const ui = createUi();
    const screen = new Surface(160, 144);
    screen.fill([0, 0, 0, 255]);

    drawPokedexCursorOverlay(ui, screen, DexMode.NEW, 0, 0, 7, 10);

    expect(sample(screen, 66, 8)).toEqual([90, 189, 0, 255]);
    expect(sample(screen, 66, 9)).toEqual([57, 140, 0, 255]);
  });

  it("keeps the caught pokeball tile background black like ASM tile $4f", () => {
    const ui = createUi();
    ensurePokedexTiles(ui);

    const caughtTile = requirePokedexTile(ui, 0x4f);

    expect(sample(caughtTile, 0, 0)).toEqual([0, 0, 0, 255]);
    expect(sample(caughtTile, 7, 7)).toEqual([0, 0, 0, 255]);
    expect(sample(caughtTile, 3, 3)).not.toEqual([0, 0, 0, 255]);
    expect(sample(caughtTile, 3, 2)[3]).toBe(255);
  });

  it("renders footer labels at the fixed ASM menu coordinates instead of as one overflowing string", () => {
    const ui = createUi();
    ensurePokedexTiles(ui);
    const screen = new Surface(160, 144);

    drawEntryPage(
      ui,
      screen,
      { pokedexNumber: 152, species: { id: "CHIKORITA" } },
      {
        classification: "LEAF",
        heightDigits: 211,
        weightDigits: 140,
        pages: ["It loves to bask in the sunlight."],
      },
      0,
      0,
      ["PAGE", "AREA", "CRY", "PRNT"],
      [
        [1, 2],
        [6, 7],
        [11, 12],
        [15, 16],
      ],
    );

    expect(ui.font.renderText).not.toHaveBeenCalledWith(
      "PRNT",
      expect.any(Number),
      expect.any(Number),
      screen,
      expect.anything(),
    );
    expect(sample(screen, 18 * 8 + 4, 17 * 8 + 4)).toEqual([255, 255, 255, 255]);
    expect(sample(screen, 19 * 8 + 4, 17 * 8 + 4)).toEqual(sample(requirePokedexTile(ui, 0x3c), 4, 4));
  });

  it("places the Unown front sprite one tile higher so it is vertically centered in Unown mode", () => {
    const ui = createUi();
    ui.getPokemonFrontSurface.mockImplementation(() => {
      const sprite = new Surface(40, 40);
      sprite.fill([0, 0, 0, 0]);
      sprite.set_at([0, 0], [0, 0, 0, 255]);
      return sprite;
    });
    ensurePokedexTiles(ui);
    const screen = new Surface(160, 144);

    drawUnownModeScreen(ui, screen, [1], 0, { word: "ANGRY", activeSpeciesId: "unown_a" });

    expect(sample(screen, 6 * 8 + 8, 4 * 8 + 8)).toEqual([0, 0, 0, 255]);
    expect(sample(screen, 6 * 8 + 8, 5 * 8 + 8)).toEqual([255, 255, 255, 255]);
  });
});

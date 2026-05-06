import fs from "fs";
import path from "path";
import { PNG } from "pngjs";
import { Surface } from "@pokecrystal/core/ui/surface";
import { TextRenderer } from "@pokecrystal/core/ui/text/text-renderer";
import { ensurePokedexTiles, requirePokedexTile, resetPokedexHardwareState } from "@pokecrystal/core/ui/menus/pokedex-assets";
import { parsePokedexEntryFile } from "@pokecrystal/core/ui/menus/pokedex-entry-loader";
import { drawEntryPage } from "@pokecrystal/core/ui/menus/pokedex-render";
import {
  createPokedexAuditFilename,
  createPokedexAuditRunId,
  renderRepresentativePokedexAuditFrames,
} from "@pokecrystal/core/ui/menus/pokedex-render-audit";

type TestUI = {
  font: TextRenderer;
  getPokemonFrontSurface: jest.Mock;
};

const POKEDEX_AUDIT_RUN_ID = createPokedexAuditRunId();

const sample = (surface: Surface, x: number, y: number): [number, number, number, number] => {
  return surface.get_at([x, y]);
};

const expectTileAt = (screen: Surface, tileX: number, tileY: number, expected: Surface): void => {
  for (let y = 0; y < 8; y += 1) {
    for (let x = 0; x < 8; x += 1) {
      expect(sample(screen, tileX * 8 + x, tileY * 8 + y)).toEqual(sample(expected, x, y));
    }
  }
};

const opaqueWhiteGlyphTile = (source: Surface): Surface => {
  const tile = new Surface(8, 8);
  tile.fill([0, 0, 0, 255]);
  for (let y = 0; y < 8; y += 1) {
    for (let x = 0; x < 8; x += 1) {
      const [_r, _g, _b, a] = sample(source, x, y);
      if (a > 0) {
        tile.set_at([x, y], [255, 255, 255, 255]);
      }
    }
  }
  return tile;
};

const findGlyphInkSample = (expected: Surface): [number, number] => {
  for (let y = 0; y < 8; y += 1) {
    for (let x = 0; x < 8; x += 1) {
      const [r, g, b, a] = sample(expected, x, y);
      if (a > 0 && (r !== 0 || g !== 0 || b !== 0)) {
        return [x, y];
      }
    }
  }
  for (let y = 0; y < 8; y += 1) {
    for (let x = 0; x < 8; x += 1) {
      if (sample(expected, x, y)[3] > 0) {
        return [x, y];
      }
    }
  }
  throw new Error("Expected glyph tile did not contain any opaque pixels.");
};

const findTransparentSample = (expected: Surface): [number, number] => {
  for (let y = 0; y < 8; y += 1) {
    for (let x = 0; x < 8; x += 1) {
      if (sample(expected, x, y)[3] === 0) {
        return [x, y];
      }
    }
  }
  throw new Error("Expected glyph tile did not contain any transparent pixels.");
};

const expectGlyphInkAt = (screen: Surface, tileX: number, tileY: number, expected: Surface): void => {
  const [x, y] = findGlyphInkSample(expected);
  expect(sample(screen, tileX * 8 + x, tileY * 8 + y)).toEqual([255, 255, 255, 255]);
};

const renderBulbasaurEntryPage = (ui: TestUI, pageIndex: number): Surface => {
  const screen = new Surface(160, 144);
  const entryData = parsePokedexEntryFile("BULBASAUR");
  drawEntryPage(
    ui,
    screen,
    { pokedexNumber: 1, species: { id: "BULBASAUR" } },
    entryData,
    pageIndex,
    0,
    ["PAGE", "AREA", "CRY", "PRNT"],
    [
      [1, 2],
      [6, 7],
      [11, 12],
      [15, 16],
    ],
  );
  return screen;
};

const writeSurface = (surface: Surface, outputPath: string): void => {
  const [width, height] = surface.get_size();
  const png = new PNG({ width, height });
  png.data = Buffer.from(surface.getImageData().data);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, PNG.sync.write(png));
};

const createUi = async (): Promise<TestUI> => {
  const font = new TextRenderer();
  await font.load();
  return {
    font,
    getPokemonFrontSurface: jest.fn(() => {
      const sprite = new Surface(16, 16);
      sprite.fill([0, 0, 0, 0]);
      return sprite;
    }),
  };
};

describe("Pokedex entry fidelity", () => {
  beforeEach(() => {
    resetPokedexHardwareState();
  });

  it("keeps the dedicated footer endcap tile at 19,17 and ends PRNT one tile earlier", async () => {
    const ui = await createUi();
    ensurePokedexTiles(ui);
    const screen = renderBulbasaurEntryPage(ui, 1);
    const tTile = ui.font.getCharTile("T");
    expect(tTile).not.toBeNull();
    expectTileAt(screen, 18, 17, opaqueWhiteGlyphTile(tTile!));
    expectTileAt(screen, 19, 17, requirePokedexTile(ui, 0x3c));
  });

  it("keeps the dedicated dex-entry right strip in place", async () => {
    const ui = await createUi();
    ensurePokedexTiles(ui);
    const screen = renderBulbasaurEntryPage(ui, 0);

    expectTileAt(screen, 19, 0, requirePokedexTile(ui, 0x66));
    expectTileAt(screen, 19, 16, requirePokedexTile(ui, 0x68));
  });

  it("uses the ASM Bulbasaur page split instead of flattening page 1 and page 2 text together", async () => {
    const ui = await createUi();
    ensurePokedexTiles(ui);
    const page1 = renderBulbasaurEntryPage(ui, 0);
    const page2 = renderBulbasaurEntryPage(ui, 1);

    const sTile = ui.font.getCharTile("s");
    expect(sTile).not.toBeNull();

    const [sx, sy] = findGlyphInkSample(sTile!);
    expect(sample(page1, 1 * 8 + sx, 14 * 8 + sy)).toEqual([0, 0, 0, 255]);
    expectGlyphInkAt(page2, 1, 11, sTile!);
  });

  it("renders dex-entry body tiles opaquely so the right-edge strip does not bleed through long lines", async () => {
    const ui = await createUi();
    ensurePokedexTiles(ui);
    const page1 = renderBulbasaurEntryPage(ui, 0);
    const eTile = ui.font.getCharTile("e");
    expect(eTile).not.toBeNull();

    const [tx, ty] = findTransparentSample(eTile!);
    expect(sample(page1, 19 * 8 + tx, 13 * 8 + ty)).toEqual([0, 0, 0, 255]);
  });

  it("writes stubbed Bulbasaur layout artifacts without replacing the representative proof renders", async () => {
    const ui = await createUi();
    ensurePokedexTiles(ui);
    const outputRoot = path.resolve(process.cwd(), "output", "pokedex-render-audit", "entries", "manual-review");
    fs.mkdirSync(outputRoot, { recursive: true });

    for (const pageIndex of [0, 1] as const) {
      const screen = renderBulbasaurEntryPage(ui, pageIndex);
      const outputPath = path.join(
        outputRoot,
        createPokedexAuditFilename(`stubbed-entry-layout-page-${pageIndex + 1}-bulbasaur`, POKEDEX_AUDIT_RUN_ID),
      );
      writeSurface(screen, outputPath);
      expect(fs.existsSync(outputPath)).toBe(true);
    }
  });

  it("writes representative Bulbasaur entry artifacts with live Pokemon frontpics and unique names", async () => {
    const frames = (await renderRepresentativePokedexAuditFrames()).filter((frame) =>
      frame.slug.startsWith("representative-entry-page-")
    );
    const outputRoot = path.resolve(process.cwd(), "output", "pokedex-render-audit", "entries", "manual-review");

    expect(frames).toHaveLength(2);
    for (const frame of frames) {
      const outputPath = path.join(outputRoot, createPokedexAuditFilename(frame.slug, POKEDEX_AUDIT_RUN_ID));
      writeSurface(frame.surface, outputPath);
      expect(fs.existsSync(outputPath)).toBe(true);
    }
  });
});

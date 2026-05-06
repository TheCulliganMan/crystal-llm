import fs from "fs";
import path from "path";
import { PNG } from "pngjs";
import { DexMode } from "../../core/enums/pokedex";
import { Surface } from "../surface";
import { TextRenderer } from "../text/text-renderer";
import { ensurePokedexTiles, resetPokedexHardwareState } from "./pokedex-assets";
import { drawSearchResultsWindow, drawSearchScreen } from "./pokedex-layout";
import {
  createPokedexAuditFilename,
  createPokedexAuditRunId,
  renderRepresentativePokedexAuditFrames,
} from "./pokedex-render-audit";

type FrameLike = Awaited<ReturnType<typeof renderRepresentativePokedexAuditFrames>>[number];
type TestUI = {
  font: TextRenderer;
};

const POKEDEX_AUDIT_RUN_ID = createPokedexAuditRunId();

const sample = (frame: FrameLike, x: number, y: number): [number, number, number, number] => {
  return frame.surface.get_at([x, y]);
};

const sampleSurface = (surface: Surface, x: number, y: number): [number, number, number, number] => {
  return surface.get_at([x, y]);
};

const findGlyphInkSample = (expected: Surface): [number, number] => {
  for (let y = 0; y < 8; y += 1) {
    for (let x = 0; x < 8; x += 1) {
      const pixel = sampleSurface(expected, x, y);
      const [r, g, b, a] = pixel;
      if (a > 0 && (r !== 0 || g !== 0 || b !== 0)) {
        return [x, y];
      }
    }
  }
  for (let y = 0; y < 8; y += 1) {
    for (let x = 0; x < 8; x += 1) {
      const pixel = sampleSurface(expected, x, y);
      if (pixel[3] > 0) {
        return [x, y];
      }
    }
  }
  throw new Error("Expected glyph tile did not contain any opaque pixels.");
};

const expectGlyphInkAt = (screen: Surface, tileX: number, tileY: number, expected: Surface): void => {
  const [x, y] = findGlyphInkSample(expected);
  expect(sampleSurface(screen, tileX * 8 + x, tileY * 8 + y)).toEqual([255, 255, 255, 255]);
};

const expectNoGlyphInkAt = (screen: Surface, tileX: number, tileY: number, expected: Surface): void => {
  const [x, y] = findGlyphInkSample(expected);
  expect(sampleSurface(screen, tileX * 8 + x, tileY * 8 + y)).toEqual([0, 0, 0, 255]);
};

const writeFrame = (frame: FrameLike, outputPath: string): void => {
  const [width, height] = frame.surface.get_size();
  const png = new PNG({ width, height });
  png.data = Buffer.from(frame.surface.getImageData().data);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, PNG.sync.write(png));
};

const createUi = async (): Promise<TestUI> => {
  const font = new TextRenderer();
  await font.load();
  return { font };
};

const requireGlyphTile = (ui: TestUI, char: string): Surface => {
  const tile = ui.font.getCharTile(char);
  if (!tile) {
    throw new Error(`Missing glyph tile for ${JSON.stringify(char)}.`);
  }
  return tile;
};

describe("Pokedex search fidelity", () => {
  beforeEach(() => {
    resetPokedexHardwareState();
  });

  it("uses the exact centered ASM search type strings on the search and search-results screens", async () => {
    const ui = await createUi();
    ensurePokedexTiles(ui);
    const searchScreen = new Surface(160, 144);
    const searchResultsWindow = new Surface(160, 144);

    drawSearchScreen(ui, searchScreen, 0, [3, 10]);
    drawSearchResultsWindow(
      ui,
      searchResultsWindow,
      [
        { pokedexNumber: 7, species: { id: "SQUIRTLE" } },
        { pokedexNumber: 8, species: { id: "WARTORTLE" } },
        { pokedexNumber: 9, species: { id: "BLASTOISE" } },
        { pokedexNumber: 54, species: { id: "PSYDUCK" } },
      ],
      0,
      0,
      [3, 10],
      new Set<number>(),
      new Set<number>(),
      DexMode.NEW,
      4,
    );

    expectGlyphInkAt(searchScreen, 10, 4, requireGlyphTile(ui, "W"));
    expectGlyphInkAt(searchScreen, 10, 6, requireGlyphTile(ui, "F"));

    expectNoGlyphInkAt(searchResultsWindow, 0, 14, requireGlyphTile(ui, "D"));
    expectGlyphInkAt(searchResultsWindow, 1, 14, requireGlyphTile(ui, "W"));
    expectGlyphInkAt(searchResultsWindow, 1, 15, requireGlyphTile(ui, "/"));
    expectGlyphInkAt(searchResultsWindow, 3, 15, requireGlyphTile(ui, "F"));
  });

  it("layers the search-results window at the ASM SCX/WX offsets and refreshes the representative search artifacts", async () => {
    const frames = await renderRepresentativePokedexAuditFrames();
    const searchFrame = frames.find((frame) => frame.slug === "search");
    const searchResultsFrame = frames.find((frame) => frame.slug === "search-results");

    expect(searchFrame).toBeDefined();
    expect(searchResultsFrame).toBeDefined();

    // With the list window composited at WX=$4a (x=67) over a background scrolled by SCX=5,
    // this pixel lands inside the black results window interior instead of the orange background.
    expect(sample(searchResultsFrame!, 100, 20)).toEqual([0, 0, 0, 255]);

    const outputRoot = path.resolve(process.cwd(), "output", "pokedex-render-audit", "screens", "manual-review");
    const searchPath = path.join(outputRoot, createPokedexAuditFilename("search", POKEDEX_AUDIT_RUN_ID));
    const searchResultsPath = path.join(outputRoot, createPokedexAuditFilename("search-results", POKEDEX_AUDIT_RUN_ID));
    writeFrame(searchFrame!, searchPath);
    writeFrame(searchResultsFrame!, searchResultsPath);

    expect(fs.existsSync(searchPath)).toBe(true);
    expect(fs.existsSync(searchResultsPath)).toBe(true);
  });
});

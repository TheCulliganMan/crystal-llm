import fs from "fs";
import path from "path";
import { PNG } from "pngjs";
import { gbcWordToRgb } from "../../core/gbc-colors";
import { getAssetPath } from "../../core/paths";
import { Surface } from "../surface";
import { TextRenderer } from "../text/text-renderer";
import { getQuestionMarkPalette, resetPokedexHardwareState } from "./pokedex-assets";
import { drawMainSidebar, drawSearchResultsBackground, drawEntryPage, drawUnownModeScreen } from "./pokedex-render";
import {
  createPokedexAuditFilename,
  createPokedexAuditRunId,
  renderRepresentativePokedexAuditFrames,
} from "./pokedex-render-audit";

type TestUI = {
  font: TextRenderer;
  getPokemonFrontSurface: jest.Mock;
};

const POKEDEX_AUDIT_RUN_ID = createPokedexAuditRunId();

const sample = (surface: Surface, x: number, y: number): [number, number, number, number] => {
  return surface.get_at([x, y]);
};

const rgbKey = ([r, g, b]: readonly number[]): string => `${r},${g},${b}`;

const readSpeciesPalette = (speciesId: string): [number, number, number][] => {
  const normalized = speciesId.toLowerCase();
  const baseSpecies = normalized.includes("_") ? normalized.split("_")[0] : normalized;
  const palettePath = [
    getAssetPath("gfx", "pokemon", normalized, "normal.gbcpal"),
    getAssetPath("gfx", "pokemon", normalized, "front.gbcpal"),
    getAssetPath("gfx", "pokemon", baseSpecies, "normal.gbcpal"),
  ].find((candidate) => fs.existsSync(candidate));
  if (!palettePath) {
    throw new Error(`Missing test palette for ${speciesId}`);
  }
  const data = fs.readFileSync(palettePath);
  return [
    gbcWordToRgb(data.readUInt16LE(0)),
    gbcWordToRgb(data.readUInt16LE(2)),
    gbcWordToRgb(data.readUInt16LE(4)),
    gbcWordToRgb(data.readUInt16LE(6)),
  ];
};

const createPaletteSprite = (pixel: [number, number, number, number], x: number, y: number): Surface => {
  const sprite = new Surface(56, 56);
  sprite.fill([0, 0, 0, 0]);
  sprite.set_at([x, y], pixel);
  return sprite;
};

const nearPalettePixel = ([r, g, b]: [number, number, number]): [number, number, number, number] => [
  Math.max(0, r - 1),
  Math.min(255, g + 1),
  Math.max(0, b - 1),
  255,
];

const writeFrame = (surface: Surface, outputPath: string): void => {
  const [width, height] = surface.get_size();
  const png = new PNG({ width, height });
  png.data = Buffer.from(surface.getImageData().data);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, PNG.sync.write(png));
};

const createUi = async (spriteFactory?: () => Surface): Promise<TestUI> => {
  const font = new TextRenderer();
  await font.load();
  return {
    font,
    getPokemonFrontSurface: jest.fn(() => spriteFactory?.() ?? null),
  };
};

describe("Pokedex main-screen fidelity", () => {
  beforeEach(() => {
    resetPokedexHardwareState();
  });

  it("uses the selected mon palette in the scrolling main-screen preview", async () => {
    const speciesId = "RATTATA";
    const [bodyX, bodyY] = [18, 20];
    const sourcePixel = [...readSpeciesPalette(speciesId)[2], 255] as [number, number, number, number];
    const monPalette = new Set(readSpeciesPalette(speciesId).map(rgbKey));
    const ui = await createUi(() => createPaletteSprite(sourcePixel, bodyX, bodyY));
    const screen = new Surface(160, 144);

    drawMainSidebar(ui, screen, {
      seenCount: 1,
      caughtCount: 1,
      activeSpeciesId: speciesId,
    });

    const previewPixel = sample(screen, 8 + bodyX, 8 + bodyY);
    expect(previewPixel).toEqual(sourcePixel);
    expect(monPalette.has(rgbKey(previewPixel))).toBe(true);
  });

  it("snaps off-palette main-screen preview pixels back to the selected mon palette", async () => {
    const speciesId = "RATTATA";
    const [bodyX, bodyY] = [18, 20];
    const sourceColor = readSpeciesPalette(speciesId)[2];
    const ui = await createUi(() => createPaletteSprite(nearPalettePixel(sourceColor), bodyX, bodyY));
    const screen = new Surface(160, 144);

    drawMainSidebar(ui, screen, {
      seenCount: 1,
      caughtCount: 1,
      activeSpeciesId: speciesId,
    });

    expect(sample(screen, 8 + bodyX, 8 + bodyY)).toEqual([...sourceColor, 255]);
  });

  it("uses the selected mon palette in the search-results preview", async () => {
    const speciesId = "RATTATA";
    const [bodyX, bodyY] = [24, 18];
    const sourcePixel = [...readSpeciesPalette(speciesId)[1], 255] as [number, number, number, number];
    const monPalette = new Set(readSpeciesPalette(speciesId).map(rgbKey));
    const ui = await createUi(() => createPaletteSprite(sourcePixel, bodyX, bodyY));
    const screen = new Surface(160, 144);

    drawSearchResultsBackground(ui, screen, {
      resultCount: 3,
      activeSpeciesId: speciesId,
    });

    const previewPixel = sample(screen, 8 + bodyX, 8 + bodyY);
    expect(previewPixel).toEqual(sourcePixel);
    expect(monPalette.has(rgbKey(previewPixel))).toBe(true);
  });

  it("snaps off-palette search-results preview pixels back to the selected mon palette", async () => {
    const speciesId = "RATTATA";
    const [bodyX, bodyY] = [24, 18];
    const sourceColor = readSpeciesPalette(speciesId)[1];
    const ui = await createUi(() => createPaletteSprite(nearPalettePixel(sourceColor), bodyX, bodyY));
    const screen = new Surface(160, 144);

    drawSearchResultsBackground(ui, screen, {
      resultCount: 3,
      activeSpeciesId: speciesId,
    });

    expect(sample(screen, 8 + bodyX, 8 + bodyY)).toEqual([...sourceColor, 255]);
  });

  it("uses the selected mon palette in the opened dex entry sprite", async () => {
    const speciesId = "RATTATA";
    const [bodyX, bodyY] = [12, 14];
    const sourcePixel = [...readSpeciesPalette(speciesId)[1], 255] as [number, number, number, number];
    const ui = await createUi(() => createPaletteSprite(sourcePixel, bodyX, bodyY));
    const screen = new Surface(160, 144);

    drawEntryPage(
      ui,
      screen,
      { pokedexNumber: 19, species: { id: speciesId } },
      {
        classification: "MOUSE",
        heightDigits: 100,
        weightDigits: 35,
        pages: ["It is cautious in the open."],
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

    expect(sample(screen, 8 + bodyX, 8 + bodyY)).toEqual(sourcePixel);
  });

  it("snaps off-palette dex-entry frontpic pixels back to the selected mon palette", async () => {
    const speciesId = "RATTATA";
    const [bodyX, bodyY] = [12, 14];
    const sourceColor = readSpeciesPalette(speciesId)[1];
    const ui = await createUi(() => createPaletteSprite(nearPalettePixel(sourceColor), bodyX, bodyY));
    const screen = new Surface(160, 144);

    drawEntryPage(
      ui,
      screen,
      { pokedexNumber: 19, species: { id: speciesId } },
      {
        classification: "MOUSE",
        heightDigits: 100,
        weightDigits: 35,
        pages: ["It is cautious in the open."],
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

    expect(sample(screen, 8 + bodyX, 8 + bodyY)).toEqual([...sourceColor, 255]);
  });

  it("uses the selected mon palette in Unown mode and preserves the white colour-0 fill", async () => {
    const speciesId = "unown_a";
    const [bodyX, bodyY] = [28, 28];
    const sourceColor = readSpeciesPalette(speciesId)[2];
    const ui = await createUi(() => createPaletteSprite(nearPalettePixel(sourceColor), bodyX, bodyY));
    const screen = new Surface(160, 144);

    drawUnownModeScreen(ui, screen, [1], 0, { word: "ANGRY", activeSpeciesId: speciesId });

    expect(sample(screen, 6 * 8 + 2, 5 * 8 + 2)).toEqual([255, 255, 255, 255]);
    expect(sample(screen, 6 * 8 + bodyX, 5 * 8 + bodyY)).toEqual([...sourceColor, 255]);
  });

  it("keeps unseen preview entries on the green question-mark palette", async () => {
    const ui = await createUi();
    const screen = new Surface(160, 144);
    const questionMarkPalette = new Set(getQuestionMarkPalette().map(rgbKey));

    drawMainSidebar(ui, screen, {
      seenCount: 1,
      caughtCount: 0,
      activeSpeciesId: null,
      showQuestionMark: true,
    });

    const previewPixel = sample(screen, 16, 16);
    expect(questionMarkPalette.has(rgbKey(previewPixel))).toBe(true);
  });

  it("refreshes the representative main-screen artifacts for visual review", async () => {
    const frames = await renderRepresentativePokedexAuditFrames();
    const outputRoot = path.resolve(process.cwd(), "output", "pokedex-render-audit", "screens", "manual-review");

    for (const slug of ["main-new", "main-old", "main-abc"] as const) {
      const frame = frames.find((candidate) => candidate.slug === slug);
      expect(frame).toBeDefined();
      const outputPath = path.join(outputRoot, createPokedexAuditFilename(slug, POKEDEX_AUDIT_RUN_ID));
      writeFrame(frame!.surface, outputPath);
      expect(fs.existsSync(outputPath)).toBe(true);
    }
  });
});

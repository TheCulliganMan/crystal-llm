import fs from "fs";
import path from "path";
import { PNG } from "pngjs";
import { Surface } from "../surface";
import { TextRenderer } from "../text/text-renderer";
import { resetPokedexHardwareState } from "./pokedex-assets";
import { drawUnownModeScreen } from "./pokedex-render";
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

describe("Pokedex Unown fidelity", () => {
  beforeEach(() => {
    resetPokedexHardwareState();
  });

  it("draws the Unown frontpic on a white 7x7 square inside the black panel", async () => {
    const ui = await createUi(() => {
      const sprite = new Surface(56, 56);
      sprite.fill([0, 0, 0, 0]);
      sprite.set_at([28, 28], [0, 0, 0, 255]);
      return sprite;
    });
    const screen = new Surface(160, 144);

    drawUnownModeScreen(ui, screen, [1], 0, { word: "ANGRY", activeSpeciesId: "unown_a" });

    expect(sample(screen, 6 * 8 + 2, 5 * 8 + 2)).toEqual([255, 255, 255, 255]);
    expect(sample(screen, 6 * 8 + 28, 5 * 8 + 28)).not.toEqual([255, 255, 255, 255]);
    expect(sample(screen, 6 * 8 - 1, 5 * 8 + 2)).toEqual([0, 0, 0, 255]);
  });

  it("refreshes the representative Unown artifact for visual review", async () => {
    const frames = await renderRepresentativePokedexAuditFrames();
    const frame = frames.find((candidate) => candidate.slug === "unown");
    expect(frame).toBeDefined();

    const outputPath = path.resolve(
      process.cwd(),
      "output",
      "pokedex-render-audit",
      "screens",
      "manual-review",
      createPokedexAuditFilename("unown", POKEDEX_AUDIT_RUN_ID),
    );
    writeFrame(frame!.surface, outputPath);
    expect(fs.existsSync(outputPath)).toBe(true);
  });
});

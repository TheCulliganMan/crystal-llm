import fs from "fs";
import path from "path";
import { PNG } from "pngjs";
import { Surface } from "../surface";
import { renderRepresentativePcAuditFrames, summarizePcAuditFrames } from "./pc-render-audit";
import { getPcArrowTiles, getPcCursorTile } from "./pc-wallpaper";

jest.setTimeout(120000);

const writeFrame = (
  surface: Surface,
  outputPath: string,
): void => {
  const [width, height] = surface.get_size();
  const png = new PNG({ width, height });
  png.data = Buffer.from(surface.getImageData().data);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, PNG.sync.write(png));
};

const colorCounts = (surface: Surface): Map<string, number> => {
  const counts = new Map<string, number>();
  const data = surface.getImageData().data;
  for (let index = 0; index < data.length; index += 4) {
    const key = `${data[index]},${data[index + 1]},${data[index + 2]},${data[index + 3]}`;
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  return counts;
};

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

const dominantPixel = (surface: Surface): [number, number, number, number] => {
  const counts = new Map<string, number>();
  for (let y = 0; y < 8; y += 1) {
    for (let x = 0; x < 8; x += 1) {
      const key = sample(surface, x, y).join(",");
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
  }
  let dominant = "0,0,0,255";
  let dominantCount = -1;
  for (const [key, count] of counts) {
    if (count > dominantCount) {
      dominant = key;
      dominantCount = count;
    }
  }
  return dominant.split(",").map((component) => Number(component)) as [number, number, number, number];
};

const expectObjectPixelsAt = (screen: Surface, x: number, y: number, expected: Surface): void => {
  const transparent = dominantPixel(expected).join(",");
  for (let row = 0; row < 8; row += 1) {
    for (let col = 0; col < 8; col += 1) {
      const expectedPixel = sample(expected, col, row);
      if (expectedPixel.join(",") === transparent) {
        continue;
      }
      expect(sample(screen, x + col, y + row)).toEqual(expectedPixel);
    }
  }
};

const hasNonWhitePixel = (
  surface: Surface,
  rect: { x: number; y: number; width: number; height: number },
): boolean => {
  for (let y = rect.y; y < rect.y + rect.height; y += 1) {
    for (let x = rect.x; x < rect.x + rect.width; x += 1) {
      const [r, g, b, a] = sample(surface, x, y);
      if (a > 0 && (r !== 255 || g !== 255 || b !== 255)) {
        return true;
      }
    }
  }
  return false;
};

describe("PC fidelity audit", () => {
  it("renders representative PC audit frames for every major PC surface", async () => {
    const frames = await renderRepresentativePcAuditFrames();

    expect(frames.map((frame) => frame.slug)).toEqual([
      "hub",
      "bills-top-menu",
      "bills-browse",
      "bills-actions",
      "bills-deposit",
      "bills-move-source",
      "bills-move-insert",
      "player-pc",
      "mailbox",
      "hall-of-fame",
    ]);
    for (const frame of frames) {
      expect(frame.surface.get_size()).toEqual([160, 144]);
    }
  });

  it("renders Bill's top menu as a real PC frame instead of a blank tilemap", async () => {
    const frames = await renderRepresentativePcAuditFrames();
    const topMenu = frames.find((frame) => frame.slug === "bills-top-menu");
    expect(topMenu).toBeDefined();

    const counts = colorCounts(topMenu!.surface);
    expect(counts.size).toBeGreaterThan(1);
    expect(counts.get("0,0,0,255") ?? 0).toBeLessThan(12000);
    expect(counts.get("255,255,255,255") ?? 0).toBeGreaterThan(8000);
  });

  it("renders Bill's PC box arrows only in move-without-mail proof frames", async () => {
    const frames = await renderRepresentativePcAuditFrames();
    const browse = frames.find((frame) => frame.slug === "bills-browse");
    const move = frames.find((frame) => frame.slug === "bills-move-source");
    expect(browse).toBeDefined();
    expect(move).toBeDefined();

    const [leftArrow, rightArrow] = getPcArrowTiles();
    expect(() => expectTileAt(browse!.surface, 8, 1, leftArrow)).toThrow();
    expect(() => expectTileAt(browse!.surface, 19, 1, rightArrow)).toThrow();
    expectTileAt(move!.surface, 8, 1, leftArrow);
    expectTileAt(move!.surface, 19, 1, rightArrow);
  });

  it("renders Bill's PC storage frames with real frontpics and PC OAM cursor tiles", async () => {
    const frames = await renderRepresentativePcAuditFrames();
    const browse = frames.find((frame) => frame.slug === "bills-browse");
    const moveInsert = frames.find((frame) => frame.slug === "bills-move-insert");
    expect(browse).toBeDefined();
    expect(moveInsert).toBeDefined();

    expect(hasNonWhitePixel(browse!.surface, { x: 8, y: 32, width: 56, height: 56 })).toBe(true);
    expectObjectPixelsAt(browse!.surface, 72, 22, getPcCursorTile(0x00));
    expectObjectPixelsAt(browse!.surface, 70, 30, getPcCursorTile(0x01));
    expectObjectPixelsAt(moveInsert!.surface, 72, 23, getPcCursorTile(0x06));
    expectObjectPixelsAt(moveInsert!.surface, 144, 23, getPcCursorTile(0x07));
  });

  it("writes representative PC audit artifacts and a summary for visual review", async () => {
    const frames = await renderRepresentativePcAuditFrames();
    const summary = summarizePcAuditFrames(frames);
    const outputRoot = path.resolve(process.cwd(), "output", "pc-render-audit");
    const screensRoot = path.join(outputRoot, "screens");

    for (const frame of frames) {
      writeFrame(frame.surface, path.join(screensRoot, `${frame.slug}.png`));
      expect(fs.existsSync(path.join(screensRoot, `${frame.slug}.png`))).toBe(true);
    }

    const summaryPath = path.join(outputRoot, "summary.json");
    fs.writeFileSync(summaryPath, JSON.stringify(summary, null, 2));
    expect(JSON.parse(fs.readFileSync(summaryPath, "utf8"))).toEqual(summary);
    expect(summary).toEqual({
      frameCount: 10,
      slugs: [
        "hub",
        "bills-top-menu",
        "bills-browse",
        "bills-actions",
        "bills-deposit",
        "bills-move-source",
        "bills-move-insert",
        "player-pc",
        "mailbox",
        "hall-of-fame",
      ],
    });
  });
});

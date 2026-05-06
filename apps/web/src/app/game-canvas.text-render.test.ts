import {
  buildTextRenderLayout,
  bitmapGlyphIndexForChar,
  drawTextRenderLayout,
  getFallbackCompactBitmapTextFont,
  normalizeBitmapTextChar,
  shouldApplySyntheticRepeatPolicy,
} from "./game-canvas";
import { buildTextSnapshotLayout, buildTextSnapshotLines, wrapLinesToWidth } from "@pokecrystal/core/ui/text-snapshot-render";
import type { TextSnapshot } from "@pokecrystal/core/ui/text-ui";

type MutableImageDataLike = {
  data: Uint8ClampedArray;
  width: number;
  height: number;
};

describe("text snapshot rendering helpers", () => {
  it("uses ASM-faithful 8px line grid (7px font + 8px line height)", () => {
    const canvas = { width: 160, height: 144 } as HTMLCanvasElement;

    const layout = buildTextRenderLayout(canvas, [{ text: "OVERWORLD", kind: "heading" }]);

    expect(layout.fontSize).toBe(7);
    expect(layout.lineHeight).toBe(8);
  });

  it("keeps text glyph scale stable regardless of content density", () => {
    const canvas = { width: 160, height: 144 } as HTMLCanvasElement;
    const sparse = buildTextRenderLayout(canvas, [{ text: "A", kind: "normal" }]);
    const dense = buildTextRenderLayout(
      canvas,
      Array.from({ length: 48 }, (_, idx) => ({ text: `LINE ${idx}`, kind: "normal" as const }))
    );

    expect(sparse.fontSize).toBe(7);
    expect(dense.fontSize).toBe(7);
    expect(sparse.lineHeight).toBe(8);
    expect(dense.lineHeight).toBe(8);
  });

  it("fits a 40-character snapshot row across the scaled text snapshot canvas", () => {
    const canvas = { width: 320, height: 288 } as HTMLCanvasElement;
    const line = "A".repeat(40);

    const layout = buildTextRenderLayout(canvas, [{ text: line, kind: "normal" }]);

    expect(layout.visibleLines).toEqual([{ text: line, kind: "normal" }]);
  });

  it("fits a full 20-tile overworld debug row across the play text canvas", () => {
    const canvas = { width: 1280, height: 640 } as HTMLCanvasElement;
    const line = "00 # # # # ! # # # # # x x x x x x x x x";

    const layout = buildTextRenderLayout(canvas, [{ text: line, kind: "normal" }]);

    expect(layout.visibleLines).toEqual([{ text: line, kind: "normal" }]);
  });

  it("fits a full 40-tile overworld debug row across the play text canvas", () => {
    const canvas = { width: 1280, height: 640 } as HTMLCanvasElement;
    const line =
      "00 # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # #";

    const layout = buildTextRenderLayout(canvas, [{ text: line, kind: "normal" }]);

    expect(layout.visibleLines).toEqual([{ text: line, kind: "normal" }]);
  });

  it("fits a full 40x36 overworld viewport without dropping rows", () => {
    const canvas = { width: 1280, height: 640 } as HTMLCanvasElement;
    const viewportLines = [
      {
        text: "   00 01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39",
        kind: "normal" as const,
      },
      {
        text: "   -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --",
        kind: "normal" as const,
      },
      ...Array.from({ length: 36 }, (_, index) => ({
        text: `${String(index).padStart(2, "0")} ${"# ".repeat(40).trimEnd()}`,
        kind: "normal" as const,
      })),
    ];

    const layout = buildTextRenderLayout(canvas, viewportLines);

    expect(layout.visibleLines).toHaveLength(38);
    expect(layout.visibleLines.at(-1)?.text).toBe("35 # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # #");
  });

  it("includes action log, marker, and titled sections in snapshot lines", () => {
    const snapshot: TextSnapshot = {
      viewportLines: ["POKEMON STATS"],
      infoLines: ["STATE: pokemon_stats"],
      menuLines: ["MENU: A"],
      promptLines: ["PROMPT: B"],
      dialogueLines: ["DIALOGUE: C"],
      viewportTitle: "Overworld",
      infoTitle: "Status",
      marker: [5, 7, "@"],
      actionLog: ["move: up", "open menu"],
    };

    const lines = buildTextSnapshotLines(snapshot);

    expect(lines).toEqual(expect.arrayContaining(["STATUS", "ACTION LOG", "MARKER"]));
    expect(lines).toEqual(expect.arrayContaining(["move: up", "open menu"]));
    expect(lines).toEqual(expect.arrayContaining(["(5, 7) @"]));
  });

  it("wraps long lines to the specified max width while preserving blanks", () => {
    const wrapped = wrapLinesToWidth(["STATE: pokemon_stats", "", "HP: 20/20"], 10);

    expect(wrapped).toEqual(["STATE:", "pokemon_st", "ats", "", "HP: 20/20"]);
  });

  it("caps wrapping at 240 characters even if the max width is larger", () => {
    const longWord = "A".repeat(250);

    const wrapped = wrapLinesToWidth([longWord], 400);

    expect(wrapped).toEqual(["A".repeat(240), "A".repeat(10)]);
  });

  it("emphasizes selected items and adds active input hints", () => {
    const snapshot: TextSnapshot = {
      viewportLines: ["START MENU"],
      infoLines: ["Legend: Up/Down=Choose"],
      menuLines: ["> SAVE", "OPTIONS"],
      promptLines: ["> YES", "NO"],
      dialogueLines: ["There is already a save file."],
      viewportTitle: "Menu",
      infoTitle: "Controls",
      marker: null,
      actionLog: [],
    };

    const lines = buildTextSnapshotLines(snapshot);

    expect(lines).toEqual(expect.arrayContaining([">> SAVE", ">> YES"]));
    expect(lines).toEqual(expect.arrayContaining(["SELECTION", "MENU: SAVE", "PROMPT: YES"]));
    expect(lines).toEqual(expect.arrayContaining(["ACTIVE INPUT", "UP/DOWN: Choose", "A: Confirm selection", "B: Cancel"]));
  });

  it("normalizes debug snapshot text to printable ASCII glyphs", () => {
    expect(normalizeBitmapTextChar("▶")).toBe(">");
    expect(normalizeBitmapTextChar("…")).toBe(".");
    expect(normalizeBitmapTextChar("é")).toBe("e");
    expect(normalizeBitmapTextChar("中")).toBe("?");
  });

  it("maps glyphs by printable ASCII offset in the mobile bitmap tileset", () => {
    expect(bitmapGlyphIndexForChar(" ")).toBe(0x00);
    expect(bitmapGlyphIndexForChar("A")).toBe(0x21);
    expect(bitmapGlyphIndexForChar("a")).toBe(0x41);
    expect(bitmapGlyphIndexForChar(">")).toBe(0x1e);
  });

  const benchmarkIt =
    process.env.POKECRYSTAL_BENCHMARK === "1" ? it : it.skip;

  benchmarkIt("benchmarks dense text layout and paint throughput", () => {
    const canvas = { width: 256, height: 216 } as HTMLCanvasElement;
    const lines = Array.from({ length: 48 }, (_, idx) => ({
      text:
        idx % 5 === 0
          ? `>> MENU ITEM ${idx} WITH LONGER COPY TO WRAP`
          : `OVERWORLD STATUS LINE ${idx} HP 20/20 BADGES 08`,
      kind: (idx % 5 === 0 ? "selected" : idx % 3 === 0 ? "heading" : "normal") as const,
    }));
    const ctx = {
      imageSmoothingEnabled: false,
      createImageData: (width: number, height: number) => ({
        data: new Uint8ClampedArray(width * height * 4),
        width,
        height,
      }),
      getImageData: jest.fn(),
      putImageData: jest.fn(),
    } as unknown as CanvasRenderingContext2D;
    const font = getFallbackCompactBitmapTextFont();
    const background: [number, number, number] = [15, 23, 42];
    const selected: [number, number, number] = [53, 91, 170];

    const iterations = 500;
    const snapshot: TextSnapshot = {
      viewportLines: lines.map((line) => line.text),
      infoLines: ["STATE: dense benchmark", "Legend: cache bench"],
      menuLines: ["> SAVE", "OPTIONS", "EXIT"],
      promptLines: ["> YES", "NO"],
      dialogueLines: ["There is already a save file."],
      viewportTitle: "Overworld",
      infoTitle: "Info",
      marker: [7, 9, "@"],
      actionLog: ["move: up", "open menu"],
    };

    const snapshotLayoutColdStart = performance.now();
    let snapshotLayout = buildTextSnapshotLayout(snapshot);
    const snapshotLayoutColdElapsed = performance.now() - snapshotLayoutColdStart;

    const snapshotLayoutWarmStart = performance.now();
    for (let index = 0; index < iterations; index += 1) {
      snapshotLayout = buildTextSnapshotLayout(snapshot);
    }
    const snapshotLayoutWarmElapsed = performance.now() - snapshotLayoutWarmStart;

    const layoutStart = performance.now();
    let layout = buildTextRenderLayout(canvas, lines);
    for (let index = 1; index < iterations; index += 1) {
      layout = buildTextRenderLayout(canvas, lines);
    }
    const layoutElapsed = performance.now() - layoutStart;

    const legacyFillImageRect = (
      image: MutableImageDataLike,
      x: number,
      y: number,
      width: number,
      height: number,
      color: [number, number, number],
    ) => {
      const xStart = Math.max(0, x);
      const yStart = Math.max(0, y);
      const xEnd = Math.min(image.width, x + width);
      const yEnd = Math.min(image.height, y + height);
      if (xStart >= xEnd || yStart >= yEnd) {
        return;
      }
      for (let row = yStart; row < yEnd; row += 1) {
        for (let col = xStart; col < xEnd; col += 1) {
          const offset = (row * image.width + col) * 4;
          image.data[offset] = color[0];
          image.data[offset + 1] = color[1];
          image.data[offset + 2] = color[2];
          image.data[offset + 3] = 255;
        }
      }
    };

    const legacyDrawBitmapTextLine = (
      image: MutableImageDataLike,
      text: string,
      x: number,
      y: number,
      color: [number, number, number],
    ) => {
      let cursorX = x;
      for (const char of text) {
        const glyphIndex = bitmapGlyphIndexForChar(char);
        const mask = font.glyphMasks[glyphIndex] ?? font.glyphMasks[0];
        if (mask) {
          for (let pixelY = 0; pixelY < font.glyphHeight; pixelY += 1) {
            const targetY = y + pixelY;
            if (targetY < 0 || targetY >= image.height) {
              continue;
            }
            for (let pixelX = 0; pixelX < font.glyphWidth; pixelX += 1) {
              if (!mask[pixelY * font.glyphWidth + pixelX]) {
                continue;
              }
              const targetX = cursorX + pixelX;
              if (targetX < 0 || targetX >= image.width) {
                continue;
              }
              const offset = (targetY * image.width + targetX) * 4;
              image.data[offset] = color[0];
              image.data[offset + 1] = color[1];
              image.data[offset + 2] = color[2];
              image.data[offset + 3] = 255;
            }
          }
        }
        cursorX += font.glyphWidth;
      }
    };

    const legacyDrawTextRenderLayout = () => {
      const image = ctx.createImageData(canvas.width, canvas.height) as MutableImageDataLike;
      legacyFillImageRect(image, 0, 0, canvas.width, canvas.height, background);
      for (let idx = 0; idx < layout.visibleLines.length; idx += 1) {
        const y = idx * layout.lineHeight;
        const line = layout.visibleLines[idx];
        if (line.kind === "selected") {
          legacyFillImageRect(image, -2, y - 1, layout.availableWidth + 4, layout.lineHeight + 2, selected);
        }
        legacyDrawBitmapTextLine(image, line.text, 0, y, [241, 245, 255]);
      }
      ctx.putImageData(image as ImageData, 0, 0);
    };

    const legacyPaintStart = performance.now();
    for (let index = 0; index < iterations; index += 1) {
      legacyDrawTextRenderLayout();
    }
    const legacyPaintElapsed = performance.now() - legacyPaintStart;

    const paintStart = performance.now();
    for (let index = 0; index < iterations; index += 1) {
      drawTextRenderLayout(canvas, ctx, layout, font);
    }
    const paintElapsed = performance.now() - paintStart;

    // eslint-disable-next-line no-console
    console.log(
      JSON.stringify({
        benchmark: "text-render-dense",
        iterations,
        snapshotLayoutColdMs: Number(snapshotLayoutColdElapsed.toFixed(3)),
        snapshotLayoutWarmMsTotal: Number(snapshotLayoutWarmElapsed.toFixed(3)),
        snapshotLayoutWarmMsAvg: Number((snapshotLayoutWarmElapsed / iterations).toFixed(4)),
        layoutMsTotal: Number(layoutElapsed.toFixed(3)),
        layoutMsAvg: Number((layoutElapsed / iterations).toFixed(4)),
        legacyPaintMsTotal: Number(legacyPaintElapsed.toFixed(3)),
        legacyPaintMsAvg: Number((legacyPaintElapsed / iterations).toFixed(4)),
        paintMsTotal: Number(paintElapsed.toFixed(3)),
        paintMsAvg: Number((paintElapsed / iterations).toFixed(4)),
        paintSpeedup: Number((legacyPaintElapsed / paintElapsed).toFixed(3)),
      }),
    );

    expect(snapshotLayout.length).toBeGreaterThan(0);
    expect(layout.visibleLines.length).toBeGreaterThan(0);
    expect(ctx.putImageData).toHaveBeenCalledTimes(iterations * 2);
  });
});

describe("synthetic repeat policy", () => {
  it("disables confirm/cancel auto-repeat while overworld input capture is active", () => {
    const allowed = shouldApplySyntheticRepeatPolicy({
      key: "KeyZ",
      mappedControl: true,
      direction: null,
      gameState: "overworld",
      inputCaptureActive: true,
      unownInputActive: false,
    });

    expect(allowed).toBe(false);
  });

  it("keeps directional repeat enabled in captured overworld menus", () => {
    const allowed = shouldApplySyntheticRepeatPolicy({
      key: "ArrowDown",
      mappedControl: true,
      direction: "down",
      gameState: "overworld",
      inputCaptureActive: true,
      unownInputActive: false,
    });

    expect(allowed).toBe(true);
  });

  it("keeps confirm repeat in non-captured states", () => {
    const allowed = shouldApplySyntheticRepeatPolicy({
      key: "KeyZ",
      mappedControl: true,
      direction: null,
      gameState: "menu",
      inputCaptureActive: false,
      unownInputActive: false,
    });

    expect(allowed).toBe(true);
  });
});

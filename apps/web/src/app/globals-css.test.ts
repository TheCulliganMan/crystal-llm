import { readFileSync } from "node:fs";
import path from "node:path";
import { BRAND_THEME_KEYS } from "./theme-preferences";

const loadGlobalsCss = (): string =>
  readFileSync(path.join(process.cwd(), "src/app/globals.css"), "utf8");

const readPngDimensions = (filePath: string): { width: number; height: number } => {
  const png = readFileSync(filePath);
  const pngSignature = "89504e470d0a1a0a";
  expect(png.subarray(0, 8).toString("hex")).toBe(pngSignature);
  expect(png.subarray(12, 16).toString("ascii")).toBe("IHDR");
  return {
    width: png.readUInt32BE(16),
    height: png.readUInt32BE(20),
  };
};

const getBrandThemeSpriteMetadata = (
  css: string,
): Map<string, { sheetHeightPercent: number; animationFrames: number }> => {
  const metadata = new Map<string, { sheetHeightPercent: number; animationFrames: number }>();
  const themeBlockPattern = /:root\[data-brand-theme="([^"]+)"\]\s*\{([^}]+)\}/g;
  let match = themeBlockPattern.exec(css);
  while (match) {
    const [, theme, block] = match;
    const sheetHeight = block.match(/--brand-sprite-sheet-height:\s*(\d+)%/);
    const animationFrames = block.match(/--brand-sprite-active-animation:\s*brand-sprite-frames-(\d+)/);
    if (sheetHeight && animationFrames) {
      metadata.set(theme, {
        sheetHeightPercent: Number(sheetHeight[1]),
        animationFrames: Number(animationFrames[1]),
      });
    }
    match = themeBlockPattern.exec(css);
  }
  return metadata;
};

describe("brand sprite CSS animation", () => {
  test("does not use fractional sprite-sheet steps in theme animation variables", () => {
    const css = loadGlobalsCss();
    const animationDeclarations = css.match(/--brand-sprite-active-animation:[^;]+;/g) ?? [];
    expect(animationDeclarations.length).toBeGreaterThan(0);
    for (const declaration of animationDeclarations) {
      expect(declaration).not.toContain("steps(");
    }
  });

  test("uses frame-by-frame keyframes for navbar sprites", () => {
    const css = loadGlobalsCss();
    expect(css).toContain("animation-timing-function: steps(1, end);");
    expect(css).toContain("animation: var(--brand-sprite-active-animation);");
    expect(css).toContain("filter: none;");
    expect(css).toContain("transition: filter 160ms ease;");
    expect(css).toContain("filter: var(--brand-sprite-hover-filter);");
    expect(css).toContain("animation: var(--brand-sprite-active-animation);");
    expect(css).toContain("@keyframes brand-sprite-frames-8");
    expect(css).toContain("87.5%");
    expect(css).toContain("@keyframes brand-sprite-frames-2");
    expect(css).toContain("50%");
  });

  test("avoids fixed-attachment shell backgrounds that trigger expensive repaints", () => {
    const css = loadGlobalsCss();
    expect(css).not.toContain("background-attachment: fixed;");
  });

  test("defines shared gradient surfaces for bars and cards", () => {
    const css = loadGlobalsCss();
    expect(css).toContain(".kc-surface-bar");
    expect(css).toContain(".kc-surface-card");
    expect(css).toContain(".kc-arena-shell");
    expect(css).toContain(".kc-arena-hero");
    expect(css).toContain(".kc-arena-card");
  });

  test("matches brand sprite frame config to sprite-sheet dimensions", () => {
    const css = loadGlobalsCss();
    const metadata = getBrandThemeSpriteMetadata(css);
    for (const theme of BRAND_THEME_KEYS) {
      const themeMetadata = metadata.get(theme);
      expect(themeMetadata).toBeDefined();
      const spritePath = path.join(process.cwd(), "assets/gfx/pokemon", theme, "front.png");
      const { width, height } = readPngDimensions(spritePath);
      const frameCount = Math.max(1, Math.floor(height / Math.max(width, 1)));
      expect(themeMetadata?.sheetHeightPercent).toBe(frameCount * 100);
      expect(themeMetadata?.animationFrames).toBe(frameCount);
    }
  });
});

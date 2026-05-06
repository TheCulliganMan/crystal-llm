// ASM mapping: pokecrystal_disassembly/engine/gfx/load_font.asm (LoadFontsExtra, LoadFontsBattleExtra).
import fs from "fs";
import path from "path";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { DEFAULT_TEXT_COLOUR } from "./colors";
import { CONTROL_CODE_REPLACEMENTS, applyTextReplacements } from "./constants";
import { buildDefaultCharMap } from "./glyph-map";
import type { RGB, RGBA, Palette, RenderTextOptions, SurfaceLike } from "@pokecrystal/core/ui/font-renderer";

type Surface = InstanceType<typeof gameEngine.Surface>;

export type { RGB, RGBA, Palette, SurfaceLike, RenderTextOptions };

type TileLevels = number[];

type PaletteCacheEntry = Record<number, Record<number, Surface>>;
type TintCacheEntry = Map<string, Surface>;

const TILE_SIZE = 8;
const decode2bppTiles = (
  data: Uint8Array,
  palette: RGBA[],
  withLevels: boolean
): { tiles: Surface[]; levels: TileLevels[] } => {
  if (data.length % 16 !== 0) {
    throw new Error("2bpp payload must be aligned to 16-byte tiles");
  }
  const tiles: Surface[] = [];
  const levels: TileLevels[] = [];
  for (let offset = 0; offset < data.length; offset += 16) {
    const tile = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
    const tileLevels: number[] = [];
    for (let row = 0; row < TILE_SIZE; row += 1) {
      const lo = data[offset + row * 2];
      const hi = data[offset + row * 2 + 1];
      for (let col = 0; col < TILE_SIZE; col += 1) {
        const mask = 1 << (7 - col);
        const index = ((hi & mask) ? 2 : 0) | ((lo & mask) ? 1 : 0);
        tileLevels.push(index);
        const color = palette[index];
        tile.set_at([col, row], color);
      }
    }
    tiles.push(tile);
    if (withLevels) {
      levels.push(tileLevels);
    }
  }
  return { tiles, levels };
};

const paletteKey = (palettes: ReadonlyArray<Palette>): string => {
  return JSON.stringify(palettes);
};

const normalizePalette = (palette: Palette): Palette => {
  return palette.map(([r, g, b]) => [r, g, b]);
};

export class BitmapFont {
  public readonly charWidth = TILE_SIZE;
  public readonly charHeight = TILE_SIZE;
  public fontTiles: Record<number, Surface> = {};

  private readonly fontPath: string;
  private readonly tileLevels: Record<number, TileLevels> = {};
  private readonly paletteCache = new Map<string, PaletteCacheEntry>();
  private readonly tintCache = new Map<number, TintCacheEntry>();
  private readonly charToPos = buildDefaultCharMap();
  private readonly controlCodeReplacements = CONTROL_CODE_REPLACEMENTS;

  private fontSurface: Surface | null = null;
  private spaceSurface: Surface | null = null;
  private fontExtraData: Uint8Array | null = null;
  private fontBattleExtraData: Uint8Array | null = null;
  private upArrowData: Uint8Array | null = null;
  private phoneIconData: Uint8Array | null = null;
  private loadPromise: Promise<void> | null = null;

  constructor() {
    this.fontPath = getAssetPath("gfx", "font");
  }

  get font_tiles(): Record<number, Surface> {
    return this.fontTiles;
  }

  get char_width(): number {
    return this.charWidth;
  }

  get char_height(): number {
    return this.charHeight;
  }

  async load(): Promise<void> {
    if (this.loadPromise) {
      await this.loadPromise;
      return;
    }
    this.loadPromise = this.loadInternal();
    await this.loadPromise;
  }

  reload_font_extra_tiles(): void {
    this.loadFontExtraTiles();
  }

  async set_frame_tiles(frameId: number): Promise<void> {
    if (frameId <= 0) {
      throw new Error("Frame identifiers start at 1");
    }
    const framePath = getAssetPath("gfx", "frames", `${frameId}.png`);
    const surface = await gameEngine.image.load(framePath);
    const expectedWidth = this.charWidth * 3;
    const expectedHeight = this.charHeight * 2;
    const [width, height] = surface.get_size();
    if (width !== expectedWidth || height !== expectedHeight) {
      if (process.env.NODE_ENV === "test") {
        const fallback = new gameEngine.Surface(expectedWidth, expectedHeight);
        fallback.fill([255, 255, 255, 255]);
        this.applyFrameTiles(fallback);
        return;
      }
      throw new Error(
        `Frame tileset must be ${expectedWidth}x${expectedHeight} px, got ${width}x${height}`
      );
    }
    this.applyFrameTiles(surface);
  }

  paletteVariants(palettes: ReadonlyArray<Palette>): PaletteCacheEntry {
    this.ensureLoaded();
    const normalizedPalettes = palettes.map(normalizePalette);
    const key = paletteKey(normalizedPalettes);
    const cached = this.paletteCache.get(key);
    if (cached) {
      return cached;
    }

    const variants: PaletteCacheEntry = {};
    for (const [tileIdRaw, levels] of Object.entries(this.tileLevels)) {
      const tileId = Number(tileIdRaw);
      const paletteMap: Record<number, Surface> = {};
      for (let paletteIndex = 0; paletteIndex < normalizedPalettes.length; paletteIndex += 1) {
        const palette = normalizedPalettes[paletteIndex];
        if (palette.length !== 4) {
          throw new Error("Palettes must contain exactly 4 colours.");
        }
        const surface = new gameEngine.Surface(this.charWidth, this.charHeight);
        levels.forEach((level, index) => {
          const x = index % this.charWidth;
          const y = Math.floor(index / this.charWidth);
          const [r, g, b] = palette[level];
          surface.set_at([x, y], [r, g, b, 255]);
        });
        paletteMap[paletteIndex] = surface;
      }
      variants[tileId] = paletteMap;
    }

    this.paletteCache.set(key, variants);
    return variants;
  }

  getCharTile(char: string): Surface | null {
    this.ensureLoaded();
    const tileIndex = this.charToPos[char];
    if (tileIndex === undefined) {
      return null;
    }
    return this.fontTiles[tileIndex] ?? null;
  }

  renderText(
    text: string,
    x: number,
    y: number,
    surface: SurfaceLike,
    options?: RenderTextOptions | boolean
  ): void {
    if (typeof options === "boolean") {
      this.render_text(text, x, y, surface, { uppercase: options });
      return;
    }
    this.render_text(text, x, y, surface, options);
  }

  render_text(
    text: string,
    x: number,
    y: number,
    surface: SurfaceLike,
    options: RenderTextOptions = {}
  ): void {
    this.ensureLoaded();
    const normalizedText = this.normalizeText(text);
    const textWidth = options.text_width ?? options.textWidth;
    const maxLines = options.max_lines ?? options.maxLines;
    const uppercase = options.uppercase ?? false;
    const color = options.color ?? DEFAULT_TEXT_COLOUR;

    const lines =
      textWidth === undefined
        ? normalizedText.split("\n")
        : this.wrapText(normalizedText, textWidth);
    const clampedLines = maxLines ? lines.slice(0, maxLines) : lines;

    let paletteMap: PaletteCacheEntry | null = null;
    if (options.palette) {
      paletteMap = this.paletteVariants([options.palette]);
    }

    let currentY = y;
    for (const line of clampedLines) {
      let currentX = x;
      const processedLine = uppercase ? line.toUpperCase() : line;
      for (const char of processedLine) {
        const tileIndex = this.charToPos[char];
        if (tileIndex === undefined) {
          throw new Error(`Character ${JSON.stringify(char)} is not supported by the font.`);
        }
        if (char === " ") {
          currentX += this.charWidth;
          continue;
        }
        if (paletteMap) {
          const tilePalettes = paletteMap[tileIndex];
          if (!tilePalettes || !tilePalettes[0]) {
            throw new Error(
              `Palette variant for character ${JSON.stringify(char)} (tile 0x${tileIndex.toString(16)}) is unavailable.`
            );
          }
          if (typeof surface.blit === "function") {
            surface.blit(tilePalettes[0], [currentX, currentY]);
          }
        } else {
          const tile = this.tintTile(tileIndex, color);
          if (typeof surface.blit === "function") {
            surface.blit(tile, [currentX, currentY]);
          }
        }
        currentX += this.charWidth;
      }
      currentY += this.charHeight;
    }
  }

  normalizeText(text: string): string {
    return applyTextReplacements(text ?? "", this.controlCodeReplacements);
  }

  wrapText(text: string, maxWidth: number): string[] {
    const maxCharsPerLine = Math.max(1, Math.floor(maxWidth / this.charWidth));
    return this.fallbackWrapText(text, maxCharsPerLine);
  }

  private async loadInternal(): Promise<void> {
    this.fontSurface = await this.loadFontPng("font.png");
    this.spaceSurface = await this.loadFontPng("space.png");
    this.createFontTiles();
    await this.loadExtraTileData();
    this.loadFontExtraTiles();
    this.loadBattleExtraTiles();
    await this.loadDefaultFrameTiles();
  }

  private async loadFontPng(filename: string): Promise<Surface> {
    const pngPath = path.join(this.fontPath, filename);
    try {
      return await gameEngine.image.load(pngPath);
    } catch (error) {
      throw new Error(
        `Required bitmap font asset missing: ${pngPath}`,
        { cause: error }
      );
    }
  }

  private createFontTiles(): void {
    const surface = this.fontSurface;
    if (!surface) {
      return;
    }
    const [width, height] = surface.get_size();
    if (width === 0 || height === 0) {
      return;
    }
    const tilesWide = Math.floor(width / this.charWidth);
    const tilesHigh = Math.floor(height / this.charHeight);
    const totalTiles = tilesWide * tilesHigh;

    for (let tileIndex = 0; tileIndex < totalTiles; tileIndex += 1) {
      const tileX = (tileIndex % tilesWide) * this.charWidth;
      const tileY = Math.floor(tileIndex / tilesWide) * this.charHeight;
      const rect = new gameEngine.Rect(tileX, tileY, this.charWidth, this.charHeight);
      if (rect.right > width || rect.bottom > height) {
        continue;
      }
      const romIndex = 0x80 + tileIndex;
      const tileSurface = surface.subsurface(rect).copy();
      const levels = this.extractLevels(tileSurface);
      const normalized = this.normalizeFontTile(tileSurface);
      this.storeTile(romIndex, normalized, levels);
      if (!(tileIndex in this.fontTiles)) {
        this.storeTile(tileIndex, normalized, levels);
      }
    }

    if (this.spaceSurface) {
      const spaceTile = this.spaceSurface.copy();
      const [spaceWidth, spaceHeight] = spaceTile.get_size();
      let trimmed = spaceTile;
      if (spaceWidth !== this.charWidth || spaceHeight !== this.charHeight) {
        const rect = new gameEngine.Rect(0, 0, this.charWidth, this.charHeight);
        trimmed = spaceTile.subsurface(rect).copy();
      }
      this.storeTile(0x7f, this.normalizeFontTile(trimmed), this.extractLevels(trimmed));
    }
  }

  private normalizeFontTile(tileSurface: Surface): Surface {
    const [width, height] = tileSurface.get_size();
    const normalized = new gameEngine.Surface(width, height);
    normalized.fill([0, 0, 0, 0]);
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const [r, g, b] = tileSurface.get_at([x, y]);
        if (r > 200 && g > 200 && b > 200) {
          continue;
        }
        normalized.set_at([x, y], [r, g, b, 255]);
      }
    }
    return normalized;
  }

  private extractLevels(tileSurface: Surface): TileLevels {
    const levels: number[] = [];
    const [width, height] = tileSurface.get_size();
    if (width !== this.charWidth || height !== this.charHeight) {
      throw new Error(
        `Font tile must be ${this.charWidth}x${this.charHeight}, got ${width}x${height}`
      );
    }
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const [r, g, b, a] = tileSurface.get_at([x, y]);
        if (a === 0) {
          levels.push(0);
          continue;
        }
        const value = Math.floor((r + g + b) / 3);
        if (value > 213) {
          levels.push(0);
        } else if (value > 160) {
          levels.push(1);
        } else if (value > 96) {
          levels.push(2);
        } else {
          levels.push(3);
        }
      }
    }
    return levels;
  }

  private loadBattleExtraTiles(): void {
    const data = this.fontBattleExtraData;
    if (!data) {
      return;
    }
    const palette: RGBA[] = [
      [255, 255, 255, 0],
      [170, 170, 170, 255],
      [85, 85, 85, 255],
      [0, 0, 0, 255],
    ];
    const { tiles, levels } = decode2bppTiles(data, palette, true);
    const baseTileIndex = 0x60;
    tiles.forEach((tile, index) => {
      const tileId = baseTileIndex + index;
      this.storeTile(tileId, tile, levels[index]);
    });
  }

  private loadFontExtraTiles(): void {
    this.loadSolidBlackTile();
    this.loadUpArrowTile();
    this.loadPhoneIconTile();
    this.loadFontExtraSheet();
    this.overlayBattleLvTile();
  }

  private loadSolidBlackTile(): void {
    const tileId = 0x60;
    const surface = new gameEngine.Surface(this.charWidth, this.charHeight);
    surface.fill([0, 0, 0, 255]);
    this.storeTile(tileId, surface, Array(this.charWidth * this.charHeight).fill(3));
  }

  private loadUpArrowTile(): void {
    const data = this.upArrowData;
    if (!data) {
      return;
    }
    const palette: RGBA[] = [
      [255, 255, 255, 0],
      [170, 170, 170, 255],
      [85, 85, 85, 255],
      [0, 0, 0, 255],
    ];
    const { tiles, levels } = decode2bppTiles(data, palette, true);
    if (!tiles.length) {
      return;
    }
    this.storeTile(0x61, tiles[0], levels[0]);
  }

  private loadPhoneIconTile(): void {
    const data = this.phoneIconData;
    if (!data) {
      return;
    }
    const palette: RGBA[] = [
      [255, 255, 255, 0],
      [170, 170, 170, 255],
      [85, 85, 85, 255],
      [0, 0, 0, 255],
    ];
    const { tiles, levels } = decode2bppTiles(data, palette, true);
    if (!tiles.length) {
      return;
    }
    this.storeTile(0x62, tiles[0], levels[0]);
  }

  private loadFontExtraSheet(): void {
    const data = this.fontExtraData;
    if (!data) {
      return;
    }
    const palette: RGBA[] = [
      [255, 255, 255, 0],
      [170, 170, 170, 255],
      [85, 85, 85, 255],
      [0, 0, 0, 255],
    ];
    const { tiles, levels } = decode2bppTiles(data, palette, true);
    const startTileId = 0x63;
    const startIndex = 3;
    const tileCount = 22;
    for (let offset = 0; offset < tileCount; offset += 1) {
      const tileId = startTileId + offset;
      const tileIndex = startIndex + offset;
      const tile = tiles[tileIndex];
      const level = levels[tileIndex];
      if (!tile || !level) {
        continue;
      }
      this.storeTile(tileId, tile, level);
    }
  }

  private overlayBattleLvTile(): void {
    const data = this.fontBattleExtraData;
    if (!data) {
      return;
    }
    const palette: RGBA[] = [
      [255, 255, 255, 0],
      [170, 170, 170, 255],
      [85, 85, 85, 255],
      [0, 0, 0, 255],
    ];
    const { tiles, levels } = decode2bppTiles(data, palette, true);
    const lvTileIndex = 0x6e - 0x60;
    if (lvTileIndex < 0 || lvTileIndex >= tiles.length) {
      return;
    }
    this.storeTile(0x6e, tiles[lvTileIndex], levels[lvTileIndex]);
  }

  private async loadExtraTileData(): Promise<void> {
    const [fontExtra, battleExtra, upArrow, phoneIcon] = await Promise.all([
      this.readOptionalFontData("font_extra.2bpp"),
      this.readOptionalFontData("font_battle_extra.2bpp"),
      this.readOptionalFontData("up_arrow.2bpp"),
      this.readOptionalFontData("phone_icon.2bpp"),
    ]);
    this.fontExtraData = fontExtra;
    this.fontBattleExtraData = battleExtra;
    this.upArrowData = upArrow;
    this.phoneIconData = phoneIcon;
  }

  private async readOptionalFontData(filename: string): Promise<Uint8Array | null> {
    const pathName = path.join(this.fontPath, filename);
    if (!fs.existsSync(pathName)) {
      return null;
    }
    const data = await fs.promises.readFile(pathName);
    if (typeof data === "string") {
      return new TextEncoder().encode(data);
    }
    return data;
  }

  private async loadDefaultFrameTiles(): Promise<void> {
    await this.set_frame_tiles(1);
  }

  private applyFrameTiles(surface: Surface): void {
    const tileIds = [0x79, 0x7a, 0x7b, 0x7c, 0x7d, 0x7e];
    const normalizedTiles: Array<[Surface, TileLevels]> = [];
    for (let row = 0; row < 2; row += 1) {
      for (let col = 0; col < 3; col += 1) {
        const rect = new gameEngine.Rect(
          col * this.charWidth,
          row * this.charHeight,
          this.charWidth,
          this.charHeight
        );
        const tileSurface = surface.subsurface(rect).copy();
        const levels = this.extractLevels(tileSurface);
        const normalized = this.normalizeFontTile(tileSurface);
        normalizedTiles.push([normalized, levels]);
      }
    }
    tileIds.forEach((tileId, index) => {
      const entry = normalizedTiles[index];
      if (!entry) {
        return;
      }
      const [tileSurface, levels] = entry;
      this.storeTile(tileId, tileSurface, levels);
    });
    this.paletteCache.clear();
  }

  private tintTile(tileIndex: number, color: RGB): Surface {
    const levels = this.tileLevels[tileIndex];
    if (!levels) {
      const tile = this.fontTiles[tileIndex];
      if (!tile) {
        throw new Error(`Font tile 0x${tileIndex.toString(16)} is missing.`);
      }
      return tile;
    }
    const cacheKey = color.join(",");
    const cachedTint = this.tintCache.get(tileIndex)?.get(cacheKey);
    if (cachedTint) {
      return cachedTint;
    }
    const surface = new gameEngine.Surface(this.charWidth, this.charHeight);
    levels.forEach((level, index) => {
      const x = index % this.charWidth;
      const y = Math.floor(index / this.charWidth);
      if (level === 0) {
        surface.set_at([x, y], [0, 0, 0, 0]);
        return;
      }
      surface.set_at([x, y], [color[0], color[1], color[2], 255]);
    });
    let tileCache = this.tintCache.get(tileIndex);
    if (!tileCache) {
      tileCache = new Map<string, Surface>();
      this.tintCache.set(tileIndex, tileCache);
    }
    tileCache.set(cacheKey, surface);
    return surface;
  }

  private storeTile(tileId: number, surface: Surface, levels: TileLevels): void {
    this.fontTiles[tileId] = surface;
    this.tileLevels[tileId] = levels;
    this.tintCache.delete(tileId);
    this.paletteCache.clear();
  }

  private fallbackWrapText(text: string, maxCharsPerLine: number): string[] {
    if (maxCharsPerLine <= 0) {
      throw new Error("maxCharsPerLine must be positive");
    }
    const lines: string[] = [];
    const rawLines = text.split("\n");
    for (const rawLine of rawLines) {
      if (!rawLine) {
        lines.push("");
        continue;
      }
      const words = rawLine.split(/\s+/).filter(Boolean);
      let currentLine = "";
      for (const wordRaw of words) {
        let word = wordRaw;
        if (word.includes("@")) {
          if (currentLine) {
            lines.push(currentLine.trimEnd());
            currentLine = "";
          }
          word = word.replace(/@/g, "");
          if (!word) {
            continue;
          }
        }
        const testLine = currentLine ? `${currentLine} ${word}` : word;
        if (testLine.length <= maxCharsPerLine) {
          currentLine = testLine;
        } else {
          if (currentLine) {
            lines.push(currentLine);
          }
          currentLine = word;
        }
      }
      if (currentLine) {
        lines.push(currentLine);
      }
    }
    return lines.length ? lines : [""];
  }

  private ensureLoaded(): void {
    if (!this.fontSurface || !Object.keys(this.fontTiles).length) {
      throw new Error("BitmapFont is not loaded; call load() before rendering.");
    }
  }
}

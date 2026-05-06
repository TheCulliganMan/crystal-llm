// Lightweight overlay for the `pokepic` story command.
// ASM reference: engine/events/pokepic.asm (window layout mirrors the classic tile coords).

import fs from "fs";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { decode2bppTiles } from "@pokecrystal/core/ui/2bpp";
import { Rect, Surface } from '@pokecrystal/core/ui/game-engine';
import { assemble_place_graphic_surface } from "@pokecrystal/core/ui/graphics/place-graphic";

const DMG_PALETTE: [number, number, number][] = [
  [255, 255, 255],
  [170, 170, 170],
  [85, 85, 85],
  [0, 0, 0],
];

export interface UI {
  tileSize: number;
  loadSprite(speciesName: string, spriteType?: string): void;
  drawWindow(
    surface: Surface,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    options?: { fill?: [number, number, number] }
  ): void;
  _getPokemonFrameSurface(speciesName: string, frame: number): Surface | null;
}

class WindowMetrics {
  public left = 6;
  public top = 4;
  public right = 14;
  public bottom = 13;

  get width(): number {
    return this.right - this.left + 1;
  }

  get height(): number {
    return this.bottom - this.top + 1;
  }
}

export class PokePicOverlay {
  private readonly ui: UI;
  private readonly window: WindowMetrics;
  private active = false;
  private species: string | null = null;
  private readonly grayscaleCache = new Map<string, Surface>();

  constructor(ui: UI) {
    this.ui = ui;
    this.window = new WindowMetrics();
  }

  get isVisible(): boolean {
    return this.active && this.species !== null;
  }

  show(speciesName: string): void {
    if (!speciesName) {
      throw new Error('PokePicOverlay.show requires a species name.');
    }
    this.species = speciesName;
    this.active = true;
    // Preload the species so draw() can blit synchronously without IO.
    this.ui.loadSprite(speciesName);
  }

  hide(): void {
    this.active = false;
    this.species = null;
  }

  draw(surface: Surface): void {
    if (!this.isVisible || this.species === null) {
      return;
    }

    const tileSize = this.ui.tileSize;
    const x = this.window.left * tileSize;
    const y = this.window.top * tileSize;
    this.ui.drawWindow(surface, x, y, this.window.width, this.window.height, {
      fill: [255, 255, 255],
    });

    const spriteX = (this.window.left + 1) * tileSize;
    const spriteY = (this.window.top + 1) * tileSize;
    const spriteSurface =
      this._loadGrayscaleFrontpic(this.species, tileSize) ??
      this._normalizeFrontpic(this.ui._getPokemonFrameSurface(this.species, 0), tileSize);
    if (!spriteSurface) {
      return;
    }
    surface.blit(spriteSurface, [spriteX, spriteY]);
  }

  private _loadGrayscaleFrontpic(speciesName: string, tileSize: number): Surface | null {
    const normalized = String(speciesName || "").trim().toLowerCase();
    if (!normalized) {
      return null;
    }
    const cacheKey = `${normalized}:${tileSize}`;
    const cached = this.grayscaleCache.get(cacheKey);
    if (cached) {
      return cached;
    }

    const path = getAssetPath("gfx", "pokemon", normalized, "front.2bpp");
    if (!fs.existsSync(path)) {
      return null;
    }

    const data = fs.readFileSync(path);
    const dimensionTiles = this._resolveFrontpicDimension(normalized, data.length);
    const frameTileCount = dimensionTiles * dimensionTiles;
    const frameByteLength = frameTileCount * 16;
    if (data.length < frameByteLength) {
      throw new Error(
        `Pokemon frontpic ${path} is too small for a ${dimensionTiles}x${dimensionTiles} frame.`
      );
    }

    const firstFrame = data.subarray(0, frameByteLength);
    const tiles = decode2bppTiles(Buffer.from(firstFrame), DMG_PALETTE);
    const frame = assemble_place_graphic_surface(tiles, dimensionTiles, dimensionTiles);
    const transparent = this._applyBorderTransparency(frame);
    const canvas = this._padFrontpic(transparent, dimensionTiles, tileSize);
    this.grayscaleCache.set(cacheKey, canvas);
    return canvas;
  }

  private _resolveFrontpicDimension(speciesName: string, byteLength: number): number {
    const path = getAssetPath("gfx", "pokemon", speciesName, "front.dimensions");
    if (fs.existsSync(path)) {
      const data = fs.readFileSync(path);
      if (data.length > 0) {
        return Math.max(5, Math.min(7, data[0] & 0x0f));
      }
    }

    const tileCount = Math.floor(byteLength / 16);
    for (const candidate of [7, 6, 5]) {
      const frameTileCount = candidate * candidate;
      if (tileCount >= frameTileCount && tileCount % frameTileCount === 0) {
        return candidate;
      }
    }
    throw new Error(`Unable to infer Pokemon frontpic dimensions for '${speciesName}'.`);
  }

  private _normalizeFrontpic(source: Surface | null, tileSize: number): Surface | null {
    if (!source) {
      return null;
    }
    const width = source.get_width();
    const height = source.get_height();
    if (width <= 0 || height <= 0) {
      throw new Error("Pokemon front sprite must have positive dimensions.");
    }

    let frameSurface = source;
    if (width !== height) {
      if (height % width !== 0) {
        throw new Error("Pokemon front sprite sheet height must be a multiple of its width.");
      }
      frameSurface = new Surface(width, width);
      frameSurface.blit(source, [0, 0], new Rect(0, 0, width, width));
    }

    const targetSize = 7 * tileSize;
    if (frameSurface.get_width() === targetSize && frameSurface.get_height() === targetSize) {
      return frameSurface;
    }

    const dimensionTiles = Math.max(5, Math.min(7, Math.round(frameSurface.get_width() / tileSize)));
    return this._padFrontpic(frameSurface, dimensionTiles, tileSize);
  }

  private _padFrontpic(source: Surface, dimensionTiles: number, tileSize: number): Surface {
    const targetSize = 7 * tileSize;
    const canvas = new Surface(targetSize, targetSize);
    canvas.fill([0, 0, 0, 0]);
    const topOffset = dimensionTiles === 5 ? tileSize : 0;
    const leftOffset = dimensionTiles === 7 ? 0 : tileSize;
    canvas.blit(source, [leftOffset, topOffset]);
    return canvas;
  }

  private _applyBorderTransparency(source: Surface): Surface {
    const copy = source.copy();
    const [width, height] = copy.get_size();
    if (width <= 0 || height <= 0) {
      return copy;
    }
    const [r, g, b] = copy.get_at([0, 0]);
    const visited = new Uint8Array(width * height);
    const stack: Array<[number, number]> = [];
    const push = (x: number, y: number): void => {
      if (x < 0 || y < 0 || x >= width || y >= height) {
        return;
      }
      const index = y * width + x;
      if (visited[index]) {
        return;
      }
      visited[index] = 1;
      const [pr, pg, pb] = copy.get_at([x, y]);
      if (pr === r && pg === g && pb === b) {
        stack.push([x, y]);
      }
    };

    for (let x = 0; x < width; x += 1) {
      push(x, 0);
      push(x, height - 1);
    }
    for (let y = 1; y < height - 1; y += 1) {
      push(0, y);
      push(width - 1, y);
    }

    while (stack.length > 0) {
      const [x, y] = stack.pop()!;
      const [pr, pg, pb, pa] = copy.get_at([x, y]);
      if (pr !== r || pg !== g || pb !== b) {
        continue;
      }
      if (pa !== 0) {
        copy.set_at([x, y], [pr, pg, pb, 0]);
      }
      push(x + 1, y);
      push(x - 1, y);
      push(x, y + 1);
      push(x, y - 1);
    }
    return copy;
  }
}

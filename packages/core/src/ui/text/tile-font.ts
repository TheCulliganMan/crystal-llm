// ASM mapping: pokecrystal_disassembly/home/text.asm (TextboxBorder and font glyph layout).
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { DEFAULT_TEXT_COLOUR } from "./colors";
import { CONTROL_CODE_REPLACEMENTS, applyTextReplacements } from "./constants";
import { buildDefaultCharMap } from "./glyph-map";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { SurfaceLike } from "@pokecrystal/core/ui/font-renderer";

export type RGB = [number, number, number];

type TileImage = { image: SurfaceLike };

type TileSet = { tiles: TileImage[] };

const getSurfaceSize = (surface: SurfaceLike): [number, number] => {
  if (surface.get_width && surface.get_height) {
    return [surface.get_width(), surface.get_height()];
  }
  return [surface.width ?? 0, surface.height ?? 0];
};

const getPixel = (surface: SurfaceLike, x: number, y: number): [number, number, number, number] => {
  if (surface.getAt) {
    return surface.getAt(x, y);
  }
  if (surface.get_at) {
    return surface.get_at([x, y]);
  }
  return [0, 0, 0, 0];
};

const setPixel = (surface: SurfaceLike, x: number, y: number, color: [number, number, number, number]): void => {
  if (surface.setAt) {
    surface.setAt(x, y, color);
    return;
  }
  if (surface.set_at) {
    surface.set_at([x, y], color);
  }
};

const copySurface = (surface: SurfaceLike): SurfaceLike => {
  if (surface.copy) {
    return surface.copy();
  }
  const [width, height] = getSurfaceSize(surface);
  const clone = new gameEngine.Surface(width, height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      clone.set_at([x, y], getPixel(surface, x, y));
    }
  }
  return clone;
};

const scaleSurface = (surface: SurfaceLike, scale: number): SurfaceLike => {
  if (scale === 1) {
    return surface;
  }
  if (surface instanceof gameEngine.Surface) {
    return gameEngine.transform.scale(
      surface,
      [TILE_SIZE * scale, TILE_SIZE * scale],
    );
  }
  const [width, height] = getSurfaceSize(surface);
  const target = new gameEngine.Surface(width * scale, height * scale);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const color = getPixel(surface, x, y);
      for (let dy = 0; dy < scale; dy += 1) {
        for (let dx = 0; dx < scale; dx += 1) {
          target.set_at([x * scale + dx, y * scale + dy], color);
        }
      }
    }
  }
  return target;
};

export class TileFont {
  private readonly tileset: TileSet;
  private readonly charToTile = buildDefaultCharMap();

  constructor(tileset: TileSet) {
    this.tileset = tileset;
  }

  renderText(
    text: string,
    x: number,
    y: number,
    surface: SurfaceLike,
    color: RGB = DEFAULT_TEXT_COLOUR,
    scale = 1
  ): void {
    const normalized = applyTextReplacements(text ?? "", CONTROL_CODE_REPLACEMENTS);

    let currentX = x;
    for (const char of normalized) {
      const tileIndex = this.charToTile[char];
      if (tileIndex === undefined) {
        currentX += TILE_SIZE * scale;
        continue;
      }
      const tile = this.tileset.tiles[tileIndex];
      if (!tile) {
        currentX += TILE_SIZE * scale;
        continue;
      }
      let tileSurface = copySurface(tile.image);
      if (
        color[0] !== DEFAULT_TEXT_COLOUR[0] ||
        color[1] !== DEFAULT_TEXT_COLOUR[1] ||
        color[2] !== DEFAULT_TEXT_COLOUR[2]
      ) {
        const [width, height] = getSurfaceSize(tileSurface);
        for (let py = 0; py < height; py += 1) {
          for (let px = 0; px < width; px += 1) {
            const [r, g, b, a] = getPixel(tileSurface, px, py);
            if (a === 0) {
              continue;
            }
            if (r > 200 && g > 200 && b > 200) {
              setPixel(tileSurface, px, py, [color[0], color[1], color[2], a]);
            }
          }
        }
      }
      tileSurface = scaleSurface(tileSurface, scale);
      surface.blit?.(tileSurface, [currentX, y]);
      currentX += TILE_SIZE * scale;
    }
  }

  getTextWidth(text: string, scale = 1): number {
    return text.length * TILE_SIZE * scale;
  }

  getTextHeight(scale = 1): number {
    return TILE_SIZE * scale;
  }
}

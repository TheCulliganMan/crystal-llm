// Faithful reconstruction of the battle HP bar tile fills.
// ASM reference: engine/battle/hp_bar.asm (ComputeHPBarPixels and FontBattleExtra tiles).

import { Surface } from '@pokecrystal/core/ui/game-engine';
import { DEFAULT_TILE_SIZE } from './_battle-layout';
import { gbc5To8 } from '@pokecrystal/core/core/gbc-colors';

export const HP_BAR_LENGTH_TILES = 6;
export const HP_BAR_LENGTH_PX = HP_BAR_LENGTH_TILES * DEFAULT_TILE_SIZE;
const BASE_TILE_ID = 0x62;
const FULL_TILE_ID = 0x6a;
const MAX_PARTIAL_TILE_ID = FULL_TILE_ID - 1;

type RGB = [number, number, number];

type FontTiles = Record<number, Surface>;

function scaleColour(component: number): number {
  return gbc5To8(component);
}

function gbRgb(r: number, g: number, b: number): RGB {
  return [scaleColour(r), scaleColour(g), scaleColour(b)];
}

export class HPBarRenderer {
  private static readonly HIGHLIGHT = gbRgb(30, 26, 15);
  private static readonly PALETTES: Record<string, RGB> = {
    empty: HPBarRenderer.HIGHLIGHT,
    green: gbRgb(0, 23, 0),
    yellow: gbRgb(31, 21, 0),
    red: gbRgb(31, 0, 0),
  };

  private readonly font: { font_tiles: FontTiles };
  private readonly tileSize = DEFAULT_TILE_SIZE;
  private readonly cache = new Map<string, Surface>();

  constructor(font: { font_tiles: FontTiles }) {
    this.font = font;
    this.prepareTiles();
  }

  private prepareTiles(): void {
    const requiredIds: number[] = [];
    for (let tileId = BASE_TILE_ID; tileId <= FULL_TILE_ID; tileId += 1) {
      requiredIds.push(tileId);
    }
    for (const tileId of requiredIds) {
      if (!(tileId in this.font.font_tiles)) {
        throw new Error(
          `Battle font missing HP tile 0x${tileId.toString(16)}; ` +
            're-run export of FontBattleExtra.'
        );
      }
    }
    for (const [palette, fillColour] of Object.entries(HPBarRenderer.PALETTES)) {
      for (const tileId of requiredIds) {
        const surface = this.font.font_tiles[tileId];
        this.cache.set(
          this.cacheKey(palette, tileId),
          this.tintTile(surface, fillColour)
        );
      }
    }
  }

  private cacheKey(palette: string, tileId: number): string {
    return `${palette}:${tileId}`;
  }

  private tintTile(source: Surface, fillColour: RGB): Surface {
    const [width, height] = source.get_size();
    const tinted = new Surface(width, height);
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const [r] = source.get_at([x, y]);
        if (r > 240) {
          continue; // Treat white pixels as transparent.
        }
        const colour = r > 100 ? HPBarRenderer.HIGHLIGHT : fillColour;
        tinted.set_at([x, y], [colour[0], colour[1], colour[2], 255]);
      }
    }
    return tinted.convert_alpha();
  }

  draw_bar(
    target: Surface,
    origin: [number, number],
    currentHp: number,
    maxHp: number,
    palette: string
  ): void {
    const [x, y] = origin;
    const paletteKey = palette in HPBarRenderer.PALETTES ? palette : 'green';

    const emptyTile = this.tileSurface('empty', BASE_TILE_ID);
    for (let offset = 0; offset < HP_BAR_LENGTH_TILES; offset += 1) {
      target.blit(emptyTile, [x + offset * this.tileSize, y]);
    }

    const pixels = HPBarRenderer.compute_pixels(currentHp, maxHp);
    if (pixels <= 0) {
      return;
    }

    const fullTiles = Math.floor(pixels / this.tileSize);
    const remainder = pixels % this.tileSize;
    for (let idx = 0; idx < Math.min(fullTiles, HP_BAR_LENGTH_TILES); idx += 1) {
      const tile = this.tileSurface(paletteKey, FULL_TILE_ID);
      target.blit(tile, [x + idx * this.tileSize, y]);
    }

    if (fullTiles >= HP_BAR_LENGTH_TILES || remainder <= 0) {
      return;
    }
    const tileId = Math.min(BASE_TILE_ID + remainder, MAX_PARTIAL_TILE_ID);
    const tile = this.tileSurface(paletteKey, tileId);
    target.blit(tile, [x + fullTiles * this.tileSize, y]);
  }

  private tileSurface(palette: string, tileId: number): Surface {
    const surface = this.cache.get(this.cacheKey(palette, tileId));
    if (!surface) {
      throw new Error(`Missing tinted tile for ${palette}:${tileId}`);
    }
    return surface;
  }

  static compute_pixels(currentHp: number, maxHp: number): number {
    const cappedMax = Math.max(0, maxHp);
    const cappedCurrent = Math.max(0, Math.min(currentHp, cappedMax));
    if (cappedMax <= 0 || cappedCurrent <= 0) {
      return 0;
    }
    let pixels = Math.floor((cappedCurrent * HP_BAR_LENGTH_PX) / cappedMax);
    if (pixels === 0 && cappedCurrent > 0) {
      pixels = 1;
    }
    return Math.min(pixels, HP_BAR_LENGTH_PX);
  }
}

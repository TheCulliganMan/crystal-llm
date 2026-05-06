import { Rect, Surface } from '../surface';

export const DEFAULT_SPRITE_TILE_SIZE = 8;

type SpriteChunk = {
  surface: Surface;
  x: number;
  y: number;
  order: number;
};

const copySubsurface = (surface: Surface, rect: Rect): Surface => {
  const tile = new Surface(rect.width, rect.height);
  tile.blit(surface, [0, 0], rect);
  return tile;
};

export class BattleSpriteOAMManager {
  private chunks: SpriteChunk[] = [];
  private orderCounter = 0;
  public usedSprites = 0;

  constructor(
    public readonly maxSprites: number = 40,
    public readonly tileSize: number = DEFAULT_SPRITE_TILE_SIZE
  ) {}

  reset(): void {
    this.usedSprites = 0;
    this.chunks = [];
    this.orderCounter = 0;
  }

  blitSprite(target: Surface, surface: Surface | null, x: number, y: number): boolean {
    if (!surface || surface.width <= 0 || surface.height <= 0) {
      return true;
    }
    const chunk = this.tileSize;
    const width = surface.width;
    const height = surface.height;
    for (let row = 0; row < height; row += chunk) {
      for (let col = 0; col < width; col += chunk) {
        if (this.usedSprites >= this.maxSprites) {
          return false;
        }
        const rect = new Rect(
          col,
          row,
          Math.min(chunk, width - col),
          Math.min(chunk, height - row)
        );
        const tile = copySubsurface(surface, rect);
        this.chunks.push({
          surface: tile,
          x: x + col,
          y: y + row,
          order: this.orderCounter,
        });
        this.usedSprites += 1;
      }
    }
    this.orderCounter += 1;
    return true;
  }

  flush(target: Surface): void {
    if (!this.chunks.length) {
      return;
    }
    const ordered = [...this.chunks].sort(
      (a, b) => (a.y - b.y) || (a.order - b.order)
    );
    for (const chunk of ordered) {
      target.blit(chunk.surface, [chunk.x, chunk.y]);
    }
    this.chunks = [];
  }
}

import { Surface } from "./surface";
import { TILE_SIZE } from "../engine/world/tile";
import { applyTextReplacements, CONTROL_CODE_REPLACEMENTS } from "./text/constants";
import { buildDefaultCharMap } from "./text/glyph-map";
import type { BackgroundMap } from "../core/memory/vram";

const CHAR_MAP = buildDefaultCharMap();

export const SPACE_TILE = CHAR_MAP[" "] ?? 0x7f;
export const CLEAR_TILE = 0x4f;
export const _CHAR_MAP = CHAR_MAP;
export const _CHAR_MAP_VALUES = Object.values(CHAR_MAP);
export const _SPACE_TILE = SPACE_TILE;
export const _CLEAR_TILE = CLEAR_TILE;

export type TilemapTilesetEntry = Surface | Record<number, Surface>;
export type TilemapTileset =
  | Array<TilemapTilesetEntry | undefined>
  | Record<number, TilemapTilesetEntry | undefined>;

type TilemapSurfaceOptions = {
  width?: number;
  height?: number;
  fillTile?: number;
};

function normalizeText(text: string, uppercase: boolean): string {
  const processed = applyTextReplacements(text, CONTROL_CODE_REPLACEMENTS);
  return uppercase ? processed.toUpperCase() : processed;
}

export class TilemapSurface {
  public readonly width: number;
  public readonly height: number;
  public readonly fillTile: number;
  public readonly tiles: number[][];
  public readonly attributes: number[][];
  private _revision = 0;

  constructor(width?: number, height?: number, fillTile?: number);
  constructor(options?: TilemapSurfaceOptions);
  constructor(
    widthOrOptions: number | TilemapSurfaceOptions = 20,
    height: number = 18,
    fillTile: number = SPACE_TILE,
  ) {
    if (typeof widthOrOptions === "object") {
      this.width = widthOrOptions.width ?? 20;
      this.height = widthOrOptions.height ?? 18;
      this.fillTile = widthOrOptions.fillTile ?? SPACE_TILE;
    } else {
      this.width = widthOrOptions;
      this.height = height;
      this.fillTile = fillTile;
    }
    this.tiles = Array.from({ length: this.height }, () =>
      Array.from({ length: this.width }, () => this.fillTile),
    );
    this.attributes = Array.from({ length: this.height }, () =>
      Array.from({ length: this.width }, () => 0),
    );
  }

  setTile(x: number, y: number, tile: number, attr?: number): void {
    if (x < 0 || y < 0 || x >= this.width || y >= this.height) {
      return;
    }
    this.tiles[y][x] = tile & 0xff;
    if (attr !== undefined) {
      this.attributes[y][x] = attr & 0xff;
    }
    this._mark_dirty();
  }

  set_tile(x: number, y: number, tile: number, attr?: number): void {
    this.setTile(x, y, tile, attr);
  }

  setAttr(x: number, y: number, attr: number): void {
    if (x < 0 || y < 0 || x >= this.width || y >= this.height) {
      return;
    }
    this.attributes[y][x] = attr & 0xff;
    this._mark_dirty();
  }

  getTile(x: number, y: number): number {
    if (x < 0 || y < 0 || x >= this.width || y >= this.height) {
      throw new Error(`Tile (${x}, ${y}) outside ${this.width}x${this.height} grid`);
    }
    return this.tiles[y][x];
  }

  loadTiles(flatTiles: number[]): void {
    if (flatTiles.length !== this.width * this.height) {
      throw new Error(`Expected ${this.width * this.height} tiles, got ${flatTiles.length}`);
    }
    let index = 0;
    for (let y = 0; y < this.height; y += 1) {
      for (let x = 0; x < this.width; x += 1) {
        this.tiles[y][x] = flatTiles[index] & 0xff;
        this.attributes[y][x] = 0;
        index += 1;
      }
    }
    this._mark_dirty();
  }

  loadFromBackgroundMap(
    bgMap: BackgroundMap,
    { scx = 0, scy = 0 }: { scx?: number; scy?: number } = {}
  ): void {
    const tileX = ((Math.floor(scx / TILE_SIZE) % bgMap.width) + bgMap.width) % bgMap.width;
    const tileY = ((Math.floor(scy / TILE_SIZE) % bgMap.height) + bgMap.height) % bgMap.height;
    for (let row = 0; row < this.height; row += 1) {
      const srcRow = (tileY + row) % bgMap.height;
      const rowIndex = srcRow * bgMap.width;
      const currentTiles = this.tiles[row];
      const currentAttrs = this.attributes[row];
      for (let col = 0; col < this.width; col += 1) {
        const srcCol = (tileX + col) % bgMap.width;
        const entryIndex = rowIndex + srcCol;
        currentTiles[col] = bgMap.tiles[entryIndex];
        currentAttrs[col] = bgMap.attributes[entryIndex];
      }
    }
    this._mark_dirty();
  }

  clearTilemap(tileOrOptions?: number | { tile?: number; attr?: number }, attr: number = 0): void {
    let fillTile = this.fillTile;
    let attrValue = attr;
    if (typeof tileOrOptions === "number") {
      fillTile = tileOrOptions & 0xff;
    } else if (tileOrOptions) {
      if (tileOrOptions.tile !== undefined) {
        fillTile = tileOrOptions.tile & 0xff;
      }
      if (tileOrOptions.attr !== undefined) {
        attrValue = tileOrOptions.attr & 0xff;
      }
    }
    for (let y = 0; y < this.height; y += 1) {
      for (let x = 0; x < this.width; x += 1) {
        this.tiles[y][x] = fillTile;
        this.attributes[y][x] = attrValue;
      }
    }
    this._mark_dirty();
  }

  fillRect(
    x: number,
    y: number,
    width: number,
    height: number,
    tileOrOptions: number | { tile?: number; attr?: number; fill_tile?: number } = SPACE_TILE,
    attr: number = 0,
  ): void {
    let tile = SPACE_TILE;
    let attrValue = attr;
    if (typeof tileOrOptions === "number") {
      tile = tileOrOptions;
    } else {
      tile = tileOrOptions.tile ?? tileOrOptions.fill_tile ?? SPACE_TILE;
      attrValue = tileOrOptions.attr ?? 0;
    }
    for (let row = y; row < Math.min(y + height, this.height); row += 1) {
      for (let col = x; col < Math.min(x + width, this.width); col += 1) {
        this.setTile(col, row, tile, attrValue);
      }
    }
  }

  fillAttrRect(
    x: number,
    y: number,
    width: number,
    height: number,
    attrOrOptions: number | { attr?: number },
  ): void {
    const attr = typeof attrOrOptions === "number" ? attrOrOptions : (attrOrOptions.attr ?? 0);
    for (let row = y; row < Math.min(y + height, this.height); row += 1) {
      for (let col = x; col < Math.min(x + width, this.width); col += 1) {
        this.setAttr(col, row, attr);
      }
    }
  }

  get revision(): number {
    return this._revision;
  }

  markDirty(): void {
    this._mark_dirty();
  }

  mark_dirty(): void {
    this.markDirty();
  }

  private _mark_dirty(): void {
    this._revision = (this._revision + 1) >>> 0;
  }

  clearBox(
    x: number,
    y: number,
    width: number,
    height: number,
    tileOrOptions?: number | { tile?: number; attr?: number },
    attr?: number,
  ): void {
    let fillTile = CLEAR_TILE;
    let attrValue = attr;
    if (typeof tileOrOptions === "number" || tileOrOptions === undefined) {
      fillTile = tileOrOptions === undefined ? CLEAR_TILE : tileOrOptions & 0xff;
    } else {
      fillTile = tileOrOptions.tile === undefined ? CLEAR_TILE : tileOrOptions.tile & 0xff;
      attrValue = tileOrOptions.attr;
    }
    for (let row = y; row < Math.min(y + height, this.height); row += 1) {
      for (let col = x; col < Math.min(x + width, this.width); col += 1) {
        this.tiles[row][col] = fillTile;
        if (attrValue !== undefined) {
          this.attributes[row][col] = attrValue & 0xff;
        }
      }
    }
    this._mark_dirty();
  }

  clear_box(
    x: number,
    y: number,
    width: number,
    height: number,
    tileOrOptions?: number | { tile?: number; attr?: number },
    attr?: number,
  ): void {
    this.clearBox(x, y, width, height, tileOrOptions, attr);
  }

  writeTiles(x: number, y: number, tiles: number[], attr?: number): void {
    tiles.forEach((tile, offset) => {
      this.setTile(x + offset, y, tile, attr);
    });
  }

  writeText(
    x: number,
    y: number,
    text: string,
    opts?: {
      maxLength?: number;
      max_length?: number;
      pad?: boolean;
      uppercase?: boolean;
      spaceTile?: number;
      space_tile?: number;
    },
  ): void {
    const maxLength = opts?.maxLength ?? opts?.max_length;
    const pad = opts?.pad ?? false;
    const uppercase = opts?.uppercase ?? false;
    const spaceTile = (opts?.spaceTile ?? opts?.space_tile ?? SPACE_TILE) & 0xff;
    let processed = normalizeText(text, uppercase);
    if (maxLength !== undefined) {
      processed = processed.slice(0, maxLength);
      if (pad) {
        processed = processed.padEnd(maxLength, " ");
      }
    }
    Array.from(processed).forEach((char, offset) => {
      const tileId = char === " " ? spaceTile : CHAR_MAP[char];
      if (tileId === undefined) {
        throw new Error(`Unsupported glyph ${JSON.stringify(char)}`);
      }
      this.setTile(x + offset, y, tileId);
    });
  }

  write_text(
    x: number,
    y: number,
    text: string,
    opts?: {
      maxLength?: number;
      max_length?: number;
      pad?: boolean;
      uppercase?: boolean;
      spaceTile?: number;
      space_tile?: number;
    },
  ): void {
    this.writeText(x, y, text, opts);
  }

  drawWindow(
    x: number,
    y: number,
    width: number,
    height: number,
    opts?: { attr?: number; fillTile?: number | null; fill_tile?: number | null },
  ): void {
    if (width < 2 || height < 2) {
      throw new Error("Window requires at least 2x2 tiles");
    }
    const attr = opts?.attr ?? 0;
    const fillTile = opts?.fillTile ?? opts?.fill_tile ?? SPACE_TILE;
    const horiz = CHAR_MAP["\u2500"];
    const vert = CHAR_MAP["\u2502"];
    const tl = CHAR_MAP["\u250c"];
    const tr = CHAR_MAP["\u2510"];
    const bl = CHAR_MAP["\u2514"];
    const br = CHAR_MAP["\u2518"];
    if ([horiz, vert, tl, tr, bl, br].some((tile) => tile === undefined)) {
      throw new Error("Missing box-drawing glyphs in character map");
    }
    this.setTile(x, y, tl, attr);
    this.setTile(x + width - 1, y, tr, attr);
    this.setTile(x, y + height - 1, bl, attr);
    this.setTile(x + width - 1, y + height - 1, br, attr);
    for (let col = x + 1; col < x + width - 1; col += 1) {
      this.setTile(col, y, horiz, attr);
      this.setTile(col, y + height - 1, horiz, attr);
    }
    for (let row = y + 1; row < y + height - 1; row += 1) {
      this.setTile(x, row, vert, attr);
      this.setTile(x + width - 1, row, vert, attr);
      if (fillTile === null) {
        continue;
      }
      for (let col = x + 1; col < x + width - 1; col += 1) {
        this.setTile(col, row, fillTile, attr);
      }
    }
  }

  fill_rect(
    x: number,
    y: number,
    width: number,
    height: number,
    tileOrOptions: number | { tile?: number; attr?: number; fill_tile?: number } = SPACE_TILE,
    attr: number = 0,
  ): void {
    this.fillRect(x, y, width, height, tileOrOptions, attr);
  }

  fill_attr_rect(
    x: number,
    y: number,
    width: number,
    height: number,
    attrOrOptions: number | { attr?: number },
  ): void {
    this.fillAttrRect(x, y, width, height, attrOrOptions);
  }

  draw_window(
    x: number,
    y: number,
    width: number,
    height: number,
    opts?: { attr?: number; fillTile?: number | null; fill_tile?: number | null },
  ): void {
    this.drawWindow(x, y, width, height, opts);
  }

  flatten(): [number[], number[]] {
    const flatTiles = this.tiles.flat();
    const flatAttrs = this.attributes.flat();
    return [flatTiles, flatAttrs];
  }

  blit(
    surface: Surface,
    tileset: TilemapTileset,
  ): void {
    for (let y = 0; y < this.height; y += 1) {
      for (let x = 0; x < this.width; x += 1) {
        const tileId = this.tiles[y][x];
        const tileEntry = Array.isArray(tileset) ? tileset[tileId] : tileset[tileId];
        let tile: Surface | null = null;
        if (tileEntry instanceof Surface) {
          tile = tileEntry;
        } else if (tileEntry && typeof tileEntry === "object") {
          const attr = this.attributes[y][x];
          const paletteIndex = attr & 0x07;
          tile = tileEntry[paletteIndex] ?? tileEntry[0] ?? null;
        }
        if (!tile) {
          const attr = this.attributes[y][x];
          throw new Error(
            `TilemapSurface requires a tile for id 0x${tileId.toString(16).padStart(2, "0")} attr 0x${attr
              .toString(16)
              .padStart(2, "0")} at (${x},${y})`,
          );
        }
        surface.blit(tile, [x * TILE_SIZE, y * TILE_SIZE]);
      }
    }
  }
}

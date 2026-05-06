import { z } from 'zod';
import { ScriptMemorySchema } from './script-memory';

const VRAM_TILE_BLOCK_TILE_COUNT = 0x100;
const VRAM_TILE_BYTES = 16;
const VRAM_TILE_BLOCK_BYTE_LENGTH = VRAM_TILE_BLOCK_TILE_COUNT * VRAM_TILE_BYTES;

const VRAM_TILEMAP_WIDTH = 32;
const VRAM_TILEMAP_HEIGHT = 32;
const VRAM_TILEMAP_AREA = VRAM_TILEMAP_WIDTH * VRAM_TILEMAP_HEIGHT;

interface VramAllocation {
  name: string;
  bank: number;
  kind: string;
  size: [number, ...number[]];
}

export const VRAM_ALLOCATIONS: VramAllocation[] = [
  { name: 'vTiles0', bank: 0, kind: 'tile_block', size: [VRAM_TILE_BLOCK_TILE_COUNT] },
  { name: 'vTiles1', bank: 0, kind: 'tile_block', size: [VRAM_TILE_BLOCK_TILE_COUNT] },
  { name: 'vTiles2', bank: 0, kind: 'tile_block', size: [VRAM_TILE_BLOCK_TILE_COUNT] },
  { name: 'vBGMap0', bank: 0, kind: 'bg_map', size: [VRAM_TILEMAP_WIDTH, VRAM_TILEMAP_HEIGHT] },
  { name: 'vBGMap1', bank: 0, kind: 'bg_map', size: [VRAM_TILEMAP_WIDTH, VRAM_TILEMAP_HEIGHT] },
  { name: 'vTiles3', bank: 1, kind: 'sprite_page', size: [VRAM_TILE_BLOCK_TILE_COUNT] },
  { name: 'vTiles4', bank: 1, kind: 'sprite_page', size: [VRAM_TILE_BLOCK_TILE_COUNT] },
  { name: 'vTiles5', bank: 1, kind: 'sprite_page', size: [VRAM_TILE_BLOCK_TILE_COUNT] },
  { name: 'vBGMap2', bank: 1, kind: 'bg_map', size: [VRAM_TILEMAP_WIDTH, VRAM_TILEMAP_HEIGHT] },
  { name: 'vBGMap3', bank: 1, kind: 'bg_map', size: [VRAM_TILEMAP_WIDTH, VRAM_TILEMAP_HEIGHT] },
];

const defaultTileBytes = () => new Array(VRAM_TILE_BLOCK_BYTE_LENGTH).fill(0);

export const TileBlockSchema = z.object({
  data: z.array(z.number()).default(defaultTileBytes),
}).refine(val => val.data.length === VRAM_TILE_BLOCK_BYTE_LENGTH, {
  message: `TileBlock expects ${VRAM_TILE_BLOCK_BYTE_LENGTH} bytes`,
});

export type TileBlock = z.infer<typeof TileBlockSchema>;

export class TileBlockManager {
  static readonly TILE_COUNT = VRAM_TILE_BLOCK_TILE_COUNT;
  static readonly TILE_BYTES = VRAM_TILE_BYTES;

  constructor(private tileBlock: TileBlock) {}

  writeTile(index: number, tileBytes: number[]): void {
    if (index < 0 || index >= TileBlockManager.TILE_COUNT) {
      throw new Error(`Tile index ${index} out of range (0-${TileBlockManager.TILE_COUNT - 1})`);
    }
    if (tileBytes.length !== TileBlockManager.TILE_BYTES) {
      throw new Error(`Expected ${TileBlockManager.TILE_BYTES} bytes per tile, received ${tileBytes.length}`);
    }
    const start = index * TileBlockManager.TILE_BYTES;
    for (let i = 0; i < TileBlockManager.TILE_BYTES; i++) {
      this.tileBlock.data[start + i] = tileBytes[i] & 0xff;
    }
  }

  readTile(index: number): number[] {
    if (index < 0 || index >= TileBlockManager.TILE_COUNT) {
      throw new Error(`Tile index ${index} out of range (0-${TileBlockManager.TILE_COUNT - 1})`);
    }
    const start = index * TileBlockManager.TILE_BYTES;
    const end = start + TileBlockManager.TILE_BYTES;
    return this.tileBlock.data.slice(start, end);
  }
}

export const SpritePageSchema = TileBlockSchema;
export type SpritePage = z.infer<typeof SpritePageSchema>;

const defaultTilemap = () => new Array(VRAM_TILEMAP_AREA).fill(0);

export class CGBAttribute {
  paletteIndex: number;
  vramBank: number;
  xFlip: boolean;
  yFlip: boolean;
  priority: boolean;

  static readonly ATTRIBUTE_MASKS = {
    palette: 0x07,
    vram_bank: 0x08,
    x_flip: 0x20,
    y_flip: 0x40,
    priority: 0x80,
  };

  constructor({
    paletteIndex = 0,
    vramBank = 0,
    xFlip = false,
    yFlip = false,
    priority = false,
  }: {
    paletteIndex?: number;
    vramBank?: number;
    xFlip?: boolean;
    yFlip?: boolean;
    priority?: boolean;
  } = {}) {
    this.paletteIndex = paletteIndex;
    this.vramBank = vramBank;
    this.xFlip = xFlip;
    this.yFlip = yFlip;
    this.priority = priority;
  }

  static fromByte(value: number): CGBAttribute {
    value &= 0xff;
    return new CGBAttribute({
      paletteIndex: value & CGBAttribute.ATTRIBUTE_MASKS.palette,
      vramBank: (value & CGBAttribute.ATTRIBUTE_MASKS.vram_bank) >> 3,
      xFlip: !!(value & CGBAttribute.ATTRIBUTE_MASKS.x_flip),
      yFlip: !!(value & CGBAttribute.ATTRIBUTE_MASKS.y_flip),
      priority: !!(value & CGBAttribute.ATTRIBUTE_MASKS.priority),
    });
  }

  toByte(): number {
    let result = this.paletteIndex & CGBAttribute.ATTRIBUTE_MASKS.palette;
    result |= (this.vramBank & 0x01) << 3;
    if (this.xFlip) result |= CGBAttribute.ATTRIBUTE_MASKS.x_flip;
    if (this.yFlip) result |= CGBAttribute.ATTRIBUTE_MASKS.y_flip;
    if (this.priority) result |= CGBAttribute.ATTRIBUTE_MASKS.priority;
    return result;
  }

  withUpdates(updates: {
    paletteIndex?: number;
    vramBank?: number;
    xFlip?: boolean;
    yFlip?: boolean;
    priority?: boolean;
  }): CGBAttribute {
    return new CGBAttribute({
      paletteIndex: updates.paletteIndex !== undefined ? updates.paletteIndex & CGBAttribute.ATTRIBUTE_MASKS.palette : this.paletteIndex,
      vramBank: updates.vramBank !== undefined ? updates.vramBank & 0x01 : this.vramBank,
      xFlip: updates.xFlip !== undefined ? updates.xFlip : this.xFlip,
      yFlip: updates.yFlip !== undefined ? updates.yFlip : this.yFlip,
      priority: updates.priority !== undefined ? updates.priority : this.priority,
    });
  }
}

export const BackgroundMapSchema = z.object({
  width: z.number().default(VRAM_TILEMAP_WIDTH),
  height: z.number().default(VRAM_TILEMAP_HEIGHT),
  tiles: z.array(z.number()).default(defaultTilemap),
  attributes: z.array(z.number()).default(defaultTilemap),
}).refine(val => val.tiles.length === VRAM_TILEMAP_AREA && val.attributes.length === VRAM_TILEMAP_AREA, {
  message: `Background maps require ${VRAM_TILEMAP_AREA} entries`,
});

export type BackgroundMap = z.infer<typeof BackgroundMapSchema>;

export class BackgroundMapManager {
  constructor(private bgMap: BackgroundMap) {}

  private getIndex(x: number, y: number): number | null {
    if (x < 0 || x >= this.bgMap.width || y < 0 || y >= this.bgMap.height) {
      return null;
    }
    return y * this.bgMap.width + x;
  }

  clear(): void {
    this.bgMap.tiles = defaultTilemap();
    this.bgMap.attributes = defaultTilemap();
  }

  private static normalizeAttribute(attr: number | CGBAttribute): number {
    return attr instanceof CGBAttribute ? attr.toByte() : attr & 0xff;
  }

  writeTile(x: number, y: number, tile: number, attr: number | CGBAttribute = 0): void {
    const position = this.getIndex(x, y);
    if (position === null) return;
    this.bgMap.tiles[position] = tile & 0xff;
    this.bgMap.attributes[position] = BackgroundMapManager.normalizeAttribute(attr);
  }

  getAttributeEntry(x: number, y: number): CGBAttribute {
    const position = this.getIndex(x, y);
    if (position === null) {
      throw new Error(`Tile (${x}, ${y}) outside ${this.bgMap.width}x${this.bgMap.height} grid`);
    }
    return CGBAttribute.fromByte(this.bgMap.attributes[position]);
  }

  setAttributeEntry(x: number, y: number, attribute: CGBAttribute): void {
    const position = this.getIndex(x, y);
    if (position === null) return;
    this.bgMap.attributes[position] = attribute.toByte();
  }

  updateAttributeEntry(x: number, y: number, updates: {
    paletteIndex?: number;
    vramBank?: number;
    xFlip?: boolean;
    yFlip?: boolean;
    priority?: boolean;
  }): void {
    const entry = this.getAttributeEntry(x, y);
    const newEntry = entry.withUpdates(updates);
    this.setAttributeEntry(x, y, newEntry);
  }

  loadRegion(
    width: number,
    height: number,
    tiles: number[],
    attributes?: number[],
    { originX = 0, originY = 0 }: { originX?: number; originY?: number } = {}
  ): void {
    const attrList = attributes || new Array(tiles.length).fill(0);
    if (tiles.length !== width * height) {
      throw new Error(`Expected ${width * height} tile entries, received ${tiles.length}`);
    }
    if (attrList.length !== width * height) {
      throw new Error(`Expected ${width * height} attribute entries, received ${attrList.length}`);
    }
    for (let row = 0; row < height; row++) {
      for (let col = 0; col < width; col++) {
        const index = row * width + col;
        this.writeTile(originX + col, originY + row, tiles[index], attrList[index]);
      }
    }
  }
}

const defaultBank0 = () => VRAMBankSchema.parse({
  tile_blocks: {
    vTiles0: TileBlockSchema.parse({}),
    vTiles1: TileBlockSchema.parse({}),
    vTiles2: TileBlockSchema.parse({}),
  },
  sprite_pages: {},
  bg_maps: {
    vBGMap0: BackgroundMapSchema.parse({}),
    vBGMap1: BackgroundMapSchema.parse({}),
  },
});

const defaultBank1 = () => VRAMBankSchema.parse({
  tile_blocks: {},
  sprite_pages: {
    vTiles3: SpritePageSchema.parse({}),
    vTiles4: SpritePageSchema.parse({}),
    vTiles5: SpritePageSchema.parse({}),
  },
  bg_maps: {
    vBGMap2: BackgroundMapSchema.parse({}),
    vBGMap3: BackgroundMapSchema.parse({}),
  },
});

export const VRAMBankSchema = z.object({
  tile_blocks: z.record(z.string(), TileBlockSchema).default({}),
  sprite_pages: z.record(z.string(), SpritePageSchema).default({}),
  bg_maps: z.record(z.string(), BackgroundMapSchema).default({}),
});

export type VRAMBank = z.infer<typeof VRAMBankSchema>;

export const VRAMSchema = z.object({
  bank0: VRAMBankSchema.default(defaultBank0),
  bank1: VRAMBankSchema.default(defaultBank1),
  script_memory: ScriptMemorySchema,
});

export type VRAM = z.infer<typeof VRAMSchema>;

export class VRAMManager {
  constructor(private vram: VRAM) {}

  resolveBgMap(name: string): BackgroundMap {
    for (const bank of [this.vram.bank0, this.vram.bank1]) {
      if (name in bank.bg_maps) {
        return bank.bg_maps[name];
      }
    }
    throw new Error(`Unknown background map '${name}'`);
  }

  resolveTileBlock(name: string): TileBlock {
    for (const bank of [this.vram.bank0, this.vram.bank1]) {
      if (name in bank.tile_blocks) {
        return bank.tile_blocks[name];
      }
      if (name in bank.sprite_pages) {
        return bank.sprite_pages[name];
      }
    }
    throw new Error(`Unknown tile block '${name}'`);
  }

  writeBgRegion(
    name: string,
    width: number,
    height: number,
    tiles: number[],
    attributes?: number[],
    { originX = 0, originY = 0 }: { originX?: number; originY?: number } = {}
  ): void {
    const bgMap = this.resolveBgMap(name);
    const bgMapManager = new BackgroundMapManager(bgMap);
    bgMapManager.loadRegion(width, height, tiles, attributes, { originX, originY });
  }
}

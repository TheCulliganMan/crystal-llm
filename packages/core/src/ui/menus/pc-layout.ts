// ASM mapping: pokecrystal_disassembly/engine/menus/bills_pc.asm layout constants.
const TILE_SIZE = 8;

export class PCTileRect {
  constructor(
    public readonly x: number,
    public readonly y: number,
    public readonly width: number,
    public readonly height: number
  ) {}

  toPixels(): [number, number] {
    return [this.x * TILE_SIZE, this.y * TILE_SIZE];
  }
}

export class PCLayout {
  constructor(
    public readonly message: PCTileRect,
    public readonly grid: PCTileRect,
    public readonly party: PCTileRect,
    public readonly actions: PCTileRect,
    public readonly depositPrompt: PCTileRect
  ) {}
}

export const PC_LAYOUT = new PCLayout(
  new PCTileRect(0, 0, 14, 4),
  new PCTileRect(0, 4, 14, 9),
  new PCTileRect(14, 1, 6, 8),
  new PCTileRect(9, 4, 11, 10),
  new PCTileRect(2, 13, 10, 5)
);

export class TileRegion {
  constructor(
    public readonly x: number,
    public readonly y: number,
    public readonly width: number,
    public readonly height: number
  ) {}

  toPixels(): [number, number] {
    return [this.x * TILE_SIZE, this.y * TILE_SIZE];
  }
}

export const HEADER_REGION = new TileRegion(8, 0, 12, 3);
export const GRID_REGION = new TileRegion(8, 2, 12, 12);
export const BOTTOM_PROMPT_REGION = new TileRegion(0, 15, 20, 3);
export const INFO_CLEAR_REGION = new TileRegion(0, 0, 8, 15);
export const INFO_PIC_ORIGIN = { x: 1, y: 4 } as const;
export const INFO_LEVEL_ORIGIN = { x: 1, y: 12 } as const;
export const INFO_GENDER_ORIGIN = { x: 5, y: 12 } as const;
export const INFO_ITEM_ORIGIN = { x: 7, y: 12 } as const;
export const INFO_NAME_ORIGIN = { x: 1, y: 14 } as const;

export class PCAttrBand {
  constructor(public readonly region: TileRegion, public readonly paletteId: number) {}
}

export const BILLS_PC_ATTR_BANDS: readonly PCAttrBand[] = [
  new PCAttrBand(new TileRegion(1, 4, 7, 7), 0x01),
];

export function applyBillsPcAttrBands(tilemap: {
  width: number;
  height: number;
  setAttr: (x: number, y: number, attr: number) => void;
}): void {
  for (const band of BILLS_PC_ATTR_BANDS) {
    for (let row = band.region.y; row < band.region.y + band.region.height; row++) {
      for (let col = band.region.x; col < band.region.x + band.region.width; col++) {
        if (row >= 0 && row < tilemap.height && col >= 0 && col < tilemap.width) {
          tilemap.setAttr(col, row, band.paletteId);
        }
      }
    }
  }
}

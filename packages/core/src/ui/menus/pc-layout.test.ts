import {
  BILLS_PC_ATTR_BANDS,
  BOTTOM_PROMPT_REGION,
  GRID_REGION,
  HEADER_REGION,
  INFO_CLEAR_REGION,
  INFO_GENDER_ORIGIN,
  INFO_ITEM_ORIGIN,
  INFO_LEVEL_ORIGIN,
  INFO_NAME_ORIGIN,
  INFO_PIC_ORIGIN,
  PC_LAYOUT,
  applyBillsPcAttrBands,
} from "./pc-layout";

describe("PC layout ASM parity", () => {
  it("pins Bill's PC window regions to tile coordinates used by the renderer", () => {
    expect(HEADER_REGION).toMatchObject({ x: 8, y: 0, width: 12, height: 3 });
    expect(GRID_REGION).toMatchObject({ x: 8, y: 2, width: 12, height: 12 });
    expect(BOTTOM_PROMPT_REGION).toMatchObject({ x: 0, y: 15, width: 20, height: 3 });
    expect(INFO_CLEAR_REGION).toMatchObject({ x: 0, y: 0, width: 8, height: 15 });
    expect(INFO_PIC_ORIGIN).toEqual({ x: 1, y: 4 });
    expect(INFO_LEVEL_ORIGIN).toEqual({ x: 1, y: 12 });
    expect(INFO_GENDER_ORIGIN).toEqual({ x: 5, y: 12 });
    expect(INFO_ITEM_ORIGIN).toEqual({ x: 7, y: 12 });
    expect(INFO_NAME_ORIGIN).toEqual({ x: 1, y: 14 });
    expect(PC_LAYOUT.actions).toMatchObject({ x: 9, y: 4, width: 11, height: 10 });
  });

  it("applies the orange wallpaper attribute band only to the 7x7 Bill's PC mural", () => {
    const attrs = Array.from({ length: 18 }, () => Array.from({ length: 20 }, () => 0));
    const tilemap = {
      width: 20,
      height: 18,
      setAttr: (x: number, y: number, attr: number) => {
        attrs[y][x] = attr;
      },
    };

    expect(BILLS_PC_ATTR_BANDS).toHaveLength(1);
    expect(BILLS_PC_ATTR_BANDS[0].region).toMatchObject({ x: 1, y: 4, width: 7, height: 7 });
    expect(BILLS_PC_ATTR_BANDS[0].paletteId).toBe(0x01);

    applyBillsPcAttrBands(tilemap);

    for (let y = 0; y < 18; y += 1) {
      for (let x = 0; x < 20; x += 1) {
        const inWallpaperBand = x >= 1 && x < 8 && y >= 4 && y < 11;
        expect(attrs[y][x]).toBe(inWallpaperBand ? 0x01 : 0x00);
      }
    }
  });
});

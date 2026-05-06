import { DexMode } from "@pokecrystal/core/core/enums/pokedex";
import {
  CURSOR_NEW,
  CURSOR_NEW_SEARCH_RESULTS,
  CURSOR_OLD,
  CURSOR_OLD_TOP,
  CURSOR_ROW_HEIGHT,
  getPokedexScrollbarOAMEntry,
  OAM_PAL_7,
  OAM_XFLIP,
  OAM_YFLIP,
  PokedexCursorOAM,
  SPRITE_X_OFFSET,
  SPRITE_Y_OFFSET,
} from "@pokecrystal/core/ui/menus/pokedex-cursor";

describe("Pokedex cursor ASM parity", () => {
  it("keeps the ASM cursor sprite tables byte-for-byte", () => {
    expect(CURSOR_NEW).toEqual([
      [9, 3, -1, 3, 0x30, 0],
      [9, 2, -1, 3, 0x31, 0],
      [10, 2, -1, 3, 0x32, 0],
      [11, 2, -1, 3, 0x32, 0],
      [12, 2, -1, 3, 0x33, 0],
      [16, 2, 0, 3, 0x33, OAM_XFLIP],
      [17, 2, 0, 3, 0x32, OAM_XFLIP],
      [18, 2, 0, 3, 0x32, OAM_XFLIP],
      [19, 2, 0, 3, 0x31, OAM_XFLIP],
      [19, 3, 0, 3, 0x30, OAM_XFLIP],
      [9, 4, -1, 3, 0x30, OAM_YFLIP],
      [9, 5, -1, 3, 0x31, OAM_YFLIP],
      [10, 5, -1, 3, 0x32, OAM_YFLIP],
      [11, 5, -1, 3, 0x32, OAM_YFLIP],
      [12, 5, -1, 3, 0x33, OAM_YFLIP],
      [16, 5, 0, 3, 0x33, OAM_XFLIP | OAM_YFLIP],
      [17, 5, 0, 3, 0x32, OAM_XFLIP | OAM_YFLIP],
      [18, 5, 0, 3, 0x32, OAM_XFLIP | OAM_YFLIP],
      [19, 5, 0, 3, 0x31, OAM_XFLIP | OAM_YFLIP],
      [19, 4, 0, 3, 0x30, OAM_XFLIP | OAM_YFLIP],
    ]);

    expect(CURSOR_OLD).toHaveLength(24);
    expect(CURSOR_OLD_TOP).toEqual([
      [9, 3, -1, 0, 0x30, 0],
      [9, 2, -1, 0, 0x34, 0],
      [10, 2, -1, 0, 0x35, 0],
      [11, 2, -1, 0, 0x35, 0],
      [12, 2, -1, 0, 0x35, 0],
      [13, 2, -1, 0, 0x36, 0],
      [16, 2, -2, 0, 0x36, OAM_XFLIP],
      [17, 2, -2, 0, 0x35, OAM_XFLIP],
      [18, 2, -2, 0, 0x35, OAM_XFLIP],
      [19, 2, -2, 0, 0x35, OAM_XFLIP],
      [20, 2, -2, 0, 0x34, OAM_XFLIP],
      [20, 3, -2, 0, 0x30, OAM_XFLIP],
      [9, 4, -1, 0, 0x30, OAM_YFLIP],
      [9, 5, -1, 0, 0x31, OAM_YFLIP],
      [10, 5, -1, 0, 0x32, OAM_YFLIP],
      [11, 5, -1, 0, 0x32, OAM_YFLIP],
      [12, 5, -1, 0, 0x32, OAM_YFLIP],
      [13, 5, -1, 0, 0x33, OAM_YFLIP],
      [16, 5, -2, 0, 0x33, OAM_XFLIP | OAM_YFLIP],
      [17, 5, -2, 0, 0x32, OAM_XFLIP | OAM_YFLIP],
      [18, 5, -2, 0, 0x32, OAM_XFLIP | OAM_YFLIP],
      [19, 5, -2, 0, 0x32, OAM_XFLIP | OAM_YFLIP],
      [20, 5, -2, 0, 0x31, OAM_XFLIP | OAM_YFLIP],
      [20, 4, -2, 0, 0x30, OAM_XFLIP | OAM_YFLIP],
    ]);
    expect(CURSOR_NEW_SEARCH_RESULTS).toHaveLength(24);
  });

  it("converts cursor tables to screen OAM coordinates with ASM offsets and palette", () => {
    const oam = new PokedexCursorOAM();
    oam.update(DexMode.NEW, 3, 1, 7);

    expect(oam.entries).toHaveLength(CURSOR_NEW.length);
    expect(oam.entries[0]).toMatchObject({
      x: 9 * 8 - 1 - SPRITE_X_OFFSET,
      y: 3 * 8 + 3 + 2 * CURSOR_ROW_HEIGHT - SPRITE_Y_OFFSET,
      tileId: 0x30,
      attributes: OAM_PAL_7,
    });
  });

  it("uses the ASM scrollbar division and max-position branch", () => {
    expect(getPokedexScrollbarOAMEntry(0, 0, 251)).toMatchObject({
      x: 161,
      y: 20,
      tileId: 0x0f,
      attributes: 0,
    });
    expect(getPokedexScrollbarOAMEntry(10, 0, 251)?.y).toBe(20 + Math.floor((10 * 121) / 251));
    expect(getPokedexScrollbarOAMEntry(250, 0, 251)?.y).toBe(141);
    expect(getPokedexScrollbarOAMEntry(0, 0, 0)).toBeNull();
  });
});

// ASM: engine/pokedex/pokedex.asm cursor sprite layouts.
import { DexMode } from "../../core/enums/pokedex";
import { TILE_SIZE } from "../../engine/world/tile";

export type CursorSprite = [number, number, number, number, number, number];
export type PokedexCursorVariant = "main" | "search_results";

const OAM_XFLIP = 1 << 5;
const OAM_YFLIP = 1 << 6;
const SPRITE_X_OFFSET = 8;
const SPRITE_Y_OFFSET = 16;
const CURSOR_ROW_HEIGHT = TILE_SIZE * 2;
const SCROLLBAR_TILE_ID = 0x0f;

const OAM_PAL_7 = 0x07;

const CURSOR_NEW: CursorSprite[] = [
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
];

const CURSOR_NEW_SEARCH_RESULTS: CursorSprite[] = [
  [9, 3, -1, 3, 0x30, 0],
  [9, 2, -1, 3, 0x31, 0],
  [10, 2, -1, 3, 0x32, 0],
  [11, 2, -1, 3, 0x32, 0],
  [12, 2, -1, 3, 0x32, 0],
  [13, 2, -1, 3, 0x33, 0],
  [16, 2, -2, 3, 0x33, OAM_XFLIP],
  [17, 2, -2, 3, 0x32, OAM_XFLIP],
  [18, 2, -2, 3, 0x32, OAM_XFLIP],
  [19, 2, -2, 3, 0x32, OAM_XFLIP],
  [20, 2, -2, 3, 0x31, OAM_XFLIP],
  [20, 3, -2, 3, 0x30, OAM_XFLIP],
  [9, 4, -1, 3, 0x30, OAM_YFLIP],
  [9, 5, -1, 3, 0x31, OAM_YFLIP],
  [10, 5, -1, 3, 0x32, OAM_YFLIP],
  [11, 5, -1, 3, 0x32, OAM_YFLIP],
  [12, 5, -1, 3, 0x32, OAM_YFLIP],
  [13, 5, -1, 3, 0x33, OAM_YFLIP],
  [16, 5, -2, 3, 0x33, OAM_XFLIP | OAM_YFLIP],
  [17, 5, -2, 3, 0x32, OAM_XFLIP | OAM_YFLIP],
  [18, 5, -2, 3, 0x32, OAM_XFLIP | OAM_YFLIP],
  [19, 5, -2, 3, 0x32, OAM_XFLIP | OAM_YFLIP],
  [20, 5, -2, 3, 0x31, OAM_XFLIP | OAM_YFLIP],
  [20, 4, -2, 3, 0x30, OAM_XFLIP | OAM_YFLIP],
];

const CURSOR_OLD: CursorSprite[] = [
  [9, 3, -1, 0, 0x30, 0],
  [9, 2, -1, 0, 0x31, 0],
  [10, 2, -1, 0, 0x32, 0],
  [11, 2, -1, 0, 0x32, 0],
  [12, 2, -1, 0, 0x32, 0],
  [13, 2, -1, 0, 0x33, 0],
  [16, 2, -2, 0, 0x33, OAM_XFLIP],
  [17, 2, -2, 0, 0x32, OAM_XFLIP],
  [18, 2, -2, 0, 0x32, OAM_XFLIP],
  [19, 2, -2, 0, 0x32, OAM_XFLIP],
  [20, 2, -2, 0, 0x31, OAM_XFLIP],
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
];

const CURSOR_OLD_TOP: CursorSprite[] = [
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
];

export const getPokedexScrollbarOAMEntry = (
  cursorIndex: number,
  scrollOffset: number,
  listingEnd: number,
): CursorOAMEntry | null => {
  if (listingEnd <= 0) {
    return null;
  }
  const absoluteIndex = Math.max(0, cursorIndex + scrollOffset);
  const offset =
    absoluteIndex >= listingEnd - 1
      ? 121
      : Math.floor((absoluteIndex * 121) / listingEnd);
  return new CursorOAMEntry(161, 20 + offset, SCROLLBAR_TILE_ID, 0);
};

export class CursorOAMEntry {
  constructor(
    public readonly x: number,
    public readonly y: number,
    public readonly tileId: number,
    public readonly attributes: number,
  ) {}
}

export class PokedexCursorOAM {
  public entries: CursorOAMEntry[] = [];

  update(
    dexMode: DexMode,
    cursorIndex: number,
    scrollOffset: number,
    listingHeight: number,
    variant: PokedexCursorVariant = "main"
  ): void {
    const row = cursorIndex - scrollOffset;
    if (row < 0 || row >= listingHeight) {
      this.entries = [];
      return;
    }

    const sprites =
      dexMode === DexMode.OLD
        ? cursorIndex === 0
          ? CURSOR_OLD_TOP
          : CURSOR_OLD
        : variant === "search_results"
          ? CURSOR_NEW_SEARCH_RESULTS
          : CURSOR_NEW;

    const rowOffset = row * CURSOR_ROW_HEIGHT;
    this.entries = sprites.map(
      ([xTile, yTile, xOffset, yOffset, tileId, attr]) =>
        new CursorOAMEntry(
          xTile * TILE_SIZE + xOffset - SPRITE_X_OFFSET,
          yTile * TILE_SIZE + yOffset + rowOffset - SPRITE_Y_OFFSET,
          tileId,
          attr | OAM_PAL_7,
        ),
    );
  }

  get count(): number {
    return this.entries.length;
  }
}

export {
  CURSOR_NEW,
  CURSOR_NEW_SEARCH_RESULTS,
  CURSOR_OLD,
  CURSOR_OLD_TOP,
  OAM_XFLIP,
  OAM_YFLIP,
  OAM_PAL_7,
  SPRITE_X_OFFSET,
  SPRITE_Y_OFFSET,
  CURSOR_ROW_HEIGHT,
  SCROLLBAR_TILE_ID,
};

import {
  SLOT_ICON_SPRITE_TILES,
  SLOT_ICON_Y_STEP_TILES,
  SLOT_STATUS_LINE_TILE,
  slotIconRowStartTile,
  slotIconTileIndices,
} from "@/app/game-corner/slot-icon-layout";

describe("slot icon layout", () => {
  it("keeps icon tiles in row-major order", () => {
    expect(slotIconTileIndices(8)).toEqual([8, 9, 10, 11]);
  });

  it("keeps the three icon rows above the status line", () => {
    const thirdRowStart = slotIconRowStartTile(2);
    const thirdRowEnd = thirdRowStart + SLOT_ICON_SPRITE_TILES - 1;
    expect(SLOT_ICON_Y_STEP_TILES).toBe(2);
    expect(thirdRowEnd).toBeLessThan(SLOT_STATUS_LINE_TILE);
  });
});


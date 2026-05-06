import {
  SLOT_REEL_X_OAM_TILES,
  SLOT_REEL_X_TILES,
  SLOT_REEL_Y_OAM_TILES,
  SLOT_REEL_Y_TILES,
} from "@/app/game-corner/slot-reel-layout";

describe("slot reel layout", () => {
  it("converts ASM OAM x coordinates into screen tile positions", () => {
    expect(SLOT_REEL_X_OAM_TILES).toEqual([6, 10, 14]);
    expect(SLOT_REEL_X_TILES).toEqual([5, 9, 13]);
  });

  it("converts ASM OAM y coordinates into screen tile positions", () => {
    expect(SLOT_REEL_Y_OAM_TILES).toEqual([6, 8, 10]);
    expect(SLOT_REEL_Y_TILES).toEqual([4, 6, 8]);
  });
});

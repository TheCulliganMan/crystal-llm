import { getLedgeJumpOffset, LEDGE_JUMP_OFFSETS } from "./jump-offsets";

describe("getLedgeJumpOffset", () => {
  it("mirrors UpdateJumpPosition indexing from jump height", () => {
    const offsets = LEDGE_JUMP_OFFSETS.map((_, index) => getLedgeJumpOffset(index * 2));

    expect(offsets).toEqual(LEDGE_JUMP_OFFSETS);
  });

  it("supports ASM stride check and maps final landing height to the terminal offset", () => {
    const totalDistancePx = LEDGE_JUMP_OFFSETS.length * 2;
    expect(getLedgeJumpOffset(totalDistancePx, totalDistancePx)).toBe(
      LEDGE_JUMP_OFFSETS[LEDGE_JUMP_OFFSETS.length - 1],
    );
  });

  it("throws when ledge jump total distance is out of range", () => {
    expect(() => getLedgeJumpOffset(0, 8)).toThrow("Ledge jump total distance mismatch");
  });
});

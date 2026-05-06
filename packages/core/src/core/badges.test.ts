import {
  NUM_BADGES,
  countOwnedBadgesAsm,
  hasOwnedBadgeAsm,
  johtoBadgeMaskAsm,
  assertAsmBadgeBanks,
  setOwnedBadgeByEngineFlagAsm,
} from "./badges";

const makeBadgeState = (johto: boolean[], kanto: boolean[]) => ({ johto, kanto });

describe("ASM badge helpers", () => {
  it("counts Johto + Kanto badges from fixed-size banks", () => {
    const count = countOwnedBadgesAsm(
      makeBadgeState(
        [true, false, true, false, false, false, true, false],
        [false, true, false, false, false, false, false, true]
      ),
      "test count"
    );
    expect(count).toBe(5);
  });

  it("resolves badge ownership by ASM badge id order", () => {
    const badges = makeBadgeState(
      [true, false, false, false, false, false, false, false],
      [false, false, false, false, false, false, false, true]
    );
    expect(hasOwnedBadgeAsm(badges, 0, "test has")).toBe(true);
    expect(hasOwnedBadgeAsm(badges, 15, "test has")).toBe(true);
    expect(hasOwnedBadgeAsm(badges, 1, "test has")).toBe(false);
  });

  it("builds Johto badge mask from badge bits", () => {
    const mask = johtoBadgeMaskAsm([true, false, true, false, false, true, false, false], "test mask");
    expect(mask).toBe((1 << 0) | (1 << 2) | (1 << 5));
  });

  it("throws on non-ASM badge bank lengths", () => {
    expect(() =>
      assertAsmBadgeBanks(
        makeBadgeState([true], Array(8).fill(false)),
        "test length"
      )
    ).toThrow("must contain exactly 8");
  });

  it("throws on out-of-range badge ids", () => {
    expect(() => hasOwnedBadgeAsm(makeBadgeState(Array(8).fill(false), Array(8).fill(false)), NUM_BADGES, "test range")).toThrow(
      "out of ASM range"
    );
  });

  it("updates badge banks by engine flag order", () => {
    const badges = makeBadgeState(Array(8).fill(false), Array(8).fill(false));

    expect(setOwnedBadgeByEngineFlagAsm(badges, "ENGINE_RISINGBADGE", true, "test set")).toBe(true);
    expect(setOwnedBadgeByEngineFlagAsm(badges, "ENGINE_BOULDERBADGE", true, "test set")).toBe(true);
    expect(setOwnedBadgeByEngineFlagAsm(badges, "ENGINE_NOT_A_BADGE", true, "test set")).toBe(false);

    expect(badges.johto[7]).toBe(true);
    expect(badges.kanto[0]).toBe(true);
    expect(countOwnedBadgesAsm(badges, "test count")).toBe(2);
  });
});

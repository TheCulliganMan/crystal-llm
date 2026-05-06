import { applyEloRating, DEFAULT_ARENA_ELO } from "./elo";

describe("applyEloRating", () => {
  it("increases winner rating and decreases loser rating", () => {
    const result = applyEloRating(DEFAULT_ARENA_ELO, DEFAULT_ARENA_ELO, "a");

    expect(result.nextRatingA).toBeGreaterThan(DEFAULT_ARENA_ELO);
    expect(result.nextRatingB).toBeLessThan(DEFAULT_ARENA_ELO);
    expect(result.nextRatingA - DEFAULT_ARENA_ELO).toBe(-(result.nextRatingB - DEFAULT_ARENA_ELO));
  });

  it("gives a smaller gain when favorite wins", () => {
    const favoriteWin = applyEloRating(1300, 900, "a");
    const underdogWin = applyEloRating(1300, 900, "b");

    expect(favoriteWin.nextRatingA - 1300).toBeLessThan(underdogWin.nextRatingB - 900);
  });

  it("supports draws", () => {
    const result = applyEloRating(1000, 1000, "draw");
    expect(result.nextRatingA).toBe(1000);
    expect(result.nextRatingB).toBe(1000);
  });

  it("enforces a rating floor", () => {
    const result = applyEloRating(100, 2400, "b", 96);
    expect(result.nextRatingA).toBeGreaterThanOrEqual(100);
  });

  it("rejects invalid k factor", () => {
    expect(() => applyEloRating(1000, 1000, "a", 0)).toThrow("kFactor must be positive");
  });
});

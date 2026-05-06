import { calculateExperience, GrowthRate } from './experience';

describe('calculateExperience', () => {
  it('should calculate the experience for a given level and growth rate', () => {
    expect(calculateExperience(GrowthRate.GROWTH_MEDIUM_SLOW, 1)).toBe(-54);
    expect(calculateExperience(GrowthRate.GROWTH_SLIGHTLY_FAST, 1)).toBe(-20);
    expect(calculateExperience(GrowthRate.GROWTH_SLIGHTLY_FAST, 10)).toBe(1720);
    expect(calculateExperience(GrowthRate.GROWTH_SLIGHTLY_FAST, 50)).toBe(118720);
    expect(calculateExperience(GrowthRate.GROWTH_SLIGHTLY_SLOW, 1)).toBe(-50);
    expect(calculateExperience(GrowthRate.GROWTH_SLIGHTLY_SLOW, 10)).toBe(2680);
    expect(calculateExperience(GrowthRate.GROWTH_SLIGHTLY_SLOW, 50)).toBe(143680);
  });
});

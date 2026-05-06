export enum GrowthRate {
  GROWTH_MEDIUM_FAST,
  GROWTH_SLIGHTLY_FAST,
  GROWTH_SLIGHTLY_SLOW,
  GROWTH_MEDIUM_SLOW,
  GROWTH_FAST,
  GROWTH_SLOW,
}

export function calculateExperience(growthRate: GrowthRate, level: number): number {
  const n = level;
  let experience: number;

  switch (growthRate) {
    case GrowthRate.GROWTH_MEDIUM_FAST:
      // n**3
      experience = n * n * n;
      break;
    case GrowthRate.GROWTH_SLIGHTLY_FAST:
      // (3 * n**3) / 4 + (10 * n**2) - 30
      const n2_sf = n * n;
      const n3_sf = n2_sf * n;
      experience = Math.floor((3 * n3_sf) / 4) + 10 * n2_sf - 30;
      break;
    case GrowthRate.GROWTH_SLIGHTLY_SLOW:
      // (3 * n**3) / 4 + (20 * n**2) - 70
      const n2_ss = n * n;
      const n3_ss = n2_ss * n;
      experience = Math.floor((3 * n3_ss) / 4) + 20 * n2_ss - 70;
      break;
    case GrowthRate.GROWTH_MEDIUM_SLOW:
      // (6/5)n³ - 15n² + 100n - 140
      const n2_ms = n * n;
      const n3_ms = n2_ms * n;
      const term1 = Math.floor((6 * n3_ms) / 5);
      const term2 = 15 * n2_ms;
      const term3 = 100 * n;
      experience = term1 - term2 + term3 - 140;
      break;
    case GrowthRate.GROWTH_FAST:
      // (4 * n**3) / 5
      experience = Math.floor((4 * n * n * n) / 5);
      break;
    case GrowthRate.GROWTH_SLOW:
      // (5 * n**3) / 4
      experience = Math.floor((5 * n * n * n) / 4);
      break;
    default:
      throw new Error(`Unknown growth rate: ${growthRate}`);
  }

  return experience;
}

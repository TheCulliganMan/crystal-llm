
import { GrowthRate } from "../../core/enums";

export function getExperienceForLevel(level: number, growthRate: GrowthRate): number {
    switch (growthRate) {
        case GrowthRate.GROWTH_FAST:
            return Math.floor(4 * Math.pow(level, 3) / 5);
        case GrowthRate.GROWTH_MEDIUM_FAST:
            return Math.pow(level, 3);
        case GrowthRate.GROWTH_MEDIUM_SLOW:
            return Math.floor(6/5 * Math.pow(level, 3) - 15 * Math.pow(level, 2) + 100 * level - 140);
        case GrowthRate.GROWTH_SLOW:
            return Math.floor(5 * Math.pow(level, 3) / 4);
        default:
            throw new Error(`Unknown growth rate: ${growthRate}`);
    }
}

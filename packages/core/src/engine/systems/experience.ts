
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
        case GrowthRate.GROWTH_ERRATIC:
            if (level <= 50) {
                return Math.floor(Math.pow(level, 3) * (100 - level) / 50);
            } else if (level <= 68) {
                return Math.floor(Math.pow(level, 3) * (150 - level) / 100);
            } else if (level <= 98) {
                return Math.floor(Math.pow(level, 3) * ((1911 - 10 * level) / 3) / 500);
            } else {
                return Math.floor(Math.pow(level, 3) * (160 - level) / 100);
            }
        case GrowthRate.GROWTH_FLUCTUATING:
            if (level <= 15) {
                return Math.floor(Math.pow(level, 3) * (((level + 1) / 3) + 24) / 50);
            } else if (level <= 36) {
                return Math.floor(Math.pow(level, 3) * (level + 14) / 50);
            } else {
                return Math.floor(Math.pow(level, 3) * ((level / 2) + 32) / 50);
            }
        default:
            throw new Error(`Unknown growth rate: ${growthRate}`);
    }
}

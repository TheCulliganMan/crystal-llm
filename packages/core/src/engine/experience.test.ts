import { calculateExperience } from './experience';
import { GrowthRate } from '@pokecrystal/core/core/enums/pokemon';

describe('calculateExperience', () => {
    it('should calculate the correct experience for GROWTH_MEDIUM_FAST', () => {
        expect(calculateExperience(GrowthRate.GROWTH_MEDIUM_FAST, 1)).toBe(1);
        expect(calculateExperience(GrowthRate.GROWTH_MEDIUM_FAST, 50)).toBe(125000);
        expect(calculateExperience(GrowthRate.GROWTH_MEDIUM_FAST, 100)).toBe(1000000);
    });

    it('should calculate the correct experience for GROWTH_SLIGHTLY_FAST', () => {
        expect(calculateExperience(GrowthRate.GROWTH_SLIGHTLY_FAST, 1)).toBe(-20);
        expect(calculateExperience(GrowthRate.GROWTH_SLIGHTLY_FAST, 50)).toBe(118720);
        expect(calculateExperience(GrowthRate.GROWTH_SLIGHTLY_FAST, 100)).toBe(849970);
    });

    it('should calculate the correct experience for GROWTH_SLIGHTLY_SLOW', () => {
        expect(calculateExperience(GrowthRate.GROWTH_SLIGHTLY_SLOW, 1)).toBe(-50);
        expect(calculateExperience(GrowthRate.GROWTH_SLIGHTLY_SLOW, 50)).toBe(143680);
        expect(calculateExperience(GrowthRate.GROWTH_SLIGHTLY_SLOW, 100)).toBe(949930);
    });

    it('should calculate the correct experience for GROWTH_MEDIUM_SLOW', () => {
        expect(calculateExperience(GrowthRate.GROWTH_MEDIUM_SLOW, 1)).toBe(-54);
        expect(calculateExperience(GrowthRate.GROWTH_MEDIUM_SLOW, 50)).toBe(117360);
        expect(calculateExperience(GrowthRate.GROWTH_MEDIUM_SLOW, 100)).toBe(1059860);
    });

    it('should calculate the correct experience for GROWTH_FAST', () => {
        expect(calculateExperience(GrowthRate.GROWTH_FAST, 1)).toBe(0);
        expect(calculateExperience(GrowthRate.GROWTH_FAST, 50)).toBe(100000);
        expect(calculateExperience(GrowthRate.GROWTH_FAST, 100)).toBe(800000);
    });

    it('should calculate the correct experience for GROWTH_SLOW', () => {
        expect(calculateExperience(GrowthRate.GROWTH_SLOW, 1)).toBe(1);
        expect(calculateExperience(GrowthRate.GROWTH_SLOW, 50)).toBe(156250);
        expect(calculateExperience(GrowthRate.GROWTH_SLOW, 100)).toBe(1250000);
    });
});

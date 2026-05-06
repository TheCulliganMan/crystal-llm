
import { calculateBattleStat } from './stats';
import { Stat, StatusCondition } from '../../core/enums';
import type { Pokemon } from '../../core/models/pokemon';
import type { Battle } from './battle/battle-logic';

describe('calculateBattleStat', () => {
    let mockPokemon: Pokemon;
    let mockBattle: Battle;

    beforeEach(() => {
        mockPokemon = {
            _calculateStat: jest.fn(),
            stat_boosts: {
                HP: 0,
                ATTACK: 0,
                DEFENSE: 0,
                SPEED: 0,
                SPECIAL_ATTACK: 0,
                SPECIAL_DEFENSE: 0,
                ACCURACY: 0,
                EVASION: 0,
            },
            status: StatusCondition.NONE,
        } as unknown as Pokemon;

        mockBattle = {
            context: {
                sideFor: jest.fn(),
                badgeBoostActive: jest.fn(),
            },
        } as unknown as Battle;
    });

    it('should calculate the stat with no modifiers', () => {
        (mockPokemon._calculateStat as jest.Mock).mockReturnValue(100);
        (mockBattle.context.sideFor as jest.Mock).mockReturnValue(null);
        expect(calculateBattleStat(mockBattle, mockPokemon, Stat.ATTACK)).toBe(100);
    });

    it('should apply a badge boost', () => {
        (mockPokemon._calculateStat as jest.Mock).mockReturnValue(100);
        (mockBattle.context.sideFor as jest.Mock).mockReturnValue('player');
        (mockBattle.context.badgeBoostActive as jest.Mock).mockReturnValue(true);
        expect(calculateBattleStat(mockBattle, mockPokemon, Stat.ATTACK)).toBe(112);
    });

    it('should apply a stat stage boost', () => {
        (mockPokemon._calculateStat as jest.Mock).mockReturnValue(100);
        (mockBattle.context.sideFor as jest.Mock).mockReturnValue(null);
        mockPokemon.stat_boosts[Stat.ATTACK] = 2;
        expect(calculateBattleStat(mockBattle, mockPokemon, Stat.ATTACK)).toBe(200);
    });

    it('should apply a burn penalty to attack', () => {
        (mockPokemon._calculateStat as jest.Mock).mockReturnValue(100);
        (mockBattle.context.sideFor as jest.Mock).mockReturnValue(null);
        mockPokemon.status = StatusCondition.BURN;
        expect(calculateBattleStat(mockBattle, mockPokemon, Stat.ATTACK)).toBe(50);
    });

    it('should not apply a burn penalty to other stats', () => {
        (mockPokemon._calculateStat as jest.Mock).mockReturnValue(100);
        (mockBattle.context.sideFor as jest.Mock).mockReturnValue(null);
        mockPokemon.status = StatusCondition.BURN;
        expect(calculateBattleStat(mockBattle, mockPokemon, Stat.DEFENSE)).toBe(100);
    });

    it('should apply a paralysis penalty to speed', () => {
        (mockPokemon._calculateStat as jest.Mock).mockReturnValue(100);
        (mockBattle.context.sideFor as jest.Mock).mockReturnValue(null);
        mockPokemon.status = StatusCondition.PARALYSIS;
        expect(calculateBattleStat(mockBattle, mockPokemon, Stat.SPEED)).toBe(25);
    });

    it('should not apply a paralysis penalty to other stats', () => {
        (mockPokemon._calculateStat as jest.Mock).mockReturnValue(100);
        (mockBattle.context.sideFor as jest.Mock).mockReturnValue(null);
        mockPokemon.status = StatusCondition.PARALYSIS;
        expect(calculateBattleStat(mockBattle, mockPokemon, Stat.ATTACK)).toBe(100);
    });

    it('should handle multiple modifiers', () => {
        (mockPokemon._calculateStat as jest.Mock).mockReturnValue(100);
        (mockBattle.context.sideFor as jest.Mock).mockReturnValue('player');
        (mockBattle.context.badgeBoostActive as jest.Mock).mockReturnValue(true);
        mockPokemon.stat_boosts[Stat.ATTACK] = 2;
        mockPokemon.status = StatusCondition.BURN;
        // Badge boost: 100 -> 112
        // Stage boost: 112 * 2 -> 224
        // Burn: 224 / 2 -> 112
        expect(calculateBattleStat(mockBattle, mockPokemon, Stat.ATTACK)).toBe(112);
    });
});

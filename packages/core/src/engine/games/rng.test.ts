
import { HardwareRNG } from './rng';
import { GameState, GameStateSchema } from '@pokecrystal/core/core/state';

describe('HardwareRNG', () => {
  let gameState: GameState;
  let rng: HardwareRNG;

  beforeEach(() => {
    gameState = GameStateSchema.parse({
      sram: {},
      wram: {},
      vram: {
        bank0: {},
        bank1: {},
      },
      hram: {
        joypad: {},
      },
    });
    rng = new HardwareRNG(gameState);
  });

  it('should generate a pseudo-random byte', () => {
    const byte = rng.nextByte();
    expect(byte).toBeGreaterThanOrEqual(0);
    expect(byte).toBeLessThanOrEqual(255);
  });

  it('should generate a value within a given range', () => {
    const value = rng.randrange(10);
    expect(value).toBeGreaterThanOrEqual(0);
    expect(value).toBeLessThan(10);
  });

  it('should handle coin flips correctly', () => {
    expect(rng.coinFlip(0)).toBe(false);
    expect(rng.coinFlip(1)).toBe(true);
  });

  it('should generate a random integer within a given range', () => {
    const value = rng.randint(5, 10);
    expect(value).toBeGreaterThanOrEqual(5);
    expect(value).toBeLessThanOrEqual(10);
  });

  it('should choose a random element from a sequence', () => {
    const seq = [1, 2, 3, 4, 5];
    const value = rng.choice(seq);
    expect(seq).toContain(value);
  });

  it('should throw an error when choosing from an empty sequence', () => {
    expect(() => rng.choice([])).toThrow('Cannot choose from an empty sequence');
  });

  it('should throw an error when randrange is called with a non-positive upper bound', () => {
    expect(() => rng.randrange(0)).toThrow('upperBound must be positive');
    expect(() => rng.randrange(-1)).toThrow('upperBound must be positive');
  });

  it('should produce a deterministic sequence', () => {
    const initialGameState = GameStateSchema.parse({
      sram: {},
      wram: {},
      hram: { hRandomAdd: 0xab, hRandomSub: 0xcd, hardware_divider: 0x1234, joypad: {} },
      vram: { bank0: {}, bank1: {} },
    });
    const rng1 = new HardwareRNG(initialGameState);
    const sequence1 = Array.from({ length: 10 }, () => rng1.nextByte());

    const secondGameState = GameStateSchema.parse({
      sram: {},
      wram: {},
      hram: { hRandomAdd: 0xab, hRandomSub: 0xcd, hardware_divider: 0x1234, joypad: {} },
      vram: { bank0: {}, bank1: {} },
    });
    const rng2 = new HardwareRNG(secondGameState);
    const sequence2 = Array.from({ length: 10 }, () => rng2.nextByte());
    expect(sequence1).toEqual(sequence2);
  });
});

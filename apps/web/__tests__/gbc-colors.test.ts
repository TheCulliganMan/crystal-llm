import { gbc5To8, gbcWordToRgb } from '@pokecrystal/core/core/gbc-colors';

describe('gbc5To8', () => {
  it('expands edge components exactly', () => {
    expect(gbc5To8(0)).toBe(0);
    expect(gbc5To8(31)).toBe(255);
  });

  it('replicates bits for mid components', () => {
    // (28 << 3) | (28 >> 2) = 224 | 7 = 231
    expect(gbc5To8(28)).toBe(231);
  });

  it('throws on out-of-range components', () => {
    expect(() => gbc5To8(-1)).toThrow();
    expect(() => gbc5To8(32)).toThrow();
  });
});

describe('gbcWordToRgb', () => {
  it('decodes canonical palette words', () => {
    expect(gbcWordToRgb(0x7fff)).toEqual([255, 255, 255]);
    expect(gbcWordToRgb(0x0000)).toEqual([0, 0, 0]);
    expect(gbcWordToRgb(0x001f)).toEqual([255, 0, 0]);
    expect(gbcWordToRgb(0x03e0)).toEqual([0, 255, 0]);
    expect(gbcWordToRgb(0x7c00)).toEqual([0, 0, 255]);
  });
});

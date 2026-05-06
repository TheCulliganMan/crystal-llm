import { sine } from './battle-anim-math';

describe('battle-anim-math', () => {
  it('matches ASM sine scaling at pi/2 and 3pi/2', () => {
    expect(sine(0x10, 0x20)).toBe(0x20);
    expect(sine(0x30, 0x20)).toBe(-0x20);
  });
});

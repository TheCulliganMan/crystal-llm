import { GB_CYCLES_PER_FRAME, TMHM_MOVES } from './constants';
import { MoveName } from './enums';

describe('Constants', () => {
  it('TMHM_MOVES should have the correct length', () => {
    // There are 50 TMs and 7 HMs.
    expect(TMHM_MOVES).toHaveLength(57);
  });

  it('GB_CYCLES_PER_FRAME should be correct', () => {
    expect(GB_CYCLES_PER_FRAME).toBe(69905);
  });
});

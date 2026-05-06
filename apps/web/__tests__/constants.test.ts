import {
  Pokemon,
  MAX_PC_BOXES,
  MAX_BOX_MONS,
  JOY_RIGHT,
  JOY_LEFT,
  JOY_UP,
  JOY_DOWN,
  JOY_A,
  JOY_B,
  JOY_SELECT,
  JOY_START,
} from '@pokecrystal/core/core/constants';

describe('Pokemon Enum', () => {
  it('should have MR__MIME at the correct index', () => {
    expect(Pokemon.MR__MIME).toBe(122);
  });
});

describe('PC Box Constants', () => {
  it('should have the correct values', () => {
    expect(MAX_PC_BOXES).toBe(14);
    expect(MAX_BOX_MONS).toBe(20);
  });
});

describe('Joypad Constants', () => {
  it('should have the correct values', () => {
    expect(JOY_RIGHT).toBe(0x01);
    expect(JOY_LEFT).toBe(0x02);
    expect(JOY_UP).toBe(0x04);
    expect(JOY_DOWN).toBe(0x08);
    expect(JOY_A).toBe(0x10);
    expect(JOY_B).toBe(0x20);
    expect(JOY_SELECT).toBe(0x40);
    expect(JOY_START).toBe(0x80);
  });
});

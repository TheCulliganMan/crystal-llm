import { decompress } from './lz';

describe('decompress', () => {
  it('handles LZ_REVERSE with positive offsets', () => {
    const data = new Uint8Array([
      0x03, // LZ_LITERAL length 4
      1, 2, 3, 4,
      0xc3, // LZ_REVERSE length 4
      0x00, 0x03, // positive offset 0x0003 (points at value 4)
      0xff, // LZ_END
    ]);
    const output = decompress(data);
    expect(Array.from(output)).toEqual([1, 2, 3, 4, 4, 3, 2, 1]);
  });
});

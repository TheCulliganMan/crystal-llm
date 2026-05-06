import { Random } from '../random';

describe('Random', () => {
  it('should produce a deterministic sequence of numbers', () => {
    const random = new Random(0);
    const sequence = [
      random.randrange(100),
      random.randrange(100),
      random.randrange(100),
      random.randrange(100),
      random.randrange(100),
    ];
    expect(sequence).toEqual([21, 70, 54, 73, 81]);
  });
});

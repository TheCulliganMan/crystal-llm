import { z } from 'zod';
import { HardwareRNG } from './rng';

export const MemoryGameResultSchema = z.object({
  matched: z.boolean(),
  firstIndex: z.number(),
  secondIndex: z.number(),
  symbol: z.string().nullable(),
});

export type MemoryGameResult = z.infer<typeof MemoryGameResultSchema>;

export class MemoryGame {
  rng: HardwareRNG;
  board: string[];
  revealed: boolean[];

  constructor(rng: HardwareRNG) {
    this.rng = rng;
    const symbols = [
      'ODDISH',
      'POLIWAG',
      'PIKACHU',
      'JIGGLYPUFF',
      'RATTATA',
      'VOLTORB',
      'DITTO',
      'ELECTABUZZ',
    ];
    this.board = symbols.flatMap(symbol => [symbol, symbol]);
    this.revealed = Array(this.board.length).fill(false);
    this.shuffle();
  }

  shuffle(): void {
    for (let index = this.board.length - 1; index > 0; index--) {
      const swapIndex = this.rng.randrange(index + 1);
      [this.board[index], this.board[swapIndex]] = [
        this.board[swapIndex],
        this.board[index],
      ];
    }
    this.revealed = Array(this.board.length).fill(false);
  }

  reveal(first: number, second: number): MemoryGameResult {
    if (first === second) {
      throw new Error('must select two distinct tiles');
    }
    for (const index of [first, second]) {
      if (index < 0 || index >= this.board.length) {
        throw new Error('tile index out of range');
      }
      if (this.revealed[index]) {
        throw new Error('tile already revealed');
      }
    }

    const firstSymbol = this.board[first];
    const secondSymbol = this.board[second];
    const matched = firstSymbol === secondSymbol;
    let symbol: string | null = null;

    if (matched) {
      this.revealed[first] = true;
      this.revealed[second] = true;
      symbol = firstSymbol;
    }

    return {
      matched,
      firstIndex: first,
      secondIndex: second,
      symbol,
    };
  }
}

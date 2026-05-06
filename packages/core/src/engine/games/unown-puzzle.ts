
import { z } from 'zod';
import { HardwareRNG } from './rng';

export const UnownPuzzleLayoutSchema = z.tuple([
  z.tuple([z.number(), z.number(), z.number(), z.number(), z.number(), z.number()]),
  z.tuple([z.number(), z.number(), z.number(), z.number(), z.number(), z.number()]),
  z.tuple([z.number(), z.number(), z.number(), z.number(), z.number(), z.number()]),
  z.tuple([z.number(), z.number(), z.number(), z.number(), z.number(), z.number()]),
  z.tuple([z.number(), z.number(), z.number(), z.number(), z.number(), z.number()]),
  z.tuple([z.number(), z.number(), z.number(), z.number(), z.number(), z.number()]),
]);
export type UnownPuzzleLayout = z.infer<typeof UnownPuzzleLayoutSchema>;

export const TARGET_LAYOUT: UnownPuzzleLayout = [
  [0, 0, 0, 0, 0, 0],
  [0, 1, 2, 3, 4, 0],
  [0, 5, 6, 7, 8, 0],
  [0, 9, 10, 11, 12, 0],
  [0, 13, 14, 15, 16, 0],
  [0, 0, 0, 0, 0, 0],
];

export const START_POSITIONS: [number, number][] = [
  // ASM parity: InitUnownPuzzlePiecePositions .PuzzlePieceInitialPositions emits row,col.
  // Convert those entries to x,y tuples for layout[y][x] writes.
  [0, 0],
  [1, 0],
  [2, 0],
  [3, 0],
  [4, 0],
  [5, 0],
  [0, 1],
  [5, 1],
  [0, 2],
  [5, 2],
  [0, 3],
  [5, 3],
  [0, 4],
  [5, 4],
  [0, 5],
  [5, 5],
];

export const UnownPuzzleResultSchema = z.object({
  solved: z.boolean(),
  moves: z.number(),
  layout: UnownPuzzleLayoutSchema,
  holding_piece: z.number().nullable(),
});
export type UnownPuzzleResult = z.infer<typeof UnownPuzzleResultSchema>;

export class EngineUnownPuzzle {
  private rng: HardwareRNG;
  private layout: number[][];
  private holding_piece: number | null = null;
  private moves = 0;

  constructor(rng: HardwareRNG) {
    this.rng = rng;
    this.layout = TARGET_LAYOUT.map(row => [...row]);
  }

  public shuffle(): void {
    this.layout = Array.from({ length: 6 }, () => Array(6).fill(0));
    this.holding_piece = null;
    this.moves = 0;

    for (let pieceId = 1; pieceId <= 16; pieceId++) {
       
      while (true) {
        const slotIndex = this.rng.nextByte() & 0x0f;
        const [x, y] = START_POSITIONS[slotIndex];
        if (this.layout[y][x] === 0) {
          this.layout[y][x] = pieceId;
          break;
        }
      }
    }
  }

  public loadState(
    layout: number[][],
    options: { holding_piece?: number | null; moves?: number } = {},
  ): void {
    const { holding_piece = null, moves = 0 } = options;
    if (moves < 0) {
      throw new Error('move count cannot be negative');
    }
    if (holding_piece !== null && (holding_piece < 1 || holding_piece > 16)) {
      throw new Error('holding_piece must be between 1 and 16');
    }
    this.layout = this.normalizeLayout(layout);
    this.assertUniquePieces(this.layout, holding_piece);
    this.holding_piece = holding_piece;
    this.moves = moves;
  }

  public isSolved(): boolean {
    return (
      this.holding_piece === null &&
      JSON.stringify(this.layout) === JSON.stringify(TARGET_LAYOUT)
    );
  }

  public snapshot(): UnownPuzzleLayout {
    return UnownPuzzleLayoutSchema.parse(this.layout);
  }

  public status(): UnownPuzzleResult {
    return {
      solved: this.isSolved(),
      moves: this.moves,
      layout: this.snapshot(),
      holding_piece: this.holding_piece,
    };
  }

  public pickup(x: number, y: number): number {
    this.assertBoardCoords(x, y);
    if (this.holding_piece !== null) {
      throw new Error('cannot pick up a piece while already holding one');
    }
    const piece = this.layout[y][x];
    if (piece === 0) {
      throw new Error('no piece present at that coordinate');
    }
    this.layout[y][x] = 0;
    this.holding_piece = piece;
    return piece;
  }

  public place(x: number, y: number): number {
    this.assertBoardCoords(x, y);
    if (this.holding_piece === null) {
      throw new Error('no piece is currently held');
    }
    if (this.layout[y][x] !== 0) {
      throw new Error('target coordinate is already occupied');
    }
    const piece = this.holding_piece;
    this.layout[y][x] = piece;
    this.holding_piece = null;
    this.moves += 1;
    return piece;
  }

  private normalizeLayout(layout: number[][]): number[][] {
    if (layout.length !== 6) {
      throw new Error('layout must contain six rows');
    }
    const normalized: number[][] = [];
    for (const row of layout) {
      if (row.length !== 6) {
        throw new Error('layout rows must contain six columns');
      }
      const normalizedRow: number[] = [];
      for (const value of row) {
        if (!Number.isInteger(value)) {
          throw new Error('layout entries must be integers');
        }
        if (value !== 0 && (value < 1 || value > 16)) {
          throw new Error('layout entries must be 0 or between 1 and 16');
        }
        normalizedRow.push(value);
      }
      normalized.push(normalizedRow);
    }
    return normalized;
  }

  private assertBoardCoords(x: number, y: number): void {
    if (!Number.isInteger(x) || !Number.isInteger(y)) {
      throw new Error('coordinates must be integer puzzle grid cells');
    }
    if (x < 0 || x >= 6 || y < 0 || y >= 6) {
      throw new Error('coordinates must be inside the 6x6 puzzle grid');
    }
  }

  private assertUniquePieces(layout: number[][], holdingPiece: number | null): void {
    const seen = new Set<number>();
    for (const row of layout) {
      for (const value of row) {
        if (value === 0) {
          continue;
        }
        if (seen.has(value)) {
          throw new Error(`piece ${value} appears more than once in the puzzle state`);
        }
        seen.add(value);
      }
    }
    if (holdingPiece !== null && seen.has(holdingPiece)) {
      throw new Error(`held piece ${holdingPiece} also appears in the puzzle layout`);
    }
  }
}

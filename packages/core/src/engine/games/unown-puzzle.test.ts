
import { EngineUnownPuzzle, START_POSITIONS, TARGET_LAYOUT } from './unown-puzzle';
import { HardwareRNG } from './rng';
import { GameState, GameStateSchema } from '@pokecrystal/core/core/state';

describe('EngineUnownPuzzle', () => {
  let gameState: GameState;
  let rng: HardwareRNG;
  let puzzle: EngineUnownPuzzle;

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
          hRandomAdd: 0x12,
          hRandomSub: 0x34,
          hardware_divider: 0xace1,
        },
      },
    );
    rng = new HardwareRNG(gameState);
    puzzle = new EngineUnownPuzzle(rng);
  });

  it('should shuffle pieces onto the perimeter', () => {
    puzzle.shuffle();
    const layout = puzzle.snapshot();

    const borderCoords: [number, number][] = [];
    for (let y = 0; y < 6; y++) {
      for (let x = 0; x < 6; x++) {
        if (x === 0 || x === 5 || y === 0 || y === 5) {
          borderCoords.push([x, y]);
        }
      }
    }

    const innerCoords: [number, number][] = [];
    for (let y = 1; y < 5; y++) {
      for (let x = 1; x < 5; x++) {
        innerCoords.push([x, y]);
      }
    }

    const borderPieces = borderCoords
      .map(([x, y]) => layout[y][x])
      .filter(piece => piece !== 0);

    expect(borderPieces.sort((a, b) => a - b)).toEqual(
      Array.from({ length: 16 }, (_, i) => i + 1),
    );

    innerCoords.forEach(([x, y]) => {
      expect(layout[y][x]).toBe(0);
    });
  });

  it('should preserve exactly one copy of every piece after repeated shuffles', () => {
    for (let i = 0; i < 25; i += 1) {
      puzzle.shuffle();
      const pieces = puzzle.snapshot().flat().filter(piece => piece !== 0);

      expect(pieces).toHaveLength(16);
      expect([...new Set(pieces)]).toHaveLength(16);
      expect(pieces.sort((a, b) => a - b)).toEqual(
        Array.from({ length: 16 }, (_, index) => index + 1),
      );
      expect(puzzle.status()).toEqual(
        expect.objectContaining({
          holding_piece: null,
          moves: 0,
          solved: false,
        }),
      );
    }
  });

  it('should pick up and place pieces', () => {
    puzzle.shuffle();
    const layout = puzzle.snapshot();

    let sourceX = -1;
    let sourceY = -1;
    for (let y = 0; y < 6; y++) {
        for (let x = 0; x < 6; x++) {
            if (layout[y][x] !== 0) {
                sourceX = x;
                sourceY = y;
                break;
            }
        }
        if (sourceX !== -1) {
            break;
        }
    }

    const heldPiece = puzzle.pickup(sourceX, sourceY);
    expect(heldPiece).toBeGreaterThan(0);
    expect(puzzle.status().holding_piece).toBe(heldPiece);
    expect(puzzle.snapshot()[sourceY][sourceX]).toBe(0);

    let targetX = -1;
    let targetY = -1;

    for (let y = 0; y < 6; y++) {
        for (let x = 0; x < 6; x++) {
            if (puzzle.snapshot()[y][x] === 0) {
                targetX = x;
                targetY = y;
                break;
            }
        }
        if (targetX !== -1) {
            break;
        }
    }

    puzzle.place(targetX, targetY);
    expect(puzzle.snapshot()[targetY][targetX]).toBe(heldPiece);
    expect(puzzle.status().holding_piece).toBeNull();
    expect(puzzle.status().moves).toBe(1);
  });

  it('should reject invalid moves', () => {
    puzzle.shuffle();

    expect(() => puzzle.place(0, 0)).toThrow('no piece is currently held');
    expect(() => puzzle.pickup(1.5, 0)).toThrow('coordinates must be integer puzzle grid cells');
    expect(() => puzzle.pickup(6, 0)).toThrow('coordinates must be inside the 6x6 puzzle grid');

    const holdingOneLayout = puzzle.snapshot().map(row => [...row]);
    const holdingOneY = holdingOneLayout.findIndex(row => row.includes(1));
    const holdingOneX = holdingOneLayout[holdingOneY].indexOf(1);
    holdingOneLayout[holdingOneY][holdingOneX] = 0;
    puzzle.loadState(holdingOneLayout, { holding_piece: 1 });
    expect(() => puzzle.pickup(0, 0)).toThrow(
      'cannot pick up a piece while already holding one',
    );

    const holdingTwoLayout = puzzle.snapshot().map(row => [...row]);
    const holdingTwoY = holdingTwoLayout.findIndex(row => row.includes(2));
    const holdingTwoX = holdingTwoLayout[holdingTwoY].indexOf(2);
    holdingTwoLayout[holdingTwoY][holdingTwoX] = 0;
    const layout = holdingTwoLayout;
    let occupiedX = -1;
    let occupiedY = -1;

    for (let y = 0; y < 6; y++) {
        for (let x = 0; x < 6; x++) {
            if (layout[y][x] !== 0) {
                occupiedX = x;
                occupiedY = y;
                break;
            }
        }
        if (occupiedX !== -1) {
            break;
        }
    }

    puzzle.loadState(holdingTwoLayout, { holding_piece: 2 });
    expect(() => puzzle.place(occupiedX, occupiedY)).toThrow(
      'target coordinate is already occupied',
    );
  });

  it('should reject impossible restored piece inventories', () => {
    const duplicateLayout = TARGET_LAYOUT.map(row => [...row]);
    duplicateLayout[1][2] = 1;
    expect(() => puzzle.loadState(duplicateLayout)).toThrow(
      'piece 1 appears more than once in the puzzle state',
    );

    const heldDuplicateLayout = TARGET_LAYOUT.map(row => [...row]);
    expect(() => puzzle.loadState(heldDuplicateLayout, { holding_piece: 1 })).toThrow(
      'held piece 1 also appears in the puzzle layout',
    );
  });

  it('should keep snapshots immutable from caller mutation', () => {
    puzzle.loadState(TARGET_LAYOUT);
    const snapshot = puzzle.snapshot();
    snapshot[1][1] = 16;

    expect(puzzle.snapshot()).toEqual(TARGET_LAYOUT);
    expect(puzzle.isSolved()).toBe(true);
  });

  it('should detect a solved layout', () => {
    puzzle.loadState([...TARGET_LAYOUT.map(row => [...row])]);
    expect(puzzle.isSolved()).toBe(true);
    const status = puzzle.status();
    expect(status.solved).toBe(true);
    expect(status.holding_piece).toBeNull();
  });

  it('should place pieces using the ASM start position ordering', () => {
    class FakeRNG {
      private index = 0;
      constructor(private sequence: number[]) {}
      nextByte(): number {
        const value = this.sequence[this.index] ?? 0;
        this.index += 1;
        return value;
      }
    }

    const fakeRng = new FakeRNG([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0]) as HardwareRNG;
    const orderedPuzzle = new EngineUnownPuzzle(fakeRng);
    orderedPuzzle.shuffle();
    const layout = orderedPuzzle.snapshot();

    expect(START_POSITIONS).toEqual([
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
    ]);

    const expectedBySlot = [16, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    START_POSITIONS.forEach(([x, y], index) => {
      expect(layout[y][x]).toBe(expectedBySlot[index]);
    });

    expect(layout[5][1]).toBe(0);
    expect(layout[5][2]).toBe(0);
    expect(layout[5][3]).toBe(0);
    expect(layout[5][4]).toBe(0);
  });
});

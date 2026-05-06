import { z } from 'zod';
import { HardwareRNG } from './rng';

export const REEL_LENGTH = 15;

// Percentages mirror the ``percent`` macro (value * $ff / 100) from the ASM.
// Keeping the literal thresholds avoids float drift and preserves the exact
// branch probabilities used by ``Slots_InitBias`` and the reel action jumptable.
const _PERCENT_1 = 0x02; // 1 percent
const _PERCENT_3 = 0x07; // 3 percent
const _PERCENT_4 = 0x0a; // 4 percent
const _PERCENT_6 = 0x0f; // 6 percent
const _PERCENT_8 = 0x14; // 8 percent
const _PERCENT_12 = 0x1e; // 12 percent
const _PERCENT_16 = 0x28; // 16 percent
const _PERCENT_19 = 0x30; // 19 percent
const _PERCENT_24 = 0x3c; // 24 percent - 1
const _PERCENT_31 = 0x4f; // 31 percent
const _PERCENT_47 = 0x78; // 47 percent
const _PERCENT_63 = 0xa0; // 63 percent
const _PERCENT_71 = 0xb4; // 71 percent
const _PERCENT_100 = 0xff; // 100 percent sentinel

export enum SlotSymbol {
  SEVEN = 0,
  POKEBALL = 1,
  CHERRY = 2,
  PIKACHU = 3,
  SQUIRTLE = 4,
  STARYU = 5,
}

export const SlotSymbolPayout: Record<SlotSymbol, number> = {
  [SlotSymbol.SEVEN]: 300,
  [SlotSymbol.POKEBALL]: 50,
  [SlotSymbol.CHERRY]: 6,
  [SlotSymbol.PIKACHU]: 8,
  [SlotSymbol.SQUIRTLE]: 10,
  [SlotSymbol.STARYU]: 15,
};

// ASM: Reel1Tilemap
export const REEL_TILEMAPS: SlotSymbol[][] = [
  [
    SlotSymbol.SEVEN,
    SlotSymbol.CHERRY,
    SlotSymbol.STARYU,
    SlotSymbol.PIKACHU,
    SlotSymbol.SQUIRTLE,
    SlotSymbol.SEVEN,
    SlotSymbol.CHERRY,
    SlotSymbol.STARYU,
    SlotSymbol.PIKACHU,
    SlotSymbol.SQUIRTLE,
    SlotSymbol.POKEBALL,
    SlotSymbol.CHERRY,
    SlotSymbol.STARYU,
    SlotSymbol.PIKACHU,
    SlotSymbol.SQUIRTLE,
  ],
  // ASM: Reel2Tilemap
  [
    SlotSymbol.SEVEN,
    SlotSymbol.PIKACHU,
    SlotSymbol.CHERRY,
    SlotSymbol.SQUIRTLE,
    SlotSymbol.STARYU,
    SlotSymbol.POKEBALL,
    SlotSymbol.PIKACHU,
    SlotSymbol.CHERRY,
    SlotSymbol.SQUIRTLE,
    SlotSymbol.STARYU,
    SlotSymbol.POKEBALL,
    SlotSymbol.PIKACHU,
    SlotSymbol.CHERRY,
    SlotSymbol.SQUIRTLE,
    SlotSymbol.STARYU,
  ],
  // ASM: Reel3Tilemap
  [
    SlotSymbol.SEVEN,
    SlotSymbol.PIKACHU,
    SlotSymbol.CHERRY,
    SlotSymbol.SQUIRTLE,
    SlotSymbol.STARYU,
    SlotSymbol.PIKACHU,
    SlotSymbol.CHERRY,
    SlotSymbol.SQUIRTLE,
    SlotSymbol.STARYU,
    SlotSymbol.PIKACHU,
    SlotSymbol.POKEBALL,
    SlotSymbol.CHERRY,
    SlotSymbol.SQUIRTLE,
    SlotSymbol.STARYU,
    SlotSymbol.PIKACHU,
  ],
];

export const BetLineMap = {
  1: ['middle'],
  2: ['bottom', 'top', 'middle'],
  3: ['diagonal_up', 'diagonal_down', 'bottom', 'top', 'middle'],
};

export const SlotMachineResultSchema = z.object({
  windows: z.tuple([
    z.tuple([z.nativeEnum(SlotSymbol), z.nativeEnum(SlotSymbol), z.nativeEnum(SlotSymbol)]),
    z.tuple([z.nativeEnum(SlotSymbol), z.nativeEnum(SlotSymbol), z.nativeEnum(SlotSymbol)]),
    z.tuple([z.nativeEnum(SlotSymbol), z.nativeEnum(SlotSymbol), z.nativeEnum(SlotSymbol)]),
  ]),
  matchedSymbol: z.nativeEnum(SlotSymbol).nullable(),
  winningLines: z.array(z.string()),
  payout: z.number(),
});

export type SlotMachineResult = z.infer<typeof SlotMachineResultSchema>;

function wrapIndex(index: number): number {
  return index % REEL_LENGTH;
}

function windowForReel(reel: SlotSymbol[], offset: number): [SlotSymbol, SlotSymbol, SlotSymbol] {
  return [
    reel[wrapIndex(offset)],
    reel[wrapIndex(offset + 1)],
    reel[wrapIndex(offset + 2)],
  ];
}

function lineOrderForBet(bet: number): string[] {
  if (bet === 1) {
    return ['middle'];
  }
  if (bet === 2) {
    return ['bottom', 'top', 'middle'];
  }
  if (bet === 3) {
    return ['diagonal_up', 'diagonal_down', 'bottom', 'top', 'middle'];
  }
  throw new Error('bet must be 1, 2, or 3');
}

function lineSymbols(
  windows: [SlotSymbol, SlotSymbol, SlotSymbol][],
  line: string,
): [SlotSymbol, SlotSymbol, SlotSymbol] {
  if (line === 'middle') {
    return [windows[0][1], windows[1][1], windows[2][1]];
  }
  if (line === 'top') {
    return [windows[0][0], windows[1][0], windows[2][0]];
  }
  if (line === 'bottom') {
    return [windows[0][2], windows[1][2], windows[2][2]];
  }
  if (line === 'diagonal_up') {
    return [windows[0][2], windows[1][1], windows[2][0]];
  }
  if (line === 'diagonal_down') {
    return [windows[0][0], windows[1][1], windows[2][2]];
  }
  throw new Error(`Unknown line identifier ${line}`);
}

function checkFirstTwoReels(
  windows: [SlotSymbol, SlotSymbol, SlotSymbol][],
  bet: number,
): [SlotSymbol | null, boolean, string[]] {
  let matchedSymbol: SlotSymbol | null = null;
  const matchedLines: string[] = [];
  let sawSeven = false;

  for (const line of lineOrderForBet(bet)) {
    let first: SlotSymbol, second: SlotSymbol;
    if (line === 'middle') {
      [first, second] = [windows[0][1], windows[1][1]];
    } else if (line === 'top') {
      [first, second] = [windows[0][0], windows[1][0]];
    } else if (line === 'bottom') {
      [first, second] = [windows[0][2], windows[1][2]];
    } else if (line === 'diagonal_up') {
      [first, second] = [windows[0][2], windows[1][1]];
    } else {
      // diagonal_down
      [first, second] = [windows[0][0], windows[1][1]];
    }

    if (first === second) {
      matchedSymbol = first;
      matchedLines.push(line);
      sawSeven = sawSeven || first === SlotSymbol.SEVEN;
    }
  }

  return [matchedSymbol, sawSeven, matchedLines];
}

function checkAllThreeReels(
  windows: [SlotSymbol, SlotSymbol, SlotSymbol][],
  bet: number,
): [SlotSymbol | null, string[]] {
  let matchedSymbol: SlotSymbol | null = null;
  const matchedLines: string[] = [];

  for (const line of lineOrderForBet(bet)) {
    const [first, second, third] = lineSymbols(windows, line);
    if (first === second && second === third) {
      matchedSymbol = first;
      matchedLines.push(line);
    }
  }

  return [matchedSymbol, matchedLines];
}

function advance(offset: number, step = 1): number {
  return (offset + step) % REEL_LENGTH;
}

export enum SlotMachineMode {
  NORMAL = 'normal',
  LUCKY = 'lucky',
}

export class SlotMachine {
  rng: HardwareRNG;

  constructor(rng: HardwareRNG) {
    this.rng = rng;
  }

  private getBias(mode: SlotMachineMode): SlotSymbol | null {
    let table: [number, SlotSymbol | null][];
    if (mode === SlotMachineMode.NORMAL) {
      table = [
        [_PERCENT_1 - 1, SlotSymbol.SEVEN],
        [_PERCENT_1 + 1, SlotSymbol.POKEBALL],
        [_PERCENT_4, SlotSymbol.STARYU],
        [_PERCENT_8, SlotSymbol.SQUIRTLE],
        [_PERCENT_16, SlotSymbol.PIKACHU],
        [_PERCENT_19, SlotSymbol.CHERRY],
        [_PERCENT_100, null], // SLOTS_NO_BIAS
      ];
    } else {
      table = [
        [_PERCENT_1, SlotSymbol.SEVEN],
        [_PERCENT_1 + 1, SlotSymbol.POKEBALL],
        [_PERCENT_3 + 1, SlotSymbol.STARYU],
        [_PERCENT_6 + 1, SlotSymbol.SQUIRTLE],
        [_PERCENT_12, SlotSymbol.PIKACHU],
        [_PERCENT_31 + 1, SlotSymbol.CHERRY],
        [_PERCENT_100, null],
      ];
    }

    const roll = this.rng.nextByte();
    for (const [threshold, symbol] of table) {
      if (roll <= threshold) {
        return symbol;
      }
    }
    return null;
  }

  private initialOffsets(reelPositions: number[] | null | undefined): number[] {
    if (reelPositions === null || reelPositions === undefined) {
      return [
        this.rng.nextByte() % REEL_LENGTH,
        this.rng.nextByte() % REEL_LENGTH,
        this.rng.nextByte() % REEL_LENGTH,
      ];
    }
    if (reelPositions.length !== 3) {
      throw new Error('reel_positions must contain three entries');
    }
    return reelPositions.map(pos => pos % REEL_LENGTH);
  }

  private stopReel1(offset: number, bias: SlotSymbol | null): number {
    if (bias === null) {
      return offset;
    }

    let counter = 4;
    while (counter > 0) {
      const window = windowForReel(REEL_TILEMAPS[0], offset);
      if (window.includes(bias)) {
        break;
      }
      offset = advance(offset);
      counter -= 1;
    }
    return offset;
  }

  private attemptSkipToSeven(offsets: number[], bet: number): number[] | null {
    const firstWindow = windowForReel(REEL_TILEMAPS[0], offsets[0]);
    if (!firstWindow.includes(SlotSymbol.SEVEN)) {
      return null;
    }

    let offsetTwo = offsets[1];
    for (let i = 0; i < REEL_LENGTH * 2; i++) {
      const windows: [SlotSymbol, SlotSymbol, SlotSymbol][] = [
        firstWindow,
        windowForReel(REEL_TILEMAPS[1], offsetTwo),
      ];
      const [, sawSeven] = checkFirstTwoReels(windows, bet);
      if (sawSeven) {
        return [offsets[0], offsetTwo, offsets[2]];
      }
      offsetTwo = advance(offsetTwo);
    }
    throw new Error('Failed to align reel 2 to seven after skip-to-7 setup');
  }

  private stopReel2(offsets: number[], bias: SlotSymbol | null, bet: number): void {
    let maybeSkip = false;
    if (bet >= 2 && (bias === null || bias === SlotSymbol.SEVEN)) {
      maybeSkip = this.rng.nextByte() < _PERCENT_31 + 1;
    }

    if (maybeSkip) {
      const aligned = this.attemptSkipToSeven(offsets, bet);
      if (aligned !== null) {
        offsets[0] = aligned[0];
        offsets[1] = aligned[1];
        offsets[2] = aligned[2];
        return;
      }
    }

    let counter = 4;
    while (true) {
      const windows: [SlotSymbol, SlotSymbol, SlotSymbol][] = [
        windowForReel(REEL_TILEMAPS[0], offsets[0]),
        windowForReel(REEL_TILEMAPS[1], offsets[1]),
      ];
      const [matchedSymbol] = checkFirstTwoReels(windows, bet);
      if (matchedSymbol !== null && matchedSymbol === bias) {
        return;
      }

      if (bias === null || counter === 0) {
        return;
      }

      offsets[1] = advance(offsets[1]);
      counter -= 1;
    }
  }

  private selectReel3Action(bias: SlotSymbol | null): string {
    if (bias === SlotSymbol.SEVEN) {
      const roll = this.rng.nextByte();
      if (roll >= _PERCENT_71) {
        return 'stop';
      }
      if (roll >= _PERCENT_47) {
        return 'slow';
      }
      if (roll >= _PERCENT_24) {
        return 'golem';
      }
      return 'chansey';
    }

    const roll = this.rng.nextByte();
    if (roll >= _PERCENT_63) {
      return 'stop';
    }
    if (roll >= _PERCENT_31 + 1) {
      return 'slow';
    }
    return 'golem';
  }

  private applyReel3Stop(offsets: number[], bias: SlotSymbol | null, bet: number): void {
    let counter = 4;
    while (true) {
      const windows: [SlotSymbol, SlotSymbol, SlotSymbol][] = [
        windowForReel(REEL_TILEMAPS[0], offsets[0]),
        windowForReel(REEL_TILEMAPS[1], offsets[1]),
        windowForReel(REEL_TILEMAPS[2], offsets[2]),
      ];
      const [matchedSymbol] = checkAllThreeReels(windows, bet);
      if (matchedSymbol !== null) {
        if (matchedSymbol === bias) {
          return;
        }
        offsets[2] = advance(offsets[2]);
        if (counter) {
          counter -= 1;
        }
        continue;
      }

      if (bias === null || counter === 0) {
        return;
      }
      offsets[2] = advance(offsets[2]);
      counter -= 1;
    }
  }

  private findOffsetForMatch(
    offsets: number[],
    bet: number,
    { targetSymbol, step = 1 }: { targetSymbol: SlotSymbol | null; step?: number },
  ): number {
    for (let i = 0; i < REEL_LENGTH * 2; i++) {
      const windows: [SlotSymbol, SlotSymbol, SlotSymbol][] = [
        windowForReel(REEL_TILEMAPS[0], offsets[0]),
        windowForReel(REEL_TILEMAPS[1], offsets[1]),
        windowForReel(REEL_TILEMAPS[2], offsets[2]),
      ];
      const [matchedSymbol] = checkAllThreeReels(windows, bet);
      if (targetSymbol === null) {
        if (matchedSymbol === null) {
          return offsets[2];
        }
      } else if (matchedSymbol === targetSymbol) {
        return offsets[2];
      }
      offsets[2] = advance(offsets[2], step);
    }

    throw new Error('Failed to resolve reel 3 action within bounds');
  }

  private applyReel3SlowAdvance(
    offsets: number[],
    bias: SlotSymbol | null,
    bet: number,
  ): void {
    const target: SlotSymbol | null = bias === SlotSymbol.SEVEN ? SlotSymbol.SEVEN : null;
    offsets[2] = this.findOffsetForMatch(offsets, bet, { targetSymbol: target, step: 1 });
  }

  private applyReel3Golem(offsets: number[], bias: SlotSymbol | null, bet: number): void {
    if (bias === SlotSymbol.SEVEN) {
      offsets[2] = this.findOffsetForMatch(offsets, bet, {
        targetSymbol: SlotSymbol.SEVEN,
      });
      return;
    }

    // Mirrors Slots_GetNumberOfGolems: pick a stride between 4 and 7
    // inclusive before escalating it every hop.
    let stride = 0;
    while (stride < 4) {
      stride = this.rng.nextByte() & 0x7;
    }
    let step = stride;
    for (let i = 0; i < REEL_LENGTH * 2; i++) {
      const windows: [SlotSymbol, SlotSymbol, SlotSymbol][] = [
        windowForReel(REEL_TILEMAPS[0], offsets[0]),
        windowForReel(REEL_TILEMAPS[1], offsets[1]),
        windowForReel(REEL_TILEMAPS[2], offsets[2]),
      ];
      const [matchedSymbol] = checkAllThreeReels(windows, bet);
      if (matchedSymbol === null) {
        return;
      }
      offsets[2] = advance(offsets[2], step);
      step += 1;
    }
    throw new Error('Golem manipulation failed to break matching reels');
  }

  private applyReel3Chansey(offsets: number[], bet: number): void {
    // Chansey is only dispatched when bias == SEVEN, so we can target sevens
    // directly. The hardware advances by 17 tiles each egg drop.
    offsets[2] = this.findOffsetForMatch(offsets, bet, {
      targetSymbol: SlotSymbol.SEVEN,
      step: 17,
    });
  }

  private stopReel3(offsets: number[], bias: SlotSymbol | null, bet: number): void {
    const windowsFirstTwo: [SlotSymbol, SlotSymbol, SlotSymbol][] = [
      windowForReel(REEL_TILEMAPS[0], offsets[0]),
      windowForReel(REEL_TILEMAPS[1], offsets[1]),
    ];
    const [matchedSymbol, sawSeven] = checkFirstTwoReels(windowsFirstTwo, bet);

    if (matchedSymbol === null || !sawSeven) {
      this.applyReel3Stop(offsets, bias, bet);
      return;
    }

    const action = this.selectReel3Action(bias);
    if (action === 'stop') {
      this.applyReel3Stop(offsets, bias, bet);
    } else if (action === 'slow') {
      this.applyReel3SlowAdvance(offsets, bias, bet);
    } else if (action === 'golem') {
      this.applyReel3Golem(offsets, bias, bet);
    } else {
      this.applyReel3Chansey(offsets, bet);
    }
  }

  spin({
    bet,
    mode = SlotMachineMode.NORMAL,
    bias,
    reelPositions,
  }: {
    bet: number;
    mode?: SlotMachineMode;
    bias?: SlotSymbol | null;
    reelPositions?: number[] | null;
  }): SlotMachineResult {
    if (![1, 2, 3].includes(bet)) {
      throw new Error('bet must be 1, 2, or 3');
    }

    if (bias === undefined) {
      bias = this.getBias(mode);
    }

    const offsets = this.initialOffsets(reelPositions);
    offsets[0] = this.stopReel1(offsets[0], bias);
    this.stopReel2(offsets, bias, bet);
    this.stopReel3(offsets, bias, bet);

    const windows: [
      [SlotSymbol, SlotSymbol, SlotSymbol],
      [SlotSymbol, SlotSymbol, SlotSymbol],
      [SlotSymbol, SlotSymbol, SlotSymbol],
    ] = [
      windowForReel(REEL_TILEMAPS[0], offsets[0]),
      windowForReel(REEL_TILEMAPS[1], offsets[1]),
      windowForReel(REEL_TILEMAPS[2], offsets[2]),
    ];

    const [matchedSymbol, winningLines] = checkAllThreeReels(windows, bet);
    const payout =
      matchedSymbol !== null ? SlotSymbolPayout[matchedSymbol as SlotSymbol] : 0;

    return {
      windows,
      matchedSymbol,
      winningLines,
      payout,
    };
  }
}

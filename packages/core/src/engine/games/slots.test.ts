import {
  REEL_LENGTH,
  REEL_TILEMAPS,
  BetLineMap,
  SlotMachine,
  SlotMachineMode,
  SlotSymbol,
  SlotSymbolPayout,
} from "./slots";

class QueueRng {
  private values: number[];

  constructor(values: number[]) {
    this.values = [...values];
  }

  nextByte(): number {
    const value = this.values.shift();
    if (value === undefined) {
      throw new Error("RNG queue exhausted");
    }
    return value;
  }
}

describe("SlotMachine ASM parity", () => {
  it("matches ASM reel length, reel tilemaps, and payout table", () => {
    expect(REEL_LENGTH).toBe(15);

    expect(REEL_TILEMAPS).toEqual([
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
    ]);

    expect(SlotSymbolPayout).toEqual({
      [SlotSymbol.SEVEN]: 300,
      [SlotSymbol.POKEBALL]: 50,
      [SlotSymbol.CHERRY]: 6,
      [SlotSymbol.PIKACHU]: 8,
      [SlotSymbol.SQUIRTLE]: 10,
      [SlotSymbol.STARYU]: 15,
    });
  });

  it("uses ASM normal bias thresholds", () => {
    const machine = new SlotMachine(new QueueRng([0x00]) as never);
    const getBias = (machine as unknown as { getBias: (mode: SlotMachineMode) => SlotSymbol | null }).getBias.bind(machine);

    (machine as unknown as { rng: QueueRng }).rng = new QueueRng([0x01]);
    expect(getBias(SlotMachineMode.NORMAL)).toBe(SlotSymbol.SEVEN);

    (machine as unknown as { rng: QueueRng }).rng = new QueueRng([0x02]);
    expect(getBias(SlotMachineMode.NORMAL)).toBe(SlotSymbol.POKEBALL);

    (machine as unknown as { rng: QueueRng }).rng = new QueueRng([0x0b]);
    expect(getBias(SlotMachineMode.NORMAL)).toBe(SlotSymbol.SQUIRTLE);

    (machine as unknown as { rng: QueueRng }).rng = new QueueRng([0x31]);
    expect(getBias(SlotMachineMode.NORMAL)).toBeNull();
  });

  it("uses ASM lucky bias thresholds", () => {
    const machine = new SlotMachine(new QueueRng([0x00]) as never);
    const getBias = (machine as unknown as { getBias: (mode: SlotMachineMode) => SlotSymbol | null }).getBias.bind(machine);

    (machine as unknown as { rng: QueueRng }).rng = new QueueRng([0x02]);
    expect(getBias(SlotMachineMode.LUCKY)).toBe(SlotSymbol.SEVEN);

    (machine as unknown as { rng: QueueRng }).rng = new QueueRng([0x03]);
    expect(getBias(SlotMachineMode.LUCKY)).toBe(SlotSymbol.POKEBALL);

    (machine as unknown as { rng: QueueRng }).rng = new QueueRng([0x09]);
    expect(getBias(SlotMachineMode.LUCKY)).toBe(SlotSymbol.SQUIRTLE);

    (machine as unknown as { rng: QueueRng }).rng = new QueueRng([0x80]);
    expect(getBias(SlotMachineMode.LUCKY)).toBeNull();
  });

  it("uses ASM bet line ordering for two-coin mode", () => {
    expect(BetLineMap[2]).toEqual(["bottom", "top", "middle"]);
  });

  it("uses ASM bet line ordering for three-coin mode", () => {
    expect(BetLineMap[3]).toEqual([
      "diagonal_up",
      "diagonal_down",
      "bottom",
      "top",
      "middle",
    ]);
  });
});

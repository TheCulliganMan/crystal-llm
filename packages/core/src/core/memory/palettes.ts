import { z } from 'zod';

export const DEFAULT_BGP_VALUE = 0xe4;
export const DEFAULT_OBP0_VALUE = 0xe4;
export const DEFAULT_OBP1_VALUE = 0xe4;
const PALETTE_MASK = 0xff;

export const PaletteRegistersSchema = z.object({
  bgp: z.number().default(DEFAULT_BGP_VALUE),
  obp0: z.number().default(DEFAULT_OBP0_VALUE),
  obp1: z.number().default(DEFAULT_OBP1_VALUE),
  _fade_steps: z.array(z.tuple([z.number(), z.number(), z.number()])).default([]),
});

export type PaletteRegisters = z.infer<typeof PaletteRegistersSchema>;

export class PaletteRegistersManager {
  private registers: PaletteRegisters;

  constructor(registers: PaletteRegisters) {
    this.registers = registers;
  }

  setPalettes({ bgp, obp0, obp1 }: { bgp?: number; obp0?: number; obp1?: number }): void {
    if (bgp !== undefined) {
      this.registers.bgp = bgp & PALETTE_MASK;
    }
    if (obp0 !== undefined) {
      this.registers.obp0 = obp0 & PALETTE_MASK;
    }
    if (obp1 !== undefined) {
      this.registers.obp1 = obp1 & PALETTE_MASK;
    }
  }

  resetToDefaults(): void {
    this.setPalettes({
      bgp: DEFAULT_BGP_VALUE,
      obp0: DEFAULT_OBP0_VALUE,
      obp1: DEFAULT_OBP1_VALUE,
    });
    this.registers._fade_steps = [];
  }

  queueFade(steps: [number, number, number][]): void {
    const sequence = steps.map(
      ([bgp, obp0, obp1]) =>
        [bgp & PALETTE_MASK, obp0 & PALETTE_MASK, obp1 & PALETTE_MASK] as [number, number, number]
    );
    if (sequence.length === 0) {
      this.registers._fade_steps = [];
      throw new Error('queueFade requires at least one step');
    }
    this.registers._fade_steps = sequence;
  }

  tick(): void {
    if (this.registers._fade_steps.length === 0) {
      return;
    }
    const [bgp, obp0, obp1] = this.registers._fade_steps.shift()!;
    this.setPalettes({ bgp, obp0, obp1 });
  }
}

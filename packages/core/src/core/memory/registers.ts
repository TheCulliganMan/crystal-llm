import { z } from 'zod';

const mask = (value: number, bits = 0xff) => value & bits;

export const DEFAULT_LCDC_VALUE = 0xf3;

export enum LCDControlFlags {
  LCD_ENABLE = 0x80,
  WINDOW_TILE_MAP = 0x40,
  WINDOW_ENABLE = 0x20,
  TILE_DATA_SELECT = 0x10,
  BG_TILE_MAP = 0x08,
  OBJ_SIZE = 0x04,
  OBJ_ENABLE = 0x02,
  OBJ_PRIORITY = 0x01,
}

export enum LCDMode {
  HBLANK = 0,
  VBLANK = 1,
  OAM_SEARCH = 2,
  TRANSFER = 3,
}

export enum LCDInterruptFlags {
  MODE2_OAM = 1 << 5,
  MODE1_VBLANK = 1 << 4,
  MODE0_HBLANK = 1 << 3,
  LY_LYC = 1 << 6,
}

export const LCDRegistersSchema = z.object({
  lcdc: z.number().default(DEFAULT_LCDC_VALUE),
  stat: z.number().default(0),
  scx: z.number().default(0),
  scy: z.number().default(0),
  wx: z.number().default(0),
  wy: z.number().default(0),
  ly: z.number().default(0),
  lyc: z.number().default(0),
});

export type LCDRegisters = z.infer<typeof LCDRegistersSchema>;

export class LCDRegistersManager {
  constructor(private registers: LCDRegisters) {}

  get lcdcFlags(): LCDControlFlags {
    return mask(this.registers.lcdc);
  }

  get windowEnabled(): boolean {
    return !!(this.lcdcFlags & LCDControlFlags.WINDOW_ENABLE);
  }

  get objEnabled(): boolean {
    return !!(this.lcdcFlags & LCDControlFlags.OBJ_ENABLE);
  }

  get priorityEnabled(): boolean {
    return !!(this.lcdcFlags & LCDControlFlags.OBJ_PRIORITY);
  }

  setLcdc(value: number): void {
    this.registers.lcdc = mask(value);
  }

  setScroll(scx: number, scy: number): void {
    this.registers.scx = mask(scx);
    this.registers.scy = mask(scy);
  }

  setWindow(wx: number, wy: number): void {
    this.registers.wx = mask(wx);
    this.registers.wy = mask(wy);
  }

  setMode(mode: LCDMode): void {
    this.registers.stat = (mask(this.registers.stat) & ~0x03) | (mode & 0x03);
  }

  get mode(): LCDMode {
    return mask(this.registers.stat & 0x03);
  }

  setLy(value: number): void {
    this.registers.ly = mask(value);
    this.updateCoincidenceFlag();
  }

  setLyc(value: number): void {
    this.registers.lyc = mask(value);
    this.updateCoincidenceFlag();
  }

  private updateCoincidenceFlag(): void {
    if (this.registers.ly === this.registers.lyc) {
      this.registers.stat |= 1 << 2;
    } else {
      this.registers.stat &= ~(1 << 2);
    }
  }

  enableStatInterrupt(flag: LCDInterruptFlags): void {
    this.registers.stat |= mask(flag);
  }

  disableStatInterrupt(flag: LCDInterruptFlags): void {
    this.registers.stat &= ~mask(flag);
  }
}

export enum InterruptType {
  VBLANK = 0,
  LCD_STAT = 1,
  TIMER = 2,
  SERIAL = 3,
  JOYPAD = 4,
}

export const INTERRUPT_VECTORS: Record<InterruptType, number> = {
  [InterruptType.VBLANK]: 0x40,
  [InterruptType.LCD_STAT]: 0x48,
  [InterruptType.TIMER]: 0x50,
  [InterruptType.SERIAL]: 0x58,
  [InterruptType.JOYPAD]: 0x60,
};

const INTERRUPT_MASK = (1 << Object.keys(InterruptType).length / 2) - 1;

export const InterruptControllerSchema = z.object({
  interrupt_flags: z.number().default(0),
  interrupt_enable: z.number().default(0),
  ime: z.boolean().default(false),
});

export type InterruptController = z.infer<typeof InterruptControllerSchema>;

export class InterruptControllerManager {
  constructor(private controller: InterruptController) {}

  request(interrupt: InterruptType): void {
    this.controller.interrupt_flags |= 1 << interrupt;
  }

  acknowledge(interrupt: InterruptType): void {
    this.controller.interrupt_flags &= ~(1 << interrupt);
  }

  setEnableMask(maskValue: number): void {
    this.controller.interrupt_enable = mask(maskValue, INTERRUPT_MASK);
  }

  get pending(): number {
    return this.controller.interrupt_flags & this.controller.interrupt_enable;
  }

  nextInterrupt(): InterruptType | null {
    if (!this.controller.ime) {
      return null;
    }
    const pending = this.pending;
    // Unrolled bitwise checks for performance. This method runs in the core emulation loop.
    // Preserves priority order without iterating over Object.values(InterruptType).
    if (pending & (1 << InterruptType.VBLANK)) return InterruptType.VBLANK;
    if (pending & (1 << InterruptType.LCD_STAT)) return InterruptType.LCD_STAT;
    if (pending & (1 << InterruptType.TIMER)) return InterruptType.TIMER;
    if (pending & (1 << InterruptType.SERIAL)) return InterruptType.SERIAL;
    if (pending & (1 << InterruptType.JOYPAD)) return InterruptType.JOYPAD;
    return null;
  }

  vectorFor(interrupt: InterruptType): number {
    return INTERRUPT_VECTORS[interrupt];
  }

  clearAll(): void {
    this.controller.interrupt_flags = 0;
    this.controller.interrupt_enable = 0;
    this.controller.ime = false;
  }
}

export const TimerRegistersSchema = z.object({
  divider: z.number().default(0),
  tima: z.number().default(0),
  tma: z.number().default(0),
  tac: z.number().default(0),
  _divider_counter: z.number().default(0),
  _timer_cycles: z.number().default(0),
});

export type TimerRegisters = z.infer<typeof TimerRegistersSchema>;

export class TimerRegistersManager {
  private static readonly TIMER_FREQUENCY_TICKS: Record<number, number> = {
    0: 1024,
    1: 16,
    2: 64,
    3: 256,
  };
  private static readonly TAC_START_MASK = 1 << 2;

  constructor(private registers: TimerRegisters) {}

  tick(cycles: number, interrupts?: InterruptControllerManager): void {
    if (cycles < 0) throw new Error('cycles must be non-negative');
    if (cycles === 0) return;

    this.registers._divider_counter = (this.registers._divider_counter + cycles) & 0xffff;
    this.registers.divider = (this.registers._divider_counter >> 8) & 0xff;

    if (!(this.registers.tac & TimerRegistersManager.TAC_START_MASK)) return;

    const threshold = TimerRegistersManager.TIMER_FREQUENCY_TICKS[this.registers.tac & 0x03];
    this.registers._timer_cycles += cycles;
    while (this.registers._timer_cycles >= threshold) {
      this.registers._timer_cycles -= threshold;
      this.incrementTima(interrupts);
    }
  }

  resetDivider(): void {
    this.registers.divider = 0;
    this.registers._divider_counter = 0;
    this.registers._timer_cycles = 0;
  }

  writeTac(value: number): void {
    this.registers.tac = mask(value);
    this.registers._timer_cycles = 0;
  }

  private incrementTima(interrupts?: InterruptControllerManager): void {
    const previous = this.registers.tima;
    this.registers.tima = (previous + 1) & 0xff;
    if (previous === 0xff) {
      this.registers.tima = this.registers.tma;
      interrupts?.request(InterruptType.TIMER);
    }
  }
}

export enum SerialConnectionStatus {
  USING_EXTERNAL_CLOCK = 0x01,
  USING_INTERNAL_CLOCK = 0x02,
  CONNECTION_NOT_ESTABLISHED = 0xff,
}

export const SerialRegistersSchema = z.object({
  sb: z.number().default(0),
  sc: z.number().default(0),
  received_new_data: z.boolean().default(false),
  connection_status: z.nativeEnum(SerialConnectionStatus).default(SerialConnectionStatus.CONNECTION_NOT_ESTABLISHED),
  ignoring_initial_data: z.boolean().default(false),
  send_byte: z.number().default(0),
  receive_byte: z.number().default(0),
});

export type SerialRegisters = z.infer<typeof SerialRegistersSchema>;

export class SerialRegistersManager {
  constructor(private registers: SerialRegisters) {}

  reset(): void {
    this.registers.sb = 0;
    this.registers.sc = 0;
    this.registers.received_new_data = false;
    this.registers.connection_status = SerialConnectionStatus.CONNECTION_NOT_ESTABLISHED;
    this.registers.ignoring_initial_data = false;
    this.registers.send_byte = 0;
    this.registers.receive_byte = 0;
  }

  loadReceiveByte(value: number): void {
    this.registers.receive_byte = mask(value);
    this.registers.received_new_data = true;
  }

  consumeReceivedData(): number {
    this.registers.received_new_data = false;
    return this.registers.receive_byte;
  }
}

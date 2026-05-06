const ROM_BANK_SIZE = 0x4000;
const OAM_TRANSFER_SIZE = 0xa0;

export class ROMBankSizeError extends Error {}
export class ROMBankNotFoundError extends Error {}
export class StackOverflowError extends Error {}
export class StackUnderflowError extends Error {}
export class DMAAreaError extends Error {}

export enum DMATransferType {
  OAM,
  GENERAL,
}

export interface DMATransfer {
  transferType: DMATransferType;
  sourceOffset: number;
  destOffset: number;
  length: number;
}

export class ROMBank {
  constructor(public bankId: number, public data: Uint8Array) {
    if (data.length !== ROM_BANK_SIZE) {
      throw new ROMBankSizeError(`Bank ${bankId} must be ${ROM_BANK_SIZE} bytes, got ${data.length}`);
    }
  }
}

export class ROMBankController {
  private fixedBank: ROMBank;
  private banks: Map<number, ROMBank> = new Map();
  private activeBankId: number | null = null;

  constructor(fixedBank: Iterable<number> | Uint8Array, switchableBanks?: [number, Iterable<number> | Uint8Array][]) {
    const fixedBytes = fixedBank instanceof Uint8Array ? fixedBank : new Uint8Array(fixedBank);
    if (fixedBytes.length !== ROM_BANK_SIZE) {
      throw new ROMBankSizeError(`Fixed bank must be ${ROM_BANK_SIZE} bytes, got ${fixedBytes.length}`);
    }
    this.fixedBank = new ROMBank(0, fixedBytes);

    if (switchableBanks) {
      for (const [bankId, data] of switchableBanks) {
        this.registerBank(bankId, data);
      }
    }

    if (this.banks.size > 0) {
      const firstBank = this.banks.keys().next().value;
      this.activeBankId = typeof firstBank === "number" ? firstBank : null;
    }
  }

  registerBank(bankId: number, data: Iterable<number> | Uint8Array): void {
    if (bankId === 0) {
      throw new Error('Bank ID 0 is reserved for the fixed ROM bank.');
    }
    const bankBytes = data instanceof Uint8Array ? data : new Uint8Array(data);
    if (bankBytes.length !== ROM_BANK_SIZE) {
      throw new ROMBankSizeError(`Bank ${bankId} must be ${ROM_BANK_SIZE} bytes, got ${bankBytes.length}`);
    }
    this.banks.set(bankId, new ROMBank(bankId, bankBytes));
    if (this.activeBankId === null) {
      this.activeBankId = bankId;
    }
  }

  switchBank(bankId: number): void {
    if (!this.banks.has(bankId)) {
      throw new ROMBankNotFoundError(`Bank ${bankId} is not registered.`);
    }
    this.activeBankId = bankId;
  }

  private resolveBankAndOffset(address: number): [ROMBank, number] {
    if (address >= 0 && address < ROM_BANK_SIZE) {
      return [this.fixedBank, address];
    }
    if (address >= ROM_BANK_SIZE && address < ROM_BANK_SIZE * 2) {
      if (this.activeBankId === null) {
        throw new ROMBankNotFoundError('No swappable ROM bank is active.');
      }
      const bank = this.banks.get(this.activeBankId)!;
      return [bank, address - ROM_BANK_SIZE];
    }
    throw new Error(`Address 0x${address.toString(16).padStart(4, '0')} lies outside the 0x0000-0x7FFF ROM window.`);
  }

  read(address: number, length = 1): Uint8Array {
    if (length <= 0) throw new Error('Length must be positive.');
    const buffer = new Uint8Array(length);
    let remaining = length;
    let current = address;
    let bufferOffset = 0;
    while (remaining > 0) {
      const [bank, offset] = this.resolveBankAndOffset(current);
      const pageRemaining = ROM_BANK_SIZE - offset;
      const chunk = Math.min(remaining, pageRemaining);
      buffer.set(bank.data.subarray(offset, offset + chunk), bufferOffset);
      current += chunk;
      remaining -= chunk;
      bufferOffset += chunk;
    }
    return buffer;
  }

  writeBank(bankId: number, data: Iterable<number> | Uint8Array): void {
    let target: ROMBank;
    if (bankId === 0) {
      target = this.fixedBank;
    } else {
      target = this.banks.get(bankId)!;
      if (!target) {
        throw new ROMBankNotFoundError(`Bank ${bankId} is not registered.`);
      }
    }
    const bankBytes = data instanceof Uint8Array ? data : new Uint8Array(data);
    if (bankBytes.length !== ROM_BANK_SIZE) {
      throw new ROMBankSizeError(`Bank ${bankId} must be ${ROM_BANK_SIZE} bytes, got ${bankBytes.length}`);
    }
    target.data.set(bankBytes);
  }

  get activeBank(): number | null {
    return this.activeBankId;
  }

  get registeredBanks(): number[] {
    return Array.from(this.banks.keys());
  }
}

export class StackMemory {
  private buffer: Uint8Array;
  private sp: number;

  constructor(private size = 0x100) {
    if (size <= 0) throw new Error('Stack size must be positive.');
    this.buffer = new Uint8Array(size);
    this.sp = size;
  }

  get stackPointer(): number {
    return this.sp;
  }

  get depth(): number {
    return this.size - this.sp;
  }

  pushByte(value: number): void {
    if (value < 0 || value > 0xff) throw new Error('Byte value must be 0-0xFF.');
    if (this.sp === 0) throw new StackOverflowError('Stack overflow.');
    this.sp--;
    this.buffer[this.sp] = value;
  }

  popByte(): number {
    if (this.sp === this.size) throw new StackUnderflowError('Stack underflow.');
    const value = this.buffer[this.sp];
    this.buffer[this.sp] = 0;
    this.sp++;
    return value;
  }

  pushWord(value: number): void {
    if (value < 0 || value > 0xffff) throw new Error('Word must fit in 16 bits.');
    const high = (value >> 8) & 0xff;
    const low = value & 0xff;
    this.pushByte(high);
    this.pushByte(low);
  }

  popWord(): number {
    const low = this.popByte();
    const high = this.popByte();
    return (high << 8) | low;
  }

  reset(): void {
    this.buffer.fill(0);
    this.sp = this.size;
  }
}

export class DMAController {
  history: DMATransfer[] = [];

  private validate(buffer: Uint8Array, offset: number, length: number): void {
    if (length <= 0) throw new Error('Length must be positive.');
    if (offset < 0 || offset + length > buffer.length) {
      throw new DMAAreaError(`DMA request exceeds buffer bounds (offset=${offset}, length=${length}).`);
    }
  }

  requestOamTransfer({ source, dest, sourceOffset = 0, destOffset = 0 }: {
    source: Uint8Array;
    dest: Uint8Array;
    sourceOffset?: number;
    destOffset?: number;
  }): number {
    const length = OAM_TRANSFER_SIZE;
    this.validate(source, sourceOffset, length);
    this.validate(dest, destOffset, length);
    dest.set(source.subarray(sourceOffset, sourceOffset + length), destOffset);
    const transfer: DMATransfer = {
      transferType: DMATransferType.OAM,
      sourceOffset,
      destOffset,
      length,
    };
    this.history.push(transfer);
    return length;
  }

  requestGeneralTransfer({ source, dest, length, sourceOffset = 0, destOffset = 0 }: {
    source: Uint8Array;
    dest: Uint8Array;
    length: number;
    sourceOffset?: number;
    destOffset?: number;
  }): number {
    this.validate(source, sourceOffset, length);
    this.validate(dest, destOffset, length);
    dest.set(source.subarray(sourceOffset, sourceOffset + length), destOffset);
    const transfer: DMATransfer = {
      transferType: DMATransferType.GENERAL,
      sourceOffset,
      destOffset,
      length,
    };
    this.history.push(transfer);
    return length;
  }

  lastTransfer(): DMATransfer | null {
    return this.history.length > 0 ? this.history[this.history.length - 1] : null;
  }
}

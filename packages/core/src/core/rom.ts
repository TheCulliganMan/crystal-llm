
export const ROM_HEADER_OFFSETS = {
  TITLE: 0x134,
  CGB_FLAG: 0x143,
  CARTRIDGE_TYPE: 0x147,
  ROM_SIZE: 0x148,
  RAM_SIZE: 0x149,
};

export class Rom {
  private readonly data: Uint8Array;
  private readonly view: DataView;

  constructor(data: ArrayBuffer) {
    this.data = new Uint8Array(data);
    this.view = new DataView(this.data.buffer);
  }

  public readByte(address: number): number {
    return this.view.getUint8(address);
  }

  public readBytes(address: number, length: number): Uint8Array {
    return this.data.subarray(address, address + length);
  }

  public get title(): string {
    const titleBytes = this.readBytes(ROM_HEADER_OFFSETS.TITLE, 16);
    // Find the first null terminator
    const nullIndex = titleBytes.indexOf(0);
    const titleEnd = nullIndex !== -1 ? nullIndex : titleBytes.length;
    // Decode as ASCII
    return new TextDecoder('ascii').decode(titleBytes.subarray(0, titleEnd));
  }

  public get isCgb(): boolean {
    const cgbFlag = this.readByte(ROM_HEADER_OFFSETS.CGB_FLAG);
    return cgbFlag === 0x80 || cgbFlag === 0xc0;
  }

  public get cartridgeType(): number {
    return this.readByte(ROM_HEADER_OFFSETS.CARTRIDGE_TYPE);
  }

  public get romSize(): number {
    const sizeCode = this.readByte(ROM_HEADER_OFFSETS.ROM_SIZE);
    if (sizeCode > 0x08) {
        return 0; // Invalid code
    }
    return (32 * 1024) << sizeCode;
  }

  public get ramSize(): number {
    const sizeCode = this.readByte(ROM_HEADER_OFFSETS.RAM_SIZE);
    switch (sizeCode) {
      case 0x00: return 0;
      case 0x01: return 2 * 1024;
      case 0x02: return 8 * 1024;
      case 0x03: return 32 * 1024;
      case 0x04: return 128 * 1024;
      case 0x05: return 64 * 1024;
      default: return 0; // Invalid code
    }
  }
}

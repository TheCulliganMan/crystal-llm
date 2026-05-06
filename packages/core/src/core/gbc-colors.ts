export type RGB = [number, number, number];

const assertRgb5 = (value: number, context: string): number => {
  if (!Number.isInteger(value) || value < 0 || value > 31) {
    throw new Error(`Expected 5-bit RGB component (0..31) for ${context}, got ${value}.`);
  }
  return value;
};

// Game Boy Color palette data stores 5-bit components. When expanding to 8-bit,
// replicate the high bits into the low bits (common emulator/hardware mapping).
export const gbc5To8 = (value: number, context = "RGB5 component"): number => {
  const component = assertRgb5(value, context);
  return (component << 3) | (component >> 2);
};

// GBC palette word format is xBBBBBGGGGGRRRRR (little-endian in memory).
export const gbcWordToRgb = (word: number): RGB => {
  const raw = word & 0x7fff;
  const r5 = raw & 0x1f;
  const g5 = (raw >> 5) & 0x1f;
  const b5 = (raw >> 10) & 0x1f;
  return [
    gbc5To8(r5, "red"),
    gbc5To8(g5, "green"),
    gbc5To8(b5, "blue"),
  ];
};


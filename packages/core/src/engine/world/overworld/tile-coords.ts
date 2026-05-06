// Shared helpers to map raw ASM tile coordinates to the scaled collision grid.

export const scaleTileCoord = (raw: number, stride: number): number => {
  const step = Math.max(1, Math.trunc(stride));
  const offset = step - 1;
  return Math.trunc(raw) * step + offset;
};

export const unscaleTileCoord = (scaled: number, stride: number): number => {
  const step = Math.max(1, Math.trunc(stride));
  const offset = step - 1;
  return Math.trunc((Math.trunc(scaled) - offset) / step);
};

export const scaleTileCoords = (rawX: number, rawY: number, stride: number): [number, number] => [
  scaleTileCoord(rawX, stride),
  scaleTileCoord(rawY, stride),
];

export const unscaleTileCoords = (scaledX: number, scaledY: number, stride: number): [number, number] => [
  unscaleTileCoord(scaledX, stride),
  unscaleTileCoord(scaledY, stride),
];

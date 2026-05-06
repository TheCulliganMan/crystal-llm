// ASM mapping: pokecrystal_disassembly/engine/overworld/step_types.asm (UpdateJumpPosition).
export const LEDGE_JUMP_OFFSETS: readonly number[] = [
  -4,
  -6,
  -8,
  -10,
  -11,
  -12,
  -12,
  -12,
  -11,
  -10,
  -9,
  -8,
  -6,
  -4,
  0,
  0,
];

const EXPECTED_LEDGE_JUMP_DISTANCE_PX = LEDGE_JUMP_OFFSETS.length * 2;
const MAX_LEDGE_JUMP_OFFSET_INDEX = LEDGE_JUMP_OFFSETS.length - 1;
const MAX_LEDGE_JUMP_HEIGHT_PX = MAX_LEDGE_JUMP_OFFSET_INDEX * 2;

export function getLedgeJumpOffset(
  progressPx: number,
  totalDistancePx?: number,
): number {
  if (totalDistancePx !== undefined) {
    const normalizedTotal = Math.max(Math.trunc(totalDistancePx), 0);
    if (normalizedTotal > 0 && normalizedTotal !== EXPECTED_LEDGE_JUMP_DISTANCE_PX) {
      throw new Error(
        `Ledge jump total distance mismatch: expected ${EXPECTED_LEDGE_JUMP_DISTANCE_PX}, got ${normalizedTotal}.`,
      );
    }
  }

  const normalizedHeightPx = Number.isFinite(progressPx)
    ? Math.max(Math.trunc(progressPx), 0)
    : 0;

  const clampedHeightPx = Math.min(normalizedHeightPx, MAX_LEDGE_JUMP_HEIGHT_PX);
  const jumpIndex = Math.floor(clampedHeightPx / 2);
  return LEDGE_JUMP_OFFSETS[jumpIndex] ?? 0;
}

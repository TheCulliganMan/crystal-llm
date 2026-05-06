// ASM + hardware mapping:
// - One frame lasts exactly 70,224 cycles (one VBlank cadence).
// - CPU clock is 4,194,304 Hz on DMG/CGB.
// - Frame rate = 4,194,304 / 70,224 ~= 59.7275 Hz.
export const GB_CPU_CYCLES_PER_SECOND = 4_194_304;
export const GB_CYCLES_PER_FRAME = 70_224;
export const GB_FRAME_RATE = GB_CPU_CYCLES_PER_SECOND / GB_CYCLES_PER_FRAME;
export const GB_FRAME_DURATION_MS = (GB_CYCLES_PER_FRAME * 1000) / GB_CPU_CYCLES_PER_SECOND;
export const GB_FRAME_DURATION_SECONDS = GB_FRAME_DURATION_MS / 1000;

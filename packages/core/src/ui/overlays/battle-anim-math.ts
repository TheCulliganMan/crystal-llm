import { AnimationSprite } from "./_battle-animation-state";

// ASM mapping: macros/data.asm::BattleAnimSineWave, macros/code.asm::calc_sine_wave.
const _BATTLE_ANIM_SINE_WAVE: readonly number[] = [
  0x0000, 0x0019, 0x0032, 0x004a, 0x0062, 0x0079, 0x008e, 0x00a2,
  0x00b5, 0x00c6, 0x00d5, 0x00e2, 0x00ed, 0x00f5, 0x00fb, 0x00ff,
  0x0100, 0x00ff, 0x00fb, 0x00f5, 0x00ed, 0x00e2, 0x00d5, 0x00c6,
  0x00b5, 0x00a2, 0x008e, 0x0079, 0x0062, 0x004a, 0x0032, 0x0019,
];

export function to_signed_byte(value: number): number {
  const masked = value & 0xff;
  return masked & 0x80 ? masked - 0x100 : masked;
}

function _calc_sine_wave(angle: number, amplitude: number): number {
  const normalizedAngle = angle & 0x3f;
  const negative = (normalizedAngle & 0x20) !== 0;
  const tableIndex = normalizedAngle & 0x1f;

  let a = amplitude & 0xff;
  let de = _BATTLE_ANIM_SINE_WAVE[tableIndex];
  let hl = 0;

  while (a) {
    if (a & 1) {
      hl = (hl + de) & 0xffff;
    }
    a >>= 1;
    de = (de << 1) & 0xffff;
  }

  let result = (hl >> 8) & 0xff;
  if (negative) {
    result = ((result ^ 0xff) + 1) & 0xff;
  }
  return to_signed_byte(result);
}

export function sine(angle: number, amplitude: number): number {
  return _calc_sine_wave(angle, amplitude);
}

export function cosine(angle: number, amplitude: number): number {
  return _calc_sine_wave(angle + 0x10, amplitude);
}

export function step_circle(angle: number, amplitude: number): [number, number] {
  const y = sine(angle, amplitude) >> 2;
  const x = cosine(angle, amplitude);
  return [x, y];
}

export function apply_offsets(sprite: AnimationSprite): void {
  const baseX = (sprite.base_x ?? sprite.x) & 0xff;
  const baseY = (sprite.base_y ?? sprite.y) & 0xff;
  sprite.x = (baseX + sprite.x_offset) & 0xff;
  sprite.y = (baseY + sprite.y_offset) & 0xff;
  sprite.base_x = baseX;
  sprite.base_y = baseY;
}

export function step_thrown_to_target(sprite: AnimationSprite): void {
  const originalVar2 = sprite.var2 & 0xff;
  sprite.var2 = (sprite.var2 - 1) & 0xff;
  sprite.y_offset = sine(originalVar2, 0x20);
  sprite.fix_y = ((sprite.fix_y ?? 0) + 2) & 0xff;

  let position =
    (((sprite.base_x ?? sprite.x) & 0xff) << 8) | (sprite.var1 & 0xff);
  const delta =
    (((sprite.param & 0xf0) >> 4) << 8) | ((sprite.param & 0x0f) << 4);
  position = (position + delta) & 0xffff;
  sprite.var1 = position & 0xff;
  sprite.base_x = (position >> 8) & 0xff;

  if ((sprite.var2 & 0x1) === 0) {
    const baseY = sprite.base_y ?? sprite.y;
    sprite.base_y = (baseY - 1) & 0xff;
  }

  apply_offsets(sprite);
}

export { _BATTLE_ANIM_SINE_WAVE };

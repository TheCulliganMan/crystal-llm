import { AnimationSprite } from './_battle-animation-state';
import type { BattleAnimationRuntime } from './battle-bg-effects';
import {
  sine as _sine,
  cosine as _cosine,
  to_signed_byte as _to_signed_byte,
  step_circle as _step_circle,
  apply_offsets as _apply_offsets,
  step_thrown_to_target as _step_thrown_to_target,
} from './_battle-anim-math';
import * as _impl from './battle-anim-runtime-impl';

// ASM mapping: pokecrystal_disassembly/engine/battle_anims/anim_*.asm (BattleAnimFunc_* runtime).

const BALL_PALETTE_MAP: Record<string, string> = {
  MASTER_BALL: 'PAL_BATTLE_OB_GREEN',
  ULTRA_BALL: 'PAL_BATTLE_OB_YELLOW',
  GREAT_BALL: 'PAL_BATTLE_OB_BLUE',
  POKE_BALL: 'PAL_BATTLE_OB_RED',
  HEAVY_BALL: 'PAL_BATTLE_OB_GRAY',
  LEVEL_BALL: 'PAL_BATTLE_OB_BROWN',
  LURE_BALL: 'PAL_BATTLE_OB_BLUE',
  FAST_BALL: 'PAL_BATTLE_OB_BLUE',
  FRIEND_BALL: 'PAL_BATTLE_OB_YELLOW',
  MOON_BALL: 'PAL_BATTLE_OB_GRAY',
  LOVE_BALL: 'PAL_BATTLE_OB_RED',
};

const POKEBALL_TARGET_X = 0x88;
const POKEBALL_BLOCKED_TARGET_X = 0x70;
const POKEBALL_BLOCKED_FINAL_Y = 0x80;

const _normalize_ball_label = (label: string | null | undefined): string | null => {
  if (!label) {
    return null;
  }
  const trimmed = label.trim();
  if (!trimmed) {
    return null;
  }
  // ASM: pokecrystal_disassembly/data/battle_anims/ball_colors.asm uses underscored identifiers.
  return trimmed.toUpperCase().replace(/[\s-]+/g, '_');
};

const _get_ball_palette = (label: string | null | undefined): string => {
  const normalized = _normalize_ball_label(label);
  if (!normalized) {
    return 'PAL_BATTLE_OB_RED';
  }
  return BALL_PALETTE_MAP[normalized] ?? 'PAL_BATTLE_OB_RED';
};

const _resolve_jump_stage = (sprite: AnimationSprite): number => {
  const jumpStage = sprite.jump_index & 0xff;
  if (jumpStage !== 0) {
    return jumpStage;
  }
  const stateStage = sprite.state & 0xff;
  if (stateStage !== 0) {
    // Backward-compatible bootstrap for constructed test sprites that still seed
    // the active phase through `state`. After the first tick, jump_index owns it.
    sprite.jump_index = stateStage;
    return stateStage;
  }
  return 0;
};

const _animate_throw_to_target = (
  sprite: AnimationSprite,
  options?: { target_x?: number },
): boolean => {
  // ASM: BattleAnimFunc_ThrowFromUserToTarget (engine/battle_anims/functions.asm).
  const targetX = options?.target_x ?? POKEBALL_TARGET_X;
  const currentX = sprite.x & 0xff;
  if (currentX >= targetX) {
    return false;
  }
  sprite.x = (currentX + 2) & 0xff;
  sprite.y = (sprite.y - 1) & 0xff;
  const angle = sprite.var1 & 0xff;
  sprite.var1 = (angle - 1) & 0xff;
  const amplitude = (sprite.param ?? 0) & 0xff;
  sprite.y_offset = _sine(angle, amplitude);
  return true;
};

const _run_pokeball_wobble_stage4 = (sprite: AnimationSprite): void => {
  const angle = sprite.var1 & 0xff;
  const amplitude = sprite.var2 || 0;
  sprite.y_offset = _sine(angle, amplitude);
  const nextAngle = (angle - 1) & 0xff;
  sprite.var1 = nextAngle;
  if (nextAngle & 0x1f) {
    return;
  }
  sprite.var1 = 0;
  const remaining = Math.max(0, (sprite.var2 || 0) - 4);
  sprite.var2 = remaining;
  if (remaining > 0) {
    return;
  }
  sprite.override_frameset = 'BATTLE_ANIM_FRAMESET_POKE_BALL_4';
  sprite.state = 5;
  sprite.jump_index = 5;
};

const _run_pokeball_wobble_stage8 = (
  sprite: AnimationSprite,
  stage: number,
): 'remove' | null => {
  const angle = sprite.var1 & 0xff;
  const amplitude = sprite.var2 || 0;
  sprite.y_offset = _sine(angle, amplitude);
  const nextAngle = (angle - 1) & 0xff;
  sprite.var1 = nextAngle;
  if ((nextAngle & 0x1f) === 0) {
    sprite.state = 11;
    sprite.jump_index = 11;
    return 'remove';
  }
  if ((nextAngle & 0x0f) === 0) {
    const nextStage = (stage + 1) & 0xff;
    sprite.state = nextStage;
    sprite.jump_index = nextStage;
  }
  return null;
};

export const compute_pokeball_anim_var = (
  sprite: AnimationSprite,
  targetShakes: number | null,
): number => {
  sprite.wobble_count = Math.max(0, sprite.wobble_count) + 1;
  const target = Math.max(0, Math.min(targetShakes ?? 0, 4));
  if (target >= 4) {
    return sprite.wobble_count >= 4 ? 1 : 0;
  }
  if (sprite.wobble_count > target) {
    return 2;
  }
  return 0;
};

const _update_pokeball = (sprite: AnimationSprite): 'remove' | null => {
  const stage = _resolve_jump_stage(sprite);
  sprite.state = stage;
  if (stage === 0) {
    sprite.palette_override = _get_ball_palette(sprite.param_label);
    sprite.state = 1;
    sprite.jump_index = 1;
    return null;
  }
  if (stage === 1) {
    if (_animate_throw_to_target(sprite)) {
      return null;
    }
    sprite.y = (sprite.y + sprite.y_offset) & 0xff;
    sprite.y_offset = 0;
    sprite.override_frameset = 'BATTLE_ANIM_FRAMESET_POKE_BALL_3';
    sprite.state = 2;
    sprite.jump_index = 2;
    return null;
  }
  if (stage === 3) {
    sprite.override_frameset = 'BATTLE_ANIM_FRAMESET_POKE_BALL_1';
    sprite.var1 = 0;
    sprite.var2 = 0x10;
    sprite.state = 4;
    sprite.jump_index = 4;
  }
  if (stage === 4) {
    _run_pokeball_wobble_stage4(sprite);
    _apply_offsets(sprite);
    return null;
  }
  if (stage === 6) {
    sprite.override_frameset = 'BATTLE_ANIM_FRAMESET_POKE_BALL_5';
    sprite.state = 5;
    sprite.jump_index = 5;
    return null;
  }
  if (stage === 7) {
    sprite.palette_override = _get_ball_palette(sprite.param_label);
    sprite.override_frameset = 'BATTLE_ANIM_FRAMESET_POKE_BALL_2';
    sprite.var2 = 0x20;
    sprite.state = 8;
    sprite.jump_index = 8;
  }
  if (stage === 8 || stage === 10) {
    const result = _run_pokeball_wobble_stage8(sprite, stage);
    _apply_offsets(sprite);
    return result;
  }
  if (stage === 11) {
    sprite.state = 11;
    return 'remove';
  }
  return null;
};

const _update_pokeball_blocked = (sprite: AnimationSprite): 'remove' | null => {
  const stage = _resolve_jump_stage(sprite);
  sprite.state = stage;
  if (stage === 0) {
    sprite.palette_override = _get_ball_palette(sprite.param_label);
    sprite.state = 1;
    sprite.jump_index = 1;
    return null;
  }
  if (stage === 1) {
    if (_animate_throw_to_target(sprite, { target_x: POKEBALL_BLOCKED_TARGET_X })) {
      return null;
    }
    sprite.state = 2;
    sprite.jump_index = 2;
    return null;
  }
  if (stage === 2) {
    const currentY = sprite.y & 0xff;
    if (currentY >= POKEBALL_BLOCKED_FINAL_Y) {
      return 'remove';
    }
    sprite.y = (currentY + 4) & 0xff;
    const currentX = sprite.x & 0xff;
    sprite.x = (currentX - 2) & 0xff;
  }
  return null;
};

const _update_ember = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.jump_index === 0) {
    sprite.jump_index = (sprite.param >> 4) & 0x0f;
  }
  if (sprite.jump_index === 1) {
    let step = sprite.param & 0x0f;
    if (step <= 0) {
      step = 1;
    }
    sprite.x = (sprite.x + step) & 0xff;
    sprite.y = (sprite.y - (step >> 1)) & 0xff;
  } else if (sprite.jump_index === 2) {
    return 'remove';
  } else if (sprite.jump_index === 3) {
    sprite.jump_index = 4;
    sprite.override_frameset = 'BATTLE_ANIM_FRAMESET_FLAMETHROWER';
  }
  return null;
};

const _update_drop = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0x30;
    sprite.var2 = 0x48;
  }
  const angle = sprite.var1 & 0xff;
  const amplitude = sprite.var2;
  sprite.y_offset = _sine(angle, amplitude);
  _apply_offsets(sprite);
  sprite.var1 = (sprite.var1 + 1) & 0xff;
  if (sprite.var1 & 0x3f) {
    return null;
  }
  sprite.var1 = 0x20;
  sprite.var2 = Math.max(0, sprite.var2 - (sprite.param || 0));
  if (sprite.var2 <= 0) {
    return 'remove';
  }
  return null;
};

const _update_rapid_spin = (sprite: AnimationSprite): 'remove' | null => {
  sprite.y_offset = (sprite.y_offset - 4) & 0xff;
  _apply_offsets(sprite);
  if (sprite.y_offset === 0xd0) {
    return 'remove';
  }
  return null;
};

const _update_recover = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var2 = (sprite.param & 0xf0) & 0xff;
    sprite.var1 = ((sprite.param & 0x0f) << 3) & 0xff;
    sprite.param = 1;
  }

  if (sprite.var2 === 0) {
    return 'remove';
  }

  const angle = sprite.var1 & 0xff;
  const amplitude = sprite.var2 & 0xff;
  sprite.var1 = (sprite.var1 + 1) & 0xff;
  sprite.y_offset = _sine(angle, amplitude);
  sprite.x_offset = _cosine(angle, amplitude);

  sprite.param ^= 1;
  if (sprite.param) {
    sprite.var2 = Math.max(0, sprite.var2 - 1);
  }

  _apply_offsets(sprite);
  return null;
};

const _update_razor_wind = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = sprite.param & 0x80 ? 0x20 : 0x00;
    sprite.param &= 0x7f;
  }

  const angle = sprite.var1 & 0xff;
  const amplitude = sprite.param || 0;
  sprite.y_offset = _sine(angle, amplitude);
  sprite.x_offset = _cosine(angle, amplitude);
  sprite.var1 = (sprite.var1 + 0x10) & 0xff;
  _apply_offsets(sprite);
  return null;
};

const _update_spikes = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var2 = 0x40;
    return null;
  }

  if (sprite.state === 1) {
    if (sprite.var2 >= 0x20) {
      _step_thrown_to_target(sprite);
    } else {
      sprite.state = 2;
    }
  }
  return null;
};

const _update_cotton = (sprite: AnimationSprite): 'remove' | null => {
  sprite.var2 = (sprite.var2 + 1) & 0xff;
  const angle = ((sprite.var2 >> 1) + (sprite.param & 0xff)) & 0xff;
  const [xOffset, yOffset] = _step_circle(angle, 0x18);
  sprite.x_offset = xOffset;
  sprite.y_offset = yOffset;
  _apply_offsets(sprite);
  return null;
};

const _update_ancient_power = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0;
  }

  if (sprite.var1 >= 0x20) {
    return 'remove';
  }

  sprite.var1 = (sprite.var1 + 1) & 0xff;
  const amplitude = sprite.param & 0xff;
  sprite.y_offset = (_sine(sprite.var1, amplitude) ^ 0xff) + 1;
  _apply_offsets(sprite);
  return null;
};

const _update_powder = (sprite: AnimationSprite): 'remove' | null => {
  const currentY = sprite.y_offset & 0xff;
  if (currentY >= 0x38) {
    return 'remove';
  }

  const fixed = (((currentY << 8) | (sprite.var1 & 0xff)) + 0x80) & 0xffff;
  sprite.var1 = fixed & 0xff;
  sprite.y_offset = (fixed >> 8) & 0xff;
  sprite.x_offset = _to_signed_byte((sprite.x_offset & 0xff) ^ 0x10);
  _apply_offsets(sprite);
  return null;
};

const _update_spin = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
  }
  if (sprite.state === 1) {
    sprite.state = 2;
    sprite.var1 = 0;
  }
  if (sprite.state === 2) {
    const angle = sprite.var1 & 0xff;
    if (angle >= 0x40) {
      const high = sprite.param & 0xf0;
      if (high) {
        sprite.param = ((high - 0x10) & 0xf0) | (sprite.param & 0x0f);
        sprite.state = 1;
        return null;
      }
      sprite.state = 3;
      return null;
    }
    let step = sprite.param & 0x0f;
    if (step === 0) {
      step = 4;
    }
    sprite.x_offset = (_cosine(angle, 0x18) - 0x18) >> 1;
    sprite.y_offset = _sine(angle, 0x18);
    sprite.var1 = (sprite.var1 + step) & 0xff;
    _apply_offsets(sprite);
    return null;
  }
  if (sprite.state === 3) {
    sprite.base_x = ((sprite.base_x ?? sprite.x) + 4) & 0xff;
    _apply_offsets(sprite);
    const baseX = sprite.base_x ?? sprite.x;
    if (baseX >= 0xb0) {
      return 'remove';
    }
  }
  return null;
};

const _update_water_gun = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = sprite.param || 0;
  }
  if (sprite.state === 1) {
    const currentY = (sprite.base_y ?? sprite.y) & 0xff;
    if (currentY < 0x30) {
      sprite.state = 2;
      sprite.override_frameset = 'BATTLE_ANIM_FRAMESET_WATER_GUN_2';
      sprite.base_y = 0x30;
      sprite.y_offset = 0;
      sprite.mirror_x = false;
      sprite.mirror_y = false;
      _apply_offsets(sprite);
      return null;
    }
    sprite.base_x = ((sprite.base_x ?? sprite.x) + 2) & 0xff;
    sprite.base_y = (currentY - 1) & 0xff;
    sprite.var1 = (sprite.var1 - 1) & 0xff;
    sprite.y_offset = _sine(sprite.var1, 8);
    _apply_offsets(sprite);
    return null;
  }
  if (sprite.state === 2) {
    sprite.y_offset += 1;
    if (sprite.y_offset >= 0x18) {
      sprite.state = 3;
      sprite.override_frameset = 'BATTLE_ANIM_FRAMESET_WATER_GUN_3';
    }
    _apply_offsets(sprite);
    return null;
  }
  return null;
};

const _update_bite = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = sprite.param & 0x80 ? 0x30 : 0x10;
    sprite.var2 = (sprite.param & 0x7f) || 0x10;
  }
  const angle = sprite.var1 & 0xff;
  const yOffset = _sine(angle, sprite.var2);
  sprite.y_offset = yOffset;
  sprite.override_frameset =
    yOffset >= 0 ? 'BATTLE_ANIM_FRAMESET_BITE_2' : 'BATTLE_ANIM_FRAMESET_BITE_1';
  sprite.var1 = (sprite.var1 + 2) & 0xff;
  _apply_offsets(sprite);
  if (sprite.var1 & 0x1f) {
    return null;
  }
  sprite.state += 1;
  if (sprite.state >= 6) {
    return 'remove';
  }
  return null;
};

const _update_move_in_circle = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = sprite.param & 0x80 ? 0x20 : 0x00;
    sprite.param &= 0x7f;
  }
  const angle = sprite.var1 & 0xff;
  const amplitude = sprite.param || 0;
  sprite.y_offset = _sine(angle, amplitude);
  sprite.x_offset = _cosine(angle, amplitude);
  sprite.var1 = (sprite.var1 + 1) & 0xff;
  _apply_offsets(sprite);
  return null;
};

const _update_move_up = (sprite: AnimationSprite): 'remove' | null => {
  const yOffset = sprite.y_offset & 0xff;
  if (yOffset !== 0 && yOffset < 0xd8) {
    return 'remove';
  }
  const speed = sprite.param || 0;
  sprite.y_offset = (yOffset - speed) & 0xff;
  _apply_offsets(sprite);
  return null;
};

const _update_user_to_target = (
  sprite: AnimationSprite,
  options: { disappear: boolean; speed_override?: number | null },
): 'remove' | null => {
  const speedRaw =
    options.speed_override !== undefined && options.speed_override !== null
      ? options.speed_override
      : sprite.param || 2;
  const speed = speedRaw <= 0 ? 1 : speedRaw;
  const targetX = 0x84;
  const baseY = sprite.base_y ?? sprite.y;
  const yStep = Math.max(1, Math.floor(speed / 2));
  sprite.base_y = (baseY - yStep) & 0xff;
  const baseX = sprite.base_x ?? sprite.x;
  sprite.base_x = (baseX + speed) & 0xff;
  _apply_offsets(sprite);
  const currentX = sprite.base_x ?? sprite.x;
  if (sprite.jump_index > 0) {
    return 'remove';
  }
  if (currentX >= targetX) {
    sprite.jump_index = 1;
    return 'remove';
  }
  return null;
};

const _update_wave_to_target = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0;
  }
  sprite.var1 = (sprite.var1 + 4) & 0xff;
  sprite.y_offset = _sine(sprite.var1, 6);
  return _update_user_to_target(sprite, { disappear: false });
};

const _update_shake = (sprite: AnimationSprite): 'remove' | null => {
  let amplitude = sprite.param & 0x0f;
  if (amplitude === 0) {
    amplitude = 2;
  }
  sprite.x_offset = sprite.state % 2 === 0 ? amplitude : -amplitude;
  sprite.state = (sprite.state + 1) & 0xff;
  if (sprite.state >= 0x20) {
    sprite.x_offset = 0;
    return 'remove';
  }
  _apply_offsets(sprite);
  return null;
};

const _update_fire_blast = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = sprite.param & 0xff;
    if (sprite.state !== 7) {
      sprite.override_frameset = 'BATTLE_ANIM_FRAMESET_BURNED';
    }
    return null;
  }
  if (sprite.state === 7) {
    if ((sprite.x & 0xff) < 0x88) {
      sprite.base_x = ((sprite.base_x ?? sprite.x) + 2) & 0xff;
      sprite.base_y = ((sprite.base_y ?? sprite.y) - 1) & 0xff;
      _apply_offsets(sprite);
      return null;
    }
    sprite.state = 8;
    sprite.override_frameset = 'BATTLE_ANIM_FRAMESET_EMBER';
  }
  if (sprite.state === 8) {
    const angle = sprite.var1 & 0xff;
    sprite.y_offset = _sine(angle, 0x10);
    sprite.x_offset = _cosine(angle, 0x10);
    sprite.var1 = (angle + 1) & 0xff;
    _apply_offsets(sprite);
    return null;
  }
  if (sprite.state === 9) {
    return 'remove';
  }
  if (sprite.state === 1) {
    sprite.y_offset = ((sprite.y_offset ?? 0) - 1) & 0xff;
    return null;
  }
  if (sprite.state === 4 || sprite.state === 5) {
    sprite.y_offset = ((sprite.y_offset ?? 0) + 1) & 0xff;
  }
  if (sprite.state === 2 || sprite.state === 4) {
    sprite.x_offset = ((sprite.x_offset ?? 0) - 1) & 0xff;
    return null;
  }
  if (sprite.state === 3 || sprite.state === 5) {
    sprite.x_offset = ((sprite.x_offset ?? 0) + 1) & 0xff;
    return null;
  }
  return null;
};

const _step_to_target = (sprite: AnimationSprite, param: number): void => {
  const step = param & 0x0f;
  const nextX = (sprite.x + step) & 0xff;
  sprite.x = nextX;
  let yStep = step >> 1;
  while (yStep > 0) {
    sprite.y = (sprite.y - 1) & 0xff;
    yStep -= 1;
  }
};

const _update_bubble = (sprite: AnimationSprite): 'remove' | null => {
  // ASM mapping: pokecrystal_disassembly/engine/battle_anims/functions.asm::BattleAnimFunc_Bubble
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0x0c;
  }
  if (sprite.state === 1) {
    if ((sprite.var1 & 0xff) !== 0) {
      sprite.var1 = (sprite.var1 - 1) & 0xff;
      _step_to_target(sprite, sprite.param);
      return null;
    }
    sprite.state = 2;
    sprite.var1 = 0;
    sprite.override_frameset = 'BATTLE_ANIM_FRAMESET_PULSING_BUBBLE';
  }
  if (sprite.state === 2) {
    if (sprite.x < 0x98) {
      const xFrac = sprite.var1 & 0xff;
      const position =
        ((((sprite.x & 0xff) << 8) | xFrac) + 0x60) & 0xffff;
      sprite.var1 = position & 0xff;
      sprite.x = (position >> 8) & 0xff;
    }
    if ((sprite.y & 0xff) < 0x20) {
      return null;
    }
    const delta = (0xff00 | (sprite.param & 0xf0)) & 0xffff;
    const yFrac = sprite.var2 & 0xff;
    const position =
      ((((sprite.y & 0xff) << 8) | yFrac) + delta) & 0xffff;
    sprite.var2 = position & 0xff;
    sprite.y = (position >> 8) & 0xff;
  }
  return null;
};

const _update_thunder_wave = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0;
  }
  sprite.var1 = (sprite.var1 + 6) & 0xff;
  sprite.y_offset = _sine(sprite.var1, 6);
  return _update_user_to_target(sprite, { disappear: true, speed_override: 2 });
};

const _scatter_horizontal = (param: number): number => {
  const radius = param & 0x3f;
  if ((param & 0x80) === 0) {
    if (radius >= 0x20) {
      return 0x100;
    }
    if (radius >= 0x18) {
      return 0x180;
    }
    return 0x200;
  }
  if (radius >= 0x20) {
    return -0x100;
  }
  if (radius >= 0x18) {
    return -0x180;
  }
  return -0x200;
};

const _update_razor_leaf = (sprite: AnimationSprite): 'remove' | null => {
  const jumpIndex = sprite.jump_index & 0xff;
  if (jumpIndex === 0) {
    sprite.jump_index = 1;
    sprite.var1 = 0x40;
  }

  if (sprite.jump_index === 1) {
    const angle = sprite.var1 & 0xff;
    if (angle < 0x30) {
      sprite.jump_index = 2;
      sprite.var1 = 0;
      sprite.var2 = 0;
      sprite.override_frameset = 'BATTLE_ANIM_FRAMESET_RAZOR_LEAF_2';
      if ((sprite.param & 0x40) !== 0) {
        // Start from the flipped half of Frameset_RazorLeaf2, matching
        // engine/battle_anims/functions.asm::BattleAnimFunc_RazorLeaf.
        sprite.active_frameset = sprite.override_frameset;
        sprite.frameset_index = 4;
        sprite.frame_duration = 0;
      }
      return null;
    }
    const radius = sprite.param & 0x3f;
    sprite.var1 = (angle - 1) & 0xff;
    sprite.y_offset = _sine(angle, radius);
    const position =
      ((((sprite.x & 0xff) << 8) | (sprite.var2 & 0xff)) + _scatter_horizontal(sprite.param)) & 0xffff;
    sprite.x = (position >> 8) & 0xff;
    sprite.var2 = position & 0xff;
    return null;
  }

  if (sprite.jump_index === 2) {
    if ((sprite.y_offset & 0xff) === 0x20) {
      return 'remove';
    }
    const angle = sprite.var1 & 0xff;
    sprite.x_offset = _sine(angle, 0x10);
    sprite.var1 =
      (sprite.param & 0x40) !== 0
        ? (angle - 1) & 0xff
        : (angle + 1) & 0xff;
    const position =
      ((((sprite.y_offset & 0xff) << 8) | (sprite.var2 & 0xff)) + 0x80) & 0xffff;
    sprite.y_offset = (position >> 8) & 0xff;
    sprite.var2 = position & 0xff;
    return null;
  }

  if (sprite.jump_index === 3) {
    sprite.override_frameset = 'BATTLE_ANIM_FRAMESET_RAZOR_LEAF_1';
    sprite.mirror_x = false;
  }

  if (sprite.jump_index >= 3 && sprite.jump_index <= 7) {
    sprite.jump_index = (sprite.jump_index + 1) & 0xff;
    return null;
  }

  if (sprite.jump_index === 8) {
    if ((sprite.x & 0xff) < 0xc0) {
      _step_to_target(sprite, 0x08);
    }
    return null;
  }
  return null;
};

const _update_solar_beam = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0x28;
    sprite.var2 = 0;
    return null;
  }

  const radius = sprite.var1 & 0xff;
  sprite.y_offset = _sine(sprite.param & 0xff, radius);
  sprite.x_offset = _cosine(sprite.param & 0xff, radius);
  if (radius === 0) {
    return 'remove';
  }
  const next = ((((sprite.var1 & 0xff) << 8) | (sprite.var2 & 0xff)) - 0x80) & 0xffff;
  sprite.var1 = (next >> 8) & 0xff;
  sprite.var2 = next & 0xff;
  return null;
};

const _update_dig = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0;
  }
  if (sprite.state === 1) {
    sprite.y_offset += 2;
    if (sprite.y_offset >= 24) {
      sprite.state = 2;
    }
    _apply_offsets(sprite);
    return null;
  }
  sprite.y_offset = Math.max(0, sprite.y_offset - 2);
  _apply_offsets(sprite);
  if (sprite.y_offset === 0) {
    return 'remove';
  }
  return null;
};

const _update_surf = (sprite: AnimationSprite, runtime?: BattleAnimationRuntime): 'remove' | null => {
  // ASM: engine/battle_anims/functions.asm::BattleAnimFunc_Surf
  const state = sprite.state ?? 0;
  if (state === 0) {
    sprite.state = 1;
    if (runtime) {
      runtime.lcd_pointer = 'scy';
      runtime.ly_override_start = 0x58;
      runtime.ly_override_end = 0x5e;
    }
    return null;
  }

  if (runtime) {
    runtime.lcd_pointer = 'scy';
    runtime.ly_override_end = 0x5e;
  }

  if (state === 1) {
    const target = sprite.param & 0xff;
    const y = sprite.y & 0xff;
    if (y < target) {
      sprite.state = 2;
      if (runtime) {
        runtime.ly_override_start = 0;
      }
      return null;
    }
    sprite.y = (y - 1) & 0xff;
    const angle = sprite.var1 & 0xff;
    sprite.y_offset = _sine(angle, 0x10);
    const start = sprite.y + sprite.y_offset - 0x10;
    if (start >= 0 && runtime) {
      runtime.ly_override_start = start & 0xff;
    }
    sprite.x_offset = ((sprite.x_offset ?? 0) + 1) & 0x07;
    sprite.var1 = (angle + 2) & 0xff;
    return null;
  }

  if (state === 2) {
    if (runtime) {
      runtime.ly_override_start = 0;
    }
    return null;
  }

  if (state === 3) {
    const y = sprite.y & 0xff;
    if (y >= 0x70) {
      if (runtime) {
        runtime.lcd_pointer = null;
        runtime.ly_override_start = 0;
        runtime.ly_override_end = 0;
      }
      return 'remove';
    }
    sprite.y = (y + 2) & 0xff;
    const start = sprite.y - 0x10;
    if (start >= 0 && runtime) {
      runtime.ly_override_start = start & 0xff;
    }
    return null;
  }

  if (runtime) {
    runtime.lcd_pointer = null;
    runtime.ly_override_start = 0;
    runtime.ly_override_end = 0;
  }
  return 'remove';
};

const _update_gust = (sprite: AnimationSprite): 'remove' | null => {
  const radii = [8, 6, 5, 4, 5, 6, 8, 12, 16];
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.param = 0;
    sprite.var1 = 0;
    sprite.var2 = 0;
  }
  const radius = radii[sprite.var2 % radii.length];
  const angle = sprite.var1 & 0xff;
  sprite.y_offset = (_sine(angle, radius) >> 4) + sprite.param;
  sprite.x_offset = _cosine(angle, radius);
  sprite.var1 = (sprite.var1 - 8) & 0xff;
  if (sprite.param >= 0xc2) {
    sprite.param = 0;
    sprite.var2 = 0;
    sprite.x_offset = 0;
    sprite.y_offset = 0;
  } else {
    sprite.param = (sprite.param - 1) & 0xff;
    if ((sprite.param & 0x7) === 0) {
      sprite.var2 = (sprite.var2 + 1) % radii.length;
    }
  }
  _apply_offsets(sprite);
  if (sprite.state === 1 || sprite.state === 3) {
    return null;
  }
  const limit = sprite.state === 2 ? 0x88 : 0xb8;
  const currentX = sprite.base_x ?? sprite.x;
  if (currentX >= limit) {
    if (sprite.state === 4) {
      return 'remove';
    }
    sprite.state = 3;
    return null;
  }
  sprite.base_x = (currentX + 1) & 0xff;
  if (((sprite.base_x ?? sprite.x) & 0x1) === 0) {
    sprite.base_y = ((sprite.base_y ?? sprite.y) - 1) & 0xff;
  }
  _apply_offsets(sprite);
  return null;
};

const _update_absorb = (sprite: AnimationSprite): 'remove' | null => {
  let speed = sprite.param & 0x0f;
  if (speed <= 0) {
    speed = 2;
  }
  sprite.base_x = ((sprite.base_x ?? sprite.x) - speed) & 0xff;
  const yStep = Math.max(1, Math.floor(speed / 2));
  sprite.base_y = ((sprite.base_y ?? sprite.y) + yStep) & 0xff;
  _apply_offsets(sprite);
  const baseX = sprite.base_x ?? sprite.x;
  if (baseX < 0x30) {
    return 'remove';
  }
  return null;
};

const _update_absorb_circle = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = sprite.var1 || 0x40;
  }
  const angle = sprite.param & 0xff;
  const radius = Math.max(0, sprite.var1);
  sprite.x_offset = _cosine(angle, radius);
  sprite.y_offset = _sine(angle, radius);
  sprite.param = (sprite.param + 1) & 0xff;
  if ((sprite.param & 1) === 0) {
    sprite.base_x = ((sprite.base_x ?? sprite.x) - 1) & 0xff;
  }
  if ((sprite.param & 3) === 0) {
    sprite.base_y = ((sprite.base_y ?? sprite.y) + 1) & 0xff;
  }
  const baseX = sprite.base_x ?? sprite.x;
  if (baseX >= 0x5a) {
    sprite.var1 = Math.min(0x60, sprite.var1 + 1);
  } else {
    sprite.var1 = Math.max(0, sprite.var1 - 1);
  }
  _apply_offsets(sprite);
  if (sprite.var1 <= 0) {
    return 'remove';
  }
  return null;
};

const _update_sound = (sprite: AnimationSprite): 'remove' | null => {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 8;
    const framesetBase = 'BATTLE_ANIM_FRAMESET_SOUND_1';
    if (!sprite.is_player_move) {
      sprite.param = ((sprite.param ^ 0xff) + 3) & 0xff;
    }
    const offset = sprite.param & 0x3;
    if (offset) {
      sprite.override_frameset = `${framesetBase.slice(0, -1)}${offset + 1}`;
    }
    return null;
  }
  if (sprite.var1 <= 0) {
    return 'remove';
  }
  sprite.var1 -= 1;
  const angle = sprite.var2 & 0xff;
  sprite.var2 = (sprite.var2 + 2) & 0xff;
  sprite.x_offset = _sine(angle, 0x10);
  const angleKind = sprite.param & 0x3;
  if (angleKind === 0) {
    sprite.y_offset = -sprite.x_offset;
  } else if (angleKind >= 2) {
    sprite.y_offset = sprite.x_offset;
  }
  _apply_offsets(sprite);
  return null;
};

const _update_user_to_target_default = (sprite: AnimationSprite): 'remove' | null =>
  _update_user_to_target(sprite, { disappear: false });

const _update_user_to_target_remove = (sprite: AnimationSprite): 'remove' | null =>
  _update_user_to_target(sprite, { disappear: true });

const _update_throw_to_target_remove = (sprite: AnimationSprite): 'remove' | null =>
  _update_user_to_target(sprite, { disappear: true, speed_override: 6 });

type AnimationHandler = (sprite: AnimationSprite, runtime?: BattleAnimationRuntime) => string | null;

const ANIMATION_HANDLERS: Record<string, AnimationHandler> = {
  BATTLE_ANIM_FUNC_NULL: _impl._update_battle_anim_null,
  BATTLE_ANIM_FUNC_USER_TO_TARGET: _update_user_to_target_default,
  BATTLE_ANIM_FUNC_USER_TO_TARGET_DISAPPEAR: _update_user_to_target_remove,
  BATTLE_ANIM_FUNC_MOVE_IN_CIRCLE: _update_move_in_circle,
  BATTLE_ANIM_FUNC_WAVE_TO_TARGET: _update_wave_to_target,
  BATTLE_ANIM_FUNC_THROW_TO_TARGET: _impl._update_throw_to_target,
  BATTLE_ANIM_FUNC_THROW_TO_TARGET_DISAPPEAR: _update_throw_to_target_remove,
  BATTLE_ANIM_FUNC_DROP: _update_drop,
  BATTLE_ANIM_FUNC_USER_TO_TARGET_SPIN: _update_spin,
  BATTLE_ANIM_FUNC_SHAKE: _update_shake,
  BATTLE_ANIM_FUNC_FIRE_BLAST: _update_fire_blast,
  BATTLE_ANIM_FUNC_RAZOR_LEAF: _update_razor_leaf,
  BATTLE_ANIM_FUNC_BUBBLE: _update_bubble,
  BATTLE_ANIM_FUNC_SURF: _update_surf,
  BATTLE_ANIM_FUNC_SING: _impl._update_sing,
  BATTLE_ANIM_FUNC_WATER_GUN: _update_water_gun,
  BATTLE_ANIM_FUNC_EMBER: _update_ember,
  BATTLE_ANIM_FUNC_POWDER: _update_powder,
  BATTLE_ANIM_FUNC_POKEBALL: _update_pokeball,
  BATTLE_ANIM_FUNC_POKEBALL_BLOCKED: _update_pokeball_blocked,
  BATTLE_ANIM_FUNC_RECOVER: _update_recover,
  BATTLE_ANIM_FUNC_THUNDER_WAVE: _update_thunder_wave,
  BATTLE_ANIM_FUNC_CLAMP_ENCORE: _impl._update_clamp_encore,
  BATTLE_ANIM_FUNC_BITE: _update_bite,
  BATTLE_ANIM_FUNC_SOLAR_BEAM: _update_solar_beam,
  BATTLE_ANIM_FUNC_GUST: _update_gust,
  BATTLE_ANIM_FUNC_RAZOR_WIND: _update_razor_wind,
  BATTLE_ANIM_FUNC_KICK: _impl._update_kick,
  BATTLE_ANIM_FUNC_ABSORB: _update_absorb,
  BATTLE_ANIM_FUNC_EGG: _impl._update_egg,
  BATTLE_ANIM_FUNC_MOVE_UP: _update_move_up,
  BATTLE_ANIM_FUNC_WRAP: _impl._update_wrap,
  BATTLE_ANIM_FUNC_LEECH_SEED: _impl._update_leech_seed,
  BATTLE_ANIM_FUNC_SOUND: _update_sound,
  BATTLE_ANIM_FUNC_CONFUSE_RAY: _impl._update_confuse_ray,
  BATTLE_ANIM_FUNC_DIZZY: _impl._update_dizzy,
  BATTLE_ANIM_FUNC_AMNESIA: _impl._update_amnesia,
  BATTLE_ANIM_FUNC_FLOAT_UP: _impl._update_float_up,
  BATTLE_ANIM_FUNC_DIG: _update_dig,
  BATTLE_ANIM_FUNC_STRING: _impl._update_string,
  BATTLE_ANIM_FUNC_PARALYZED: _impl._update_paralyzed,
  BATTLE_ANIM_FUNC_SPIRAL_DESCENT: _impl._update_spiral_descent,
  BATTLE_ANIM_FUNC_POISON_GAS: _impl._update_poison_gas,
  BATTLE_ANIM_FUNC_HORN: _impl._update_horn,
  BATTLE_ANIM_FUNC_NEEDLE: _impl._update_needle,
  BATTLE_ANIM_FUNC_PETAL_DANCE: _impl._update_petal_dance,
  BATTLE_ANIM_FUNC_THIEF_PAYDAY: _impl._update_thief_payday,
  BATTLE_ANIM_FUNC_ABSORB_CIRCLE: _update_absorb_circle,
  BATTLE_ANIM_FUNC_BONEMERANG: _impl._update_bonemerang,
  BATTLE_ANIM_FUNC_SHINY: _impl._update_shiny,
  BATTLE_ANIM_FUNC_SKY_ATTACK: _impl._update_sky_attack,
  BATTLE_ANIM_FUNC_GROWTH_SWORDS_DANCE: _impl._update_growth_swords_dance,
  BATTLE_ANIM_FUNC_SMOKE_FLAME_WHEEL: _impl._update_smoke_flame_wheel,
  BATTLE_ANIM_FUNC_PRESENT_SMOKESCREEN: _impl._update_present_smokescreen,
  BATTLE_ANIM_FUNC_STRENGTH_SEISMIC_TOSS: _impl._update_strength_seismic_toss,
  BATTLE_ANIM_FUNC_SPEED_LINE: _impl._update_speed_line,
  BATTLE_ANIM_FUNC_SLUDGE: _impl._update_sludge,
  BATTLE_ANIM_FUNC_METRONOME_HAND: _impl._update_metronome_hand,
  BATTLE_ANIM_FUNC_METRONOME_SPARKLE_SKETCH: _impl._update_metronome_sparkle_sketch,
  BATTLE_ANIM_FUNC_AGILITY: _impl._update_agility,
  BATTLE_ANIM_FUNC_SACRED_FIRE: _impl._update_sacred_fire,
  BATTLE_ANIM_FUNC_SAFEGUARD_PROTECT: _impl._update_safeguard_protect,
  BATTLE_ANIM_FUNC_LOCK_ON_MIND_READER: _impl._update_lock_on_mind_reader,
  BATTLE_ANIM_FUNC_SPIKES: _update_spikes,
  BATTLE_ANIM_FUNC_HEAL_BELL_NOTES: _impl._update_heal_bell_notes,
  BATTLE_ANIM_FUNC_BATON_PASS: _impl._update_baton_pass,
  BATTLE_ANIM_FUNC_CONVERSION: _impl._update_conversion,
  BATTLE_ANIM_FUNC_ENCORE_BELLY_DRUM: _impl._update_encore_belly_drum,
  BATTLE_ANIM_FUNC_SWAGGER_MORNING_SUN: _impl._update_swagger_morning_sun,
  BATTLE_ANIM_FUNC_HIDDEN_POWER: _impl._update_hidden_power,
  BATTLE_ANIM_FUNC_CURSE: _impl._update_curse,
  BATTLE_ANIM_FUNC_PERISH_SONG: _impl._update_perish_song,
  BATTLE_ANIM_FUNC_RAPID_SPIN: _update_rapid_spin,
  BATTLE_ANIM_FUNC_BETA_PURSUIT: _impl._update_beta_pursuit,
  BATTLE_ANIM_FUNC_RAIN_SANDSTORM: _impl._update_rain_sandstorm,
  BATTLE_ANIM_FUNC_BATTLE_ANIM_OBJ_B0: _impl._update_anim_obj_b0,
  BATTLE_ANIM_FUNC_PSYCH_UP: _impl._update_psych_up,
  BATTLE_ANIM_FUNC_ANCIENT_POWER: _update_ancient_power,
  BATTLE_ANIM_FUNC_ROCK_SMASH: _impl._update_rock_smash,
  BATTLE_ANIM_FUNC_COTTON: _update_cotton,
};

export const register_animation_handler = (name: string, handler: AnimationHandler): void => {
  ANIMATION_HANDLERS[name] = handler;
};

export const update_animation_sprite = (
  sprite: AnimationSprite,
  runtime?: BattleAnimationRuntime
): string | null => {
  if (!sprite.function_id) {
    return null;
  }
  const handler = ANIMATION_HANDLERS[sprite.function_id];
  if (!handler) {
    throw new Error(`Missing animation handler for ${sprite.function_id}`);
  }
  return handler(sprite, runtime);
};

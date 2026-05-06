import { AnimationSprite } from "./_battle-animation-state";
import {
  sine,
  cosine,
  apply_offsets,
  to_signed_byte,
  step_thrown_to_target,
  step_circle,
} from "./_battle-anim-math";

// ASM mapping: engine/battle/anim_*.asm BattleAnimFunc_* implementations.

const _DIZZY_FRAMESETS: Record<string, string> = {
  BATTLE_ANIM_FRAMESET_CHICK_1: "BATTLE_ANIM_FRAMESET_CHICK_2",
  BATTLE_ANIM_FRAMESET_CHICK_2: "BATTLE_ANIM_FRAMESET_CHICK_1",
  BATTLE_ANIM_FRAMESET_IMP: "BATTLE_ANIM_FRAMESET_IMP_FLIPPED",
  BATTLE_ANIM_FRAMESET_IMP_FLIPPED: "BATTLE_ANIM_FRAMESET_IMP",
};

const _AMNESIA_FRAMESETS: Record<string, string> = {
  BATTLE_ANIM_FRAMESET_AMNESIA_1: "BATTLE_ANIM_FRAMESET_AMNESIA_2",
  BATTLE_ANIM_FRAMESET_AMNESIA_2: "BATTLE_ANIM_FRAMESET_AMNESIA_3",
  BATTLE_ANIM_FRAMESET_AMNESIA_3: "BATTLE_ANIM_FRAMESET_AMNESIA_1",
};

const _LOCK_ON_FRAMESETS: Record<string, string> = {
  BATTLE_ANIM_FRAMESET_LOCK_ON_1: "BATTLE_ANIM_FRAMESET_LOCK_ON_2",
  BATTLE_ANIM_FRAMESET_LOCK_ON_2: "BATTLE_ANIM_FRAMESET_LOCK_ON_3",
  BATTLE_ANIM_FRAMESET_LOCK_ON_3: "BATTLE_ANIM_FRAMESET_LOCK_ON_4",
  BATTLE_ANIM_FRAMESET_LOCK_ON_4: "BATTLE_ANIM_FRAMESET_LOCK_ON_1",
};

function _set_stage(sprite: AnimationSprite, stage: number): void {
  sprite.jump_index = stage & 0xff;
  sprite.state = sprite.jump_index;
}

function _inc_stage(sprite: AnimationSprite, delta = 1): void {
  _set_stage(sprite, (sprite.jump_index + delta) & 0xff);
}

function _increment_frameset_suffix(frameset: string, step = 1): string {
  if (!frameset) {
    return frameset;
  }
  const parts = frameset.split("_");
  if (parts.length < 2) {
    return frameset;
  }
  const tail = parts[parts.length - 1];
  if (!/^\d+$/.test(tail)) {
    return frameset;
  }
  const nextValue = Number.parseInt(tail, 10) + step;
  parts[parts.length - 1] = String(nextValue);
  return parts.join("_");
}

function div_floor(value: number, divisor: number): number {
  return Math.floor(value / divisor);
}

export function _update_kick(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    return null;
  }
  if (sprite.state === 1) {
    if (sprite.x >= 0x98) {
      return null;
    }
    sprite.x = (sprite.x + 2) & 0xff;
    sprite.override_frameset = "BATTLE_ANIM_FRAMESET_KICK";
    sprite.param = 0;
    sprite.var1 = 2;
    sprite.y = (sprite.y - 1) & 0xff;
    return null;
  }
  if (sprite.state === 2) {
    sprite.jump_index += 1;
    sprite.var1 = 0x2c;
    sprite.param = 0;
    sprite.var2 = 0x80;
    sprite.state = 3;
  }
  if (sprite.state === 3) {
    if (sprite.x >= 0x98) {
      return null;
    }
    sprite.x = (sprite.x + 2) & 0xff;
    sprite.var1 += 1;
    sprite.y_offset = sine(sprite.var1 & 0xff, 8);
    apply_offsets(sprite);
  }
  return null;
}

export function _update_paralyzed(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0;
    const param = sprite.param & 0xff;
    sprite.param = (param & 0x70) >> 4;
    if (param & 0x80) {
      sprite.x_offset = to_signed_byte((param & 0xf) ^ 0xff) + 1;
      sprite.override_frameset = "BATTLE_ANIM_FRAMESET_PARALYZED_FLIPPED";
    } else {
      sprite.x_offset = param & 0xf;
    }
    apply_offsets(sprite);
    return null;
  }
  if (sprite.var1 === 0) {
    sprite.var1 = sprite.param;
    const currentOffset = sprite.x_offset & 0xff;
    sprite.x_offset = to_signed_byte(currentOffset ^ 0xff) + 1;
    apply_offsets(sprite);
  } else {
    sprite.var1 -= 1;
  }
  return null;
}

export function _update_leech_seed(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var2 = 0x40;
    return null;
  }
  if (sprite.state === 1) {
    if (sprite.var2 < 0x20) {
      sprite.state = 2;
      sprite.var2 = 0x40;
      sprite.override_frameset = "BATTLE_ANIM_FRAMESET_LEECH_SEED_2";
      sprite.jump_index += 1;
      return null;
    }
    step_thrown_to_target(sprite);
    return null;
  }
  if (sprite.state === 2) {
    if (sprite.var2 === 0) {
      sprite.state = 3;
      sprite.override_frameset = "BATTLE_ANIM_FRAMESET_LEECH_SEED_3";
      sprite.jump_index += 1;
      return null;
    }
    sprite.var2 -= 1;
    return null;
  }
  return null;
}

export function _update_horn(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.jump_index = sprite.param & 0xff;
    sprite.var1 = sprite.y;
    return null;
  }
  if (sprite.state === 1) {
    if (sprite.x >= 0x58) {
      sprite.state = 2;
      return null;
    }
    sprite.x = (sprite.x + 2) & 0xff;
    return null;
  }
  if (sprite.state === 2) {
    if (sprite.var2 >= 0x20) {
      return "remove";
    }
    const amplitude = sprite.var2 & 0xff;
    sprite.x_offset = sine(amplitude, 8);

    const yOff = div_floor(sprite.x_offset, 2);
    sprite.base_y = sprite.var1 & 0xff;
    sprite.y_offset = -yOff;

    sprite.var2 += 8;
    apply_offsets(sprite);
    return null;
  }
  return null;
}

export function _update_needle(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.jump_index = (sprite.param >> 4) & 0xf;
    return null;
  }

  if (sprite.jump_index === 2) {
    const sineVal = sine(sprite.var1 & 0xff, 0x10);
    if (sineVal & 0x80) {
      sprite.y_offset = sineVal;
    }
    sprite.var1 = (sprite.var1 - 4) & 0xff;
  }

  if (sprite.x >= 0x84) {
    return "remove";
  }

  const speed = sprite.param & 0xf;
  sprite.x = (sprite.x + speed) & 0xff;

  const ySpeed = div_floor(speed, 2);
  sprite.y = (sprite.y - ySpeed) & 0xff;

  apply_offsets(sprite);
  return null;
}

export function _update_sing(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    const notes = [
      "BATTLE_ANIM_FRAMESET_MUSIC_NOTE_1",
      "BATTLE_ANIM_FRAMESET_MUSIC_NOTE_2",
      "BATTLE_ANIM_FRAMESET_MUSIC_NOTE_3",
    ];
    const idx = sprite.param & 0xff;
    if (idx < notes.length) {
      sprite.override_frameset = notes[idx];
    }
    return null;
  }

  if (sprite.x < 0xb8) {
    sprite.x = (sprite.x + 2) & 0xff;
    sprite.y = (sprite.y - 1) & 0xff;

    sprite.var1 = (sprite.var1 - 1) & 0xff;
    sprite.y_offset = sine(sprite.var1, 8);
    apply_offsets(sprite);
    return null;
  }

  return "remove";
}

export function _update_dizzy(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
  }

  const angle = sprite.param & 0xff;
  const yOff = div_floor(sine(angle, 0x10), 4);
  sprite.y_offset = yOff;
  sprite.x_offset = cosine(angle, 0x10);

  sprite.param = (sprite.param + 2) & 0xff;

  if ((sprite.param & 0x1f) === 0) {
    let current = sprite.override_frameset ?? "";
    if (!current) {
      current = sprite.object_id.includes("IMP")
        ? "BATTLE_ANIM_FRAMESET_IMP"
        : "BATTLE_ANIM_FRAMESET_CHICK_1";
    }

    if (current in _DIZZY_FRAMESETS) {
      sprite.override_frameset = _DIZZY_FRAMESETS[current];
    }
  }

  apply_offsets(sprite);
  return null;
}

export function _update_confuse_ray(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var2 = sprite.param & 0x3f;
    return null;
  }

  sprite.var2 = (sprite.var2 + 1) & 0xff;
  const angle = sprite.var2;
  const amplitude = ((sprite.param & 0xff) >> 4) | ((sprite.param & 0xf) << 4);
  sprite.y_offset = sine(angle, amplitude);
  sprite.x_offset = cosine(angle, amplitude);

  if (sprite.x >= 0x80) {
    return null;
  }

  if ((sprite.var2 & 3) === 0) {
    sprite.y = (sprite.y - 1) & 0xff;
  }

  if ((sprite.var2 & 1) === 0) {
    sprite.x = (sprite.x + 1) & 0xff;
  }

  apply_offsets(sprite);
  return null;
}

export function _update_float_up(sprite: AnimationSprite): string | null {
  sprite.var1 = (sprite.var1 + 1) & 0xff;
  sprite.x_offset = sine(sprite.var1, 4);
  sprite.y = (sprite.y - 1) & 0xff;
  apply_offsets(sprite);
  return null;
}

export function _update_amnesia(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0;
    sprite.var2 = 0;
    return null;
  }

  sprite.var1 = (sprite.var1 + 1) & 0xff;
  if ((sprite.var1 & 0x07) === 0) {
    const current = sprite.override_frameset ?? "BATTLE_ANIM_FRAMESET_AMNESIA_1";
    if (current in _AMNESIA_FRAMESETS) {
      sprite.override_frameset = _AMNESIA_FRAMESETS[current];
    }
  }

  sprite.var2 = (sprite.var2 + 1) & 0xff;
  if (sprite.var2 >= 0x20) {
    return "remove";
  }
  return null;
}

export function _update_clamp_encore(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0;
    return null;
  }

  const amplitude = sprite.param & 0x7f;
  const val = sine(sprite.var1 & 0xff, amplitude);
  sprite.x_offset = val;
  sprite.y_offset = sine((sprite.var1 + 64) & 0xff, div_floor(amplitude, 2));

  sprite.var1 = (sprite.var1 + 4) & 0xff;

  apply_offsets(sprite);
  return null;
}

export function _update_shiny(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    const angle = sprite.param & 0xff;
    sprite.y_offset = sine(angle, 0x10);
    sprite.x_offset = cosine(angle, 0x10);
    sprite.var2 = 0x0f;
    apply_offsets(sprite);
  }
  return null;
}

export function _update_metronome_hand(sprite: AnimationSprite): string | null {
  const angle = sprite.var1 & 0xff;
  sprite.var1 = (sprite.var1 + 2) & 0xff;
  sprite.y_offset = sine(angle, 2);
  sprite.x_offset = cosine(angle, 8);
  apply_offsets(sprite);
  return null;
}

export function _update_metronome_sparkle_sketch(
  sprite: AnimationSprite
): string | null {
  if (sprite.y_offset >= 0x20) {
    return "remove";
  }

  const angle = sprite.param & 0xff;
  sprite.x_offset = cosine(angle, 8);
  sprite.param = (sprite.param + 2) & 0xff;

  if ((sprite.param & 7) === 0) {
    sprite.y_offset += 1;
  }

  apply_offsets(sprite);
  return null;
}

export function _update_agility(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.x = (sprite.x + (sprite.param & 0xff)) & 0xff;
    sprite.base_x = sprite.x & 0xff;
    return null;
  }
  return "remove";
}

export function _update_sludge(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0x0c;
    return null;
  }
  if (sprite.state === 1) {
    if (sprite.var1 === 0) {
      sprite.state = 2;
      sprite.override_frameset = "BATTLE_ANIM_FRAMESET_SLUDGE_BUBBLE_BURST";
      return null;
    }
    sprite.var1 -= 1;
    return null;
  }
  if (sprite.state === 2) {
    sprite.y_offset -= 1;
    apply_offsets(sprite);
  }
  return null;
}

export function _update_rain_sandstorm(
  sprite: AnimationSprite
): string | null {
  if (sprite.state === 0) {
    const variant = sprite.param & 0xff;
    if (variant !== 0 && variant !== 1 && variant !== 2) {
      throw new Error(`Unknown Rain/Sandstorm variant: ${variant}`);
    }
    sprite.state = variant + 1;
    return null;
  }

  const newY = (sprite.y_offset + 4) & 0xff;
  sprite.y_offset = newY >= 0x70 ? 0 : newY;

  let rawX = sprite.x_offset & 0xff;
  if (sprite.state === 1) {
    rawX = (rawX + 2) & 0xff;
  } else if (sprite.state === 2) {
    rawX = (rawX + 8) & 0xff;
  } else if (sprite.state === 3) {
    rawX = (rawX + 4) & 0xff;
  }
  sprite.x_offset = to_signed_byte(rawX);
  apply_offsets(sprite);
  return null;
}

export function _update_poison_gas(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    if (sprite.x >= 0x84) {
      sprite.state = 1;
      return null;
    }
    sprite.x = (sprite.x + 1) & 0xff;
    sprite.var1 = (sprite.var1 + 1) & 0xff;
    sprite.x_offset = cosine(sprite.var1, 0x18);
    if ((sprite.x & 1) === 0) {
      sprite.y = (sprite.y - 1) & 0xff;
    }
    apply_offsets(sprite);
    return null;
  }

  return _update_spiral_descent(sprite);
}

export function _update_rock_smash(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0x40;
    const frameBase =
      sprite.param & 0x40
        ? "BATTLE_ANIM_FRAMESET_SMALL_ROCK"
        : "BATTLE_ANIM_FRAMESET_BIG_ROCK";
    sprite.override_frameset = frameBase;
    return null;
  }

  const angle = sprite.var1 & 0xff;
  if (angle < 0x30) {
    return "remove";
  }

  const amplitude = sprite.param & 0x3f;

  sprite.var1 = (sprite.var1 - 1) & 0xff;
  sprite.y_offset = sine(angle, amplitude);

  sprite.x_offset = sine((sprite.var1 * 13) & 0xff, 4);
  apply_offsets(sprite);
  return null;
}

export function _update_spiral_descent(sprite: AnimationSprite): string | null {
  const angle = sprite.var1 & 0xff;

  const ySine = div_floor(sine(angle, 0x18), 8);
  sprite.y_offset = ySine + sprite.var2;

  sprite.x_offset = cosine(angle, 0x18);

  sprite.var1 = (sprite.var1 + 1) & 0xff;

  if ((sprite.var1 & 7) === 0) {
    if (sprite.var2 >= 0x28) {
      return "remove";
    }
    sprite.var2 += 1;
  }

  apply_offsets(sprite);
  return null;
}

export function _update_petal_dance(sprite: AnimationSprite): string | null {
  return _update_spiral_descent(sprite);
}

export function _update_thief_payday(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0x28;
    sprite.var2 = (sprite.y - 0x28) & 0xff;
    return null;
  }

  const angle = sprite.var1 & 0xff;
  const amplitude = sprite.var2 & 0xff;
  sprite.y_offset = sine(angle, amplitude);

  if ((sprite.var1 & (sprite.param || 0xff)) === 0) {
    sprite.x = (sprite.x - 1) & 0xff;
  }

  sprite.var1 = (sprite.var1 + 1) & 0xff;
  if ((sprite.var1 & 0x3f) === 0) {
    sprite.var1 = 0x20;
    sprite.var2 = div_floor(sprite.var2, 2);
  }

  apply_offsets(sprite);
  return null;
}

export function _update_bonemerang(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.base_y = sprite.y & 0xff;
    sprite.var2 = sprite.y & 0xff;
    return null;
  }

  const angle = sprite.param & 0xff;
  sprite.y_offset = sine(angle, 0x30);

  const xCos = cosine((angle + 8) & 0xff, 0x30);
  sprite.x_offset = xCos;

  sprite.param = (sprite.param + 1) & 0xff;
  apply_offsets(sprite);
  return null;
}

export function _update_speed_line(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    return null;
  }

  if (sprite.param & 0x80) {
    sprite.x_offset -= 1;
  } else {
    sprite.x_offset += 1;
  }
  apply_offsets(sprite);
  return null;
}

export function _update_growth_swords_dance(
  sprite: AnimationSprite
): string | null {
  const angle = sprite.param & 0xff;

  const ySine = div_floor(sine(angle, 0x18), 8);
  sprite.y_offset = ySine + sprite.var2;

  sprite.x_offset = cosine(angle, 0x18);

  sprite.param = (sprite.param + 1) & 0xff;
  sprite.var2 -= 2;
  apply_offsets(sprite);
  return null;
}

export function _update_smoke_flame_wheel(
  sprite: AnimationSprite
): string | null {
  const angle = sprite.param & 0xff;
  const ySine = div_floor(sine(angle, 0x18), 8);
  sprite.y_offset = ySine + sprite.var2;
  sprite.x_offset = cosine(angle, 0x18);

  sprite.param = (sprite.param + 2) & 0xff;

  if ((sprite.param & 7) === 0) {
    if (to_signed_byte(sprite.var2) === -24) {
      return "remove";
    }
    sprite.var2 = (sprite.var2 - 1) & 0xff;
  }

  apply_offsets(sprite);
  return null;
}

export function _update_present_smokescreen(
  sprite: AnimationSprite
): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0x34;
    sprite.var2 = 0x10;
  }

  if (sprite.x < 0x6c) {
    return "remove";
  }

  sprite.x = (sprite.x + 2) & 0xff;
  sprite.y = (sprite.y - 1) & 0xff;

  let val = sine(sprite.var1 & 0xff, sprite.var2 & 0xff);
  if (val < 0) {
    val = -val;
  } else if (val & 0x80) {
    val = (val ^ 0xff) + 1;
  }
  sprite.y_offset = val;
  sprite.var1 = (sprite.var1 - 4) & 0xff;

  if ((sprite.var1 & 0x1f) === 0) {
    sprite.var2 = div_floor(sprite.var2, 2);
  }

  apply_offsets(sprite);
  return null;
}

export function _update_heal_bell_notes(
  sprite: AnimationSprite
): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    const framesets = [
      "BATTLE_ANIM_FRAMESET_MUSIC_NOTE_1",
      "BATTLE_ANIM_FRAMESET_MUSIC_NOTE_2",
      "BATTLE_ANIM_FRAMESET_MUSIC_NOTE_3",
    ];
    const idx = sprite.param & 0xff;
    if (idx < framesets.length) {
      sprite.override_frameset = framesets[idx];
    }
    return null;
  }

  if (sprite.y_offset >= 0x38) {
    return "remove";
  }

  sprite.y_offset += 1;
  sprite.var1 = (sprite.var1 + 1) & 0xff;
  sprite.x_offset = cosine(sprite.var1, 0x18);

  const baseY = (sprite.base_y ?? sprite.y) & 0xff;
  if ((baseY & 1) === 0) {
    const baseX = (sprite.base_x ?? sprite.x) & 0xff;
    sprite.base_x = (baseX - 1) & 0xff;
  }

  apply_offsets(sprite);
  return null;
}

export function _update_baton_pass(sprite: AnimationSprite): string | null {
  if (sprite.param === 0) {
    return null;
  }

  sprite.var1 = (sprite.var1 + 1) & 0xff;
  let val = sine(sprite.var1, sprite.param & 0xff);

  if (val < 0) {
    val = -val;
  }

  sprite.y_offset = val;

  if ((sprite.var1 & 0x1f) === 0) {
    sprite.param = sprite.param >> 1;
  }

  apply_offsets(sprite);
  return null;
}

export function _update_lock_on_mind_reader(
  sprite: AnimationSprite
): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    sprite.var1 = 0x28;
    return null;
  }

  if (sprite.state === 1) {
    if (sprite.var1 === 0) {
      sprite.state = 2;
      sprite.var1 = 0x10;
      return null;
    }

    sprite.var1 -= 1;

    if ((sprite.var1 & 3) === 0) {
      const current = sprite.override_frameset ?? "BATTLE_ANIM_FRAMESET_LOCK_ON_1";
      if (current in _LOCK_ON_FRAMESETS) {
        sprite.override_frameset = _LOCK_ON_FRAMESETS[current];
      }
    }

    return null;
  }

  if (sprite.state === 2) {
    if (sprite.var1 === 0) {
      return "remove";
    }
    sprite.var1 -= 1;
  }

  return null;
}

export function _update_safeguard_protect(
  sprite: AnimationSprite
): string | null {
  const angle = sprite.param & 0xff;
  sprite.y_offset = sine(angle, 0x18);
  sprite.x_offset = div_floor(cosine(angle, 0x18), 2);
  sprite.param = (sprite.param + 1) & 0xff;
  apply_offsets(sprite);
  return null;
}

export function _update_perish_song(sprite: AnimationSprite): string | null {
  const angle = sprite.param & 0xff;
  sprite.y_offset = div_floor(sine(angle, 0x50), 4) + sprite.var1;
  sprite.x_offset = cosine(angle, 0x50);

  sprite.param = (sprite.param + 2) & 0xff;
  sprite.var1 = (sprite.var1 + 1) & 0xff;
  apply_offsets(sprite);
  return null;
}

export function _update_sky_attack(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    sprite.state = 1;
    return null;
  }
  if (sprite.state === 1) {
    const palette_cycle = [
      "PAL_BATTLE_OB_GRAY",
      "PAL_BATTLE_OB_YELLOW",
      "PAL_BATTLE_OB_RED",
      "PAL_BATTLE_OB_BLUE",
    ];
    sprite.param = (sprite.param + 1) & 0xff;
    const idx = (sprite.param >> 2) % palette_cycle.length;
    sprite.palette_override = palette_cycle[idx];
    return null;
  }
  if (sprite.state === 2) {
    if (sprite.x >= 0x84) {
      return null;
    }
    sprite.x = (sprite.x + 4) & 0xff;
    return null;
  }
  if (sprite.state === 3) {
    if (sprite.x >= 0xd0) {
      return "remove";
    }
    sprite.x = (sprite.x + 4) & 0xff;
    return null;
  }
  return null;
}

export function _update_sacred_fire(sprite: AnimationSprite): string | null {
  const angle = sprite.param & 0xff;
  const ySine = div_floor(sine(angle, 0x18), 8);
  sprite.y_offset = ySine + sprite.var2;
  sprite.x_offset = cosine(angle, 0x18);

  sprite.param = (sprite.param + 2) & 0xff;

  if ((sprite.param & 3) === 0) {
    if (to_signed_byte(sprite.var2) === -48) {
      return "remove";
    }
    sprite.var2 = (sprite.var2 - 2) & 0xff;
  }

  apply_offsets(sprite);
  return null;
}

export function _update_curse(sprite: AnimationSprite): string | null {
  if (sprite.x < 0x30) {
    return "remove";
  }
  sprite.x = (sprite.x - 2) & 0xff;
  sprite.y = (sprite.y + 2) & 0xff;
  apply_offsets(sprite);
  return null;
}

export function _update_beta_pursuit(sprite: AnimationSprite): string | null {
  if (sprite.state === 0) {
    if (sprite.param !== 0) {
      sprite.state = 2;
    } else {
      sprite.state = 1;
      sprite.y_offset = 0xec;
    }
    return null;
  }

  if (sprite.state === 1) {
    if (to_signed_byte(sprite.y_offset) >= 4) {
      return "remove";
    }
    sprite.y_offset = (sprite.y_offset + 4) & 0xff;
    apply_offsets(sprite);
    return null;
  }

  if (sprite.state === 2) {
    if (to_signed_byte(sprite.y_offset) <= -40) {
      return null;
    }
    sprite.y_offset = (sprite.y_offset - 4) & 0xff;
    apply_offsets(sprite);
    return null;
  }

  return null;
}

export function _update_psych_up(sprite: AnimationSprite): string | null {
  sprite.param = (sprite.param + 1) & 0xff;
  const angle = sprite.param;
  sprite.y_offset = div_floor(sine(angle, 0x18), 4);
  sprite.x_offset = cosine(angle, 0x18);
  apply_offsets(sprite);
  return null;
}

export function _update_conversion(sprite: AnimationSprite): string | null {
  const angle = sprite.param & 0xff;
  sprite.param = (sprite.param + 1) & 0xff;
  const radius = sprite.var1 & 0xff;
  sprite.y_offset = sine(angle, radius);
  sprite.x_offset = cosine(angle, radius);
  sprite.var2 = (sprite.var2 + 1) & 0xff;
  if (sprite.var2 < 0x40) {
    sprite.var1 = (radius + 1) & 0xff;
  } else {
    sprite.var1 = (radius - 1) & 0xff;
    if (sprite.var1 === 0) {
      return "remove";
    }
  }
  apply_offsets(sprite);
  return null;
}

export function _update_encore_belly_drum(
  sprite: AnimationSprite
): string | null {
  const progress = sprite.var1 & 0xff;
  if (progress >= 0x10) {
    return "remove";
  }
  sprite.var1 = (progress + 2) & 0xff;
  const angle = sprite.param & 0xff;
  sprite.y_offset = sine(angle, progress);
  sprite.x_offset = cosine(angle, progress);
  apply_offsets(sprite);
  return null;
}

export function _update_hidden_power(sprite: AnimationSprite): string | null {
  if (sprite.jump_index === 0) {
    const [xOff, yOff] = step_circle(sprite.param & 0xff, 0x18);
    sprite.param = (sprite.param + 1) & 0xff;
    sprite.x_offset = xOff;
    sprite.y_offset = yOff;
    apply_offsets(sprite);
    return null;
  }

  if (sprite.jump_index === 1) {
    sprite.var1 = 0x18;
    _inc_stage(sprite);
    return null;
  }

  const radius = sprite.var1 & 0xff;
  if (radius >= 0x80) {
    return "remove";
  }
  const angle = sprite.param & 0xff;
  const [xOff, yOff] = step_circle(angle, radius);
  sprite.x_offset = xOff;
  sprite.y_offset = yOff;
  sprite.var1 = (radius + 8) & 0xff;
  apply_offsets(sprite);
  return null;
}

export function _update_swagger_morning_sun(
  sprite: AnimationSprite
): string | null {
  const angle = sprite.param & 0x3f;
  const speed = (sprite.param >> 6) & 0x03;
  sprite.var1 = (sprite.var1 + speed) & 0xff;
  const amplitude = sprite.var1 & 0xff;
  sprite.y_offset = sine(angle, amplitude);
  sprite.x_offset = cosine(angle, amplitude);
  apply_offsets(sprite);
  return null;
}

export function _update_anim_obj_b0(sprite: AnimationSprite): string | null {
  let accumulator = ((sprite.x & 0xff) << 8) | (sprite.var1 & 0xff);
  const highNibble = (sprite.param >> 4) & 0xf;
  const lowNibble = sprite.param & 0xf;
  const delta = ((highNibble * 0x11) << 8) | (lowNibble << 4);
  accumulator = (accumulator + delta) & 0xffff;
  sprite.x = (accumulator >> 8) & 0xff;
  sprite.var1 = accumulator & 0xff;
  sprite.base_x = sprite.x & 0xff;
  sprite.base_y = (sprite.base_y ?? sprite.y) & 0xff;
  apply_offsets(sprite);
  return null;
}

export function _update_string(sprite: AnimationSprite): string | null {
  if (sprite.jump_index === 0) {
    _inc_stage(sprite);
    if (sprite.param === 0) {
      sprite.mirror_y = true;
    }
    const index = (sprite.param & 0xff) + 1;
    sprite.override_frameset = `BATTLE_ANIM_FRAMESET_STRING_SHOT_${index}`;
  }
  return null;
}

export function _update_wrap(sprite: AnimationSprite): string | null {
  if (sprite.jump_index !== 1) {
    return null;
  }
  const current = sprite.override_frameset ?? "";
  const nextFrameset = _increment_frameset_suffix(
    current || "BATTLE_ANIM_FRAMESET_BIND_1"
  );
  sprite.override_frameset = nextFrameset;
  sprite.var1 = 0x08;
  _inc_stage(sprite);
  return null;
}

export function _update_strength_seismic_toss(
  sprite: AnimationSprite
): string | null {
  if (sprite.jump_index === 0) {
    if ((sprite.y_offset & 0xff) === 0xe0) {
      sprite.var1 = 0x02;
      _inc_stage(sprite);
      return null;
    }
    let accumulator =
      (((sprite.y_offset & 0xff) << 8) | (sprite.var1 & 0xff)) - 0x80;
    accumulator &= 0xffff;
    sprite.y_offset = (accumulator >> 8) & 0xff;
    sprite.var1 = accumulator & 0xff;
    apply_offsets(sprite);
    return null;
  }

  if (sprite.jump_index === 1) {
    if (sprite.var2) {
      sprite.var2 = (sprite.var2 - 1) & 0xff;
      return null;
    }
    sprite.var2 = 0x04;
    sprite.var1 = ((sprite.var1 ^ 0xff) + 1) & 0xff;
    sprite.y_offset = (sprite.y_offset + sprite.var1) & 0xff;
    apply_offsets(sprite);
    return null;
  }

  const speed = 4;
  const yStep = Math.max(1, div_floor(speed, 2));
  const baseY = (sprite.base_y ?? sprite.y) & 0xff;
  sprite.base_y = (baseY - yStep) & 0xff;
  const baseX = (sprite.base_x ?? sprite.x) & 0xff;
  sprite.base_x = (baseX + speed) & 0xff;
  apply_offsets(sprite);
  const currentX = sprite.base_x ?? sprite.x;
  if (currentX >= 0x84) {
    return "remove";
  }
  return null;
}

export function _update_battle_anim_null(
  _sprite: AnimationSprite
): string | null {
  return null;
}

export function _update_throw_to_target(sprite: AnimationSprite): string | null {
  // ASM: BattleAnimFunc_ThrowFromUserToTarget (engine/battle_anims/functions.asm).
  const targetX = 0x88;
  const currentX = sprite.x & 0xff;
  if (currentX >= targetX) {
    return null;
  }
  sprite.x = (currentX + 2) & 0xff;
  sprite.y = (sprite.y - 1) & 0xff;
  const angle = sprite.var1 & 0xff;
  sprite.var1 = (angle - 1) & 0xff;
  sprite.y_offset = sine(angle, sprite.param & 0xff);
  return null;
}

function _egg_vertical_wave_motion(sprite: AnimationSprite): void {
  const angle = sprite.var1 & 0xff;
  const amplitude = sprite.var2 & 0xff;
  sprite.y_offset = sine(angle, amplitude);
  sprite.var1 = (sprite.var1 + 1) & 0xff;
  if ((sprite.var1 & 0x3f) !== 0) {
    return;
  }
  sprite.var1 = 0x20;
  sprite.var2 = (sprite.var2 - 0x08) & 0xff;
  if (sprite.var2 !== 0) {
    return;
  }
  sprite.var1 = 0;
  sprite.var2 = 0;
  _inc_stage(sprite);
}

export function _update_egg(sprite: AnimationSprite): string | null {
  if (sprite.jump_index === 0) {
    sprite.var1 = 0x28;
    sprite.var2 = 0x10;
    _set_stage(sprite, sprite.param & 0xff);
    return null;
  }

  if (sprite.jump_index === 1) {
    if (sprite.x < 0x40) {
      sprite.x = (sprite.x + 1) & 0xff;
    }
    _egg_vertical_wave_motion(sprite);
    sprite.base_x = sprite.x & 0xff;
    sprite.base_y = sprite.y & 0xff;
    apply_offsets(sprite);
    return null;
  }

  if (sprite.jump_index === 6) {
    if (sprite.x < 0x4b) {
      sprite.x = (sprite.x + 1) & 0xff;
    }
    _egg_vertical_wave_motion(sprite);
    sprite.base_x = sprite.x & 0xff;
    sprite.base_y = sprite.y & 0xff;
    apply_offsets(sprite);
    return null;
  }

  if (sprite.jump_index === 2) {
    if (sprite.x >= 0x88) {
      _inc_stage(sprite, 2);
      return null;
    }
    if ((sprite.x & 0xf) === 0) {
      sprite.var2 = 0x10;
      _inc_stage(sprite);
      return null;
    }
  }

  if (sprite.jump_index === 3) {
    if (sprite.var2) {
      sprite.var2 = (sprite.var2 - 1) & 0xff;
      return null;
    }
    _set_stage(sprite, 2);
  }
  if (sprite.jump_index === 2) {
    sprite.x = (sprite.x + 1) & 0xff;
    let accumulator = (((sprite.y & 0xff) << 8) | (sprite.var1 & 0xff)) - 0x80;
    accumulator &= 0xffff;
    sprite.y = (accumulator >> 8) & 0xff;
    sprite.var1 = accumulator & 0xff;
    sprite.base_x = sprite.x & 0xff;
    sprite.base_y = sprite.y & 0xff;
    apply_offsets(sprite);
    return null;
  }

  if (sprite.jump_index === 5) {
    return "remove";
  }

  if (sprite.jump_index === 7) {
    sprite.override_frameset = "BATTLE_ANIM_FRAMESET_EGG_WOBBLE";
    _inc_stage(sprite);
    return null;
  }

  if (sprite.jump_index === 8) {
    sprite.var1 = (sprite.var1 + 2) & 0xff;
    sprite.x_offset = sine(sprite.var1, 2);
    apply_offsets(sprite);
    return null;
  }

  if (sprite.jump_index === 9) {
    sprite.override_frameset = "BATTLE_ANIM_FRAMESET_EGG_CRACKED_BOTTOM";
    sprite.y_offset = 4;
    _inc_stage(sprite);
    apply_offsets(sprite);
    return null;
  }

  if (sprite.jump_index === 11) {
    sprite.override_frameset = "BATTLE_ANIM_FRAMESET_EGG_CRACKED_TOP";
    _inc_stage(sprite);
    sprite.var1 = 0x40;
    return null;
  }

  if (sprite.jump_index === 12) {
    sprite.y_offset = sine(sprite.var1 & 0xff, 0x20);
    if (sprite.var1 >= 0x30) {
      _inc_stage(sprite);
      return null;
    }
    sprite.var1 = (sprite.var1 - 1) & 0xff;
    apply_offsets(sprite);
    return null;
  }

  if (sprite.jump_index === 4 || sprite.jump_index === 10 || sprite.jump_index === 13) {
    return null;
  }

  return null;
}

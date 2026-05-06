import { AnimationSpriteSchema } from './battle-animation-state';
import { cosine, sine } from './battle-anim-math';
import { compute_pokeball_anim_var, update_animation_sprite } from './battle-anim-runtime';
import { BattleAnimationRuntime } from './battle-bg-effects';

describe('battle-anim-runtime', () => {
  it('dispatches registered animation handlers', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'TEST_OBJ',
      x: 0,
      y: 0,
      param: 2,
      function_id: 'BATTLE_ANIM_FUNC_MOVE_UP',
    });

    update_animation_sprite(sprite);
    expect(sprite.y_offset).toBe((0 - 2) & 0xff);
  });

  it('sets surf line overrides on startup', () => {
    const runtime = new BattleAnimationRuntime();
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'SURF',
      x: 88,
      y: 104,
      param: 0x08,
      function_id: 'BATTLE_ANIM_FUNC_SURF',
    });

    update_animation_sprite(sprite, runtime);

    expect(sprite.state).toBe(1);
    expect(runtime.lcd_pointer).toBe('scy');
    expect(runtime.ly_override_start).toBe(0x58);
    expect(runtime.ly_override_end).toBe(0x5e);
  });

  it('steps bubble animation with ASM timing', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'BUBBLE',
      x: 64,
      y: 92,
      param: 0xc1,
      function_id: 'BATTLE_ANIM_FUNC_BUBBLE',
    });

    for (let i = 0; i < 12; i += 1) {
      update_animation_sprite(sprite);
    }

    expect(sprite.state).toBe(1);
    expect(sprite.x).toBe(76);
    expect(sprite.y).toBe(92);

    update_animation_sprite(sprite);

    expect(sprite.state).toBe(2);
    expect(sprite.override_frameset).toBe('BATTLE_ANIM_FRAMESET_PULSING_BUBBLE');
    expect(sprite.x).toBe(76);
    expect(sprite.y).toBe(91);
    expect(sprite.var1).toBe(0x60);
    expect(sprite.var2).toBe(0xc0);
  });

  it('matches the ASM sound wave angle rules', () => {
    const angle = 0x08;
    const xOffset = sine(angle, 0x10);

    const negative = AnimationSpriteSchema.parse({
      object_id: 'SOUND',
      x: 64,
      y: 88,
      param: 0,
      var2: angle,
      var1: 2,
      state: 1,
      function_id: 'BATTLE_ANIM_FUNC_SOUND',
    });
    update_animation_sprite(negative);
    expect(negative.x_offset).toBe(xOffset);
    expect(negative.y_offset).toBe(-xOffset);

    const flat = AnimationSpriteSchema.parse({
      object_id: 'SOUND',
      x: 64,
      y: 88,
      param: 1,
      var2: angle,
      var1: 2,
      state: 1,
      y_offset: 7,
      function_id: 'BATTLE_ANIM_FUNC_SOUND',
    });
    update_animation_sprite(flat);
    expect(flat.x_offset).toBe(xOffset);
    expect(flat.y_offset).toBe(7);

    const positive = AnimationSpriteSchema.parse({
      object_id: 'SOUND',
      x: 64,
      y: 88,
      param: 2,
      var2: angle,
      var1: 2,
      state: 1,
      function_id: 'BATTLE_ANIM_FUNC_SOUND',
    });
    update_animation_sprite(positive);
    expect(positive.x_offset).toBe(xOffset);
    expect(positive.y_offset).toBe(xOffset);
  });

  it('keeps the last sound wave frame before removal', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'SOUND',
      x: 64,
      y: 88,
      param: 0,
      var1: 1,
      var2: 0,
      state: 1,
      function_id: 'BATTLE_ANIM_FUNC_SOUND',
    });

    const first = update_animation_sprite(sprite);
    expect(first).toBeNull();
    expect(sprite.var1).toBe(0);

    const second = update_animation_sprite(sprite);
    expect(second).toBe('remove');
  });

  it('returns the caught state on the fourth pokeball check', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'POKE_BALL',
      x: 0,
      y: 0,
      param: 0,
    });

    const results = [];
    for (let i = 0; i < 4; i += 1) {
      results.push(compute_pokeball_anim_var(sprite, 4));
    }

    expect(results).toEqual([0, 0, 0, 1]);
  });

  it('returns the escaped state after the target wobble count', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'POKE_BALL',
      x: 0,
      y: 0,
      param: 0,
    });

    const results = [];
    for (let i = 0; i < 3; i += 1) {
      results.push(compute_pokeball_anim_var(sprite, 2));
    }

    expect(results).toEqual([0, 0, 2]);
  });

  it('uses the pre-decrement angle for throw-to-target offsets', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'ACID',
      x: 0x40,
      y: 0x20,
      param: 0x20,
      var1: 0x10,
      function_id: 'BATTLE_ANIM_FUNC_THROW_TO_TARGET',
    });

    update_animation_sprite(sprite);

    expect(sprite.x).toBe(0x42);
    expect(sprite.y).toBe(0x1f);
    expect(sprite.var1).toBe(0x0f);
    expect(sprite.y_offset).toBe(sine(0x10, 0x20));
  });

  it('wraps pokeball throw coordinates with 8-bit math', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'POKE_BALL',
      x: 0x40,
      y: 0x00,
      param: 0x10,
      var1: 0x08,
      state: 1,
      function_id: 'BATTLE_ANIM_FUNC_POKEBALL',
    });

    update_animation_sprite(sprite);

    expect(sprite.x).toBe(0x42);
    expect(sprite.y).toBe(0xff);
    expect(sprite.y_offset).toBe(sine(0x08, 0x10));
    expect(sprite.jump_index).toBe(1);
    expect(sprite.state).toBe(1);
  });

  it('advances razor leaf through the ASM jump-table phases instead of spinning forever', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'BATTLE_ANIM_OBJ_RAZOR_LEAF',
      x: 48,
      y: 80,
      param: 0x28,
      function_id: 'BATTLE_ANIM_FUNC_RAZOR_LEAF',
      override_frameset: 'BATTLE_ANIM_FRAMESET_RAZOR_LEAF_1',
      mirror_x: true,
    });

    for (let i = 0; i < 18; i += 1) {
      update_animation_sprite(sprite);
    }

    expect(sprite.jump_index).toBe(2);
    expect(sprite.override_frameset).toBe('BATTLE_ANIM_FRAMESET_RAZOR_LEAF_2');
    expect(sprite.x).toBeGreaterThan(48);
    expect(sprite.y_offset).not.toBe(0);

    sprite.jump_index = 3;
    update_animation_sprite(sprite);

    expect(sprite.override_frameset).toBe('BATTLE_ANIM_FRAMESET_RAZOR_LEAF_1');
    expect(sprite.mirror_x).toBe(false);
    expect(sprite.jump_index).toBe(4);

    update_animation_sprite(sprite);
    update_animation_sprite(sprite);
    update_animation_sprite(sprite);
    update_animation_sprite(sprite);

    expect(sprite.jump_index).toBe(8);
    const startX = sprite.x;
    const startY = sprite.y;

    update_animation_sprite(sprite);

    expect(sprite.x).toBe((startX + 8) & 0xff);
    expect(sprite.y).toBe((startY - 4) & 0xff);
  });

  it('increments the pokeball wobble stage at the 0x10 angle boundary', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'POKE_BALL',
      x: 0x50,
      y: 0x60,
      param: 0,
      var1: 0x11,
      var2: 0x20,
      state: 8,
      function_id: 'BATTLE_ANIM_FUNC_POKEBALL',
    });

    update_animation_sprite(sprite);

    expect(sprite.state).toBe(9);
    expect(sprite.jump_index).toBe(9);
  });

  it('bootstraps pokeball phase progression from legacy state when jump_index is unset', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'POKE_BALL',
      x: 0x50,
      y: 0x60,
      param: 0,
      var1: 0x11,
      var2: 0x20,
      state: 8,
      jump_index: 0,
      function_id: 'BATTLE_ANIM_FUNC_POKEBALL',
    });

    update_animation_sprite(sprite);

    expect(sprite.state).toBe(9);
    expect(sprite.jump_index).toBe(9);
  });

  it('uses the pokeball jump index for anim_setobj-driven shake startup', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'POKE_BALL',
      x: 0x88,
      y: 0x41,
      param: 0,
      state: 5,
      jump_index: 7,
      function_id: 'BATTLE_ANIM_FUNC_POKEBALL',
      param_label: 'POKE_BALL',
    });

    update_animation_sprite(sprite);

    expect(sprite.palette_override).toBe('PAL_BATTLE_OB_RED');
    expect(sprite.override_frameset).toBe('BATTLE_ANIM_FRAMESET_POKE_BALL_2');
    expect(sprite.var2).toBe(0x20);
    expect(sprite.state).toBe(8);
    expect(sprite.jump_index).toBe(8);
  });

  it('uses the pokeball jump index for anim_incobj-driven wobble progression', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'POKE_BALL',
      x: 0x88,
      y: 0x41,
      param: 0,
      var1: 0x11,
      var2: 0x20,
      state: 9,
      jump_index: 10,
      function_id: 'BATTLE_ANIM_FUNC_POKEBALL',
    });

    update_animation_sprite(sprite);

    expect(sprite.state).toBe(11);
    expect(sprite.jump_index).toBe(11);
  });

  it('wraps pokeball landing offsets with 8-bit math', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'POKE_BALL',
      x: 0x88,
      y: 0x00,
      param: 0,
      var1: 0x00,
      y_offset: -1,
      state: 1,
      function_id: 'BATTLE_ANIM_FUNC_POKEBALL',
    });

    update_animation_sprite(sprite);

    expect(sprite.y).toBe(0xff);
    expect(sprite.jump_index).toBe(2);
  });

  it('normalizes ball labels with spaces for palette overrides', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'POKE_BALL',
      x: 0,
      y: 0,
      param: 0,
      function_id: 'BATTLE_ANIM_FUNC_POKEBALL',
      param_label: 'GREAT BALL',
    });

    update_animation_sprite(sprite);

    expect(sprite.palette_override).toBe('PAL_BATTLE_OB_BLUE');
  });

  it('uses fire blast param as the ASM jump-table entry instead of a custom drift path', () => {
    const upward = AnimationSpriteSchema.parse({
      object_id: 'FIRE_BLAST',
      x: 64,
      y: 80,
      param: 1,
      function_id: 'BATTLE_ANIM_FUNC_FIRE_BLAST',
      y_offset: 0,
    });

    update_animation_sprite(upward);
    expect(upward.state).toBe(1);
    expect(upward.override_frameset).toBe('BATTLE_ANIM_FRAMESET_BURNED');

    update_animation_sprite(upward);
    expect(upward.y_offset).toBe(0xff);
    expect(upward.x_offset).toBe(0);

    const rightward = AnimationSpriteSchema.parse({
      object_id: 'FIRE_BLAST',
      x: 64,
      y: 80,
      param: 3,
      function_id: 'BATTLE_ANIM_FUNC_FIRE_BLAST',
      x_offset: 0,
      y_offset: 0,
    });

    update_animation_sprite(rightward);
    update_animation_sprite(rightward);
    expect(rightward.x_offset).toBe(1);
    expect(rightward.y_offset).toBe(0);
  });

  it('keeps gust wobbling in place before the later movement stages', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'GUST',
      x: 64,
      y: 80,
      param: 0,
      function_id: 'BATTLE_ANIM_FUNC_GUST',
    });

    update_animation_sprite(sprite);
    expect(sprite.state).toBe(1);
    const startBaseX = sprite.base_x ?? sprite.x;
    const startBaseY = sprite.base_y ?? sprite.y;

    update_animation_sprite(sprite);

    expect(sprite.base_x ?? sprite.x).toBe(startBaseX);
    expect(sprite.base_y ?? sprite.y).toBe(startBaseY);
  });

  it('runs solar beam as a shrinking circle instead of a straight horizontal shot', () => {
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'SOLAR_BEAM',
      x: 72,
      y: 88,
      param: 0x20,
      function_id: 'BATTLE_ANIM_FUNC_SOLAR_BEAM',
    });

    update_animation_sprite(sprite);

    expect(sprite.state).toBe(1);
    expect(sprite.var1).toBe(0x28);
    expect(sprite.var2).toBe(0);

    update_animation_sprite(sprite);

    expect(sprite.x).toBe(72);
    expect(sprite.var1).toBe(0x27);
    expect(sprite.var2).toBe(0x80);
    expect(sprite.x_offset).toBe(cosine(0x20, 0x28));
  });
});

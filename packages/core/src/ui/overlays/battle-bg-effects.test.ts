import { AnimationPlayer } from './battle-animation';
import { load_animation_scripts, register_animation_scripts } from './battle-animation-loader';
import { BattleAnimationRuntime, buildParsedBgEffect, make_effect } from './battle-bg-effects';

describe('battle bg effects', () => {
  it('blinks the target on the wait cadence without sliding offsets', () => {
    const baseline = load_animation_scripts();
    register_animation_scripts([
      ...baseline.values(),
      {
        name: 'BattleAnim_EnemyDamage',
        script: [
          {
            command: 'anim_bgeffect',
            args: ['BATTLE_BG_EFFECT_HIDE_MON', '$0', 'BG_EFFECT_TARGET', '$0'],
          },
          { command: 'anim_wait', args: ['5'] },
          {
            command: 'anim_bgeffect',
            args: ['BATTLE_BG_EFFECT_SHOW_MON', '$0', 'BG_EFFECT_TARGET', '$0'],
          },
          { command: 'anim_wait', args: ['5'] },
          {
            command: 'anim_bgeffect',
            args: ['BATTLE_BG_EFFECT_HIDE_MON', '$0', 'BG_EFFECT_TARGET', '$0'],
          },
          { command: 'anim_wait', args: ['5'] },
          {
            command: 'anim_bgeffect',
            args: ['BATTLE_BG_EFFECT_SHOW_MON', '$0', 'BG_EFFECT_TARGET', '$0'],
          },
          { command: 'anim_wait', args: ['5'] },
          { command: 'anim_ret', args: [] },
        ],
        labels: { BattleAnim_EnemyDamage: 0 },
      },
    ]);

    const player = new AnimationPlayer({ tile_size: 8 });
    player.play_animation('enemy_damage', true);
    const runtime = player.runtime_state;
    const frames: Array<{ visible: boolean; offset: number }> = [];

    for (let i = 0; i < 24; i += 1) {
      player.update();
      frames.push({ visible: runtime.enemy_visible, offset: runtime.enemy_offset_x });
    }

    expect(frames.every((frame) => frame.offset === 0)).toBe(true);
    expect(frames.slice(0, 6).every((frame) => frame.visible === false)).toBe(true);
    expect(frames.slice(6, 12).every((frame) => frame.visible === true)).toBe(true);
    expect(frames.slice(12, 18).every((frame) => frame.visible === false)).toBe(true);
    expect(frames.slice(18, 24).every((frame) => frame.visible === true)).toBe(true);
  });

  it('builds surf line scroll offsets when LCD overrides are active', () => {
    const effect = make_effect(buildParsedBgEffect({
      name: 'BATTLE_BG_EFFECT_SURF',
      duration: 0,
      raw_turn: 'BG_EFFECT_TARGET',
      param: 0,
      is_player_move: true,
      turn_value: null,
    }));
    const runtime = new BattleAnimationRuntime();
    runtime.lcd_pointer = 'scy';
    runtime.ly_override_start = 0x58;
    runtime.ly_override_end = 0x5e;

    effect.update(runtime);
    const active = effect.update(runtime);

    expect(active).toBe(true);
    expect(runtime.line_scroll_y[0]).toBe(0);
    expect(runtime.line_scroll_y.slice(0x59, 0x5f).some((value) => value !== 0)).toBe(true);
  });

  it('throws on unsupported bg effect names instead of degrading silently', () => {
    expect(() =>
      make_effect(
        buildParsedBgEffect({
          name: 'BATTLE_BG_EFFECT_DOES_NOT_EXIST',
          duration: 0,
          raw_turn: 'BG_EFFECT_USER',
          param: 0,
          is_player_move: true,
          turn_value: null,
        }),
      ),
    ).toThrow('Unsupported battle bg effect: BATTLE_BG_EFFECT_DOES_NOT_EXIST');
  });
});

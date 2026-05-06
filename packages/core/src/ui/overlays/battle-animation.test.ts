import { MoveName } from '@pokecrystal/core/core/enums';
import logger from '@pokecrystal/core/core/logger';
import { AnimationPlayer } from './battle-animation';
import { AnimationContext, AnimationObjectTable, AnimationSpriteSchema } from './battle-animation-state';
import { buildParsedBgEffect } from './battle-bg-effects';

describe('battle-animation', () => {
  it('throws on unknown animation opcodes', () => {
    const player = new AnimationPlayer({});
    player.animations = new Map([
      [
        'BattleAnim_UnknownOpcode',
        {
          name: 'BattleAnim_UnknownOpcode',
          script: [
            { command: 'anim_not_real', args: [] },
            { command: 'anim_ret', args: [] },
          ],
          labels: { BattleAnim_UnknownOpcode: 0 },
        },
      ],
    ]);

    player.play_animation('UNKNOWN_OPCODE', true);
    expect(() => player.update()).toThrow(
      "Unknown battle animation opcode 'anim_not_real' in BattleAnim_UnknownOpcode.",
    );
  });

  it('accepts direct battle animation labels without remapping them through move names', () => {
    const player = new AnimationPlayer({});
    player.animations = new Map([
      [
        'BattleAnim_TestDirect',
        {
          name: 'BattleAnim_TestDirect',
          script: [{ command: 'anim_ret', args: [] }],
          labels: { BattleAnim_TestDirect: 0 },
        },
      ],
    ]);

    expect(() => player.play_animation('BattleAnim_TestDirect', true)).not.toThrow();
    expect(() => player.update()).not.toThrow();
  });

  it('ignores object flip flags on player-side animations', () => {
    const player = new AnimationPlayer({});
    (player as unknown as { handle_obj: (command: { command: string; args: string[] }) => void }).handle_obj({
      command: 'anim_obj',
      args: ['BATTLE_ANIM_OBJ_SOUND', '64', '76', '$0'],
    });

    const sprite = player.active_sprites[0];
    expect(sprite.mirror_x).toBe(false);
  });

  it('skips anim_obj overflow without throwing when no battle anim object slot is free', () => {
    const player = new AnimationPlayer({});
    const objects = (player as unknown as { objects: AnimationObjectTable }).objects;

    for (let i = 0; i < 10; i += 1) {
      const added = objects.add(
        AnimationSpriteSchema.parse({
          object_id: `BATTLE_ANIM_OBJ_FILL_${i}`,
          x: i,
          y: i,
          param: 0,
        }),
      );
      expect(added).not.toBeNull();
    }

    expect(() =>
      (player as unknown as { handle_obj: (command: { command: string; args: string[] }) => void }).handle_obj({
        command: 'anim_obj',
        args: ['BATTLE_ANIM_OBJ_EXTRA', '1', '2', '$0'],
      }),
    ).not.toThrow();

    expect(player.active_sprites).toHaveLength(10);
    expect(player.active_sprites.some((sprite) => sprite.object_id === 'BATTLE_ANIM_OBJ_EXTRA')).toBe(false);
  });

  it('throws on malformed anim_bgeffect arguments instead of warning and continuing', () => {
    const warn = jest.spyOn(console, 'warn').mockImplementation(() => {});
    const player = new AnimationPlayer({});

    expect(() =>
      (player as unknown as {
        handle_bgeffect: (command: { command: string; args: string[] }) => void;
      }).handle_bgeffect({
        command: 'anim_bgeffect',
        args: ['BATTLE_BG_EFFECT_FLASH_INVERTED', '$0', '$8'],
      }),
    ).toThrow('anim_bgeffect requires four arguments');

    expect(warn).not.toHaveBeenCalled();
  });

  it('does not toggle battler row modes for anim_battlergfx commands', () => {
    const player = new AnimationPlayer({});
    const runtime = player.runtime_state;

    expect(runtime.player_row_mode).toBe(0);
    expect(runtime.enemy_row_mode).toBe(0);

    (player as unknown as { handle_battlergfx_2row: (command: { command: string; args: string[] }) => void })
      .handle_battlergfx_2row({ command: 'anim_battlergfx_2row', args: [] });

    expect(runtime.player_row_mode).toBe(0);
    expect(runtime.enemy_row_mode).toBe(0);

    (player as unknown as { handle_battlergfx_1row: (command: { command: string; args: string[] }) => void })
      .handle_battlergfx_1row({ command: 'anim_battlergfx_1row', args: [] });

    expect(runtime.player_row_mode).toBe(0);
    expect(runtime.enemy_row_mode).toBe(0);
  });

  it('preserves enemy-side context when an animation calls a subroutine', () => {
    const player = new AnimationPlayer({});
    const animation = {
      name: 'BattleAnim_TestSubroutine',
      script: [
        { command: 'anim_call', args: ['.spawn'] },
        { command: 'anim_ret', args: [] },
        { command: 'anim_obj', args: ['BATTLE_ANIM_OBJ_HIT', '64', '76', '$0'] },
        { command: 'anim_ret', args: [] },
      ],
      labels: {
        BattleAnim_TestSubroutine: 0,
        '.spawn': 2,
      },
    };
    (player as unknown as { current_context: AnimationContext | null }).current_context = new AnimationContext({
      animation: animation as any,
      pointer: 0,
      is_player_move: false,
    });

    (player as unknown as { handle_call: (command: { command: string; args: string[] }) => void }).handle_call({
      command: 'anim_call',
      args: ['.spawn'],
    });
    (player as unknown as { handle_obj: (command: { command: string; args: string[] }) => void }).handle_obj({
      command: 'anim_obj',
      args: ['BATTLE_ANIM_OBJ_HIT', '64', '76', '$0'],
    });

    expect(player.active_sprites.length).toBeGreaterThan(0);
    expect(player.active_sprites[0].is_player_move).toBe(false);
  });

  it('preserves cry selector metadata from anim_cry commands', () => {
    const player = new AnimationPlayer({});

    (player as unknown as {
      handle_cry: (command: { command: string; args: string[] }) => void;
    }).handle_cry({
      command: 'anim_cry',
      args: ['1'],
    });

    expect(player.pending_audio()).toEqual([
      expect.objectContaining({
        sound_id: 'cry',
        sound_type: 'cry',
        cry_selector: 1,
      }),
    ]);
  });

  it('mirrors enemy base X before applying X offsets', () => {
    const player = new AnimationPlayer({});
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'BATTLE_ANIM_OBJ_HORN_ATTACK',
      x: 0x00,
      y: 0x00,
      param: 0,
      fix_y: 0xff,
      relative_coords: true,
      is_player_move: false,
      x_offset: 2,
      base_x: 0xb5,
      base_y: 0x00,
    });

    const resolved = (player as unknown as { resolve_sprite_position: (sprite: unknown) => [number, number] })
      .resolve_sprite_position(sprite);

    expect(resolved[0]).toBe(0x01);
    expect(resolved[1]).toBe(0x28);
  });

  it('applies Recover enemy Y offset adjustment to match ASM fix_y mirroring', () => {
    const player = new AnimationPlayer({});
    (player as unknown as { root_animation_name: string | null; enemy_fixy_adjust_active: boolean }).root_animation_name = 'BattleAnim_Recover';
    (player as unknown as { enemy_fixy_adjust_active: boolean }).enemy_fixy_adjust_active = true;
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'BATTLE_ANIM_OBJ_RECOVER',
      x: 0x00,
      y: 0x00,
      param: 0,
      fix_y: 0x90,
      relative_coords: true,
      is_player_move: false,
      base_y: 0x58,
    });

    const [, y] = (player as unknown as { resolve_sprite_position: (sprite: unknown) => [number, number] })
      .resolve_sprite_position(sprite);

    expect(y).toBe(0x30);
  });

  it('uses 8-bit enemy Y fix offsets for relative coords', () => {
    const player = new AnimationPlayer({});
    const sprite = AnimationSpriteSchema.parse({
      object_id: 'BATTLE_ANIM_OBJ_HORN_ATTACK',
      x: 0x00,
      y: 0x00,
      param: 0,
      fix_y: 0x00,
      relative_coords: true,
      is_player_move: false,
      base_y: 0x10,
    });

    const [, y] = (player as unknown as { resolve_sprite_position: (sprite: unknown) => [number, number] })
      .resolve_sprite_position(sprite);

    expect(y).toBe(0xf0);
  });

  it('keeps the previous frame on oamwait', () => {
    const player = new AnimationPlayer({});
    (player as unknown as {
      anim_data: {
        resolve_frameset_name: () => string;
        get_frameset_frames: () => Array<{
          command: string;
          oam_set: string | null;
          duration: number;
          xflip: boolean;
          yflip: boolean;
        }>;
      };
    }).anim_data = {
      resolve_frameset_name: () => 'BATTLE_ANIM_TEST',
      get_frameset_frames: () => [
        { command: 'frame', oam_set: 'BATTLE_ANIM_OAMSET_00', duration: 1, xflip: false, yflip: false },
        { command: 'wait', oam_set: null, duration: 2, xflip: false, yflip: false },
        { command: 'delete', oam_set: null, duration: 0, xflip: false, yflip: false },
      ],
    } as any;

    const sprite = AnimationSpriteSchema.parse({
      object_id: 'BATTLE_ANIM_TEST',
      x: 0,
      y: 0,
      param: 0,
    });

    const advance = (player as unknown as {
      advance_sprite_frameset: (sprite: typeof sprite) => 'remove' | null;
    }).advance_sprite_frameset;

    advance.call(player, sprite);
    advance.call(player, sprite);
    const result = advance.call(player, sprite);

    expect(result).toBeNull();
    expect(sprite.frame).toBe(0);
  });

  it('restarts the frameset on oamrestart', () => {
    const player = new AnimationPlayer({});
    (player as unknown as {
      anim_data: {
        resolve_frameset_name: () => string;
        get_frameset_frames: () => Array<{
          command: string;
          oam_set: string | null;
          duration: number;
          xflip: boolean;
          yflip: boolean;
        }>;
      };
    }).anim_data = {
      resolve_frameset_name: () => 'BATTLE_ANIM_TEST',
      get_frameset_frames: () => [
        { command: 'frame', oam_set: 'BATTLE_ANIM_OAMSET_00', duration: 0, xflip: false, yflip: false },
        { command: 'restart', oam_set: null, duration: 0, xflip: false, yflip: false },
      ],
    } as any;

    const sprite = AnimationSpriteSchema.parse({
      object_id: 'BATTLE_ANIM_TEST',
      x: 0,
      y: 0,
      param: 0,
    });

    const advance = (player as unknown as {
      advance_sprite_frameset: (sprite: typeof sprite) => 'remove' | null;
    }).advance_sprite_frameset;

    advance.call(player, sprite);
    const result = advance.call(player, sprite);

    expect(result).toBeNull();
    expect(sprite.frame).toBe(0);
    expect(sprite.frameset_index).toBe(0);
  });

  it('repeats the previous frame on oamend', () => {
    const player = new AnimationPlayer({});
    (player as unknown as {
      anim_data: {
        resolve_frameset_name: () => string;
        get_frameset_frames: () => Array<{
          command: string;
          oam_set: string | null;
          duration: number;
          xflip: boolean;
          yflip: boolean;
        }>;
      };
    }).anim_data = {
      resolve_frameset_name: () => 'BATTLE_ANIM_TEST',
      get_frameset_frames: () => [
        { command: 'frame', oam_set: 'BATTLE_ANIM_OAMSET_00', duration: 0, xflip: false, yflip: false },
        { command: 'frame', oam_set: 'BATTLE_ANIM_OAMSET_01', duration: 0, xflip: false, yflip: false },
        { command: 'end', oam_set: null, duration: 0, xflip: false, yflip: false },
      ],
    } as any;

    const sprite = AnimationSpriteSchema.parse({
      object_id: 'BATTLE_ANIM_TEST',
      x: 0,
      y: 0,
      param: 0,
    });

    const advance = (player as unknown as {
      advance_sprite_frameset: (sprite: typeof sprite) => 'remove' | null;
    }).advance_sprite_frameset;

    advance.call(player, sprite);
    advance.call(player, sprite);
    const result = advance.call(player, sprite);

    expect(result).toBeNull();
    expect(sprite.frame).toBe(1);
    expect(sprite.frameset_index).toBe(1);
  });

  it('matches ASM frame duration timing (duration 1 advances on next update)', () => {
    const player = new AnimationPlayer({});
    (player as unknown as {
      anim_data: {
        resolve_frameset_name: () => string;
        get_frameset_frames: () => Array<{
          command: string;
          oam_set: string | null;
          duration: number;
          xflip: boolean;
          yflip: boolean;
        }>;
      };
    }).anim_data = {
      resolve_frameset_name: () => 'BATTLE_ANIM_TEST',
      get_frameset_frames: () => [
        { command: 'frame', oam_set: 'BATTLE_ANIM_OAMSET_00', duration: 1, xflip: false, yflip: false },
        { command: 'frame', oam_set: 'BATTLE_ANIM_OAMSET_01', duration: 0, xflip: false, yflip: false },
      ],
    } as any;

    const sprite = AnimationSpriteSchema.parse({
      object_id: 'BATTLE_ANIM_TEST',
      x: 0,
      y: 0,
      param: 0,
    });

    const advance = (player as unknown as {
      advance_sprite_frameset: (sprite: typeof sprite) => 'remove' | null;
    }).advance_sprite_frameset;

    advance.call(player, sprite);
    expect(sprite.frame).toBe(0);

    advance.call(player, sprite);
    expect(sprite.frame).toBe(1);
  });

  it('matches ASM oamwait timing (duration 1 consumes exactly one update tick)', () => {
    const player = new AnimationPlayer({});
    (player as unknown as {
      anim_data: {
        resolve_frameset_name: () => string;
        get_frameset_frames: () => Array<{
          command: string;
          oam_set: string | null;
          duration: number;
          xflip: boolean;
          yflip: boolean;
        }>;
      };
    }).anim_data = {
      resolve_frameset_name: () => 'BATTLE_ANIM_TEST',
      get_frameset_frames: () => [
        { command: 'frame', oam_set: 'BATTLE_ANIM_OAMSET_00', duration: 0, xflip: false, yflip: false },
        { command: 'wait', oam_set: null, duration: 1, xflip: false, yflip: false },
        { command: 'delete', oam_set: null, duration: 0, xflip: false, yflip: false },
      ],
    } as any;

    const sprite = AnimationSpriteSchema.parse({
      object_id: 'BATTLE_ANIM_TEST',
      x: 0,
      y: 0,
      param: 0,
    });

    const advance = (player as unknown as {
      advance_sprite_frameset: (sprite: typeof sprite) => 'remove' | null;
    }).advance_sprite_frameset;

    advance.call(player, sprite);
    expect(sprite.frame).toBe(0);

    const result = advance.call(player, sprite);
    expect(result).toBeNull();
    expect(sprite.frame).toBe(0);

    expect(advance.call(player, sprite)).toBe('remove');
  });

  it('throws when an animation object resolves to no frameset', () => {
    const player = new AnimationPlayer({});
    (player as unknown as {
      anim_data: {
        resolve_frameset_name: () => string | null;
      };
    }).anim_data = {
      resolve_frameset_name: () => null,
    } as any;

    const sprite = AnimationSpriteSchema.parse({
      object_id: 'BATTLE_ANIM_TEST',
      x: 0,
      y: 0,
      param: 0,
    });

    expect(() =>
      (player as unknown as {
        advance_sprite_frameset: (sprite: typeof sprite) => 'remove' | null;
      }).advance_sprite_frameset(sprite),
    ).toThrow('Missing battle animation frameset for BATTLE_ANIM_TEST.');
  });

  it('throws when rendering would silently drop a missing battle animation sprite', () => {
    const player = new AnimationPlayer({});
    (player as unknown as {
      anim_data: {
        render_sprite: () => null;
      };
    }).anim_data = {
      render_sprite: () => null,
    } as any;

    const sprite = AnimationSpriteSchema.parse({
      object_id: 'BATTLE_ANIM_TEST',
      x: 0,
      y: 0,
      frame: 3,
      param: 0,
    });

    expect(() => player.render_sprite(sprite)).toThrow(
      'Missing rendered battle animation sprite for BATTLE_ANIM_TEST frame 3.',
    );
  });

  it('skips missing battle animation tiles in the browser runtime and logs once', () => {
    const originalWindow = (globalThis as typeof globalThis & { window?: unknown }).window;
    try {
      (globalThis as typeof globalThis & { window?: unknown }).window = {} as unknown;
      const warn = jest.spyOn(logger, 'warn').mockImplementation(() => {});
      const player = new AnimationPlayer({});
      (player as unknown as {
        anim_data: {
          render_sprite: () => null;
        };
      }).anim_data = {
        render_sprite: () => {
          throw new Error(
            'Missing battle animation tile 0 for BATTLE_ANIM_OBJ_BALL_POOF (AnimObjSmokeGFX/BATTLE_ANIM_OAMSET_00).',
          );
        },
      } as any;

      const sprite = AnimationSpriteSchema.parse({
        object_id: 'BATTLE_ANIM_OBJ_BALL_POOF',
        x: 0,
        y: 0,
        frame: 0,
        param: 0,
      });

      expect(player.render_sprite(sprite)).toBeNull();
      expect(player.render_sprite(sprite)).toBeNull();
      expect(warn).toHaveBeenCalledTimes(1);
    } finally {
      (globalThis as typeof globalThis & { window?: unknown }).window = originalWindow;
    }
  });

  it('skips battler object bg-effect overflow without throwing', () => {
    const player = new AnimationPlayer({});
    const objects = (player as unknown as { objects: AnimationObjectTable }).objects;

    for (let i = 0; i < 10; i += 1) {
      expect(
        objects.add(
          AnimationSpriteSchema.parse({
            object_id: `BATTLE_ANIM_OBJ_FILL_${i}`,
            x: i,
            y: i,
            param: 0,
          }),
        ),
      ).not.toBeNull();
    }

    const effect = buildParsedBgEffect({
      name: 'BATTLE_BG_EFFECT_BATTLEROBJ_1ROW',
      duration: 1,
      raw_turn: 'BG_EFFECT_USER',
      param: 0,
      is_player_move: true,
      turn_value: null,
    });

    expect(() =>
      (player as unknown as { maybe_spawn_battler_object: (parsed: typeof effect) => void }).maybe_spawn_battler_object(effect),
    ).not.toThrow();

    expect(player.active_sprites).toHaveLength(10);
    expect(
      (player as unknown as { bg_effect_sprites: Record<string, unknown> }).bg_effect_sprites[
        'BATTLE_BG_EFFECT_BATTLEROBJ_1ROW'
      ],
    ).toBeUndefined();
  });
});

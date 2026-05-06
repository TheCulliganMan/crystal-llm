import { BattleAnimObjectDef } from './_battle-anim-data';
import type { AnimationCommand } from './battle-animation-state';

export const ENEMY_MIRROR_BASE = 0xb4;
const GFX_OPCODES = ['anim_1gfx', 'anim_2gfx', 'anim_3gfx', 'anim_4gfx', 'anim_5gfx'];
const NOOP_OPCODES = [
  'anim_beatup',
  'anim_0xea',
  'anim_0xeb',
  'anim_0xec',
  'anim_0xed',
  'anim_0xe7',
  'anim_0xf5',
  'anim_0xf6',
  'anim_0xf7',
];

const HANDLER_MAP = {
  anim_wait: '_handle_wait',
  wait: '_handle_wait',
  anim_sound: '_handle_sound',
  playsound: '_handle_play_sound',
  anim_cry: '_handle_cry',
  anim_obj: '_handle_obj',
  anim_clearobjs: '_handle_clear_objs',
  anim_incobj: '_handle_inc_obj',
  anim_setobj: '_handle_set_obj',
  anim_bgp: '_handle_bgp',
  anim_obp0: '_handle_obp0',
  anim_obp1: '_handle_obp1',
  anim_resetobp0: '_handle_reset_obp0',
  anim_keepsprites: '_handle_keep_sprites',
  anim_oamon: '_handle_oam_on',
  anim_oamoff: '_handle_oam_off',
  anim_ret: '_handle_ret',
  frame: '_handle_frame',
  anim_call: '_handle_call',
  anim_jump: '_handle_jump',
  anim_loop: '_handle_loop',
  anim_jumpuntil: '_handle_jump_until',
  anim_setvar: '_handle_set_var',
  anim_incvar: '_handle_inc_var',
  anim_if_var_equal: '_handle_if_var_equal',
  anim_if_param_equal: '_handle_if_param_equal',
  anim_if_param_and: '_handle_if_param_and',
  anim_bgeffect: '_handle_bgeffect',
  anim_incbgeffect: '_handle_incbgeffect',
  anim_battlergfx_1row: '_handle_battlergfx_1row',
  anim_battlergfx_2row: '_handle_battlergfx_2row',
  anim_checkpokeball: '_handle_check_pokeball',
  anim_transform: '_handle_transform',
  anim_raisesub: '_handle_raise_sub',
  anim_dropsub: '_handle_drop_sub',
  anim_minimizeopp: '_handle_minimize_opp',
  anim_updateactorpic: '_handle_update_actor_pic',
  anim_minimize: '_handle_minimize',
} as const;

type BattleAnimHandler = (command: AnimationCommand) => unknown;
type HandlerMethodName = (typeof HANDLER_MAP)[keyof typeof HANDLER_MAP];
type BattleAnimPlayer = {
  [method in HandlerMethodName]?: BattleAnimHandler;
} & {
  _handle_gfx?: BattleAnimHandler;
  handle_gfx?: BattleAnimHandler;
  _handle_noop?: BattleAnimHandler;
  handle_noop?: BattleAnimHandler;
  [key: string]: BattleAnimHandler | undefined;
};

export const mirror_enemy_x = (x: number): number => (ENEMY_MIRROR_BASE - x) & 0xff;

export const adjust_enemy_object_coords = (
  x: number,
  y: number,
  objDef: BattleAnimObjectDef | null,
  tileSize: number,
  fixY?: number | null,
): [number, number, boolean, boolean] => {
  if (!objDef) {
    return [x, y, false, false];
  }
  let mirroredX = x;
  let mirroredY = y;
  let mirrorX = false;
  let mirrorY = false;
  if (objDef.flags & 0x01) {
    const currentFixY = fixY ?? objDef.fix_y;
    mirroredX = mirror_enemy_x(x);
    mirroredY = currentFixY === 0xff ? (y + 5 * tileSize) & 0xff : (currentFixY - y) & 0xff;
  }
  if (objDef.flags & 0x20) {
    mirrorX = true;
  }
  if (objDef.flags & 0x40) {
    mirrorY = true;
  }
  return [mirroredX, mirroredY, mirrorX, mirrorY];
};

export const update_ember_sprite = (sprite: { jump_index: number; param: number; x: number; y: number; override_frameset?: string | null }): 'remove' | null => {
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

export const build_command_handlers = (player: BattleAnimPlayer): Record<string, BattleAnimHandler> => {
  const handlers: Record<string, BattleAnimHandler> = {};
  const entries = Object.entries(HANDLER_MAP) as Array<[keyof typeof HANDLER_MAP, HandlerMethodName]>;
  for (const [name, method] of entries) {
    let func = player[method];
    if (typeof func !== 'function' && method.startsWith('_handle_')) {
      const fallbackName = `handle_${method.slice('_handle_'.length)}`;
      func = player[fallbackName];
    }
    if (typeof func !== 'function') {
      throw new Error(`Missing handler ${method} for opcode ${name}`);
    }
    handlers[name] = func.bind(player);
  }
  for (const opcode of GFX_OPCODES) {
    let gfxHandler = player._handle_gfx;
    if (typeof gfxHandler !== 'function') {
      gfxHandler = player.handle_gfx;
    }
    if (typeof gfxHandler !== 'function') {
      throw new Error(`Missing handler _handle_gfx for opcode ${opcode}`);
    }
    handlers[opcode] = gfxHandler.bind(player);
  }
  for (const opcode of NOOP_OPCODES) {
    let noopHandler = player._handle_noop;
    if (typeof noopHandler !== 'function') {
      noopHandler = player.handle_noop;
    }
    if (typeof noopHandler !== 'function') {
      throw new Error(`Missing handler _handle_noop for opcode ${opcode}`);
    }
    handlers[opcode] = noopHandler.bind(player);
  }
  return handlers;
};

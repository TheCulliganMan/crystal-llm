import { AnimationPlayer } from './battle-animation';
import { load_animation_scripts, load_animation_table } from './battle-animation-loader';
import {
  Animation,
  AnimationCommand,
  AnimationContext,
  AnimationObjectTable,
  AnimationSprite,
  AnimationSpriteSchema,
  AnimationSound,
  BattleAnimationFlag,
  LoopState,
  parse_battle_int,
} from './battle-animation-state';
import { DMG_BGP_DEFAULT, DMG_OBP0_ASSIGNED, DMG_OBP_DEFAULT } from './_battle-palettes';
import {
  BattleAnimData,
  BattleAnimFrame,
  BattleAnimOAMSet,
  OamEntry,
} from './battle-anim-data';
import { update_animation_sprite } from './battle-anim-runtime';
import {
  BattlerSide,
  BattleAnimationRuntime,
  BattleBgEffect,
  ParsedBgEffect,
  buildParsedBgEffect,
  make_effect,
} from './battle-bg-effects';
import { mirror_enemy_x } from './battle-animation-helpers';

export interface BattleAnimationAuditMatrix {
  move_animation_labels: string[];
  support_animation_labels: string[];
  audited_animation_labels: string[];
}

export interface BattleAnimationAuditVariant {
  animation_name: string;
  param: number;
  param_label: string | null;
  shake_count: number | null;
}

export interface BattleAnimationCommandTrace {
  animation_name: string;
  pointer: number;
  command: string;
  args: string[];
  is_player_move: boolean;
}

export interface BattleAnimationSpriteTrace {
  object_id: string;
  function_id: string | null;
  x: number;
  y: number;
  resolved_x: number;
  resolved_y: number;
  fix_y: number;
  base_x: number | null;
  base_y: number | null;
  x_offset: number;
  y_offset: number;
  var1: number;
  var2: number;
  param: number;
  state: number;
  jump_index: number;
  frame: number;
  frame_duration: number;
  frameset_index: number;
  active_frameset: string | null;
  mirror_x: boolean;
  mirror_y: boolean;
  relative_coords: boolean;
  is_player_move: boolean;
  param_label: string | null;
  target_shakes: number | null;
  wobble_count: number;
  frame_entry: BattleAnimFrame | null;
  oam_set: BattleAnimOAMSet | null;
}

export interface BattleAnimationFrameTrace {
  frame: number;
  current_context: {
    animation_name: string;
    pointer: number;
    is_player_move: boolean;
  } | null;
  root_animation_name: string | null;
  battle_anim_delay: number;
  current_frame_tag: string | null;
  palette_state: Record<string, number | null>;
  anim_var: number;
  anim_param: number;
  anim_param_label: string | null;
  target_shakes: number | null;
  loop_state: LoopState | null;
  flags: number;
  row_mode: number;
  subroutine_depth: number;
  oam_enabled: boolean;
  keep_sprites: boolean;
  runtime: Record<string, unknown>;
  commands: BattleAnimationCommandTrace[];
  sounds: AnimationSound[];
  bg_effects: Record<string, unknown>[];
  sprites: BattleAnimationSpriteTrace[];
}

export interface BattleAnimationTrace {
  animation_name: string;
  is_player_move: boolean;
  completed: boolean;
  frames: BattleAnimationFrameTrace[];
}

export interface BattleAnimationReferenceFrame {
  frame: number;
  current_context: {
    animation_name: string;
    pointer: number;
    is_player_move: boolean;
  } | null;
  battle_anim_delay: number;
  anim_var: number;
  anim_param: number;
  anim_param_label: string | null;
  target_shakes: number | null;
  loop_state: LoopState | null;
  flags: number;
  palette_state: Record<string, number | null>;
  row_mode: number;
  subroutine_depth: number;
  oam_enabled: boolean;
  keep_sprites: boolean;
  current_frame_tag: string | null;
  runtime: Record<string, unknown>;
  bg_effects: Record<string, unknown>[];
  sprites: BattleAnimationSpriteTrace[];
  commands: BattleAnimationCommandTrace[];
  sounds: AnimationSound[];
}

export interface BattleAnimationReferenceTrace {
  animation_name: string;
  is_player_move: boolean;
  completed: boolean;
  frames: BattleAnimationReferenceFrame[];
}

const MAX_TRACE_FRAMES = 1600;
const MAX_COMMANDS_PER_FRAME = 1024;
const MAX_ANIM_OBJECTS = 10;

const PARAM_LABEL_VARIANTS = [
  'MASTER_BALL',
  'ULTRA_BALL',
  'GREAT_BALL',
  'POKE_BALL',
  'HEAVY_BALL',
  'LEVEL_BALL',
  'LURE_BALL',
  'FAST_BALL',
  'FRIEND_BALL',
  'MOON_BALL',
  'LOVE_BALL',
];
const ENEMY_FIXY_ADJUST = new Set([
  'BattleAnim_Kinesis',
  'BattleAnim_Recover',
  'BattleAnim_Softboiled',
  'BattleAnim_MilkDrink',
]);

const cloneFrameEntry = (entry: BattleAnimFrame | null): BattleAnimFrame | null => {
  if (!entry) {
    return null;
  }
  return { ...entry };
};

const cloneOamSet = (set: BattleAnimOAMSet | null): BattleAnimOAMSet | null => {
  if (!set) {
    return null;
  }
  return {
    ...set,
    entries: set.entries.map((entry: OamEntry) => ({ ...entry })),
  };
};

const cloneSound = (sound: AnimationSound): AnimationSound => ({
  ...sound,
});

const traceSpriteFromData = (
  data: BattleAnimData,
  sprite: AnimationSprite,
  resolvedPosition: [number, number],
): BattleAnimationSpriteTrace => {
  const framesetName = data.resolve_frameset_name(
    sprite.object_id,
    sprite.override_frameset ?? null,
  );
  const frames = framesetName
    ? data.get_frameset_frames(framesetName)
    : null;
  const frameEntry =
    frames && sprite.frame >= 0 && sprite.frame < frames.length
      ? cloneFrameEntry(frames[sprite.frame] ?? null)
      : null;
  const oamSet =
    frameEntry?.oam_set
      ? cloneOamSet(data.get_oam_set(frameEntry.oam_set))
      : null;
  const [resolvedX, resolvedY] = resolvedPosition;
  return {
    object_id: sprite.object_id,
    function_id: sprite.function_id ?? null,
    x: sprite.x,
    y: sprite.y,
    resolved_x: resolvedX,
    resolved_y: resolvedY,
    fix_y: sprite.fix_y,
    base_x: sprite.base_x ?? null,
    base_y: sprite.base_y ?? null,
    x_offset: sprite.x_offset,
    y_offset: sprite.y_offset,
    var1: sprite.var1,
    var2: sprite.var2,
    param: sprite.param,
    state: sprite.state,
    jump_index: sprite.jump_index,
    frame: sprite.frame,
    frame_duration: sprite.frame_duration,
    frameset_index: sprite.frameset_index,
    active_frameset: framesetName ?? null,
    mirror_x: sprite.mirror_x,
    mirror_y: sprite.mirror_y,
    relative_coords: sprite.relative_coords,
    is_player_move: sprite.is_player_move,
    param_label: sprite.param_label ?? null,
    target_shakes: sprite.target_shakes ?? null,
    wobble_count: sprite.wobble_count,
    frame_entry: frameEntry,
    oam_set: oamSet,
  };
};

const traceSprite = (
  player: AnimationPlayer,
  sprite: AnimationPlayer['active_sprites'][number],
): BattleAnimationSpriteTrace => {
  const [resolvedX, resolvedY] = player.resolve_sprite_position(sprite);
  return traceSpriteFromData(player.anim_data, sprite, [resolvedX, resolvedY]);
};

const traceFrame = (
  player: AnimationPlayer,
  frame: number,
): BattleAnimationFrameTrace => {
  const state = player.trace_state as {
    current_context: BattleAnimationFrameTrace['current_context'];
    root_animation_name: string | null;
    battle_anim_delay: number;
    current_frame_tag: string | null;
    palette_state: Record<string, number | null>;
    anim_var: number;
    anim_param: number;
    anim_param_label: string | null;
    target_shakes: number | null;
    loop_state: LoopState | null;
    flags: number;
    row_mode: number;
    subroutine_depth: number;
    oam_enabled: boolean;
    keep_sprites: boolean;
    runtime: Record<string, unknown>;
    bg_effects: Record<string, unknown>[];
  };
  return {
    frame,
    current_context: state.current_context,
    root_animation_name: state.root_animation_name,
    battle_anim_delay: state.battle_anim_delay,
    current_frame_tag: state.current_frame_tag,
    palette_state: { ...state.palette_state },
    anim_var: state.anim_var,
    anim_param: state.anim_param,
    anim_param_label: state.anim_param_label,
    target_shakes: state.target_shakes,
    loop_state: state.loop_state ? { ...state.loop_state } : null,
    flags: state.flags,
    row_mode: state.row_mode,
    subroutine_depth: state.subroutine_depth,
    oam_enabled: state.oam_enabled,
    keep_sprites: state.keep_sprites,
    runtime: state.runtime,
    commands: player.trace_commands.map((command) => ({
      ...command,
      args: [...command.args],
    })),
    sounds: player.pending_audio().map(cloneSound),
    bg_effects: state.bg_effects.map((effect) => ({ ...effect })),
    sprites: player.active_sprites.map((sprite) => traceSprite(player, sprite)),
  };
};

export const build_battle_animation_audit_matrix = (): BattleAnimationAuditMatrix => {
  const scripts = load_animation_scripts();
  const moveAnimationLabels = Array.from(new Set(load_animation_table()));
  const reachable = new Set<string>(moveAnimationLabels);
  const queue = [...moveAnimationLabels];

  while (queue.length) {
    const animationName = queue.shift() as string;
    const animation = scripts.get(animationName);
    if (!animation) {
      continue;
    }
    for (const command of animation.script) {
      const opcode = command.command.toLowerCase();
      if (!['anim_call', 'anim_jump'].includes(opcode)) {
        continue;
      }
      const target = command.args[0]?.trim();
      if (!target || target.startsWith('.')) {
        continue;
      }
      if (!scripts.has(target) || reachable.has(target)) {
        continue;
      }
      reachable.add(target);
      queue.push(target);
    }
  }

  const supportAnimationLabels = Array.from(reachable)
    .filter((label) => !moveAnimationLabels.includes(label))
    .sort();

  return {
    move_animation_labels: [...moveAnimationLabels],
    support_animation_labels: supportAnimationLabels,
    audited_animation_labels: [...moveAnimationLabels, ...supportAnimationLabels],
  };
};

export const build_battle_animation_audit_variants = (
  animation_name: string,
): BattleAnimationAuditVariant[] => {
  const scripts = load_animation_scripts();
  const animation = scripts.get(animation_name);
  if (!animation) {
    return [{ animation_name, param: 0, param_label: null, shake_count: null }];
  }
  const variants = new Map<string, BattleAnimationAuditVariant>();
  const addVariant = (variant: Omit<BattleAnimationAuditVariant, 'animation_name'>) => {
    const full = { animation_name, ...variant };
    variants.set(
      `${full.param}:${full.param_label ?? ''}:${full.shake_count ?? ''}`,
      full,
    );
  };
  addVariant({ param: 0, param_label: null, shake_count: null });

  for (const command of animation.script) {
    const opcode = command.command.toLowerCase();
    if (opcode === 'anim_if_param_equal' && command.args.length >= 1) {
      const parsed = parse_battle_int(command.args[0]);
      if (parsed !== null) {
        addVariant({ param: Math.max(0, parsed), param_label: null, shake_count: null });
      } else {
        addVariant({
          param: 0,
          param_label: command.args[0].trim().toUpperCase(),
          shake_count: null,
        });
      }
    }
    if (opcode === 'anim_if_param_and' && command.args.length >= 1) {
      const mask = parse_battle_int(command.args[0]) ?? 0;
      if (mask > 0) {
        addVariant({ param: mask, param_label: null, shake_count: null });
      }
    }
    if (
      opcode === 'anim_obj' &&
      command.args[0]?.trim().toUpperCase() === 'BATTLE_ANIM_OBJ_POKE_BALL'
    ) {
      for (const shake_count of [0, 1, 2, 3, 4]) {
        addVariant({ param: 0, param_label: 'POKE_BALL', shake_count });
      }
      for (const param_label of PARAM_LABEL_VARIANTS) {
        addVariant({ param: 0, param_label, shake_count: 4 });
      }
    }
  }

  return Array.from(variants.values()).sort((left, right) =>
    `${left.param}:${left.param_label ?? ''}:${left.shake_count ?? ''}`.localeCompare(
      `${right.param}:${right.param_label ?? ''}:${right.shake_count ?? ''}`,
    ),
  );
};

export const trace_battle_animation = (options: {
  animation_name: string;
  is_player_move: boolean;
  param?: number | null;
  param_label?: string | null;
  shake_count?: number | null;
  tile_size?: number;
}): BattleAnimationTrace => {
  const player = new AnimationPlayer({ tile_size: options.tile_size ?? 8 });
  player.play_animation(
    options.animation_name,
    options.is_player_move,
    options.param ?? 0,
    {
      param_label: options.param_label ?? null,
      shake_count: options.shake_count ?? null,
    },
  );

  const frames: BattleAnimationFrameTrace[] = [];
  for (let frame = 0; frame < MAX_TRACE_FRAMES; frame += 1) {
    if (!player.is_active()) {
      return {
        animation_name: options.animation_name,
        is_player_move: options.is_player_move,
        completed: true,
        frames,
      };
    }
    player.update();
    frames.push(traceFrame(player, frame));
  }

  throw new Error(
    `Battle animation trace exceeded ${MAX_TRACE_FRAMES} frames for ${options.animation_name}.`,
  );
};

class ScriptReferenceRunner {
  private readonly animations = load_animation_scripts();
  private readonly anim_data = new BattleAnimData();
  private current_context: AnimationContext | null = null;
  private subroutine_stack: AnimationContext[] = [];
  private loop_state: LoopState | null = null;
  private battle_anim_delay = 0;
  private current_frame_tag: string | null = null;
  private anim_var = 0;
  private anim_param = 0;
  private anim_param_label: string | null = null;
  private target_shakes: number | null = null;
  private keep_sprites = false;
  private oam_enabled = true;
  private row_mode = 0;
  private flags = 0;
  private wait_commanded = false;
  private readonly objects = new AnimationObjectTable(MAX_ANIM_OBJECTS);
  private readonly bg_effects: Record<string, BattleBgEffect> = {};
  private readonly bg_effect_sprites: Record<string, AnimationSprite> = {};
  private readonly runtime = new BattleAnimationRuntime();
  private palette_state: Record<string, number | null> = {
    bgp: DMG_BGP_DEFAULT,
    obp0: DMG_OBP0_ASSIGNED,
    obp1: DMG_OBP_DEFAULT,
  };
  private last_commands: BattleAnimationCommandTrace[] = [];
  private last_sounds: AnimationSound[] = [];

  constructor(
    private readonly animation_name: string,
    private readonly is_player_move: boolean,
    param: number,
    param_label: string | null,
    shake_count: number | null,
    private readonly tile_size: number,
  ) {
    const animation = this.animations.get(animation_name);
    if (!animation) {
      throw new Error(`Missing animation '${animation_name}' in script reference runner.`);
    }
    this.current_context = new AnimationContext({
      animation,
      pointer: 0,
      is_player_move,
    });
    this.anim_param = Math.max(0, Math.trunc(param));
    this.anim_param_label = param_label;
    this.target_shakes = shake_count;
  }

  run(): BattleAnimationReferenceTrace {
    const frames: BattleAnimationReferenceFrame[] = [];

    for (let frame = 0; frame < MAX_TRACE_FRAMES; frame += 1) {
      if (!this.is_active()) {
        return {
          animation_name: this.animation_name,
          is_player_move: this.is_player_move,
          completed: true,
          frames,
        };
      }
      this.step_frame();
      frames.push({
        frame,
        current_context: this.current_context
          ? {
              animation_name: this.current_context.animation.name,
              pointer: this.current_context.pointer,
              is_player_move: this.current_context.is_player_move,
            }
          : null,
        battle_anim_delay: this.battle_anim_delay,
        anim_var: this.anim_var,
        anim_param: this.anim_param,
        anim_param_label: this.anim_param_label,
        target_shakes: this.target_shakes,
        loop_state: this.loop_state ? { ...this.loop_state } : null,
        flags: this.flags,
        palette_state: { ...this.palette_state },
        row_mode: this.row_mode,
        subroutine_depth: this.subroutine_stack.length,
        oam_enabled: this.oam_enabled,
        keep_sprites: this.keep_sprites,
        current_frame_tag: this.current_frame_tag,
        runtime: this.runtime.snapshot(),
        bg_effects: Object.entries(this.bg_effects).map(([name, effect]) => effect.snapshot(name)),
        sprites: this.objects.sprites.map((sprite) =>
          traceSpriteFromData(this.anim_data, sprite, this.resolve_sprite_position(sprite)),
        ),
        commands: this.last_commands.map((command) => ({
          ...command,
          args: [...command.args],
        })),
        sounds: this.last_sounds.map(cloneSound),
      });
    }

    throw new Error(
      `Battle animation reference trace exceeded ${MAX_TRACE_FRAMES} frames for ${this.animation_name}.`,
    );
  }

  private is_active(): boolean {
    return (
      this.current_context !== null ||
      this.battle_anim_delay > 0 ||
      Object.keys(this.bg_effects).length > 0
    );
  }

  private step_frame(): void {
    this.last_commands = [];
    this.last_sounds = [];
    this.runtime.reset_transforms();
    this.apply_row_mode();
    if (this.battle_anim_delay > 0) {
      this.battle_anim_delay -= 1;
      this.update_objects();
      this.update_bg_effects();
      return;
    }
    this.execute_commands_for_frame();
    this.update_objects();
    this.update_bg_effects();
  }

  private execute_commands_for_frame(): void {
    let executed = 0;
    while (this.current_context) {
      if (executed >= MAX_COMMANDS_PER_FRAME) {
        throw new Error(
          `Battle animation reference exceeded per-frame command budget for ${this.animation_name}.`,
        );
      }
      const context = this.current_context;
      if (context.pointer >= context.animation.script.length) {
        this.finish_current_context();
        continue;
      }
      const pointer = context.pointer;
      const command = context.animation.script[context.pointer];
      context.pointer += 1;
      this.last_commands.push({
        animation_name: context.animation.name,
        pointer,
        command: command.command,
        args: [...command.args],
        is_player_move: context.is_player_move,
      });
      this.dispatch_command(command);
      executed += 1;
      if (this.wait_commanded) {
        this.wait_commanded = false;
        return;
      }
      if (this.battle_anim_delay > 0) {
        return;
      }
    }
  }

  private dispatch_command(command: AnimationCommand): void {
    switch (command.command.toLowerCase()) {
      case 'anim_wait':
      case 'wait':
        this.handle_wait(command);
        return;
      case 'anim_sound':
        this.handle_sound(command);
        return;
      case 'playsound':
        this.handle_play_sound(command);
        return;
      case 'anim_cry':
        this.handle_cry(command);
        return;
      case 'anim_obj':
        this.handle_obj(command);
        return;
      case 'anim_clearobjs':
        this.objects.clear();
        return;
      case 'anim_incobj':
        if (command.args.length) {
          this.objects.increment(command.args[0]);
        }
        return;
      case 'anim_setobj':
        if (command.args.length >= 2) {
          this.objects.set_jump_index(command.args[0], parse_battle_int(command.args[1]) ?? 0);
        }
        return;
      case 'anim_bgeffect':
        this.handle_bgeffect(command);
        return;
      case 'anim_incbgeffect':
        if (command.args.length) {
          this.bg_effects[command.args[0]]?.increment();
        }
        return;
      case 'anim_transform':
        this.handle_transform('transform', false);
        return;
      case 'anim_raisesub':
        this.handle_transform('substitute', false);
        return;
      case 'anim_dropsub':
      case 'anim_updateactorpic':
        this.handle_transform(null, false);
        return;
      case 'anim_minimize':
        this.handle_transform('minimize', false);
        return;
      case 'anim_minimizeopp':
        this.handle_transform('minimize', true);
        return;
      case 'anim_battlergfx_1row':
      case 'anim_battlergfx_2row':
      case 'anim_checkpokeball':
      case 'anim_beatup':
      case 'anim_0xe7':
      case 'anim_0xea':
      case 'anim_0xeb':
      case 'anim_0xec':
      case 'anim_0xed':
      case 'anim_0xf5':
      case 'anim_0xf6':
      case 'anim_0xf7':
      case 'anim_1gfx':
      case 'anim_2gfx':
      case 'anim_3gfx':
      case 'anim_4gfx':
      case 'anim_5gfx':
      case 'frame':
        if (command.command.toLowerCase() === 'frame' && command.args.length) {
          this.current_frame_tag = command.args[0];
        } else if (command.command.toLowerCase().startsWith('anim_') && command.command.toLowerCase().endsWith('gfx') && command.args.length) {
          this.current_frame_tag = command.args[0];
        }
        return;
      case 'anim_bgp':
        this.palette_state.bgp = command.args.length ? parse_battle_int(command.args[0]) : null;
        return;
      case 'anim_obp0':
        this.palette_state.obp0 = command.args.length ? parse_battle_int(command.args[0]) : null;
        return;
      case 'anim_obp1':
        this.palette_state.obp1 = command.args.length ? parse_battle_int(command.args[0]) : null;
        return;
      case 'anim_resetobp0':
        this.palette_state.obp0 = DMG_OBP0_ASSIGNED;
        return;
      case 'anim_keepsprites':
        this.keep_sprites = true;
        return;
      case 'anim_oamon':
        this.oam_enabled = true;
        return;
      case 'anim_oamoff':
        this.oam_enabled = false;
        return;
      case 'anim_ret':
        this.finish_current_context();
        return;
      case 'anim_call':
        this.handle_call(command);
        return;
      case 'anim_jump':
        this.handle_jump(command);
        return;
      case 'anim_loop':
        this.handle_loop(command);
        return;
      case 'anim_jumpuntil':
        this.handle_jump_until(command);
        return;
      case 'anim_setvar':
        this.anim_var = Math.max(0, parse_battle_int(command.args[0]) ?? 0);
        return;
      case 'anim_incvar':
        this.anim_var = (this.anim_var + 1) & 0xff;
        return;
      case 'anim_if_var_equal':
        this.handle_if_var_equal(command);
        return;
      case 'anim_if_param_equal':
        this.handle_if_param_equal(command);
        return;
      case 'anim_if_param_and':
        this.handle_if_param_and(command);
        return;
      default:
        throw new Error(
          `Unknown battle animation opcode '${command.command}' in reference runner for ${this.animation_name}.`,
        );
    }
  }

  private handle_wait(command: AnimationCommand): void {
    const value = command.args.length ? parse_battle_int(command.args[0]) : 0;
    this.battle_anim_delay = Math.max(0, Math.trunc(value ?? 0));
    this.wait_commanded = true;
  }

  private handle_sound(command: AnimationCommand): void {
    let duration = null;
    let tracks = null;
    if (command.args.length > 1) {
      duration = parse_battle_int(command.args[0]);
      tracks = parse_battle_int(command.args[1]);
    } else if (command.args.length > 0) {
      const packed = parse_battle_int(command.args[0]);
      if (packed !== null) {
        duration = packed >> 2;
        tracks = packed & 0x03;
      }
    }
    const sound_id = command.args.length > 2 ? command.args[2] : null;
    if (!sound_id) {
      return;
    }
    this.last_sounds.push({
      sound_id,
      duration,
      tracks,
      sound_type: 'sound',
      pitch: null,
      panning: null,
    });
  }

  private handle_play_sound(command: AnimationCommand): void {
    if (!command.args.length) {
      return;
    }
    this.last_sounds.push({
      sound_id: command.args[0],
      duration: null,
      tracks: null,
      sound_type: 'sound',
      pitch: null,
      panning: null,
    });
  }

  private handle_cry(command: AnimationCommand): void {
    const crySelector = command.args.length ? parse_battle_int(command.args[0]) : 0;
    this.last_sounds.push({
      sound_id: 'cry',
      duration: null,
      tracks: null,
      sound_type: 'cry',
      pitch: null,
      cry_selector: crySelector ?? 0,
      panning: this.current_context?.is_player_move ? 'player' : 'enemy',
    });
  }

  private handle_obj(command: AnimationCommand): void {
    if (command.args.length < 4) {
      throw new Error('anim_obj requires at least four arguments');
    }
    const objectId = command.args[0];
    const objDef = this.anim_data.object_defs.get(objectId.toUpperCase());
    const flags = objDef ? objDef.flags : 0;
    const isPlayerMove = this.current_context?.is_player_move ?? this.is_player_move;
    const applyObjFlags = !isPlayerMove;
    const relativeCoords = Boolean(flags & 0x01);
    const mirrorXFlag = applyObjFlags ? Boolean(flags & 0x20) : false;
    const mirrorYFlag = applyObjFlags ? Boolean(flags & 0x40) : false;

    let x = 0;
    let y = 0;
    let paramIndex = 3;
    if (command.args.length >= 6) {
      x = this.tile_size * (parse_battle_int(command.args[1]) ?? 0) + (parse_battle_int(command.args[2]) ?? 0);
      y = this.tile_size * (parse_battle_int(command.args[3]) ?? 0) + (parse_battle_int(command.args[4]) ?? 0);
      paramIndex = 5;
    } else {
      x = parse_battle_int(command.args[1]) ?? 0;
      y = parse_battle_int(command.args[2]) ?? 0;
    }

    const sprite = AnimationSpriteSchema.parse({
      object_id: objectId,
      function_id: objDef ? objDef.function : null,
      x,
      y,
      fix_y: objDef ? objDef.fix_y : 0,
      override_frameset: objDef ? objDef.frameset : null,
      param: parse_battle_int(command.args[paramIndex]) ?? 0,
      mirror_x: mirrorXFlag,
      mirror_y: mirrorYFlag,
      relative_coords: relativeCoords,
      is_player_move: isPlayerMove,
      jump_index: 0,
      param_label: this.anim_param_label,
      target_shakes: this.target_shakes,
    });
    this.objects.add(sprite);
  }

  private handle_bgeffect(command: AnimationCommand): void {
    const parsed = this.parse_bg_effect(command);
    this.bg_effects[parsed.name] = make_effect(parsed);
    this.maybe_spawn_battler_object(parsed);
  }

  private parse_bg_effect(command: AnimationCommand): ParsedBgEffect {
    if (command.args.length < 4) {
      throw new Error('anim_bgeffect requires four arguments');
    }
    const rawTurn = command.args[2];
    return buildParsedBgEffect({
      name: command.args[0],
      duration: Math.max(0, parse_battle_int(command.args[1]) ?? 0),
      raw_turn: rawTurn,
      param: Math.max(0, parse_battle_int(command.args[3]) ?? 0),
      is_player_move: this.current_context?.is_player_move ?? this.is_player_move,
      turn_value: parse_battle_int(rawTurn) ?? null,
    });
  }

  private maybe_spawn_battler_object(effect: ParsedBgEffect): void {
    if (!['BATTLE_BG_EFFECT_BATTLEROBJ_1ROW', 'BATTLE_BG_EFFECT_BATTLEROBJ_2ROW'].includes(effect.name)) {
      return;
    }
    this.remove_battler_object_sprite(effect.name);
    const side = effect.target_side();
    const objectId = side === BattlerSide.PLAYER
      ? (effect.name.endsWith('1ROW') ? 'BATTLE_ANIM_OBJ_PLAYERHEAD_1ROW' : 'BATTLE_ANIM_OBJ_PLAYERHEAD_2ROW')
      : (effect.name.endsWith('1ROW') ? 'BATTLE_ANIM_OBJ_ENEMYFEET_1ROW' : 'BATTLE_ANIM_OBJ_ENEMYFEET_2ROW');
    const objDef = this.anim_data.object_defs.get(objectId);
    const isPlayerMove = side === BattlerSide.PLAYER;
    const applyObjFlags = !isPlayerMove;
    const sprite = AnimationSpriteSchema.parse({
      object_id: objectId,
      x: side === BattlerSide.PLAYER ? this.tile_size * 6 : this.tile_size * 16 + 4,
      y: this.tile_size * 8,
      function_id: objDef ? objDef.function : null,
      param: 0,
      mirror_x: objDef ? (objDef.flags & 0x20) !== 0 && applyObjFlags : false,
      mirror_y: objDef ? (objDef.flags & 0x40) !== 0 && applyObjFlags : false,
      relative_coords: objDef ? (objDef.flags & 0x01) !== 0 : false,
      is_player_move: isPlayerMove,
      override_frameset: objDef ? objDef.frameset : null,
      fix_y: objDef ? objDef.fix_y : 0,
    });
    const added = this.objects.add(sprite);
    if (added) {
      this.bg_effect_sprites[effect.name] = added;
    }
  }

  private remove_battler_object_sprite(effectName: string): void {
    const sprite = this.bg_effect_sprites[effectName];
    if (sprite) {
      delete this.bg_effect_sprites[effectName];
      this.objects.remove(sprite);
    }
  }

  private handle_transform(value: string | null, targetOpposite: boolean): void {
    const isPlayer = this.current_context?.is_player_move ?? this.is_player_move;
    const targetPlayer = targetOpposite ? !isPlayer : isPlayer;
    if (targetPlayer) {
      this.runtime.player_sprite_override = value;
    } else {
      this.runtime.enemy_sprite_override = value;
    }
  }

  private finish_current_context(): void {
    if (this.subroutine_stack.length) {
      this.current_context = this.subroutine_stack.pop() ?? null;
      this.sync_flags();
      return;
    }
    this.current_context = null;
    this.loop_state = null;
    this.flags |= BattleAnimationFlag.STOP;
    this.sync_flags();
    if (!this.keep_sprites) {
      this.objects.reset();
      for (const key of Object.keys(this.bg_effect_sprites)) {
        delete this.bg_effect_sprites[key];
      }
    } else {
      this.keep_sprites = false;
    }
    this.row_mode = 0;
    if (!Object.keys(this.bg_effects).length) {
      this.palette_state = {
        bgp: DMG_BGP_DEFAULT,
        obp0: DMG_OBP_DEFAULT,
        obp1: DMG_OBP_DEFAULT,
      };
    }
  }

  private sync_flags(): void {
    if (this.subroutine_stack.length) {
      this.flags |= BattleAnimationFlag.IN_SUBROUTINE;
    } else {
      this.flags &= ~BattleAnimationFlag.IN_SUBROUTINE;
    }
  }

  private jump_within_context(label: string): void {
    const context = this.current_context;
    if (!context) {
      return;
    }
    const position = context.animation.labels[label];
    if (position === undefined) {
      return;
    }
    context.pointer = position;
  }

  private resolve_label(label: string): [Animation, number] | null {
    if (!label) {
      return null;
    }
    if (label.startsWith('.')) {
      const context = this.current_context;
      if (!context) {
        return null;
      }
      const position = context.animation.labels[label];
      if (position === undefined) {
        return null;
      }
      return [context.animation, position];
    }
    const animation = this.animations.get(label);
    if (!animation) {
      return null;
    }
    return [animation, animation.labels[label] ?? 0];
  }

  private handle_call(command: AnimationCommand): void {
    if (!command.args.length) {
      return;
    }
    const target = this.resolve_label(command.args[0]);
    if (!target) {
      return;
    }
    if (this.current_context) {
      this.subroutine_stack.push(this.current_context.clone());
    }
    const [animation, pointer] = target;
    this.current_context = new AnimationContext({
      animation,
      pointer,
      is_player_move: this.current_context?.is_player_move ?? this.is_player_move,
    });
    this.sync_flags();
  }

  private handle_jump(command: AnimationCommand): void {
    if (!command.args.length) {
      return;
    }
    if (command.args[0].startsWith('.')) {
      this.jump_within_context(command.args[0]);
      return;
    }
    const target = this.resolve_label(command.args[0]);
    if (!target) {
      return;
    }
    const [animation, pointer] = target;
    this.current_context = new AnimationContext({
      animation,
      pointer,
      is_player_move: this.current_context?.is_player_move ?? this.is_player_move,
    });
  }

  private handle_loop(command: AnimationCommand): void {
    if (command.args.length < 2) {
      return;
    }
    const count = parse_battle_int(command.args[0]) ?? 0;
    const label = command.args[1];
    const context = this.current_context;
    if (!context) {
      return;
    }
    const commandIndex = Math.max(0, context.pointer - 1);
    const state = this.loop_state;
    if (!state || state.command_index !== commandIndex) {
      const remaining = count <= 0 ? -1 : count - 1;
      this.loop_state = { command_index: commandIndex, remaining, label };
      this.jump_within_context(label);
      return;
    }
    if (state.remaining === -1) {
      this.jump_within_context(label);
      return;
    }
    if (state.remaining > 0) {
      state.remaining -= 1;
      this.jump_within_context(label);
      return;
    }
    this.loop_state = null;
  }

  private handle_jump_until(command: AnimationCommand): void {
    if (!command.args.length) {
      return;
    }
    if (this.anim_param > 0) {
      this.anim_param = Math.max(0, this.anim_param - 1);
      this.jump_within_context(command.args[0]);
    }
  }

  private handle_if_var_equal(command: AnimationCommand): void {
    if (command.args.length < 2) {
      return;
    }
    const value = parse_battle_int(command.args[0]) ?? 0;
    if (value === this.anim_var) {
      this.jump_within_context(command.args[1]);
    }
  }

  private handle_if_param_equal(command: AnimationCommand): void {
    if (command.args.length < 2) {
      return;
    }
    const parsed = parse_battle_int(command.args[0]);
    if (parsed !== null && parsed === this.anim_param) {
      this.jump_within_context(command.args[1]);
      return;
    }
    if (this.anim_param_label && command.args[0].trim().toUpperCase() === this.anim_param_label) {
      this.jump_within_context(command.args[1]);
    }
  }

  private handle_if_param_and(command: AnimationCommand): void {
    if (command.args.length < 2) {
      return;
    }
    const mask = parse_battle_int(command.args[0]) ?? 0;
    if (this.anim_param & mask) {
      this.jump_within_context(command.args[1]);
    }
  }

  private update_bg_effects(): void {
    const expired: string[] = [];
    for (const [name, effect] of Object.entries(this.bg_effects)) {
      if (!effect.update(this.runtime)) {
        expired.push(name);
      }
    }
    for (const name of expired) {
      delete this.bg_effects[name];
      this.remove_battler_object_sprite(name);
    }
    if (this.runtime.bgp !== null && this.runtime.bgp !== undefined) {
      this.palette_state.bgp = this.runtime.bgp;
    }
    if (this.runtime.obp0 !== null && this.runtime.obp0 !== undefined) {
      this.palette_state.obp0 = this.runtime.obp0;
    }
    if (this.runtime.obp1 !== null && this.runtime.obp1 !== undefined) {
      this.palette_state.obp1 = this.runtime.obp1;
    }
  }

  private update_objects(): void {
    const removed: AnimationSprite[] = [];
    for (const sprite of [...this.objects.sprites]) {
      const result = update_animation_sprite(sprite, this.runtime);
      if (result === 'remove' || this.advance_sprite_frameset(sprite) === 'remove') {
        removed.push(sprite);
      }
    }
    for (const sprite of removed) {
      this.objects.remove(sprite);
    }
  }

  private advance_sprite_frameset(sprite: AnimationSprite): 'remove' | null {
    const framesetName = this.anim_data.resolve_frameset_name(
      sprite.object_id,
      sprite.override_frameset ?? null,
    );
    if (!framesetName) {
      throw new Error(`Missing battle animation frameset for ${sprite.object_id}.`);
    }
    const frames = this.anim_data.get_frameset_frames(framesetName);
    if (!frames || !frames.length) {
      throw new Error(`Battle animation frameset ${framesetName} has no frames.`);
    }
    if (sprite.active_frameset !== framesetName) {
      sprite.active_frameset = framesetName;
      sprite.frameset_index = -1;
      sprite.frame_duration = 0;
      sprite.frame = -1;
    }
    if (sprite.frame_duration > 0) {
      sprite.frame_duration -= 1;
      return null;
    }
    let guard = 0;
    while (guard < frames.length + 2) {
      guard += 1;
      sprite.frameset_index += 1;
      if (sprite.frameset_index >= frames.length) {
        throw new Error(`Frameset ${framesetName} overran without oamend/oamdelete.`);
      }
      const entry = frames[sprite.frameset_index];
      switch (entry.command) {
        case 'frame':
          sprite.frame = sprite.frameset_index;
          sprite.frame_duration = Math.max(0, Math.trunc(entry.duration ?? 0) - 1);
          return null;
        case 'wait':
          sprite.frame_duration = Math.max(0, Math.trunc(entry.duration ?? 0) - 1);
          return null;
        case 'delete':
          return 'remove';
        case 'restart':
          sprite.frameset_index = -1;
          continue;
        case 'end':
          sprite.frameset_index = Math.max(-1, sprite.frameset_index - 2);
          continue;
        default:
          continue;
      }
    }
    throw new Error(`Frameset ${framesetName} exceeded loop guard.`);
  }

  private resolve_sprite_position(sprite: AnimationSprite): [number, number] {
    const baseX = (Math.trunc(sprite.base_x ?? sprite.x) & 0xff) >>> 0;
    const baseY = (Math.trunc(sprite.base_y ?? sprite.y) & 0xff) >>> 0;
    const xOff = Math.trunc(sprite.x_offset);
    const yOff = Math.trunc(sprite.y_offset);

    if (sprite.relative_coords && !sprite.is_player_move) {
      const x = (mirror_enemy_x(baseX) + xOff) & 0xff;
      let yBase = sprite.fix_y === 0xff
        ? (baseY + this.tile_size * 5) & 0xff
        : (sprite.fix_y - baseY) & 0xff;
      if (sprite.fix_y !== 0xff && ENEMY_FIXY_ADJUST.has(this.animation_name)) {
        yBase = (yBase - this.tile_size) & 0xff;
      }
      return [x, (yBase + yOff) & 0xff];
    }

    return [(baseX + xOff) & 0xff, (baseY + yOff) & 0xff];
  }

  private apply_row_mode(): void {
    this.runtime.player_row_state = 0;
    this.runtime.enemy_row_state = 0;
    if (this.row_mode === 0) {
      this.runtime.player_row_mode = 0;
      this.runtime.enemy_row_mode = 0;
      return;
    }
    this.runtime.player_row_mode = this.row_mode;
    this.runtime.enemy_row_mode = this.row_mode;
  }
}

export const build_disassembly_reference_trace = (options: {
  animation_name: string;
  is_player_move: boolean;
  param?: number | null;
  param_label?: string | null;
  shake_count?: number | null;
  tile_size?: number;
}): BattleAnimationReferenceTrace => {
  const runner = new ScriptReferenceRunner(
    options.animation_name,
    options.is_player_move,
    options.param ?? 0,
    options.param_label ?? null,
    options.shake_count ?? null,
    options.tile_size ?? 8,
  );
  return runner.run();
};

export const compare_battle_animation_trace = (
  actual: BattleAnimationTrace,
  reference: BattleAnimationReferenceTrace,
): void => {
  if (actual.completed !== reference.completed) {
    throw new Error(
      `${actual.animation_name}: completion mismatch (actual ${actual.completed}, reference ${reference.completed}).`,
    );
  }
  if (actual.frames.length < reference.frames.length) {
    throw new Error(
      `${actual.animation_name}: live trace ended before reference script trace (actual ${actual.frames.length}, reference ${reference.frames.length}).`,
    );
  }

  for (let index = 0; index < reference.frames.length; index += 1) {
    const actualFrame = actual.frames[index];
    const referenceFrame = reference.frames[index];
    const prefix = `${actual.animation_name} frame ${index}`;
    const actualCommands = actualFrame.commands.map((command) => ({
      animation_name: command.animation_name,
      pointer: command.pointer,
      command: command.command,
      args: command.args,
      is_player_move: command.is_player_move,
    }));
    const referenceCommands = referenceFrame.commands.map((command) => ({
      animation_name: command.animation_name,
      pointer: command.pointer,
      command: command.command,
      args: command.args,
      is_player_move: command.is_player_move,
    }));
    if (JSON.stringify(actualCommands) !== JSON.stringify(referenceCommands)) {
      throw new Error(
        `${prefix}: command trace mismatch.\nactual=${JSON.stringify(actualCommands)}\nreference=${JSON.stringify(referenceCommands)}`,
      );
    }
    if (JSON.stringify(actualFrame.current_context) !== JSON.stringify(referenceFrame.current_context)) {
      throw new Error(
        `${prefix}: context mismatch.\nactual=${JSON.stringify(actualFrame.current_context)}\nreference=${JSON.stringify(referenceFrame.current_context)}`,
      );
    }
    if (actualFrame.battle_anim_delay !== referenceFrame.battle_anim_delay) {
      throw new Error(
        `${prefix}: delay mismatch (actual ${actualFrame.battle_anim_delay}, reference ${referenceFrame.battle_anim_delay}).`,
      );
    }
    if (actualFrame.anim_var !== referenceFrame.anim_var) {
      throw new Error(
        `${prefix}: anim_var mismatch (actual ${actualFrame.anim_var}, reference ${referenceFrame.anim_var}).`,
      );
    }
    if (actualFrame.anim_param !== referenceFrame.anim_param) {
      throw new Error(
        `${prefix}: anim_param mismatch (actual ${actualFrame.anim_param}, reference ${referenceFrame.anim_param}).`,
      );
    }
    if (JSON.stringify(actualFrame.loop_state) !== JSON.stringify(referenceFrame.loop_state)) {
      throw new Error(
        `${prefix}: loop state mismatch.\nactual=${JSON.stringify(actualFrame.loop_state)}\nreference=${JSON.stringify(referenceFrame.loop_state)}`,
      );
    }
    if (actualFrame.flags !== referenceFrame.flags) {
      throw new Error(
        `${prefix}: flags mismatch (actual ${actualFrame.flags}, reference ${referenceFrame.flags}).`,
      );
    }
    if (actualFrame.oam_enabled !== referenceFrame.oam_enabled) {
      throw new Error(
        `${prefix}: oam_enabled mismatch (actual ${actualFrame.oam_enabled}, reference ${referenceFrame.oam_enabled}).`,
      );
    }
    if (actualFrame.keep_sprites !== referenceFrame.keep_sprites) {
      throw new Error(
        `${prefix}: keep_sprites mismatch (actual ${actualFrame.keep_sprites}, reference ${referenceFrame.keep_sprites}).`,
      );
    }
    if (actualFrame.current_frame_tag !== referenceFrame.current_frame_tag) {
      throw new Error(
        `${prefix}: current_frame_tag mismatch (actual ${actualFrame.current_frame_tag}, reference ${referenceFrame.current_frame_tag}).`,
      );
    }
    if (JSON.stringify(actualFrame.palette_state) !== JSON.stringify(referenceFrame.palette_state)) {
      throw new Error(
        `${prefix}: palette state mismatch.\nactual=${JSON.stringify(actualFrame.palette_state)}\nreference=${JSON.stringify(referenceFrame.palette_state)}`,
      );
    }
    if (actualFrame.row_mode !== referenceFrame.row_mode) {
      throw new Error(
        `${prefix}: row_mode mismatch (actual ${actualFrame.row_mode}, reference ${referenceFrame.row_mode}).`,
      );
    }
    if (actualFrame.subroutine_depth !== referenceFrame.subroutine_depth) {
      throw new Error(
        `${prefix}: subroutine depth mismatch (actual ${actualFrame.subroutine_depth}, reference ${referenceFrame.subroutine_depth}).`,
      );
    }
    if (JSON.stringify(actualFrame.runtime) !== JSON.stringify(referenceFrame.runtime)) {
      throw new Error(
        `${prefix}: runtime snapshot mismatch.\nactual=${JSON.stringify(actualFrame.runtime)}\nreference=${JSON.stringify(referenceFrame.runtime)}`,
      );
    }
    if (JSON.stringify(actualFrame.bg_effects) !== JSON.stringify(referenceFrame.bg_effects)) {
      throw new Error(
        `${prefix}: bg effect trace mismatch.\nactual=${JSON.stringify(actualFrame.bg_effects)}\nreference=${JSON.stringify(referenceFrame.bg_effects)}`,
      );
    }
    if (JSON.stringify(actualFrame.sprites) !== JSON.stringify(referenceFrame.sprites)) {
      throw new Error(
        `${prefix}: sprite trace mismatch.\nactual=${JSON.stringify(actualFrame.sprites)}\nreference=${JSON.stringify(referenceFrame.sprites)}`,
      );
    }
    const actualSounds = actualFrame.sounds.map((sound) => ({
      sound_id: sound.sound_id,
      duration: sound.duration ?? null,
      tracks: sound.tracks ?? null,
      sound_type: sound.sound_type,
      cry_selector: sound.cry_selector ?? 0,
    }));
    const referenceSounds = referenceFrame.sounds.map((sound) => ({
      sound_id: sound.sound_id,
      duration: sound.duration ?? null,
      tracks: sound.tracks ?? null,
      sound_type: sound.sound_type,
      cry_selector: sound.cry_selector ?? 0,
    }));
    if (JSON.stringify(actualSounds) !== JSON.stringify(referenceSounds)) {
      throw new Error(
        `${prefix}: sound trace mismatch.\nactual=${JSON.stringify(actualSounds)}\nreference=${JSON.stringify(referenceSounds)}`,
      );
    }
  }
};

export const validate_battle_animation_trace = (trace: BattleAnimationTrace): void => {
  for (const frame of trace.frames) {
    for (const sprite of frame.sprites) {
      if (sprite.frame_entry && sprite.frame_entry.command !== 'frame') {
        throw new Error(
          `${trace.animation_name} frame ${frame.frame}: sprite ${sprite.object_id} resolved non-frame entry.`,
        );
      }
      if (sprite.frame_entry?.oam_set && !sprite.oam_set) {
        throw new Error(
          `${trace.animation_name} frame ${frame.frame}: sprite ${sprite.object_id} missing OAM set ${sprite.frame_entry.oam_set}.`,
        );
      }
      if (sprite.oam_set && !sprite.oam_set.entries.length) {
        throw new Error(
          `${trace.animation_name} frame ${frame.frame}: sprite ${sprite.object_id} has empty OAM set ${sprite.oam_set.name}.`,
        );
      }
    }
  }
};

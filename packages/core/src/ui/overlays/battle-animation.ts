import { gameEngine } from '../game-engine';
import { MoveName } from '@pokecrystal/core/core/enums';
import { load_animation_scripts } from './_battle-animation-loader';
import {
  Animation,
  AnimationCommand,
  AnimationContext,
  AnimationObjectTable,
  AnimationSound,
  AnimationSprite,
  AnimationSpriteSchema,
  BattleAnimationFlag,
  LoopState,
  parse_battle_int,
} from './_battle-animation-state';
import { compute_pokeball_anim_var, update_animation_sprite } from './_battle-anim-runtime';
import {
  BattlerSide,
  BattleAnimationRuntime,
  BattleBgEffect,
  ParsedBgEffect,
  buildParsedBgEffect,
  make_effect,
} from './_battle-bg-effects';
import { build_command_handlers, mirror_enemy_x } from './_battle-animation-helpers';
import { enqueue_sound } from './_battle-animation-sound';
import { animation_label_for_move, is_player_context, resolve_panning, tile_size_px } from './_battle-animation-util';
import { DMG_OBP0_ASSIGNED, assign_dmg_palettes, revert_to_dmg_defaults } from './_battle-palettes';
import { BattleAnimData, RenderedSprite } from './_battle-anim-data';
import logger from '@pokecrystal/core/core/logger';

// Mirrors pokecrystal_disassembly/engine/battle_anims/core.asm (QueueBattleAnimation/InitBattleAnimation).
const MAX_ANIM_OBJECTS = 10;
const MAX_COMMANDS_PER_FRAME = 1024;
// ASM parity: BattleAnim_Recover/Softboiled/MilkDrink place BATTLE_ANIM_OBJ_RECOVER with RELATIVE_X + fix_y=$90
// and rely on enemy-side Y mirroring behavior in engine/battle_anims/functions.asm::BattleAnimFunc_Recover.
const ENEMY_FIXY_ADJUST = new Set([
  'BattleAnim_Kinesis',
  'BattleAnim_Recover',
  'BattleAnim_Softboiled',
  'BattleAnim_MilkDrink',
]);
const browserBattleAnimWarnings = new Set<string>();

export type AnimationQueueEntry =
  | [Animation, number, boolean, string | null, number | null]
  | [Animation, number, boolean];

type TraceCommandRecord = {
  animation_name: string;
  pointer: number;
  command: string;
  args: string[];
  is_player_move: boolean;
};

export class AnimationPlayer {
  public readonly ui: { tile_size?: number };
  public animation_queue: AnimationQueueEntry[] = [];
  private current_context: AnimationContext | null = null;
  private subroutine_stack: AnimationContext[] = [];
  public anim_data = new BattleAnimData();
  public animations = load_animation_scripts();
  private battle_anim_delay = 0;
  public current_frame_tag: string | null = null;
  public pending_sounds: AnimationSound[] = [];
  private objects = new AnimationObjectTable(MAX_ANIM_OBJECTS);
  private bg_effect_sprites: Record<string, AnimationSprite> = {};
  public keep_sprites = false;
  public oam_enabled = true;
  public palette_state: Record<string, number | null> = { bgp: null, obp0: null, obp1: null };
  private anim_var = 0;
  private anim_param = 0;
  private anim_param_label: string | null = null;
  private target_shakes: number | null = null;
  private loop_state: LoopState | null = null;
  public runtime = new BattleAnimationRuntime();
  private bg_effects: Record<string, BattleBgEffect> = {};
  private flagsValue: number = 0;
  private row_mode = 0;
  private wait_commanded = false;
  private last_executed_commands: TraceCommandRecord[] = [];
  private command_handlers = build_command_handlers(
    this as unknown as Parameters<typeof build_command_handlers>[0],
  );
  private root_animation_name: string | null = null;
  private enemy_fixy_adjust_active = false;

  constructor(ui: { tile_size?: number }) {
    this.ui = ui;
  }

  private handle_check_pokeball(command: AnimationCommand): void {
    const sprite = this.active_sprites.length ? this.active_sprites[this.active_sprites.length - 1] : null;
    if (!sprite) {
      return;
    }
    let targetShakes = sprite.target_shakes;
    if (targetShakes === null || targetShakes === undefined) {
      targetShakes = this.target_shakes ?? this.anim_param;
    }
    this.anim_var = compute_pokeball_anim_var(sprite, targetShakes ?? null);
  }

  play_animation(
    move_name: MoveName | string,
    is_player_move: boolean,
    param: number | null = null,
    options?: { param_label?: string | null; shake_count?: number | null },
  ): void {
    const moveLabel = typeof move_name === 'string' ? move_name : String(move_name);
    const rawLabel = typeof move_name === 'string' ? move_name.trim() : String(move_name);
    const animName = this.animations.has(rawLabel)
      ? rawLabel
      : animation_label_for_move(move_name);
    const animation = this.animations.get(animName);
    if (!animation) {
      throw new Error(`Battle animation '${animName}' not found for '${moveLabel}'.`);
    }
    const normalizedParam = param === null || param === undefined ? 0 : Math.max(0, Math.trunc(param));
    const label = options?.param_label ? options.param_label.trim().toUpperCase() : null;
    const normalizedShakes = options?.shake_count === null || options?.shake_count === undefined
      ? null
      : Math.max(0, Math.trunc(options.shake_count));
    this.animation_queue.push([animation, normalizedParam, Boolean(is_player_move), label, normalizedShakes]);
  }

  update(): void {
    this.last_executed_commands = [];
    this.runtime.reset_transforms();
    this.apply_row_mode();

    if (this.battle_anim_delay > 0) {
      this.battle_anim_delay -= 1;
      this.update_objects();
      this.update_bg_effects();
      return;
    }

    if (!this.current_context) {
      this.start_next_animation();
      if (!this.current_context) {
        this.update_objects();
        this.update_bg_effects();
        return;
      }
    }

    this.execute_commands_for_frame();
    this.update_objects();
    this.update_bg_effects();
  }

  pending_audio(): AnimationSound[] {
    const sounds = [...this.pending_sounds];
    this.pending_sounds = [];
    return sounds;
  }

  get active_sprites(): AnimationSprite[] {
    return this.objects.sprites;
  }

  get current_animation_script(): Animation | null {
    return this.current_context ? this.current_context.animation : null;
  }

  is_active(): boolean {
    return (
      this.battle_anim_delay > 0 ||
      this.current_context !== null ||
      this.animation_queue.length > 0 ||
      Object.keys(this.bg_effects).length > 0
    );
  }

  get pending_wait_frames(): number {
    return this.battle_anim_delay;
  }

  get flags(): number {
    return this.flagsValue;
  }

  get subroutine_depth(): number {
    return this.subroutine_stack.length;
  }

  get runtime_state(): BattleAnimationRuntime {
    return this.runtime;
  }

  get trace_commands(): Array<TraceCommandRecord> {
    return this.last_executed_commands.map((command) => ({
      ...command,
      args: [...command.args],
    }));
  }

  get trace_state(): Record<string, unknown> {
    return {
      current_context: this.current_context
        ? {
            animation_name: this.current_context.animation.name,
            pointer: this.current_context.pointer,
            is_player_move: this.current_context.is_player_move,
          }
        : null,
      root_animation_name: this.root_animation_name,
      battle_anim_delay: this.battle_anim_delay,
      current_frame_tag: this.current_frame_tag,
      keep_sprites: this.keep_sprites,
      oam_enabled: this.oam_enabled,
      palette_state: { ...this.palette_state },
      anim_var: this.anim_var,
      anim_param: this.anim_param,
      anim_param_label: this.anim_param_label,
      target_shakes: this.target_shakes,
      loop_state: this.loop_state ? { ...this.loop_state } : null,
      flags: this.flagsValue,
      row_mode: this.row_mode,
      subroutine_depth: this.subroutine_stack.length,
      runtime: this.runtime.snapshot(),
      bg_effects: Object.entries(this.bg_effects).map(([name, effect]) =>
        effect.snapshot(name),
      ),
    };
  }

  private reset_object_table(resetSlots: boolean = true): void {
    this.objects.reset(resetSlots);
    this.bg_effect_sprites = {};
  }

  reset(): void {
    this.animation_queue = [];
    this.current_context = null;
    this.subroutine_stack = [];
    this.battle_anim_delay = 0;
    this.current_frame_tag = null;
    this.pending_sounds = [];
    this.reset_object_table();
    this.keep_sprites = false;
    this.oam_enabled = true;
    this.row_mode = 0;
    this.wait_commanded = false;
    this.palette_state = { bgp: null, obp0: null, obp1: null };
    this.anim_var = 0;
    this.anim_param = 0;
    this.anim_param_label = null;
    this.target_shakes = null;
    this.loop_state = null;
    this.bg_effects = {};
    this.runtime.player_visible = true;
    this.runtime.enemy_visible = true;
    this.runtime.reset_transforms();
    this.flagsValue = 0;
    this.sync_subroutine_flag();
    this.revert_palettes();
    this.root_animation_name = null;
    this.enemy_fixy_adjust_active = false;
  }

  private start_next_animation(): void {
    this.battle_anim_delay = 0;
    if (!this.animation_queue.length) {
      this.current_context = null;
      this.subroutine_stack = [];
      this.loop_state = null;
      this.anim_param_label = null;
      this.target_shakes = null;
      this.flagsValue |= BattleAnimationFlag.STOP;
      this.sync_subroutine_flag();
      if (!this.keep_sprites) {
        this.reset_object_table();
      } else {
        this.keep_sprites = false;
      }
      this.row_mode = 0;
      if (!Object.keys(this.bg_effects).length) {
        this.revert_palettes();
      }
      return;
    }

    const entry = this.unpack_queue_entry(this.animation_queue.shift() as AnimationQueueEntry);
    const [animation, param, isPlayerMove, paramLabel, targetShakes] = entry;
    if (!this.keep_sprites) {
      this.reset_object_table();
    }
    this.keep_sprites = false;
    this.row_mode = 0;
    this.current_context = new AnimationContext({
      animation,
      pointer: 0,
      is_player_move: isPlayerMove,
    });
    this.subroutine_stack = [];
    this.loop_state = null;
    this.anim_var = 0;
    this.anim_param = param;
    this.anim_param_label = paramLabel;
    this.target_shakes = targetShakes;
    this.root_animation_name = animation.name;
    this.enemy_fixy_adjust_active = ENEMY_FIXY_ADJUST.has(animation.name);
    this.flagsValue &= ~BattleAnimationFlag.STOP;
    this.sync_subroutine_flag();
    this.assign_palettes();
  }

  private unpack_queue_entry(entry: AnimationQueueEntry): [Animation, number, boolean, string | null, number | null] {
    if (entry.length === 5) {
      return entry as [Animation, number, boolean, string | null, number | null];
    }
    if (entry.length === 3) {
      const [animation, param, isPlayerMove] = entry as [Animation, number, boolean];
      return [animation, param, isPlayerMove, null, null];
    }
    throw new Error('Unsupported animation queue entry shape');
  }

  private execute_commands_for_frame(): void {
    // ASM mapping: pokecrystal_disassembly/engine/battle_anims/anim_commands.asm (RunBattleAnimCommand loop).
    let executed = 0;
    while (this.current_context) {
      if (executed >= MAX_COMMANDS_PER_FRAME) {
        throw new Error('Battle animation exceeded per-frame command budget.');
      }
      const context = this.current_context;
      if (context.pointer >= context.animation.script.length) {
        this.finish_current_context();
        if (!this.current_context) {
          return;
        }
        continue;
      }
      const command = context.animation.script[context.pointer];
      const pointer = context.pointer;
      context.pointer += 1;
      const normalized = command.command.toLowerCase();
      const handler = this.command_handlers[normalized];
      if (!handler) {
        throw new Error(
          `Unknown battle animation opcode '${command.command}' in ${context.animation.name}.`,
        );
      }
      this.last_executed_commands.push({
        animation_name: context.animation.name,
        pointer,
        command: command.command,
        args: [...command.args],
        is_player_move: context.is_player_move,
      });
      handler(command);
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

  private finish_current_context(): void {
    if (this.subroutine_stack.length) {
      this.current_context = this.subroutine_stack.pop() ?? null;
      this.sync_subroutine_flag();
      return;
    }
    this.current_context = null;
    this.loop_state = null;
    this.root_animation_name = null;
    this.enemy_fixy_adjust_active = false;
    this.flagsValue |= BattleAnimationFlag.STOP;
    this.sync_subroutine_flag();
    if (!this.keep_sprites) {
      this.reset_object_table();
    } else {
      this.keep_sprites = false;
    }
    this.row_mode = 0;
    if (!Object.keys(this.bg_effects).length) {
      this.revert_palettes();
    }
  }

  private sync_subroutine_flag(): void {
    if (this.subroutine_stack.length) {
      this.flagsValue |= BattleAnimationFlag.IN_SUBROUTINE;
    } else {
      this.flagsValue &= ~BattleAnimationFlag.IN_SUBROUTINE;
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
    const index = animation.labels[label] ?? 0;
    return [animation, index];
  }

  private handle_wait(command: AnimationCommand): void {
    const value = command.args.length ? parse_battle_int(command.args[0]) : 0;
    this.set_battle_anim_delay(value ?? 0);
    this.wait_commanded = true;
  }

  private handle_frame(command: AnimationCommand): void {
    if (command.args.length) {
      this.current_frame_tag = command.args[0];
    }
  }

  private handle_sound(command: AnimationCommand): void {
    // ASM: engine/battle_anims/anim_commands.asm:BattleAnimCmd_Sound packs (duration<<2)|tracks.
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
    if (sound_id) {
      const resolvedTracks = tracks === null ? null : Math.max(0, Math.trunc(tracks) & 0x03);
      const resolvedDuration = duration === null ? null : Math.max(0, Math.trunc(duration));
      enqueue_sound(this.pending_sounds, sound_id, resolvedDuration, resolvedTracks, {
        sound_type: 'sound',
        pitch: null,
        panning: resolve_panning(resolvedTracks, is_player_context(this.current_context)),
      });
    }
  }

  private handle_play_sound(command: AnimationCommand): void {
    if (command.args.length) {
      enqueue_sound(this.pending_sounds, command.args[0], null, null, {
        sound_type: 'sound',
        pitch: null,
        panning: resolve_panning(null, is_player_context(this.current_context)),
      });
    }
  }

  private handle_cry(command: AnimationCommand): void {
    // ASM: battle_anims/anim_commands.asm::BattleAnimCmd_Cry.
    // The argument selects one of .CryData entries (pitch/length tweak),
    // not a playback channel mask.
    const crySelector = command.args.length ? parse_battle_int(command.args[0]) : 0;
    const resolvedCrySelector = crySelector === null ? 0 : Math.max(0, Math.trunc(crySelector)) & 0x03;
    enqueue_sound(this.pending_sounds, 'cry', null, null, {
      sound_type: 'cry',
      tracks: null,
      // Preserve the ASM selector for future parity checks (currently no runtime
      // sample table remapping is applied in the TS renderer yet).
      cry_selector: resolvedCrySelector,
      pitch: null,
      panning: is_player_context(this.current_context) ? 'player' : 'enemy',
    });
  }

  private handle_obj(command: AnimationCommand): void {
    if (command.args.length < 4) {
      throw new Error('anim_obj requires at least four arguments');
    }
    const objectId = command.args[0];
    const objDef = this.anim_data.object_defs.get(objectId.toUpperCase());
    const flags = objDef ? objDef.flags : 0;
    const fixY = objDef ? objDef.fix_y : 0;
    const relativeCoords = Boolean(flags & 0x01);
    const isPlayerMove = is_player_context(this.current_context);
    const applyObjFlags = !isPlayerMove;
    const mirrorXFlag = applyObjFlags ? Boolean(flags & 0x20) : false;
    const mirrorYFlag = applyObjFlags ? Boolean(flags & 0x40) : false;

    let x = 0;
    let y = 0;
    let paramIndex = 3;
    if (command.args.length >= 6) {
      const tileSize = tile_size_px(this.ui);
      x = tileSize * (parse_battle_int(command.args[1]) ?? 0) + (parse_battle_int(command.args[2]) ?? 0);
      y = tileSize * (parse_battle_int(command.args[3]) ?? 0) + (parse_battle_int(command.args[4]) ?? 0);
      paramIndex = 5;
    } else {
      x = parse_battle_int(command.args[1]) ?? 0;
      y = parse_battle_int(command.args[2]) ?? 0;
      paramIndex = 3;
    }
    const param = parse_battle_int(command.args[paramIndex]) ?? 0;
    const sprite = AnimationSpriteSchema.parse({
      object_id: objectId,
      function_id: objDef ? objDef.function : null,
      x,
      y,
      fix_y: fixY,
      override_frameset: objDef ? objDef.frameset : null,
      param,
      mirror_x: mirrorXFlag,
      mirror_y: mirrorYFlag,
      relative_coords: relativeCoords,
      is_player_move: isPlayerMove,
      jump_index: 0,
      param_label: this.anim_param_label,
      target_shakes: this.target_shakes,
    });
    const added = this.objects.add(sprite);
    if (!added) {
      // ASM anim_obj does not branch on QueueBattleAnimation carry; exhaustion is a silent skip.
      return;
    }
  }

  private handle_clear_objs(_command: AnimationCommand): void {
    this.reset_object_table();
  }

  private handle_inc_obj(command: AnimationCommand): void {
    if (!command.args.length) {
      throw new Error('anim_incobj requires an object index');
    }
    this.objects.increment(command.args[0]);
  }

  private handle_set_obj(command: AnimationCommand): void {
    if (command.args.length < 2) {
      throw new Error('anim_setobj requires an index and state value');
    }
    const target = command.args[0];
    const state = parse_battle_int(command.args[1]) ?? 0;
    this.objects.set_jump_index(target, state);
  }

  public handle_gfx(command: AnimationCommand): void {
    if (command.args.length) {
      this.current_frame_tag = command.args[0];
    }
  }

  private handle_bgp(command: AnimationCommand): void {
    if (command.args.length) {
      this.palette_state.bgp = parse_battle_int(command.args[0]);
    }
  }

  private handle_obp0(command: AnimationCommand): void {
    if (command.args.length) {
      this.palette_state.obp0 = parse_battle_int(command.args[0]);
    }
  }

  private handle_obp1(command: AnimationCommand): void {
    if (command.args.length) {
      this.palette_state.obp1 = parse_battle_int(command.args[0]);
    }
  }

  private handle_reset_obp0(_command: AnimationCommand): void {
    this.palette_state.obp0 = DMG_OBP0_ASSIGNED;
  }

  private handle_keep_sprites(_command: AnimationCommand): void {
    this.keep_sprites = true;
  }

  private handle_battlergfx_1row(_command: AnimationCommand): void {
    // ASM: anim_battlergfx_* only loads battler gfx; visibility/row effects are handled by BG effects.
    return;
  }

  private handle_battlergfx_2row(_command: AnimationCommand): void {
    // ASM: anim_battlergfx_* only loads battler gfx; visibility/row effects are handled by BG effects.
    return;
  }

  private handle_oam_on(_command: AnimationCommand): void {
    this.oam_enabled = true;
  }

  private handle_oam_off(_command: AnimationCommand): void {
    this.oam_enabled = false;
  }

  private handle_ret(_command: AnimationCommand): void {
    this.finish_current_context();
  }

  private handle_call(command: AnimationCommand): void {
    if (!command.args.length) {
      return;
    }
    const target = this.resolve_label(command.args[0]);
    if (!target) {
      return;
    }
    const [animation, pointer] = target;
    const current = this.current_context;
    if (current) {
      this.subroutine_stack.push(current.clone());
    }
    // ASM: anim_call preserves hBattleTurn; keep the same side for subroutines.
    const inheritedTurn = current ? current.is_player_move : true;
    this.current_context = new AnimationContext({ animation, pointer, is_player_move: inheritedTurn });
    this.sync_subroutine_flag();
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
    // ASM: anim_jump preserves hBattleTurn; keep the same side across jumps.
    const inheritedTurn = this.current_context ? this.current_context.is_player_move : true;
    this.current_context = new AnimationContext({ animation, pointer, is_player_move: inheritedTurn });
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

  private handle_set_var(command: AnimationCommand): void {
    if (command.args.length) {
      this.anim_var = Math.max(0, parse_battle_int(command.args[0]) ?? 0);
    }
  }

  private handle_inc_var(_command: AnimationCommand): void {
    this.anim_var = (this.anim_var + 1) & 0xff;
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
    if (parsed !== null && parsed !== undefined) {
      if (parsed === this.anim_param) {
        this.jump_within_context(command.args[1]);
      }
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

  public handle_noop(_command: AnimationCommand): void {
    return;
  }

  private handle_transform(_command: AnimationCommand): void {
    if (is_player_context(this.current_context)) {
      this.runtime.player_sprite_override = 'transform';
    } else {
      this.runtime.enemy_sprite_override = 'transform';
    }
  }

  private handle_raise_sub(_command: AnimationCommand): void {
    if (is_player_context(this.current_context)) {
      this.runtime.player_sprite_override = 'substitute';
    } else {
      this.runtime.enemy_sprite_override = 'substitute';
    }
  }

  private handle_drop_sub(_command: AnimationCommand): void {
    if (is_player_context(this.current_context)) {
      this.runtime.player_sprite_override = null;
    } else {
      this.runtime.enemy_sprite_override = null;
    }
  }

  private handle_minimize(_command: AnimationCommand): void {
    if (is_player_context(this.current_context)) {
      this.runtime.player_sprite_override = 'minimize';
    } else {
      this.runtime.enemy_sprite_override = 'minimize';
    }
  }

  private handle_minimize_opp(_command: AnimationCommand): void {
    if (is_player_context(this.current_context)) {
      this.runtime.enemy_sprite_override = 'minimize';
    } else {
      this.runtime.player_sprite_override = 'minimize';
    }
  }

  private handle_update_actor_pic(_command: AnimationCommand): void {
    if (is_player_context(this.current_context)) {
      this.runtime.player_sprite_override = null;
    } else {
      this.runtime.enemy_sprite_override = null;
    }
  }

  private handle_bgeffect(command: AnimationCommand): void {
    const parsed = this.parse_bg_effect(command);
    const effect = make_effect(parsed);
    this.bg_effects[parsed.name] = effect;
    this.maybe_spawn_battler_object(parsed);
  }

  private handle_incbgeffect(command: AnimationCommand): void {
    if (!command.args.length) {
      return;
    }
    const effect = this.bg_effects[command.args[0]];
    if (!effect) {
      return;
    }
    effect.increment();
  }

  private parse_bg_effect(command: AnimationCommand): ParsedBgEffect {
    if (command.args.length < 4) {
      throw new Error('anim_bgeffect requires four arguments');
    }
    const effectName = command.args[0];
    const duration = parse_battle_int(command.args[1]) ?? 0;
    const rawTurn = command.args[2];
    const param = parse_battle_int(command.args[3]) ?? 0;
    const context = this.current_context;
    const isPlayerMove = context ? context.is_player_move : true;
    const turnValue = parse_battle_int(rawTurn);
    return buildParsedBgEffect({
      name: effectName,
      duration: Math.max(0, duration),
      raw_turn: rawTurn,
      param: Math.max(0, param),
      is_player_move: isPlayerMove,
      turn_value: turnValue ?? null,
    });
  }

  private maybe_spawn_battler_object(effect: ParsedBgEffect): void {
    if (!['BATTLE_BG_EFFECT_BATTLEROBJ_1ROW', 'BATTLE_BG_EFFECT_BATTLEROBJ_2ROW'].includes(effect.name)) {
      return;
    }
    this.remove_battler_object_sprite(effect.name);
    const tileSize = tile_size_px(this.ui);
    const side = effect.target_side();
    const playerId = effect.name.endsWith('1ROW')
      ? 'BATTLE_ANIM_OBJ_PLAYERHEAD_1ROW'
      : 'BATTLE_ANIM_OBJ_PLAYERHEAD_2ROW';
    const enemyId = effect.name.endsWith('1ROW')
      ? 'BATTLE_ANIM_OBJ_ENEMYFEET_1ROW'
      : 'BATTLE_ANIM_OBJ_ENEMYFEET_2ROW';
    const objectId = side === BattlerSide.PLAYER ? playerId : enemyId;
    const objDef = this.anim_data.object_defs.get(objectId);
    let x = tileSize * 6;
    if (side === BattlerSide.ENEMY) {
      x = tileSize * 16 + 4;
    }
    const isPlayerMove = side === BattlerSide.PLAYER;
    const applyObjFlags = !isPlayerMove;
    const sprite = AnimationSpriteSchema.parse({
      object_id: objectId,
      x,
      y: tileSize * 8,
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
    if (!added) {
      // ASM bg-effect battlerobj queueing also keeps executing after exhaustion.
      return;
    }
    this.bg_effect_sprites[effect.name] = added;
  }

  private remove_battler_object_sprite(effectName: string): void {
    const sprite = this.bg_effect_sprites[effectName];
    if (sprite) {
      delete this.bg_effect_sprites[effectName];
      this.objects.remove(sprite);
    }
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

  private assign_palettes(): void {
    assign_dmg_palettes(this.palette_state);
  }

  private revert_palettes(): void {
    revert_to_dmg_defaults(this.palette_state);
  }

  render_sprite(sprite: AnimationSprite): RenderedSprite | null {
    const renderOptions = {
      frameset_override: sprite.override_frameset ?? null,
      palette_override: sprite.palette_override ?? null,
      extra_xflip: sprite.mirror_x,
      extra_yflip: sprite.mirror_y,
    };
    let rendered: RenderedSprite | null;
    try {
      rendered = this.anim_data.render_sprite(sprite.object_id, sprite.frame, renderOptions);
    } catch (error) {
      if (
        typeof window !== 'undefined' &&
        error instanceof Error &&
        error.message.startsWith('Missing battle animation tile ')
      ) {
        if (!browserBattleAnimWarnings.has(error.message)) {
          browserBattleAnimWarnings.add(error.message);
          logger.warn('[battle-animation] Skipping battle animation sprite after asset load failure', {
            object_id: sprite.object_id,
            frame: sprite.frame,
            error: error.message,
          });
        }
        return null;
      }
      throw error;
    }
    if (!rendered) {
      if (typeof window !== 'undefined') {
        return null;
      }
      throw new Error(`Missing rendered battle animation sprite for ${sprite.object_id} frame ${sprite.frame}.`);
    }
    return rendered;
  }

  resolve_sprite_position(sprite: AnimationSprite): [number, number] {
    let baseX = sprite.base_x ?? sprite.x;
    let baseY = sprite.base_y ?? sprite.y;
    const xOffset = sprite.x_offset;
    const yOffset = sprite.y_offset;
    baseX = (Math.trunc(baseX) & 0xff) >>> 0;
    baseY = (Math.trunc(baseY) & 0xff) >>> 0;
    const xOff = Math.trunc(xOffset);
    const yOff = Math.trunc(yOffset);

    if (sprite.relative_coords && !sprite.is_player_move) {
      const tileSize = tile_size_px(this.ui);
      const x = (mirror_enemy_x(baseX) + xOff) & 0xff;
      let yBase: number;
      if (sprite.fix_y === 0xff) {
        yBase = (baseY + tileSize * 5) & 0xff;
      } else {
        yBase = (sprite.fix_y - baseY) & 0xff;
        if (this.enemy_fixy_adjust_active) {
          yBase = (yBase - tileSize) & 0xff;
        }
      }
      const y = (yBase + yOff) & 0xff;
      return [x, y];
    }

    const x = (baseX + xOff) & 0xff;
    const y = (baseY + yOff) & 0xff;
    return [x, y];
  }

  private update_bg_effects(): void {
    const effectNames = Object.keys(this.bg_effects);
    if (!effectNames.length) {
      return;
    }
    const expired: string[] = [];
    for (const name of effectNames) {
      const effect = this.bg_effects[name];
      const active = effect.update(this.runtime);
      if (!active) {
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
    if (!this.active_sprites.length) {
      return;
    }
    const removed: AnimationSprite[] = [];
    for (const sprite of [...this.active_sprites]) {
      const result = update_animation_sprite(sprite, this.runtime);
      if (result === 'remove') {
        removed.push(sprite);
        continue;
      }
      const framesetResult = this.advance_sprite_frameset(sprite);
      if (framesetResult === 'remove') {
        removed.push(sprite);
      }
    }
    for (const sprite of removed) {
      this.objects.remove(sprite);
    }
  }

  private advance_sprite_frameset(sprite: AnimationSprite): 'remove' | null {
    // ASM mapping: engine/battle_anims/helpers.asm::GetBattleAnimFrame + core.asm::BattleAnimOAMUpdate.
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
          sprite.frame_duration = this.asm_frameset_duration(entry.duration);
          return null;
        case 'wait':
          sprite.frame_duration = this.asm_frameset_duration(entry.duration);
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

  // ASM parity: GetBattleAnimFrame initializes BATTLEANIMSTRUCT_DURATION with the oam duration byte
  // then decrements it each frame before deciding to fetch the next oam command. That means a value
  // of 1 should hold for exactly one update tick, 0 should advance immediately.
  private asm_frameset_duration(duration: number): number {
    return Math.max(0, Math.trunc(duration ?? 0) - 1);
  }

  private set_battle_anim_delay(frames: number): void {
    this.battle_anim_delay = Math.max(0, Math.trunc(frames ?? 0));
  }
}

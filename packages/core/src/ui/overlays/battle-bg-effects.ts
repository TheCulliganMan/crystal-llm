import { sine } from './battle-anim-math';

export enum BattlerSide {
  PLAYER = 'player',
  ENEMY = 'enemy',
}

// ASM mapping: BattleBGEffect_EnterMon/ReturnMon (engine/battle_anims/bg_effects.asm) resize
// the battler pic using BGSquares indices; map those indices to square sizes in tiles here.
const PIC_RESIZE_ROW_COUNTS: Record<number, number> = {
  0: 6, // SixBySix
  1: 4, // FourByFour
  2: 2, // TwoByTwo
  3: 7, // SevenBySeven
  4: 5, // FiveByFive
  5: 3, // ThreeByThree
};

const build_pic_resize_rows = (indices: number[], { hold_last = false }: { hold_last?: boolean } = {}): number[] => {
  const rows = indices.map((index) => {
    const value = PIC_RESIZE_ROW_COUNTS[index];
    if (value === undefined) {
      throw new Error(`Unknown pic resize index ${index}.`);
    }
    return value;
  });
  if (hold_last && rows.length) {
    rows.push(rows[rows.length - 1]);
  }
  return rows;
};

export class BattleAnimationRuntime {
  screen_offset_x = 0;
  screen_offset_y = 0;
  player_offset_x = 0;
  player_offset_y = 0;
  enemy_offset_x = 0;
  enemy_offset_y = 0;
  overlay_alpha = 0;
  overlay_colour: [number, number, number] | null = null;
  player_visible = true;
  enemy_visible = true;
  player_row_mode = 0;
  enemy_row_mode = 0;
  player_row_state = 0;
  enemy_row_state = 0;
  lcd_pointer: 'scy' | null = null;
  ly_override_start = 0;
  ly_override_end = 0;
  line_scroll_y: number[] = Array.from({ length: 0x5f }, () => 0);
  player_sprite_override: string | null = null;
  enemy_sprite_override: string | null = null;
  player_sprite_type_override: string | null = null;
  enemy_sprite_type_override: string | null = null;
  bgp: number | null = null;
  obp0: number | null = null;
  obp1: number | null = null;
  overlay_target: BattlerSide | null = null;

  reset_transforms(): void {
    this.screen_offset_x = 0;
    this.screen_offset_y = 0;
    this.player_offset_x = 0;
    this.player_offset_y = 0;
    this.enemy_offset_x = 0;
    this.enemy_offset_y = 0;
    this.player_row_mode = 0;
    this.enemy_row_mode = 0;
    this.player_row_state = 0;
    this.enemy_row_state = 0;
    this.overlay_alpha = 0;
    this.overlay_colour = null;
    this.bgp = null;
    this.obp0 = null;
    this.obp1 = null;
    this.lcd_pointer = null;
    this.ly_override_start = 0;
    this.ly_override_end = 0;
    this.line_scroll_y.fill(0);
    this.player_sprite_override = null;
    this.enemy_sprite_override = null;
    this.player_sprite_type_override = null;
    this.enemy_sprite_type_override = null;
    this.overlay_target = null;
  }

  set_visibility(side: BattlerSide, visible: boolean): void {
    if (side === BattlerSide.PLAYER) {
      this.player_visible = visible;
    } else {
      this.enemy_visible = visible;
    }
  }

  apply_side_offset(side: BattlerSide, axis: 'x' | 'y', amount: number): void {
    if (axis === 'x') {
      if (side === BattlerSide.PLAYER) {
        this.player_offset_x += amount;
      } else {
        this.enemy_offset_x += amount;
      }
    } else if (side === BattlerSide.PLAYER) {
      this.player_offset_y += amount;
    } else {
      this.enemy_offset_y += amount;
    }
  }

  snapshot(): Record<string, unknown> {
    return {
      screen_offset_x: this.screen_offset_x,
      screen_offset_y: this.screen_offset_y,
      player_offset_x: this.player_offset_x,
      player_offset_y: this.player_offset_y,
      enemy_offset_x: this.enemy_offset_x,
      enemy_offset_y: this.enemy_offset_y,
      overlay_alpha: this.overlay_alpha,
      overlay_colour: this.overlay_colour ? [...this.overlay_colour] : null,
      player_visible: this.player_visible,
      enemy_visible: this.enemy_visible,
      player_row_mode: this.player_row_mode,
      enemy_row_mode: this.enemy_row_mode,
      player_row_state: this.player_row_state,
      enemy_row_state: this.enemy_row_state,
      lcd_pointer: this.lcd_pointer,
      ly_override_start: this.ly_override_start,
      ly_override_end: this.ly_override_end,
      line_scroll_y: [...this.line_scroll_y],
      player_sprite_override: this.player_sprite_override,
      enemy_sprite_override: this.enemy_sprite_override,
      player_sprite_type_override: this.player_sprite_type_override,
      enemy_sprite_type_override: this.enemy_sprite_type_override,
      bgp: this.bgp,
      obp0: this.obp0,
      obp1: this.obp1,
      overlay_target: this.overlay_target,
    };
  }
}

export interface ParsedBgEffect {
  name: string;
  duration: number;
  raw_turn: string;
  param: number;
  is_player_move: boolean;
  turn_value: number | null;
  target_side: (defaultSide?: BattlerSide) => BattlerSide;
}

export const buildParsedBgEffect = (data: {
  name: string;
  duration: number;
  raw_turn: string;
  param: number;
  is_player_move: boolean;
  turn_value: number | null;
}): ParsedBgEffect => ({
  ...data,
  target_side(defaultSide: BattlerSide = BattlerSide.ENEMY): BattlerSide {
    const token = data.raw_turn.toUpperCase();
    if (token === 'BG_EFFECT_USER') {
      return data.is_player_move ? BattlerSide.PLAYER : BattlerSide.ENEMY;
    }
    if (token === 'BG_EFFECT_TARGET') {
      return data.is_player_move ? BattlerSide.ENEMY : BattlerSide.PLAYER;
    }
    return defaultSide;
  },
});

export class BattleBgEffect {
  protected readonly effect: ParsedBgEffect;
  protected remaining = 0;
  private loopDuration: number | null = null;

  constructor(effect: ParsedBgEffect) {
    this.effect = effect;
  }

  update(_runtime: BattleAnimationRuntime): boolean {
    throw new Error(`${this.constructor.name} must implement update().`);
  }

  increment(): void {
    if (this.loopDuration === null) {
      return;
    }
    this.remaining = this.loopDuration;
  }

  protected setLoopDuration(duration: number): void {
    this.loopDuration = Math.max(1, Math.trunc(duration));
  }

  snapshot(name: string): Record<string, unknown> {
    return {
      name,
      class_name: this.constructor.name,
      remaining: this.remaining,
      loop_duration: this.loopDuration,
      effect: {
        name: this.effect.name,
        duration: this.effect.duration,
        raw_turn: this.effect.raw_turn,
        param: this.effect.param,
        is_player_move: this.effect.is_player_move,
        turn_value: this.effect.turn_value,
      },
    };
  }
}

export class ScreenShakeEffect extends BattleBgEffect {
  private axis: 'x' | 'y';
  private amplitude: number;
  private frequency: number;
  private frame = 0;

  constructor(effect: ParsedBgEffect, options: { axis: 'x' | 'y' }) {
    super(effect);
    this.axis = options.axis;
    this.remaining = Math.max(1, effect.duration || 1);
    let amplitude = effect.turn_value ?? 0;
    if (amplitude <= 0) {
      amplitude = Math.max(1, (effect.param >> 4) || 4);
    }
    const frequency = effect.param & 0x0f;
    this.amplitude = amplitude;
    this.frequency = Math.max(1, frequency || 2);
    this.setLoopDuration(this.remaining);
  }

  update(runtime: BattleAnimationRuntime): boolean {
    if (this.remaining <= 0) {
      return false;
    }
    const phase = Math.floor(this.frame / this.frequency) % 2;
    const offset = phase === 0 ? this.amplitude : -this.amplitude;
    if (this.axis === 'x') {
      runtime.screen_offset_x += offset;
    } else {
      runtime.screen_offset_y += offset;
    }
    this.frame += 1;
    this.remaining -= 1;
    return this.remaining > 0;
  }
}

export class LungeEffect extends BattleBgEffect {
  private axis: 'x' | 'y';
  private direction: number;
  private speed = 2;
  private distance = 0;
  private max_distance = 8;
  private returning = false;

  constructor(effect: ParsedBgEffect, options: { axis?: 'x' | 'y' } = {}) {
    super(effect);
    this.axis = options.axis ?? 'x';
    const target = effect.target_side(BattlerSide.PLAYER);
    this.direction = target === BattlerSide.PLAYER ? 1 : -1;
  }

  update(runtime: BattleAnimationRuntime): boolean {
    const side =
      this.effect.target_side(BattlerSide.PLAYER) === BattlerSide.PLAYER
        ? BattlerSide.PLAYER
        : BattlerSide.ENEMY;
    const offset = this.advance();
    runtime.apply_side_offset(side, this.axis, offset);
    return !(this.returning && this.distance === 0);
  }

  private advance(): number {
    if (!this.returning) {
      this.distance += this.speed;
      if (this.distance >= this.max_distance) {
        this.distance = this.max_distance;
        this.returning = true;
      }
    } else {
      this.distance = Math.max(0, this.distance - this.speed);
    }
    return this.distance * this.direction;
  }
}

export class BounceDownEffect extends BattleBgEffect {
  private side: BattlerSide;
  private speed = 2;
  private distance = 0;
  private max_distance = 8;
  private returning = false;
  private endRequested = false;

  constructor(effect: ParsedBgEffect) {
    super(effect);
    this.side = effect.target_side(BattlerSide.PLAYER);
  }

  update(runtime: BattleAnimationRuntime): boolean {
    const offset = this.advance();
    runtime.apply_side_offset(this.side, 'y', offset);
    if (this.endRequested && this.returning && this.distance === 0) {
      return false;
    }
    return true;
  }

  increment(): void {
    this.endRequested = true;
  }

  private advance(): number {
    if (!this.returning) {
      this.distance += this.speed;
      if (this.distance >= this.max_distance) {
        this.distance = this.max_distance;
        this.returning = true;
      }
    } else {
      this.distance = Math.max(0, this.distance - this.speed);
      if (this.distance === 0 && !this.endRequested) {
        this.returning = false;
      }
    }
    return this.distance;
  }
}

export class FlashEffect extends BattleBgEffect {
  private colour: [number, number, number];
  private frequency: number;
  private alpha: number;
  private frame = 0;
  private target_side: BattlerSide | null;

  constructor(
    effect: ParsedBgEffect,
    options: { colour: [number, number, number]; alpha?: number; target_side?: BattlerSide | null },
  ) {
    super(effect);
    this.colour = options.colour;
    this.remaining = Math.max(1, effect.duration || 4);
    this.frequency = Math.max(1, effect.param || 2);
    const rawAlpha = options.alpha ?? 96;
    this.alpha = Math.max(0, Math.min(255, rawAlpha));
    this.target_side = options.target_side ?? null;
    this.setLoopDuration(this.remaining);
  }

  update(runtime: BattleAnimationRuntime): boolean {
    if (this.remaining <= 0) {
      return false;
    }
    if (Math.floor(this.frame / this.frequency) % 2 === 0) {
      runtime.overlay_colour = this.colour;
      runtime.overlay_alpha = this.alpha;
      runtime.overlay_target = this.target_side;
    }
    this.frame += 1;
    this.remaining -= 1;
    return this.remaining > 0;
  }
}

export class FadeMonToWhiteWaitFadeBackEffect extends BattleBgEffect {
  private static readonly PALETTES = [
    0xe4, 0x90, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x90,
    0xe4,
  ];
  private static readonly WHITE: [number, number, number] = [255, 255, 255];
  private static readonly BASE_AVG = (() => {
    const palette = 0xe4;
    let total = 0;
    for (let index = 0; index < 4; index += 1) {
      total += (palette >> (index * 2)) & 0x03;
    }
    return total / 4;
  })();

  private side: BattlerSide;
  private alphas: number[];
  private stepDelay: number;
  private framesLeft: number;
  private index = 0;

  constructor(effect: ParsedBgEffect) {
    super(effect);
    this.side = effect.target_side();
    this.alphas = FadeMonToWhiteWaitFadeBackEffect.PALETTES.map((palette) =>
      FadeMonToWhiteWaitFadeBackEffect.alphaFromPalette(palette),
    );
    this.stepDelay = Math.max(1, effect.param >> 4);
    const initialDelay = effect.param & 0x0f;
    this.framesLeft = Math.max(1, initialDelay || this.stepDelay);
  }

  private static alphaFromPalette(palette: number): number {
    let avg = 0;
    for (let index = 0; index < 4; index += 1) {
      avg += (palette >> (index * 2)) & 0x03;
    }
    avg /= 4;
    if (FadeMonToWhiteWaitFadeBackEffect.BASE_AVG <= 0) {
      return 0;
    }
    const whiteness = (FadeMonToWhiteWaitFadeBackEffect.BASE_AVG - avg) / FadeMonToWhiteWaitFadeBackEffect.BASE_AVG;
    if (whiteness <= 0) {
      return 0;
    }
    if (whiteness >= 1) {
      return 255;
    }
    return Math.round(whiteness * 255);
  }

  increment(): void {
    this.index = this.alphas.length;
  }

  update(runtime: BattleAnimationRuntime): boolean {
    if (this.index >= this.alphas.length) {
      return false;
    }
    const alpha = this.alphas[this.index];
    if (alpha > 0) {
      runtime.overlay_colour = FadeMonToWhiteWaitFadeBackEffect.WHITE;
      runtime.overlay_alpha = alpha;
      runtime.overlay_target = this.side;
    }
    this.framesLeft -= 1;
    if (this.framesLeft <= 0) {
      this.index += 1;
      this.framesLeft = this.stepDelay;
    }
    return this.index < this.alphas.length;
  }
}

export class VisibilityEffect extends BattleBgEffect {
  private visible: boolean;

  constructor(effect: ParsedBgEffect, options: { visible: boolean }) {
    super(effect);
    this.visible = options.visible;
  }

  update(runtime: BattleAnimationRuntime): boolean {
    runtime.set_visibility(this.effect.target_side(), this.visible);
    return false;
  }
}

export class FaintEffect extends BattleBgEffect {
  private side: BattlerSide;
  private totalFrames: number;
  private step: number;
  private frequency = 2;
  private maxSteps: number;
  private maxOffset: number;

  constructor(effect: ParsedBgEffect) {
    super(effect);
    this.side = effect.target_side(BattlerSide.PLAYER);
    this.totalFrames = Math.max(1, effect.duration || 14);
    this.remaining = this.totalFrames;
    this.step = Math.max(1, effect.param || 4);
    this.maxSteps = Math.max(1, Math.floor((this.totalFrames + this.frequency - 1) / this.frequency));
    this.maxOffset = this.step * this.maxSteps;
    this.setLoopDuration(this.remaining);
  }

  update(runtime: BattleAnimationRuntime): boolean {
    if (this.remaining <= 0) {
      runtime.set_visibility(this.side, false);
      return false;
    }
    const progress = this.totalFrames - this.remaining;
    const stepIndex = Math.min(this.maxSteps, Math.floor((progress + 1) / this.frequency));
    const offset = Math.min(this.maxOffset, stepIndex * this.step);
    runtime.apply_side_offset(this.side, 'y', offset);
    this.remaining -= 1;
    if (this.remaining <= 0) {
      runtime.set_visibility(this.side, false);
      return false;
    }
    return true;
  }
}

export class SlidePicEffect extends BattleBgEffect {
  private side: BattlerSide;
  private hide_at_end: boolean;
  private duration: number;
  private start_offset: number;
  private direction: 'in' | 'out';
  private sign: number;

  constructor(effect: ParsedBgEffect, options: { direction: 'in' | 'out'; hide_at_end: boolean }) {
    super(effect);
    this.side = effect.target_side(BattlerSide.PLAYER);
    this.hide_at_end = options.hide_at_end;
    this.duration = Math.max(1, effect.duration || 6);
    this.remaining = this.duration;
    const start = effect.param > 0 ? effect.param : 80;
    this.start_offset = start;
    this.direction = options.direction;
    this.sign = this.side === BattlerSide.PLAYER ? 1 : -1;
    this.setLoopDuration(this.duration);
  }

  update(runtime: BattleAnimationRuntime): boolean {
    const progress = this.duration - this.remaining;
    let offset = 0;
    if (this.direction === 'in') {
      const numerator = Math.max(0, this.remaining - 1);
      offset = Math.round(this.start_offset * (numerator / this.duration));
    } else {
      offset = Math.round(this.start_offset * (progress / this.duration));
    }
    runtime.apply_side_offset(this.side, 'x', offset * this.sign);
    runtime.set_visibility(this.side, true);
    this.remaining -= 1;
    if (this.remaining > 0) {
      return true;
    }
    runtime.apply_side_offset(this.side, 'x', 0);
    if (this.hide_at_end) {
      runtime.set_visibility(this.side, false);
    }
    return false;
  }
}

export class PaletteCycleEffect extends BattleBgEffect {
  private colours: [number, number, number][];
  private alpha: number;
  private frequency: number;
  private frame = 0;

  constructor(
    effect: ParsedBgEffect,
    options: { colours: [number, number, number][]; alpha?: number; frequency?: number },
  ) {
    super(effect);
    this.colours = [...options.colours];
    const rawAlpha = options.alpha ?? 96;
    this.alpha = Math.max(0, Math.min(255, rawAlpha));
    this.frequency = Math.max(1, options.frequency ?? 1);
    this.remaining = Math.max(1, effect.duration || this.colours.length * this.frequency);
    this.setLoopDuration(this.remaining);
  }

  update(runtime: BattleAnimationRuntime): boolean {
    if (this.remaining <= 0) {
      return false;
    }
    const phase = Math.floor(this.frame / this.frequency) % this.colours.length;
    runtime.overlay_colour = this.colours[phase];
    runtime.overlay_alpha = this.alpha;
    this.frame += 1;
    this.remaining -= 1;
    return this.remaining > 0;
  }
}

export class OscillationEffect extends BattleBgEffect {
  private axis: 'x' | 'y';
  private amplitude: number;
  private frequency: number;
  private screen: boolean;
  private force_side: BattlerSide | null;
  private frame = 0;

  constructor(
    effect: ParsedBgEffect,
    options: {
      axis?: 'x' | 'y';
      amplitude?: number;
      frequency?: number;
      screen?: boolean;
      force_side?: BattlerSide | null;
    } = {},
  ) {
    super(effect);
    this.axis = options.axis ?? 'x';
    this.frequency = Math.max(1, options.frequency ?? 2);
    this.amplitude = Math.max(1, options.amplitude ?? 4);
    this.remaining = Math.max(1, effect.duration || this.frequency * 2);
    this.screen = Boolean(options.screen);
    this.force_side = options.force_side ?? null;
    this.setLoopDuration(this.remaining);
  }

  update(runtime: BattleAnimationRuntime): boolean {
    if (this.remaining <= 0) {
      return false;
    }
    const offset = Math.round(this.amplitude * Math.sin((this.frame * Math.PI) / this.frequency));
    if (this.screen) {
      if (this.axis === 'x') {
        runtime.screen_offset_x += offset;
      } else {
        runtime.screen_offset_y += offset;
      }
    } else {
      const target = this.force_side ?? this.effect.target_side();
      runtime.apply_side_offset(target, this.axis, offset);
    }
    this.frame += 1;
    this.remaining -= 1;
    return this.remaining > 0;
  }
}

export class WaterEffect extends BattleBgEffect {
  private colour: [number, number, number] = [28, 84, 160];

  constructor(effect: ParsedBgEffect) {
    super(effect);
    this.remaining = Math.max(1, effect.duration || 6);
    this.setLoopDuration(this.remaining);
  }

  update(runtime: BattleAnimationRuntime): boolean {
    if (this.remaining <= 0) {
      return false;
    }
    runtime.overlay_colour = this.colour;
    runtime.overlay_alpha = 112;
    this.remaining -= 1;
    return this.remaining > 0;
  }
}

export class SurfWaveEffect extends BattleBgEffect {
  // ASM: engine/battle_anims/bg_effects.asm::BattleBGEffect_Surf + InitSurfWaves.
  private wave: number[] = [];
  private state = 0;
  private seenActive = false;

  constructor(effect: ParsedBgEffect) {
    super(effect);
    this.wave = Array.from({ length: 0x40 }, () => 0);
  }

  update(runtime: BattleAnimationRuntime): boolean {
    if (this.state === 0) {
      this.init_waves(2, 2);
      this.state = 1;
      return true;
    }
    if (runtime.lcd_pointer !== 'scy') {
      return !this.seenActive;
    }
    this.seenActive = true;
    this.rotate_waves();
    this.apply_line_overrides(runtime);
    return true;
  }

  private init_waves(amplitude: number, offset: number): void {
    let progress = 0;
    for (let i = 0; i < this.wave.length; i += 1) {
      this.wave[i] = sine(progress, amplitude);
      progress = (progress + offset) & 0xff;
    }
  }

  private rotate_waves(): void {
    if (!this.wave.length) {
      return;
    }
    const first = this.wave[0];
    for (let i = 0; i < this.wave.length - 1; i += 1) {
      this.wave[i] = this.wave[i + 1];
    }
    this.wave[this.wave.length - 1] = first;
  }

  private apply_line_overrides(runtime: BattleAnimationRuntime): void {
    const start = runtime.ly_override_start & 0xff;
    const end = runtime.ly_override_end & 0xff;
    let waveIndex = 0;
    for (let line = 0; line < runtime.line_scroll_y.length; line += 1) {
      let value = 0;
      if (line > start && line <= end) {
        value = this.wave[waveIndex] ?? 0;
      }
      runtime.line_scroll_y[line] = value;
      waveIndex = (waveIndex + 1) & 0x3f;
    }
  }
}

export class BattlerObjEffect extends BattleBgEffect {
  private mode: number;
  private side: BattlerSide;
  private state = 0;

  constructor(effect: ParsedBgEffect, options: { mode: number }) {
    super(effect);
    this.mode = options.mode;
    this.side = effect.target_side();
    this.remaining = Math.max(1, effect.duration || 6);
    this.setLoopDuration(this.remaining);
  }

  update(runtime: BattleAnimationRuntime): boolean {
    if (this.remaining <= 0) {
      return false;
    }
    this.state += 1;
    if (this.side === BattlerSide.PLAYER) {
      runtime.player_row_mode = this.mode;
      runtime.player_row_state = this.state;
    } else {
      runtime.enemy_row_mode = this.mode;
      runtime.enemy_row_state = this.state;
    }
    this.remaining -= 1;
    return this.remaining > 0;
  }
}

export class PicResizeEffect extends BattleBgEffect {
  private side: BattlerSide;
  private rows: number[];
  private stepDelay: number;
  private stepIndex = 0;
  private frameCounter = 0;
  private finished = false;
  private hideAtEnd: boolean;

  constructor(effect: ParsedBgEffect, options: { rows: number[]; hide_at_end: boolean }) {
    super(effect);
    this.side = effect.target_side();
    this.rows = options.rows;
    this.hideAtEnd = options.hide_at_end;
    this.stepDelay = Math.max(1, effect.duration || 1);
    if (!this.rows.length) {
      this.finished = true;
    }
  }

  update(runtime: BattleAnimationRuntime): boolean {
    if (this.finished) {
      if (this.hideAtEnd) {
        runtime.set_visibility(this.side, false);
      }
      return false;
    }
    const rowState = this.rows[this.stepIndex] ?? 0;
    if (this.side === BattlerSide.PLAYER) {
      runtime.player_row_mode = 1;
      runtime.player_row_state = rowState;
    } else {
      runtime.enemy_row_mode = 1;
      runtime.enemy_row_state = rowState;
    }
    runtime.set_visibility(this.side, true);
    this.frameCounter += 1;
    if (this.frameCounter >= this.stepDelay) {
      this.frameCounter = 0;
      this.stepIndex += 1;
      if (this.stepIndex >= this.rows.length) {
        this.finished = true;
        if (this.hideAtEnd) {
          runtime.set_visibility(this.side, false);
          return false;
        }
      }
    }
    return true;
  }
}

export class RapidCyclePalsEffect extends BattleBgEffect {
  private palettes: number[];
  private loop: boolean;
  private frequency: number;
  private frame = 0;

  constructor(effect: ParsedBgEffect, options: { palettes: number[]; loop?: boolean; frequency?: number }) {
    super(effect);
    this.palettes = [...options.palettes];
    this.loop = Boolean(options.loop);
    this.frequency = Math.max(1, options.frequency ?? 2);
    this.remaining = Math.max(1, effect.duration || this.palettes.length * this.frequency);
    if (!this.loop) {
      this.setLoopDuration(this.remaining);
    }
  }

  update(runtime: BattleAnimationRuntime): boolean {
    if (!this.loop && this.remaining <= 0) {
      return false;
    }
    let index = Math.floor(this.frame / this.frequency);
    if (index >= this.palettes.length) {
      if (this.loop) {
        index %= this.palettes.length;
      } else {
        return false;
      }
    }
    const palette = this.palettes[index];
    runtime.bgp = palette;
    runtime.obp1 = palette;
    this.frame += 1;
    if (!this.loop) {
      this.remaining -= 1;
    }
    return true;
  }
}

export const make_effect = (effect: ParsedBgEffect): BattleBgEffect => {
  const name = effect.name;
  if (!name) {
    throw new Error('Battle bg effect name must be provided.');
  }
  if (name === 'BATTLE_BG_EFFECT_SHAKE_SCREEN_X') {
    return new ScreenShakeEffect(effect, { axis: 'x' });
  }
  if (name === 'BATTLE_BG_EFFECT_SHAKE_SCREEN_Y') {
    return new ScreenShakeEffect(effect, { axis: 'y' });
  }
  if (
    [
      'BATTLE_BG_EFFECT_TACKLE',
      'BATTLE_BG_EFFECT_BODY_SLAM',
      'BATTLE_BG_EFFECT_BETA_PURSUIT',
      'BATTLE_BG_EFFECT_ROLLOUT',
      'BATTLE_BG_EFFECT_VITAL_THROW',
    ].includes(name)
  ) {
    return new LungeEffect(effect, { axis: 'x' });
  }
  if (name === 'BATTLE_BG_EFFECT_BOUNCE_DOWN') {
    return new BounceDownEffect(effect);
  }
  if (name === 'BATTLE_BG_EFFECT_FLASH_INVERTED') {
    return new FlashEffect(effect, { colour: [255, 255, 255] });
  }
  if (name === 'BATTLE_BG_EFFECT_FLASH_WHITE') {
    return new FlashEffect(effect, { colour: [255, 255, 255], alpha: 128 });
  }
  if (name === 'BATTLE_BG_EFFECT_WHITE_HUES') {
    return new PaletteCycleEffect(effect, { colours: [[248, 248, 248]], alpha: 128, frequency: 1 });
  }
  if (name === 'BATTLE_BG_EFFECT_BLACK_HUES') {
    return new PaletteCycleEffect(effect, { colours: [[0, 0, 0]], alpha: 128, frequency: 1 });
  }
  if (name === 'BATTLE_BG_EFFECT_ALTERNATE_HUES') {
    return new PaletteCycleEffect(effect, {
      colours: [
        [255, 255, 255],
        [8, 8, 8],
      ],
      alpha: 112,
      frequency: 2,
    });
  }
  if (
    ['BATTLE_BG_EFFECT_CYCLE_OBPALS_GRAY_AND_YELLOW', 'BATTLE_BG_EFFECT_CYCLE_MID_OBPALS_GRAY_AND_YELLOW'].includes(
      name,
    )
  ) {
    return new PaletteCycleEffect(effect, {
      colours: [
        [192, 192, 192],
        [255, 232, 120],
      ],
      alpha: 96,
      frequency: 3,
    });
  }
  if (name === 'BATTLE_BG_EFFECT_CYCLE_BGPALS_INVERTED') {
    return new PaletteCycleEffect(effect, {
      colours: [
        [255, 255, 255],
        [16, 16, 8],
      ],
      alpha: 104,
      frequency: 2,
    });
  }
  if (name === 'BATTLE_BG_EFFECT_CYCLE_MON_LIGHT_DARK_REPEATING') {
    return new PaletteCycleEffect(effect, {
      colours: [
        [224, 224, 224],
        [16, 16, 16],
      ],
      alpha: 96,
      frequency: 3,
    });
  }
  if (name === 'BATTLE_BG_EFFECT_ACID_ARMOR') {
    return new PaletteCycleEffect(effect, {
      colours: [
        [148, 200, 148],
        [48, 112, 64],
      ],
      alpha: 100,
      frequency: 2,
    });
  }
  if (['BATTLE_BG_EFFECT_HIDE_MON', 'BATTLE_BG_EFFECT_WITHDRAW'].includes(name)) {
    return new VisibilityEffect(effect, { visible: false });
  }
  // ASM: BattleBGEffect_ShowMon (engine/battle_anims/bg_effects.asm) restores the pic without slide offsets.
  if (name === 'BATTLE_BG_EFFECT_SHOW_MON') {
    return new VisibilityEffect(effect, { visible: true });
  }
  if (
    [
      'BATTLE_BG_EFFECT_BETA_SEND_OUT_MON1',
      'BATTLE_BG_EFFECT_BETA_SEND_OUT_MON2',
    ].includes(name)
  ) {
    return new SlidePicEffect(effect, { direction: 'in', hide_at_end: false });
  }
  if (name === 'BATTLE_BG_EFFECT_ENTER_MON') {
    // ASM square expansion: player 2 -> 4 -> 6, enemy 3 -> 5 -> 7.
    const rows = effect.target_side() === BattlerSide.PLAYER
      ? build_pic_resize_rows([2, 1, 0])
      : build_pic_resize_rows([5, 4, 3]);
    return new PicResizeEffect(effect, { rows, hide_at_end: false });
  }
  if (name === 'BATTLE_BG_EFFECT_RETURN_MON') {
    // ASM square collapse: player 6 -> 4 -> 2, enemy 7 -> 5 -> 3, then hide.
    const rows = effect.target_side() === BattlerSide.PLAYER
      ? build_pic_resize_rows([0, 1, 2], { hold_last: true })
      : build_pic_resize_rows([3, 4, 5], { hold_last: true });
    return new PicResizeEffect(effect, { rows, hide_at_end: true });
  }
  if (name === 'BATTLE_BG_EFFECT_REMOVE_MON') {
    return new SlidePicEffect(effect, { direction: 'out', hide_at_end: true });
  }
  if (name === 'BATTLE_BG_EFFECT_FAINT_MON') {
    return new FaintEffect(effect);
  }
  if (['BATTLE_BG_EFFECT_WOBBLE_MON', 'BATTLE_BG_EFFECT_WAVE_DEFORM_MON'].includes(name)) {
    return new OscillationEffect(effect, {
      axis: 'y',
      amplitude: effect.param || 4,
      frequency: effect.duration || 3,
    });
  }
  if (name === 'BATTLE_BG_EFFECT_WOBBLE_PLAYER') {
    return new OscillationEffect(effect, {
      axis: 'x',
      amplitude: effect.param || 3,
      frequency: effect.duration || 4,
      force_side: BattlerSide.PLAYER,
    });
  }
  if (name === 'BATTLE_BG_EFFECT_WOBBLE_SCREEN') {
    return new OscillationEffect(effect, {
      axis: 'x',
      amplitude: effect.param || 3,
      frequency: effect.duration || 4,
      screen: true,
    });
  }
  if (name === 'BATTLE_BG_EFFECT_VIBRATE_MON') {
    return new OscillationEffect(effect, { axis: 'x', amplitude: effect.param || 2, frequency: 1 });
  }
  if (['BATTLE_BG_EFFECT_DIG', 'BATTLE_BG_EFFECT_FLAIL', 'BATTLE_BG_EFFECT_DOUBLE_TEAM'].includes(name)) {
    return new OscillationEffect(effect, {
      axis: 'y',
      amplitude: effect.param || 5,
      frequency: effect.param || 3,
    });
  }
  if (name === 'BATTLE_BG_EFFECT_SURF') {
    return new SurfWaveEffect(effect);
  }
  if (
    [
      'BATTLE_BG_EFFECT_START_WATER',
      'BATTLE_BG_EFFECT_WATER',
      'BATTLE_BG_EFFECT_END_WATER',
      'BATTLE_BG_EFFECT_WHIRLPOOL',
    ].includes(name)
  ) {
    return new WaterEffect(effect);
  }
  if (name === 'BATTLE_BG_EFFECT_NIGHT_SHADE') {
    return new PaletteCycleEffect(effect, { colours: [[4, 4, 12]], alpha: 128, frequency: 1 });
  }
  if (name === 'BATTLE_BG_EFFECT_PSYCHIC') {
    return new PaletteCycleEffect(effect, {
      colours: [
        [152, 80, 192],
        [64, 32, 128],
      ],
      alpha: 120,
      frequency: 2,
    });
  }
  if (name === 'BATTLE_BG_EFFECT_TELEPORT') {
    return new PaletteCycleEffect(effect, { colours: [[120, 120, 220]], alpha: 110, frequency: 1 });
  }
  if (['BATTLE_BG_EFFECT_FADE_MON_TO_LIGHT', 'BATTLE_BG_EFFECT_FADE_MON_TO_LIGHT_REPEATING'].includes(name)) {
    return new FlashEffect(effect, {
      colour: [255, 255, 255],
      alpha: 128,
      target_side: effect.target_side(),
    });
  }
  if (name === 'BATTLE_BG_EFFECT_FADE_MON_TO_WHITE_WAIT_FADE_BACK') {
    return new FadeMonToWhiteWaitFadeBackEffect(effect);
  }
  if (
    [
      'BATTLE_BG_EFFECT_FADE_MON_TO_BLACK',
      'BATTLE_BG_EFFECT_FADE_MON_TO_BLACK_REPEATING',
      'BATTLE_BG_EFFECT_FADE_MONS_TO_BLACK_REPEATING',
    ].includes(name)
  ) {
    return new FlashEffect(effect, {
      colour: [0, 0, 0],
      alpha: 128,
      target_side: effect.target_side(),
    });
  }
  if (name === 'BATTLE_BG_EFFECT_BATTLEROBJ_1ROW') {
    return new BattlerObjEffect(effect, { mode: 1 });
  }
  if (name === 'BATTLE_BG_EFFECT_BATTLEROBJ_2ROW') {
    return new BattlerObjEffect(effect, { mode: 2 });
  }
  if (name === 'BATTLE_BG_EFFECT_RAPID_FLASH') {
    return new RapidCyclePalsEffect(effect, { palettes: [0xe4, 0x6c], loop: true, frequency: 2 });
  }
  if (name === 'BATTLE_BG_EFFECT_FLASH_MON_REPEATING') {
    return new RapidCyclePalsEffect(effect, { palettes: [0xe4, 0xfc, 0xe4, 0x00], loop: true, frequency: 2 });
  }
  if (name === 'BATTLE_BG_EFFECT_FADE_MON_FROM_WHITE') {
    return new RapidCyclePalsEffect(effect, { palettes: [0x00, 0x40, 0x90, 0xe4], loop: false, frequency: 2 });
  }
  throw new Error(`Unsupported battle bg effect: ${name}`);
};

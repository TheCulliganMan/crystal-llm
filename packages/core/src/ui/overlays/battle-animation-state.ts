import { z } from 'zod';

export const parse_int_token = (rawValue: string | number | null | undefined): number | null => {
  if (rawValue === null || rawValue === undefined) {
    return null;
  }
  if (typeof rawValue === 'number' && !Number.isNaN(rawValue)) {
    return rawValue;
  }
  const text = String(rawValue).replace(/,+$/, '').trim();
  if (!text) {
    return null;
  }
  if (text.startsWith('$')) {
    const parsed = Number.parseInt(text.slice(1), 16);
    return Number.isNaN(parsed) ? null : parsed;
  }
  if (text.startsWith('%')) {
    const parsed = Number.parseInt(text.slice(1), 2);
    return Number.isNaN(parsed) ? null : parsed;
  }
  const parsed = Number.parseInt(text, 10);
  return Number.isNaN(parsed) ? null : parsed;
};

export const AnimationCommandSchema = z.object({
  command: z.string(),
  args: z.array(z.string()).default([]),
});

export type AnimationCommand = z.infer<typeof AnimationCommandSchema>;

export const AnimationSchema = z.object({
  name: z.string(),
  script: z.array(AnimationCommandSchema),
  labels: z.record(z.string(), z.number()).default({}),
});

export type Animation = z.infer<typeof AnimationSchema>;

export const AnimationSpriteSchema = z.object({
  object_id: z.string(),
  x: z.number(),
  y: z.number(),
  fix_y: z.number().default(0),
  base_x: z.number().nullable().optional(),
  base_y: z.number().nullable().optional(),
  frameset_index: z.number().default(-1),
  frame_duration: z.number().default(0),
  frame: z.number().default(-1),
  active_frameset: z.string().nullable().optional(),
  x_offset: z.number().default(0),
  y_offset: z.number().default(0),
  var1: z.number().default(0),
  var2: z.number().default(0),
  param: z.number(),
  function_id: z.string().nullable().optional(),
  override_frameset: z.string().nullable().optional(),
  palette_override: z.string().nullable().optional(),
  index: z.number().default(0),
  state: z.number().default(0),
  jump_index: z.number().default(0),
  mirror_x: z.boolean().default(false),
  mirror_y: z.boolean().default(false),
  relative_coords: z.boolean().default(false),
  is_player_move: z.boolean().default(true),
  param_label: z.string().nullable().optional(),
  target_shakes: z.number().nullable().optional(),
  wobble_count: z.number().default(0),
});

export type AnimationSprite = z.infer<typeof AnimationSpriteSchema>;

export const AnimationSoundSchema = z.object({
  sound_id: z.string(),
  duration: z.number().nullable().optional(),
  tracks: z.number().nullable().optional(),
  cry_selector: z.number().nullable().optional(),
  sound_type: z.string().default('sound'),
  pitch: z.number().nullable().optional(),
  panning: z.string().nullable().optional(),
});

export type AnimationSound = z.infer<typeof AnimationSoundSchema>;

export const isCrySound = (sound: AnimationSound): boolean => sound.sound_type === 'cry';

export enum BattleAnimationFlag {
  STOP = 1 << 0,
  IN_SUBROUTINE = 1 << 1,
}

export interface LoopState {
  command_index: number;
  remaining: number;
  label: string;
}

export class AnimationContext {
  public animation: Animation;
  public pointer: number;
  public is_player_move: boolean;

  constructor({ animation, pointer = 0, is_player_move = true }: { animation: Animation; pointer?: number; is_player_move?: boolean }) {
    this.animation = animation;
    this.pointer = pointer;
    this.is_player_move = is_player_move;
  }

  clone(): AnimationContext {
    return new AnimationContext({
      animation: this.animation,
      pointer: this.pointer,
      is_player_move: this.is_player_move,
    });
  }
}

export class AnimationObjectTable {
  private readonly max_objects: number;
  private objects: AnimationSprite[] = [];
  private lastSlot = 0;

  constructor(maxObjects: number = 10) {
    this.max_objects = maxObjects;
  }

  get sprites(): AnimationSprite[] {
    return this.objects;
  }

  reset(resetSlots: boolean = true) {
    this.objects = [];
    if (resetSlots) {
      this.lastSlot = 0;
    }
  }

  add(sprite: AnimationSprite, indexOverride?: number): AnimationSprite | null {
    if (this.objects.length >= this.max_objects) {
      // ASM QueueBattleAnimation sets carry and returns here; the caller keeps running.
      return null;
    }
    if (indexOverride !== undefined) {
      sprite.index = indexOverride;
      this.lastSlot = Math.max(this.lastSlot, indexOverride);
    } else {
      this.lastSlot = (this.lastSlot % 0xff) + 1;
      sprite.index = this.lastSlot;
    }
    this.objects.push(sprite);
    return sprite;
  }

  clear() {
    this.objects = [];
    this.lastSlot = 0;
  }

  private resolveIdentifier(identifier: string | number): [number | null, string | null] {
    if (typeof identifier === 'number') {
      return [identifier, null];
    }
    const slot = parse_int_token(identifier);
    return [slot, identifier];
  }

  find(identifier: string | number): AnimationSprite | null {
    const [slot, objectId] = this.resolveIdentifier(identifier);
    for (let i = this.objects.length - 1; i >= 0; i -= 1) {
      const sprite = this.objects[i];
      if (slot !== null && sprite.index === slot) {
        return sprite;
      }
      if (objectId !== null && sprite.object_id === objectId) {
        return sprite;
      }
    }
    return null;
  }

  increment(identifier: string | number): AnimationSprite | null {
    const sprite = this.find(identifier);
    if (!sprite) {
      return null;
    }
    sprite.jump_index = (sprite.jump_index + 1) & 0xff;
    return sprite;
  }

  set_jump_index(identifier: string | number, value: number): AnimationSprite | null {
    const sprite = this.find(identifier);
    if (!sprite) {
      return null;
    }
    sprite.jump_index = value & 0xff;
    return sprite;
  }

  // Keep compatibility with older call sites and docs that called set_state; this
  // still maps to the ASM jump-table index to avoid mutating runtime state.
  set_state(identifier: string | number, value: number): AnimationSprite | null {
    const sprite = this.find(identifier);
    if (!sprite) {
      return null;
    }
    sprite.jump_index = value & 0xff;
    return sprite;
  }

  remove(sprite: AnimationSprite) {
    const index = this.objects.indexOf(sprite);
    if (index >= 0) {
      this.objects.splice(index, 1);
    }
  }

  mirror_all(screenWidth: number) {
    for (const sprite of this.objects) {
      const newX = Math.max(0, screenWidth - sprite.x);
      sprite.x = newX;
      sprite.mirror_x = !sprite.mirror_x;
    }
  }
}

export const parse_battle_int = (rawValue: string | number | null | undefined): number | null =>
  parse_int_token(rawValue);

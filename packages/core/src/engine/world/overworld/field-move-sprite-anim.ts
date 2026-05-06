// ASM mapping: pokecrystal_disassembly/engine/overworld/field_moves.asm (Cut/Headbutt/Fly sprite anims).
import { GameState } from "@pokecrystal/core/core/state";
import { METATILE_SIZE, METATILE_WIDTH, TILE_SIZE } from "@pokecrystal/core/engine/world/tile";

export const OBJ_SIZE = 4;
export const OAM_CAPACITY_BYTES = 0xa0;
export const FIELD_MOVE_SPRITE_OAM_ENTRIES = 36;
export const FIELDMOVE_TREE = 0x84;
export const FIELDMOVE_GRASS = 0x80;
export const FIELDMOVE_FLY = 0x84;
export const JUMPTABLE_EXIT_F = 0x80;
export const SPRITE_ANIM_OBJ_RED_WALK = 0x0a;
export const SPRITE_ANIM_OBJ_LEAF = 0x16;
export const SPRITE_ANIM_OBJ_CUT_TREE = 0x17;
export const SPRITE_ANIM_OBJ_FLY_LEAF = 0x18;
export const SPRITE_ANIM_FUNC_FLY_FROM = 0x16;
export const SPRITE_ANIM_FUNC_FLY_LEAF = 0x17;
export const SPRITE_ANIM_FUNC_FLY_TO = 0x18;

export const SPRITEANIMSTRUCT_INDEX = 0x00;
export const SPRITEANIMSTRUCT_FRAMESET_ID = 0x01;
export const SPRITEANIMSTRUCT_ANIM_SEQ_ID = 0x02;
export const SPRITEANIMSTRUCT_TILE_ID = 0x03;
export const SPRITEANIMSTRUCT_XCOORD = 0x04;
export const SPRITEANIMSTRUCT_YCOORD = 0x05;
export const SPRITEANIMSTRUCT_XOFFSET = 0x06;
export const SPRITEANIMSTRUCT_YOFFSET = 0x07;
export const SPRITEANIMSTRUCT_DURATION = 0x08;
export const SPRITEANIMSTRUCT_DURATIONOFFSET = 0x09;
export const SPRITEANIMSTRUCT_FRAME = 0x0a;
export const SPRITEANIMSTRUCT_JUMPTABLE_INDEX = 0x0b;
export const SPRITEANIMSTRUCT_VAR1 = 0x0c;
export const SPRITEANIMSTRUCT_VAR2 = 0x0d;
export const SPRITEANIMSTRUCT_VAR3 = 0x0e;
export const SPRITEANIMSTRUCT_VAR4 = 0x0f;
export const SPRITEANIMSTRUCT_LENGTH = 0x10;

export const CUT_HEADBUTT_PIXEL_FACING: Record<string, [number, number]> = {
  down: [10, 13],
  up: [10, 9],
  left: [8, 11],
  right: [12, 11],
};

const CUT_LEAF_SPAWN_COORDS: Array<[number, number]> = [
  [11, 12],
  [9, 12],
  [11, 14],
  [9, 14],
  [11, 8],
  [9, 8],
  [11, 10],
  [9, 10],
  [7, 12],
  [9, 12],
  [7, 10],
  [9, 10],
  [11, 12],
  [13, 12],
  [11, 10],
  [13, 10],
];

const CUT_LEAF_DIRECTION_INDEX: Record<string, number> = {
  down: 0,
  up: 4,
  left: 8,
  right: 12,
};

const LEAF_VAR1_OFFSETS = [0x00, 0x10, 0x20, 0x30] as const;
const LEAF_Y_OFFSET_MASK = (6 * TILE_SIZE) >> 1;
const LEAF_Y_BASE = 8 * TILE_SIZE;

function tile_to_pixels(coord: [number, number]): [number, number] {
  return [coord[0] * TILE_SIZE, coord[1] * TILE_SIZE];
}

export function cut_headbutt_get_pixel_facing(direction: string): [number, number] {
  const normalized = direction.toLowerCase();
  return CUT_HEADBUTT_PIXEL_FACING[normalized] ?? CUT_HEADBUTT_PIXEL_FACING.down;
}

export function cut_get_leaf_spawn_coords(
  direction: string,
  metatile_x: number,
  metatile_y: number
): [number, number] {
  const normalized = direction.toLowerCase();
  const direction_offset = CUT_LEAF_DIRECTION_INDEX[normalized] ?? 0;
  const x_flag = metatile_x & 1 ? 1 : 0;
  const y_flag = metatile_y & 1 ? 2 : 0;
  const index = direction_offset + x_flag + y_flag;
  const coords = CUT_LEAF_SPAWN_COORDS[index];
  return tile_to_pixels(coords);
}

export class SpriteAnimStruct {
  public index: number;
  public frameset_id: number = 0;
  public anim_seq_id: number = 0;
  public tile_id: number = 0;
  public x_coord: number = 0;
  public y_coord: number = 0;
  public x_offset: number = 0;
  public y_offset: number = 0;
  public duration: number = 0;
  public duration_offset: number = 0;
  public frame: number = 0;
  public jumptable_index: number = 0;
  public var1: number = 0;
  public var2: number = 0;
  public var3: number = 0;
  public var4: number = 0;
  public oam_slots: number = 1;

  constructor(index: number) {
    this.index = index;
  }
}

export function InitSpriteAnimStruct(
  index: number,
  {
    frameset_id = 0,
    anim_seq_id = 0,
    tile_id = 0,
    x_coord = 0,
    y_coord = 0,
    x_offset = 0,
    y_offset = 0,
    duration = 0,
    duration_offset = 0,
    frame = 0,
    jumptable_index = 0,
    var1 = 0,
    var2 = 0,
    var3 = 0,
    var4 = 0,
    oam_slots = 1,
  }: {
    frameset_id?: number;
    anim_seq_id?: number;
    tile_id?: number;
    x_coord?: number;
    y_coord?: number;
    x_offset?: number;
    y_offset?: number;
    duration?: number;
    duration_offset?: number;
    frame?: number;
    jumptable_index?: number;
    var1?: number;
    var2?: number;
    var3?: number;
    var4?: number;
    oam_slots?: number;
  } = {}
): SpriteAnimStruct {
  const struct = new SpriteAnimStruct(index);
  struct.frameset_id = frameset_id;
  struct.anim_seq_id = anim_seq_id;
  struct.tile_id = tile_id;
  struct.x_coord = x_coord;
  struct.y_coord = y_coord;
  struct.x_offset = x_offset;
  struct.y_offset = y_offset;
  struct.duration = duration;
  struct.duration_offset = duration_offset;
  struct.frame = frame;
  struct.jumptable_index = jumptable_index;
  struct.var1 = var1;
  struct.var2 = var2;
  struct.var3 = var3;
  struct.var4 = var4;
  struct.oam_slots = Math.max(0, Math.trunc(oam_slots));
  return struct;
}

export class FieldMoveSpriteManager {
  private structs: SpriteAnimStruct[] = [];
  private next_index = 0;
  private oam_base = 0;
  private pending_oam_slots = 0;
  private state: GameState | null = null;

  public reserve_oam(state: GameState, { base_addr = null }: { base_addr?: number | null } = {}): void {
    const base = base_addr === null ? FIELD_MOVE_SPRITE_OAM_ENTRIES * OBJ_SIZE : base_addr;
    this.state = state;
    this.set_oam_base(base);
  }

  public clear_sprite_anims(state: GameState): void {
    this.structs = [];
    this.next_index = 0;
    this.pending_oam_slots = 0;
    this.state = state;
    this.set_oam_base(0);
  }

  public init_sprite_anim_struct(
    {
      tile_id,
      x_coord,
      y_coord,
      frameset_id = 0,
      anim_seq_id = 0,
      var1 = 0,
      var2 = 0,
      var3 = 0,
      var4 = 0,
      oam_slots = 1,
    }: {
      tile_id: number;
      x_coord: number;
      y_coord: number;
      frameset_id?: number;
      anim_seq_id?: number;
      var1?: number;
      var2?: number;
      var3?: number;
      var4?: number;
      oam_slots?: number;
    }
  ): SpriteAnimStruct {
    const struct = InitSpriteAnimStruct(this.next_index, {
      tile_id,
      x_coord,
      y_coord,
      frameset_id,
      anim_seq_id,
      var1,
      var2,
      var3,
      var4,
      oam_slots,
    });
    this.structs.push(struct);
    this.next_index += 1;
    this.pending_oam_slots += struct.oam_slots;
    return struct;
  }

  public do_next_frame_for_all_sprites(): void {
    this.sync_oam_cursor(this.oam_base);
    for (const struct of this.structs) {
      struct.frame = (struct.frame + 1) & 0xff;
    }
    const next_cursor = this.oam_base + this.pending_oam_slots * OBJ_SIZE;
    this.sync_oam_cursor(next_cursor);
  }

  public get_structs(): SpriteAnimStruct[] {
    return this.structs;
  }

  private set_oam_base(base: number): void {
    const normalized = base & 0xff;
    if (normalized > OAM_CAPACITY_BYTES) {
      throw new Error(
        `OAM base ${base.toString(16)} exceeds capacity ${OAM_CAPACITY_BYTES.toString(16)}`
      );
    }
    this.oam_base = normalized;
    this.sync_oam_cursor(this.oam_base);
    this.pending_oam_slots = 0;
  }

  private sync_oam_cursor(cursor: number): void {
    if (cursor > OAM_CAPACITY_BYTES) {
      throw new Error(
        `Sprite animations consumed ${cursor} bytes of OAM (limit ${OAM_CAPACITY_BYTES}).`
      );
    }
    if (this.state) {
      this.state.wram.wCurSpriteOAMAddr = cursor & 0xff;
    }
  }
}

export class CutAnimationState {
  private manager: FieldMoveSpriteManager;
  private state: GameState;
  public player_x: number;
  public player_y: number;
  public target_tile_x: number;
  public target_tile_y: number;
  public direction: string;
  public variant: string;
  public jumptable_index: number;
  public frame_counter: number;
  public completed: boolean;

  constructor(
    manager: FieldMoveSpriteManager,
    state: GameState,
    {
      player_x,
      player_y,
      target_tile_x,
      target_tile_y,
      direction,
      variant,
    }: {
      player_x: number;
      player_y: number;
      target_tile_x: number;
      target_tile_y: number;
      direction: string;
      variant: string;
    }
  ) {
    this.manager = manager;
    this.state = state;
    this.player_x = player_x;
    this.player_y = player_y;
    this.target_tile_x = target_tile_x;
    this.target_tile_y = target_tile_y;
    this.direction = direction.toLowerCase();
    this.variant = variant.toLowerCase();
    this.jumptable_index = this.variant === "tree" ? 0 : 1;
    this.frame_counter = 0;
    this.completed = false;
    this.update_jumptable_index(this.jumptable_index);
  }

  public tick(): void {
    if (this.completed) {
      return;
    }
    this.manager.do_next_frame_for_all_sprites();
    if (this.jumptable_index & JUMPTABLE_EXIT_F) {
      this.completed = true;
      return;
    }
    const handler = {
      0: () => this.spawn_tree(),
      1: () => this.spawn_leaves(),
      2: () => this.start_waiting(),
      3: () => this.wait_anim_sfx(),
    }[this.jumptable_index];
    if (handler) {
      handler();
    }
  }

  private spawn_tree(): void {
    const coords = this.tree_spawn_coord();
    this.manager.init_sprite_anim_struct({
      tile_id: FIELDMOVE_TREE,
      x_coord: coords[0],
      y_coord: coords[1],
      frameset_id: SPRITE_ANIM_OBJ_CUT_TREE,
      oam_slots: 0,
    });
    this.update_frame_counter(32);
    this.update_jumptable_index(2);
  }

  private spawn_leaves(): void {
    const base = this.leaf_spawn_coord();
    for (const var1 of LEAF_VAR1_OFFSETS) {
      this.manager.init_sprite_anim_struct({
        tile_id: FIELDMOVE_GRASS,
        x_coord: base[0],
        y_coord: base[1],
        frameset_id: SPRITE_ANIM_OBJ_LEAF,
        var1,
        var3: 4,
      });
    }
    this.update_frame_counter(32);
    this.update_jumptable_index(2);
  }

  private start_waiting(): void {
    this.update_jumptable_index(3);
  }

  private wait_anim_sfx(): void {
    if (this.frame_counter > 0) {
      this.update_frame_counter(this.frame_counter - 1);
      return;
    }
    this.update_jumptable_index(this.jumptable_index | JUMPTABLE_EXIT_F);
    this.completed = true;
    this.manager.clear_sprite_anims(this.state);
  }

  private leaf_spawn_coord(): [number, number] {
    const { wram } = this.state;
    const metatile_x = wram.player_x ?? Math.trunc(this.player_x / METATILE_WIDTH);
    const metatile_y = wram.player_y ?? Math.trunc(this.player_y / METATILE_WIDTH);
    const [offset_x, offset_y] = cut_get_leaf_spawn_coords(
      this.direction,
      metatile_x,
      metatile_y
    );
    const base_x = this.target_tile_x * METATILE_SIZE;
    const base_y = this.target_tile_y * METATILE_SIZE;
    return [base_x + offset_x, base_y + offset_y];
  }

  private tree_spawn_coord(): [number, number] {
    const offsets = cut_headbutt_get_pixel_facing(this.direction);
    const base_x = this.target_tile_x * METATILE_SIZE;
    const base_y = this.target_tile_y * METATILE_SIZE;
    return [base_x + offsets[0], base_y + offsets[1]];
  }

  private update_frame_counter(value: number): void {
    this.frame_counter = value;
    this.state.wram.wFrameCounter = value;
  }

  private update_jumptable_index(value: number): void {
    this.jumptable_index = value;
    this.state.wram.wJumptableIndex = value;
  }
}

export class FlyAnimationState {
  private manager: FieldMoveSpriteManager;
  private state: GameState;
  public player_x: number;
  public player_y: number;
  public variant: string;
  private sound_player: ((id: string) => void) | null;
  private leaf_counter: number;
  public frame_counter: number;
  public completed: boolean;

  constructor(
    manager: FieldMoveSpriteManager,
    state: GameState,
    {
      player_x = 0,
      player_y = 0,
      variant = "from",
      sound_player = null,
    }: {
      player_x?: number;
      player_y?: number;
      variant?: string;
      sound_player?: ((id: string) => void) | null;
    } = {}
  ) {
    this.manager = manager;
    this.state = state;
    this.player_x = player_x;
    this.player_y = player_y;
    this.variant = variant.toLowerCase();
    this.sound_player = sound_player;
    this.leaf_counter = 0;
    this.frame_counter = this.variant === "from" ? 128 : 64;
    this.completed = false;
    this.spawn_player_sprite();
    this.state.wram.wFrameCounter = this.frame_counter;
    this.state.wram.wFrameCounter2 = this.leaf_counter;
    this.state.wram.wJumptableIndex = 0;
  }

  public tick(): void {
    if (this.completed) {
      return;
    }
    this.state.wram.wCurSpriteOAMAddr = 0;
    this.manager.do_next_frame_for_all_sprites();
    this.spawn_leaf();
    this.tick_frame_counter();
  }

  private spawn_player_sprite(): void {
    const anim_seq = this.variant === "from" ? SPRITE_ANIM_FUNC_FLY_FROM : SPRITE_ANIM_FUNC_FLY_TO;
    const var4 = this.variant === "from" ? 0 : 11 * TILE_SIZE;
    this.manager.init_sprite_anim_struct({
      tile_id: FIELDMOVE_FLY,
      x_coord: this.player_x,
      y_coord: this.player_y,
      frameset_id: SPRITE_ANIM_OBJ_RED_WALK,
      anim_seq_id: anim_seq,
      var4,
    });
  }

  private spawn_leaf(): void {
    const previous_counter = this.leaf_counter;
    this.leaf_counter = (this.leaf_counter + 1) & 0xff;
    this.state.wram.wFrameCounter2 = this.leaf_counter;
    if (previous_counter & 0x07) {
      return;
    }
    const selector = this.leaf_counter & LEAF_Y_OFFSET_MASK;
    const delta = (selector << 1) + LEAF_Y_BASE;
    this.manager.init_sprite_anim_struct({
      tile_id: FIELDMOVE_GRASS,
      x_coord: this.player_x,
      y_coord: this.player_y + delta,
      frameset_id: SPRITE_ANIM_OBJ_FLY_LEAF,
      anim_seq_id: SPRITE_ANIM_FUNC_FLY_LEAF,
    });
  }

  private tick_frame_counter(): void {
    const previous_counter = this.frame_counter & 0xff;
    if (previous_counter === 0) {
      this.finish();
      return;
    }
    this.frame_counter = (previous_counter - 1) & 0xff;
    this.state.wram.wFrameCounter = this.frame_counter;
    if (previous_counter < 64 || (previous_counter & 0x07)) {
      return;
    }
    if (this.sound_player) {
      this.sound_player("SFX_FLY");
    }
  }

  private finish(): void {
    this.completed = true;
    this.state.wram.wJumptableIndex |= JUMPTABLE_EXIT_F;
    this.manager.clear_sprite_anims(this.state);
  }
}

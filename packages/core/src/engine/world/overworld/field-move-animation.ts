// ASM mapping: pokecrystal_disassembly/engine/overworld/field_moves.asm (Cut/Headbutt/Fly/Whirlpool animations).
import fs from "fs";
import path from "path";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { GameState } from "@pokecrystal/core/core/state";
import { getAssetPath, getTilesetMetatilesPath } from "@pokecrystal/core/core/paths";
import { METATILE_SIZE, METATILE_WIDTH, TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { TileBlockManager, VRAMManager } from "@pokecrystal/core/core/memory/vram";
import { CUT_HEADBUTT_PIXEL_FACING, cut_get_leaf_spawn_coords } from "./field-move-sprite-anim";
import { cosine, sine } from "@pokecrystal/core/ui/overlays/battle-anim-math";
import { WHIRLPOOL_TILE_INDEXES } from "./tileset-animation";
import { NpcPaletteManager } from "./palette";

export type Surface = InstanceType<typeof gameEngine.Surface>;

export type SurfaceSupplier = Surface | (() => Surface);

export class FieldMoveAnimationLayer {
  public surface: SurfaceSupplier;
  public offset_x: number;
  public offset_y: number;

  constructor(surface: SurfaceSupplier, offset_x: number = 0, offset_y: number = 0) {
    this.surface = surface;
    this.offset_x = offset_x;
    this.offset_y = offset_y;
  }

  public resolved_surface(): Surface {
    return typeof this.surface === "function" ? this.surface() : this.surface;
  }
}

export class FieldMoveAnimationFrame {
  public duration: number;
  public layers: FieldMoveAnimationLayer[];

  constructor(duration: number, layers: FieldMoveAnimationLayer[]) {
    this.duration = duration;
    this.layers = layers;
  }
}

export class FieldMoveAnimationDefinition {
  public name: string;
  public frames: FieldMoveAnimationFrame[];
  public anchor: [number, number];
  public directional_anchors: Record<string, [number, number]> | null;

  constructor(
    name: string,
    frames: FieldMoveAnimationFrame[],
    {
      anchor = [0, 0],
      directional_anchors = null,
    }: { anchor?: [number, number]; directional_anchors?: Record<string, [number, number]> | null } = {}
  ) {
    this.name = name;
    this.frames = frames;
    this.anchor = anchor;
    this.directional_anchors = directional_anchors;
  }

  get duration(): number {
    return this.frames.reduce((total, frame) => total + frame.duration, 0);
  }

  public anchor_for_direction(direction?: string | null): [number, number] {
    if (!direction || !this.directional_anchors) {
      return this.anchor;
    }
    const normalized = direction.toLowerCase();
    return this.directional_anchors[normalized] ?? this.anchor;
  }
}

class TileAnimationTimer {
  private state: GameState | null;
  private value_cache: number;

  constructor(state: GameState | null = null) {
    this.state = state;
    this.value_cache = state ? state.wram.wTileAnimationTimer & 0xff : 0;
  }

  public bind(state: GameState): void {
    this.state = state;
    this.value_cache = state.wram.wTileAnimationTimer & 0xff;
  }

  public tick(): void {
    if (this.state) {
      const updated = ((this.state.wram.wTileAnimationTimer ?? 0) + 1) & 0xff;
      this.state.wram.wTileAnimationTimer = updated;
      this.value_cache = updated;
      return;
    }
    this.value_cache = (this.value_cache + 1) & 0xff;
  }

  get value(): number {
    if (this.state) {
      this.value_cache = this.state.wram.wTileAnimationTimer & 0xff;
    }
    return this.value_cache;
  }

  get whirlpool_phase(): number {
    return this.value & 0x3;
  }
}

const tile_animation_timer_instance = new TileAnimationTimer();

export function bind_tile_animation_timer(game_state: GameState): void {
  tile_animation_timer_instance.bind(game_state);
}

export function tile_animation_timer(): TileAnimationTimer {
  return tile_animation_timer_instance;
}

export class FieldMoveAnimationPlayer {
  public definition: FieldMoveAnimationDefinition;
  public world_x: number;
  public world_y: number;
  private anchor: [number, number];
  private frame_index: number;
  private frame_timer: number;

  constructor(
    definition: FieldMoveAnimationDefinition,
    world_x: number,
    world_y: number,
    direction?: string | null
  ) {
    this.definition = definition;
    this.world_x = world_x;
    this.world_y = world_y;
    this.anchor = definition.anchor_for_direction(direction ?? null);
    this.frame_index = 0;
    this.frame_timer = definition.frames.length ? definition.frames[0].duration : 0;
  }

  public advance(): void {
    if (this.is_finished) {
      return;
    }
    this.frame_timer -= 1;
    if (this.frame_timer > 0) {
      return;
    }
    this.frame_index += 1;
    if (this.is_finished) {
      return;
    }
    this.frame_timer = this.definition.frames[this.frame_index].duration;
  }

  get is_finished(): boolean {
    return this.frame_index >= this.definition.frames.length;
  }

  public draw(surface: Surface, screen_x: number, screen_y: number): void {
    if (this.is_finished) {
      return;
    }
    const frame = this.definition.frames[this.frame_index];
    const [anchor_x, anchor_y] = this.anchor;
    for (const layer of frame.layers) {
      const dest: [number, number] = [
        screen_x + layer.offset_x - anchor_x,
        screen_y + layer.offset_y - anchor_y,
      ];
      surface.blit(layer.resolved_surface(), dest);
    }
  }
}

const CUT_TREE = "cut_tree.png";
const CUT_TREE_2BPP = "cut_tree.2bpp";
const CUT_GRASS = "cut_grass.png";
const CUT_GRASS_2BPP = "cut_grass.2bpp";
const HEADBUTT_TREE = "headbutt_tree.png";
const HEADBUTT_TREE_2BPP = "headbutt_tree.2bpp";
const FLY_ICON_2BPP = "bird.2bpp";
// ASM: engine/events/field_moves.asm::ShakeHeadbuttTree sets wFrameCounter to 32.
export const HEADBUTT_SHAKE_FRAMES = 32;

const CUT_TREE_SIZE: [number, number] = [16, 16];
const CUT_GRASS_SIZE: [number, number] = [16, 16];
const HEADBUTT_TREE_SIZE: [number, number] = [16, 32];

const SURFACE_SPECS: Record<string, [number, number]> = {
  [CUT_TREE_2BPP]: CUT_TREE_SIZE,
  [CUT_GRASS_2BPP]: CUT_GRASS_SIZE,
  [HEADBUTT_TREE_2BPP]: HEADBUTT_TREE_SIZE,
};

const TREE_PALETTE_ID = 6; // ASM: PAL_OW_TREE from data/sprite_anims/oam.asm.
const TREE_PALETTE_FILES = new Set<string>([CUT_TREE_2BPP, CUT_GRASS_2BPP, HEADBUTT_TREE_2BPP]);

const FIELD_MOVE_PALETTE: [number, number, number, number][] = [
  [0, 0, 0, 0],
  [94, 171, 225, 255],
  [65, 125, 199, 255],
  [34, 81, 168, 255],
];

const FIELD_MOVE_GRAYSCALE_PALETTE: [number, number, number, number][] = [
  [255, 255, 255, 255],
  [170, 170, 170, 255],
  [85, 85, 85, 255],
  [0, 0, 0, 255],
];

type CutLeafAnimState = {
  var1: number;
  var2: number;
};

type FlySpriteAnimState = {
  x: number;
  y: number;
  xOffset: number;
  var2: number;
  var3: number;
  var4: number;
};

type FlyLeafAnimState = {
  x: number;
  y: number;
  xOffset: number;
  var1: number;
};

type OamEntry = {
  x_tile: number;
  y_tile: number;
  x_pixel: number;
  y_pixel: number;
  tile: number;
};

const OAM_LEAF: OamEntry[] = [{ x_tile: -1, y_tile: -1, x_pixel: 4, y_pixel: 4, tile: 0 }];

const OAM_TREE: OamEntry[] = [
  { x_tile: -1, y_tile: -1, x_pixel: 0, y_pixel: 0, tile: 0 },
  { x_tile: 0, y_tile: -1, x_pixel: 0, y_pixel: 0, tile: 1 },
  { x_tile: -1, y_tile: 0, x_pixel: 0, y_pixel: 0, tile: 2 },
  { x_tile: 0, y_tile: 0, x_pixel: 0, y_pixel: 0, tile: 3 },
];

const OAM_CUT_TREE_2: OamEntry[] = [
  { x_tile: -2, y_tile: -1, x_pixel: 6, y_pixel: 0, tile: 0 },
  { x_tile: 0, y_tile: -1, x_pixel: 2, y_pixel: 0, tile: 1 },
  { x_tile: -2, y_tile: 0, x_pixel: 6, y_pixel: 0, tile: 2 },
  { x_tile: 0, y_tile: 0, x_pixel: 2, y_pixel: 0, tile: 3 },
];

const OAM_CUT_TREE_3: OamEntry[] = [
  { x_tile: -2, y_tile: -1, x_pixel: 4, y_pixel: 0, tile: 0 },
  { x_tile: 0, y_tile: -1, x_pixel: 4, y_pixel: 0, tile: 1 },
  { x_tile: -2, y_tile: 0, x_pixel: 4, y_pixel: 0, tile: 2 },
  { x_tile: 0, y_tile: 0, x_pixel: 4, y_pixel: 0, tile: 3 },
];

const OAM_CUT_TREE_4: OamEntry[] = [
  { x_tile: -2, y_tile: -1, x_pixel: 0, y_pixel: 0, tile: 0 },
  { x_tile: 1, y_tile: -1, x_pixel: 0, y_pixel: 0, tile: 1 },
  { x_tile: -2, y_tile: 0, x_pixel: 0, y_pixel: 0, tile: 2 },
  { x_tile: 1, y_tile: 0, x_pixel: 0, y_pixel: 0, tile: 3 },
];

const offset_oam_entries = (entries: OamEntry[], tile_offset: number): OamEntry[] => {
  return entries.map((entry) => ({
    ...entry,
    tile: entry.tile + tile_offset,
  }));
};

export class FieldMoveAnimationLibrary {
  private asset_dir: string;
  private cache: Map<string, Surface> = new Map();
  private tile_cache: Map<string, Surface[]> = new Map();
  private definitions: Map<string, FieldMoveAnimationDefinition> = new Map();
  private readonly palette_manager: NpcPaletteManager;
  private time_of_day: string;
  private fly_player_surface!: Surface;
  private fly_leaf_tile!: Surface;

  constructor({ time_of_day = null }: { time_of_day?: string | null } = {}) {
    this.asset_dir = getAssetPath("gfx", "overworld");
    this.palette_manager = new NpcPaletteManager();
    this.time_of_day = this.palette_manager.normalise_time_of_day(time_of_day);
    this.init();
  }

  private init(): void {
    this.fly_player_surface = this.create_fly_player_surface();
    const leaf_tiles = this.load_tiles_from_2bpp(CUT_GRASS_2BPP);
    if (!leaf_tiles.length) {
      throw new Error("Cut grass tileset contains no tiles.");
    }
    this.fly_leaf_tile = leaf_tiles[0];
    this.build_definitions();
  }

  public get(animation: string, variant?: string | null): FieldMoveAnimationDefinition {
    const key = this.resolve_name(animation, variant ?? null);
    const definition = this.definitions.get(key);
    if (!definition) {
      throw new Error(`No field-move animation registered for '${key}'.`);
    }
    return definition;
  }

  public set_time_of_day(time_of_day?: string | null): void {
    const normalized = this.palette_manager.normalise_time_of_day(time_of_day);
    if (normalized === this.time_of_day) {
      return;
    }
    this.time_of_day = normalized;
    this.cache.clear();
    this.tile_cache.clear();
    this.definitions.clear();
    this.init();
  }

  private resolve_name(animation: string, variant?: string | null): string {
    if (variant) {
      return `${animation.toLowerCase()}_${variant.toLowerCase()}`;
    }
    return animation.toLowerCase();
  }

  private build_definitions(): void {
    const base_anchor = CUT_HEADBUTT_PIXEL_FACING.down;
    const anchors = CUT_HEADBUTT_PIXEL_FACING;
    this.definitions.set(
      "cut_tree",
      new FieldMoveAnimationDefinition("cut_tree", this.build_cut_tree_frames(), {
        anchor: base_anchor,
        directional_anchors: anchors,
      })
    );
    this.definitions.set(
      "cut_grass",
      new FieldMoveAnimationDefinition("cut_grass", this.build_cut_grass_frames("down", 1, 1), {
        anchor: [0, 0],
      })
    );
    this.definitions.set(
      "headbutt_headbutt",
      new FieldMoveAnimationDefinition("headbutt_headbutt", this.build_headbutt_frames(), {
        anchor: base_anchor,
        directional_anchors: anchors,
      })
    );
    this.definitions.set(
      "whirlpool_whirlpool",
      new FieldMoveAnimationDefinition("whirlpool_whirlpool", this.build_whirlpool_frames(), {
        anchor: [Math.trunc(METATILE_SIZE / 2), Math.trunc(METATILE_SIZE / 2)],
      })
    );
    this.definitions.set(
      "fly_from",
      new FieldMoveAnimationDefinition("fly_from", this.build_fly_from_frames(), {
        anchor: [8, 24],
      })
    );
    this.definitions.set(
      "fly_to",
      new FieldMoveAnimationDefinition("fly_to", this.build_fly_to_frames(), {
        anchor: [8, 24],
      })
    );
  }

  private build_cut_tree_frames(): FieldMoveAnimationFrame[] {
    // ASM: data/sprite_anims/framesets.asm::Frameset_CutTree + oam.asm::OAMData_Tree/CutTree*.
    const tiles = this.load_tiles_from_2bpp(CUT_TREE_2BPP);
    const tree = this.build_oam_surface(tiles, OAM_TREE);
    const cut2 = this.build_oam_surface(tiles, OAM_CUT_TREE_2);
    const cut3 = this.build_oam_surface(tiles, OAM_CUT_TREE_3);
    const cut4 = this.build_oam_surface(tiles, OAM_CUT_TREE_4);
    const treeLayer = new FieldMoveAnimationLayer(tree.surface, tree.offset_x, tree.offset_y);
    const cut2Layer = new FieldMoveAnimationLayer(cut2.surface, cut2.offset_x, cut2.offset_y);
    const cut3Layer = new FieldMoveAnimationLayer(cut3.surface, cut3.offset_x, cut3.offset_y);
    const cut4Layer = new FieldMoveAnimationLayer(cut4.surface, cut4.offset_x, cut4.offset_y);
    return [
      new FieldMoveAnimationFrame(2, [treeLayer]),
      new FieldMoveAnimationFrame(16, [cut2Layer]),
      new FieldMoveAnimationFrame(1, [cut2Layer]),
      new FieldMoveAnimationFrame(1, [cut3Layer]),
      new FieldMoveAnimationFrame(1, [cut3Layer]),
      new FieldMoveAnimationFrame(1, [cut4Layer]),
      new FieldMoveAnimationFrame(10, []),
    ];
  }

  public build_cut_grass_definition(
    direction: string,
    metatile_x: number,
    metatile_y: number
  ): FieldMoveAnimationDefinition {
    const frames = this.build_cut_grass_frames(direction, metatile_x, metatile_y);
    return new FieldMoveAnimationDefinition("cut_grass", frames, { anchor: [0, 0] });
  }

  private build_cut_grass_frames(
    direction: string,
    metatile_x: number,
    metatile_y: number
  ): FieldMoveAnimationFrame[] {
    // ASM: engine/events/field_moves.asm::Cut_GetLeafSpawnCoords + engine/sprite_anims/functions.asm::SpriteAnimFunc_CutLeaves.
    const tiles = this.load_tiles_from_2bpp(CUT_GRASS_2BPP);
    const leafSprite = this.build_oam_surface(tiles, OAM_LEAF);
    const [player_x, player_y] = this.infer_cut_player_tile(direction, metatile_x, metatile_y);
    const [base_x, base_y] = cut_get_leaf_spawn_coords(direction, player_x, player_y);
    const leafStates: CutLeafAnimState[] = [
      { var1: 0x00, var2: 0x0000 },
      { var1: 0x10, var2: 0x0000 },
      { var1: 0x20, var2: 0x0000 },
      { var1: 0x30, var2: 0x0000 },
    ];
    const frames: FieldMoveAnimationFrame[] = [];
    for (let tick = 0; tick < 32; tick += 1) {
      const layers: FieldMoveAnimationLayer[] = [];
      for (const state of leafStates) {
        state.var2 = (state.var2 + 0x80) & 0xffff;
        const amplitude = (state.var2 >> 8) & 0xff;
        const angle = state.var1 & 0xff;
        state.var1 = (state.var1 + 3) & 0xff;
        const y_offset = sine(angle, amplitude);
        const x_offset = cosine(angle, amplitude);
        layers.push(
          new FieldMoveAnimationLayer(
            leafSprite.surface,
            base_x + x_offset + leafSprite.offset_x,
            base_y + y_offset + leafSprite.offset_y
          )
        );
      }
      frames.push(new FieldMoveAnimationFrame(1, layers));
    }
    return frames;
  }

  private build_headbutt_frames(): FieldMoveAnimationFrame[] {
    // ASM: data/sprite_anims/framesets.asm::Frameset_HeadbuttTree + oam.asm::OAMData_Tree.
    const tiles = this.load_tiles_from_2bpp(HEADBUTT_TREE_2BPP);
    const tree = this.build_oam_surface(tiles, OAM_TREE);
    const headbutt = this.build_oam_surface(tiles, offset_oam_entries(OAM_TREE, 4));
    const headbutt_flipped = this.flip_surface_horizontal(headbutt.surface);
    const treeLayer = new FieldMoveAnimationLayer(tree.surface, tree.offset_x, tree.offset_y);
    const headbuttLayer = new FieldMoveAnimationLayer(headbutt.surface, headbutt.offset_x, headbutt.offset_y);
    const headbuttFlipLayer = new FieldMoveAnimationLayer(headbutt_flipped, headbutt.offset_x, headbutt.offset_y);
    const layers = [treeLayer, headbuttLayer, treeLayer, headbuttFlipLayer];
    const frames: FieldMoveAnimationFrame[] = [];
    let frame_index = -1;
    let duration = 0;
    const advance = (): number => {
      if (duration > 0) {
        duration -= 1;
        return frame_index;
      }
      frame_index = (frame_index + 1) % layers.length;
      duration = 2;
      return frame_index;
    };
    advance(); // Prime state to match the pre-loop DoNextFrame call.
    for (let tick = 0; tick < HEADBUTT_SHAKE_FRAMES; tick += 1) {
      const layer = layers[advance()];
      frames.push(new FieldMoveAnimationFrame(1, [layer]));
    }
    return frames;
  }

  private build_whirlpool_frames(): FieldMoveAnimationFrame[] {
    const animation = new WhirlpoolTileAnimation();
    return [new FieldMoveAnimationFrame(1, [new FieldMoveAnimationLayer(() => animation.current_surface())])];
  }

  private infer_cut_player_tile(
    direction: string,
    metatile_x: number,
    metatile_y: number
  ): [number, number] {
    const normalized = direction.toLowerCase();
    if (normalized === "down") {
      return [metatile_x, metatile_y - 1];
    }
    if (normalized === "up") {
      return [metatile_x, metatile_y + 1];
    }
    if (normalized === "left") {
      return [metatile_x + 1, metatile_y];
    }
    if (normalized === "right") {
      return [metatile_x - 1, metatile_y];
    }
    return [metatile_x, metatile_y];
  }

  private async tree_anchor(filename: string): Promise<[number, number]> {
    const surface = await this.load_surface_for_anchor(filename);
    const [width, height] = surface.get_size();
    return [Math.trunc(width / 2), height];
  }

  private async load_surface_for_anchor(filename: string): Promise<Surface> {
    if (filename.endsWith(".2bpp")) {
      const size = SURFACE_SPECS[filename];
      if (!size) {
        throw new Error(`Unknown dimensions for ${filename} when computing anchor.`);
      }
      return this.load_2bpp_surface(filename, size[0], size[1]);
    }
    return await this.load_surface(filename);
  }

  private load_tiles_from_2bpp(filename: string): Surface[] {
    const cache_key = this._tile_cache_key(filename);
    const cached = this.tile_cache.get(cache_key);
    if (cached) {
      return cached;
    }
    const file_path = path.join(this.asset_dir, filename);
    if (!fs.existsSync(file_path)) {
      throw new Error(`Missing field-move tileset ${file_path}`);
    }
    const raw = fs.readFileSync(file_path);
    let tiles: Surface[];
    if (TREE_PALETTE_FILES.has(filename)) {
      const grayscale = decode_2bpp_tiles(raw, FIELD_MOVE_GRAYSCALE_PALETTE);
      tiles = this.palette_manager.apply_many(grayscale, TREE_PALETTE_ID, this.time_of_day);
    } else {
      tiles = decode_2bpp_tiles(raw);
    }
    this.tile_cache.set(cache_key, tiles);
    return tiles;
  }

  private load_2bpp_surface(filename: string, width: number, height: number): Surface {
    const cache_key = this._tile_cache_key(filename);
    const cached = this.cache.get(cache_key);
    if (cached) {
      return cached;
    }
    const columns = Math.trunc(width / TILE_SIZE);
    if (columns <= 0) {
      throw new Error(`Invalid surface width ${width} for ${filename}`);
    }
    const tiles = this.load_tiles_from_2bpp(filename);
    const rows = Math.trunc(height / TILE_SIZE);
    const expected = columns * rows;
    if (expected > tiles.length) {
      throw new Error(`Expected ${expected} tiles for ${filename}, got ${tiles.length}`);
    }
    const surface = new gameEngine.Surface(width, height);
    for (let index = 0; index < expected; index += 1) {
      const tile = tiles[index];
      const x = (index % columns) * TILE_SIZE;
      const y = Math.trunc(index / columns) * TILE_SIZE;
      surface.blit(tile, [x, y]);
    }
    this.cache.set(cache_key, surface);
    return surface;
  }

  private _tile_cache_key(filename: string): string {
    if (!TREE_PALETTE_FILES.has(filename)) {
      return filename;
    }
    return `${filename}:${this.time_of_day}`;
  }

  private build_oam_surface(
    tiles: Surface[],
    entries: OamEntry[]
  ): { surface: Surface; offset_x: number; offset_y: number } {
    let min_x = 0;
    let min_y = 0;
    let max_x = 0;
    let max_y = 0;
    const positioned = entries.map((entry) => {
      const x = entry.x_tile * TILE_SIZE + entry.x_pixel;
      const y = entry.y_tile * TILE_SIZE + entry.y_pixel;
      min_x = Math.min(min_x, x);
      min_y = Math.min(min_y, y);
      max_x = Math.max(max_x, x + TILE_SIZE);
      max_y = Math.max(max_y, y + TILE_SIZE);
      return { x, y, tile: entry.tile };
    });
    const width = Math.max(1, max_x - min_x);
    const height = Math.max(1, max_y - min_y);
    const surface = new gameEngine.Surface(width, height);
    for (const entry of positioned) {
      const tile = tiles[entry.tile];
      if (!tile) {
        throw new Error(`Missing tile ${entry.tile} in field move OAM surface.`);
      }
      surface.blit(tile, [entry.x - min_x, entry.y - min_y]);
    }
    return { surface, offset_x: min_x, offset_y: min_y };
  }

  private flip_surface_horizontal(source: Surface): Surface {
    const width = source.get_width();
    const height = source.get_height();
    const surface = new gameEngine.Surface(width, height);
    const src_ctx = source.getContext();
    const dst_ctx = surface.getContext();
    if (!src_ctx || !dst_ctx) {
      throw new Error("Failed to access headbutt surface context.");
    }
    const image = src_ctx.getImageData(0, 0, width, height);
    const flipped = dst_ctx.createImageData(width, height);
    const src = image.data;
    const dest = flipped.data;
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const src_index = (y * width + x) * 4;
        const dest_index = (y * width + (width - 1 - x)) * 4;
        dest[dest_index] = src[src_index] ?? 0;
        dest[dest_index + 1] = src[src_index + 1] ?? 0;
        dest[dest_index + 2] = src[src_index + 2] ?? 0;
        dest[dest_index + 3] = src[src_index + 3] ?? 0;
      }
    }
    dst_ctx.putImageData(flipped, 0, 0);
    return surface;
  }

  private create_fly_player_surface(): Surface {
    // ASM loads the flying party member icon into FIELDMOVE_FLY. The renderer uses
    // the bird icon as the deterministic visible stand-in when party species is not
    // available to this stateless animation library.
    const file_path = getAssetPath("gfx", "icons", FLY_ICON_2BPP);
    if (!fs.existsSync(file_path)) {
      throw new Error(`Missing fly icon tileset ${file_path}`);
    }
    const tiles = decode_2bpp_tiles(fs.readFileSync(file_path));
    if (tiles.length < 4) {
      throw new Error(`Expected at least 4 tiles for ${FLY_ICON_2BPP}, got ${tiles.length}`);
    }
    const surface = new gameEngine.Surface(16, 16);
    for (let index = 0; index < 4; index += 1) {
      const x = (index % 2) * TILE_SIZE;
      const y = Math.trunc(index / 2) * TILE_SIZE;
      surface.blit(tiles[index]!, [x, y]);
    }
    return surface;
  }

  private build_fly_from_frames(): FieldMoveAnimationFrame[] {
    // ASM: engine/events/field_moves.asm::FlyFromAnim/FlyFunction_FrameTimer
    // and engine/sprite_anims/functions.asm::SpriteAnimFunc_FlyFrom/FlyLeaf.
    const frames: FieldMoveAnimationFrame[] = [];
    const player: FlySpriteAnimState = {
      x: 10 * TILE_SIZE + 4,
      y: 10 * TILE_SIZE,
      xOffset: 0,
      var2: 0,
      var3: 0,
      var4: 0,
    };
    const origin: [number, number] = [player.x, player.y];
    this.build_fly_frames(frames, player, 128, "from", origin);
    return frames;
  }

  private build_fly_to_frames(): FieldMoveAnimationFrame[] {
    // ASM: engine/events/field_moves.asm::FlyToAnim/FlyFunction_FrameTimer
    // and engine/sprite_anims/functions.asm::SpriteAnimFunc_FlyTo/FlyLeaf.
    const frames: FieldMoveAnimationFrame[] = [];
    const player: FlySpriteAnimState = {
      x: 10 * TILE_SIZE + 4,
      y: 31 * TILE_SIZE,
      xOffset: 0,
      var2: 0,
      var3: 0,
      var4: 11 * TILE_SIZE,
    };
    const origin: [number, number] = [player.x, 10 * TILE_SIZE + 4];
    this.build_fly_frames(frames, player, 64, "to", origin);
    return frames;
  }

  private build_fly_frames(
    frames: FieldMoveAnimationFrame[],
    player: FlySpriteAnimState,
    frame_counter: number,
    variant: "from" | "to",
    origin: [number, number]
  ): void {
    const leaves: FlyLeafAnimState[] = [];
    let leaf_counter = 0;
    while (true) {
      const previous_leaf_counter = leaf_counter;
      leaf_counter = (leaf_counter + 1) & 0xff;
      if ((previous_leaf_counter & 0x07) === 0) {
        const selector = leaf_counter & ((6 * TILE_SIZE) >> 1);
        leaves.push({
          x: player.x,
          y: player.y + ((selector << 1) + 8 * TILE_SIZE),
          xOffset: 0,
          var1: 0,
        });
      }

      if (variant === "from") {
        this.advance_fly_from_player(player);
      } else {
        this.advance_fly_to_player(player);
      }
      this.advance_fly_leaves(leaves);

      frames.push(new FieldMoveAnimationFrame(1, this.build_fly_layers(player, leaves, origin)));
      if (frame_counter === 0) {
        break;
      }
      frame_counter = (frame_counter - 1) & 0xff;
    }
  }

  private advance_fly_from_player(player: FlySpriteAnimState): void {
    if (player.y === 0) {
      return;
    }
    const previous_var2 = player.var2;
    player.var2 = (player.var2 + 1) & 0xff;
    if (previous_var2 < 0x40) {
      return;
    }
    player.y -= 2;
    if (player.var4 < 0x40) {
      player.var4 = (player.var4 + 8) & 0xff;
    }
    const angle = player.var3;
    player.var3 = (player.var3 + 1) & 0xff;
    player.xOffset = cosine(angle, player.var4);
  }

  private advance_fly_to_player(player: FlySpriteAnimState): void {
    if (player.y === 10 * TILE_SIZE + 4) {
      return;
    }
    player.y += 2;
    const amplitude = player.var4;
    if (player.var4 !== 0) {
      player.var4 = (player.var4 - 2) & 0xff;
    }
    const angle = player.var3;
    player.var3 = (player.var3 + 1) & 0xff;
    player.xOffset = cosine(angle, amplitude);
  }

  private advance_fly_leaves(leaves: FlyLeafAnimState[]): void {
    for (let index = leaves.length - 1; index >= 0; index -= 1) {
      const leaf = leaves[index]!;
      if ((leaf.x & 0xff) >= ((-9 * TILE_SIZE) & 0xff)) {
        leaves.splice(index, 1);
        continue;
      }
      leaf.x += 2;
      leaf.y -= 1;
      const angle = leaf.var1;
      leaf.var1 = (leaf.var1 + 1) & 0xff;
      leaf.xOffset = cosine(angle, 0x40);
    }
  }

  private build_fly_layers(
    player: FlySpriteAnimState,
    leaves: FlyLeafAnimState[],
    origin: [number, number]
  ): FieldMoveAnimationLayer[] {
    const [origin_x, origin_y] = origin;
    const layers = leaves.map(
      (leaf) =>
        new FieldMoveAnimationLayer(
          this.fly_leaf_tile,
          leaf.x + leaf.xOffset - origin_x,
          leaf.y - origin_y
        )
    );
    layers.push(
      new FieldMoveAnimationLayer(
        this.fly_player_surface,
        player.x + player.xOffset - origin_x,
        player.y - origin_y
      )
    );
    return layers;
  }

  private async load_surface(filename: string): Promise<Surface> {
    const cached = this.cache.get(filename);
    if (cached) {
      return cached;
    }
    const file_path = path.join(this.asset_dir, filename);
    if (!fs.existsSync(file_path)) {
      throw new Error(`Missing overworld field-move sprite: ${file_path}`);
    }
    const source = await gameEngine.image.load(file_path);
    const surface = new gameEngine.Surface(source.get_width(), source.get_height());
    surface.blit(source, [0, 0]);
    this.cache.set(filename, surface);
    return surface;
  }
}

export function decode_2bpp_tiles(
  data: Buffer,
  palette: [number, number, number, number][] = FIELD_MOVE_PALETTE
): Surface[] {
  if (data.length % 16 !== 0) {
    throw new Error("Unexpected 2bpp payload size for whirlpool tiles.");
  }
  const tiles: Surface[] = [];
  for (let offset = 0; offset < data.length; offset += 16) {
    const tile = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
    for (let row = 0; row < TILE_SIZE; row += 1) {
      const lo = data[offset + row * 2];
      const hi = data[offset + row * 2 + 1];
      for (let col = 0; col < TILE_SIZE; col += 1) {
        const mask = 1 << (7 - col);
        const index = ((hi & mask) ? 2 : 0) | ((lo & mask) ? 1 : 0);
        const color = palette[index];
        tile.set_at([col, row], color);
      }
    }
    tiles.push(tile);
  }
  return tiles;
}

function compose_whirlpool_frame(tiles: Surface[]): Surface {
  const width = TILE_SIZE * METATILE_WIDTH;
  const height = TILE_SIZE * METATILE_WIDTH;
  const columns = METATILE_WIDTH;
  const rows = METATILE_WIDTH;
  const expected_tiles = columns * rows;
  if (tiles.length !== expected_tiles) {
    throw new Error(`Expected ${expected_tiles} whirlpool tiles, got ${tiles.length}`);
  }
  const frame = new gameEngine.Surface(width, height);
  tiles.forEach((tile, index) => {
    const x = (index % columns) * TILE_SIZE;
    const y = Math.trunc(index / columns) * TILE_SIZE;
    frame.blit(tile, [x, y]);
  });
  return frame;
}

export class WhirlpoolTileAnimation {
  private static tileset_cache: Map<string, Surface[]> = new Map();

  private readonly frames: Surface[];
  private static readonly TILESET_NAME = "johto";
  private static readonly WHIRLPOOL_METATILE_ID = 0x07;
  private static readonly FRAME_FILES = ["1.2bpp", "2.2bpp", "3.2bpp", "4.2bpp"];
  private static readonly FRAME_COUNT = 4;

  constructor() {
    const animated_tiles = WhirlpoolTileAnimation.load_animated_tiles();
    const layout = WhirlpoolTileAnimation.load_whirlpool_layout();
    const base_tiles = WhirlpoolTileAnimation.load_tileset_tiles(
      WhirlpoolTileAnimation.TILESET_NAME
    );
    const stubTileset = base_tiles.length <= 4;
    const frames: Surface[] = [];
    for (let frame_index = 0; frame_index < WhirlpoolTileAnimation.FRAME_COUNT; frame_index += 1) {
      const composed_tiles: Surface[] = [];
      for (const tile_id of layout) {
        const animated = animated_tiles.get(tile_id);
        if (animated) {
          if (!animated[frame_index]) {
            throw new Error(`Whirlpool tile ${tile_id.toString(16)} missing frame ${frame_index}`);
          }
          composed_tiles.push(animated[frame_index]);
          continue;
        }
        if (tile_id < 0 || tile_id >= base_tiles.length) {
          if (stubTileset) {
            throw new Error(
              `Whirlpool tileset '${WhirlpoolTileAnimation.TILESET_NAME}' did not load enough base tiles for ASM-faithful composition.`
            );
          }
          throw new Error(
            `Whirlpool metatile references tile ${tile_id.toString(16)} outside tileset '${WhirlpoolTileAnimation.TILESET_NAME}'.`
          );
        }
        composed_tiles.push(base_tiles[tile_id]);
      }
      frames.push(compose_whirlpool_frame(composed_tiles));
    }
    this.frames = frames;
  }

  private static load_animated_tiles(): Map<number, Surface[]> {
    const base = getAssetPath("gfx", "tilesets", "whirlpool");
    const animated_tiles = new Map<number, Surface[]>();
    WHIRLPOOL_TILE_INDEXES.forEach((tile_index, idx) => {
      const filename = WhirlpoolTileAnimation.FRAME_FILES[idx];
      const file_path = path.join(base, filename);
      if (!fs.existsSync(file_path)) {
        throw new Error(`Missing whirlpool tileset ${file_path}`);
      }
      const tiles = decode_2bpp_tiles(fs.readFileSync(file_path));
      if (tiles.length < WhirlpoolTileAnimation.FRAME_COUNT) {
        throw new Error(
          `Whirlpool source ${filename} contains ${tiles.length} tiles; ${WhirlpoolTileAnimation.FRAME_COUNT} required for animation.`
        );
      }
      animated_tiles.set(tile_index, tiles.slice(0, WhirlpoolTileAnimation.FRAME_COUNT));
    });
    return animated_tiles;
  }

  private static load_whirlpool_layout(): number[] {
    const layout_path = getTilesetMetatilesPath(WhirlpoolTileAnimation.TILESET_NAME);
    if (!fs.existsSync(layout_path)) {
      throw new Error(`Missing ${WhirlpoolTileAnimation.TILESET_NAME} metatile data at ${layout_path}`);
    }
    const data = fs.readFileSync(layout_path);
    const bytes_per_metatile = METATILE_WIDTH * METATILE_WIDTH;
    const start = WhirlpoolTileAnimation.WHIRLPOOL_METATILE_ID * bytes_per_metatile;
    const end = start + bytes_per_metatile;
    if (end > data.length) {
      throw new Error(
        `Metatile ${WhirlpoolTileAnimation.WHIRLPOOL_METATILE_ID.toString(16)} exceeds ${WhirlpoolTileAnimation.TILESET_NAME} metatile payload (${data.length} bytes).`
      );
    }
    const chunk = data.subarray(start, end);
    if (chunk.length !== bytes_per_metatile) {
      throw new Error("Incomplete whirlpool metatile payload.");
    }
    return Array.from(chunk);
  }

  private static load_tileset_tiles(tileset_name: string): Surface[] {
    const cached = WhirlpoolTileAnimation.tileset_cache.get(tileset_name);
    if (cached) {
      return cached;
    }
    const tileset_path = getAssetPath("gfx", "tilesets", `${tileset_name}.png`);
    const source =
      typeof gameEngine.image.loadSync === "function"
        ? gameEngine.image.loadSync(tileset_path)
        : null;
    if (!source) {
      throw new Error(`Tileset ${tileset_name} must be preloaded before use.`);
    }
    const width = source.get_width();
    const height = source.get_height();
    if (width % TILE_SIZE || height % TILE_SIZE) {
      throw new Error(`Tileset ${tileset_name} has unexpected dimensions ${width}x${height}.`);
    }
    const tiles: Surface[] = [];
    for (let y = 0; y < height; y += TILE_SIZE) {
      for (let x = 0; x < width; x += TILE_SIZE) {
        const tile = source.subsurface(new gameEngine.Rect(x, y, TILE_SIZE, TILE_SIZE));
        tiles.push(tile);
      }
    }
    WhirlpoolTileAnimation.tileset_cache.set(tileset_name, tiles);
    return tiles;
  }

  public current_surface(): Surface {
    return this.frames[tile_animation_timer_instance.whirlpool_phase];
  }
}

export class FieldMoveAnimationController {
  private library: FieldMoveAnimationLibrary;
  private players: Map<string, FieldMoveAnimationPlayer> = new Map();
  private readonly drawScratch: FieldMoveAnimationPlayer[] = [];

  constructor(
    game_state: GameState | null,
    { time_of_day = null }: { time_of_day?: string | null } = {}
  ) {
    if (game_state) {
      tile_animation_timer_instance.bind(game_state);
    }
    this.library = new FieldMoveAnimationLibrary({ time_of_day });
  }

  public set_time_of_day(time_of_day?: string | null): void {
    this.library.set_time_of_day(time_of_day);
  }

  public start(
    animation: string,
    variant: string,
    metatile_x: number,
    metatile_y: number,
    { direction = null }: { direction?: string | null } = {}
  ): void {
    const normalized_direction = (direction ?? "down").toLowerCase();
    const normalized_animation = animation.toLowerCase();
    const normalized_variant = variant.toLowerCase();
    const definition =
      normalized_animation === "cut" && normalized_variant === "grass"
        ? this.library.build_cut_grass_definition(normalized_direction, metatile_x, metatile_y)
        : this.library.get(animation, variant);
    const world_x = metatile_x * METATILE_SIZE;
    const world_y = metatile_y * METATILE_SIZE;
    const norm_variant = normalized_variant || normalized_animation;
    const key = `${normalized_animation}-${norm_variant}-${metatile_x}-${metatile_y}-${normalized_direction}`;
    this.players.set(
      key,
      new FieldMoveAnimationPlayer(definition, world_x, world_y, normalized_direction)
    );
  }

  public complete(animation: string, metatile_x: number, metatile_y: number): void {
    const normalized_animation = animation.toLowerCase();
    const keys: string[] = [];
    for (const key of this.players.keys()) {
      const [anim, , x, y] = key.split("-");
      if (anim === normalized_animation && Number(x) === metatile_x && Number(y) === metatile_y) {
        keys.push(key);
      }
    }
    keys.forEach((key) => this.players.delete(key));
  }

  public advance(): void {
    const expired: string[] = [];
    for (const [key, player] of this.players.entries()) {
      player.advance();
      if (player.is_finished) {
        expired.push(key);
      }
    }
    expired.forEach((key) => this.players.delete(key));
  }

  public draw(screen: Surface, camera_x: number, camera_y: number, origin: [number, number]): void {
    if (!this.players.size) {
      return;
    }
    const [origin_x, origin_y] = origin;
    let drawCount = 0;
    for (const player of this.players.values()) {
      this.drawScratch[drawCount] = player;
      drawCount += 1;
    }
    this.drawScratch.length = drawCount;
    if (drawCount > 1) {
      this.drawScratch.sort((a, b) => a.world_y - b.world_y);
    }
    for (let index = 0; index < drawCount; index += 1) {
      const player = this.drawScratch[index]!;
      const comp_x = player.world_x + origin_x;
      const comp_y = player.world_y + origin_y;
      const screen_x = comp_x - camera_x;
      const screen_y = comp_y - camera_y;
      player.draw(screen, screen_x, screen_y);
    }
  }
}

export class FieldMoveVramLoader {
  private game_state: GameState;
  private current_frame: number = -1;
  private whirlpool_frames: Map<number, number[][]> = new Map();
  private whirlpool_surfaces: Map<number, Surface[]> = new Map();
  private overworld_base: string;

  private static readonly WHIRLPOOL_FILES = ["1.2bpp", "2.2bpp", "3.2bpp", "4.2bpp"];
  private static readonly WHIRLPOOL_DEST_INDEXES = WHIRLPOOL_TILE_INDEXES;
  private static readonly TILE_BYTES = 16;
  private static readonly CUT_GFX: Array<[string, string, number, number]> = [
    ["cut_grass.2bpp", "vTiles0", 0x80, 4],
    ["cut_tree.2bpp", "vTiles0", 0x84, 4],
  ];
  private static readonly HEADBUTT_GFX: Array<[string, string, number, number]> = [
    ["cut_grass.2bpp", "vTiles0", 0x80, 4],
    ["headbutt_tree.2bpp", "vTiles0", 0x84, 8],
  ];

  constructor(game_state: GameState) {
    this.game_state = game_state;
    this.overworld_base = getAssetPath("gfx", "overworld");
    this.load_whirlpool_tiles();
  }

  public request_cut_tiles(): void {
    FieldMoveVramLoader.CUT_GFX.forEach(([filename, block, index, count]) => {
      this.request_2bpp_tiles(filename, block, index, count);
    });
  }

  public request_headbutt_tiles(): void {
    FieldMoveVramLoader.HEADBUTT_GFX.forEach(([filename, block, index, count]) => {
      this.request_2bpp_tiles(filename, block, index, count);
    });
  }

  private request_2bpp_tiles(filename: string, block_name: string, start_index: number, tile_count: number): void {
    const file_path = path.join(this.overworld_base, filename);
    if (!fs.existsSync(file_path)) {
      throw new Error(`Missing field move tileset ${file_path}`);
    }
    const raw = fs.readFileSync(file_path);
    const expected = tile_count * FieldMoveVramLoader.TILE_BYTES;
    if (raw.length !== expected) {
      throw new Error(`${filename} contains ${raw.length} bytes, expected ${expected}`);
    }
    const vram_manager = new VRAMManager(this.game_state.vram);
    const tile_block = new TileBlockManager(vram_manager.resolveTileBlock(block_name));
    this.validate_tile_range(tile_block, start_index, tile_count, filename);
    for (let offset = 0; offset < tile_count; offset += 1) {
      const chunk = Array.from(raw.subarray(offset * FieldMoveVramLoader.TILE_BYTES, (offset + 1) * FieldMoveVramLoader.TILE_BYTES));
      tile_block.writeTile(start_index + offset, chunk);
      this.verify_tile_bytes(tile_block, start_index + offset, chunk, filename);
    }
  }

  private validate_tile_range(tile_block: TileBlockManager, start_index: number, tile_count: number, source: string): void {
    const end_index = start_index + tile_count;
    if (end_index > TileBlockManager.TILE_COUNT) {
      throw new Error(
        `${source} would write tiles ${start_index.toString(16)}-${(end_index - 1).toString(16)}, but ${TileBlockManager.TILE_COUNT.toString(16)} tiles are available in ${source}`
      );
    }
  }

  private verify_tile_bytes(tile_block: TileBlockManager, tile_index: number, expected_bytes: number[], source: string): void {
    const written = tile_block.readTile(tile_index);
    if (written.length !== expected_bytes.length) {
      throw new Error(`VRAM verification failed for ${source} tile ${tile_index.toString(16)}`);
    }
    for (let i = 0; i < expected_bytes.length; i += 1) {
      if (written[i] !== expected_bytes[i]) {
        throw new Error(`VRAM verification failed for ${source} tile ${tile_index.toString(16)}`);
      }
    }
  }

  private load_whirlpool_tiles(): void {
    const base_path = getAssetPath("gfx", "tilesets", "whirlpool");
    FieldMoveVramLoader.WHIRLPOOL_DEST_INDEXES.forEach((dest_index, idx) => {
      const filename = FieldMoveVramLoader.WHIRLPOOL_FILES[idx];
      const file_path = path.join(base_path, filename);
      if (!fs.existsSync(file_path)) {
        throw new Error(`Missing whirlpool assets at ${file_path}`);
      }
      const data = fs.readFileSync(file_path);
      const tiles = decode_2bpp_tiles(data);
      const total_tiles = Math.trunc(data.length / FieldMoveVramLoader.TILE_BYTES);
      if (tiles.length < total_tiles) {
        throw new Error("Mismatch between decoded whirlpool tiles and raw data.");
      }
      if (total_tiles < 4) {
        throw new Error(`${filename} contains ${total_tiles} tiles, expected at least 4.`);
      }
      const tile_chunks: number[][] = [];
      for (let i = 0; i < total_tiles; i += 1) {
        const start = i * FieldMoveVramLoader.TILE_BYTES;
        const end = start + FieldMoveVramLoader.TILE_BYTES;
        tile_chunks.push(Array.from(data.subarray(start, end)));
      }
      const frames = tile_chunks.slice(0, 4);
      const surfaces = tiles.slice(0, 4);
      if (frames.length < 4 || surfaces.length < 4) {
        throw new Error(`Whirlpool asset ${filename} lacks 4 frames.`);
      }
      this.whirlpool_frames.set(dest_index, frames);
      this.whirlpool_surfaces.set(dest_index, surfaces);
    });
    this.write_all_whirlpool_tiles();
  }

  private write_all_whirlpool_tiles(): void {
    const vram_manager = new VRAMManager(this.game_state.vram);
    const tile_block = new TileBlockManager(vram_manager.resolveTileBlock("vTiles2"));
    for (const [tile_index, frames] of this.whirlpool_frames.entries()) {
      if (!frames.length) {
        continue;
      }
      this.assert_tile_index(tile_block, tile_index, "whirlpool_init");
      tile_block.writeTile(tile_index, frames[0]);
    }
  }

  public update_whirlpool_tiles(timer: number, { force = false }: { force?: boolean } = {}): boolean {
    const frame = timer & 0x03;
    if (!force && frame === this.current_frame) {
      return false;
    }
    this.current_frame = frame;
    const vram_manager = new VRAMManager(this.game_state.vram);
    const tile_block = new TileBlockManager(vram_manager.resolveTileBlock("vTiles2"));
    for (const [tile_index, frames] of this.whirlpool_frames.entries()) {
      this.assert_tile_index(tile_block, tile_index, "whirlpool_update");
      tile_block.writeTile(tile_index, frames[frame]);
    }
    return true;
  }

  public surface_for_whirlpool_frame(tile_index: number, frame_index: number): Surface | null {
    const frames = this.whirlpool_surfaces.get(tile_index) ?? null;
    if (!frames || frame_index < 0 || frame_index >= frames.length) {
      return null;
    }
    return frames[frame_index];
  }

  private assert_tile_index(tile_block: TileBlockManager, tile_index: number, source: string): void {
    if (tile_index < 0 || tile_index >= TileBlockManager.TILE_COUNT) {
      throw new Error(
        `${source} attempted to write tile ${tile_index.toString(16)} (max ${(TileBlockManager.TILE_COUNT - 1).toString(16)}).`
      );
    }
  }
}

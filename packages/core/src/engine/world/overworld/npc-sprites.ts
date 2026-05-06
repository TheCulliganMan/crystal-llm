// ASM mapping: pokecrystal_disassembly/data/sprites/sprites.asm (sprite frame layout + palettes).
import { gameEngine, Surface } from "@pokecrystal/core/ui/game-engine";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { SpriteAnimation } from "@pokecrystal/core/engine/systems/animation";
import { NpcPaletteManager } from "./palette";

export type DirectionFrames = Record<string, Surface[]>;

const ICON_SPRITE_PREFIX = "icon_";
const BIG_DOLL_TILE_SIZE = 8;
const SYMMETRIC_BIG_DOLL_SPRITES = new Set(["big_lapras", "big_snorlax"]);

const isSurfaceLike = (value: unknown): value is Surface =>
  Boolean(value && typeof value === "object" && typeof (value as Surface).blit === "function");

const isPromiseLike = (value: unknown): value is Promise<unknown> =>
  Boolean(value && typeof (value as Promise<unknown>).then === "function");

const convertSurface = (surface: Surface): Surface =>
  typeof surface.convert_alpha === "function" ? surface.convert_alpha() : surface;

class SpriteDefinition {
  public frames: DirectionFrames;
  public tinted: Map<string, DirectionFrames> = new Map();

  constructor(frames: DirectionFrames) {
    this.frames = frames;
  }
}

export class NpcSpriteCache {
  private frame_duration: number;
  private definitions: Map<string, SpriteDefinition> = new Map();
  private palette_manager: NpcPaletteManager;

  constructor(frame_duration: number = 10, { palette_manager = null }: { palette_manager?: NpcPaletteManager | null } = {}) {
    this.frame_duration = frame_duration;
    this.palette_manager = palette_manager ?? new NpcPaletteManager();
  }

  public instantiate(
    sprite_id: string,
    palette_id: number | null,
    time_of_day?: string | null
  ): Record<string, SpriteAnimation> {
    const definition = this.load(sprite_id);
    const palette_key = palette_id ?? 0;
    const time_key = this.palette_manager.normaliseTimeOfDay(time_of_day ?? null);
    const frames = this.resolve_frames(definition, palette_key, time_key);
    const animations: Record<string, SpriteAnimation> = {};
    for (const [direction, directional_frames] of Object.entries(frames)) {
      if (!directional_frames || directional_frames.length === 0) {
        throw new Error(
          `Sprite '${sprite_id}' produced no frames for direction '${direction}'.`
        );
      }
      animations[direction] = new SpriteAnimation(
        Array.from(directional_frames),
        this.frame_duration,
        false
      );
    }
    return animations;
  }

  private resolve_frames(definition: SpriteDefinition, palette_id: number, time_key: string): DirectionFrames {
    const cache_key = `${palette_id & 0xf}-${time_key}`;
    const cached = definition.tinted.get(cache_key);
    if (cached) {
      return cached;
    }
    const tinted: DirectionFrames = {};
    for (const [direction, frames] of Object.entries(definition.frames)) {
      tinted[direction] = this.palette_manager.apply_many(frames, palette_id, time_key);
    }
    definition.tinted.set(cache_key, tinted);
    return tinted;
  }

  private load(sprite_id: string): SpriteDefinition {
    const normalised = sprite_id.trim().toLowerCase();
    if (!normalised) {
      throw new Error("Sprite identifier may not be empty when loading NPC art.");
    }
    const cached = this.definitions.get(normalised);
    if (cached) {
      return cached;
    }
    const frames = this.load_directional_frames(normalised);
    const definition = new SpriteDefinition(frames);
    this.definitions.set(normalised, definition);
    return definition;
  }

  private load_directional_frames(sprite_id: string): DirectionFrames {
    const sheet = this.load_sheet(sprite_id);
    const [width, height] = sheet.get_size();
    if (width <= 0 || height <= 0) {
      throw new Error(`Sprite '${sprite_id}' has invalid dimensions ${width}x${height}.`);
    }
    if (SYMMETRIC_BIG_DOLL_SPRITES.has(sprite_id) && width === 16 && height === 32) {
      return this.map_frames(sprite_id, [this.compose_symmetric_big_doll_frame(sheet)]);
    }
    if (height % width !== 0) {
      throw new Error(
        `Sprite '${sprite_id}' height ${height} is not divisible by width ${width}; cannot derive square animation frames.`
      );
    }

    const frame_count = Math.trunc(height / width);
    const frames: Surface[] = [];
    for (let index = 0; index < frame_count; index += 1) {
      frames.push(this.extract_frame(sheet, width, index));
    }
    if (this.is_padded_static_sheet(frames)) {
      return this.map_frames(sprite_id, [frames[0]]);
    }
    return this.map_frames(sprite_id, frames);
  }

  private load_sheet(sprite_id: string): Surface {
    const sprite_path = sprite_id.startsWith(ICON_SPRITE_PREFIX)
      ? getAssetPath("gfx", "icons", `${sprite_id.slice(ICON_SPRITE_PREFIX.length)}.png`)
      : getAssetPath("gfx", "sprites", `${sprite_id}.png`);
    const loadSync = gameEngine.image.loadSync;
    if (typeof loadSync === "function") {
      const cached = loadSync(sprite_path);
      if (cached) {
        return convertSurface(cached);
      }
    }
    const loaded = gameEngine.image.load(sprite_path);
    if (isPromiseLike(loaded)) {
      throw new Error(
        "NPC sprite loading requires a synchronous image loader. Preload sprite assets first."
      );
    }
    if (!isSurfaceLike(loaded)) {
      throw new Error(`Failed to load sprite '${sprite_id}': invalid surface returned.`);
    }
    return convertSurface(loaded);
  }

  private extract_frame(sheet: Surface, frame_size: number, index: number): Surface {
    const rect = new gameEngine.Rect(0, index * frame_size, frame_size, frame_size);
    const frame = new gameEngine.Surface(frame_size, frame_size);
    frame.blit(sheet, [0, 0], rect);
    return frame;
  }

  private compose_symmetric_big_doll_frame(sheet: Surface): Surface {
    const frame = new gameEngine.Surface(32, 32);
    frame.fill([0, 0, 0, 0]);
    const entries: Array<[number, number, boolean, number]> = [
      [0, 0, false, 0],
      [0, 8, false, 1],
      [8, 0, false, 2],
      [8, 8, false, 3],
      [16, 0, false, 4],
      [16, 8, false, 5],
      [24, 0, false, 6],
      [24, 8, false, 7],
      [0, 24, true, 0],
      [0, 16, true, 1],
      [8, 24, true, 2],
      [8, 16, true, 3],
      [16, 24, true, 4],
      [16, 16, true, 5],
      [24, 24, true, 6],
      [24, 16, true, 7],
    ];
    for (const [destY, destX, flipX, tileId] of entries) {
      const sourceX = (tileId % 2) * BIG_DOLL_TILE_SIZE;
      const sourceY = Math.floor(tileId / 2) * BIG_DOLL_TILE_SIZE;
      const tile = new gameEngine.Surface(BIG_DOLL_TILE_SIZE, BIG_DOLL_TILE_SIZE);
      tile.blit(
        sheet,
        [0, 0],
        new gameEngine.Rect(sourceX, sourceY, BIG_DOLL_TILE_SIZE, BIG_DOLL_TILE_SIZE)
      );
      frame.blit(flipX ? gameEngine.transform.flip(tile, true, false) : tile, [destX, destY]);
    }
    return frame;
  }

  private is_uniform_frame(frame: Surface): boolean {
    const image = frame.getImageData();
    const data = image.data;
    if (data.length <= 4) {
      return true;
    }
    const r = data[0];
    const g = data[1];
    const b = data[2];
    const a = data[3];
    for (let index = 4; index < data.length; index += 4) {
      if (
        data[index] !== r ||
        data[index + 1] !== g ||
        data[index + 2] !== b ||
        data[index + 3] !== a
      ) {
        return false;
      }
    }
    return true;
  }

  private is_padded_static_sheet(frames: Surface[]): boolean {
    return (
      frames.length > 1 &&
      !this.is_uniform_frame(frames[0]) &&
      frames.slice(1).every((frame) => this.is_uniform_frame(frame))
    );
  }

  private stride_cycle(
    standing: Surface,
    walking: Surface,
    { mirror_walk }: { mirror_walk: boolean }
  ): Surface[] {
    let mirrored = walking;
    if (mirror_walk) {
      try {
        mirrored = gameEngine.transform.flip(walking, true, false);
      } catch {
        mirrored = walking;
      }
    }
    return [standing, walking, standing, mirrored];
  }

  private mirror_frames(frames: Iterable<InstanceType<typeof gameEngine.Surface>>): InstanceType<typeof gameEngine.Surface>[] {
    const mirrored: InstanceType<typeof gameEngine.Surface>[] = [];
    for (const frame of frames) {
      try {
        mirrored.push(gameEngine.transform.flip(frame, true, false));
      } catch {
        mirrored.push(frame);
      }
    }
    return mirrored;
  }

  private map_frames(
    sprite_id: string,
    frames: Iterable<InstanceType<typeof gameEngine.Surface>>
  ): DirectionFrames {
    const frame_list = Array.from(frames);
    const count = frame_list.length;
    let down: InstanceType<typeof gameEngine.Surface>[];
    let up: InstanceType<typeof gameEngine.Surface>[];
    let left: InstanceType<typeof gameEngine.Surface>[];

    if (sprite_id.startsWith(ICON_SPRITE_PREFIX)) {
      down = [...frame_list];
      up = [...frame_list];
      left = [...frame_list];
      return {
        down,
        up,
        left,
        right: [...frame_list],
      };
    }

    if (count === 6) {
      const [
        standing_down,
        standing_up,
        standing_left,
        walking_down,
        walking_up,
        walking_left,
      ] = frame_list;
      down = this.stride_cycle(standing_down, walking_down, { mirror_walk: true });
      up = this.stride_cycle(standing_up, walking_up, { mirror_walk: true });
      left = this.stride_cycle(standing_left, walking_left, { mirror_walk: false });
    } else if (count === 3) {
      down = [frame_list[0]];
      up = [frame_list[1]];
      left = [frame_list[2]];
    } else if (count === 2) {
      down = [frame_list[0]];
      up = [frame_list[1]];
      left = [frame_list[0]];
    } else if (count === 1) {
      down = [frame_list[0]];
      up = [frame_list[0]];
      left = [frame_list[0]];
    } else {
      throw new Error(
        `Sprite '${sprite_id}' has unsupported frame count ${count}. Only 1, 2, 3, or 6 frames are currently handled.`
      );
    }

    const right = this.mirror_frames(left);

    return {
      down,
      up,
      left,
      right,
    };
  }
}

// ASM mapping: pokecrystal_disassembly/engine/overworld/overworld.asm (SetFacingGrassShake).
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { NpcPaletteManager } from "./palette";

export type GrassRustleTarget = {
  x: number;
  y: number;
};

const FRAME_PERIOD = 4;
const GRASS_PALETTE_ID = 6;

type Surface = InstanceType<typeof gameEngine.Surface>;

type DrawLayout = [number, number, number, number];

type Renderable = [number, number, Surface, [number, number]];

const writeRenderable = (
  entries: Renderable[],
  index: number,
  tileY: number,
  tileX: number,
  surface: Surface,
  destX: number,
  destY: number
): number => {
  const existing = entries[index];
  if (existing) {
    existing[0] = tileY;
    existing[1] = tileX;
    existing[2] = surface;
    existing[3][0] = destX;
    existing[3][1] = destY;
  } else {
    entries[index] = [tileY, tileX, surface, [destX, destY]];
  }
  return index + 1;
};

class GrassRustleInstance {
  public readonly target: GrassRustleTarget;
  public remaining_frames: number;
  public age: number;

  constructor(target: GrassRustleTarget, remaining_frames: number) {
    this.target = target;
    this.remaining_frames = remaining_frames;
    this.age = 0;
  }

  public tick(): boolean {
    if (this.remaining_frames <= 0) {
      return false;
    }
    this.age += 1;
    this.remaining_frames -= 1;
    return this.remaining_frames > 0;
  }

  public frame_index(frame_count: number, frame_period: number = FRAME_PERIOD): number {
    if (frame_count <= 0) {
      throw new Error("Grass rustle requires at least one frame to animate.");
    }
    const period = Math.max(1, frame_period);
    return Math.floor(this.age / period) % frame_count;
  }
}

export class GrassRustleController {
  private readonly uses_palette: boolean;
  private palette_manager: NpcPaletteManager | null;
  private palette_id: number;
  private time_of_day: string | null | undefined;
  private base_frames: Surface[];
  private frames: Surface[];
  private effects: Map<GrassRustleTarget, GrassRustleInstance> = new Map();
  private readonly renderablesScratch: Renderable[] = [];

  constructor({
    palette_manager = null,
    palette_id = null,
    time_of_day = null,
    uses_palette = true,
  }: {
    palette_manager?: NpcPaletteManager | null;
    palette_id?: number | null;
    time_of_day?: string | null;
    uses_palette?: boolean;
  } = {}) {
    this.uses_palette = uses_palette;
    this.palette_manager = this.uses_palette ? (palette_manager ?? new NpcPaletteManager()) : null;
    this.palette_id = palette_id ?? GRASS_PALETTE_ID;
    this.time_of_day = time_of_day;

    this.base_frames = GrassRustleController.load_frames();
    if (this.base_frames.length === 0) {
      throw new Error("Grass rustle animation must expose at least one frame.");
    }

    this.frames = this.uses_palette ? this.apply_palette(this.time_of_day) : [...this.base_frames];
  }

  public getBaseFrames(): ReadonlyArray<Surface> {
    return this.base_frames;
  }

  public set_time_of_day(time_of_day?: string | null): void {
    if (!this.uses_palette) {
      return;
    }
    this.time_of_day = time_of_day ?? null;
    this.frames = this.apply_palette(this.time_of_day);
  }

  private apply_palette(time_of_day?: string | null): Surface[] {
    if (!this.palette_manager) {
      throw new Error("Palette manager missing when tinting grass rustle.");
    }
    return this.palette_manager.apply_many(this.base_frames, this.palette_id, time_of_day);
  }

  private static load_frames(): Surface[] {
    const asset_path = getAssetPath("gfx", "overworld", "grass_rustle.png");
    let surface: Surface;
    try {
      const loaded = gameEngine.image.loadSync?.(asset_path) ?? gameEngine.image.load(asset_path);
      if (loaded instanceof Promise) {
        throw new Error("Grass rustle sprite must be preloaded for synchronous use.");
      }
      surface = loaded;
    } catch (exc) {
      throw new Error(`Failed to load grass rustle sprite from ${asset_path}.`);
    }

    surface = GrassRustleController.apply_sprite_transparency(surface);
    let flipped: Surface;
    try {
      flipped = gameEngine.transform.flip(surface, true, false);
    } catch {
      flipped = surface;
    }
    return [surface, flipped];
  }

  private static apply_sprite_transparency(surface: Surface): Surface {
    if (typeof surface.get_at !== "function") {
      return surface;
    }
    const [r, g, b, a] = surface.get_at([0, 0]);
    const transparent: [number, number, number, number] = [r, g, b, 0];
    const width = surface.get_width();
    const height = surface.get_height();
    for (let x = 0; x < width; x += 1) {
      for (let y = 0; y < height; y += 1) {
        const colour = surface.get_at([x, y]);
        if (colour[0] === r && colour[1] === g && colour[2] === b && colour[3] === a) {
          surface.set_at([x, y], transparent);
        }
      }
    }
    return surface;
  }

  public spawn(target: GrassRustleTarget, duration_frames: number): void {
    if (target === null || target === undefined) {
      throw new Error("Grass rustle target may not be null.");
    }
    if (duration_frames <= 0) {
      throw new Error("Grass rustle duration must be positive.");
    }
    this.effects.set(target, new GrassRustleInstance(target, Math.trunc(duration_frames)));
  }

  public tick(): void {
    const expired: GrassRustleTarget[] = [];
    for (const [key, instance] of this.effects.entries()) {
      if (!instance.tick()) {
        expired.push(key);
      }
    }
    for (const key of expired) {
      this.effects.delete(key);
    }
  }

  public renderables(
    draw_layouts: Map<GrassRustleTarget, DrawLayout> | Record<string, DrawLayout>,
    output: Renderable[] = this.renderablesScratch
  ): Renderable[] {
    let renderCount = 0;
    const layoutLookup =
      draw_layouts instanceof Map
        ? (target: GrassRustleTarget) => draw_layouts.get(target) ?? null
        : (target: GrassRustleTarget) =>
            (draw_layouts as Record<string, DrawLayout>)[String(target)] ?? null;

    for (const instance of this.effects.values()) {
      const layout = layoutLookup(instance.target);
      if (!layout) {
        continue;
      }
      if (instance.remaining_frames <= 0) {
        continue;
      }
      const [sprite_x, sprite_y, sprite_w, sprite_h] = layout;
      const frame_index = instance.frame_index(this.frames.length);
      const frame = this.frames[frame_index];
      if (typeof frame.get_width !== "function" || typeof frame.get_height !== "function") {
        continue;
      }
      const dest_x = sprite_x + Math.floor((sprite_w - frame.get_width()) / 2);
      const dest_y = sprite_y + sprite_h - frame.get_height();
      const tile_x = Math.trunc(instance.target.x);
      const tile_y = Math.trunc(instance.target.y);
      renderCount = writeRenderable(output, renderCount, tile_y, tile_x, frame, dest_x, dest_y);
    }

    output.length = renderCount;
    if (renderCount > 1) {
      output.sort((a, b) => (a[0] - b[0]) || (a[1] - b[1]));
    }
    return output;
  }
}

import { Surface, Rect } from '../surface';
import { load_trainer_portrait_surface } from '../trainer-portraits';
import { revert_to_dmg_defaults } from './_battle-palettes';
import { normalise_trainer_id } from './trainer-sprite-id';

const TILE_SIZE = 8;
const RIGHTMOST_TILE = 19;
const STEP_FRAMES = 4;
const MAX_COLUMNS = 6;
const HOLD_FRAMES = 40;

export class TrainerVictorySlide {
  private stage = 0;
  private frame_counter = 0;
  private hold_frames_remaining = 0;
  public is_finished = false;

  constructor(
    public readonly surface: Surface,
    options?: { palette_state?: Record<string, number | null> | null },
  ) {
    if (surface.width <= 0 || surface.height <= 0) {
      throw new Error('Trainer victory slide requires a visible surface.');
    }
    if (options?.palette_state) {
      revert_to_dmg_defaults(options.palette_state);
    }
  }

  get visible_columns(): number {
    return Math.min(this.stage + 1, MAX_COLUMNS);
  }

  get current_width_px(): number {
    return Math.min(this.visible_columns * TILE_SIZE, this.surface.width);
  }

  get left_px(): number {
    return (RIGHTMOST_TILE - this.visible_columns + 1) * TILE_SIZE;
  }

  draw(target: Surface): void {
    const widthPx = this.current_width_px;
    const left = this.left_px;
    const srcRect = new Rect(0, 0, widthPx, this.surface.height);
    target.blit(this.surface, [left, 0], srcRect);
    if (this.is_finished) {
      return;
    }
    if (this.stage >= MAX_COLUMNS) {
      this.hold_frames_remaining -= 1;
      if (this.hold_frames_remaining <= 0) {
        this.is_finished = true;
      }
      return;
    }
    this.frame_counter += 1;
    if (this.frame_counter >= STEP_FRAMES) {
      this.frame_counter = 0;
      this.stage += 1;
      if (this.stage >= MAX_COLUMNS) {
        this.hold_frames_remaining = HOLD_FRAMES;
      }
    }
  }
}

export const build_trainer_victory_slide = (
  ui: { _apply_colorkey_transparency: (surface: Surface) => Surface },
  options: { trainer_class: string; palette_state?: Record<string, number | null> | null },
): TrainerVictorySlide => {
  const spriteId = normalise_trainer_id(options.trainer_class);
  let surface = load_trainer_portrait_surface(spriteId);
  surface = ui._apply_colorkey_transparency(surface);
  return new TrainerVictorySlide(surface, { palette_state: options.palette_state ?? null });
};

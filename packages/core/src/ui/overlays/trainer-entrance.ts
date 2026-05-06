import { Rect, Surface } from '../surface';
import { BattleHardwareRegisters } from './_battle-vram';
import { BattleSpriteOAMManager } from './battle-oam';

export const SCREEN_WIDTH = 160;
export const SCREEN_HEIGHT = 144;
export const START_TOP_OFFSET = 0x90;
export const START_MID_OFFSET = 0x72;
export const TOTAL_FRAMES = 0x49;
export const BACKPIC_TILE_WIDTH = 6;
export const BACKPIC_TILE_HEIGHT = 6;

const TILE_SIZE = 8;
const ENEMY_TILE_WIDTH = 7;
const ENEMY_TILE_HEIGHT = 7;
const PLAYER_BASE_X = 2 * TILE_SIZE;
const PLAYER_BASE_Y = 6 * TILE_SIZE;
const ENEMY_BASE_X = 12 * TILE_SIZE;
const ENEMY_BASE_Y = 0 * TILE_SIZE;
const INTRO_STEP_PIXELS = 2;
export const INTRO_TOP_LINES = 0x3e;
export const INTRO_MID_LINES = 0x22;
export const INTRO_BOTTOM_LINES = 0x30;

type BattleHardwareLike = Partial<BattleHardwareRegisters> & {
  scx?: number;
  scy?: number;
  wx?: number;
  wy?: number;
  set_window?: (wx: number, wy: number) => void;
  set_scroll?: (scx: number, scy: number) => void;
};

// ASM mapping:
// - engine/battle/sliding_intro.asm::BattleIntroSlidingPics (scroll split + OAM slide-in timing)
// - engine/battle/sliding_intro.asm::.subfunction5 (LY override layout: 0x3e/0x22/0x30 scanlines)
// - engine/battle/core.asm::InitBattleDisplay + CopyBackpic (placement at 2,6 and 12,0)
export class TrainerEntranceAnimation {
  private framesRemaining = TOTAL_FRAMES;
  private finished = false;
  private readonly player_surface: Surface;
  private readonly enemy_surface: Surface | null;
  private readonly background_surface: Surface | null;
  private readonly hardware: BattleHardwareLike | null;
  private readonly screen_width: number;
  private readonly screen_height: number;
  private readonly strict_player_size: boolean;

  constructor(
    options: {
      player_surface: Surface;
      enemy_surface?: Surface | null;
      background_surface?: Surface | null;
      hardware?: BattleHardwareLike | null;
      palette_state?: Record<string, number | null> | null;
      screen_size?: [number, number];
      strict_player_size?: boolean;
      enemy_party_size?: number;
    }
  ) {
    this.player_surface = options.player_surface;
    this.enemy_surface = options.enemy_surface ?? null;
    this.background_surface = options.background_surface ?? null;
    this.hardware = options.hardware ?? null;
    const screenSize = options.screen_size ?? [SCREEN_WIDTH, SCREEN_HEIGHT];
    this.screen_width = screenSize[0];
    this.screen_height = screenSize[1];
    this.strict_player_size = Boolean(options.strict_player_size);
    this.validate_surfaces();

    if (this.hardware) {
      this.set_window(0, START_TOP_OFFSET);
      this.set_scroll(START_TOP_OFFSET, 0);
    }
  }

  get is_finished(): boolean {
    return this.finished;
  }

  draw(surface: Surface, _oam_manager?: BattleSpriteOAMManager | null): void {
    if (this.finished) {
      return;
    }
    const frameIndex = this.frame_index();
    const topScx = this.current_top_scx(frameIndex);
    const midScx = this.current_mid_scx(frameIndex);
    this.render_background(surface, topScx, midScx);
    if (this.enemy_surface) {
      surface.blit(this.enemy_surface, [ENEMY_BASE_X + topScx, ENEMY_BASE_Y]);
    }
    surface.blit(this.player_surface, [PLAYER_BASE_X - topScx, PLAYER_BASE_Y]);
    if (this.hardware) {
      this.set_scroll(topScx, 0);
    }
    this.framesRemaining -= 1;
    if (this.framesRemaining <= 0) {
      this.finished = true;
      if (this.hardware) {
        this.set_scroll(0, 0);
        this.set_window(0, START_TOP_OFFSET);
      }
    }
  }

  private set_scroll(scx: number, scy: number): void {
    if (!this.hardware) {
      return;
    }
    if (typeof this.hardware.set_scroll === 'function') {
      this.hardware.set_scroll(scx, scy);
      return;
    }
    this.hardware.scx = scx & 0xff;
    this.hardware.scy = scy & 0xff;
  }

  private set_window(wx: number, wy: number): void {
    if (!this.hardware) {
      return;
    }
    if (typeof this.hardware.set_window === 'function') {
      this.hardware.set_window(wx, wy);
      return;
    }
    this.hardware.wx = wx & 0xff;
    this.hardware.wy = wy & 0xff;
  }

  private validate_surfaces(): void {
    if (this.strict_player_size) {
      const expectedPlayerWidth = BACKPIC_TILE_WIDTH * TILE_SIZE;
      const expectedPlayerHeight = BACKPIC_TILE_HEIGHT * TILE_SIZE;
      if (
        this.player_surface.width !== expectedPlayerWidth ||
        this.player_surface.height !== expectedPlayerHeight
      ) {
        throw new Error(
          `Trainer entrance backpic must be ${expectedPlayerWidth}x${expectedPlayerHeight}px, ` +
          `got ${this.player_surface.width}x${this.player_surface.height}px.`
        );
      }
    }
    if (this.enemy_surface) {
      const expectedEnemyWidth = ENEMY_TILE_WIDTH * TILE_SIZE;
      const expectedEnemyHeight = ENEMY_TILE_HEIGHT * TILE_SIZE;
      if (
        this.enemy_surface.width !== expectedEnemyWidth ||
        this.enemy_surface.height !== expectedEnemyHeight
      ) {
        throw new Error(
          `Trainer entrance frontpic must be ${expectedEnemyWidth}x${expectedEnemyHeight}px, ` +
          `got ${this.enemy_surface.width}x${this.enemy_surface.height}px.`
        );
      }
    }
    if (this.background_surface) {
      if (
        this.background_surface.width < this.screen_width ||
        this.background_surface.height < this.screen_height
      ) {
        throw new Error(
          `Trainer entrance background must be at least ${this.screen_width}x${this.screen_height}px, ` +
          `got ${this.background_surface.width}x${this.background_surface.height}px.`
        );
      }
    }
    if (this.screen_width !== SCREEN_WIDTH || this.screen_height !== SCREEN_HEIGHT) {
      throw new Error(
        `Trainer entrance animation requires ${SCREEN_WIDTH}x${SCREEN_HEIGHT}px screen, ` +
        `got ${this.screen_width}x${this.screen_height}px.`
      );
    }
  }

  private frame_index(): number {
    return TOTAL_FRAMES - this.framesRemaining;
  }

  private current_top_scx(frameIndex: number): number {
    const offset = START_TOP_OFFSET - frameIndex * INTRO_STEP_PIXELS;
    return Math.max(0, offset);
  }

  private current_mid_scx(frameIndex: number): number {
    return (START_MID_OFFSET + frameIndex * INTRO_STEP_PIXELS) & 0xff;
  }

  private render_background(surface: Surface, top_scx: number, mid_scx: number): void {
    if (this.background_surface) {
      this.blit_scrolled_segment(surface, 0, INTRO_TOP_LINES, top_scx);
      this.blit_scrolled_segment(surface, INTRO_TOP_LINES, INTRO_MID_LINES, mid_scx);
      this.blit_scrolled_segment(
        surface,
        INTRO_TOP_LINES + INTRO_MID_LINES,
        INTRO_BOTTOM_LINES,
        0
      );
    } else {
      surface.fill([0, 0, 0, 255]);
    }
  }

  private blit_scrolled_segment(
    target: Surface,
    destY: number,
    height: number,
    scx: number
  ): void {
    if (!this.background_surface || height <= 0) {
      return;
    }
    const width = this.screen_width;
    const bgWidth = this.background_surface.width;
    const offsetX = ((scx % bgWidth) + bgWidth) % bgWidth;
    const sourceY = destY;
    const firstWidth = Math.min(bgWidth - offsetX, width);
    const firstRect = new Rect(offsetX, sourceY, firstWidth, height);
    target.blit(this.background_surface, [0, destY], firstRect);
    if (firstWidth < width) {
      const secondRect = new Rect(0, sourceY, width - firstWidth, height);
      target.blit(this.background_surface, [firstWidth, destY], secondRect);
    }
  }
}

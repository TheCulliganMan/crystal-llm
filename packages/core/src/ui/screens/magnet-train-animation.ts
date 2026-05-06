import { gameEngine, Surface, Rect } from "@pokecrystal/core/ui/game-engine";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { ScreenUI } from "@pokecrystal/core/ui/screens/screen-types";
import { nextFrame } from "@pokecrystal/core/ui/async-loop";
import { GB_FRAME_RATE } from "@pokecrystal/core/core/gb-timing";
import { MagnetTrainGraphics } from "./magnet-train-graphics";

// ASM reference: engine/events/magnet_train.asm.

const SCREEN_WIDTH_TILES = 20;
const SCREEN_HEIGHT_TILES = 18;
const WAIT_FRAMES = 128;
const MAX_FRAMES = 900;

const TOP_BAND_PX = 6 * TILE_SIZE - 1;
const MID_BAND_PX = 6 * TILE_SIZE;
const BOTTOM_BAND_PX = 6 * TILE_SIZE + 1;

type MagnetTrainState = {
  direction: number;
  initPosition: number;
  holdPosition: number;
  finalPosition: number;
  position: number;
  offset: number;
  waitCounter: number;
  phase: number;
  arrivalSfxPlayed: boolean;
};

type OverworldLike = {
  ui?: ScreenUI;
  audio_engine?: { playMusic?: (name: string, role?: string) => void; playSound?: (name: string) => void };
  update?: () => void;
  draw?: () => void;
};

export class MagnetTrainAnimator {
  private readonly musicToken = "MUSIC_MAGNET_TRAIN";
  private readonly arrivalSfx = "SFX_TRAIN_ARRIVED";

  play(directionToGoldenrod: boolean, overworld: OverworldLike | null): void {
    if (!overworld) {
      return;
    }
    const ui = overworld.ui;
    const screen = ui?.screen ?? null;
    if (!ui || !screen) {
      return;
    }

    const baseSurface = MagnetTrainGraphics.createSync().buildBaseSurface();
    const state = this.initialState(directionToGoldenrod);
    const engine = overworld.audio_engine;
    engine?.playMusic?.(this.musicToken, "special");

    this.runLoop(overworld, screen, baseSurface, state, engine);
  }

  async playAsync(directionToGoldenrod: boolean, overworld: OverworldLike | null): Promise<void> {
    if (!overworld) {
      return;
    }
    const ui = overworld.ui;
    const screen = ui?.screen ?? null;
    if (!ui || !screen) {
      return;
    }

    const baseSurface = (await MagnetTrainGraphics.create()).buildBaseSurface();
    const state = this.initialState(directionToGoldenrod);
    const engine = overworld.audio_engine;
    engine?.playMusic?.(this.musicToken, "special");

    await this.runLoopAsync(overworld, screen, baseSurface, state, engine);
  }

  private initialState(directionToGoldenrod: boolean): MagnetTrainState {
    if (directionToGoldenrod) {
      return {
        direction: -1,
        initPosition: -96,
        holdPosition: -64,
        finalPosition: 96,
        position: -96,
        offset: -96,
        waitCounter: 0,
        phase: 0,
        arrivalSfxPlayed: false,
      };
    }
    return {
      direction: 1,
      initPosition: 96,
      holdPosition: 64,
      finalPosition: -96,
      position: 96,
      offset: 96,
      waitCounter: 0,
      phase: 0,
      arrivalSfxPlayed: false,
    };
  }

  private runLoop(
    overworld: OverworldLike,
    screen: Surface,
    baseSurface: Surface,
    state: MagnetTrainState,
    audioEngine: OverworldLike["audio_engine"]
  ): void {
    const [width, height] = baseSurface.get_size();
    const clock = new gameEngine.time.Clock();
    for (let i = 0; i < MAX_FRAMES; i += 1) {
      const { done, playArrival } = this.stepState(state);
      const scxBg = (state.offset * 2) & 0xff;
      const scxTrain = state.position & 0xff;

      const frameSurface = new gameEngine.Surface(width, height);
      this.blitScrolledBand(frameSurface, baseSurface, 0, TOP_BAND_PX, scxBg);
      this.blitScrolledBand(frameSurface, baseSurface, TOP_BAND_PX, MID_BAND_PX, scxTrain);
      this.blitScrolledBand(
        frameSurface,
        baseSurface,
        TOP_BAND_PX + MID_BAND_PX,
        BOTTOM_BAND_PX,
        scxBg
      );

      screen.blit(frameSurface, [0, 0]);
      overworld.update?.();
      overworld.draw?.();
      overworld.ui?.update?.();

      if (playArrival) {
        audioEngine?.playSound?.(this.arrivalSfx);
      }

      state.offset += state.direction * 2;
      if (done) {
        break;
      }
      clock.tick(GB_FRAME_RATE);
    }
  }

  private async runLoopAsync(
    overworld: OverworldLike,
    screen: Surface,
    baseSurface: Surface,
    state: MagnetTrainState,
    audioEngine: OverworldLike["audio_engine"]
  ): Promise<void> {
    const [width, height] = baseSurface.get_size();
    for (let i = 0; i < MAX_FRAMES; i += 1) {
      const { done, playArrival } = this.stepState(state);
      const scxBg = (state.offset * 2) & 0xff;
      const scxTrain = state.position & 0xff;

      const frameSurface = new gameEngine.Surface(width, height);
      this.blitScrolledBand(frameSurface, baseSurface, 0, TOP_BAND_PX, scxBg);
      this.blitScrolledBand(frameSurface, baseSurface, TOP_BAND_PX, MID_BAND_PX, scxTrain);
      this.blitScrolledBand(
        frameSurface,
        baseSurface,
        TOP_BAND_PX + MID_BAND_PX,
        BOTTOM_BAND_PX,
        scxBg
      );

      screen.blit(frameSurface, [0, 0]);
      overworld.update?.();
      overworld.draw?.();
      overworld.ui?.update?.();

      if (playArrival) {
        audioEngine?.playSound?.(this.arrivalSfx);
      }

      state.offset += state.direction * 2;
      if (done) {
        break;
      }
      await nextFrame();
    }
  }

  private stepState(state: MagnetTrainState): { done: boolean; playArrival: boolean } {
    let playArrival = false;
    if (state.phase === 0) {
      state.waitCounter = WAIT_FRAMES;
      state.phase = 1;
    } else if ([1, 3, 5].includes(state.phase)) {
      if (state.waitCounter > 0) {
        state.waitCounter -= 1;
      } else {
        state.phase += 1;
      }
    } else if (state.phase === 2) {
      if (state.position === state.holdPosition) {
        state.waitCounter = WAIT_FRAMES;
        state.phase = 3;
      } else {
        state.position -= state.direction;
      }
    } else if (state.phase === 4) {
      if (state.position === state.finalPosition) {
        state.phase = 5;
      } else {
        state.position -= state.direction * 2;
      }
    } else if (state.phase === 6) {
      state.phase = 7;
    }
    const done = state.phase >= 7;
    if (done && !state.arrivalSfxPlayed) {
      state.arrivalSfxPlayed = true;
      playArrival = true;
    }
    return { done, playArrival };
  }

  private blitScrolledBand(
    dest: Surface,
    src: Surface,
    y: number,
    height: number,
    scx: number
  ): void {
    const width = src.get_width();
    if (height <= 0) {
      return;
    }
    let shift = scx < 128 ? scx : scx - 256;
    if (shift === 0) {
      dest.blit(src, [0, y], new Rect(0, y, width, height));
      return;
    }
    if (shift > 0) {
      dest.blit(src, [0, y], new Rect(shift, y, width - shift, height));
      dest.blit(src, [width - shift, y], new Rect(0, y, shift, height));
    } else {
      shift = -shift;
      dest.blit(src, [shift, y], new Rect(0, y, width - shift, height));
      dest.blit(src, [0, y], new Rect(width - shift, y, shift, height));
    }
  }
}

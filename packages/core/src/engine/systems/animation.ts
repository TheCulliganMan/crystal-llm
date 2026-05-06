/**
 * @file This module contains classes for handling sprite animation.
 */

import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { useStore } from "@pokecrystal/core/core/state";

type Surface = InstanceType<typeof gameEngine.Surface>;

/**
 * Handles frame-based sprite animation.
 */
export class SpriteAnimation {
  frames: Surface[];
  frameDuration: number;
  currentFrameIndex: number;
  frameCounter: number;
  animate: boolean;
  facing: number; // STANDING
  private _manualFrame: boolean;

  constructor(
    frames: Surface[],
    frameDuration: number = 8,
    animate: boolean = false
  ) {
    this.frames = frames;
    this.frameDuration = frameDuration;
    this.currentFrameIndex = 0;
    this.frameCounter = 0;
    this.animate = animate;
    this.facing = -1; // STANDING
    this._manualFrame = false;
  }

  /**
   * Updates the animation frame.
   */
  update() {
    if (this._manualFrame) {
      return;
    }

    if (!this.animate) {
      return;
    }
    this.frameCounter += 1;
    const effectiveDuration = useStore.getState().wram.instant_mode ? 1 : this.frameDuration;
    if (this.frameCounter >= effectiveDuration) {
      this.frameCounter = 0;
      this.currentFrameIndex =
        (this.currentFrameIndex + 1) % this.frames.length;
    }
  }

  /**
   * Set the facing value and update animation state without jitter.
   * @param facing - The direction the sprite is facing.
   * @param force - Re-primes the stride even when `facing` matches the current direction.
   */
  setFacing(facing: number, force: boolean = false) {
    if (facing === this.facing && !force) {
      return;
    }

    const previous = this.facing;
    this.facing = facing;

    if (facing === -1) {
      // STANDING
      this.currentFrameIndex = 0;
      this.frameCounter = 0;
      this.animate = false;
      this._manualFrame = false;
      return;
    }

    if (this.frames.length <= 1) {
      this.currentFrameIndex = 0;
      this.animate = false;
      this._manualFrame = false;
      return;
    }

    this.animate = true;
    if (previous === -1) {
      // Transitioning from idle to walking: start on the stepping frame.
      this.currentFrameIndex = 1 % this.frames.length;
      this.frameCounter = 0;
      this._manualFrame = false;
      return;
    }

    if (force) {
      // Re-priming a continued stride: advance the walk cycle once per step.
      this.currentFrameIndex =
        (this.currentFrameIndex + 1) % this.frames.length;
      this.frameCounter = 0;
      this._manualFrame = false;
    }
  }

  /**
   * Manually set the current frame index, disabling automatic animation.
   * @param index - The frame index to set.
   */
  setFrame(index: number) {
    if (!this.frames || this.frames.length === 0) {
      return;
    }
    this.currentFrameIndex = index % this.frames.length;
    this._manualFrame = true;
    this.animate = false; // Disable auto-animate
  }

  /**
   * Returns the current frame of the animation.
   */
  get currentFrame(): Surface {
    return this.frames[this.currentFrameIndex];
  }
}

/**
 * Advance the walk cycle exactly once, then hold the frame for the step.
 *
 * This mirrors the handheld stride cadence where each tile step flips
 * between standing and stepping without free-running animation.
 * @param animation - The sprite animation to update.
 */
export function primeWalkStride(animation: SpriteAnimation | null | undefined) {
  if (!animation) {
    return;
  }
  animation.setFacing(0, true);
  animation.setFrame(animation.currentFrameIndex);
}

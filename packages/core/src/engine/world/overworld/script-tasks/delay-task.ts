import { ScriptTask } from "./script-task";
import type { OverworldContext } from "@pokecrystal/core/engine/world/story-events/commands/base";

export class DelayTask extends ScriptTask {
  private framesRemaining: number;

  constructor(frames: number, options: { onComplete?: (() => void) | null; blocking?: boolean } = {}) {
    super({ blocking: options.blocking, onComplete: options.onComplete ?? null });
    this.framesRemaining = Math.max(0, Math.trunc(frames));
  }

  start(overworld: OverworldContext): void {
    super.start(overworld);
    if (this.framesRemaining === 0) {
      this.completed = true;
    }
  }

  update(_overworld: OverworldContext): void {
    if (this.completed || this.framesRemaining <= 0) {
      this.completed = true;
      return;
    }
    this.framesRemaining -= 1;
    if (this.framesRemaining <= 0) {
      this.completed = true;
    }
  }
}

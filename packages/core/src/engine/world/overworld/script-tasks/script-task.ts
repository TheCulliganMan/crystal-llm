import type { OverworldContext } from "@pokecrystal/core/engine/world/story-events/commands/base";

export class ScriptTask<T = OverworldContext> {
  public blocking: boolean;
  public onComplete: (() => void) | null;
  public started = false;
  public completed = false;

  constructor(options: { blocking?: boolean; onComplete?: (() => void) | null } = {}) {
    this.blocking = options.blocking ?? true;
    this.onComplete = options.onComplete ?? null;
  }

  start(_overworld: T): void {
    this.started = true;
  }

  update(_overworld: T): void {
    throw new Error("Script tasks must implement update()");
  }

  finish(_overworld: T): void {
    return;
  }
}

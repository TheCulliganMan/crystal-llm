import { z } from "zod";
import { InputAdapter, InputDirection, InputEvent } from "./adapters";
import { KEYS } from "./keycodes";

const DirectionSchema = z.enum(["up", "down", "left", "right"]);

const DIRECTION_KEYS: Record<InputDirection, number> = {
  up: KEYS.UP,
  down: KEYS.DOWN,
  left: KEYS.LEFT,
  right: KEYS.RIGHT,
};

const DEFAULT_PATTERN: InputDirection[] = ["right", "down", "left", "up"];

export const parseAutoMovePattern = (pattern?: string | null): InputDirection[] => {
  if (!pattern || !String(pattern).trim()) {
    return [...DEFAULT_PATTERN];
  }
  const normalized: InputDirection[] = [];
  for (const token of pattern.split(",")) {
    const direction = token.trim().toLowerCase();
    if (!direction) {
      continue;
    }
    const parsed = DirectionSchema.safeParse(direction);
    if (!parsed.success) {
      throw new Error(
        `Unsupported auto-move direction '${direction}'. Use up, down, left, or right.`
      );
    }
    normalized.push(parsed.data);
  }
  if (normalized.length === 0) {
    throw new Error("Auto-move pattern must contain at least one direction.");
  }
  return normalized;
};

export class AutoMoveAdapter implements InputAdapter {
  private readonly pattern: InputDirection[];
  private readonly holdFrames: number;
  private readonly restFrames: number;
  private currentIndex = 0;
  private currentDirection: InputDirection | null = null;
  private remainingHoldFrames = 0;
  private remainingRestFrames = 0;

  constructor(
    pattern?: Iterable<InputDirection> | null,
    options?: { holdFrames?: number; restFrames?: number }
  ) {
    const directions = pattern ? Array.from(pattern) : [...DEFAULT_PATTERN];
    if (directions.length === 0) {
      throw new Error("Auto-move pattern cannot be empty.");
    }
    for (const direction of directions) {
      const parsed = DirectionSchema.safeParse(direction);
      if (!parsed.success) {
        throw new Error(
          `Unsupported auto-move direction '${direction}'. Use up, down, left, or right.`
        );
      }
    }
    const holdFrames = options?.holdFrames ?? 12;
    if (holdFrames < 1) {
      throw new Error("Auto-move holdFrames must be at least 1.");
    }
    this.pattern = directions;
    this.holdFrames = holdFrames;
    this.restFrames = Math.max(0, options?.restFrames ?? 4);
  }

  poll(): InputEvent[] {
    const now = Date.now() / 1000;
    const events: InputEvent[] = [];

    if (this.remainingRestFrames > 0) {
      this.remainingRestFrames -= 1;
      return events;
    }

    if (this.currentDirection === null) {
      const direction = this.pattern[this.currentIndex];
      this.currentDirection = direction;
      this.remainingHoldFrames = this.holdFrames - 1;
      events.push(this.buildEvent(KEYS.KEYDOWN, direction, true, now));
      return events;
    }

    if (this.remainingHoldFrames > 0) {
      this.remainingHoldFrames -= 1;
      return events;
    }

    events.push(this.buildEvent(KEYS.KEYUP, this.currentDirection, false, now));
    this.currentDirection = null;
    this.remainingRestFrames = this.restFrames;
    this.currentIndex = (this.currentIndex + 1) % this.pattern.length;
    return events;
  }

  close(): void {
    return;
  }

  private buildEvent(
    eventType: number,
    direction: InputDirection,
    isPress: boolean,
    timestamp: number
  ): InputEvent {
    const key = DIRECTION_KEYS[direction];
    return new InputEvent({
      type: eventType,
      source: "auto_move",
      direction,
      is_press: isPress,
      key,
      timestamp,
    });
  }
}

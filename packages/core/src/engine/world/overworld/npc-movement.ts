const FACING_DEFAULTS: Record<string, string> = {
  SPRITEMOVEDATA_00: "down",
  SPRITEMOVEDATA_STILL: "down",
  SPRITEMOVEDATA_WANDER: "down",
  SPRITEMOVEDATA_SPINRANDOM_SLOW: "down",
  SPRITEMOVEDATA_WALK_UP_DOWN: "down",
  SPRITEMOVEDATA_WALK_LEFT_RIGHT: "down",
  SPRITEMOVEDATA_STANDING_DOWN: "down",
  SPRITEMOVEDATA_STANDING_UP: "up",
  SPRITEMOVEDATA_STANDING_LEFT: "left",
  SPRITEMOVEDATA_STANDING_RIGHT: "right",
  SPRITEMOVEDATA_SPINRANDOM_FAST: "down",
  SPRITEMOVEDATA_SPINCOUNTERCLOCKWISE: "left",
  SPRITEMOVEDATA_SPINCLOCKWISE: "right",
  SPRITEMOVEDATA_PLAYER: "down",
  SPRITEMOVEDATA_INDEXED_1: "down",
  SPRITEMOVEDATA_INDEXED_2: "down",
  SPRITEMOVEDATA_0E: "down",
};

const SPIN_CLOCKWISE_ORDER = ["up", "right", "down", "left"] as const;
const SPIN_COUNTERCLOCKWISE_ORDER = ["up", "left", "down", "right"] as const;

export function defaultFacingForMovement(movement: string): string | undefined {
  return FACING_DEFAULTS[movement.toUpperCase()];
}

export function applyDefaultFacing(
  movement: string,
  { direction }: { direction?: string | null }
): string {
  const fallback = direction ?? "down";
  return defaultFacingForMovement(movement) ?? fallback;
}

export function stepPatternForObject(
  movement: string,
  {
    moveRangeX,
    moveRangeY,
  }: {
    moveRangeX: number;
    moveRangeY: number;
  }
): string[] | null {
  const normalized = movement.toUpperCase();
  if (normalized === "SPRITEMOVEDATA_WALK_LEFT_RIGHT" && moveRangeX > 0) {
    const leftSteps = new Array(moveRangeX).fill("left");
    const rightSteps = new Array(moveRangeX).fill("right");
    return [...leftSteps, ...rightSteps];
  }
  if (normalized === "SPRITEMOVEDATA_WALK_UP_DOWN" && moveRangeY > 0) {
    const upSteps = new Array(moveRangeY).fill("up");
    const downSteps = new Array(moveRangeY).fill("down");
    return [...upSteps, ...downSteps];
  }
  return null;
}

export function spinCycleForMovement(movement: string): string[] | null {
  const normalized = movement.toUpperCase();
  if (normalized === "SPRITEMOVEDATA_SPINCLOCKWISE") {
    return [...SPIN_CLOCKWISE_ORDER];
  }
  if (normalized === "SPRITEMOVEDATA_SPINCOUNTERCLOCKWISE") {
    return [...SPIN_COUNTERCLOCKWISE_ORDER];
  }
  return null;
}

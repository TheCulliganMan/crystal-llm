import { FrameType, frameTypeRenderId } from "@pokecrystal/core/core/enums/ui-enums";

const MIN_FRAME_TYPE = FrameType.FRAME_1;
const MAX_FRAME_TYPE = FrameType.FRAME_8;
const MIN_FRAME_RENDER_ID = 1;
const MAX_FRAME_RENDER_ID = 8;

const asInteger = (value: unknown, label: string): number => {
  if (typeof value !== "number" || !Number.isInteger(value)) {
    throw new Error(`${label} must be an integer.`);
  }
  return value;
};

const assertRenderId = (value: unknown): number => {
  const numeric = asInteger(value, "Textbox frame render id");
  if (numeric < MIN_FRAME_RENDER_ID || numeric > MAX_FRAME_RENDER_ID) {
    throw new Error(
      `Textbox frame render id ${numeric} is out of range (${MIN_FRAME_RENDER_ID}-${MAX_FRAME_RENDER_ID}).`
    );
  }
  return numeric;
};

const assertFrameType = (value: unknown): FrameType => {
  const numeric = asInteger(value, "Textbox frame type");
  if (numeric < MIN_FRAME_TYPE || numeric > MAX_FRAME_TYPE) {
    throw new Error(
      `Textbox frame type ${numeric} is out of range (${MIN_FRAME_TYPE}-${MAX_FRAME_TYPE}).`
    );
  }
  return numeric as FrameType;
};

// ASM mapping: options_menu stores wTextboxFrame as 0-7 (FrameType), while
// frame assets are loaded by render ids 1-8 (gfx/frames/{id}.png).
export const resolveTextboxFrameRenderId = (
  frameType: unknown,
  fallbackRenderId: unknown = MIN_FRAME_RENDER_ID
): number => {
  if (frameType === undefined || frameType === null) {
    return assertRenderId(fallbackRenderId);
  }
  return frameTypeRenderId(assertFrameType(frameType));
};

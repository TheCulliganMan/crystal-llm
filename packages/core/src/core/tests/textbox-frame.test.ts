import { FrameType } from "@pokecrystal/core/core/enums/ui-enums";
import { resolveTextboxFrameRenderId } from "@pokecrystal/core/core/textbox-frame";

describe("resolveTextboxFrameRenderId", () => {
  it("converts frame type values to render ids", () => {
    expect(resolveTextboxFrameRenderId(FrameType.FRAME_1)).toBe(1);
    expect(resolveTextboxFrameRenderId(FrameType.FRAME_8)).toBe(8);
  });

  it("falls back to a validated render id when frame type is missing", () => {
    expect(resolveTextboxFrameRenderId(undefined, 1)).toBe(1);
    expect(resolveTextboxFrameRenderId(null, 8)).toBe(8);
  });

  it("throws for out-of-range frame type values", () => {
    expect(() => resolveTextboxFrameRenderId(8)).toThrow(
      "Textbox frame type 8 is out of range (0-7)."
    );
  });

  it("throws for invalid fallback render ids", () => {
    expect(() => resolveTextboxFrameRenderId(undefined, 0)).toThrow(
      "Textbox frame render id 0 is out of range (1-8)."
    );
  });
});

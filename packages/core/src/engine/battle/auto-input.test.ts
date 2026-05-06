import { createJoypadState } from "@pokecrystal/core/core/memory/hram";
import { B_PAD_A, B_PAD_DOWN } from "@pokecrystal/core/input/controls";
import { DudeAutoInputController } from "./auto-input";

const findInputFrame = (
  controller: DudeAutoInputController,
  mask: number,
  maxFrames: number,
): number | null => {
  const joypad = createJoypadState();
  for (let frame = 1; frame <= maxFrames; frame += 1) {
    controller.step(joypad);
    if ((joypad.hJoyPressed & mask) !== 0) {
      return frame;
    }
  }
  return null;
};

describe("DudeAutoInputController tutorial timing", () => {
  it("keeps PromptButton auto-A timing on the raw ASM cadence", () => {
    const controller = new DudeAutoInputController();
    controller.queueA();

    const aFrame = findInputFrame(controller, B_PAD_A, 120);

    expect(aFrame).toBe(81);
  });

  it("compresses battle-menu auto input so tutorial PACK selection is not excessively delayed", () => {
    const controller = new DudeAutoInputController();
    const joypad = createJoypadState();
    controller.queueDownA();

    let downFrame: number | null = null;
    let aFrame: number | null = null;

    for (let frame = 1; frame <= 200; frame += 1) {
      controller.step(joypad);
      if (downFrame === null && (joypad.hJoyPressed & B_PAD_DOWN) !== 0) {
        downFrame = frame;
      }
      if (aFrame === null && (joypad.hJoyPressed & B_PAD_A) !== 0) {
        aFrame = frame;
      }
      if (downFrame !== null && aFrame !== null) {
        break;
      }
    }

    expect(downFrame).not.toBeNull();
    expect(aFrame).not.toBeNull();
    expect(downFrame as number).toBeLessThan(96);
    expect(aFrame as number).toBeLessThan(160);
    expect(aFrame as number).toBeGreaterThan(downFrame as number);
  });
});

/** @jest-environment node */
import { GB_FRAME_RATE } from "@pokecrystal/core/core/gb-timing";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { MagnetTrainAnimator } from "./magnet-train-animation";
import { MagnetTrainGraphics } from "./magnet-train-graphics";

type TraceEntry = {
  frame: number;
  position: number;
  phase: number;
  playArrival: boolean;
  done: boolean;
};

const traceAnimation = (directionToGoldenrod: boolean): TraceEntry[] => {
  const animator = new MagnetTrainAnimator();
  const state = (animator as any).initialState(directionToGoldenrod);
  const stepState = (animator as any).stepState.bind(animator);
  const trace: TraceEntry[] = [];

  for (let frame = 1; frame < 500; frame += 1) {
    const result = stepState(state) as { done: boolean; playArrival: boolean };
    trace.push({
      frame,
      position: state.position,
      phase: state.phase,
      playArrival: result.playArrival,
      done: result.done,
    });
    if (result.done) {
      break;
    }
  }

  return trace;
};

describe("MagnetTrainAnimator", () => {
  it("builds the base scene from the ASM magnet train tilemaps", () => {
    const surface = MagnetTrainGraphics.createSync().buildBaseSurface();
    const data = surface.getImageData().data;
    let nonBlackPixels = 0;

    for (let index = 0; index < data.length; index += 4) {
      if (data[index] !== 0 || data[index + 1] !== 0 || data[index + 2] !== 0) {
        nonBlackPixels += 1;
      }
    }

    expect(surface.get_size()).toEqual([160, 144]);
    expect(nonBlackPixels).toBeGreaterThan(0);
  });

  it("throws instead of silently rendering a blank train scene when graphics are unavailable", () => {
    const originalLoadSync = gameEngine.image.loadSync;
    gameEngine.image.loadSync = jest.fn(() => null);

    try {
      expect(() => MagnetTrainGraphics.createSync()).toThrow("Unable to load magnet train tileset:");
    } finally {
      gameEngine.image.loadSync = originalLoadSync;
    }
  });

  it("matches ASM scroll endpoints and frame timing toward Saffron", () => {
    const trace = traceAnimation(false);

    expect(trace[0]).toMatchObject({ frame: 1, position: 96, phase: 1 });
    expect(trace[129]).toMatchObject({ frame: 130, position: 96, phase: 2 });
    expect(trace[130]).toMatchObject({ frame: 131, position: 95, phase: 2 });
    expect(trace[161]).toMatchObject({ frame: 162, position: 64, phase: 2 });
    expect(trace[162]).toMatchObject({ frame: 163, position: 64, phase: 3 });
    expect(trace[291]).toMatchObject({ frame: 292, position: 64, phase: 4 });
    expect(trace[292]).toMatchObject({ frame: 293, position: 62, phase: 4 });
    expect(trace[371]).toMatchObject({ frame: 372, position: -96, phase: 4 });
    expect(trace[372]).toMatchObject({ frame: 373, position: -96, phase: 5 });
    expect(trace[373]).toMatchObject({ frame: 374, position: -96, phase: 6 });
    expect(trace[374]).toEqual({
      frame: 375,
      position: -96,
      phase: 7,
      playArrival: true,
      done: true,
    });
  });

  it("matches ASM scroll endpoints and frame timing toward Goldenrod", () => {
    const trace = traceAnimation(true);

    expect(trace[0]).toMatchObject({ frame: 1, position: -96, phase: 1 });
    expect(trace[129]).toMatchObject({ frame: 130, position: -96, phase: 2 });
    expect(trace[130]).toMatchObject({ frame: 131, position: -95, phase: 2 });
    expect(trace[161]).toMatchObject({ frame: 162, position: -64, phase: 2 });
    expect(trace[162]).toMatchObject({ frame: 163, position: -64, phase: 3 });
    expect(trace[291]).toMatchObject({ frame: 292, position: -64, phase: 4 });
    expect(trace[292]).toMatchObject({ frame: 293, position: -62, phase: 4 });
    expect(trace[371]).toMatchObject({ frame: 372, position: 96, phase: 4 });
    expect(trace[374]).toEqual({
      frame: 375,
      position: 96,
      phase: 7,
      playArrival: true,
      done: true,
    });
  });

  it("uses the exact LY override band heights from the ASM", () => {
    const animator = new MagnetTrainAnimator();
    const screen = new gameEngine.Surface(160, 144);
    const bandCalls: Array<{ y: number; height: number }> = [];
    const delaySpy = jest.spyOn(gameEngine.time.Clock.prototype, "tick").mockImplementation(() => undefined);
    const stepSpy = jest
      .spyOn(animator as any, "stepState")
      .mockReturnValue({ done: true, playArrival: false });
    const bandSpy = jest
      .spyOn(animator as any, "blitScrolledBand")
      .mockImplementation((_dest: unknown, _src: unknown, y: number, height: number) => {
        bandCalls.push({ y, height });
      });

    try {
      animator.play(false, {
        ui: { screen, update: jest.fn() },
        audio_engine: { playMusic: jest.fn(), playSound: jest.fn() },
      });

      expect(bandCalls).toEqual([
        { y: 0, height: 47 },
        { y: 47, height: 48 },
        { y: 95, height: 49 },
      ]);
    } finally {
      delaySpy.mockRestore();
      stepSpy.mockRestore();
      bandSpy.mockRestore();
    }
  });

  it("paces sync playback at the Game Boy frame rate", () => {
    const animator = new MagnetTrainAnimator();
    const screen = new gameEngine.Surface(160, 144);
    const tickSpy = jest.spyOn(gameEngine.time.Clock.prototype, "tick").mockImplementation(() => undefined);

    try {
      animator.play(false, {
        ui: { screen, update: jest.fn() },
        audio_engine: { playMusic: jest.fn(), playSound: jest.fn() },
      });

      expect(tickSpy).toHaveBeenCalledWith(GB_FRAME_RATE);
    } finally {
      tickSpy.mockRestore();
    }
  });
});

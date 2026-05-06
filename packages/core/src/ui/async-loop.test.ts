import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { __resetNextFrameClockForTests, nextFrame } from "@pokecrystal/core/ui/async-loop";

describe("nextFrame", () => {
  beforeEach(() => {
    __resetNextFrameClockForTests();
  });

  it("falls back to GB frame-duration timers when requestAnimationFrame is unavailable", async () => {
    const globalScope = globalThis as typeof globalThis & {
      requestAnimationFrame?: typeof requestAnimationFrame;
    };
    const previousRaf = globalScope.requestAnimationFrame;
    delete globalScope.requestAnimationFrame;
    const timeoutSpy = jest
      .spyOn(globalThis, "setTimeout")
      .mockImplementation(((callback: (...args: unknown[]) => void) => {
        callback();
        return 1 as ReturnType<typeof setTimeout>;
      }) as typeof setTimeout);

    try {
      await nextFrame();
      expect(timeoutSpy).toHaveBeenCalledTimes(1);
      const [callback, ms] = timeoutSpy.mock.calls[0] as [TimerHandler, number];
      expect(typeof callback).toBe("function");
      expect(ms).toBeCloseTo(GB_FRAME_DURATION_MS, 6);
    } finally {
      timeoutSpy.mockRestore();
      if (previousRaf === undefined) {
        delete globalScope.requestAnimationFrame;
      } else {
        Object.defineProperty(globalThis, "requestAnimationFrame", {
          configurable: true,
          writable: true,
          value: previousRaf,
        });
      }
    }
  });

  it("throttles consecutive requestAnimationFrame ticks to GB cadence", async () => {
    const globalScope = globalThis as typeof globalThis & {
      requestAnimationFrame?: typeof requestAnimationFrame;
    };
    const previousRaf = globalScope.requestAnimationFrame;
    const rafCallbacks: FrameRequestCallback[] = [];
    const rafMock = jest.fn((callback: FrameRequestCallback): number => {
      rafCallbacks.push(callback);
      return rafCallbacks.length;
    });
    Object.defineProperty(globalThis, "requestAnimationFrame", {
      configurable: true,
      writable: true,
      value: rafMock,
    });
    const timeoutSpy = jest
      .spyOn(globalThis, "setTimeout")
      .mockImplementation(((callback: (...args: unknown[]) => void) => {
        callback();
        return 1 as ReturnType<typeof setTimeout>;
      }) as typeof setTimeout);

    try {
      const first = nextFrame();
      expect(rafCallbacks).toHaveLength(1);
      const firstCallback = rafCallbacks.shift();
      expect(firstCallback).toBeDefined();
      firstCallback?.(0);
      await first;

      const second = nextFrame();
      expect(rafCallbacks).toHaveLength(1);
      const secondCallback = rafCallbacks.shift();
      expect(secondCallback).toBeDefined();
      secondCallback?.(GB_FRAME_DURATION_MS / 2);
      await second;

      expect(timeoutSpy).toHaveBeenCalledTimes(1);
      const [callback, ms] = timeoutSpy.mock.calls[0] as [TimerHandler, number];
      expect(typeof callback).toBe("function");
      expect(ms).toBeCloseTo(GB_FRAME_DURATION_MS / 2, 2);
    } finally {
      timeoutSpy.mockRestore();
      if (previousRaf === undefined) {
        delete globalScope.requestAnimationFrame;
      } else {
        Object.defineProperty(globalThis, "requestAnimationFrame", {
          configurable: true,
          writable: true,
          value: previousRaf,
        });
      }
    }
  });
});

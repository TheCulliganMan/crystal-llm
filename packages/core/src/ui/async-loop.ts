import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";

const FRAME_EPSILON_MS = 0.5;

type FrameClockState = {
  nextFrameTargetMs: number | null;
};

const nowMs = (): number => {
  if (typeof performance !== "undefined" && typeof performance.now === "function") {
    return performance.now();
  }
  return Date.now();
};

const scheduleFrameWait = (
  state: FrameClockState,
  timestamp: number,
  resolve: () => void,
  allowImmediateFirstFrame: boolean,
): void => {
  const previousTarget =
    state.nextFrameTargetMs
    ?? (timestamp - (allowImmediateFirstFrame ? GB_FRAME_DURATION_MS : 0));
  const targetMs = previousTarget + GB_FRAME_DURATION_MS;
  const remainingMs = targetMs - timestamp;

  if (remainingMs <= FRAME_EPSILON_MS) {
    state.nextFrameTargetMs = targetMs;
    resolve();
    return;
  }

  setTimeout(() => {
    state.nextFrameTargetMs = targetMs;
    resolve();
  }, remainingMs);
};

const waitForGbFrame = (state: FrameClockState): Promise<void> =>
  new Promise((resolve) => {
    if (typeof requestAnimationFrame === "function") {
      requestAnimationFrame((timestamp) => scheduleFrameWait(state, timestamp, resolve, true));
      return;
    }
    scheduleFrameWait(state, nowMs(), resolve, false);
  });

const defaultFrameClockState: FrameClockState = {
  nextFrameTargetMs: null,
};

export const createGbFrameAwaiter = (): (() => Promise<void>) => {
  const state: FrameClockState = {
    nextFrameTargetMs: null,
  };
  return () => waitForGbFrame(state);
};

export const __resetNextFrameClockForTests = (): void => {
  defaultFrameClockState.nextFrameTargetMs = null;
};

export const nextFrame = (): Promise<void> => waitForGbFrame(defaultFrameClockState);

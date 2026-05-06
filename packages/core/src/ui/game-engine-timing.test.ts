import { GB_FRAME_RATE } from "@pokecrystal/core/core/gb-timing";
import { normalizeClockFps } from "./game-engine";

describe("game-engine timing normalization", () => {
  it("maps legacy 60fps callers to the exact GB frame cadence", () => {
    expect(normalizeClockFps(60)).toBe(GB_FRAME_RATE);
    expect(normalizeClockFps(60.0005)).toBe(GB_FRAME_RATE);
  });

  it("passes through non-legacy fps values and rejects invalid input", () => {
    expect(normalizeClockFps(30)).toBe(30);
    expect(normalizeClockFps(59)).toBe(59);
    expect(normalizeClockFps(0)).toBe(0);
    expect(normalizeClockFps(Number.NaN)).toBe(0);
  });
});

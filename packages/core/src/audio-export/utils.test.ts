import {
  freqToRegWave,
  frameTickToSampleOffset,
  next128hzTickStrict,
  nextFrameTickWithStepStrict,
  parseNumber,
  regToFreqWave,
  sampleOffsetToFrameTick,
  ticksToFrames,
} from "@pokecrystal/core/audio-export/utils";

describe("audio-export utils", () => {
  it("parses decimal/hex/bin tokens", () => {
    expect(parseNumber("42")).toBe(42);
    expect(parseNumber("$ff")).toBe(255);
    expect(parseNumber("%1010")).toBe(10);
    expect(parseNumber("-0x10")).toBe(-16);
  });

  it("maps sample offsets and frame ticks", () => {
    const tick = sampleOffsetToFrameTick(44_100);
    expect(tick).toBe(512);
    expect(frameTickToSampleOffset(tick)).toBe(44_100);
  });

  it("computes strict scheduler ticks", () => {
    expect(nextFrameTickWithStepStrict(7, 7)).toBe(15);
    expect(next128hzTickStrict(3)).toBe(7);
  });

  it("converts wave frequency register values", () => {
    const reg = freqToRegWave(440);
    const freq = regToFreqWave(reg);
    expect(Math.abs(freq - 440)).toBeLessThan(3);
  });

  it("converts ticks to frames with remainder", () => {
    const [frames] = ticksToFrames(256, 0);
    expect(frames).toBeGreaterThan(0);
  });
});

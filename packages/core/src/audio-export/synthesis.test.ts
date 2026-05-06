import { createChannelState } from "@pokecrystal/core/audio-export/schemas";
import {
  computeIntegral,
  noiseKernelWrapper,
  pulseKernel,
  waveKernel,
} from "@pokecrystal/core/audio-export/synthesis";

describe("audio-export synthesis", () => {
  it("computes cumulative integral", () => {
    const input = new Float32Array([1, -1, 2]);
    const out = computeIntegral(input);
    expect(Array.from(out)).toEqual([0, 1, 0, 2]);
  });

  it("renders pulse and wave kernels", () => {
    const duty = computeIntegral(new Float32Array([1, -1, 1, -1, 1, -1, 1, -1]));
    const [pulse] = pulseKernel(32, 1_000_000, 0, duty, 1000);
    expect(pulse.length).toBe(32);

    const wave = computeIntegral(new Float32Array(new Array(32).fill(0).map((_, i) => Math.sin(i))));
    const [mono] = waveKernel(32, 1_000_000, 0, wave, 1000);
    expect(mono.length).toBe(32);
  });

  it("renders noise and mutates lfsr state", () => {
    const state = createChannelState();
    const before = state.noise_lfsr;
    const envelope = new Float64Array(64);
    envelope.fill(12 / 15);
    const out = noiseKernelWrapper(64, { period_num: 32, period_den: 1, width_mode: 1 }, envelope, state);
    expect(out.length).toBe(64);
    expect(state.noise_lfsr).not.toBe(before);
  });

  it("keeps pulse kernel output stable around phase wrap", () => {
    const pattern = computeIntegral(new Float32Array([1, 0, 0, 0, 0, 1, 1, 1]));
    const precision = 2 ** 48;
    const freq = 261.625565;
    const inc = Math.trunc((freq * precision) / 44_100);
    const phase = Math.trunc((6 * precision) / 8);
    const [samples, nextPhase] = pulseKernel(128, inc, phase, pattern, 28_000);

    expect({
  nextPhase,
  firstSamples: Array.from(samples.slice(0, 12)).map((value) => Number(value.toFixed(3))),
  wrapWindow: Array.from(samples.slice(60, 72)).map((value) => Number(value.toFixed(3)))
}).toMatchInlineSnapshot(`
{
  "firstSamples": [
    28000,
    28000,
    28000,
    28000,
    28000,
    28000,
    28000,
    28000,
    28000,
    28000,
    28000,
    28000,
  ],
  "nextPhase": 143373985444864,
  "wrapWindow": [
    28000,
    28000,
    28000,
    5895.843,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
  ],
}
`);
  });

  it("keeps wave kernel output stable around phase wrap", () => {
    const pattern = new Float32Array(Array.from({ length: 32 }, (_, idx) => (((idx * 3) & 0xf) / 15.0) * 2.0 - 1.0));
    const integral = computeIntegral(pattern);
    const precision = 2 ** 48;
    const freq = 523.25113;
    const inc = Math.trunc((freq * precision) / 44_100);
    const phase = Math.trunc(precision / 32);
    const [samples, nextPhase] = waveKernel(128, inc, phase, integral, 28_000);

    expect({
  nextPhase,
  firstSamples: Array.from(samples.slice(0, 12)).map((value) => Number(value.toFixed(3))),
  wrapWindow: Array.from(samples.slice(60, 72)).map((value) => Number(value.toFixed(3)))
}).toMatchInlineSnapshot(`
{
  "firstSamples": [
    -16800,
    -16800,
    -12698.265,
    -5600.001,
    -5600.001,
    2603.472,
    5600.001,
    6705.209,
    16800,
    16800,
    22006.943,
    28000,
  ],
  "nextPhase": 154806575556736,
  "wrapWindow": [
    -4593.407,
    1866.668,
    1866.668,
    10708.33,
    13066.668,
    14810.066,
    24266.666,
    24266.666,
    -1062.25,
    -24266.666,
    -24266.666,
    -14319.797,
  ],
}
`);
  });

  it("keeps noise kernel output stable for drum-style constant envelope", () => {
    const state = createChannelState();
    const envelope = new Float64Array(512);
    envelope.fill(1.0);
    const samples = noiseKernelWrapper(512, { period_num: 896, period_den: 1, width_mode: 1 }, envelope, state);

    expect({
  lfsr: state.noise_lfsr,
  accumulator: Number(state.noise_accumulator.toFixed(6)),
  firstSamples: Array.from(samples.slice(0, 16)),
  middleSamples: Array.from(samples.slice(248, 264))
}).toMatchInlineSnapshot(`
{
  "accumulator": 710.971791,
  "firstSamples": [
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
  ],
  "lfsr": 385,
  "middleSamples": [
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
    -28000,
  ],
}
`);
  });
});

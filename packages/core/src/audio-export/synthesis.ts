import {
  GB_DUTY_PATTERNS,
  MAX_WAVE_AMPLITUDE,
  NOISE_CLOCK_HZ,
  SAMPLE_RATE,
} from "./constants";
import type { ChannelState, NoiseFrequency } from "./schemas";

export const computeIntegral = (pattern: Float32Array): Float32Array => {
  const out = new Float32Array(pattern.length + 1);
  let running = 0.0;
  for (let i = 0; i < pattern.length; i += 1) {
    running = Math.fround(running + pattern[i]);
    out[i + 1] = running;
  }
  return out;
};

export class PhaseAccumulator {
  private readonly sampleRate: number;
  private frequency = 440;
  private dutyIntegral: Float32Array;
  private phase = 0;

  constructor(sampleRate = SAMPLE_RATE) {
    this.sampleRate = sampleRate;
    const base = Float32Array.from(GB_DUTY_PATTERNS[2].map((v) => (v > 0 ? 1 : 0)));
    this.dutyIntegral = computeIntegral(base);
  }

  setFrequency(hz: number): void {
    this.frequency = Math.max(0.1, hz);
  }

  setDutyPattern(pattern: number[]): void {
    const base = Float32Array.from(pattern.map((v) => (v > 0 ? 1 : 0)));
    this.dutyIntegral = computeIntegral(base);
  }

  generateSamples(count: number): Float32Array {
    const inc = (this.frequency * 2 ** 48) / this.sampleRate;
    const [out, nextPhase] = pulseKernel(count, inc, this.phase, this.dutyIntegral, MAX_WAVE_AMPLITUDE);
    this.phase = nextPhase;
    return out;
  }
}

const lookupIntegral = (p0: number, p1: number, table: Float32Array): number => {
  const last = table.length - 1;
  const idx0 = Math.max(0, Math.min(last, Math.floor(p0)));
  const next0 = Math.min(last, idx0 + 1);
  const frac0 = p0 - idx0;
  const v0 = table[idx0] + (table[next0] - table[idx0]) * frac0;

  const idx1 = Math.max(0, Math.min(last, Math.floor(p1)));
  const next1 = Math.min(last, idx1 + 1);
  const frac1 = p1 - idx1;
  const v1 = table[idx1] + (table[next1] - table[idx1]) * frac1;
  return v1 - v0;
};

export const pulseKernel = (
  sampleCount: number,
  inc: number,
  phaseAcc: number,
  dutyIntegral: Float32Array,
  scale: number,
): [Float32Array, number] => {
  const out = new Float32Array(sampleCount);
  const prec = 2 ** 48;
  const factor = inc === 0 ? 0 : (prec / (inc * 8.0)) * scale;

  let phase = phaseAcc;
  for (let i = 0; i < sampleCount; i += 1) {
    const pStart = (phase * 8.0) / prec;
    const pEnd = ((phase + inc) * 8.0) / prec;
    let val = 0;
    if (pEnd >= 8.0) {
      val = lookupIntegral(pStart, 8.0, dutyIntegral) + lookupIntegral(0.0, pEnd - 8.0, dutyIntegral);
    } else {
      val = lookupIntegral(pStart, pEnd, dutyIntegral);
    }
    out[i] = val * factor;
    phase += inc;
    if (phase >= prec) {
      phase -= prec;
    }
  }

  return [out, phase];
};

export const waveKernel = (
  sampleCount: number,
  inc: number,
  phaseAcc: number,
  patternIntegral: Float32Array,
  scale: number,
): [Float32Array, number] => {
  const out = new Float32Array(sampleCount);
  const prec = 2 ** 48;
  const factor = inc === 0 ? 0 : (prec / (inc * 32.0)) * scale;

  let phase = phaseAcc;
  for (let i = 0; i < sampleCount; i += 1) {
    const pStart = (phase * 32.0) / prec;
    const pEnd = ((phase + inc) * 32.0) / prec;
    let val = 0;
    if (pEnd >= 32.0) {
      val = lookupIntegral(pStart, 32.0, patternIntegral) + lookupIntegral(0.0, pEnd - 32.0, patternIntegral);
    } else {
      val = lookupIntegral(pStart, pEnd, patternIntegral);
    }
    out[i] = val * factor;
    phase += inc;
    if (phase >= prec) {
      phase -= prec;
    }
  }

  return [out, phase];
};

export const noiseKernelWrapper = (
  sampleCount: number,
  frequency: NoiseFrequency,
  envelope: Float64Array,
  state: ChannelState,
): Int16Array => {
  const out = new Int16Array(sampleCount);
  const periodCycles = Math.max(1, Math.floor(frequency.period_num / Math.max(1, frequency.period_den)));
  const clocksPerSample = NOISE_CLOCK_HZ / SAMPLE_RATE;
  let acc = Math.max(0, state.noise_accumulator);
  let lfsr = state.noise_lfsr;

  for (let i = 0; i < sampleCount; i += 1) {
    const amp = MAX_WAVE_AMPLITUDE * Math.max(0, Math.min(1, envelope[i] ?? 0));
    let current = (lfsr & 1) === 0 ? amp : -amp;
    const remainingBeforeStep = periodCycles - acc;
    if (clocksPerSample <= remainingBeforeStep) {
      out[i] = toInt16Trunc(current);
      acc += clocksPerSample;
      continue;
    }
    let sampleClocks = clocksPerSample;
    let integrated = 0;

    while (sampleClocks > 0) {
      const remaining = periodCycles - acc;
      if (sampleClocks <= remaining) {
        integrated += current * sampleClocks;
        acc += sampleClocks;
        sampleClocks = 0;
      } else {
        integrated += current * remaining;
        sampleClocks -= remaining;
        acc = 0;
        const fb = ((lfsr & 1) ^ ((lfsr >> 1) & 1)) & 1;
        lfsr = (lfsr >> 1) & 0x7fff;
        lfsr = (lfsr & ~(1 << 14)) | (fb << 14);
        if (frequency.width_mode !== 0) {
          lfsr = (lfsr & ~(1 << 6)) | (fb << 6);
        }
        current = (lfsr & 1) === 0 ? amp : -amp;
      }
    }

    out[i] = toInt16Trunc(integrated / clocksPerSample);
  }

  state.noise_lfsr = lfsr;
  state.noise_accumulator = acc;
  return out;
};

const toInt16Trunc = (value: number): number => {
  const nearest = Math.round(value);
  const normalized = Math.abs(value - nearest) < 1e-9 ? nearest : value;
  const truncated = normalized < 0 ? Math.ceil(normalized) : Math.floor(normalized);
  return Math.max(-32768, Math.min(32767, truncated));
};

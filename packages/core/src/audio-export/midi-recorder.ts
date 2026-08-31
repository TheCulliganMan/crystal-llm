import { SAMPLE_RATE } from "./constants";

type MidiEvent = {
  tick: number;
  order: number;
  payload: Uint8Array;
};

export class MidiRecorder {
  private readonly ticksPerBeat: number;
  private readonly tempoUsPerBeat: number;
  private readonly sampleRate: number;
  private readonly ticksPerSecond: number;
  private readonly events: MidiEvent[] = [];

  constructor(options?: {
    ticksPerBeat?: number;
    tempoUsPerBeat?: number;
    sampleRate?: number;
  }) {
    this.ticksPerBeat = options?.ticksPerBeat ?? 960;
    this.tempoUsPerBeat = options?.tempoUsPerBeat ?? 1_000_000;
    this.sampleRate = options?.sampleRate ?? SAMPLE_RATE;
    if (this.ticksPerBeat <= 0 || this.tempoUsPerBeat <= 0 || this.sampleRate <= 0) {
      throw new Error("Invalid MidiRecorder constructor argument.");
    }
    this.ticksPerSecond = (this.ticksPerBeat * 1_000_000) / this.tempoUsPerBeat;
  }

  recordNote(options: {
    channel: number;
    note: number;
    startSample: number;
    durationSamples: number;
    velocity: number;
  }): void {
    if (options.durationSamples <= 0) {
      return;
    }
    const channel = clamp(options.channel, 0, 15);
    const note = clamp(options.note, 0, 127);
    const velocity = clamp(options.velocity, 1, 127);
    const start = this.samplesToTicks(options.startSample);
    const end = start + Math.max(1, this.samplesToTicks(options.durationSamples));
    this.events.push({
      tick: start,
      order: 2,
      payload: Uint8Array.of(0x90 | channel, note, velocity),
    });
    this.events.push({
      tick: end,
      order: 1,
      payload: Uint8Array.of(0x80 | channel, note, 0),
    });
  }

  recordProgramChange(channel: number, program: number): void {
    this.events.push({
      tick: 0,
      order: 0,
      payload: Uint8Array.of(0xc0 | clamp(channel, 0, 15), clamp(program, 0, 127)),
    });
  }

  recordMarker(label: string, sample: number): void {
    this.events.push({
      tick: this.samplesToTicks(sample),
      order: 0,
      payload: metaEvent(0x06, new TextEncoder().encode(label)),
    });
  }

  recordSequencerSpecific(payload: Uint8Array): void {
    this.events.push({ tick: 0, order: 0, payload: metaEvent(0x7f, payload) });
  }

  toBytes(): Uint8Array {
    const header = Uint8Array.of(
      0x4d, 0x54, 0x68, 0x64,
      0, 0, 0, 6,
      0, 0,
      0, 1,
      (this.ticksPerBeat >> 8) & 0xff,
      this.ticksPerBeat & 0xff,
    );
    const events = [...this.events].sort((left, right) =>
      left.tick === right.tick ? left.order - right.order : left.tick - right.tick,
    );
    const track: number[] = [];
    push(track, encodeVlq(0));
    push(
      track,
      metaEvent(
        0x51,
        Uint8Array.of(
          (this.tempoUsPerBeat >> 16) & 0xff,
          (this.tempoUsPerBeat >> 8) & 0xff,
          this.tempoUsPerBeat & 0xff,
        ),
      ),
    );
    let previousTick = 0;
    for (const event of events) {
      push(track, encodeVlq(event.tick - previousTick));
      push(track, event.payload);
      previousTick = event.tick;
    }
    track.push(0, 0xff, 0x2f, 0);
    const trackBytes = Uint8Array.from(track);
    const trackHeader = Uint8Array.of(
      0x4d, 0x54, 0x72, 0x6b,
      (trackBytes.length >>> 24) & 0xff,
      (trackBytes.length >>> 16) & 0xff,
      (trackBytes.length >>> 8) & 0xff,
      trackBytes.length & 0xff,
    );
    const output = new Uint8Array(header.length + trackHeader.length + trackBytes.length);
    output.set(header);
    output.set(trackHeader, header.length);
    output.set(trackBytes, header.length + trackHeader.length);
    return output;
  }

  private samplesToTicks(samples: number): number {
    return Math.max(0, Math.round((samples / this.sampleRate) * this.ticksPerSecond));
  }
}

function metaEvent(type: number, payload: Uint8Array): Uint8Array {
  const length = encodeVlq(payload.length);
  const output = new Uint8Array(2 + length.length + payload.length);
  output.set([0xff, type], 0);
  output.set(length, 2);
  output.set(payload, 2 + length.length);
  return output;
}

function encodeVlq(value: number): Uint8Array {
  let remaining = Math.max(0, Math.floor(value));
  const bytes = [remaining & 0x7f];
  while ((remaining >>= 7) > 0) {
    bytes.unshift(0x80 | (remaining & 0x7f));
  }
  return Uint8Array.from(bytes);
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(maximum, Math.floor(value)));
}

function push(target: number[], bytes: Uint8Array): void {
  target.push(...bytes);
}

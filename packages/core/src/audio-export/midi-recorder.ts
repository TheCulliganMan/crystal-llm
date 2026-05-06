import {
  MIDI_CONTROLLER_PAN,
  MIDI_CONTROLLER_VOLUME,
  SAMPLE_RATE,
} from "./constants";

type MidiNoteEvent = {
  startTick: number;
  durationTick: number;
  note: number;
  velocity: number;
  channel: number;
};

export class MidiRecorder {
  private readonly ticksPerBeat: number;
  private readonly tempoUsPerBeat: number;
  private readonly sampleRate: number;
  private readonly events = new Map<number, MidiNoteEvent[]>();
  private readonly ticksPerSecond: number;
  private loopStartTick: number | null = null;
  private loopEndTick: number | null = null;
  private controlEvents: Array<[number, number, Uint8Array]> = [];

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

    this.ticksPerSecond = (this.ticksPerBeat * 1_000_000.0) / this.tempoUsPerBeat;
  }

  recordNote(options: {
    channel: number;
    note: number;
    startSample: number;
    durationSamples: number;
    velocity: number;
  }): void {
    const channel = clamp(options.channel, 0, 15);
    const note = clamp(options.note, 0, 127);
    const velocity = clamp(options.velocity, 1, 127);
    if (options.durationSamples <= 0) {
      return;
    }

    const startTick = this.samplesToTicks(options.startSample);
    const durationTick = Math.max(1, this.samplesToTicks(options.durationSamples));
    const entry: MidiNoteEvent = { channel, note, velocity, startTick, durationTick };

    const list = this.events.get(channel) ?? [];
    list.push(entry);
    this.events.set(channel, list);
  }

  setLoopPoints(options: { startSample: number | null; endSample: number | null }): void {
    if (options.startSample == null || options.endSample == null) {
      this.loopStartTick = null;
      this.loopEndTick = null;
      return;
    }
    const start = Math.max(0, options.startSample);
    const end = Math.max(0, options.endSample);
    if (end <= start) {
      throw new Error("Loop end must be greater than loop start.");
    }
    this.loopStartTick = this.samplesToTicks(start);
    this.loopEndTick = this.samplesToTicks(end);
  }

  recordProgramChange(options: { channel: number; program: number; sampleOffset?: number }): void {
    const channel = clamp(options.channel, 0, 15);
    const program = clamp(options.program, 0, 127);
    const tick = this.samplesToTicks(Math.max(0, options.sampleOffset ?? 0));
    this.controlEvents.push([tick, 0, new Uint8Array([0xc0 | channel, program])]);
  }

  recordPan(options: { channel: number; pan: number; sampleOffset?: number }): void {
    const channel = clamp(options.channel, 0, 15);
    const pan = clamp(options.pan, 0, 127);
    const tick = this.samplesToTicks(Math.max(0, options.sampleOffset ?? 0));
    this.controlEvents.push([tick, 0, new Uint8Array([0xb0 | channel, MIDI_CONTROLLER_PAN, pan])]);
  }

  recordVolume(options: { channel: number; volume: number; sampleOffset?: number }): void {
    const channel = clamp(options.channel, 0, 15);
    const volume = clamp(options.volume, 0, 127);
    const tick = this.samplesToTicks(Math.max(0, options.sampleOffset ?? 0));
    this.controlEvents.push([tick, 0, new Uint8Array([0xb0 | channel, MIDI_CONTROLLER_VOLUME, volume])]);
  }

  toBytes(): Uint8Array {
    const header = this.buildHeader();
    const track = this.buildTrack();
    const out = new Uint8Array(header.length + track.length);
    out.set(header, 0);
    out.set(track, header.length);
    return out;
  }

  private samplesToTicks(samples: number): number {
    const seconds = Math.max(0, samples) / this.sampleRate;
    return Math.floor(seconds * this.ticksPerSecond + 0.5);
  }

  private buildHeader(): Uint8Array {
    const out = new Uint8Array(14);
    out.set([0x4d, 0x54, 0x68, 0x64], 0); // MThd
    out.set([0, 0, 0, 6], 4);
    out.set([0, 0], 8); // format 0
    out.set([0, 1], 10); // one track
    out.set([(this.ticksPerBeat >> 8) & 0xff, this.ticksPerBeat & 0xff], 12);
    return out;
  }

  private buildTrack(): Uint8Array {
    const events: Array<[number, number, Uint8Array]> = [...this.controlEvents];

    for (const channelEvents of this.events.values()) {
      for (const evt of channelEvents) {
        events.push([evt.startTick, 2, new Uint8Array([0x90 | (evt.channel & 0xf), evt.note & 0x7f, evt.velocity & 0x7f])]);
        events.push([evt.startTick + evt.durationTick, 1, new Uint8Array([0x80 | (evt.channel & 0xf), evt.note & 0x7f, 0])]);
      }
    }

    for (const [tick, marker] of [
      [this.loopStartTick, "loopStart"],
      [this.loopEndTick, "loopEnd"],
    ] as const) {
      if (tick == null) {
        continue;
      }
      const encoded = new TextEncoder().encode(marker);
      const payload = new Uint8Array(3 + encoded.length);
      payload.set([0xff, 0x06, encoded.length], 0);
      payload.set(encoded, 3);
      events.push([tick, 0, payload]);
    }

    events.sort((a, b) => (a[0] === b[0] ? a[1] - b[1] : a[0] - b[0]));

    const bytes: number[] = [];
    pushBytes(bytes, encodeVlq(0));
    pushBytes(bytes, new Uint8Array([0xff, 0x51, 0x03, (this.tempoUsPerBeat >> 16) & 0xff, (this.tempoUsPerBeat >> 8) & 0xff, this.tempoUsPerBeat & 0xff]));

    let lastTick = 0;
    for (const [tick, _order, payload] of events) {
      const delta = Math.max(0, tick - lastTick);
      pushBytes(bytes, encodeVlq(delta));
      pushBytes(bytes, payload);
      lastTick = tick;
    }

    bytes.push(0x00, 0xff, 0x2f, 0x00);

    const trackData = Uint8Array.from(bytes);
    const header = new Uint8Array(8);
    header.set([0x4d, 0x54, 0x72, 0x6b], 0); // MTrk
    const len = trackData.length;
    header.set([(len >> 24) & 0xff, (len >> 16) & 0xff, (len >> 8) & 0xff, len & 0xff], 4);

    const out = new Uint8Array(header.length + trackData.length);
    out.set(header, 0);
    out.set(trackData, header.length);
    return out;
  }
}

const clamp = (value: number, lo: number, hi: number): number => Math.max(lo, Math.min(hi, Math.floor(value)));

const encodeVlq = (value: number): Uint8Array => {
  let v = Math.max(0, Math.floor(value));
  const buffer = [v & 0x7f];
  while ((v >>= 7)) {
    buffer.unshift(0x80 | (v & 0x7f));
  }
  return Uint8Array.from(buffer);
};

const pushBytes = (target: number[], payload: Uint8Array): void => {
  for (const b of payload) {
    target.push(b);
  }
};

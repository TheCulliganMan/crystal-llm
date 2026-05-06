import { Midi } from "@tonejs/midi";
import {
  GB_DUTY_PATTERNS,
  MAX_WAVE_AMPLITUDE,
  MIDI_PROGRAM_PULSE,
  MIDI_PROGRAM_WAVE,
  SAMPLE_RATE,
} from "./constants";
import {
  assertMidiDurationLimit,
  assertMidiFileByteLength,
  assertMidiFiniteNonNegativeSeconds,
  MIDI_MAX_NOTES,
  MIDI_MAX_TRACKS,
} from "./midi-safety";
import { computeIntegral, pulseKernel, waveKernel } from "./synthesis";

export interface ScheduledNote {
  note: number;
  startSeconds: number;
  durationSeconds: number;
  velocity?: number;
  voice?: "pulse" | "wave";
  dutyCycle?: number;
}

export interface MidiLoopPoints {
  startSeconds: number | null;
  endSeconds: number | null;
}

export interface MidiSequence {
  notes: ScheduledNote[];
  loopPoints: MidiLoopPoints;
  durationSeconds: number;
}

export interface MidiRenderResult {
  buffer: Int16Array;
  sampleRate: number;
  loopStartSample: number | null;
  loopEndSample: number | null;
}

export class GameBoyMidiInstrument {
  private readonly sampleRate: number;
  private readonly masterVolume: number;
  private readonly pulseIntegrals: Record<number, Float32Array>;
  private readonly pulsePhases: Record<number, number>;
  private readonly waveIntegral: Float32Array;
  private wavePhase = 0;

  constructor(options?: {
    sampleRate?: number;
    masterVolume?: number;
    waveTableNybbles?: number[];
  }) {
    this.sampleRate = options?.sampleRate ?? SAMPLE_RATE;
    this.masterVolume = options?.masterVolume ?? 0.4;
    if (this.sampleRate <= 0 || this.masterVolume <= 0) {
      throw new Error("sampleRate and masterVolume must be positive");
    }

    this.pulseIntegrals = {};
    this.pulsePhases = {};
    for (const [keyRaw, pattern] of Object.entries(GB_DUTY_PATTERNS)) {
      const key = Number(keyRaw);
      const signed = Float32Array.from(pattern.map((v) => (v > 0 ? 1 : -1)));
      this.pulseIntegrals[key] = computeIntegral(signed);
      this.pulsePhases[key] = 0;
    }

    const wave = normalizeWaveTable(options?.waveTableNybbles);
    this.waveIntegral = computeIntegral(wave);
  }

  renderNote(note: number, durationSeconds: number, options?: {
    velocity?: number;
    voice?: "pulse" | "wave";
    dutyCycle?: number;
  }): Int16Array {
    if (durationSeconds <= 0) {
      throw new Error("durationSeconds must be positive");
    }
    if (note < 0 || note > 127) {
      throw new Error("note must be in [0, 127]");
    }

    const velocity = options?.velocity ?? 100;
    const voice = options?.voice ?? "pulse";
    const dutyCycle = options?.dutyCycle ?? 2;

    const samples = Math.max(1, Math.round(durationSeconds * this.sampleRate));
    const freq = midiNoteToFrequency(note);
    const scale = MAX_WAVE_AMPLITUDE * (Math.max(1, Math.min(127, velocity)) / 127.0) * this.masterVolume;

    let mono: Float32Array;
    if (voice === "pulse") {
      const integral = this.pulseIntegrals[dutyCycle];
      if (!integral) {
        throw new Error(`Unsupported dutyCycle ${dutyCycle}`);
      }
      const inc = (freq * 2 ** 48) / this.sampleRate;
      const [rendered, newPhase] = pulseKernel(samples, inc, this.pulsePhases[dutyCycle], integral, scale);
      this.pulsePhases[dutyCycle] = newPhase;
      mono = rendered;
    } else {
      const inc = (freq * 2 ** 48) / this.sampleRate;
      const [rendered, newPhase] = waveKernel(samples, inc, this.wavePhase, this.waveIntegral, scale);
      this.wavePhase = newPhase;
      mono = rendered;
    }

    return toStereoInt16(mono);
  }

  renderNotes(notes: ScheduledNote[]): Int16Array {
    if (notes.length === 0) {
      throw new Error("At least one note is required");
    }

    let end = 0;
    for (const note of notes) {
      assertMidiFiniteNonNegativeSeconds(note.startSeconds, "Scheduled note startSeconds");
      if (!Number.isFinite(note.durationSeconds) || note.durationSeconds <= 0) {
        throw new Error("Scheduled note durationSeconds must be finite and positive");
      }
      const noteEnd = note.startSeconds + note.durationSeconds;
      assertMidiDurationLimit(noteEnd, "Scheduled note end time");
      end = Math.max(end, noteEnd);
    }
    assertMidiDurationLimit(end, "Scheduled note end time");

    const totalSamples = Math.max(1, Math.ceil(end * this.sampleRate));
    const mix = new Int32Array(totalSamples * 2);

    const ordered = [...notes].sort((a, b) => a.startSeconds - b.startSeconds);
    for (const note of ordered) {
      const start = Math.round(note.startSeconds * this.sampleRate);
      const rendered = this.renderNote(note.note, note.durationSeconds, {
        velocity: note.velocity ?? 100,
        voice: note.voice ?? "pulse",
        dutyCycle: note.dutyCycle ?? 2,
      });
      const frames = rendered.length / 2;
      for (let i = 0; i < frames; i += 1) {
        const idx = start + i;
        if (idx >= totalSamples) {
          throw new Error("Scheduled note exceeds allocated buffer length");
        }
        mix[idx * 2] += rendered[i * 2];
        mix[idx * 2 + 1] += rendered[i * 2 + 1];
      }
    }

    return clipInt32Stereo(mix);
  }

  renderMidiFileWithLoops(midiBytes: ArrayBuffer, channelVoiceMap?: Record<number, "pulse" | "wave">): MidiRenderResult {
    const sequence = midiFileToSequence(midiBytes, channelVoiceMap);
    const buffer = this.renderNotes(sequence.notes);
    const totalSamples = Math.floor(buffer.length / 2);

    let loopStartSample: number | null = null;
    let loopEndSample: number | null = null;
    if (sequence.loopPoints.startSeconds != null && sequence.loopPoints.endSeconds != null && sequence.loopPoints.endSeconds > sequence.loopPoints.startSeconds) {
      loopStartSample = Math.max(0, Math.round(sequence.loopPoints.startSeconds * this.sampleRate));
      loopEndSample = Math.min(totalSamples, Math.round(sequence.loopPoints.endSeconds * this.sampleRate));
      if (loopEndSample <= loopStartSample) {
        loopStartSample = null;
        loopEndSample = null;
      }
    }

    return {
      buffer,
      sampleRate: this.sampleRate,
      loopStartSample,
      loopEndSample,
    };
  }
}

export const midiNoteToFrequency = (note: number): number => 440 * Math.pow(2, (note - 69) / 12);

export const toStereoInt16 = (mono: Float32Array): Int16Array => {
  const out = new Int16Array(mono.length * 2);
  for (let i = 0; i < mono.length; i += 1) {
    const value = Math.max(-32768, Math.min(32767, Math.round(mono[i])));
    out[i * 2] = value;
    out[i * 2 + 1] = value;
  }
  return out;
};

export const clipInt32Stereo = (stereo: Int32Array): Int16Array => {
  const out = new Int16Array(stereo.length);
  for (let i = 0; i < stereo.length; i += 1) {
    out[i] = Math.max(-32768, Math.min(32767, stereo[i]));
  }
  return out;
};

const normalizeWaveTable = (waveTableNybbles?: number[]): Float32Array => {
  const defaults = [0, 2, 4, 6, 8, 10, 12, 14, 15, 15, 15, 14, 14, 13, 13, 12, 12, 11, 10, 9, 8, 7, 6, 5, 4, 4, 3, 3, 2, 2, 1, 1];
  const values = waveTableNybbles ?? defaults;
  if (values.length !== 32) {
    throw new Error("Wave tables must contain 32 nybbles");
  }
  return Float32Array.from(values.map((v) => (v - 7.5) / 7.5));
};

export const midiFileToSequence = (midiBytes: ArrayBuffer, channelVoiceMap?: Record<number, "pulse" | "wave">): MidiSequence => {
  assertMidiFileByteLength(midiBytes.byteLength);
  const midi = new Midi(midiBytes);
  if (midi.tracks.length > MIDI_MAX_TRACKS) {
    throw new Error(`MIDI has too many tracks (${midi.tracks.length}). Limit is ${MIDI_MAX_TRACKS}`);
  }
  assertMidiDurationLimit(midi.duration, "MIDI duration");

  const map = { ...(channelVoiceMap ?? {}) };
  const notes: ScheduledNote[] = [];
  let loopStart: number | null = null;
  let loopEnd: number | null = null;

  for (const track of midi.tracks) {
    const channel = track.channel ?? 0;
    if (track.instrument.number === MIDI_PROGRAM_WAVE && map[channel] == null) {
      map[channel] = "wave";
    }
    if (track.instrument.number === MIDI_PROGRAM_PULSE && map[channel] == null) {
      map[channel] = "pulse";
    }

    for (const note of track.notes) {
      assertMidiFiniteNonNegativeSeconds(note.time, "MIDI note start time");
      if (!Number.isFinite(note.duration) || note.duration < 0) {
        throw new Error("MIDI note duration must be finite and non-negative");
      }
      const durationSeconds = Math.max(0.001, note.duration);
      const endSeconds = note.time + durationSeconds;
      assertMidiDurationLimit(endSeconds, "MIDI note end time");
      if (!Number.isFinite(note.midi) || note.midi < 0 || note.midi > 127) {
        throw new Error(`MIDI note value out of range: ${note.midi}`);
      }
      const voice = map[channel] ?? (channel === 2 ? "wave" : "pulse");
      notes.push({
        note: note.midi,
        startSeconds: note.time,
        durationSeconds,
        velocity: Math.max(1, Math.round(note.velocity * 127)),
        voice,
      });
      if (notes.length > MIDI_MAX_NOTES) {
        throw new Error(`MIDI has too many note events (${notes.length}). Limit is ${MIDI_MAX_NOTES}`);
      }
    }

  }

  for (const meta of midi.header.meta ?? []) {
    const label = meta.text.trim().toLowerCase();
    const time = midi.header.ticksToSeconds(meta.ticks);
    assertMidiFiniteNonNegativeSeconds(time, "MIDI metadata time");
    assertMidiDurationLimit(time, "MIDI metadata time");
    if ((label === "loopstart" || label === "loop start") && loopStart == null) {
      loopStart = time;
    }
    if (label === "loopend" || label === "loop end") {
      loopEnd = time;
    }
  }

  if (loopStart != null && loopEnd == null) {
    loopEnd = midi.duration;
  }
  if (loopStart != null) {
    assertMidiDurationLimit(loopStart, "MIDI loop start");
  }
  if (loopEnd != null) {
    assertMidiDurationLimit(loopEnd, "MIDI loop end");
  }

  return {
    notes: notes.sort((a, b) => (a.startSeconds === b.startSeconds ? a.note - b.note : a.startSeconds - b.startSeconds)),
    loopPoints: { startSeconds: loopStart, endSeconds: loopEnd },
    durationSeconds: midi.duration,
  };
};

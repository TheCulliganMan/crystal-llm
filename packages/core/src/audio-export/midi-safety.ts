import { MAX_CHANNEL_DURATION_SECONDS } from "./constants";

export const MIDI_MAX_FILE_BYTES = 2 * 1024 * 1024;
export const MIDI_MAX_TRACKS = 64;
export const MIDI_MAX_NOTES = 16_384;
export const MIDI_MAX_DURATION_SECONDS = MAX_CHANNEL_DURATION_SECONDS;

const toKib = (bytes: number): number => Math.ceil(bytes / 1024);

export const assertMidiFileByteLength = (byteLength: number): void => {
  if (!Number.isFinite(byteLength) || byteLength <= 0) {
    throw new Error("MIDI file is empty");
  }
  if (byteLength > MIDI_MAX_FILE_BYTES) {
    throw new Error(`MIDI file is too large. Limit is ${toKib(MIDI_MAX_FILE_BYTES)} KB`);
  }
};

export const assertMidiFiniteNonNegativeSeconds = (seconds: number, label: string): void => {
  if (!Number.isFinite(seconds) || seconds < 0) {
    throw new Error(`${label} must be finite and non-negative`);
  }
};

export const assertMidiDurationLimit = (seconds: number, label: string): void => {
  assertMidiFiniteNonNegativeSeconds(seconds, label);
  if (seconds > MIDI_MAX_DURATION_SECONDS) {
    throw new Error(`${label} exceeds ${MIDI_MAX_DURATION_SECONDS}s safety limit`);
  }
};

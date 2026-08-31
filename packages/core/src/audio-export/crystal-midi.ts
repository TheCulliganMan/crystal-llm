import type { ParsedMusicData } from "./parsers";
import { MidiRecorder } from "./midi-recorder";

const CRYSTAL_MIDI_MAGIC = new TextEncoder().encode("POKECRYSTAL-MIDI-1\0");

export type CrystalMidiProgram = {
  profile: "pokecrystal-midi-v1";
  music_data: ParsedMusicData;
  cry_pitch: number | null;
  cry_length: number | null;
};

export function buildCrystalMidi(
  program: CrystalMidiProgram,
  notes: Array<{
    channel: number;
    note: number;
    startSample: number;
    durationSamples: number;
    velocity: number;
  }> = [],
  loop?: { startSample: number; endSample: number } | null,
): Uint8Array {
  const recorder = new MidiRecorder();
  for (const channel of [0, 1, 2, 9]) {
    recorder.recordProgramChange(channel, channel === 9 ? 0 : 80);
  }
  for (const note of notes) {
    recorder.recordNote(note);
  }
  if (loop) {
    recorder.recordMarker("loopStart", loop.startSample);
    recorder.recordMarker("loopEnd", loop.endSample);
  }
  const json = new TextEncoder().encode(JSON.stringify(program));
  const payload = new Uint8Array(CRYSTAL_MIDI_MAGIC.length + json.length);
  payload.set(CRYSTAL_MIDI_MAGIC);
  payload.set(json, CRYSTAL_MIDI_MAGIC.length);
  recorder.recordSequencerSpecific(payload);
  return recorder.toBytes();
}

export function parseCrystalMidi(bytes: Uint8Array): CrystalMidiProgram {
  if (readAscii(bytes, 0, 4) !== "MThd") {
    throw new Error("Crystal MIDI is missing the MThd header");
  }
  let offset = 8 + readU32(bytes, 4);
  while (offset + 8 <= bytes.length) {
    const chunk = readAscii(bytes, offset, 4);
    const length = readU32(bytes, offset + 4);
    offset += 8;
    const end = offset + length;
    if (end > bytes.length) {
      throw new Error("Crystal MIDI contains a truncated chunk");
    }
    if (chunk === "MTrk") {
      const program = parseTrackForCrystalProgram(bytes, offset, end);
      if (program) {
        return program;
      }
    }
    offset = end;
  }
  throw new Error("MIDI does not contain a PokeCrystal sequencer profile");
}

export function countMidiNoteOnEvents(bytes: Uint8Array): number {
  if (readAscii(bytes, 0, 4) !== "MThd") {
    throw new Error("MIDI is missing the MThd header");
  }
  let offset = 8 + readU32(bytes, 4);
  let count = 0;
  while (offset + 8 <= bytes.length) {
    const chunk = readAscii(bytes, offset, 4);
    const length = readU32(bytes, offset + 4);
    offset += 8;
    const end = offset + length;
    if (end > bytes.length) {
      throw new Error("MIDI contains a truncated chunk");
    }
    if (chunk === "MTrk") {
      count += countTrackNoteOnEvents(bytes, offset, end);
    }
    offset = end;
  }
  return count;
}

function countTrackNoteOnEvents(bytes: Uint8Array, start: number, end: number): number {
  let offset = start;
  let runningStatus = 0;
  let count = 0;
  while (offset < end) {
    [, offset] = readVlq(bytes, offset, end);
    let status = bytes[offset++];
    if (status < 0x80) {
      if (!runningStatus) {
        throw new Error("MIDI running status has no preceding status byte");
      }
      offset -= 1;
      status = runningStatus;
    } else if (status < 0xf0) {
      runningStatus = status;
    }
    if (status === 0xff || status === 0xf0 || status === 0xf7) {
      if (status === 0xff) {
        offset += 1;
      }
      const [length, payloadStart] = readVlq(bytes, offset, end);
      offset = payloadStart + length;
      continue;
    }
    const command = status & 0xf0;
    if (command === 0xc0 || command === 0xd0) {
      offset += 1;
      continue;
    }
    const velocity = bytes[offset + 1];
    if (command === 0x90 && velocity > 0) {
      count += 1;
    }
    offset += 2;
  }
  return count;
}

function parseTrackForCrystalProgram(
  bytes: Uint8Array,
  start: number,
  end: number,
): CrystalMidiProgram | null {
  let offset = start;
  let runningStatus = 0;
  while (offset < end) {
    [, offset] = readVlq(bytes, offset, end);
    let status = bytes[offset++];
    if (status < 0x80) {
      if (!runningStatus) {
        throw new Error("MIDI running status has no preceding status byte");
      }
      offset -= 1;
      status = runningStatus;
    } else if (status < 0xf0) {
      runningStatus = status;
    }
    if (status === 0xff) {
      const type = bytes[offset++];
      const [length, payloadStart] = readVlq(bytes, offset, end);
      offset = payloadStart;
      const payloadEnd = offset + length;
      if (payloadEnd > end) {
        throw new Error("MIDI meta event is truncated");
      }
      if (
        type === 0x7f &&
        startsWith(bytes.subarray(offset, payloadEnd), CRYSTAL_MIDI_MAGIC)
      ) {
        const json = new TextDecoder().decode(
          bytes.subarray(offset + CRYSTAL_MIDI_MAGIC.length, payloadEnd),
        );
        const program = JSON.parse(json) as CrystalMidiProgram;
        if (program.profile !== "pokecrystal-midi-v1" || !program.music_data) {
          throw new Error("PokeCrystal MIDI profile is malformed");
        }
        return program;
      }
      offset = payloadEnd;
      continue;
    }
    if (status === 0xf0 || status === 0xf7) {
      const [length, payloadStart] = readVlq(bytes, offset, end);
      offset = payloadStart + length;
      continue;
    }
    const command = status & 0xf0;
    offset += command === 0xc0 || command === 0xd0 ? 1 : 2;
  }
  return null;
}

function readVlq(bytes: Uint8Array, start: number, end: number): [number, number] {
  let value = 0;
  let offset = start;
  for (let count = 0; count < 4 && offset < end; count += 1) {
    const byte = bytes[offset++];
    value = (value << 7) | (byte & 0x7f);
    if ((byte & 0x80) === 0) {
      return [value, offset];
    }
  }
  throw new Error("MIDI contains an invalid variable-length value");
}

function readU32(bytes: Uint8Array, offset: number): number {
  return (
    bytes[offset] * 0x1000000 +
    (bytes[offset + 1] << 16) +
    (bytes[offset + 2] << 8) +
    bytes[offset + 3]
  );
}

function readAscii(bytes: Uint8Array, offset: number, length: number): string {
  return String.fromCharCode(...bytes.subarray(offset, offset + length));
}

function startsWith(bytes: Uint8Array, prefix: Uint8Array): boolean {
  return bytes.length >= prefix.length && prefix.every((byte, index) => bytes[index] === byte);
}

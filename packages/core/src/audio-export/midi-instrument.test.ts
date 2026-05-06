import { Midi } from "@tonejs/midi";
import {
  GameBoyMidiInstrument,
  midiFileToSequence,
} from "@pokecrystal/core/audio-export/midi-instrument";
import {
  MIDI_MAX_DURATION_SECONDS,
  MIDI_MAX_FILE_BYTES,
  MIDI_MAX_NOTES,
} from "@pokecrystal/core/audio-export/midi-safety";

const toArrayBuffer = (bytes: Uint8Array): ArrayBuffer =>
  bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);

describe("GameBoyMidiInstrument", () => {
  it("renders a single note", () => {
    const inst = new GameBoyMidiInstrument({ sampleRate: 8_000, masterVolume: 0.2 });
    const out = inst.renderNote(60, 0.1);
    expect(out.length).toBeGreaterThan(0);
  });

  it("parses midi sequence", () => {
    const midi = new Midi();
    const track = midi.addTrack();
    track.addNote({ midi: 60, time: 0, duration: 0.25, velocity: 0.8 });
    const bytes = midi.toArray();
    const sequence = midiFileToSequence(toArrayBuffer(bytes));
    expect(sequence.notes.length).toBe(1);
    expect(sequence.notes[0].note).toBe(60);
  });

  it("rejects oversized midi payloads", () => {
    expect(() => midiFileToSequence(new ArrayBuffer(MIDI_MAX_FILE_BYTES + 1))).toThrow(
      /MIDI file is too large/i,
    );
  });

  it("rejects midi files that exceed duration safety limits", () => {
    const midi = new Midi();
    const track = midi.addTrack();
    track.addNote({
      midi: 60,
      time: MIDI_MAX_DURATION_SECONDS + 1,
      duration: 0.25,
      velocity: 0.8,
    });
    const bytes = midi.toArray();

    expect(() => midiFileToSequence(toArrayBuffer(bytes))).toThrow(
      new RegExp(`exceeds ${MIDI_MAX_DURATION_SECONDS}s safety limit`),
    );
  });

  it("rejects midi files with too many note events", () => {
    const midi = new Midi();
    const track = midi.addTrack();
    for (let i = 0; i <= MIDI_MAX_NOTES; i += 1) {
      track.addNote({
        midi: 48 + (i % 24),
        time: i * 0.01,
        duration: 0.01,
        velocity: 0.5,
      });
    }
    const bytes = midi.toArray();

    expect(() => midiFileToSequence(toArrayBuffer(bytes))).toThrow(/too many note events/i);
  });
});

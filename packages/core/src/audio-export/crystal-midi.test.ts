import {
  buildCrystalMidi,
  countMidiNoteOnEvents,
  parseCrystalMidi,
  type CrystalMidiProgram,
} from "./crystal-midi";

describe("PokeCrystal MIDI profile", () => {
  it("is a valid MIDI container and round-trips the exact Crystal program", () => {
    const program: CrystalMidiProgram = {
      profile: "pokecrystal-midi-v1",
      music_data: {
        channel_count: 1,
        channels: {
          Test_Ch1: {
            number: 1,
            commands: [{ command: "note", args: ["C_", "4"] }],
          },
        },
        subroutines: {},
      },
      cry_pitch: null,
      cry_length: null,
    };
    const midi = buildCrystalMidi(program, [
      { channel: 0, note: 60, startSample: 0, durationSamples: 22050, velocity: 100 },
    ]);
    expect(new TextDecoder().decode(midi.subarray(0, 4))).toBe("MThd");
    expect(new TextDecoder().decode(midi.subarray(14, 18))).toBe("MTrk");
    expect(parseCrystalMidi(midi)).toEqual(program);
    expect(countMidiNoteOnEvents(midi)).toBe(1);
  });
});

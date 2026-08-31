import {
  initializeCrystalAudioSynth,
  synthesizeCrystalMidi,
} from "./browser-synth";
import { PcmConverter } from "./converter";
import { buildCrystalMidi } from "./crystal-midi";
import type { ParsedMusicData } from "./parsers";

describe("browser audio synthesis", () => {
  it("recreates the canonical downsampled PCM without shipping PCM bytes", () => {
    const musicData: ParsedMusicData = {
      channel_count: 1,
      channels: {
        Test_Ch1: {
          number: 1,
          commands: [
            { command: "tempo", args: ["0x100"] },
            { command: "duty_cycle", args: ["2"] },
            { command: "note_type", args: ["12", "12", "2"] },
            { command: "octave", args: ["4"] },
            { command: "note", args: ["C_", "4"] },
            { command: "sound_ret", args: [] },
          ],
        },
      },
      subroutines: {},
    };
    const context = { drumkits: {}, waveSamples: {} };
    initializeCrystalAudioSynth(context);
    const midi = buildCrystalMidi({
      profile: "pokecrystal-midi-v1",
      music_data: musicData,
      cry_pitch: null,
      cry_length: null,
    });
    const result = synthesizeCrystalMidi(Buffer.from(midi).toString("base64"));
    const canonical = new PcmConverter(
      musicData,
      context.drumkits,
      context.waveSamples,
      { qualityMode: "accurate", loopedMusicExportSeconds: null },
    ).convert("pcm");
    const expected = Array.from(
      { length: Math.ceil(canonical.stereo.length / 4) * 2 },
      (_, index) => {
        const frame = Math.floor(index / 2) * 2;
        return canonical.stereo[frame * 2 + (index % 2)];
      },
    );
    expect(result.sampleRate).toBe(22_050);
    expect(Array.from(result.samples)).toEqual(expected);
  });
});

import { createHash } from "node:crypto";
import { PcmConverter } from "@pokecrystal/core/audio-export/converter";
import type { ParsedMusicData } from "@pokecrystal/core/audio-export/parsers";
import type { NoiseNote } from "@pokecrystal/core/audio-export/schemas";

type QualityMode = "accurate" | "enhanced";
type Drumkits = Record<number, Record<number, NoiseNote[]>>;
type WaveSamples = Record<number, number[]>;

interface RegressionFixture {
  name: string;
  musicData: ParsedMusicData;
  drumkits?: Drumkits;
  waveSamples?: WaveSamples;
  waveInstrumentMap?: Record<number, number>;
  qualityMode: QualityMode;
}

const wavePattern = Array.from({ length: 32 }, (_, idx) => (idx * 3) & 0xf);
const drumkitsFixture: Drumkits = {
  1: {
    0: [
      { length: 4, volume: 15, fade: 0, frequency: 0x2f },
      { length: 8, volume: 8, fade: 2, frequency: 0x1f },
    ],
  },
};

const fixtures: RegressionFixture[] = [
  {
    name: "pulse-loop-baseline-accurate",
    musicData: {
      channel_count: 1,
      channels: {
        Music_Test_Ch1: {
          number: 1,
          commands: [
            { command: "note_type", args: ["12"] },
            { command: "label", args: [".mainloop"] },
            { command: "duty_cycle", args: ["2"] },
            { command: "octave", args: ["4"] },
            { command: "note", args: ["C_", "4"] },
            { command: "rest", args: ["2"] },
            { command: "sound_loop", args: ["1", ".mainloop"] },
          ],
        },
      },
      subroutines: {},
    },
    qualityMode: "accurate",
  },
  {
    name: "wave-channel-vibrato",
    musicData: {
      channel_count: 1,
      channels: {
        Music_Test_Ch3: {
          number: 3,
          commands: [
            { command: "note_type", args: ["12", "2", "4"] },
            { command: "vibrato", args: ["2", "0x21"] },
            { command: "octave", args: ["4"] },
            { command: "note", args: ["C_", "8"] },
            { command: "channel_volume", args: ["3"] },
            { command: "note", args: ["G_", "8"] },
          ],
        },
      },
      subroutines: {},
    },
    waveSamples: { 4: wavePattern },
    qualityMode: "accurate",
  },
  {
    name: "noise-and-drum",
    musicData: {
      channel_count: 1,
      channels: {
        Music_Test_Ch4: {
          number: 4,
          commands: [
            { command: "drum_speed", args: ["12"] },
            { command: "toggle_noise", args: ["1"] },
            { command: "drum_note", args: ["0", "8"] },
            { command: "noise_note", args: ["4", "10", "0", "0x2f"] },
            { command: "rest", args: ["2"] },
          ],
        },
      },
      subroutines: {},
    },
    drumkits: drumkitsFixture,
    qualityMode: "accurate",
  },
];

const summarizePcm = (pcm: Int16Array) => {
  const bytes = Buffer.from(pcm.buffer, pcm.byteOffset, pcm.byteLength);
  return {
    hash: createHash("sha256").update(bytes).digest("hex"),
    sampleCount: pcm.length,
    firstSamples: Array.from(pcm.slice(0, 16)),
    lastSamples: Array.from(pcm.slice(-16)),
  };
};

describe("audio-export regression fixtures", () => {
  it("produces stable PCM signatures for ASM-derived fixtures", () => {
    const summary = fixtures.map(({ name, musicData, drumkits = {}, waveSamples = { 0: new Array(32).fill(0) }, waveInstrumentMap, qualityMode }) => {
      const converter = new PcmConverter(
        musicData,
        drumkits,
        waveSamples,
        waveInstrumentMap != null ? { qualityMode, waveInstrumentMap } : { qualityMode },
      );

      const firstPass = converter.convert("pcm");
      const secondPass = converter.convert("pcm");
      expect(Buffer.from(firstPass.stereo.buffer, firstPass.stereo.byteOffset, firstPass.stereo.byteLength).equals(
        Buffer.from(secondPass.stereo.buffer, secondPass.stereo.byteOffset, secondPass.stereo.byteLength),
      )).toBe(true);

      return {
        name,
        qualityMode,
        durationSeconds: Number(firstPass.metadata.durationSeconds.toFixed(6)),
        ...summarizePcm(firstPass.stereo),
      };
    });

    expect(summary).toMatchInlineSnapshot(`
[
  {
    "durationSeconds": 1.205465,
    "firstSamples": [
      28000,
      28000,
      28000,
      28000,
      27999,
      27999,
      27999,
      27999,
      27998,
      27998,
      27997,
      27997,
      27996,
      27996,
      27995,
      27995,
    ],
    "hash": "50b51d9142ccd899179e6074904c2ebf153d3844bba78673f2966f47a8d809da",
    "lastSamples": [
      -5152,
      -5152,
      -5152,
      -5152,
      -5152,
      -5152,
      -5152,
      -5152,
      -5151,
      -5151,
      -5151,
      -5151,
      -5151,
      -5151,
      -5151,
      -5151,
    ],
    "name": "pulse-loop-baseline-accurate",
    "qualityMode": "accurate",
    "sampleCount": 106322,
  },
  {
    "durationSeconds": 3.21458,
    "firstSamples": [
      -24266,
      -24266,
      -24266,
      -24266,
      -24266,
      -24266,
      -24265,
      -24265,
      -24264,
      -24264,
      -23486,
      -23486,
      -22221,
      -22221,
      -20951,
      -20951,
    ],
    "hash": "cc6dce0f0408931a3af4dd0eee60f7b47758288a64179faf05f889511a6b861a",
    "lastSamples": [
      -1256,
      -1256,
      -605,
      -605,
      100,
      100,
      1104,
      1104,
      2078,
      2078,
      2901,
      2901,
      2351,
      2351,
      932,
      932,
    ],
    "name": "wave-channel-vibrato",
    "qualityMode": "accurate",
    "sampleCount": 283526,
  },
  {
    "durationSeconds": 3.013673,
    "firstSamples": [
      -28000,
      -28000,
      -28000,
      -28000,
      -27999,
      -27999,
      -27999,
      -27999,
      -27998,
      -27998,
      -27997,
      -27997,
      -27996,
      -27996,
      -27995,
      -27995,
    ],
    "hash": "342002f7452de8fb8b9292713ccbe2d2903143b0e99bc06648f43c94d24feea1",
    "lastSamples": [
      176,
      176,
      176,
      176,
      176,
      176,
      176,
      176,
      176,
      176,
      176,
      176,
      176,
      176,
      176,
      176,
    ],
    "name": "noise-and-drum",
    "qualityMode": "accurate",
    "sampleCount": 265806,
  },
]
`);
  });
});

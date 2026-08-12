import { WavConverter } from "@pokecrystal/core/audio-export/converter";
import type { ParsedMusicData } from "@pokecrystal/core/audio-export/parsers";

describe("WavConverter", () => {
  const baseMusicData: ParsedMusicData = {
    channel_count: 1,
    channels: {
      Music_Test_Ch1: {
        number: 1,
        commands: [
          { command: "note_type", args: ["12"] },
          { command: "label", args: [".mainloop"] },
          { command: "note", args: ["C_", "4"] },
          { command: "rest", args: ["2"] },
          { command: "sound_loop", args: ["1", ".mainloop"] },
        ],
      },
    },
    subroutines: {},
  };

  it("renders PCM output", () => {
    const converter = new WavConverter(baseMusicData, {}, { 0: new Array(32).fill(0) });
    const result = converter.convert("pcm");
    expect(result.stereo.length).toBeGreaterThan(0);
    expect(result.sampleRate).toBe(44_100);
  });

  it("applies cartridge cry pitch and length parameters to every tonal channel", () => {
    const cryData: ParsedMusicData = {
      channel_count: 2,
      channels: {
        Cry_Test_Ch5: {
          number: 5,
          commands: [
            { command: "square_note", args: ["0", "15", "0", "1024"] },
          ],
        },
        Cry_Test_Ch8: {
          number: 8,
          commands: [
            { command: "drum_speed", args: ["1"] },
            { command: "rest", args: ["1"] },
          ],
        },
      },
      subroutines: {},
    };

    const base = new WavConverter(cryData, {}, { 0: new Array(32).fill(0) }, {
      cryPitch: 0,
      cryLength: 0x100,
    }).convert("pcm");
    const altered = new WavConverter(cryData, {}, { 0: new Array(32).fill(0) }, {
      cryPitch: 0x80,
      cryLength: 0x200,
    }).convert("pcm");

    expect(altered.stereo.length / base.stereo.length).toBeCloseTo(2, 2);
    expect(Array.from(altered.stereo.slice(0, 512))).not.toEqual(Array.from(base.stereo.slice(0, 512)));
  });

  it("renders MIDI bytes", () => {
    const converter = new WavConverter(baseMusicData, {}, { 0: new Array(32).fill(0) });
    const result = converter.convert("midi");
    expect(result.midiBytes).toBeDefined();
    expect(result.midiBytes?.length).toBeGreaterThan(10);
  });

  it("throws on unsupported command", () => {
    const data: ParsedMusicData = {
      ...baseMusicData,
      channels: {
        Music_Test_Ch1: {
          number: 1,
          commands: [{ command: "unknown_command", args: [] }],
        },
      },
    };
    const converter = new WavConverter(data, {}, { 0: new Array(32).fill(0) });
    expect(() => converter.convert("pcm")).toThrow("Unsupported ASM audio command");
  });

  it("supports boolean stereo_panning tokens", () => {
    const data: ParsedMusicData = {
      channel_count: 1,
      channels: {
        Music_Test_Ch1: {
          number: 1,
          commands: [
            { command: "note_type", args: ["12"] },
            { command: "stereo_panning", args: ["TRUE", "FALSE"] },
            { command: "note", args: ["C_", "8"] },
          ],
        },
      },
      subroutines: {},
    };
    const converter = new WavConverter(data, {}, { 0: new Array(32).fill(0) });
    const stereo = converter.convert("pcm").stereo;
    let leftPeak = 0;
    let rightPeak = 0;
    for (let i = 0; i < stereo.length / 2; i += 1) {
      leftPeak = Math.max(leftPeak, Math.abs(stereo[i * 2]));
      rightPeak = Math.max(rightPeak, Math.abs(stereo[i * 2 + 1]));
    }
    expect(leftPeak).toBeGreaterThan(0);
    expect(rightPeak).toBe(0);
  });

  it("applies pulse volume envelope decay", () => {
    const data: ParsedMusicData = {
      channel_count: 1,
      channels: {
        Music_Test_Ch1: {
          number: 1,
          commands: [
            { command: "note_type", args: ["12", "15", "3"] },
            { command: "note", args: ["C_", "32"] },
          ],
        },
      },
      subroutines: {},
    };
    const converter = new WavConverter(data, {}, { 0: new Array(32).fill(15) });
    const stereo = converter.convert("pcm").stereo;
    const frameCount = stereo.length / 2;
    const quarter = Math.max(1, Math.floor(frameCount / 4));
    const startAvg = averageAbs(stereo, 0, quarter);
    const endAvg = averageAbs(stereo, frameCount - quarter, frameCount);
    expect(startAvg).toBeGreaterThan(endAvg);
  });

  it("gates drum_note output behind toggle_noise", () => {
    const drumkits = {
      1: {
        0: [
          { length: 4, volume: 15, fade: 0, frequency: 0x2f },
        ],
      },
    };

    const mutedData: ParsedMusicData = {
      channel_count: 1,
      channels: {
        Music_Test_Ch4: {
          number: 4,
          commands: [
            { command: "drum_speed", args: ["12"] },
            { command: "drum_note", args: ["0", "8"] },
          ],
        },
      },
      subroutines: {},
    };
    const audibleData: ParsedMusicData = {
      channel_count: 1,
      channels: {
        Music_Test_Ch4: {
          number: 4,
          commands: [
            { command: "drum_speed", args: ["12"] },
            { command: "toggle_noise", args: ["1"] },
            { command: "drum_note", args: ["0", "8"] },
          ],
        },
      },
      subroutines: {},
    };

    const muted = new WavConverter(mutedData, drumkits, { 0: new Array(32).fill(0) }).convert("pcm").stereo;
    const audible = new WavConverter(audibleData, drumkits, { 0: new Array(32).fill(0) }).convert("pcm").stereo;
    expect(maxAbs(muted)).toBe(0);
    expect(maxAbs(audible)).toBeGreaterThan(0);
  });

  it("treats wave volume 0 as silence on wave channels", () => {
    const waveSample = new Array(32).fill(0).map((_, idx) => (idx % 16));
    const data: ParsedMusicData = {
      channel_count: 1,
      channels: {
        Music_Test_Ch3: {
          number: 3,
          commands: [
            { command: "note_type", args: ["12", "0", "4"] },
            { command: "note", args: ["C_", "16"] },
          ],
        },
      },
      subroutines: {},
    };
    const converter = new WavConverter(data, {}, { 4: waveSample });
    const stereo = converter.convert("pcm").stereo;
    expect(maxAbs(stereo)).toBe(0);
  });

  it("supports square_note with pitch_sweep", () => {
    const data: ParsedMusicData = {
      channel_count: 1,
      channels: {
        Music_Test_Ch1: {
          number: 1,
          commands: [
            { command: "note_type", args: ["12"] },
            { command: "pitch_sweep", args: ["1", "-6"] },
            { command: "square_note", args: ["8", "12", "2", "1792"] },
            { command: "square_note", args: ["8", "10", "2", "1728"] },
          ],
        },
      },
      subroutines: {},
    };
    const stereo = new WavConverter(data, {}, { 0: new Array(32).fill(0) }).convert("pcm").stereo;
    expect(maxAbs(stereo)).toBeGreaterThan(0);
  });

  it("supports vibrato and pitch_slide commands", () => {
    const data: ParsedMusicData = {
      channel_count: 1,
      channels: {
        Music_Test_Ch1: {
          number: 1,
          commands: [
            { command: "note_type", args: ["12", "12", "2"] },
            { command: "vibrato", args: ["1", "10", "3"] },
            { command: "note", args: ["C_", "8"] },
            { command: "pitch_slide", args: ["4", "4", "G_"] },
            { command: "note", args: ["E_", "8"] },
          ],
        },
      },
      subroutines: {},
    };
    const stereo = new WavConverter(data, {}, { 0: new Array(32).fill(0) }).convert("pcm").stereo;
    expect(maxAbs(stereo)).toBeGreaterThan(0);
  });

  it("supports duty_cycle_pattern and toggle_perfect_pitch", () => {
    const data: ParsedMusicData = {
      channel_count: 1,
      channels: {
        Music_Test_Ch1: {
          number: 1,
          commands: [
            { command: "note_type", args: ["12", "12", "2"] },
            { command: "duty_cycle_pattern", args: ["0", "1", "2", "3"] },
            { command: "toggle_perfect_pitch", args: [] },
            { command: "note", args: ["C_", "8"] },
            { command: "rest", args: ["4"] },
            { command: "note", args: ["C_", "8"] },
          ],
        },
      },
      subroutines: {},
    };
    const stereo = new WavConverter(data, {}, { 0: new Array(32).fill(0) }).convert("pcm").stereo;
    expect(maxAbs(stereo)).toBeGreaterThan(0);
  });

  it("applies NR50 volume command from the primary channel", () => {
    const data: ParsedMusicData = {
      channel_count: 1,
      channels: {
        Music_Test_Ch1: {
          number: 1,
          commands: [
            { command: "note_type", args: ["12", "15", "0"] },
            { command: "note", args: ["C_", "8"] },
            { command: "volume", args: ["0", "0"] },
            { command: "note", args: ["C_", "8"] },
          ],
        },
      },
      subroutines: {},
    };
    const stereo = new WavConverter(data, {}, { 0: new Array(32).fill(0) }).convert("pcm").stereo;
    const frames = stereo.length / 2;
    const half = Math.max(1, Math.floor(frames / 2));
    const firstHalf = averageAbs(stereo, 0, half);
    const secondHalf = averageAbs(stereo, half, frames);
    expect(firstHalf).toBeGreaterThan(0);
    expect(secondHalf).toBeLessThan(firstHalf * 0.6);
  });

  it("inherits the primary channel tempo for sibling SFX channels that omit tempo", () => {
    const data: ParsedMusicData = {
      channel_count: 2,
      channels: {
        Sfx_Test_Ch5: {
          number: 5,
          commands: [
            { command: "toggle_sfx", args: [] },
            { command: "tempo", args: ["64"] },
            { command: "note_type", args: ["4"] },
            { command: "note", args: ["C_", "4"] },
          ],
        },
        Sfx_Test_Ch6: {
          number: 6,
          commands: [
            { command: "toggle_sfx", args: [] },
            { command: "note_type", args: ["4"] },
            { command: "note", args: ["C_", "4"] },
          ],
        },
      },
      subroutines: {},
    };

    const result = new WavConverter(data, {}, { 0: new Array(32).fill(0) }).convert("pcm");

    expect(result.metadata.durationSeconds).toBeCloseTo(0.066961, 3);
  });

  it("applies primary-channel tempo changes to sibling music channels and preserves solo stem length", () => {
    const data: ParsedMusicData = {
      channel_count: 2,
      channels: {
        Music_Test_Ch1: {
          number: 1,
          commands: [
            { command: "tempo", args: ["64"] },
            { command: "note_type", args: ["4"] },
            { command: "note", args: ["C_", "4"] },
            { command: "tempo", args: ["128"] },
            { command: "note", args: ["C_", "4"] },
          ],
        },
        Music_Test_Ch2: {
          number: 2,
          commands: [
            { command: "note_type", args: ["4"] },
            { command: "note", args: ["C_", "4"] },
            { command: "note", args: ["C_", "4"] },
          ],
        },
      },
      subroutines: {},
    };

    const full = new WavConverter(data, {}, { 0: new Array(32).fill(0) }, { loopedMusicExportSeconds: null }).convert();
    const solo = new WavConverter(data, {}, { 0: new Array(32).fill(0) }, {
      loopedMusicExportSeconds: null,
      soloChannel: 2,
    }).convert();

    expect(full.metadata.durationSeconds).toBeCloseTo(0.201, 3);
    expect(solo.stereo.length).toBe(full.stereo.length);
    expect(solo.metadata.durationSeconds).toBe(full.metadata.durationSeconds);
  });

  it("expands scoped subroutine sources emitted by the ASM parser", () => {
    const data: ParsedMusicData = {
      channel_count: 1,
      channels: {
        Music_Test_Ch1: {
          number: 1,
          commands: [
            { command: "note_type", args: ["4"] },
            { command: "note", args: ["C_", "4"] },
            { command: "sound_call", args: ["Music_Test_Ch1.sub1"] },
          ],
        },
      },
      subroutines: {
        "Music_Test_Ch1.sub1": {
          commands: [
            { command: "label", args: [".sub1"] },
            { command: "note", args: ["D_", "4"] },
            { command: "sound_ret", args: [] },
          ],
        },
      },
    };

    const result = new WavConverter(data, {}, { 0: new Array(32).fill(0) }, { loopedMusicExportSeconds: null }).convert();

    expect(result.metadata.durationSeconds).toBeCloseTo(0.536, 3);
  });
});

const maxAbs = (samples: Int16Array): number => {
  let peak = 0;
  for (let i = 0; i < samples.length; i += 1) {
    peak = Math.max(peak, Math.abs(samples[i]));
  }
  return peak;
};

const averageAbs = (stereo: Int16Array, startFrame: number, endFrame: number): number => {
  let total = 0;
  let count = 0;
  for (let frame = startFrame; frame < endFrame; frame += 1) {
    total += Math.abs(stereo[frame * 2]);
    count += 1;
  }
  return count > 0 ? total / count : 0;
};

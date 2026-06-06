import {
  pcmBytesToInt16,
  pcmClipToBytes,
  renderPcmClip,
} from "@pokecrystal/core/audio-export/pcm-clip";
import type { ParsedMusicData } from "@pokecrystal/core/audio-export/parsers";

const waveSamples = { 0: new Array(32).fill(0) };

describe("PcmClip rendering", () => {
  it("renders looped music as a bounded intro plus one loop body", () => {
    const musicData: ParsedMusicData = {
      channel_count: 3,
      channels: {
        Music_Test_Ch1: {
          number: 1,
          commands: [
            { command: "note_type", args: ["12", "15", "0"] },
            { command: "octave", args: ["4"] },
            { command: "label", args: [".mainloop"] },
            { command: "note", args: ["C_", "4"] },
            { command: "sound_loop", args: ["0", ".mainloop"] },
          ],
        },
        Music_Test_Ch2: {
          number: 2,
          commands: [
            { command: "note_type", args: ["12", "12", "0"] },
            { command: "octave", args: ["4"] },
            { command: "label", args: [".mainloop"] },
            { command: "note", args: ["E_", "4"] },
            { command: "sound_loop", args: ["0", ".mainloop"] },
          ],
        },
        Music_Test_Ch3: {
          number: 3,
          commands: [
            { command: "note_type", args: ["12", "2", "4"] },
            { command: "octave", args: ["4"] },
            { command: "label", args: [".mainloop"] },
            { command: "note", args: ["G_", "4"] },
            { command: "sound_loop", args: ["0", ".mainloop"] },
          ],
        },
      },
      subroutines: {},
    };

    const clip = renderPcmClip({
      kind: "music",
      token: "MUSIC_TEST",
      musicData,
      context: { drumkits: {}, waveSamples },
    });

    expect(clip.sampleRate).toBe(44_100);
    expect(clip.loopStartSample).toBe(0);
    expect(clip.loopEndSample).toBe(Math.floor(clip.pcm.length / 2));
    expect(clip.durationFrames).toBeLessThan(60);
    expect(clip.ownedChannels).toEqual([1, 2, 3]);
  });

  it("round-trips raw little-endian stereo PCM bytes", () => {
    const bytes = pcmClipToBytes({ pcm: new Int16Array([0, 1234, -1234, 32767]) });
    expect(Array.from(pcmBytesToInt16(bytes.buffer))).toEqual([0, 1234, -1234, 32767]);
  });

  it("returns SFX duration, channel ownership, and priority metadata", () => {
    const musicData: ParsedMusicData = {
      channel_count: 1,
      channels: {
        Sfx_Fanfare_Ch5: {
          number: 5,
          commands: [
            { command: "note_type", args: ["12", "15", "0"] },
            { command: "octave", args: ["4"] },
            { command: "note", args: ["C_", "4"] },
            { command: "sound_ret", args: [] },
          ],
        },
      },
      subroutines: {},
    };

    const clip = renderPcmClip({
      kind: "sfx",
      token: "SFX_FANFARE",
      musicData,
      context: { drumkits: {}, waveSamples },
    });

    expect(clip.durationFrames).toBeGreaterThan(0);
    expect(clip.ownedChannels).toEqual([5]);
    expect(clip.priorityClass).toBe("priority");
    expect(clip.loopStartSample).toBeNull();
  });
});

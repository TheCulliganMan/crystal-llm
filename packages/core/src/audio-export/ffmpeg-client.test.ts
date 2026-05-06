import { convertMidiToMp3Client } from "@pokecrystal/core/audio-export/ffmpeg-client";
import { MIDI_MAX_FILE_BYTES } from "@pokecrystal/core/audio-export/midi-safety";

describe("convertMidiToMp3Client", () => {
  it("rejects oversized midi uploads before reading bytes", async () => {
    const arrayBuffer = jest.fn(async () => new ArrayBuffer(8));
    const oversizedFile = {
      name: "huge.mid",
      size: MIDI_MAX_FILE_BYTES + 1,
      arrayBuffer,
    } as unknown as File;

    await expect(convertMidiToMp3Client(oversizedFile)).rejects.toMatchObject({
      code: "INVALID_INPUT",
      message: expect.stringMatching(/too large/i),
    });
    expect(arrayBuffer).not.toHaveBeenCalled();
  });
});

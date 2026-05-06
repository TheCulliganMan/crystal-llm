/** @jest-environment jsdom */

import "@testing-library/jest-dom";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AudioConvertClient, computeSpectrumFrame } from "@/app/audio/audio-convert-client";
import { convertMidiToMp3Client, resetFfmpegClient } from "@pokecrystal/core/audio-export/ffmpeg-client";
import { MIDI_MAX_FILE_BYTES } from "@pokecrystal/core/audio-export/midi-safety";
import { midiFileToSequence } from "@pokecrystal/core/audio-export/midi-instrument";

jest.mock("@pokecrystal/core/audio-export/ffmpeg-client", () => ({
  convertMidiToMp3Client: jest.fn(async () => ({
    mp3Blob: new Blob([new Uint8Array([1, 2, 3])], { type: "audio/mpeg" }),
    previewUrl: "blob:test",
    metadata: {
      inputSampleRate: 44_100,
      outputSampleRate: 22_050,
      bitrate: "64k",
      durationSeconds: 1.2,
      loopStartSample: null,
      loopEndSample: null,
    },
    diagnostics: { noteCount: 5 },
  })),
  resetFfmpegClient: jest.fn(),
  AudioConvertError: class AudioConvertError extends Error {
    code: string;
    constructor(message: string, code: string) {
      super(message);
      this.code = code;
    }
  },
}));

jest.mock("@pokecrystal/core/audio-export/midi-instrument", () => ({
  midiFileToSequence: jest.fn(() => ({
    notes: [
      { note: 60, startSeconds: 0, durationSeconds: 0.5, velocity: 100, voice: "pulse" },
      { note: 64, startSeconds: 0.5, durationSeconds: 0.5, velocity: 100, voice: "wave" },
    ],
    loopPoints: { startSeconds: 0.25, endSeconds: 0.9 },
    durationSeconds: 1.2,
  })),
}));

describe("AudioConvertClient", () => {
  const originalFetch = global.fetch;

  const makeMidiFile = (name = "test.mid"): File => {
    const file = new File([new Uint8Array([0x4d, 0x54, 0x68, 0x64])], name, { type: "audio/midi" });
    Object.defineProperty(file, "arrayBuffer", {
      value: jest.fn(async () => new Uint8Array([0x4d, 0x54, 0x68, 0x64]).buffer),
      configurable: true,
    });
    return file;
  };

  beforeEach(() => {
    jest.clearAllMocks();
    global.fetch = jest.fn(async () => ({
      ok: true,
      status: 200,
      arrayBuffer: async () => new Uint8Array([0x4d, 0x54, 0x68, 0x64]).buffer,
    })) as unknown as typeof fetch;
  });

  afterAll(() => {
    global.fetch = originalFetch;
  });

  it("converts after selecting a midi file", async () => {
    render(<AudioConvertClient />);

    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    const file = makeMidiFile("test.mid");

    fireEvent.change(input, { target: { files: [file] } });
    await waitFor(() => {
      expect(screen.getByText("test.mid")).toBeInTheDocument();
      expect(screen.getByText(/Track Visualization/i)).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: /convert to mp3/i }));

    await waitFor(() => {
      expect(screen.getByText(/Duration:/)).toBeInTheDocument();
    });
  });

  it("shows drop zone copy and validates bad extension", async () => {
    render(<AudioConvertClient />);
    expect(screen.getByText(/Drop your MIDI file here/i)).toBeInTheDocument();

    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    const file = new File([new Uint8Array([1, 2, 3])], "bad.txt", { type: "text/plain" });
    fireEvent.change(input, { target: { files: [file] } });

    expect(screen.getByText(/INVALID_INPUT:/)).toBeInTheDocument();
  });

  it("rejects oversized midi uploads before parse", async () => {
    render(<AudioConvertClient />);
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    const file = new File(
      [new Uint8Array(MIDI_MAX_FILE_BYTES + 1)],
      "too-big.mid",
      { type: "audio/midi" },
    );
    fireEvent.change(input, { target: { files: [file] } });

    await waitFor(() => {
      expect(screen.getByText(/INVALID_INPUT: MIDI file is too large/i)).toBeInTheDocument();
    });
    expect(midiFileToSequence).not.toHaveBeenCalled();
  });

  it("shows retry for ffmpeg init failures", async () => {
    (convertMidiToMp3Client as jest.Mock).mockRejectedValueOnce(
      new Error("FFMPEG_INIT_FAILED: ffmpeg wasm failed to initialize"),
    );

    render(<AudioConvertClient />);
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    const file = makeMidiFile("test.mid");

    fireEvent.change(input, { target: { files: [file] } });
    await waitFor(() => {
      expect(screen.getByText("test.mid")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: /convert to mp3/i }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /retry ffmpeg init/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /retry ffmpeg init/i }));
    expect(resetFfmpegClient).toHaveBeenCalledTimes(1);
  });

  it("disables convert for invalid numeric options", async () => {
    render(<AudioConvertClient />);

    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    const file = makeMidiFile("test.mid");
    fireEvent.change(input, { target: { files: [file] } });
    await waitFor(() => {
      expect(screen.getByText("test.mid")).toBeInTheDocument();
    });

    const volumeField = screen.getByLabelText(/master volume/i);
    const sampleRateField = screen.getByLabelText(/sample rate/i);
    fireEvent.change(volumeField, { target: { value: "0" } });
    fireEvent.change(sampleRateField, { target: { value: "999999" } });

    const convertButton = screen.getByRole("button", { name: /convert to mp3/i });
    expect(convertButton).toBeDisabled();
    expect(convertMidiToMp3Client).not.toHaveBeenCalled();
  });

  it("passes validated numeric options to converter", async () => {
    render(<AudioConvertClient />);

    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    const file = makeMidiFile("bounds.mid");
    fireEvent.change(input, { target: { files: [file] } });
    await waitFor(() => {
      expect(screen.getByText("bounds.mid")).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText(/master volume/i), { target: { value: "1" } });
    fireEvent.change(screen.getByLabelText(/sample rate/i), { target: { value: "96000" } });
    fireEvent.click(screen.getByRole("button", { name: /convert to mp3/i }));

    await waitFor(() => {
      expect(convertMidiToMp3Client).toHaveBeenCalledWith(
        expect.objectContaining({ name: "bounds.mid" }),
        expect.objectContaining({
          masterVolume: 1,
          sampleRate: 96000,
        }),
      );
    });
  });

  it("reports midi parse failure when parser throws", async () => {
    (midiFileToSequence as jest.Mock).mockImplementationOnce(() => {
      throw new Error("bad midi stream");
    });

    render(<AudioConvertClient />);
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    const file = makeMidiFile("bad.mid");
    fireEvent.change(input, { target: { files: [file] } });

    await waitFor(() => {
      expect(screen.getByText(/MIDI_PARSE_FAILED:/)).toBeInTheDocument();
    });
  });

  it("loads a built-in sample midi without file upload", async () => {
    render(<AudioConvertClient />);

    fireEvent.click(screen.getByRole("button", { name: /route 29/i }));
    await waitFor(() => {
      expect(screen.getByText("route29.mid")).toBeInTheDocument();
      expect(screen.getByText(/Track Visualization/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /convert to mp3/i }));
    await waitFor(() => {
      expect(convertMidiToMp3Client).toHaveBeenCalledWith(
        expect.objectContaining({ name: "route29.mid" }),
        expect.objectContaining({ masterVolume: 0.4, sampleRate: 44_100 }),
      );
    });
  });
});

describe("computeSpectrumFrame", () => {
  it("distributes energy across bars using logarithmic buckets", () => {
    const freq = new Uint8Array(128);
    freq[2] = 180;
    freq[8] = 210;
    freq[28] = 235;
    freq[64] = 190;
    const frame = computeSpectrumFrame(freq, 16, 0.08);

    const activeBars = frame.bars.filter((value) => value > 0.05).length;
    expect(activeBars).toBeGreaterThanOrEqual(4);
    expect(frame.nextPeak).toBeGreaterThan(0.7);
  });

  it("normalizes bar values to avoid clipping while preserving peaks", () => {
    const freq = new Uint8Array(64).fill(250);
    const frame = computeSpectrumFrame(freq, 12, 0.2);

    expect(frame.bars.every((value) => value <= 1)).toBe(true);
    expect(frame.bars.some((value) => value > 0.95)).toBe(true);
    expect(frame.nextPeak).toBeGreaterThan(0.9);
  });
});

import {
  MP3_EXPORT_BITRATE,
  MP3_EXPORT_SAMPLE_RATE,
  SAMPLE_RATE,
} from "./constants";
import { assertMidiFileByteLength } from "./midi-safety";

export interface ConvertOptions {
  sampleRate?: number;
  masterVolume?: number;
  qualityMode?: "accurate" | "enhanced";
  outputSampleRate?: number;
  bitrate?: string;
}

export interface ConvertResult {
  mp3Blob: Blob;
  previewUrl: string;
  metadata: {
    inputSampleRate: number;
    outputSampleRate: number;
    bitrate: string;
    durationSeconds: number;
    loopStartSample: number | null;
    loopEndSample: number | null;
  };
  diagnostics: {
    noteCount: number;
  };
}

export class AudioConvertError extends Error {
  constructor(
    message: string,
    public readonly code:
      | "INVALID_INPUT"
      | "FFMPEG_INIT_FAILED"
      | "FFMPEG_ENCODE_FAILED"
      | "MIDI_PARSE_FAILED",
  ) {
    super(message);
    this.name = "AudioConvertError";
  }
}

type FFmpegLike = {
  load: (opts: { coreURL: string; wasmURL: string; classWorkerURL?: string }) => Promise<boolean | void>;
  writeFile: (name: string, data: Uint8Array) => Promise<void>;
  exec: (args: string[]) => Promise<void>;
  readFile: (name: string) => Promise<Uint8Array | ArrayBuffer | string>;
  deleteFile: (name: string) => Promise<void>;
  terminate?: () => void;
};

let ffmpeg: FFmpegLike | null = null;
let ffmpegReady = false;

const FFMPEG_CORE_URL = "/ffmpeg/ffmpeg-core.js";
const FFMPEG_WASM_URL = "/ffmpeg/ffmpeg-core.wasm";
const CDN_FFMPEG_CORE_URL = "https://unpkg.com/@ffmpeg/core@0.12.9/dist/umd/ffmpeg-core.js";
const CDN_FFMPEG_WASM_URL = "https://unpkg.com/@ffmpeg/core@0.12.9/dist/umd/ffmpeg-core.wasm";

const resolveAssetUrl = (assetPath: string): string => {
  if (typeof window === "undefined") {
    return assetPath;
  }
  return new URL(assetPath, window.location.origin).toString();
};

const createFfmpeg = async (): Promise<FFmpegLike> => {
  const ffmpegModule = await import("@ffmpeg/ffmpeg");
  return new ffmpegModule.FFmpeg() as unknown as FFmpegLike;
};

const messageFromError = (error: unknown): string =>
  error instanceof Error ? error.message : String(error);

const ensureFfmpeg = async (): Promise<FFmpegLike> => {
  if (!ffmpegReady) {
    const errors: string[] = [];
    const localCoreUrl = resolveAssetUrl(FFMPEG_CORE_URL);
    const localWasmUrl = resolveAssetUrl(FFMPEG_WASM_URL);

    const tryLoad = async (label: string, opts: { coreURL: string; wasmURL: string; useBlob?: boolean }): Promise<boolean> => {
      const instance = await createFfmpeg();
      try {
        if (opts.useBlob) {
          const utilModule = await import("@ffmpeg/util");
          const coreBlobUrl = await utilModule.toBlobURL(opts.coreURL, "text/javascript");
          const wasmBlobUrl = await utilModule.toBlobURL(opts.wasmURL, "application/wasm");
          await instance.load({ coreURL: coreBlobUrl, wasmURL: wasmBlobUrl });
        } else {
          await instance.load({ coreURL: opts.coreURL, wasmURL: opts.wasmURL });
        }
        ffmpeg = instance;
        ffmpegReady = true;
        return true;
      } catch (error) {
        instance.terminate?.();
        errors.push(`${label}=${messageFromError(error)}`);
        return false;
      }
    };

    if (await tryLoad("local_direct", { coreURL: localCoreUrl, wasmURL: localWasmUrl })) {
      return ffmpeg as FFmpegLike;
    }
    if (await tryLoad("local_blob", { coreURL: localCoreUrl, wasmURL: localWasmUrl, useBlob: true })) {
      return ffmpeg as FFmpegLike;
    }
    if (await tryLoad("cdn_blob", { coreURL: CDN_FFMPEG_CORE_URL, wasmURL: CDN_FFMPEG_WASM_URL, useBlob: true })) {
      return ffmpeg as FFmpegLike;
    }

    throw new AudioConvertError(
      `ffmpeg wasm failed to initialize (${errors.join("; ")})`,
      "FFMPEG_INIT_FAILED",
    );
  }
  if (!ffmpeg) {
    throw new AudioConvertError("ffmpeg wasm failed to initialize", "FFMPEG_INIT_FAILED");
  }
  return ffmpeg;
};

export const resetFfmpegClient = (): void => {
  ffmpeg?.terminate?.();
  ffmpeg = null;
  ffmpegReady = false;
};

export const convertMidiToMp3Client = async (
  input: File,
  options?: ConvertOptions,
): Promise<ConvertResult> => {
  if (!input || !/\.midi?$/i.test(input.name)) {
    throw new AudioConvertError("Please provide a .mid or .midi file", "INVALID_INPUT");
  }
  try {
    assertMidiFileByteLength(input.size);
  } catch (error) {
    throw new AudioConvertError(
      error instanceof Error ? error.message : "MIDI file failed safety checks",
      "INVALID_INPUT",
    );
  }

  const midiBytes = await input.arrayBuffer();
  const sampleRate = options?.sampleRate ?? SAMPLE_RATE;
  const masterVolume = options?.masterVolume ?? 0.4;
  const outputSampleRate = options?.outputSampleRate ?? MP3_EXPORT_SAMPLE_RATE;
  const bitrate = options?.bitrate ?? MP3_EXPORT_BITRATE;

  const midiModule = await import("./midi-instrument");
  let sequence;
  try {
    sequence = midiModule.midiFileToSequence(midiBytes);
  } catch (error) {
    throw new AudioConvertError(
      error instanceof Error ? error.message : "Failed to parse MIDI file",
      "MIDI_PARSE_FAILED",
    );
  }

  if (sequence.notes.length === 0) {
    throw new AudioConvertError("MIDI contains no note events", "INVALID_INPUT");
  }

  const instrument = new midiModule.GameBoyMidiInstrument({ sampleRate, masterVolume });
  const rendered = instrument.renderMidiFileWithLoops(midiBytes);
  const wavBytes = createWavFromStereo16(rendered.buffer, sampleRate);

  const ff = await ensureFfmpeg();
  const inName = `in-${Date.now()}.wav`;
  const outName = `out-${Date.now()}.mp3`;

  try {
    await ff.writeFile(inName, wavBytes);
    await ff.exec([
      "-i",
      inName,
      "-codec:a",
      "libmp3lame",
      "-ar",
      String(outputSampleRate),
      "-b:a",
      bitrate,
      outName,
    ]);

    const encoded = await ff.readFile(outName);
    await ff.deleteFile(inName);
    await ff.deleteFile(outName);

    if (typeof encoded === "string") {
      throw new AudioConvertError("Unexpected text payload from ffmpeg output", "FFMPEG_ENCODE_FAILED");
    }
    const bytes = encoded instanceof Uint8Array ? encoded : new Uint8Array(encoded as unknown as ArrayBuffer);
    const plainBuffer = new ArrayBuffer(bytes.byteLength);
    new Uint8Array(plainBuffer).set(bytes);
    const mp3Blob = new Blob([plainBuffer], { type: "audio/mpeg" });
    const previewUrl = URL.createObjectURL(mp3Blob);

    return {
      mp3Blob,
      previewUrl,
      metadata: {
        inputSampleRate: sampleRate,
        outputSampleRate,
        bitrate,
        durationSeconds: rendered.buffer.length / 2 / sampleRate,
        loopStartSample: rendered.loopStartSample,
        loopEndSample: rendered.loopEndSample,
      },
      diagnostics: {
        noteCount: sequence.notes.length,
      },
    };
  } catch (error) {
    throw new AudioConvertError(error instanceof Error ? error.message : "ffmpeg encode failed", "FFMPEG_ENCODE_FAILED");
  }
};

const createWavFromStereo16 = (interleavedStereo: Int16Array, sampleRate: number): Uint8Array => {
  const channels = 2;
  const bitsPerSample = 16;
  const byteRate = sampleRate * channels * (bitsPerSample / 8);
  const blockAlign = channels * (bitsPerSample / 8);
  const dataBytes = interleavedStereo.length * 2;
  const buffer = new ArrayBuffer(44 + dataBytes);
  const view = new DataView(buffer);

  writeAscii(view, 0, "RIFF");
  view.setUint32(4, 36 + dataBytes, true);
  writeAscii(view, 8, "WAVE");
  writeAscii(view, 12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, channels, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, byteRate, true);
  view.setUint16(32, blockAlign, true);
  view.setUint16(34, bitsPerSample, true);
  writeAscii(view, 36, "data");
  view.setUint32(40, dataBytes, true);

  let offset = 44;
  for (let i = 0; i < interleavedStereo.length; i += 1) {
    view.setInt16(offset, interleavedStereo[i], true);
    offset += 2;
  }

  return new Uint8Array(buffer);
};

const writeAscii = (view: DataView, offset: number, text: string): void => {
  for (let i = 0; i < text.length; i += 1) {
    view.setUint8(offset + i, text.charCodeAt(i));
  }
};

/// <reference lib="webworker" />

import { FFmpeg } from "@ffmpeg/ffmpeg";

type EncodeRequest = {
  id: string;
  type: "encode";
  pcm: ArrayBuffer;
  inputSampleRate: number;
  outputSampleRate: number;
  bitrate: string;
};

type InitRequest = {
  id: string;
  type: "init";
};

type WorkerRequest = EncodeRequest | InitRequest;

const ctx: DedicatedWorkerGlobalScope = self as unknown as DedicatedWorkerGlobalScope;
let ffmpeg: FFmpeg | null = null;
let loaded = false;

const ensureLoaded = async (): Promise<void> => {
  if (!ffmpeg) {
    ffmpeg = new FFmpeg();
  }
  if (loaded) {
    return;
  }

  await ffmpeg.load({
    coreURL: "/ffmpeg/ffmpeg-core.js",
    wasmURL: "/ffmpeg/ffmpeg-core.wasm",
  });
  loaded = true;
};

ctx.addEventListener("message", async (event: MessageEvent<WorkerRequest>) => {
  const msg = event.data;

  try {
    if (msg.type === "init") {
      await ensureLoaded();
      ctx.postMessage({ id: msg.id, ok: true, type: "init" });
      return;
    }

    await ensureLoaded();
    if (!ffmpeg) {
      throw new Error("ffmpeg failed to initialize");
    }

    const inName = `in-${msg.id}.wav`;
    const outName = `out-${msg.id}.mp3`;
    const wavBytes = createWavFromStereo16(new Int16Array(msg.pcm), msg.inputSampleRate);

    await ffmpeg.writeFile(inName, wavBytes);
    await ffmpeg.exec([
      "-i",
      inName,
      "-codec:a",
      "libmp3lame",
      "-ar",
      String(msg.outputSampleRate),
      "-b:a",
      msg.bitrate,
      outName,
    ]);

    const out = await ffmpeg.readFile(outName);
    await ffmpeg.deleteFile(inName);
    await ffmpeg.deleteFile(outName);

    if (typeof out === "string") {
      throw new Error("Unexpected text payload from ffmpeg output.");
    }
    const bytes = out instanceof Uint8Array ? out : new Uint8Array(out as unknown as ArrayBuffer);
    const plainBuffer = new ArrayBuffer(bytes.byteLength);
    new Uint8Array(plainBuffer).set(bytes);
    ctx.postMessage({ id: msg.id, ok: true, type: "encode", mp3: plainBuffer }, [plainBuffer]);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    ctx.postMessage({ id: msg.id, ok: false, error: message });
  }
});

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

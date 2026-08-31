import { PcmConverter } from "./converter";
import { parseCrystalMidi } from "./crystal-midi";
import type { NoiseNote } from "./schemas";

type BrowserSynthContext = {
  drumkits: Record<number, Record<number, NoiseNote[]>>;
  waveSamples: Record<number, number[]>;
  waveInstrumentMap?: Record<number, number>;
};

let context: BrowserSynthContext | null = null;

export function initializeCrystalAudioSynth(value: BrowserSynthContext): void {
  if (!value || !value.drumkits || !value.waveSamples) {
    throw new Error("Crystal audio synth context is malformed");
  }
  context = value;
}

export function synthesizeCrystalMidi(midiBase64: string): {
  samples: Int16Array;
  sampleRate: number;
} {
  if (!context) {
    throw new Error("Crystal audio synth is not initialized");
  }
  const midiBytes = Uint8Array.from(
    atob(midiBase64),
    (character) => character.charCodeAt(0),
  );
  const program = parseCrystalMidi(midiBytes);
  const rendered = new PcmConverter(
    program.music_data,
    context.drumkits,
    context.waveSamples,
    {
      qualityMode: "accurate",
      waveInstrumentMap: context.waveInstrumentMap,
      loopedMusicExportSeconds: null,
      cryPitch: program.cry_pitch,
      cryLength: program.cry_length,
    },
  ).convert("pcm");
  if (rendered.sampleRate !== 44_100 || rendered.stereo.length % 2 !== 0) {
    throw new Error("Crystal audio synth produced a non-canonical render");
  }
  const inputFrames = rendered.stereo.length / 2;
  const output = new Int16Array(Math.ceil(inputFrames / 2) * 2);
  for (
    let inputFrame = 0, outputFrame = 0;
    inputFrame < inputFrames;
    inputFrame += 2, outputFrame += 1
  ) {
    output[outputFrame * 2] = rendered.stereo[inputFrame * 2];
    output[outputFrame * 2 + 1] = rendered.stereo[inputFrame * 2 + 1];
  }
  return { samples: output, sampleRate: 22_050 };
}

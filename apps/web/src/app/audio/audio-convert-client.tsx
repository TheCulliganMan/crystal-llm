"use client";

import type { ChangeEvent, DragEvent, KeyboardEvent } from "react";
import { useEffect, useMemo, useRef, useState, type RefObject } from "react";
import {
  AudioConvertError,
  convertMidiToMp3Client,
  type ConvertResult,
  resetFfmpegClient,
} from "@pokecrystal/core/audio-export/ffmpeg-client";
import { assertMidiFileByteLength, MIDI_MAX_FILE_BYTES } from "@pokecrystal/core/audio-export/midi-safety";
import type { MidiSequence, ScheduledNote } from "@pokecrystal/core/audio-export/midi-instrument";

interface MidiSampleOption {
  id: string;
  title: string;
  subtitle: string;
  path: string;
  fileName: string;
}

const toClassName = (...values: Array<string | undefined | false | null>) =>
  values.filter(Boolean).join(" ");

const chipClassName =
  "rounded-full border border-base-300 bg-base-100 px-2.5 py-1 text-xs font-medium text-base-content/80";

const SAMPLE_MIDI_OPTIONS: MidiSampleOption[] = [
  {
    id: "newbark",
    title: "New Bark Town",
    subtitle: "Slow town ambience",
    path: "/api/audio/newbarktown.mid",
    fileName: "newbarktown.mid",
  },
  {
    id: "route29",
    title: "Route 29",
    subtitle: "Early-game route theme",
    path: "/api/audio/route29.mid",
    fileName: "route29.mid",
  },
  {
    id: "goldenrod",
    title: "Goldenrod City",
    subtitle: "Busier city track",
    path: "/api/audio/goldenrodcity.mid",
    fileName: "goldenrodcity.mid",
  },
];

export const AudioConvertClient = () => {
  const [file, setFile] = useState<File | null>(null);
  const [midiSequence, setMidiSequence] = useState<MidiSequence | null>(null);
  const [selectedSampleId, setSelectedSampleId] = useState<string | null>(null);
  const [loadingSampleId, setLoadingSampleId] = useState<string | null>(null);
  const [volume, setVolume] = useState("0.4");
  const [sampleRate, setSampleRate] = useState("44100");
  const [loading, setLoading] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const [isPreviewPlaying, setIsPreviewPlaying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<ConvertResult | null>(null);
  const previewAudioRef = useRef<HTMLAudioElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const parsedVolume = useMemo(() => Number(volume), [volume]);
  const parsedSampleRate = useMemo(() => Number(sampleRate), [sampleRate]);
  const isVolumeValid = Number.isFinite(parsedVolume) && parsedVolume > 0 && parsedVolume <= 1;
  const isSampleRateValid = Number.isInteger(parsedSampleRate) && parsedSampleRate >= 8_000 && parsedSampleRate <= 96_000;
  const canSubmit = useMemo(
    () => file != null && !loading && isVolumeValid && isSampleRateValid,
    [file, loading, isVolumeValid, isSampleRateValid],
  );
  const initFailed = error?.startsWith("FFMPEG_INIT_FAILED:") ?? false;
  const sampleLoading = loadingSampleId != null;

  const focusRingClass = "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-accent)] focus-visible:ring-offset-2 focus-visible:ring-offset-base-200";
  const openFilePicker = () => {
    fileInputRef.current?.click();
  };

  const handleDropzoneKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === " " || event.key === "Enter") {
      event.preventDefault();
      openFilePicker();
    }
  };

  const parseMidiBytes = async (bytes: ArrayBuffer): Promise<MidiSequence> => {
    const midiModule = await import("@pokecrystal/core/audio-export/midi-instrument");
    return midiModule.midiFileToSequence(bytes);
  };

  const parseMidiFile = async (picked: File): Promise<MidiSequence> => parseMidiBytes(await picked.arrayBuffer());

  const setPickedFile = async (picked: File | null) => {
    if (!picked) {
      return;
    }
    if (!/\.midi?$/i.test(picked.name)) {
      setError("INVALID_INPUT: Please choose a .mid or .midi file");
      setMidiSequence(null);
      return;
    }
    try {
      assertMidiFileByteLength(picked.size);
    } catch (sizeError) {
      setError(`INVALID_INPUT: ${sizeError instanceof Error ? sizeError.message : String(sizeError)}`);
      setMidiSequence(null);
      return;
    }
    try {
      const parsedSequence = await parseMidiFile(picked);
      setMidiSequence(parsedSequence);
      setError(null);
      setSelectedSampleId(null);
    } catch (parseError) {
      setMidiSequence(null);
      setError(`MIDI_PARSE_FAILED: ${parseError instanceof Error ? parseError.message : String(parseError)}`);
      return;
    }
    setFile(picked);
  };

  const onLoadSample = async (sample: MidiSampleOption) => {
    setLoadingSampleId(sample.id);
    setError(null);
    try {
      const response = await fetch(sample.path);
      if (!response.ok) {
        throw new Error(`sample request failed with ${response.status}`);
      }
      const bytes = await response.arrayBuffer();
      assertMidiFileByteLength(bytes.byteLength);
      const sampleFile = new File([bytes], sample.fileName, { type: "audio/midi" });
      const parsedSequence = await parseMidiBytes(bytes);
      setFile(sampleFile);
      setMidiSequence(parsedSequence);
      setSelectedSampleId(sample.id);
      setError(null);
    } catch (sampleError) {
      setMidiSequence(null);
      setFile(null);
      setSelectedSampleId(null);
      setError(`SAMPLE_LOAD_FAILED: ${sampleError instanceof Error ? sampleError.message : String(sampleError)}`);
    } finally {
      setLoadingSampleId(null);
    }
  };

  const onConvert = async () => {
    if (!file) {
      return;
    }
    setLoading(true);
    setError(null);
    setIsPreviewPlaying(false);
    setResult((prev) => {
      if (prev?.previewUrl) {
        URL.revokeObjectURL(prev.previewUrl);
      }
      return null;
    });

    try {
      const converted = await convertMidiToMp3Client(file, {
        masterVolume: parsedVolume,
        sampleRate: parsedSampleRate,
      });
      setResult(converted);
    } catch (err) {
      if (err instanceof AudioConvertError) {
        setError(`${err.code}: ${err.message}`);
      } else {
        setError(err instanceof Error ? err.message : String(err));
      }
    } finally {
      setLoading(false);
    }
  };

  const onRetryFfmpeg = () => {
    resetFfmpegClient();
    setError(null);
  };

  return (
    <section className="card w-full border border-base-300 bg-base-100">
      <div className="card-body gap-4">
        <h2 className="text-lg font-semibold tracking-wide">MIDI to MP3</h2>

        <section className="card card-bordered bg-base-100">
          <div className="card-body gap-3">
            <p className="text-xs font-semibold uppercase tracking-[0.08em] text-base-content/70">Input</p>
            <div
              role="button"
              tabIndex={0}
              className={toClassName(
                focusRingClass,
                "rounded-box border border-dashed p-6 transition-colors",
                dragActive ? "border-accent bg-accent/15" : "border-base-content/20 bg-base-200/50",
              )}
              aria-label="Dropzone for MIDI upload"
              onDragEnter={(event: DragEvent<HTMLDivElement>) => {
                event.preventDefault();
                setDragActive(true);
              }}
              onDragOver={(event: DragEvent<HTMLDivElement>) => {
                event.preventDefault();
                setDragActive(true);
              }}
              onDragLeave={(event: DragEvent<HTMLDivElement>) => {
                event.preventDefault();
                setDragActive(false);
              }}
              onDrop={(event: DragEvent<HTMLDivElement>) => {
                event.preventDefault();
                setDragActive(false);
                void setPickedFile(event.dataTransfer.files?.[0] ?? null);
              }}
              onClick={openFilePicker}
              onKeyDown={handleDropzoneKeyDown}
            >
              <div className="space-y-2">
                <p className="font-semibold">Drop your MIDI file here</p>
                <p className="text-sm text-base-content/70">
                  Supports `.mid` and `.midi` files up to {Math.ceil(MIDI_MAX_FILE_BYTES / 1024)} KB. All rendering and MP3 encoding stay in your browser.
                </p>
                <button
                  type="button"
                  className="btn btn-outline btn-sm"
                  onClick={(event) => {
                    event.stopPropagation();
                    openFilePicker();
                  }}
                >
                  {file ? "Choose a different file" : "Browse files"}
                  <input
                    ref={fileInputRef}
                    hidden
                    type="file"
                    accept=".mid,.midi,audio/midi"
                    onChange={(event: ChangeEvent<HTMLInputElement>) => void setPickedFile(event.target.files?.[0] ?? null)}
                    aria-label="Upload MIDI file"
                  />
                </button>
              </div>
            </div>

            <div className="space-y-2">
              <p className="text-sm text-base-content/70">Or load a built-in sample</p>
              <div className="overflow-x-auto rounded-box border border-base-300">
                <table className="table table-sm">
                  <thead>
                    <tr>
                      <th>Sample</th>
                      <th>Description</th>
                      <th className="text-right">Action</th>
                    </tr>
                  </thead>
                  <tbody>
                    {SAMPLE_MIDI_OPTIONS.map((sample) => {
                      const isActive = selectedSampleId === sample.id && file?.name === sample.fileName;
                      const isLoading = sampleLoading && loadingSampleId === sample.id;
                      return (
                        <tr key={sample.id}>
                          <td>{sample.title}</td>
                          <td>
                            <span className="text-sm text-base-content/70">{sample.subtitle}</span>
                          </td>
                          <td className="text-right">
                            <button
                              type="button"
                              className={toClassName(
                                "btn btn-sm",
                                isActive ? "btn-primary" : "btn-outline",
                              )}
                              disabled={sampleLoading}
                              onClick={() => void onLoadSample(sample)}
                              aria-label={`Load ${sample.title}`}
                            >
                              {isLoading ? <span className="loading loading-spinner loading-xs" /> : null}
                              {isLoading ? "Loading" : isActive ? "Loaded" : "Load"}
                            </button>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </section>

        {file ? (
          <div className="flex flex-wrap gap-2">
            <span className={chipClassName}>{file.name}</span>
            <span className={chipClassName}>{`${Math.max(1, Math.round(file.size / 1024))} KB`}</span>
          </div>
        ) : null}

        {midiSequence ? (
          <section className="card card-bordered bg-base-200">
            <div className="card-body gap-2">
              <p className="text-xs font-semibold uppercase tracking-[0.08em] text-base-content/70">Track Visualization</p>
              <MidiTimeline sequence={midiSequence} />
            </div>
          </section>
        ) : null}

        <section className="card card-bordered bg-base-100">
          <div className="card-body gap-3">
            <p className="text-xs font-semibold uppercase tracking-[0.08em] text-base-content/70">Render Options</p>
            <div className="grid gap-3 md:grid-cols-2">
              <label className="form-control w-full gap-1">
                <span className="label-text text-xs font-medium uppercase tracking-[0.08em]">Master Volume</span>
                <input
                  type="text"
                  className={toClassName("input input-bordered input-sm w-full", !isVolumeValid ? "input-error" : "")}
                value={volume}
                onChange={(event: ChangeEvent<HTMLInputElement>) => setVolume(event.target.value)}
                />
                <span className={toClassName("text-xs", !isVolumeValid ? "text-error" : "text-base-content/70")}>
                  {!isVolumeValid ? "Use a value between 0.01 and 1.00" : "0.4 matches current default"}
                </span>
              </label>
              <label className="form-control w-full gap-1">
                <span className="label-text text-xs font-medium uppercase tracking-[0.08em]">Sample Rate</span>
                <input
                  type="text"
                  className={toClassName("input input-bordered input-sm w-full", !isSampleRateValid ? "input-error" : "")}
                value={sampleRate}
                onChange={(event: ChangeEvent<HTMLInputElement>) => setSampleRate(event.target.value)}
                />
                <span className={toClassName("text-xs", !isSampleRateValid ? "text-error" : "text-base-content/70")}>
                  {!isSampleRateValid ? "Use an integer between 8000 and 96000" : "44100 recommended"}
                </span>
              </label>
            </div>
            <div className="flex flex-wrap gap-2">
              <button type="button" className="btn btn-sm btn-primary" onClick={onConvert} disabled={!canSubmit}>
                Convert to MP3
              </button>
              {initFailed ? (
                <button type="button" className="btn btn-sm btn-outline btn-warning" onClick={onRetryFfmpeg}>
                  Retry ffmpeg init
                </button>
              ) : null}
            </div>
          </div>
        </section>

        {loading ? <progress className="progress progress-primary w-full" /> : null}
        {error ? (
          <div role="alert" className="alert alert-error">
            <span>{error}</span>
            {initFailed ? (
              <span className="text-xs text-error-content/90">
                If this persists, hard refresh to reload `/ffmpeg` assets.
              </span>
            ) : null}
          </div>
        ) : null}

        {result ? (
          <section className="card card-bordered bg-gradient-to-b from-base-200 to-base-300">
            <div className="card-body gap-2">
              <p className="text-sm text-base-content/70">
                Duration: {result.metadata.durationSeconds.toFixed(2)}s | Notes: {result.diagnostics.noteCount}
              </p>
              <audio
                ref={previewAudioRef}
                controls
                src={result.previewUrl}
                onPlay={() => setIsPreviewPlaying(true)}
                onPause={() => setIsPreviewPlaying(false)}
                onEnded={() => setIsPreviewPlaying(false)}
              />
              <section className="card card-bordered bg-base-100">
                <div className="card-body gap-2">
                  <p className="text-xs font-semibold uppercase tracking-[0.08em] text-base-content/70">Live Preview Visualizer</p>
                  <PreviewSpectrum audioRef={previewAudioRef} active={isPreviewPlaying} />
                </div>
              </section>
              <a
                href={result.previewUrl}
                download={(file?.name ?? "converted").replace(/\.midi?$/i, "") + ".mp3"}
                className="btn btn-sm btn-outline w-fit"
              >
                Download MP3
              </a>
            </div>
          </section>
        ) : null}
      </div>
    </section>
  );
};

const MidiTimeline = ({ sequence }: { sequence: MidiSequence }) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const notes = sequence.notes;
  const duration = Math.max(0.001, sequence.durationSeconds);
  const pitchMin = notes.length > 0 ? Math.min(...notes.map((note) => note.note)) : 0;
  const pitchMax = notes.length > 0 ? Math.max(...notes.map((note) => note.note)) : 127;
  const pitchSpan = Math.max(1, pitchMax - pitchMin);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) {
      return;
    }
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      return;
    }

    const width = canvas.width;
    const height = canvas.height;
    ctx.clearRect(0, 0, width, height);

    ctx.fillStyle = "rgba(15, 23, 42, 0.55)";
    ctx.fillRect(0, 0, width, height);

    ctx.strokeStyle = "rgba(148, 163, 184, 0.25)";
    ctx.lineWidth = 1;
    for (let i = 0; i <= 8; i += 1) {
      const x = Math.round((i / 8) * width);
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      ctx.stroke();
    }
    for (let i = 0; i <= 6; i += 1) {
      const y = Math.round((i / 6) * height);
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }

    drawNoteRects(ctx, notes, duration, width, height, pitchMin, pitchSpan);

    if (sequence.loopPoints.startSeconds != null && sequence.loopPoints.endSeconds != null) {
      const xStart = Math.round((sequence.loopPoints.startSeconds / duration) * width);
      const xEnd = Math.round((sequence.loopPoints.endSeconds / duration) * width);
      ctx.strokeStyle = "rgba(251, 191, 36, 0.95)";
      if ("setLineDash" in ctx && typeof ctx.setLineDash === "function") {
        ctx.setLineDash([4, 3]);
      }
      ctx.beginPath();
      ctx.moveTo(xStart, 0);
      ctx.lineTo(xStart, height);
      ctx.moveTo(xEnd, 0);
      ctx.lineTo(xEnd, height);
      ctx.stroke();
      if ("setLineDash" in ctx && typeof ctx.setLineDash === "function") {
        ctx.setLineDash([]);
      }
    }
  }, [duration, notes, pitchMin, pitchSpan, sequence.loopPoints.endSeconds, sequence.loopPoints.startSeconds]);

  return (
    <div className="space-y-2">
      <canvas
        ref={canvasRef}
        width={960}
        height={220}
        className="block w-full h-40 rounded-md border border-base-300 bg-base-200"
      />
      <div className="flex flex-wrap gap-2">
        <span className={chipClassName}>{`${notes.length} notes`}</span>
        <span className={chipClassName}>{`${duration.toFixed(2)}s`}</span>
        <span className={chipClassName}>{`Pitch ${pitchMin} - ${pitchMax}`}</span>
      </div>
    </div>
  );
};

const drawNoteRects = (
  ctx: CanvasRenderingContext2D,
  notes: ScheduledNote[],
  duration: number,
  width: number,
  height: number,
  pitchMin: number,
  pitchSpan: number,
): void => {
  for (const note of notes) {
    const start = Math.max(0, note.startSeconds);
    const end = Math.min(duration, note.startSeconds + note.durationSeconds);
    const x = Math.round((start / duration) * width);
    const w = Math.max(1, Math.round(((end - start) / duration) * width));
    const relativePitch = (note.note - pitchMin) / pitchSpan;
    const y = Math.round((1 - relativePitch) * (height - 8));

    const base = note.voice === "wave" ? "96, 165, 250" : "56, 189, 248";
    const alpha = 0.35 + ((note.velocity ?? 100) / 127) * 0.45;
    ctx.fillStyle = `rgba(${base}, ${Math.min(0.9, alpha)})`;
    ctx.fillRect(x, y, w, 4);
  }
};

type SpectrumFrame = Readonly<{
  bars: number[];
  nextPeak: number;
}>;

const LOG_BIN_START = 1;
const MIN_PEAK_LEVEL = 0.08;
const PEAK_DECAY = 0.94;

export const computeSpectrumFrame = (freq: Uint8Array, barCount: number, peakLevel: number): SpectrumFrame => {
  if (barCount <= 0 || freq.length === 0) {
    return { bars: [], nextPeak: MIN_PEAK_LEVEL };
  }

  const maxBin = Math.max(LOG_BIN_START + 1, freq.length - 1);
  const barValues: number[] = [];
  let framePeak = 0;

  for (let i = 0; i < barCount; i += 1) {
    const start = Math.floor(LOG_BIN_START * Math.pow(maxBin / LOG_BIN_START, i / barCount));
    const rawEnd = Math.floor(LOG_BIN_START * Math.pow(maxBin / LOG_BIN_START, (i + 1) / barCount));
    const end = Math.max(start + 1, rawEnd);

    let sumSquares = 0;
    let bins = 0;
    for (let bin = start; bin < end && bin < freq.length; bin += 1) {
      const norm = freq[bin] / 255;
      sumSquares += norm * norm;
      bins += 1;
    }

    const rms = bins > 0 ? Math.sqrt(sumSquares / bins) : 0;
    framePeak = Math.max(framePeak, rms);
    barValues.push(rms);
  }

  const nextPeak = Math.max(MIN_PEAK_LEVEL, framePeak, peakLevel * PEAK_DECAY);
  const bars = barValues.map((value) => {
    const normalized = Math.min(1, value / nextPeak);
    return Math.pow(normalized, 0.75);
  });

  return { bars, nextPeak };
};

const PreviewSpectrum = ({
  audioRef,
  active,
}: {
  audioRef: RefObject<HTMLAudioElement | null>;
  active: boolean;
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const sourceRef = useRef<MediaElementAudioSourceNode | null>(null);
  const contextRef = useRef<AudioContext | null>(null);
  const rafRef = useRef<number | null>(null);
  const peakRef = useRef(MIN_PEAK_LEVEL);

  useEffect(() => {
    const audio = audioRef.current;
    const canvas = canvasRef.current;
    if (!audio || !canvas) {
      return;
    }
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      return;
    }

    const setupGraph = async () => {
      if (!contextRef.current) {
        const Ctor = window.AudioContext || (window as Window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
        if (!Ctor) {
          return;
        }
        contextRef.current = new Ctor();
      }
      const context = contextRef.current;
      if (!context) {
        return;
      }
      if (context.state === "suspended" && active) {
        await context.resume();
      }
      if (!analyserRef.current) {
        analyserRef.current = context.createAnalyser();
        analyserRef.current.fftSize = 256;
        analyserRef.current.smoothingTimeConstant = 0.8;
      }
      if (!sourceRef.current) {
        sourceRef.current = context.createMediaElementSource(audio);
        sourceRef.current.connect(analyserRef.current);
        analyserRef.current.connect(context.destination);
      }
    };

    const paintIdle = () => {
      const { width, height } = canvas;
      ctx.fillStyle = "rgba(15, 23, 42, 0.52)";
      ctx.fillRect(0, 0, width, height);
      ctx.strokeStyle = "rgba(56, 189, 248, 0.35)";
      ctx.beginPath();
      ctx.moveTo(0, height / 2);
      for (let x = 0; x < width; x += 1) {
        const wave = Math.sin((x / width) * Math.PI * 12) * 5;
        ctx.lineTo(x, height / 2 + wave);
      }
      ctx.stroke();
    };

    const paint = () => {
      if (!analyserRef.current) {
        paintIdle();
        return;
      }
      const analyser = analyserRef.current;
      const freq = new Uint8Array(analyser.frequencyBinCount);
      const wave = new Uint8Array(analyser.frequencyBinCount);
      analyser.getByteFrequencyData(freq);
      analyser.getByteTimeDomainData(wave);

      const { width, height } = canvas;
      ctx.fillStyle = "rgba(15, 23, 42, 0.52)";
      ctx.fillRect(0, 0, width, height);

      const bars = 48;
      const barWidth = width / bars;
      const spectrum = computeSpectrumFrame(freq, bars, peakRef.current);
      peakRef.current = spectrum.nextPeak;
      for (let i = 0; i < bars; i += 1) {
        const magnitude = spectrum.bars[i] ?? 0;
        const barHeight = Math.max(2, magnitude * (height - 8));
        const x = i * barWidth;
        const y = height - barHeight - 2;
        const hue = 190 - i * 1.8;
        ctx.fillStyle = `hsla(${hue}, 90%, 60%, 0.82)`;
        ctx.fillRect(x + 1, y, Math.max(2, barWidth - 2), barHeight);
      }

      ctx.strokeStyle = "rgba(251, 191, 36, 0.9)";
      ctx.lineWidth = 1.35;
      ctx.beginPath();
      for (let x = 0; x < width; x += 1) {
        const idx = Math.min(wave.length - 1, Math.floor((x / width) * wave.length));
        const normalized = (wave[idx] - 128) / 128;
        const y = (height * 0.3) + normalized * 18;
        if (x === 0) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
      }
      ctx.stroke();
    };

    const frame = () => {
      paint();
      rafRef.current = window.requestAnimationFrame(frame);
    };

    void setupGraph().then(() => {
      if (rafRef.current == null) {
        frame();
      }
    });

    return () => {
      if (rafRef.current != null) {
        window.cancelAnimationFrame(rafRef.current);
      }
      rafRef.current = null;
    };
  }, [active, audioRef]);

  return (
    <canvas
      ref={canvasRef}
      width={960}
      height={220}
      className="block w-full h-40 rounded-md border border-base-300 bg-base-200"
    />
  );
};

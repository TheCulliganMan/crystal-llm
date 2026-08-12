import {
  CHANNEL3_DEFAULT_INSTRUMENT,
  CHANNEL_GAINS,
  DMG_HPF_DECAY,
  DEFAULT_TEMPO,
  DEFAULT_LOOPED_MUSIC_EXPORT_SECONDS,
  ENHANCED_MASTER_GAIN,
  FRAME_SEQUENCER_RATE,
  FRAME_TO_SAMPLE_DENOMINATOR,
  FRAME_TO_SAMPLE_NUMERATOR,
  FREQUENCY_TABLE,
  GB_DUTY_PATTERNS,
  MAX_COMMANDS_PER_CHANNEL,
  MAX_WAVE_AMPLITUDE,
  MP3_EXPORT_BITRATE,
  MP3_EXPORT_SAMPLE_RATE,
  NOTE_INDEX,
  PAN_CROSSFEED,
  SAMPLE_RATE,
  SOUND_LOOP_INFINITE_REPEAT_LIMIT,
  WAVE_NOTE_FREQUENCY_SCALAR,
} from "./constants";
import { MidiRecorder } from "./midi-recorder";
import type { ParsedMusicData, AudioCommand } from "./parsers";
import { createChannelState, type ChannelState, type NoiseNote } from "./schemas";
import { computeIntegral, noiseKernelWrapper, pulseKernel, waveKernel } from "./synthesis";
import {
  freqToRegWave,
  next128hzTickStrict,
  nextFrameTickWithStepStrict,
  parseNumber,
  regToFreqWave,
  sampleOffsetToFrameTick,
  ticksToFrames,
} from "./utils";

export type ConvertFormat = "pcm" | "midi";

interface MidiEvent {
  startSample: number;
  durationSamples: number;
  note: number;
  velocity: number;
  channel: number;
}

export interface ConverterResult {
  stereo: Int16Array;
  sampleRate: number;
  midiBytes?: Uint8Array;
  metadata: {
    durationSeconds: number;
    bitrate?: string;
    exportSampleRate?: number;
    loopFramesByChannel?: Record<number, number>;
    loopSamplesByChannel?: Record<number, number>;
  };
}

interface WavConverterOptions {
  waveInstrumentMap?: Record<number, number>;
  qualityMode?: "accurate" | "enhanced";
  infiniteLoopRepeatLimit?: number;
  loopedMusicExportSeconds?: number | null;
  soloChannel?: number | null;
  /** Exact wCryPitch value installed by _PlayCry. */
  cryPitch?: number | null;
  /** Exact wCryLength value installed as tempo on every non-noise cry channel. */
  cryLength?: number | null;
}

export class WavConverter {
  private readonly musicData: ParsedMusicData;
  private readonly drumkits: Record<number, Record<number, NoiseNote[]>>;
  private readonly waveSamples: Record<number, number[]>;
  private readonly waveInstrumentMap: Record<number, number>;

  private channelOutputs = new Map<number, Array<{ audio: Int16Array; pan: [boolean, boolean] }>>();
  private channelLoopPoints = new Map<number, [number, number]>();
  private channelFrameTotals = new Map<number, number>();
  private channelSampleTotals = new Map<number, number>();
  private midiEvents = new Map<number, MidiEvent[]>();
  private nr50Events: Array<{ offset: number; left: number; right: number }> = [{ offset: 0, left: 7, right: 7 }];
  private sharedSampleRemainder = 0;
  private readonly qualityMode: "accurate" | "enhanced";
  private readonly infiniteLoopRepeatLimit: number;
  private readonly loopedMusicExportSeconds: number | null;
  private readonly soloChannel: number | null;
  private readonly cryPitch: number | null;
  private readonly cryLength: number | null;
  private sharedTempoEvents: Array<{ frame: number; tempo: number; remainder: number }> = [];
  private sharedTrackTempo: number | null = null;
  private tempoSourceChannel: number | null = null;
  private primaryChannelNumber: number | null = null;
  private perfectPitchEnabled = false;
  private nextWaveSampleIndex: number;
  private nextWaveInstrumentId: number;

  constructor(
    musicData: ParsedMusicData,
    drumkits: Record<number, Record<number, NoiseNote[]>>,
    waveSamples: Record<number, number[]>,
    options?: WavConverterOptions,
  ) {
    this.musicData = musicData;
    this.drumkits = drumkits;
    this.waveSamples = { ...waveSamples };
    if (Object.keys(this.waveSamples).length > 0) {
      for (let idx = 0; idx < 16; idx += 1) {
        this.waveSamples[idx] = this.waveSamples[idx] ?? new Array(32).fill(0);
      }
    }
    this.waveInstrumentMap = options?.waveInstrumentMap ?? Object.fromEntries(Object.keys(waveSamples).map((k) => [Number(k), Number(k)]));
    this.qualityMode = options?.qualityMode ?? "enhanced";
    this.infiniteLoopRepeatLimit = Math.max(1, options?.infiniteLoopRepeatLimit ?? SOUND_LOOP_INFINITE_REPEAT_LIMIT);
    this.loopedMusicExportSeconds =
      options && Object.prototype.hasOwnProperty.call(options, "loopedMusicExportSeconds")
        ? options.loopedMusicExportSeconds ?? null
        : DEFAULT_LOOPED_MUSIC_EXPORT_SECONDS;
    this.soloChannel = options?.soloChannel ?? null;
    this.cryPitch = options?.cryPitch == null ? null : options.cryPitch & 0xffff;
    this.cryLength = options?.cryLength == null ? null : options.cryLength & 0xffff;
    this.nextWaveSampleIndex = Math.max(0x10, maxObjectKey(this.waveSamples, -1) + 1);
    this.nextWaveInstrumentId = Math.max(0x10, maxObjectKey(this.waveInstrumentMap, -1) + 1);
  }

  convert(format: ConvertFormat = "pcm"): ConverterResult {
    this.channelOutputs.clear();
    this.channelLoopPoints.clear();
    this.channelFrameTotals.clear();
    this.channelSampleTotals.clear();
    this.midiEvents.clear();
    this.nr50Events = [{ offset: 0, left: 7, right: 7 }];
    this.sharedSampleRemainder = 0;
    this.sharedTempoEvents = [];
    this.sharedTrackTempo = null;
    this.tempoSourceChannel = null;
    this.primaryChannelNumber = null;
    this.perfectPitchEnabled = false;

    const channels = Object.entries(this.musicData.channels).sort((a, b) => (a[1].number ?? 0) - (b[1].number ?? 0));
    if (channels.length > 0) {
      this.primaryChannelNumber = channels[0][1].number ?? null;
    }
    this.primeSharedTempoEvents(channels);
    for (const [label, info] of channels) {
      const channelNumber = info.number ?? 0;
      const segments = this.processChannel(label, channelNumber);
      this.channelOutputs.set(
        channelNumber,
        this.soloChannel == null || channelNumber === this.soloChannel ? segments : [],
      );
    }

    this.ensureAllChannelsSynced(format === "midi" ? "midi" : "mp3");

    const stereo = this.mixStereo();
    const loopFramesByChannel = Object.fromEntries(
      Array.from(this.channelLoopPoints.entries()).map(([channel, [frame]]) => [channel, frame]),
    );
    const loopSamplesByChannel = Object.fromEntries(
      Array.from(this.channelLoopPoints.entries()).map(([channel, [, sample]]) => [channel, sample]),
    );

    if (format === "midi") {
      const recorder = new MidiRecorder({ sampleRate: SAMPLE_RATE });
      for (const [sourceChannel, entries] of this.midiEvents.entries()) {
        for (const entry of entries) {
          recorder.recordNote({
            channel: Math.max(0, Math.min(15, sourceChannel - 1)),
            note: entry.note,
            velocity: entry.velocity,
            startSample: entry.startSample,
            durationSamples: entry.durationSamples,
          });
        }
      }
      const midiBytes = recorder.toBytes();
      return {
        stereo,
        sampleRate: SAMPLE_RATE,
        midiBytes,
        metadata: {
          durationSeconds: stereo.length / 2 / SAMPLE_RATE,
          bitrate: MP3_EXPORT_BITRATE,
          exportSampleRate: MP3_EXPORT_SAMPLE_RATE,
          loopFramesByChannel,
          loopSamplesByChannel,
        },
      };
    }

    return {
      stereo,
      sampleRate: SAMPLE_RATE,
      metadata: {
        durationSeconds: stereo.length / 2 / SAMPLE_RATE,
        bitrate: MP3_EXPORT_BITRATE,
        exportSampleRate: MP3_EXPORT_SAMPLE_RATE,
        loopFramesByChannel,
        loopSamplesByChannel,
      },
    };
  }

  private processChannel(channelLabel: string, channelNumber: number): Array<{ audio: Int16Array; pan: [boolean, boolean] }> {
    const state = createChannelState();
    const initialTempoEvent = this.initialSharedTempoEvent();
    if (initialTempoEvent != null) {
      state.tempo = initialTempoEvent.tempo;
      state.duration_modifier = initialTempoEvent.remainder;
    }
    const segments: Array<{ audio: Int16Array; pan: [boolean, boolean] }> = [];
    const isPulseChannel = channelNumber === 1 || channelNumber === 2 || channelNumber === 5 || channelNumber === 6;
    const isWaveChannel = channelNumber === 3 || channelNumber === 7;
    const isNoiseChannel = channelNumber === 4 || channelNumber === 8;

    if (this.cryPitch != null) {
      state.pitch_offset = this.cryPitch;
    }
    if (this.cryLength != null && !isNoiseChannel) {
      state.tempo = this.cryLength;
      state.duration_modifier = 0;
    }

    if (this.perfectPitchEnabled) {
      state.instrument_pitch_offset = 1;
    }
    if (isWaveChannel && channelNumber === 3 && state.wave_instrument == null) {
      state.wave_instrument = CHANNEL3_DEFAULT_INSTRUMENT;
    }

    let frameTotal = 0;
    let sampleTotal = 0;
    let currentDrumkit: number | null = null;
    let noiseSamplingEnabled = state.noise_sampling_enabled;
    let pulsePhase = 0;

    for (const cmd of this.commandStream(channelLabel, channelNumber)) {
      const op = cmd.command;
      const args = cmd.args;

      if (op === "__loop_point__") {
        this.channelLoopPoints.set(channelNumber, [frameTotal, sampleTotal]);
        continue;
      }

      if (op === "volume") {
        if (channelNumber === this.primaryChannelNumber) {
          const left = args.length > 0 ? parseNumber(args[0]) : 7;
          const right = args.length > 1 ? parseNumber(args[1]) : 7;
          this.recordNr50Event(sampleTotal, left, right);
        }
        continue;
      }

      if (op === "tempo") {
        state.tempo = parseNumber(args[0]);
        if (this.tempoSourceChannel == null) {
          this.tempoSourceChannel = channelNumber;
        }
        if (channelNumber === this.tempoSourceChannel) {
          this.sharedTrackTempo = state.tempo;
          this.recordSharedTempoEvent(frameTotal, state.tempo, state.duration_modifier);
        }
        continue;
      }

      if (op === "speed") {
        state.note_length = Math.max(1, parseNumber(args[0]));
        state.default_length = state.note_length;
        continue;
      }

      if (op === "duty_cycle") {
        state.duty_cycle = parseNumber(args[0]) & 0x3;
        continue;
      }

      if (op === "duty_cycle_pattern") {
        const pattern = this.decodeDutyCyclePatternByte(args);
        state.duty_cycle_pattern_byte = pattern;
        state.duty_cycle_loop_enabled = true;
        state.duty_cycle = (pattern >> 6) & 0x3;
        continue;
      }

      if (op === "pitch_offset") {
        state.pitch_offset = parseNumber(args[0]);
        continue;
      }

      if (op === "pitch_slide" && channelNumber === 1) {
        const durationTicks = parseNumber(args[0]);
        const targetOctave = parseNumber(args[1]);
        const targetNote = args[2];
        const tmpState: ChannelState = {
          ...state,
          octave: Math.max(0, Math.min(7, 8 - targetOctave)),
        };
        const targetReg = this.noteToRegisterPulse(targetNote, tmpState);
        const [slideFrames] = this.computeNoteFrames(
          durationTicks,
          state.note_length,
          this.resolveTempoForFrame(channelNumber, frameTotal, state.tempo),
          state.duration_modifier,
        );
        state.pitch_slide_target = targetReg;
        state.pitch_slide_frames = Math.max(1, slideFrames);
        continue;
      }

      if (op === "vibrato") {
        const delay = args.length > 0 ? parseNumber(args[0]) : 0;
        const [extent, rate] = this.parseVibratoExtentAndRate(args.slice(1));
        state.vibrato_delay = Math.max(0, delay);
        state.vibrato_delay_count = state.vibrato_delay;
        state.vibrato_extent = extent;
        state.vibrato_rate = Math.max(0, rate);
        state.vibrato_rate_counter = state.vibrato_rate;
        state.vibrato_extent_up = Math.floor((extent + 1) / 2);
        state.vibrato_extent_down = Math.floor(extent / 2);
        state.vibrato_direction_up = false;
        continue;
      }

      if (op === "pitch_sweep" || op === "sweep") {
        if (channelNumber === 1) {
          const value = this.parsePitchSweepArgs(args);
          state.pitch_sweep_value = value;
          state.pitch_sweep_enabled = this.isPitchSweepActive(value);
        }
        continue;
      }

      if (op === "toggle_perfect_pitch") {
        this.perfectPitchEnabled = !this.perfectPitchEnabled;
        state.instrument_pitch_offset = this.perfectPitchEnabled ? 1 : 0;
        continue;
      }

      if (op === "octave") {
        const raw = parseNumber(args[0]);
        state.octave = Math.max(0, Math.min(7, 8 - raw));
        continue;
      }

      if (op === "inc_octave") {
        state.octave -= 1;
        if (state.octave < 0) {
          state.octave = 7;
        }
        continue;
      }

      if (op === "dec_octave") {
        state.octave += 1;
        if (state.octave > 7) {
          state.octave = 0;
        }
        continue;
      }

      if (op === "transpose") {
        state.transpose_octaves = parseNumber(args[0]);
        state.transpose_pitches = parseNumber(args[1]);
        continue;
      }

      if (op === "stereo_panning" || op === "force_stereo_panning") {
        if (args.length >= 2 && isBooleanToken(args[0]) && isBooleanToken(args[1])) {
          state.current_pan = [parseBooleanToken(args[0]), parseBooleanToken(args[1])];
        } else {
          const v = parseNumber(args[0]);
          state.current_pan = [Boolean(v & 0x10), Boolean(v & 0x1)];
        }
        continue;
      }

      if (op === "note_type" || op === "drum_speed") {
        state.note_length = Math.max(1, parseNumber(args[0]));
        state.default_length = state.note_length;
        if (op === "note_type" && args.length >= 2) {
          if (isWaveChannel) {
            state.wave_volume = parseNumber(args[1]);
            if (args.length >= 3) {
              state.wave_instrument = parseNumber(args[2]);
            }
          } else {
            const initial = parseNumber(args[1]);
            const fade = args.length >= 3 ? parseNumber(args[2]) : 0;
            state.volume_envelope = [initial, this.normalizeEnvelopeFade(fade)];
          }
        }
        continue;
      }

      if (op === "volume_envelope") {
        if (isWaveChannel) {
          state.wave_volume = parseNumber(args[0]);
          if (args.length >= 2) {
            state.wave_instrument = parseNumber(args[1]);
          }
        } else {
          const initial = parseNumber(args[0]);
          const fade = args.length >= 2 ? parseNumber(args[1]) : 0;
          state.volume_envelope = [initial, this.normalizeEnvelopeFade(fade)];
        }
        continue;
      }

      if (op === "channel_volume") {
        if (isWaveChannel) {
          state.wave_volume = parseNumber(args[0]);
        } else {
          const vol = parseNumber(args[0]);
          state.volume_envelope = [vol, 0];
        }
        continue;
      }

      if (op === "fade_wave") {
        if (isWaveChannel) {
          if (args.length >= 1) {
            state.wave_instrument = parseNumber(args[0]);
          }
        } else {
          const fade = args.length >= 1 ? parseNumber(args[0]) : 0;
          const initial = state.volume_envelope?.[0] ?? 15;
          state.volume_envelope = [initial, this.normalizeEnvelopeFade(fade)];
        }
        continue;
      }

      if (op === "load_wave") {
        const pattern = this.parseInlineWave(args);
        state.wave_instrument = this.registerInlineWave(pattern);
        continue;
      }

      if ((op === "toggle_noise" || op === "sfx_toggle_noise") && isNoiseChannel) {
        if (noiseSamplingEnabled) {
          noiseSamplingEnabled = false;
          state.noise_sampling_enabled = false;
        } else {
          noiseSamplingEnabled = true;
          state.noise_sampling_enabled = true;
          if (args.length > 0) {
            currentDrumkit = parseNumber(args[0]);
          }
        }
        continue;
      }

      if (op === "rest") {
        const ticks = parseNumber(args[0]);
        const [frames, nextMod] = this.computeNoteFrames(
          ticks,
          state.note_length,
          this.resolveTempoForFrame(channelNumber, frameTotal, state.tempo),
          state.duration_modifier,
        );
        state.duration_modifier = nextMod;
        const samples = this.framesToSamplesPrecise(frames);
        frameTotal += frames;
        sampleTotal += samples;
        segments.push({ audio: new Int16Array(samples), pan: state.current_pan });
        if (isPulseChannel && state.duty_cycle_loop_enabled && frames > 0) {
          for (let i = 0; i < frames; i += 1) {
            this.advanceDutyCyclePatternFrame(state);
          }
        }
        continue;
      }

      if (op === "note") {
        const [noteNameRaw, lenRaw] = args;
        const len = lenRaw != null ? parseNumber(lenRaw) : state.default_length ?? 4;
        const [frames, nextMod] = this.computeNoteFrames(
          len,
          state.note_length,
          this.resolveTempoForFrame(channelNumber, frameTotal, state.tempo),
          state.duration_modifier,
        );
        state.duration_modifier = nextMod;
        const samples = this.framesToSamplesPrecise(frames);

        if (isWaveChannel) {
          const baseReg = this.noteToRegisterWave(noteNameRaw, state);
          this.resetVibratoNoteState(state, baseReg);
          const instrument = state.wave_instrument ?? CHANNEL3_DEFAULT_INSTRUMENT;
          const vibratoSegments = this.computeVibratoSegments(frames, state, baseReg);
          const totalVibratoFrames = Math.max(1, sumFrames(vibratoSegments.map(([segmentFrames]) => segmentFrames)));
          let vibratoSamplesUsed = 0;
          const waveChunks: Float32Array[] = [];
          let wavePhase = Math.floor((2 ** 48) / 32);
          for (let idx = 0; idx < vibratoSegments.length; idx += 1) {
            const [segmentFrames, segmentReg] = vibratoSegments[idx];
            const segmentSamples = idx === vibratoSegments.length - 1
              ? samples - vibratoSamplesUsed
              : Math.floor((segmentFrames * samples + Math.floor(totalVibratoFrames / 2)) / totalVibratoFrames);
            if (segmentSamples <= 0) {
              continue;
            }
            vibratoSamplesUsed += segmentSamples;
            const segmentFreq = regToFreqWave(segmentReg);
            const [chunk, nextWavePhase] = this.renderWave(
              segmentSamples,
              segmentFreq,
              instrument,
              state.wave_volume,
              wavePhase,
            );
            wavePhase = nextWavePhase;
            waveChunks.push(chunk);
          }
          let audio = concatFloat32Arrays(waveChunks);
          if (audio.length < samples) {
            audio = concatFloat32Arrays([audio, new Float32Array(samples - audio.length)]);
          } else if (audio.length > samples) {
            audio = audio.slice(0, samples);
          }
          if (state.volume_envelope) {
            audio = this.applyVolumeEnvelope(audio, state.volume_envelope, sampleTotal);
          }
          segments.push({ audio: toInt16ArrayTrunc(audio), pan: state.current_pan });
          this.pushMidiEvent(channelNumber, {
            startSample: sampleTotal,
            durationSamples: samples,
            note: this.frequencyToMidi(regToFreqWave(baseReg)),
            velocity: this.velocityFromWaveVolume(state.wave_volume),
            channel: channelNumber,
          });
        } else {
          const baseReg = this.noteToRegisterPulse(noteNameRaw, state);
          this.resetVibratoNoteState(state, baseReg);
          const slideTarget = state.pitch_slide_target;
          const slideFrames = state.pitch_slide_frames;
          state.pitch_slide_target = null;
          state.pitch_slide_frames = 0;
          if (channelNumber === 1) {
            state.pitch_sweep_shadow = baseReg;
            state.pulse1_active = true;
            this.preflightPitchSweep(state);
          }
          pulsePhase = (6 * 2 ** 48) / 8;
          const sweepSegments = slideTarget != null && channelNumber === 1
            ? this.computePitchSlideSegments(frames, baseReg, slideTarget, slideFrames).map<[number, number | null]>(
                ([segmentFrames, segmentReg]) => [segmentFrames, segmentReg],
              )
            : this.computeSweepSegments(frames, baseReg, state, channelNumber, sampleTotal);
          const totalSweepFrames = Math.max(1, sumFrames(sweepSegments.map(([segmentFrames]) => segmentFrames)));
          let sweepSamplesUsed = 0;
          const pulseChunks: Float32Array[] = [];
          for (let sweepIdx = 0; sweepIdx < sweepSegments.length; sweepIdx += 1) {
            const [sweepFrames, sweepReg] = sweepSegments[sweepIdx];
            const sweepSamples = sweepIdx === sweepSegments.length - 1
              ? samples - sweepSamplesUsed
              : Math.floor((sweepFrames * samples + Math.floor(totalSweepFrames / 2)) / totalSweepFrames);
            if (sweepSamples <= 0) {
              continue;
            }
            sweepSamplesUsed += sweepSamples;
              if (sweepReg == null) {
              pulseChunks.push(new Float32Array(sweepSamples));
              continue;
            }
            const vibratoSegments = this.computeVibratoSegments(sweepFrames, state, sweepReg);
            const totalVibratoFrames = Math.max(1, sumFrames(vibratoSegments.map(([segmentFrames]) => segmentFrames)));
            let vibratoSamplesUsed = 0;
            for (let vibIdx = 0; vibIdx < vibratoSegments.length; vibIdx += 1) {
              const [vibratoFrames, vibratoReg] = vibratoSegments[vibIdx];
              const vibratoSamples = vibIdx === vibratoSegments.length - 1
                ? sweepSamples - vibratoSamplesUsed
                : Math.floor((vibratoFrames * sweepSamples + Math.floor(totalVibratoFrames / 2)) / totalVibratoFrames);
              if (vibratoSamples <= 0) {
                continue;
              }
              vibratoSamplesUsed += vibratoSamples;
              const vibratoFreq = this.registerToFrequency(vibratoReg);
              if (state.duty_cycle_loop_enabled && state.duty_cycle_pattern_byte != null && vibratoFrames > 0) {
                const frameSamples = this.splitSamplesAcrossFrames(vibratoSamples, vibratoFrames);
                for (const frameSamplesCount of frameSamples) {
                  if (frameSamplesCount > 0) {
                    const [chunk, nextPulsePhase] = this.renderPulse(
                      frameSamplesCount,
                      vibratoFreq,
                      state.duty_cycle,
                      pulsePhase,
                    );
                    pulsePhase = nextPulsePhase;
                    pulseChunks.push(chunk);
                  }
                  this.advanceDutyCyclePatternFrame(state);
                }
              } else {
                const [chunk, nextPulsePhase] = this.renderPulse(vibratoSamples, vibratoFreq, state.duty_cycle, pulsePhase);
                pulsePhase = nextPulsePhase;
                pulseChunks.push(chunk);
              }
            }
          }
          let audio = concatFloat32Arrays(pulseChunks);
          if (audio.length < samples) {
            audio = concatFloat32Arrays([audio, new Float32Array(samples - audio.length)]);
          } else if (audio.length > samples) {
            audio = audio.slice(0, samples);
          }
          if (state.volume_envelope) {
            audio = this.applyVolumeEnvelope(audio, state.volume_envelope, sampleTotal);
          }
          segments.push({ audio: toInt16ArrayTrunc(audio), pan: state.current_pan });
          this.pushMidiEvent(channelNumber, {
            startSample: sampleTotal,
            durationSamples: samples,
            note: this.frequencyToMidi(this.registerToFrequency(baseReg)),
            velocity: this.velocityFromEnvelope(state.volume_envelope),
            channel: channelNumber,
          });
        }

        frameTotal += frames;
        sampleTotal += samples;
        continue;
      }

      if (op === "square_note") {
        const rawLength = parseNumber(args[0]);
        const volume = parseNumber(args[1]);
        const fade = parseNumber(args[2]);
        const register = this.applyPitchOffset(parseNumber(args[3]), state);
        const [frames, nextMod] = this.computeNoteFrames(
          this.setNoteDurationTicks(rawLength),
          state.note_length,
          this.resolveTempoForFrame(channelNumber, frameTotal, state.tempo),
          state.duration_modifier,
        );
        state.duration_modifier = nextMod;
        const samples = this.framesToSamplesPrecise(frames);
        const frequency = this.registerToFrequency(register);
        let audio: Float32Array;
        if (state.duty_cycle_loop_enabled && frames > 0) {
          const frameSamples = this.splitSamplesAcrossFrames(samples, frames);
          const chunks: Float32Array[] = [];
          for (const frameSampleCount of frameSamples) {
            if (frameSampleCount > 0) {
              const [chunk, nextPulsePhase] = this.renderPulse(
                frameSampleCount,
                frequency,
                state.duty_cycle,
                pulsePhase,
              );
              pulsePhase = nextPulsePhase;
              chunks.push(chunk);
            }
            this.advanceDutyCyclePatternFrame(state);
          }
          audio = concatFloat32Arrays(chunks);
        } else {
          const [chunk, nextPulsePhase] = this.renderPulse(samples, frequency, state.duty_cycle, pulsePhase);
          pulsePhase = nextPulsePhase;
          audio = chunk;
        }
        audio = this.applyVolumeEnvelope(audio, [volume, fade], sampleTotal);
        segments.push({ audio: toInt16ArrayTrunc(audio), pan: state.current_pan });
        this.pushMidiEvent(channelNumber, {
          startSample: sampleTotal,
          durationSamples: samples,
          note: this.frequencyToMidi(frequency),
          velocity: this.velocityFromEnvelope([volume, fade]),
          channel: channelNumber,
        });
        frameTotal += frames;
        sampleTotal += samples;
        continue;
      }

      if (op === "wave_note") {
        const len = args[1] != null ? parseNumber(args[1]) : state.default_length ?? 4;
        const [frames, nextMod] = this.computeNoteFrames(
          len,
          state.note_length,
          this.resolveTempoForFrame(channelNumber, frameTotal, state.tempo),
          state.duration_modifier,
        );
        state.duration_modifier = nextMod;
        const samples = this.framesToSamplesPrecise(frames);
        const semitone = parseNumber(args[2] ?? "0");
        const reg = this.applyPitchOffset(
          FREQUENCY_TABLE[Math.max(0, Math.min(FREQUENCY_TABLE.length - 1, semitone))],
          state,
        );
        const freq = regToFreqWave(reg) * WAVE_NOTE_FREQUENCY_SCALAR;
        const instrument = parseNumber(args[0] ?? String(state.wave_instrument ?? CHANNEL3_DEFAULT_INSTRUMENT));
        let [audio] = this.renderWave(samples, freq, instrument, state.wave_volume);
        if (state.volume_envelope) {
          audio = this.applyVolumeEnvelope(audio, state.volume_envelope, sampleTotal);
        }
        segments.push({ audio: toInt16ArrayTrunc(audio), pan: state.current_pan });
        this.pushMidiEvent(channelNumber, {
          startSample: sampleTotal,
          durationSamples: samples,
          note: this.frequencyToMidi(freq),
          velocity: this.velocityFromWaveVolume(state.wave_volume),
          channel: channelNumber,
        });
        frameTotal += frames;
        sampleTotal += samples;
        continue;
      }

      if (op === "noise_note") {
        const len = parseNumber(args[0]);
        const [frames, nextMod] = this.computeNoteFrames(
          len + 1,
          state.note_length,
          this.resolveTempoForFrame(channelNumber, frameTotal, state.tempo),
          state.duration_modifier,
        );
        state.duration_modifier = nextMod;
        const samples = this.framesToSamplesPrecise(frames);
        const note: NoiseNote = {
          length: len,
          volume: parseNumber(args[1]),
          fade: parseNumber(args[2]),
          frequency: parseNumber(args[3]),
        };
        const audio = this.renderNoise(samples, note, state, sampleTotal);
        segments.push({ audio, pan: state.current_pan });
        frameTotal += frames;
        sampleTotal += samples;
        continue;
      }

      if (op === "drum_note") {
        if (!noiseSamplingEnabled || currentDrumkit == null) {
          continue;
        }
        const instrument = parseNumber(args[0]);
        const len = parseNumber(args[1] ?? String(state.default_length ?? 4));
        const [frames, nextMod] = this.computeNoteFrames(
          len,
          state.note_length,
          this.resolveTempoForFrame(channelNumber, frameTotal, state.tempo),
          state.duration_modifier,
        );
        state.duration_modifier = nextMod;
        const samples = this.framesToSamplesPrecise(frames);
        const audio = this.renderDrum(currentDrumkit, instrument, samples, state, sampleTotal);
        segments.push({ audio, pan: state.current_pan });
        frameTotal += frames;
        sampleTotal += samples;
        continue;
      }

      if (
        op === "sound_ret"
        || op === "sound_call"
        || op === "sound_jump"
        || op === "sound_loop"
        || op === "label"
        || op === "assert"
        || op === "db"
        || op === "sfx_priority_on"
        || op === "sfx_priority_off"
        || op === "toggle_sfx"
      ) {
        continue;
      }

      throw new Error(`Unsupported ASM audio command '${op}' in strict converter mode.`);
    }

    this.channelFrameTotals.set(channelNumber, frameTotal);
    this.channelSampleTotals.set(channelNumber, sampleTotal);
    return segments;
  }

  private *commandStream(channelLabel: string, channelNumber: number): Iterable<AudioCommand> {
    const sources = {
      ...this.musicData.channels,
      ...this.musicData.subroutines,
      ...(this.musicData.shared_sources ?? {}),
    };

    const labelIndex = this.buildLabelIndices(sources);

    const stack: Array<{ src: string; pc: number; loops: Map<string, number> }> = [{
      src: channelLabel,
      pc: 0,
      loops: new Map(),
    }];

    let processed = 0;
    let mainloopSeen = false;

    while (stack.length > 0 && processed < MAX_COMMANDS_PER_CHANNEL) {
      const frame = stack[stack.length - 1];
      const src = sources[frame.src];
      if (!src) {
        stack.pop();
        continue;
      }
      if (frame.pc >= src.commands.length) {
        stack.pop();
        continue;
      }

      const cmd = src.commands[frame.pc];
      frame.pc += 1;
      processed += 1;

      const op = cmd.command;
      if (op === "label") {
        const lbl = cmd.args[0];
        if (lbl === ".mainloop" && stack.length === 1) {
          if (!mainloopSeen) {
            mainloopSeen = true;
            yield { command: "__loop_point__", args: [] };
          } else {
            return;
          }
        }
        continue;
      }

      if (op === "sound_call") {
        const directTarget = this.resolveSource(cmd.args[0], sources);
        const target = directTarget ?? this.resolveLabel(frame.src, cmd.args[0], labelIndex);
        if (target) {
          stack.push({ src: target.src, pc: target.pc, loops: new Map() });
        }
        continue;
      }

      if (op === "sound_ret") {
        if (stack.length > 1) {
          stack.pop();
        } else {
          return;
        }
        continue;
      }

      if (op === "sound_jump") {
        const directTarget = this.resolveSource(cmd.args[0], sources);
        const target = directTarget ?? this.resolveLabel(frame.src, cmd.args[0], labelIndex);
        if (target) {
          frame.src = target.src;
          frame.pc = target.pc;
        }
        continue;
      }

      if (op === "sound_loop") {
        const loopCount = parseNumber(cmd.args[0]);
        const directTarget = this.resolveSource(cmd.args[1], sources);
        const target = directTarget ?? this.resolveLabel(frame.src, cmd.args[1], labelIndex);
        if (!target) {
          continue;
        }
        const targetIsMainLoop = sources[target.src]?.commands[target.pc - 1]?.command === "label"
          && sources[target.src]?.commands[target.pc - 1]?.args[0] === ".mainloop";
        if (loopCount === 0 && targetIsMainLoop && stack.length === 1) {
          if (mainloopSeen) {
            return;
          }
          mainloopSeen = true;
          yield { command: "__loop_point__", args: [] };
        }

        const key = `${frame.src}:${frame.pc - 1}`;
        const remaining = frame.loops.get(key);
        if (remaining == null) {
          frame.loops.set(key, loopCount === 0 ? this.infiniteLoopRepeatLimit : Math.max(0, loopCount - 1));
        }

        const current = frame.loops.get(key) ?? 0;
        if (current <= 0) {
          frame.loops.delete(key);
          continue;
        }

        frame.loops.set(key, current - 1);
        frame.src = target.src;
        frame.pc = target.pc;
        continue;
      }

      yield cmd;
    }

    if (processed >= MAX_COMMANDS_PER_CHANNEL) {
      throw new Error(`Channel ${channelNumber} exceeded command safety limit.`);
    }
  }

  private buildLabelIndices(sources: Record<string, { commands: AudioCommand[] }>): Record<string, Record<string, number>> {
    const result: Record<string, Record<string, number>> = {};
    for (const [name, source] of Object.entries(sources)) {
      const mapping: Record<string, number> = {};
      source.commands.forEach((cmd, i) => {
        if (cmd.command === "label" && cmd.args[0]) {
          mapping[cmd.args[0]] = i + 1;
          if (cmd.args[0].startsWith(".")) {
            mapping[`${name}${cmd.args[0]}`] = i + 1;
          }
        }
      });
      result[name] = mapping;
    }
    return result;
  }

  private resolveLabel(sourceName: string, targetLabel: string, labelIndex: Record<string, Record<string, number>>): { src: string; pc: number } | null {
    if (labelIndex[sourceName]?.[targetLabel] != null) {
      return { src: sourceName, pc: labelIndex[sourceName][targetLabel] };
    }
    for (const [src, idx] of Object.entries(labelIndex)) {
      if (idx[targetLabel] != null) {
        return { src, pc: idx[targetLabel] };
      }
    }
    return null;
  }

  private resolveSource(targetLabel: string, sources: Record<string, { commands: AudioCommand[] }>): { src: string; pc: number } | null {
    if (sources[targetLabel] != null) {
      return { src: targetLabel, pc: 0 };
    }
    return null;
  }

  private recordNr50Event(offset: number, left: number, right: number): void {
    const normalizedOffset = Math.max(0, offset);
    const normalizedLeft = Math.max(0, Math.min(7, left));
    const normalizedRight = Math.max(0, Math.min(7, right));
    const existingIndex = this.nr50Events.findIndex((event) => event.offset === normalizedOffset);
    if (existingIndex >= 0) {
      this.nr50Events[existingIndex] = { offset: normalizedOffset, left: normalizedLeft, right: normalizedRight };
    } else {
      this.nr50Events.push({ offset: normalizedOffset, left: normalizedLeft, right: normalizedRight });
      this.nr50Events.sort((a, b) => a.offset - b.offset);
    }
  }

  private nr50GainAtOffset(offset: number): [number, number] {
    let left = 1.0;
    let right = 1.0;
    for (const event of this.nr50Events) {
      if (event.offset > offset) {
        break;
      }
      left = event.left > 0 ? event.left / 7.0 : 0.0;
      right = event.right > 0 ? event.right / 7.0 : 0.0;
    }
    return [left, right];
  }

  private mixStereo(): Int16Array {
    let maxSamples = 0;
    for (const segments of this.channelOutputs.values()) {
      let local = 0;
      for (const seg of segments) {
        local += seg.audio.length;
      }
      maxSamples = Math.max(maxSamples, local);
    }

    const stereo = new Int32Array(maxSamples * 2);

    for (const [channelNumber, segments] of this.channelOutputs.entries()) {
      let cursor = 0;
      for (const seg of segments) {
        const gain = CHANNEL_GAINS[channelNumber] ?? 1.0;
        const [leftOn, rightOn] = seg.pan;
        const leftScale = leftOn ? 1.0 : PAN_CROSSFEED;
        const rightScale = rightOn ? 1.0 : PAN_CROSSFEED;
        for (let i = 0; i < seg.audio.length; i += 1) {
          const sample = seg.audio[i] * gain;
          if (cursor + i >= maxSamples) {
            break;
          }
          const [masterLeft, masterRight] = this.nr50GainAtOffset(cursor + i);
          stereo[(cursor + i) * 2] += rintEven(sample * leftScale * masterLeft);
          stereo[(cursor + i) * 2 + 1] += rintEven(sample * rightScale * masterRight);
        }
        cursor += seg.audio.length;
      }
    }

    this.applyAnalogFilters(stereo);
    this.applyOutputQualityMode(stereo);

    let peak = 0;
    for (let i = 0; i < stereo.length; i += 1) {
      peak = Math.max(peak, Math.abs(stereo[i]));
    }
    if (peak > 32767) {
      const scale = 32767.0 / peak;
      for (let i = 0; i < stereo.length; i += 1) {
        stereo[i] = rintEven(stereo[i] * scale);
      }
    }

    const clipped = new Int16Array(stereo.length);
    for (let i = 0; i < stereo.length; i += 1) {
      clipped[i] = Math.max(-32767, Math.min(32767, stereo[i]));
    }
    return clipped;
  }

  private loopChannelToFrames(
    channelNumber: number,
    segments: Array<{ audio: Int16Array; pan: [boolean, boolean] }>,
    targetFrames: number,
  ): Array<{ audio: Int16Array; pan: [boolean, boolean] }> {
    const loopPoint = this.channelLoopPoints.get(channelNumber);
    if (!loopPoint) {
      return segments;
    }

    const [loopFrameOffset, loopSampleOffset] = loopPoint;
    let currentFrames = this.channelFrameTotals.get(channelNumber) ?? 0;
    if (loopFrameOffset >= currentFrames) {
      return segments;
    }

    const loopFrames = currentFrames - loopFrameOffset;
    if (loopFrames <= 0) {
      return segments;
    }

    const [prefix, loopBody] = this.splitSegmentsAtSample(segments, loopSampleOffset);
    if (loopBody.length === 0) {
      return segments;
    }

    const loopSamples = loopBody.reduce((sum, segment) => sum + segment.audio.length, 0);
    if (loopSamples <= 0) {
      return segments;
    }

    const out = [...prefix, ...loopBody];
    let framesRemaining = targetFrames - currentFrames;
    while (framesRemaining > 0) {
      if (framesRemaining >= loopFrames) {
        out.push(...loopBody.map((segment) => ({ audio: segment.audio.slice(), pan: segment.pan })));
        currentFrames += loopFrames;
        framesRemaining = targetFrames - currentFrames;
        continue;
      }

      const sampleBudget = this.framesToSamplesWithPrimaryRatio(framesRemaining, channelNumber);
      if (sampleBudget <= 0) {
        break;
      }
      out.push(...this.takePrefixSamples(loopBody, sampleBudget));
      currentFrames = targetFrames;
      framesRemaining = 0;
    }

    this.channelFrameTotals.set(channelNumber, currentFrames);
    this.channelSampleTotals.set(
      channelNumber,
      out.reduce((sum, segment) => sum + segment.audio.length, 0),
    );
    return out;
  }

  private extendChannelToFrames(
    channelNumber: number,
    segments: Array<{ audio: Int16Array; pan: [boolean, boolean] }>,
    targetFrames: number,
  ): Array<{ audio: Int16Array; pan: [boolean, boolean] }> {
    let currentFrames = this.channelFrameTotals.get(channelNumber) ?? 0;
    if (currentFrames >= targetFrames) {
      return segments;
    }

    const loopExtended = this.loopChannelToFrames(channelNumber, segments, targetFrames);
    currentFrames = this.channelFrameTotals.get(channelNumber) ?? currentFrames;
    if (currentFrames >= targetFrames) {
      return loopExtended;
    }

    const remainingFrames = targetFrames - currentFrames;
    const silenceSamples = this.framesToSamplesWithPrimaryRatio(remainingFrames, channelNumber);
    if (silenceSamples > 0) {
      loopExtended.push({ audio: new Int16Array(silenceSamples), pan: [true, true] });
      this.channelSampleTotals.set(channelNumber, (this.channelSampleTotals.get(channelNumber) ?? 0) + silenceSamples);
    }
    this.channelFrameTotals.set(channelNumber, targetFrames);
    return loopExtended;
  }

  private ensureAllChannelsSynced(exportFormat: "pcm" | "midi" | "mp3"): void {
    if (this.channelFrameTotals.size === 0) {
      return;
    }

    let maxFrames = 0;
    let maxSamplesPresent = 0;
    for (const [channelNumber, segments] of this.channelOutputs.entries()) {
      maxFrames = Math.max(maxFrames, this.channelFrameTotals.get(channelNumber) ?? 0);
      maxSamplesPresent = Math.max(
        maxSamplesPresent,
        segments.reduce((sum, segment) => sum + segment.audio.length, 0),
      );
    }
    if (maxFrames <= 0) {
      return;
    }

    const loopTarget = this.targetLoopedMusicSampleBudget(exportFormat);
    if (loopTarget != null && loopTarget > maxSamplesPresent) {
      const primary = this.primaryChannelNumber;
      if (primary != null) {
        const primaryFrames = Math.max(1, this.channelFrameTotals.get(primary) ?? maxFrames);
        const primarySamples = Math.max(1, this.channelSampleTotals.get(primary) ?? 0);
        maxFrames = Math.max(
          maxFrames,
          Math.floor((loopTarget * primaryFrames + primarySamples - 1) / primarySamples),
        );
      }
    }

    let maxExtendedSamples = maxSamplesPresent;
    for (const [channelNumber, segments] of this.channelOutputs.entries()) {
      const extended = this.extendChannelToFrames(channelNumber, segments, maxFrames);
      this.channelOutputs.set(channelNumber, extended);
      maxExtendedSamples = Math.max(
        maxExtendedSamples,
        extended.reduce((sum, segment) => sum + segment.audio.length, 0),
      );
    }

    const [estimatedSamples] = this.framesToSamplesExact(maxFrames, 0);
    let targetSamples = Math.max(maxExtendedSamples, loopTarget ?? 0, estimatedSamples);
    if (targetSamples <= 0) {
      targetSamples = estimatedSamples;
    }

    for (const [channelNumber, segments] of this.channelOutputs.entries()) {
      const aligned = this.alignSegmentsToTarget(segments, targetSamples);
      this.channelOutputs.set(channelNumber, aligned);
      this.channelSampleTotals.set(channelNumber, targetSamples);
    }
  }

  private targetLoopedMusicSampleBudget(exportFormat: "pcm" | "midi" | "mp3"): number | null {
    if (exportFormat !== "mp3") {
      return null;
    }
    if (this.qualityMode === "accurate") {
      return null;
    }
    if (Object.keys(this.musicData.channels).length < 3) {
      return null;
    }
    const primary = this.primaryChannelNumber;
    if (primary == null || !this.channelLoopPoints.has(primary)) {
      return null;
    }
    const current = this.channelSampleTotals.get(primary) ?? 0;
    if (this.loopedMusicExportSeconds == null) {
      return null;
    }
    const target = Math.round(this.loopedMusicExportSeconds * SAMPLE_RATE);
    return current >= target ? null : target;
  }

  private framesToSamplesWithPrimaryRatio(frames: number, channelNumber: number): number {
    if (frames <= 0) {
      return 0;
    }
    const primary = this.primaryChannelNumber;
    if (primary == null) {
      const [samples] = this.framesToSamplesExact(frames, 0);
      return samples;
    }

    const primaryFrames = this.channelFrameTotals.get(primary) ?? 0;
    const primarySamples = this.channelSampleTotals.get(primary) ?? 0;
    if (primaryFrames <= 0 || primarySamples <= 0) {
      const [samples] = this.framesToSamplesExact(frames, 0);
      return samples;
    }

    const currentFrames = this.channelFrameTotals.get(channelNumber) ?? primaryFrames;
    const currentSamples = this.channelSampleTotals.get(channelNumber) ?? primarySamples;
    const ratioSamples = Math.floor((frames * currentSamples + Math.max(1, currentFrames) - 1) / Math.max(1, currentFrames));
    return ratioSamples > 0 ? ratioSamples : Math.floor((frames * primarySamples + primaryFrames - 1) / primaryFrames);
  }

  private splitSegmentsAtSample(
    segments: Array<{ audio: Int16Array; pan: [boolean, boolean] }>,
    sampleOffset: number,
  ): [
    Array<{ audio: Int16Array; pan: [boolean, boolean] }>,
    Array<{ audio: Int16Array; pan: [boolean, boolean] }>,
  ] {
    const prefix: Array<{ audio: Int16Array; pan: [boolean, boolean] }> = [];
    const suffix: Array<{ audio: Int16Array; pan: [boolean, boolean] }> = [];
    let cursor = 0;
    const splitPoint = Math.max(0, sampleOffset);

    for (const segment of segments) {
      const nextCursor = cursor + segment.audio.length;
      if (nextCursor <= splitPoint) {
        prefix.push({ audio: segment.audio.slice(), pan: segment.pan });
      } else if (cursor >= splitPoint) {
        suffix.push({ audio: segment.audio.slice(), pan: segment.pan });
      } else {
        const localSplit = splitPoint - cursor;
        if (localSplit > 0) {
          prefix.push({ audio: segment.audio.slice(0, localSplit), pan: segment.pan });
        }
        if (localSplit < segment.audio.length) {
          suffix.push({ audio: segment.audio.slice(localSplit), pan: segment.pan });
        }
      }
      cursor = nextCursor;
    }

    return [prefix, suffix];
  }

  private takePrefixSamples(
    segments: Array<{ audio: Int16Array; pan: [boolean, boolean] }>,
    sampleBudget: number,
  ): Array<{ audio: Int16Array; pan: [boolean, boolean] }> {
    if (sampleBudget <= 0) {
      return [];
    }

    const out: Array<{ audio: Int16Array; pan: [boolean, boolean] }> = [];
    let remaining = sampleBudget;
    for (const segment of segments) {
      if (remaining <= 0) {
        break;
      }
      const take = Math.min(remaining, segment.audio.length);
      if (take > 0) {
        out.push({ audio: segment.audio.slice(0, take), pan: segment.pan });
        remaining -= take;
      }
    }
    return out;
  }

  private alignSegmentsToTarget(
    segments: Array<{ audio: Int16Array; pan: [boolean, boolean] }>,
    targetSamples: number,
  ): Array<{ audio: Int16Array; pan: [boolean, boolean] }> {
    const out: Array<{ audio: Int16Array; pan: [boolean, boolean] }> = [];
    let remaining = Math.max(0, targetSamples);
    for (const segment of segments) {
      if (remaining <= 0) {
        break;
      }
      const take = Math.min(remaining, segment.audio.length);
      if (take > 0) {
        out.push({
          audio: take === segment.audio.length ? segment.audio : segment.audio.slice(0, take),
          pan: segment.pan,
        });
        remaining -= take;
      }
    }
    if (remaining > 0) {
      out.push({ audio: new Int16Array(remaining), pan: [true, true] });
    }
    return out;
  }

  private applyAnalogFilters(stereo: Int32Array): void {
    if (stereo.length === 0) {
      return;
    }

    const length = Math.floor(stereo.length / 2);
    const left = new Float64Array(length);
    const right = new Float64Array(length);
    for (let i = 0; i < length; i += 1) {
      left[i] = stereo[i * 2];
      right[i] = stereo[i * 2 + 1];
    }

    const decay = DMG_HPF_DECAY;
    let capL = 0.0;
    let capR = 0.0;
    for (let i = 0; i < length; i += 1) {
      const inL = left[i];
      const outL = inL - capL;
      capL = inL - outL * decay;
      left[i] = outL;

      const inR = right[i];
      const outR = inR - capR;
      capR = inR - outR * decay;
      right[i] = outR;
    }

    const dt = 1.0 / SAMPLE_RATE;
    const rc = 1.0 / (2.0 * Math.PI * 4300.0);
    const alpha = dt / (rc + dt);
    this.applyLowPass(left, alpha);
    this.applyLowPass(left, alpha);
    this.applyLowPass(right, alpha);
    this.applyLowPass(right, alpha);

    for (let i = 0; i < length; i += 1) {
      stereo[i * 2] = rintEven(left[i]);
      stereo[i * 2 + 1] = rintEven(right[i]);
    }
  }

  private applyLowPass(channel: Float64Array, alpha: number): void {
    if (channel.length === 0) {
      return;
    }
    let prev = channel[0];
    for (let i = 1; i < channel.length; i += 1) {
      prev = alpha * channel[i] + (1.0 - alpha) * prev;
      channel[i] = prev;
    }
  }

  private applyOutputQualityMode(stereo: Int32Array): void {
    if (stereo.length === 0 || this.qualityMode === "accurate") {
      return;
    }
    const length = Math.floor(stereo.length / 2);
    const alpha = 0.25;
    let prevL = stereo[0];
    let prevR = stereo[1];
    for (let i = 0; i < length; i += 1) {
      const idx = i * 2;
      if (i > 0) {
        prevL = alpha * stereo[idx] + (1.0 - alpha) * prevL;
        prevR = alpha * stereo[idx + 1] + (1.0 - alpha) * prevR;
      }
      stereo[idx] = rintEven(prevL * ENHANCED_MASTER_GAIN);
      stereo[idx + 1] = rintEven(prevR * ENHANCED_MASTER_GAIN);
    }
  }

  private registerFromNote(name: string, st: ChannelState): number {
    if (NOTE_INDEX[name] == null) {
      return 0;
    }
    const pitchIdx = 1 + NOTE_INDEX[name] + st.transpose_pitches;
    if (pitchIdx < 1 || pitchIdx >= FREQUENCY_TABLE.length) {
      throw new Error(`Note index ${pitchIdx} is outside FrequencyTable bounds for ${name}.`);
    }

    const freqVal = FREQUENCY_TABLE[pitchIdx];
    let freqSigned = freqVal < 0x8000 ? freqVal : freqVal - 0x10000;
    let octaveAcc = st.octave + st.transpose_octaves;
    if (octaveAcc < 0 || octaveAcc > 15) {
      throw new Error(`Octave accumulator ${octaveAcc} is out of range.`);
    }
    while (octaveAcc < 7) {
      freqSigned >>= 1;
      octaveAcc += 1;
    }
    return freqSigned & 0x7ff;
  }

  private registerToFrequency(n: number): number {
    const clamped = Math.max(0, Math.min(2047, n));
    return 131072.0 / (2048 - clamped);
  }

  private noteToRegisterPulse(name: string, st: ChannelState): number {
    const reg = this.registerFromNote(name, st);
    return this.applyPitchOffset(reg, st);
  }

  private applyPitchOffset(register: number, st: ChannelState): number {
    // The engine performs a 16-bit add and the APU consumes the low 11 bits.
    return (register + st.pitch_offset + st.instrument_pitch_offset) & 0x7ff;
  }

  private noteToRegisterWave(name: string, st: ChannelState): number {
    const pulseReg = this.noteToRegisterPulse(name, st);
    const hz = this.registerToFrequency(pulseReg) * WAVE_NOTE_FREQUENCY_SCALAR;
    return freqToRegWave(hz);
  }

  private renderPulse(sampleCount: number, frequency: number, dutyCycle: number, phaseAcc: number): [Float32Array, number] {
    // ASM parity (audio/engine.asm::GetFrequency): pulse DAC is unsigned pre-filter.
    const pattern = Float32Array.from((GB_DUTY_PATTERNS[dutyCycle] ?? GB_DUTY_PATTERNS[2]).map((v) => (v > 0 ? 1 : 0)));
    const integral = computeIntegral(pattern);
    const inc = Math.trunc((frequency * 2 ** 48) / SAMPLE_RATE);
    return pulseKernel(sampleCount, inc, phaseAcc, integral, MAX_WAVE_AMPLITUDE);
  }

  private renderWave(
    sampleCount: number,
    frequency: number,
    instrumentId: number,
    volume: number,
    phaseAcc: number = Math.floor((2 ** 48) / 32),
  ): [Float32Array, number] {
    const sampleIdx = this.waveInstrumentMap[instrumentId] ?? instrumentId;
    const nybbles = this.waveSamples[sampleIdx] ?? new Array(32).fill(0);
    const shiftMap: Record<number, number> = { 0: 4, 1: 0, 2: 1, 3: 2 };
    const shift = shiftMap[volume & 0x3] ?? 4;
    if (shift >= 4) {
      return [new Float32Array(sampleCount), phaseAcc];
    }
    const pattern = Float32Array.from(nybbles.map((v) => ((((v >> shift) & 0xf) / 15.0) * 2.0) - 1.0));
    const integral = computeIntegral(pattern);
    const inc = Math.trunc((frequency * 2 ** 48) / SAMPLE_RATE);
    return waveKernel(sampleCount, inc, phaseAcc, integral, MAX_WAVE_AMPLITUDE);
  }

  private renderNoise(sampleCount: number, note: NoiseNote, state: ChannelState, startSampleOffset: number): Int16Array {
    if (note.volume <= 0 || sampleCount <= 0) {
      return new Int16Array(Math.max(0, sampleCount));
    }
    if (startSampleOffset === 0) {
      state.noise_lfsr = 0x7fff;
      state.noise_accumulator = 0;
    }
    const shift = (note.frequency >> 4) & 0xf;
    const width = (note.frequency >> 3) & 0x1;
    const divCode = note.frequency & 0x7;
    const divisors = [8, 16, 32, 48, 64, 80, 96, 112];
    const period = divisors[divCode] << (shift + 1);
    const envelope = this.generateEnvelopeCurve(sampleCount, [note.volume, note.fade], startSampleOffset);
    return noiseKernelWrapper(sampleCount, { period_num: period, period_den: 1, width_mode: width }, envelope, state);
  }

  private renderDrum(kitId: number, instrumentId: number, sampleCount: number, state: ChannelState, startSampleOffset: number): Int16Array {
    const kit = this.drumkits[kitId] ?? {};
    const notes = kit[instrumentId] ?? [];
    if (notes.length === 0) {
      return new Int16Array(sampleCount);
    }

    let remainder = 0;
    let localOffset = 0;
    const chunks: Int16Array[] = [];
    for (let i = 0; i < notes.length; i += 1) {
      const note = notes[i];
      const frames = this.noiseLengthToFrames(note.length);
      const [subSamples, nextRemainder] = this.framesToSamplesExact(frames, remainder);
      remainder = nextRemainder;
      if (subSamples <= 0) {
        continue;
      }
      const rendered = this.renderNoise(subSamples, note, state, i === 0 ? 0 : startSampleOffset + localOffset);
      chunks.push(rendered);
      localOffset += subSamples;
    }
    const concatenated = concatInt16Arrays(chunks);
    if (concatenated.length >= sampleCount) {
      return concatenated.slice(0, sampleCount);
    }
    if (concatenated.length === sampleCount) {
      return concatenated;
    }
    return concatInt16Arrays([concatenated, new Int16Array(sampleCount - concatenated.length)]);
  }

  private noiseLengthToFrames(lengthRaw: number): number {
    if (lengthRaw <= 0) {
      return 64;
    }
    if (lengthRaw < 64) {
      return Math.max(1, 64 - lengthRaw);
    }
    return lengthRaw;
  }

  private setNoteDurationTicks(rawLength: number): number {
    return Math.max(1, rawLength + 1);
  }

  private decodeDutyCyclePatternByte(args: string[]): number {
    const values = args.slice(0, 4).map((token) => Math.max(0, Math.min(3, parseNumber(token))));
    while (values.length < 4) {
      values.push(0);
    }
    const packed = ((values[0] & 0x3) << 6)
      | ((values[1] & 0x3) << 4)
      | ((values[2] & 0x3) << 2)
      | (values[3] & 0x3);
    return ((packed >> 2) | ((packed & 0x3) << 6)) & 0xff;
  }

  private advanceDutyCyclePatternFrame(state: ChannelState): void {
    if (!state.duty_cycle_loop_enabled || state.duty_cycle_pattern_byte == null) {
      return;
    }
    const rotated = ((state.duty_cycle_pattern_byte << 2) | (state.duty_cycle_pattern_byte >> 6)) & 0xff;
    state.duty_cycle_pattern_byte = rotated;
    state.duty_cycle = (rotated >> 6) & 0x3;
  }

  private splitSamplesAcrossFrames(sampleCount: number, frameCount: number): number[] {
    if (sampleCount <= 0 || frameCount <= 0) {
      return [];
    }
    const out: number[] = [];
    for (let i = 0; i < frameCount; i += 1) {
      const start = Math.floor((i * sampleCount) / frameCount);
      const end = Math.floor(((i + 1) * sampleCount) / frameCount);
      out.push(Math.max(0, end - start));
    }
    return out;
  }

  private parseVibratoExtentAndRate(args: string[]): [number, number] {
    if (args.length === 0) {
      return [0, 0];
    }
    if (args.length === 1) {
      const combined = parseNumber(args[0]);
      return [Math.max(0, (combined >> 4) & 0xf), Math.max(0, combined & 0xf)];
    }
    return [Math.max(0, parseNumber(args[0])), Math.max(0, parseNumber(args[1]))];
  }

  private parsePitchSweepArgs(args: string[]): number {
    if (args.length === 0) {
      return 0;
    }
    if (args.length === 1) {
      return parseNumber(args[0]) & 0xff;
    }
    const time = parseNumber(args[0]) & 0xf;
    const shiftRaw = parseNumber(args[1]);
    const direction = shiftRaw < 0 ? 0x8 : 0;
    const shift = Math.abs(shiftRaw) & 0x7;
    return ((time & 0xf) << 4) | ((direction | shift) & 0xf);
  }

  private isPitchSweepActive(value: number): boolean {
    if (value === 0) {
      return false;
    }
    const time = (value >> 4) & 0x7;
    const shift = value & 0x7;
    return time > 0 && shift > 0;
  }

  private preflightPitchSweep(state: ChannelState): void {
    if (!state.pitch_sweep_enabled) {
      return;
    }
    const value = state.pitch_sweep_value & 0xff;
    const shift = value & 0x7;
    if (shift === 0) {
      return;
    }
    const shadow = Math.max(0, Math.min(2047, state.pitch_sweep_shadow));
    const direction = (value & 0x8) !== 0 ? -1 : 1;
    const candidate = shadow + direction * (shadow >> shift);
    if (candidate < 0 || candidate > 2047) {
      state.pulse1_active = false;
    }
  }

  private resetVibratoNoteState(state: ChannelState, baseRegister: number): void {
    state.vibrato_delay_count = Math.max(0, state.vibrato_delay);
    state.vibrato_latched_reg = Math.max(0, Math.min(2047, baseRegister));
  }

  private computeVibratoSegments(frames: number, state: ChannelState, baseRegister: number): Array<[number, number]> {
    if (frames <= 0) {
      return [];
    }
    const base = Math.max(0, Math.min(2047, baseRegister));
    let delayCount = Math.max(0, state.vibrato_delay_count);
    const extent = Math.max(0, state.vibrato_extent);
    const extentUp = Math.max(state.vibrato_extent_up, Math.floor((extent + 1) / 2));
    const extentDown = Math.max(state.vibrato_extent_down, Math.floor(extent / 2));
    const rate = Math.max(0, state.vibrato_rate) & 0xf;
    let rateCounter = Math.max(0, state.vibrato_rate_counter) & 0xf;
    let directionUp = Boolean(state.vibrato_direction_up);
    let latched = state.vibrato_latched_reg <= 0 || state.vibrato_latched_reg > 2047 ? base : state.vibrato_latched_reg;
    const baseLow = base & 0xff;
    const baseHigh = (base >> 8) & 0x7;
    const perFrame: number[] = [];

    for (let i = 0; i < frames; i += 1) {
      if (delayCount > 0) {
        delayCount -= 1;
      } else if (extentUp > 0 || extentDown > 0) {
        if (rateCounter === 0) {
          if (directionUp) {
            directionUp = false;
            const low = Math.max(0, baseLow - extentDown);
            latched = (baseHigh << 8) | low;
          } else {
            directionUp = true;
            const low = Math.min(0xff, baseLow + extentUp);
            latched = (baseHigh << 8) | low;
          }
          rateCounter = rate;
        } else {
          rateCounter = (rateCounter - 1) & 0xf;
        }
      }

      perFrame.push(Math.max(0, Math.min(2047, latched)));
    }

    state.vibrato_delay_count = delayCount;
    state.vibrato_rate_counter = rateCounter;
    state.vibrato_direction_up = directionUp;
    state.vibrato_latched_reg = latched;

    const out: Array<[number, number]> = [];
    for (const reg of perFrame) {
      if (out.length > 0 && out[out.length - 1][1] === reg) {
        out[out.length - 1][0] += 1;
      } else {
        out.push([1, reg]);
      }
    }
    return out;
  }

  private computeSweepSegments(
    frames: number,
    baseRegister: number,
    state: ChannelState,
    channel: number,
    startSampleOffset: number,
  ): Array<[number, number | null]> {
    if (channel !== 1 || frames <= 0 || baseRegister <= 0) {
      return [[frames, baseRegister]];
    }
    if (!state.pitch_sweep_enabled) {
      state.pitch_sweep_shadow = baseRegister;
      return [[frames, baseRegister]];
    }
    if (!state.pulse1_active) {
      return [[frames, null]];
    }

    const value = state.pitch_sweep_value & 0xff;
    const shift = value & 0x7;
    const timeNibble = (value >> 4) & 0x7;
    if (shift === 0 || timeNibble === 0) {
      state.pitch_sweep_shadow = baseRegister;
      return [[frames, baseRegister]];
    }

    const direction = (value & 0x8) !== 0 ? -1 : 1;
    const ticksPerStep = timeNibble * 4;
    const startTick = sampleOffsetToFrameTick(startSampleOffset);
    const firstEdge = next128hzTickStrict(startTick);
    let tickFrameRemainder = 0;
    const initialDiff = Math.max(0, firstEdge - startTick);
    const [framesUntilInitial, remA] = ticksToFrames(initialDiff, tickFrameRemainder);
    tickFrameRemainder = remA;
    const [delayFrames, remB] = ticksToFrames(ticksPerStep, tickFrameRemainder);
    tickFrameRemainder = remB;
    let framesUntil = Math.max(1, framesUntilInitial + delayFrames);

    let shadow = Math.max(0, Math.min(2047, state.pitch_sweep_shadow || baseRegister));
    state.pitch_sweep_shadow = shadow;
    let current = shadow;
    let remaining = frames;
    const out: Array<[number, number | null]> = [];

    while (remaining > 0) {
      const segmentFrames = Math.min(remaining, framesUntil);
      out.push([segmentFrames, current]);
      remaining -= segmentFrames;
      framesUntil -= segmentFrames;
      if (remaining <= 0) {
        break;
      }
      if (framesUntil > 0) {
        continue;
      }

      const delta = shadow >> shift;
      const next = shadow + direction * delta;
      if (next < 0 || next > 2047) {
        state.pulse1_active = false;
        state.pitch_sweep_shadow = shadow;
        if (remaining > 0) {
          out.push([remaining, null]);
        }
        break;
      }

      shadow = Math.max(0, Math.min(2047, next));
      state.pitch_sweep_shadow = shadow;
      current = shadow;
      const [stepFrames, nextRemainder] = ticksToFrames(ticksPerStep, tickFrameRemainder);
      tickFrameRemainder = nextRemainder;
      framesUntil = Math.max(1, stepFrames);
    }

    return out;
  }

  private computePitchSlideSegments(
    frames: number,
    startRegister: number,
    targetRegister: number,
    slideFrames: number,
  ): Array<[number, number]> {
    if (frames <= 0) {
      return [];
    }
    const ramp = Math.max(1, Math.min(frames, slideFrames));
    const start = Math.max(0, Math.min(2047, Math.trunc(startRegister)));
    const target = Math.max(0, Math.min(2047, Math.trunc(targetRegister)));
    if (start === target) {
      return [[frames, start]];
    }

    const direction = target > start ? 1 : -1;
    const distance = Math.abs(target - start);
    const step = Math.floor(distance / ramp);
    const remainder = distance % ramp;
    let fractionalAccumulator = 0;
    let current = start;
    let done = false;
    const perFrame: number[] = [];

    for (let i = 0; i < frames; i += 1) {
      perFrame.push(current);
      if (i >= ramp || done) {
        continue;
      }
      let next = current + direction * step;
      fractionalAccumulator += remainder;
      if (fractionalAccumulator >= ramp) {
        next += direction;
        fractionalAccumulator -= ramp;
      }

      if ((direction > 0 && next >= target) || (direction < 0 && next <= target)) {
        current = target;
        done = true;
      } else {
        current = Math.max(0, Math.min(2047, next));
      }
    }

    const out: Array<[number, number]> = [];
    for (const reg of perFrame) {
      if (out.length > 0 && out[out.length - 1][1] === reg) {
        out[out.length - 1][0] += 1;
      } else {
        out.push([1, reg]);
      }
    }
    return out;
  }

  private computeNoteFrames(ticks: number, noteLength: number, tempo: number, remainder: number): [number, number] {
    const t = Math.max(1, ticks);
    const n = Math.max(1, noteLength);
    const tp = Math.max(1, tempo || DEFAULT_TEMPO);
    const total = n * t * tp + remainder;
    return [Math.max(1, total >> 8), total & 0xff];
  }

  private primeSharedTempoEvents(channels: Array<[string, ParsedMusicData["channels"][string]]>): void {
    for (const [label, info] of channels) {
      const before = this.sharedTempoEvents.length;
      this.scanChannelForSharedTempoEvents(label, info.number ?? 0);
      if (this.sharedTempoEvents.length !== before) {
        this.tempoSourceChannel = info.number ?? null;
        return;
      }
    }
  }

  private scanChannelForSharedTempoEvents(channelLabel: string, channelNumber: number): void {
    const state = createChannelState();
    let frameTotal = 0;
    for (const cmd of this.commandStream(channelLabel, channelNumber)) {
      const op = cmd.command;
      const args = cmd.args;
      if (op === "tempo") {
        state.tempo = parseNumber(args[0]);
        this.recordSharedTempoEvent(frameTotal, state.tempo, state.duration_modifier);
        continue;
      }
      if (op === "speed") {
        state.note_length = Math.max(1, parseNumber(args[0]));
        state.default_length = state.note_length;
        continue;
      }
      if (op === "note_type" || op === "drum_speed") {
        state.note_length = Math.max(1, parseNumber(args[0]));
        state.default_length = state.note_length;
        continue;
      }
      if (op === "note") {
        const len = args[1] != null ? parseNumber(args[1]) : state.default_length ?? 4;
        const [frames, nextMod] = this.computeNoteFrames(len, state.note_length, state.tempo, state.duration_modifier);
        state.duration_modifier = nextMod;
        frameTotal += frames;
        continue;
      }
      if (op === "rest") {
        const [frames, nextMod] = this.computeNoteFrames(parseNumber(args[0]), state.note_length, state.tempo, state.duration_modifier);
        state.duration_modifier = nextMod;
        frameTotal += frames;
        continue;
      }
      if (op === "square_note") {
        const [frames, nextMod] = this.computeNoteFrames(
          this.setNoteDurationTicks(parseNumber(args[0])),
          state.note_length,
          state.tempo,
          state.duration_modifier,
        );
        state.duration_modifier = nextMod;
        frameTotal += frames;
        continue;
      }
      if (op === "wave_note" || op === "drum_note") {
        const len = args[1] != null ? parseNumber(args[1]) : state.default_length ?? 4;
        const [frames, nextMod] = this.computeNoteFrames(len, state.note_length, state.tempo, state.duration_modifier);
        state.duration_modifier = nextMod;
        frameTotal += frames;
        continue;
      }
      if (op === "noise_note") {
        const [frames, nextMod] = this.computeNoteFrames(
          this.setNoteDurationTicks(parseNumber(args[0])),
          state.note_length,
          state.tempo,
          state.duration_modifier,
        );
        state.duration_modifier = nextMod;
        frameTotal += frames;
      }
    }
  }

  private initialSharedTempoEvent(): { frame: number; tempo: number; remainder: number } | null {
    if (this.sharedTempoEvents.length === 0) {
      return null;
    }
    return this.sharedTempoEvents[0];
  }

  private recordSharedTempoEvent(frame: number, tempo: number, remainder: number): void {
    const normalizedFrame = Math.max(0, frame);
    const existing = this.sharedTempoEvents.find((event) => event.frame === normalizedFrame);
    if (existing) {
      existing.tempo = tempo;
      existing.remainder = remainder & 0xff;
      return;
    }
    this.sharedTempoEvents.push({ frame: normalizedFrame, tempo, remainder: remainder & 0xff });
    this.sharedTempoEvents.sort((a, b) => a.frame - b.frame);
  }

  private resolveTempoForFrame(channelNumber: number, frame: number, fallback: number): number {
    if (channelNumber === this.primaryChannelNumber || this.sharedTempoEvents.length === 0) {
      return fallback;
    }

    let tempo = fallback;
    for (const event of this.sharedTempoEvents) {
      if (event.frame > frame) {
        break;
      }
      tempo = event.tempo;
    }
    return tempo;
  }

  private framesToSamplesExact(frames: number, remainder: number): [number, number] {
    if (frames <= 0) {
      return [0, remainder];
    }
    const total = frames * FRAME_TO_SAMPLE_NUMERATOR + remainder;
    return [Math.floor(total / FRAME_TO_SAMPLE_DENOMINATOR), total % FRAME_TO_SAMPLE_DENOMINATOR];
  }

  private framesToSamplesPrecise(frames: number): number {
    const [samples, next] = this.framesToSamplesExact(frames, this.sharedSampleRemainder);
    this.sharedSampleRemainder = next;
    return samples;
  }

  private pushMidiEvent(channel: number, event: MidiEvent): void {
    const list = this.midiEvents.get(channel) ?? [];
    list.push(event);
    this.midiEvents.set(channel, list);
  }

  private frequencyToMidi(freq: number): number {
    if (freq <= 0) {
      return 0;
    }
    const midi = rintEven(69 + 12 * Math.log2(freq / 440));
    return Math.max(0, Math.min(127, midi));
  }

  private velocityFromEnvelope(envelope: [number, number] | null): number {
    const initial = envelope?.[0] ?? 15;
    return Math.max(1, Math.min(127, rintEven((Math.max(0, Math.min(15, initial)) / 15.0) * 127.0)));
  }

  private velocityFromWaveVolume(volume: number): number {
    const scale = ({ 0: 0.0, 1: 1.0, 2: 0.5, 3: 0.25 } as const)[volume & 0x3] ?? 0.0;
    if (scale <= 0) {
      return 1;
    }
    return Math.max(1, Math.min(127, rintEven(127.0 * scale)));
  }

  private normalizeEnvelopeFade(fade: number): number {
    return Math.abs(fade) === 8 ? 0 : fade;
  }

  private parseEnvelopeParameters(env: [number, number]): [number, number, number] {
    const initial = Math.max(0, Math.min(15, env[0]));
    const fade = this.normalizeEnvelopeFade(env[1]);
    const direction = fade < 0 ? 1 : -1;
    const magnitude = Math.abs(fade);
    let period = magnitude & 0x7;
    if (magnitude > 0 && period === 0) {
      period = 8;
    }
    return [initial, direction, period];
  }

  private generateEnvelopeCurve(sampleCount: number, envelope: [number, number], startSampleOffset: number): Float64Array {
    const [initial, direction, period] = this.parseEnvelopeParameters(envelope);
    const curve = new Float64Array(Math.max(0, sampleCount));
    if (sampleCount <= 0) {
      return curve;
    }
    if (period === 0) {
      curve.fill(initial / 15.0);
      return curve;
    }

    const startTick = Math.floor((Math.max(0, startSampleOffset) * FRAME_SEQUENCER_RATE) / SAMPLE_RATE);
    const firstStep7 = nextFrameTickWithStepStrict(startTick, 7);
    const firstChange = firstStep7 + Math.max(0, (period - 1) * 8);
    const changeTicks = period * 8;

    for (let i = 0; i < sampleCount; i += 1) {
      const sampleOffset = Math.max(0, startSampleOffset + i);
      const tick = Math.floor((sampleOffset * FRAME_SEQUENCER_RATE) / SAMPLE_RATE);
      const delta = tick - firstChange;
      const changes = delta >= 0 ? 1 + Math.floor(delta / changeTicks) : 0;
      const vol = Math.max(0, Math.min(15, initial + direction * changes));
      curve[i] = vol / 15.0;
    }
    return curve;
  }

  private applyVolumeEnvelope<T extends Int16Array | Float32Array>(
    wave: T,
    envelope: [number, number],
    startSampleOffset: number,
  ): T {
    if (wave.length === 0) {
      return wave;
    }

    const levels = this.generateEnvelopeCurve(wave.length, envelope, startSampleOffset);
    const isIntInput = wave instanceof Int16Array;
    const out = isIntInput ? new Int16Array(wave.length) : new Float32Array(wave.length);

    for (let i = 0; i < wave.length; i += 1) {
      const value = wave[i] * levels[i];
      out[i] = isIntInput ? clipI16(rintEven(value)) : Math.fround(value);
    }
    return out as T;
  }

  private parseInlineWave(args: string[]): number[] {
    if (args.length === 0) {
      throw new Error("load_wave requires inline wave data.");
    }
    const mode = args[0].toLowerCase();
    const useDb = mode === "db";
    const useDn = mode === "dn";
    const values = useDb || useDn ? args.slice(1) : args;
    const samples: number[] = [];

    if (useDb) {
      for (const token of values) {
        const value = parseNumber(token);
        if (value < 0 || value > 0xff) {
          throw new Error(`load_wave db value ${token} out of range.`);
        }
        samples.push((value >> 4) & 0xf, value & 0xf);
      }
    } else {
      for (const token of values) {
        const value = parseNumber(token);
        if (value < 0 || value > 0xf) {
          throw new Error(`load_wave dn value ${token} out of range.`);
        }
        samples.push(value);
      }
    }

    if (samples.length !== 32) {
      throw new Error(`load_wave expects 32 nybbles, got ${samples.length}.`);
    }
    return samples;
  }

  private registerInlineWave(pattern: number[]): number {
    const sampleIndex = this.nextWaveSampleIndex;
    this.waveSamples[sampleIndex] = [...pattern];
    this.nextWaveSampleIndex += 1;

    let instrumentId = this.nextWaveInstrumentId;
    while (this.waveInstrumentMap[instrumentId] != null) {
      instrumentId += 1;
    }
    this.waveInstrumentMap[instrumentId] = sampleIndex;
    this.nextWaveInstrumentId = instrumentId + 1;
    return instrumentId;
  }
}

const clipI16 = (v: number): number => Math.max(-32768, Math.min(32767, v));
const toInt16Trunc = (v: number): number => {
  const nearest = Math.round(v);
  const normalized = Math.abs(v - nearest) < 1e-9 ? nearest : v;
  return clipI16(normalized < 0 ? Math.ceil(normalized) : Math.floor(normalized));
};
const toInt16ArrayTrunc = (values: Float32Array): Int16Array => {
  const out = new Int16Array(values.length);
  for (let i = 0; i < values.length; i += 1) {
    out[i] = toInt16Trunc(values[i]);
  }
  return out;
};
const rintEven = (v: number): number => {
  if (!Number.isFinite(v)) {
    return 0;
  }
  const floor = Math.floor(v);
  const frac = v - floor;
  if (frac < 0.5) {
    return floor;
  }
  if (frac > 0.5) {
    return floor + 1;
  }
  return floor % 2 === 0 ? floor : floor + 1;
};
const sumFrames = (values: number[]): number => values.reduce((total, value) => total + value, 0);
const concatInt16Arrays = (chunks: Int16Array[]): Int16Array => {
  if (chunks.length === 0) {
    return new Int16Array(0);
  }
  const total = chunks.reduce((size, chunk) => size + chunk.length, 0);
  const out = new Int16Array(total);
  let cursor = 0;
  for (const chunk of chunks) {
    out.set(chunk, cursor);
    cursor += chunk.length;
  }
  return out;
};
const concatFloat32Arrays = (chunks: Float32Array[]): Float32Array => {
  if (chunks.length === 0) {
    return new Float32Array(0);
  }
  const total = chunks.reduce((size, chunk) => size + chunk.length, 0);
  const out = new Float32Array(total);
  let cursor = 0;
  for (const chunk of chunks) {
    out.set(chunk, cursor);
    cursor += chunk.length;
  }
  return out;
};
const isBooleanToken = (token: string): boolean => token.toUpperCase() === "TRUE" || token.toUpperCase() === "FALSE";
const parseBooleanToken = (token: string): boolean => token.toUpperCase() === "TRUE";
const maxObjectKey = (table: Record<number, unknown>, fallback: number): number => {
  let max = fallback;
  for (const key of Object.keys(table)) {
    const value = Number(key);
    if (Number.isFinite(value) && value > max) {
      max = value;
    }
  }
  return max;
};

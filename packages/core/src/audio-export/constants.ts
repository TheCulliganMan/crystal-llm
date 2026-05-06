export const SAMPLE_RATE = 44_100;
export const MP3_EXPORT_SAMPLE_RATE = 22_050;
export const MP3_EXPORT_BITRATE = "64k";
export const GB_FRAME_RATE_NUMERATOR = 4_194_304;
export const GB_FRAME_RATE_DENOMINATOR = 70_224;
export const NOISE_CLOCK_HZ = 524_288;

export const MAX_WAVE_AMPLITUDE = 28_000;

export const FRAME_TO_SAMPLE_NUMERATOR = SAMPLE_RATE * GB_FRAME_RATE_DENOMINATOR;
export const FRAME_TO_SAMPLE_DENOMINATOR = GB_FRAME_RATE_NUMERATOR;

export const FRAME_SEQUENCER_RATE = 512;
export const TICK_TO_FRAME_NUMERATOR = GB_FRAME_RATE_NUMERATOR;
export const TICK_TO_FRAME_DENOMINATOR = GB_FRAME_RATE_DENOMINATOR * FRAME_SEQUENCER_RATE;

export const NOTE_FREQUENCIES: Record<string, number> = {
  C_: 16.35,
  "C#": 17.32,
  D_: 18.35,
  "D#": 19.45,
  E_: 20.6,
  F_: 21.83,
  "F#": 23.12,
  G_: 24.5,
  "G#": 25.96,
  A_: 27.5,
  "A#": 29.14,
  B_: 30.87,
};

export const NOTE_ORDER = Object.keys(NOTE_FREQUENCIES);
export const NOTE_INDEX = Object.fromEntries(NOTE_ORDER.map((name, i) => [name, i])) as Record<string, number>;
export const WAVE_NOTE_FREQUENCY_SCALAR = 0.5;

export const FREQUENCY_TABLE = [
  0x0000, 0xf82c, 0xf89d, 0xf907, 0xf96b, 0xf9ca, 0xfa23, 0xfa77, 0xfac7, 0xfb12, 0xfb58, 0xfb9b, 0xfbda, 0xfc16,
  0xfc4e, 0xfc83, 0xfcb5, 0xfce5, 0xfd11, 0xfd3b, 0xfd63, 0xfd89, 0xfdac, 0xfdcd, 0xfded,
];

export const CHANNEL3_DEFAULT_INSTRUMENT = 4;
export const CHANNEL3_HIGH_INSTRUMENT = 5;
export const CHANNEL3_HIGH_OCTAVE = 6;
export const CHANNEL3_HIGH_NOTE_FREQ_THRESHOLD = 950.0;
export const PULSE_INSTRUMENT_DUTY_CYCLES: Record<number, number> = { 5: 2 };
export const PULSE_INSTRUMENT_PITCH_OFFSETS: Record<number, number> = {
  7: Math.round((Math.pow(2, 2 / 12) - 1.0) * 1000.0),
};

export const SOUND_LOOP_INFINITE_REPEAT_LIMIT = 1_000_000;
export const MAX_COMMANDS_PER_CHANNEL = SOUND_LOOP_INFINITE_REPEAT_LIMIT * 2 + 2;
export const MAX_CHANNEL_DURATION_SECONDS = 900;
export const DEFAULT_LOOPED_MUSIC_EXPORT_SECONDS = 96.0;
export const DEFAULT_TEMPO = 0x0100;

export const DMG_HPF_DECAY = 0.999958;
export const CENTER_PAN_COMPENSATION = 1.0;
export const PAN_CROSSFEED = 0.0;

export const CHANNEL_GAINS: Record<number, number> = {};
export const ENHANCED_CHANNEL_GAINS: Record<number, number> = { 3: 1.0, 7: 1.0 };
export const ENHANCED_MASTER_GAIN = 2.0;

export const GB_DUTY_PATTERNS: Record<number, number[]> = {
  0: [0, 0, 0, 0, 0, 0, 0, 1],
  1: [1, 0, 0, 0, 0, 0, 0, 1],
  2: [1, 0, 0, 0, 0, 1, 1, 1],
  3: [0, 1, 1, 1, 1, 1, 1, 0],
};

export const MIDI_CONTROLLER_PAN = 10;
export const MIDI_CONTROLLER_VOLUME = 7;
export const MIDI_PROGRAM_PULSE = 80;
export const MIDI_PROGRAM_WAVE = 38;

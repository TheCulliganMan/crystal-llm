export interface NoiseNote {
  length: number;
  volume: number;
  fade: number;
  frequency: number;
}

export interface NoiseFrequency {
  period_num: number;
  period_den: number;
  width_mode: number;
}

export interface ChannelState {
  tempo: number;
  note_length: number;
  duration_modifier: number;
  default_length: number | null;
  volume_envelope: [number, number] | null;
  octave: number;
  duty_cycle: number;
  duty_cycle_loop_enabled: boolean;
  duty_cycle_pattern_byte: number | null;
  transpose_octaves: number;
  transpose_pitches: number;
  wave_instrument: number | null;
  wave_volume: number;
  instrument_id: number | null;
  instrument_pitch_offset: number;
  vibrato_delay: number;
  vibrato_delay_count: number;
  vibrato_extent: number;
  vibrato_rate: number;
  vibrato_rate_counter: number;
  vibrato_extent_up: number;
  vibrato_extent_down: number;
  vibrato_direction_up: boolean;
  vibrato_latched_reg: number;
  pitch_offset: number;
  current_pan: [boolean, boolean];
  pitch_sweep_value: number;
  pitch_sweep_enabled: boolean;
  pitch_sweep_shadow: number;
  pulse1_active: boolean;
  pitch_slide_target: number | null;
  pitch_slide_frames: number;
  noise_sampling_enabled: boolean;
  noise_lfsr: number;
  noise_accumulator: number;
}

export const createChannelState = (): ChannelState => ({
  tempo: 0x0100,
  note_length: 1,
  duration_modifier: 0,
  default_length: null,
  volume_envelope: null,
  octave: 4,
  duty_cycle: 2,
  duty_cycle_loop_enabled: false,
  duty_cycle_pattern_byte: null,
  transpose_octaves: 0,
  transpose_pitches: 0,
  wave_instrument: null,
  wave_volume: 0,
  instrument_id: null,
  instrument_pitch_offset: 0,
  vibrato_delay: 0,
  vibrato_delay_count: 0,
  vibrato_extent: 0,
  vibrato_rate: 0,
  vibrato_rate_counter: 0,
  vibrato_extent_up: 0,
  vibrato_extent_down: 0,
  vibrato_direction_up: false,
  vibrato_latched_reg: 0,
  pitch_offset: 0,
  current_pan: [true, true],
  pitch_sweep_value: 0,
  pitch_sweep_enabled: false,
  pitch_sweep_shadow: 0,
  pulse1_active: true,
  pitch_slide_target: null,
  pitch_slide_frames: 0,
  noise_sampling_enabled: false,
  noise_lfsr: 0x7fff,
  noise_accumulator: 0.0,
});

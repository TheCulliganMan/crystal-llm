import { AnimationSound, AnimationSoundSchema } from './_battle-animation-state';

export const enqueue_sound = (
  queue: AnimationSound[],
  sound_id: string,
  duration: number | null,
  tracks: number | null,
  options: {
    sound_type: string;
    pitch: number | null;
    panning: string | null;
    tracks?: number | null;
    cry_selector?: number | null;
  },
): void => {
  const entry: AnimationSound = AnimationSoundSchema.parse({
    sound_id,
    duration,
    tracks,
    sound_type: options.sound_type,
    pitch: options.pitch,
    panning: options.panning,
    cry_selector: options.cry_selector ?? null,
  });
  queue.push(entry);
};

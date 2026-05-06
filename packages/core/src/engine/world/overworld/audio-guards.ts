export type OverworldAudioEngine = {
  play_sound?: (id: string) => void;
  playSound?: (id: string) => void;
};

export function playOverworldSound(
  audio_engine: OverworldAudioEngine | null | undefined,
  sound_id: string,
  {
    logger,
    context,
  }: { logger?: { debug?: (message: string) => void } | null; context?: string } = {}
): boolean {
  if (!audio_engine || !sound_id) {
    return false;
  }
  const playSound = audio_engine.play_sound ?? audio_engine.playSound;
  if (typeof playSound !== "function") {
    if (logger?.debug) {
      const label = context ? `${context} ` : "";
      logger.debug(`Unable to play ${label}${sound_id}: missing play_sound().`);
    }
    return false;
  }
  try {
    playSound.call(audio_engine, sound_id);
    return true;
  } catch (exc) {
    if (logger?.debug) {
      const label = context ? `${context} ` : "";
      logger.debug(`Unable to play ${label}${sound_id}: ${exc}`);
    }
    return false;
  }
}

type QueueDelayFn = (frames: number, options: { on_complete: () => void; blocking?: boolean }) => boolean;

type OverworldFadeContext = {
  queue_delay?: QueueDelayFn;
  fade_to_white?: (frames: number) => void;
  fade_from_white?: (frames: number) => void;
};

type ScheduleTaskFunction = (schedule: (callback: () => void) => boolean) => void;

type ScriptRunnerFadeContext = {
  _queue_overworld_task?: ScheduleTaskFunction;
};

const FADE_FRAMES = 8;

const queueFadeFrames = (
  overworld: OverworldFadeContext | null,
  runner: ScriptRunnerFadeContext | null,
  frames: number = FADE_FRAMES
): void => {
  if (!overworld || !runner) {
    return;
  }

  const queue_delay = overworld.queue_delay;
  if (typeof queue_delay !== 'function') {
    return;
  }

  const schedule_task = runner._queue_overworld_task;
  if (typeof schedule_task !== 'function') {
    return;
  }

  const schedule = (callback: () => void): boolean => {
    const scheduled = queue_delay.call(overworld, frames, { on_complete: callback, blocking: true });
    return !!scheduled;
  };

  schedule_task(schedule);
};

export const fadeOutToWhite = (
  overworld: OverworldFadeContext | null,
  runner: ScriptRunnerFadeContext | null = null
): void => {
  if (!overworld) {
    return;
  }
  const fadeMethod = overworld.fade_to_white;
  if (typeof fadeMethod === 'function') {
    fadeMethod(FADE_FRAMES);
    queueFadeFrames(overworld, runner, FADE_FRAMES);
  }
};

export const fadeInFromWhite = (
  overworld: OverworldFadeContext | null,
  runner: ScriptRunnerFadeContext | null = null
): void => {
  if (!overworld) {
    return;
  }
  const fadeMethod = overworld.fade_from_white;
  if (typeof fadeMethod === 'function') {
    fadeMethod(FADE_FRAMES);
    queueFadeFrames(overworld, runner, FADE_FRAMES);
  }
};

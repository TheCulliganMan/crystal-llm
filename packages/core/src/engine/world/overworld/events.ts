import { playOverworldSound } from "./audio-guards";

export enum ElevatorPhase {
  IDLE,
  DOOR_CLOSING,
  FADING_OUT,
  TRAVELLING,
  FADING_IN,
  DOOR_OPENING,
}

export type ElevatorOverworld = {
  lock_player_movement?: () => void;
  unlock_player_movement?: () => void;
  fade_to_black?: (frames: number) => void;
  fade_from_black?: (frames: number) => void;
  start_earthquake?: (intensity: number, frames: number) => void;
  audio_engine?: {
    play_sound: (id: string) => void;
    playSound?: (id: string) => void;
  } | null;
  script_runner?: {
    last_sound_effect?: string | null;
  } | null;
};

export class ElevatorRideStateMachine {
  door_close_frames = 10;
  door_open_frames = 10;
  fade_frames = 8;
  travel_frames = 60;
  play_sound = true;
  start_earthquake = true;
  phase = ElevatorPhase.IDLE;
  remaining_frames = 0;
  origin: string | null = null;
  destination: string | null = null;
  private _sound_triggered = false;

  start(
    overworld: ElevatorOverworld,
    {
      origin = null,
      destination = null,
    }: { origin?: string | null; destination?: string | null } = {}
  ): void {
    this.origin = origin;
    this.destination = destination;
    this.phase = ElevatorPhase.DOOR_CLOSING;
    this.remaining_frames = Math.max(0, Math.trunc(this.door_close_frames));
    this._sound_triggered = false;
    overworld.lock_player_movement?.();
  }

  get active(): boolean {
    return this.phase !== ElevatorPhase.IDLE;
  }

  update(overworld: ElevatorOverworld): void {
    if (!this.active) {
      return;
    }

    if (this.remaining_frames > 0) {
      this.remaining_frames -= 1;
      return;
    }

    switch (this.phase) {
      case ElevatorPhase.DOOR_CLOSING:
        this._begin_fade_to_black(overworld);
        return;
      case ElevatorPhase.FADING_OUT:
        this._begin_travel(overworld);
        return;
      case ElevatorPhase.TRAVELLING:
        this._begin_fade_in(overworld);
        return;
      case ElevatorPhase.FADING_IN:
        this._begin_door_open(overworld);
        return;
      case ElevatorPhase.DOOR_OPENING:
        this._finish(overworld);
        return;
      default:
        return;
    }
  }

  private _begin_fade_to_black(overworld: ElevatorOverworld): void {
    this.phase = ElevatorPhase.FADING_OUT;
    this.remaining_frames = Math.max(0, Math.trunc(this.fade_frames));
    overworld.fade_to_black?.(this.fade_frames);
  }

  private _begin_travel(overworld: ElevatorOverworld): void {
    this.phase = ElevatorPhase.TRAVELLING;
    this.remaining_frames = Math.max(0, Math.trunc(this.travel_frames));
    if (this.start_earthquake) {
      overworld.start_earthquake?.(1, Math.max(1, Math.trunc(this.travel_frames)));
    }
    if (this.play_sound && !this._sound_triggered) {
      this._trigger_sound(overworld);
    }
  }

  private _begin_fade_in(overworld: ElevatorOverworld): void {
    this.phase = ElevatorPhase.FADING_IN;
    this.remaining_frames = Math.max(0, Math.trunc(this.fade_frames));
    overworld.fade_from_black?.(this.fade_frames);
  }

  private _begin_door_open(overworld: ElevatorOverworld): void {
    this.phase = ElevatorPhase.DOOR_OPENING;
    this.remaining_frames = Math.max(0, Math.trunc(this.door_open_frames));
  }

  private _finish(overworld: ElevatorOverworld): void {
    this.phase = ElevatorPhase.IDLE;
    this.remaining_frames = 0;
    this._sound_triggered = false;
    overworld.unlock_player_movement?.();
  }

  private _trigger_sound(overworld: ElevatorOverworld): void {
    const audioEngine = overworld.audio_engine;
    if (!audioEngine) {
      return;
    }
    if (!playOverworldSound(audioEngine, "SFX_ELEVATOR")) {
      return;
    }
    this._sound_triggered = true;
    if (overworld.script_runner) {
      try {
        overworld.script_runner.last_sound_effect = "SFX_ELEVATOR";
      } catch {
        return;
      }
    }
  }
}

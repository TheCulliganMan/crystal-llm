
import { GameState } from '@pokecrystal/core/core/state';
import { RADIO_CHANNEL_CONSTANTS, RadioChannelConstant } from '@pokecrystal/assets/content/radio';
import { Event, EventManager } from '../events/events';
import { normalizeRadioStationToken, resolveRadioStationSong } from './radio-music';

type RadioChannelEventData = {
  station: string;
  duration_frames?: number;
  channel?: string;
  source?: string;
};

export class RadioEventController {
  private readonly eventManager: EventManager;
  private readonly audioController: {
    startRadioChannel?: (station: string, durationFrames?: number) => void;
    stopRadioChannel?: () => void;
    playMusic?: (song: string, role?: string) => void;
    restartMapMusic?: () => void;
  };
  private readonly stationInfo: Map<string, RadioChannelConstant>;
  private registered = false;

  constructor({
    eventManager,
    audioEngine,
  }: {
    eventManager: EventManager;
    audioEngine: {
      startRadioChannel?: (station: string, durationFrames?: number) => void;
      stopRadioChannel?: () => void;
      playMusic?: (song: string, role?: string) => void;
      restartMapMusic?: () => void;
    };
  }) {
    this.eventManager = eventManager;
    this.audioController = audioEngine;
    this.stationInfo = new Map(RADIO_CHANNEL_CONSTANTS.map(entry => [entry.constant, entry]));
  }

  public register(): void {
    if (this.registered) {
      return;
    }

    this.eventManager.on('play_radio_channel', this.handlePlayRadioChannel.bind(this));
    this.eventManager.on('stop_radio_channel', this.handleStopRadioChannel.bind(this));
    this.registered = true;
  }

  private handlePlayRadioChannel(event: Event<RadioChannelEventData>, _: GameState): void {
    const station = event.data?.station;
    if (!station) {
      throw new Error('play_radio_channel requires a station name');
    }

    const stationToken = normalizeRadioStationToken(station);
    const stationInfo = this.stationInfo.get(stationToken);

    if (!stationInfo) {
      throw new Error(`Missing radio station '${stationToken}'.`);
    }

    const resolved = resolveRadioStationSong(stationToken, _);
    if (!stationInfo.song || !resolved) {
      throw new Error(`Radio station '${stationToken}' is missing a song mapping.`);
    }

    const durationFrames = event.data?.duration_frames ?? 0;
    const frames = Math.max(0, durationFrames);
    try {
      if (typeof this.audioController.startRadioChannel === "function") {
        this.audioController.startRadioChannel(stationToken, frames);
      } else {
        this.audioController.playMusic?.(resolved.song, 'general');
      }
    } catch (exc) {
      const message = exc instanceof Error ? exc.message : String(exc);
      throw new Error(`Failed to start radio station '${stationToken}': ${message}`);
    }
  }

  private handleStopRadioChannel(event: Event<RadioChannelEventData>, _: GameState): void {
    void event;
    if (typeof this.audioController.stopRadioChannel === "function") {
      this.audioController.stopRadioChannel();
      return;
    }
    this.audioController.restartMapMusic?.();
  }
}

import type { GameState } from "@pokecrystal/core/core/state";
import { PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { defaultMusicTokenForMap } from "@pokecrystal/core/engine/world/map-music";
import { determineTrainerEncounterMusic } from "@pokecrystal/core/engine/battle/battle/music";
import {
  normalizeRadioStationToken,
  resolveRadioStationFromSong,
  resolveRadioStationSong,
} from "@pokecrystal/core/engine/world/radio-music";

type AudioTransport = {
  update?: () => void;
  playMusic?: (name: string, role?: string) => void;
  play_music?: (name: string, role?: string | { role?: string }) => void;
  stopMusic?: () => void;
  stop_music?: () => void;
  playSound?: (name: string) => void;
  play_sound?: (name: string) => void;
  isSoundPlaying?: (name?: string) => boolean;
  is_sound_playing?: (name?: string) => boolean;
  fadeOutMusicFrames?: (durationFrames: number) => void;
  fadeToMusicFrames?: (name: string, durationFrames: number, role?: string) => void;
  setMusicMutedByController?: (muted: boolean) => void;
  canResolveMusicToken?: (name: string) => boolean;
  canResolveSoundToken?: (name: string) => boolean;
};

type RequestedMusic = {
  token: string;
  role: string;
};

const playTransportMusic = (transport: AudioTransport, token: string, role: string): void => {
  if (typeof transport.playMusic === "function") {
    transport.playMusic(token, role);
    return;
  }
  transport.play_music?.(token, { role });
};

const stopTransportMusic = (transport: AudioTransport): void => {
  if (typeof transport.stopMusic === "function") {
    transport.stopMusic();
    return;
  }
  transport.stop_music?.();
};

const playTransportSound = (transport: AudioTransport, token: string): void => {
  if (typeof transport.playSound === "function") {
    transport.playSound(token);
    return;
  }
  transport.play_sound?.(token);
};

const resolveMapMusicToken = (mapName: string, playerState: PlayerState): string => {
  let token = defaultMusicTokenForMap(mapName);
  // ASM mapping: home/audio.asm::{PlayMapMusicBike,SpecialMapMusic}
  if (playerState === PlayerState.BIKE) {
    token = "MUSIC_BICYCLE";
  } else if (playerState === PlayerState.SURF || playerState === PlayerState.SURF_PIKA) {
    token = "MUSIC_SURF";
  }
  return token;
};

const normalizeRole = (role: string | { role?: string } | undefined, fallback: string): string => {
  if (typeof role === "string") {
    const trimmed = role.trim();
    return trimmed || fallback;
  }
  const trimmed = role?.role?.trim();
  return trimmed || fallback;
};

export class OverworldAudioController {
  private requestedMapMusic: RequestedMusic | null = null;
  private requestedScriptMusic: RequestedMusic | null = null;
  private encounterMusic: RequestedMusic | null = null;
  private activeRadio: { station: string; song: string; remainingFrames: number } | null = null;
  private engineManagedFade: RequestedMusic | null = null;
  private renderedMusic: RequestedMusic | null = null;

  constructor(
    private readonly gameState: GameState,
    private readonly transport: AudioTransport,
  ) {
    this.restoreSavedRadioMusic();
  }

  public requestMapMusic(mapName: string, playerState: PlayerState): void {
    const token = resolveMapMusicToken(mapName, playerState);
    this.assertMusicToken(token);
    this.requestedMapMusic = { token, role: "map" };
  }

  public requestMusic(token: string, role: string = "general"): void {
    const trimmed = String(token ?? "").trim();
    if (!trimmed) {
      throw new Error("Audio controller cannot queue empty music.");
    }
    if (!this.isSilenceToken(trimmed)) {
      this.assertMusicToken(trimmed);
    }
    this.requestedScriptMusic = { token: trimmed, role };
  }

  public requestEncounterMusic(trainerClass: string): void {
    const token = determineTrainerEncounterMusic(trainerClass);
    this.assertMusicToken(token);
    this.encounterMusic = { token, role: "encounter" };
  }

  public startRadioChannel(station: string, durationFrames: number = 0): void {
    const stationToken = normalizeRadioStationToken(station);
    if (!stationToken) {
      throw new Error("Radio station is required.");
    }
    const resolved = resolveRadioStationSong(stationToken, this.gameState);
    if (!resolved) {
      throw new Error(`Radio station '${stationToken}' is missing a song mapping.`);
    }
    this.assertMusicToken(resolved.song);
    this.activeRadio = {
      station: resolved.station,
      song: resolved.song,
      remainingFrames: Math.max(0, Math.trunc(durationFrames)),
    };
    this.gameState.wram.wMapMusic = resolved.song;
  }

  public stopRadioChannel(): void {
    this.activeRadio = null;
  }

  public playCry(cryId: string): void {
    const token = String(cryId ?? "").trim();
    if (!token) {
      throw new Error("Cry token is required.");
    }
    this.assertSoundToken(token);
    playTransportSound(this.transport, token);
  }

  public fadeToMusic(targetToken: string, durationFrames: number, role: string = "general"): void {
    const token = String(targetToken ?? "").trim();
    if (!token) {
      throw new Error("Fade target token is required.");
    }
    if (!this.isSilenceToken(token)) {
      this.assertMusicToken(token);
    }
    const frames = Math.max(0, Math.trunc(durationFrames));
    if (frames === 0) {
      this.engineManagedFade = null;
      this.requestMusic(token, role);
      return;
    }
    if (typeof this.transport.fadeToMusicFrames === "function") {
      this.transport.fadeToMusicFrames(token, frames, role);
      this.engineManagedFade = { token, role };
      return;
    }
    this.transport.fadeOutMusicFrames?.(frames);
    this.requestedScriptMusic = { token, role };
  }

  public restartMapMusic(): void {
    // ASM mapping: engine/overworld/scripting.asm::Script_dontrestartmapmusic
    if (this.gameState.wram.dont_restart_map_music) {
      this.gameState.wram.dont_restart_map_music = false;
      return;
    }
    this.encounterMusic = null;
    this.requestedScriptMusic = null;
    this.engineManagedFade = null;
    this.renderedMusic = null;
  }

  public hasTemporaryMusicOverride(): boolean {
    return (
      this.requestedScriptMusic !== null ||
      this.encounterMusic !== null ||
      this.engineManagedFade !== null
    );
  }

  public update(): void {
    this.transport.update?.();
    this.tickRadioTimer();
    this.applyResolvedMusic();
  }

  private tickRadioTimer(): void {
    if (!this.activeRadio || this.activeRadio.remainingFrames <= 0) {
      return;
    }
    this.activeRadio.remainingFrames -= 1;
    if (this.activeRadio.remainingFrames <= 0) {
      this.activeRadio = null;
    }
  }

  private restoreSavedRadioMusic(): void {
    const resolved = resolveRadioStationFromSong(this.gameState.wram.wMapMusic, this.gameState);
    if (!resolved) {
      return;
    }
    this.activeRadio = {
      station: resolved.station,
      song: resolved.song,
      remainingFrames: 0,
    };
  }

  private applyResolvedMusic(): void {
    if (
      this.engineManagedFade &&
      this.renderedMusic &&
      this.renderedMusic.token === this.engineManagedFade.token &&
      this.renderedMusic.role === this.engineManagedFade.role
    ) {
      this.engineManagedFade = null;
    }
    const resolved = this.resolveDesiredMusic();
    if (!resolved) {
      if (this.renderedMusic) {
        stopTransportMusic(this.transport);
        this.renderedMusic = null;
      }
      return;
    }
    if (
      this.renderedMusic &&
      this.renderedMusic.token === resolved.token &&
      this.renderedMusic.role === resolved.role
    ) {
      if (resolved.role === "map" || resolved.role === "radio") {
        this.gameState.wram.wMapMusic = resolved.token;
      }
      return;
    }
    if (this.isSilenceToken(resolved.token)) {
      stopTransportMusic(this.transport);
      this.renderedMusic = null;
      return;
    }
    playTransportMusic(this.transport, resolved.token, resolved.role);
    this.renderedMusic = resolved;
    if (resolved.role === "map" || resolved.role === "radio") {
      this.gameState.wram.wMapMusic = resolved.token;
    }
  }

  private resolveDesiredMusic(): RequestedMusic | null {
    if (this.engineManagedFade) {
      return this.renderedMusic;
    }
    if (this.activeRadio) {
      return { token: this.activeRadio.song, role: "radio" };
    }
    if (this.requestedScriptMusic) {
      return this.requestedScriptMusic;
    }
    if (this.encounterMusic) {
      return this.encounterMusic;
    }
    return this.requestedMapMusic;
  }

  private isSilenceToken(token: string): boolean {
    return token.trim().toUpperCase() === "MUSIC_NONE";
  }

  private assertMusicToken(token: string): void {
    if (this.transport.canResolveMusicToken?.(token) === false) {
      throw new Error(`Missing music asset for '${token}'.`);
    }
  }

  private assertSoundToken(token: string): void {
    if (this.transport.canResolveSoundToken?.(token) === false) {
      throw new Error(`Missing sound asset for '${token}'.`);
    }
  }
}

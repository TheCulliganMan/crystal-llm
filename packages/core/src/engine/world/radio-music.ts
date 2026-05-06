import type { GameState } from "@pokecrystal/core/core/state";
import { isInJohto } from "@pokecrystal/core/core/home";
import { RADIO_CHANNEL_CONSTANTS } from "@pokecrystal/assets/content/radio";

const RADIO_SONGS = new Map(RADIO_CHANNEL_CONSTANTS.map((entry) => [entry.constant, entry.song]));
const RADIO_STATIONS_BY_SONG = new Map(
  RADIO_CHANNEL_CONSTANTS.map((entry) => [entry.song, entry.constant]),
);
const POKEMON_MUSIC_STATIONS = new Set(["POKEMON_MUSIC", "LETS_ALL_SING"]);

export type ResolvedRadioStation = {
  station: string;
  song: string;
};

export const normalizeRadioStationToken = (station: string): string => {
  return String(station ?? "").trim().toUpperCase();
};

export const shouldRocketRadioOverride = (gameState: GameState): boolean => {
  return Boolean(gameState.wram.engine_flags["ENGINE_ROCKETS_IN_RADIO_TOWER"]) && isInJohto(gameState) === 0;
};

export const resolveRadioStationSong = (station: string, gameState: GameState): ResolvedRadioStation | null => {
  let stationToken = normalizeRadioStationToken(station);
  if (!stationToken) {
    return null;
  }
  if (shouldRocketRadioOverride(gameState)) {
    stationToken = "ROCKET_RADIO";
  }
  if (POKEMON_MUSIC_STATIONS.has(stationToken)) {
    const weekday = Math.max(0, Math.trunc(gameState.sram.day_of_week ?? 0));
    return {
      station: stationToken,
      song: weekday % 2 === 0 ? "MUSIC_POKEMON_MARCH" : "MUSIC_POKEMON_LULLABY",
    };
  }
  const song = RADIO_SONGS.get(stationToken);
  return song ? { station: stationToken, song } : null;
};

export const resolveRadioStationFromSong = (
  song: string,
  gameState: GameState,
): ResolvedRadioStation | null => {
  const songToken = String(song ?? "").trim().toUpperCase();
  const station = RADIO_STATIONS_BY_SONG.get(songToken);
  if (!station) {
    return null;
  }
  const resolved = resolveRadioStationSong(station, gameState);
  return resolved?.song === songToken ? resolved : null;
};

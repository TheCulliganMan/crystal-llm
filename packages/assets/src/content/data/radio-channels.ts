import { z } from 'zod';

export const RadioChannelFrequencySchema = z.object({
  raw: z.number(),
  frequency: z.number(),
  handler: z.string(),
});

export type RadioChannelFrequency = z.infer<typeof RadioChannelFrequencySchema>;

export const RADIO_CHANNEL_FREQUENCIES: RadioChannelFrequency[] = [
  {
    "raw": 16,
    "frequency": 4.50,
    "handler": "PKMNTalkAndPokedexShow",
  },
  {
    "raw": 28,
    "frequency": 7.50,
    "handler": "PokemonMusic",
  },
  {
    "raw": 32,
    "frequency": 8.50,
    "handler": "LuckyChannel",
  },
  {
    "raw": 40,
    "frequency": 10.50,
    "handler": "BuenasPassword",
  },
  {
    "raw": 52,
    "frequency": 13.50,
    "handler": "RuinsOfAlphRadio",
  },
  {
    "raw": 64,
    "frequency": 16.50,
    "handler": "PlacesAndPeople",
  },
  {
    "raw": 72,
    "frequency": 18.50,
    "handler": "LetsAllSing",
  },
  {
    "raw": 78,
    "frequency": 20.00,
    "handler": "PokeFluteRadio",
  },
  {
    "raw": 80,
    "frequency": 20.50,
    "handler": "EvolutionRadio",
  },
];

export const RadioChannelConstantSchema = z.object({
  constant: z.string(),
  id: z.number(),
  song: z.string(),
});

export type RadioChannelConstant = z.infer<typeof RadioChannelConstantSchema>;

export const RADIO_CHANNEL_CONSTANTS: RadioChannelConstant[] = [
  {
    "constant": "OAKS_POKEMON_TALK",
    "id": 0,
    "song": "MUSIC_POKEMON_TALK",
  },
  {
    "constant": "POKEDEX_SHOW",
    "id": 1,
    "song": "MUSIC_POKEMON_CENTER",
  },
  {
    "constant": "POKEMON_MUSIC",
    "id": 2,
    "song": "MUSIC_TITLE",
  },
  {
    "constant": "LUCKY_CHANNEL",
    "id": 3,
    "song": "MUSIC_GAME_CORNER",
  },
  {
    "constant": "BUENAS_PASSWORD",
    "id": 4,
    "song": "MUSIC_BUENAS_PASSWORD",
  },
  {
    "constant": "PLACES_AND_PEOPLE",
    "id": 5,
    "song": "MUSIC_VIRIDIAN_CITY",
  },
  {
    "constant": "LETS_ALL_SING",
    "id": 6,
    "song": "MUSIC_BICYCLE",
  },
  {
    "constant": "ROCKET_RADIO",
    "id": 7,
    "song": "MUSIC_ROCKET_OVERTURE",
  },
  {
    "constant": "POKE_FLUTE_RADIO",
    "id": 8,
    "song": "MUSIC_POKE_FLUTE_CHANNEL",
  },
  {
    "constant": "UNOWN_RADIO",
    "id": 9,
    "song": "MUSIC_RUINS_OF_ALPH_RADIO",
  },
  {
    "constant": "EVOLUTION_RADIO",
    "id": 10,
    "song": "MUSIC_LAKE_OF_RAGE_ROCKET_RADIO",
  },
];

export const RADIO_STATION_NAMES: Record<string, string> = {
  "BuenasPasswordName": "BUENA'S PASSWORD",
  "LetsAllSingName": "Let's All Sing!",
  "LuckyChannelName": "Lucky Channel",
  "NotBuenasPasswordName": "",
  "OaksPKMNTalkName": "OAK's Poké Talk",
  "PlacesAndPeopleName": "Places & People",
  "PokeFluteStationName": "Poké FLUTE",
  "PokedexShowName": "PokéDEX Show",
  "PokemonMusicName": "PokéMON Music",
  "UnownStationName": "?????",
};

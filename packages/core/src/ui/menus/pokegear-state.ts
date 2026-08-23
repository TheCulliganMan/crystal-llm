// ASM: engine/pokegear/pokegear.asm state machine and radio helpers.
import { z } from "zod";
import { GameState } from "../../core/state";
import { LANDMARK_SPECIAL } from "../../core/constants";
import { getWorldMapLocation } from "../../core/home";
import { getMapMetadataByGroup } from "../../engine/world/maps";
import {
  getPokegearLandmarksSync,
  LandmarkEntry,
} from "@pokecrystal/assets/content/pokegear";
import {
  RADIO_CHANNEL_CONSTANTS,
  RADIO_CHANNEL_FREQUENCIES,
  RADIO_STATION_NAMES,
} from "@pokecrystal/assets/content/radio";
import { loadPhoneContactDirectory } from "./pokegear-contacts";
import { resolveRadioStationSong, shouldRocketRadioOverride } from "../../engine/world/radio-music";

export const PHONE_LIST_LENGTH = 4;

const RadioFrequencyEntrySchema = z.object({
  raw: z.number(),
  frequency: z.number(),
  handler: z.string(),
});

const RadioChannelConstantEntrySchema = z.object({
  constant: z.string(),
  id: z.number(),
  song: z.string(),
});

type RadioFrequencyEntry = z.infer<typeof RadioFrequencyEntrySchema>;
type RadioChannelConstantEntry = z.infer<typeof RadioChannelConstantEntrySchema>;

const LANDMARK_DATA = getPokegearLandmarksSync();
const RADIO_FREQUENCY_DATA: RadioFrequencyEntry[] = RADIO_CHANNEL_FREQUENCIES.map((entry) =>
  RadioFrequencyEntrySchema.parse(entry),
);
const RADIO_CONSTANT_DATA: RadioChannelConstantEntry[] = RADIO_CHANNEL_CONSTANTS.map((entry) =>
  RadioChannelConstantEntrySchema.parse(entry),
);
const RADIO_NAME_MAP = RADIO_STATION_NAMES;

const LANDMARK_ENTRY_BY_ID = new Map<number, LandmarkEntry>(
  LANDMARK_DATA.map((entry) => [entry.id, entry]),
);
const LANDMARK_ID_BY_CONSTANT = new Map<string, number>(
  LANDMARK_DATA.map((entry) => [entry.constant, entry.id]),
);
const CHANNEL_INFO_BY_CONSTANT = new Map<string, RadioChannelConstantEntry>(
  RADIO_CONSTANT_DATA.map((entry) => [entry.constant, entry]),
);

const CHANNEL_LABELS: Record<string, string> = {
  OAKS_POKEMON_TALK: "OaksPKMNTalkName",
  POKEDEX_SHOW: "PokedexShowName",
  POKEMON_MUSIC: "PokemonMusicName",
  LUCKY_CHANNEL: "LuckyChannelName",
  BUENAS_PASSWORD: "BuenasPasswordName",
  PLACES_AND_PEOPLE: "PlacesAndPeopleName",
  LETS_ALL_SING: "LetsAllSingName",
  POKE_FLUTE_RADIO: "PokeFluteStationName",
  UNOWN_RADIO: "UnownStationName",
  EVOLUTION_RADIO: "UnownStationName",
  ROCKET_RADIO: "LuckyChannelName",
};

const EVOLUTION_LANDMARKS = new Set([
  LANDMARK_ID_BY_CONSTANT.get("LANDMARK_MAHOGANY_TOWN"),
  LANDMARK_ID_BY_CONSTANT.get("LANDMARK_ROUTE_43"),
  LANDMARK_ID_BY_CONSTANT.get("LANDMARK_LAKE_OF_RAGE"),
].filter((value): value is number => typeof value === "number"));

const FAST_SHIP_LANDMARK =
  LANDMARK_ID_BY_CONSTANT.get("LANDMARK_FAST_SHIP") ?? LANDMARK_SPECIAL;

export enum PokegearCard {
  CLOCK = 0,
  MAP = 1,
  PHONE = 2,
  RADIO = 3,
}

export enum PokegearState {
  CLOCK_INIT = 0,
  CLOCK_JOYPAD = 1,
  MAP_CHECK_REGION = 2,
  JOHTO_MAP_INIT = 3,
  JOHTO_MAP_JOYPAD = 4,
  KANTO_MAP_INIT = 5,
  KANTO_MAP_JOYPAD = 6,
  PHONE_INIT = 7,
  PHONE_JOYPAD = 8,
  MAKE_PHONE_CALL = 9,
  FINISH_PHONE_CALL = 10,
  RADIO_INIT = 11,
  RADIO_JOYPAD = 12,
}

export class RadioStation {
  constructor(
    public readonly constant: string,
    public readonly identifier: number,
    public readonly song: string,
    public readonly name: string,
    public readonly frequency: number,
  ) {}
}

export type PokegearScriptRunner = {
  queuePhoneCall?: (contact: string) => void;
  consumePhoneCall?: (contact: string) => { contact?: string } | void;
  runPhoneScript?: (script: string) => void;
};

export class PokegearStateMachine {
  public availableCards: PokegearCard[] = [];
  public currentCard: PokegearCard = PokegearCard.CLOCK;
  public state: PokegearState = PokegearState.CLOCK_INIT;
  public playerLandmarkId: number = LANDMARK_SPECIAL;
  public cursorLandmarkId: number = LANDMARK_SPECIAL;
  public mapRegion = "SPECIAL";

  private mapLandmarkIds: number[] = [];
  private mapIndex = 0;
  private radioIndex = 0;

  constructor(
    private readonly gameState: GameState,
    private readonly scriptRunner?: PokegearScriptRunner,
  ) {
    this.refresh();
  }

  refresh(): void {
    this.refreshCards();
    this.normalizePhoneNumbers();
    this.updatePlayerLandmark();
    this.ensureMapEntries();
    this.clampPhoneCursor();
    this.clampRadioIndex();
  }

  switchCard(direction: number): PokegearCard {
    if (this.availableCards.length === 0) {
      this.availableCards = [PokegearCard.CLOCK];
      this.currentCard = PokegearCard.CLOCK;
    }
    const index = this.availableCards.indexOf(this.currentCard);
    const nextIndex = (index + direction + this.availableCards.length) % this.availableCards.length;
    this.currentCard = this.availableCards[nextIndex];
    this.gameState.wram.pokegear_card = this.currentCard;
    this.state = {
      [PokegearCard.CLOCK]: PokegearState.CLOCK_JOYPAD,
      [PokegearCard.MAP]: PokegearState.MAP_CHECK_REGION,
      [PokegearCard.PHONE]: PokegearState.PHONE_JOYPAD,
      [PokegearCard.RADIO]: PokegearState.RADIO_JOYPAD,
    }[this.currentCard];
    return this.currentCard;
  }

  forceCard(card: PokegearCard): void {
    if (this.availableCards.includes(card)) {
      this.currentCard = card;
      this.gameState.wram.pokegear_card = card;
      this.switchCard(0);
    }
  }

  get mapEntries(): LandmarkEntry[] {
    return this.mapLandmarkIds.map((landmarkId) => LANDMARK_ENTRY_BY_ID.get(landmarkId)!);
  }

  get mapCursorEntry(): LandmarkEntry {
    return LANDMARK_ENTRY_BY_ID.get(this.cursorLandmarkId)!;
  }

  get mapPlayerEntry(): LandmarkEntry {
    return LANDMARK_ENTRY_BY_ID.get(this.playerLandmarkId) ?? LANDMARK_ENTRY_BY_ID.get(0)!;
  }

  phoneServiceAvailable(): boolean {
    const metadata = getMapMetadataByGroup(this.gameState.wram.wMapGroup, this.gameState.wram.wMapNumber);
    if (!metadata) {
      return true;
    }
    const phoneService = ((metadata.phoneService ?? 0) & 0xf0) >> 4;
    return phoneService === 0;
  }

  moveMapCursor(offset: number): void {
    if (this.mapLandmarkIds.length === 0) {
      return;
    }
    this.mapIndex = (this.mapIndex + offset + this.mapLandmarkIds.length) % this.mapLandmarkIds.length;
    this.cursorLandmarkId = this.mapLandmarkIds[this.mapIndex];
    this.gameState.wram.pokegear_map_cursor_landmark = this.cursorLandmarkId;
  }

  resetMapCursorToPlayer(): void {
    const index = this.mapLandmarkIds.indexOf(this.playerLandmarkId);
    if (index < 0) {
      return;
    }
    this.mapIndex = index;
    this.cursorLandmarkId = this.playerLandmarkId;
    this.gameState.wram.pokegear_map_cursor_landmark = this.cursorLandmarkId;
  }

  get phoneNumbers(): string[] {
    return [...this.gameState.sram.phone_numbers];
  }

  get phoneCursor(): number {
    return this.gameState.wram.pokegear_phone_cursor_position;
  }

  get phoneScroll(): number {
    return this.gameState.wram.pokegear_phone_scroll_position;
  }

  movePhoneCursor(offset: number): void {
    const numbers = this.phoneNumbers;
    if (numbers.length === 0) {
      this.gameState.wram.pokegear_phone_cursor_position = 0;
      this.gameState.wram.pokegear_phone_scroll_position = 0;
      return;
    }
    const total = numbers.length;
    const cursor = (this.gameState.wram.pokegear_phone_cursor_position + offset + total) % total;
    this.gameState.wram.pokegear_phone_cursor_position = cursor;
    this.clampPhoneCursor();
  }

  beginPhoneCall(): string | null {
    const numbers = this.phoneNumbers;
    if (numbers.length === 0) {
      return null;
    }
    let cursor = this.gameState.wram.pokegear_phone_cursor_position;
    cursor = Math.max(0, Math.min(cursor, numbers.length - 1));
    const contact = numbers[cursor];
    const queue = this.gameState.wram.scheduled_phone_calls;
    if (!queue.includes(contact)) {
      queue.push(contact);
    }
    this.scriptRunner?.queuePhoneCall?.(contact);
    return contact;
  }

  consumePhoneCall(): string | null {
    const queue = this.gameState.wram.scheduled_phone_calls;
    if (queue.length === 0) {
      return null;
    }
    const contact = queue.shift()!;
    const result = this.scriptRunner?.consumePhoneCall?.(contact);
    if (result && typeof result === "object" && "contact" in result) {
      return result.contact ?? contact;
    }
    return contact;
  }

  hasScriptRunner(): boolean {
    return this.scriptRunner !== undefined;
  }

  runPhoneScript(scriptName: string): void {
    this.scriptRunner?.runPhoneScript?.(scriptName);
  }

  get radioFrequency(): RadioFrequencyEntry {
    return RADIO_FREQUENCY_DATA[this.radioIndex];
  }

  setRadioIndex(index: number): number {
    const clamped = Math.max(0, Math.min(index, RADIO_FREQUENCY_DATA.length - 1));
    this.radioIndex = clamped;
    const rawValue = RADIO_FREQUENCY_DATA[clamped].raw;
    this.gameState.wram.pokegear_radio_frequency_raw = rawValue;
    return clamped;
  }

  tuneRadio(step: number): number {
    return this.setRadioIndex(this.radioIndex + step);
  }

  currentRadioStation(): RadioStation | null {
    const entry = this.radioFrequency;
    const constant = this.resolveStation(entry.handler);
    if (!constant) {
      return null;
    }
    const info = CHANNEL_INFO_BY_CONSTANT.get(constant);
    if (!info) {
      throw new Error(`Missing radio station channel info for '${constant}'.`);
    }
    const label = CHANNEL_LABELS[constant];
    if (!label) {
      throw new Error(`Missing radio station label for '${constant}'.`);
    }
    const name = RADIO_NAME_MAP[label];
    if (name === undefined) {
      throw new Error(`Missing radio station name mapping for label '${label}'.`);
    }
    const resolved = resolveRadioStationSong(constant, this.gameState);
    if (!resolved) {
      throw new Error(`Radio station '${constant}' is missing a song mapping.`);
    }
    return new RadioStation(resolved.station, info.id, resolved.song, name, entry.frequency);
  }

  private refreshCards(): void {
    const flags = this.gameState.wram.engine_flags;
    const cards: PokegearCard[] = [PokegearCard.CLOCK];
    if (flags["ENGINE_POKEGEAR"]) {
      if (flags["ENGINE_MAP_CARD"]) {
        cards.push(PokegearCard.MAP);
      }
      if (flags["ENGINE_PHONE_CARD"]) {
        cards.push(PokegearCard.PHONE);
      }
      if (flags["ENGINE_RADIO_CARD"]) {
        cards.push(PokegearCard.RADIO);
      }
    }
    this.availableCards = cards;
    if (!cards.includes(this.currentCard)) {
      this.currentCard = cards[0];
    }
    this.gameState.wram.pokegear_card = this.currentCard;
  }

  private normalizePhoneNumbers(): void {
    const directory = loadPhoneContactDirectory();
    const numbers = this.gameState.sram.phone_numbers;
    for (let index = 0; index < numbers.length; index += 1) {
      const entry = numbers[index];
      const resolved = directory.resolveContactId(entry ?? "");
      if (!resolved) {
        throw new Error(`Unknown phone contact '${entry ?? ""}' in SRAM phone list.`);
      }
      if (resolved !== entry) {
        numbers[index] = resolved;
      }
    }
  }

  private updatePlayerLandmark(): void {
    let group = this.gameState.wram.wMapGroup;
    let mapId = this.gameState.wram.wMapNumber;
    let landmark = getWorldMapLocation(group, mapId);
    if (landmark === LANDMARK_SPECIAL) {
      group = this.gameState.wram.wBackupMapGroup;
      mapId = this.gameState.wram.wBackupMapNumber;
      landmark = getWorldMapLocation(group, mapId);
    }
    this.playerLandmarkId = landmark;
    this.gameState.wram.pokegear_map_player_landmark = landmark;
    const entry = LANDMARK_ENTRY_BY_ID.get(landmark);
    if (!entry) {
      this.mapRegion = "SPECIAL";
    } else {
      const regionValue = String(entry.region ?? "SPECIAL");
      this.mapRegion = regionValue === "OTHER" ? "JOHTO" : regionValue;
    }
    this.gameState.wram.pokegear_map_region = this.mapRegion;
  }

  private ensureMapEntries(): void {
    const regionKey = this.mapRegion === "KANTO" ? "KANTO" : "JOHTO";
    this.mapLandmarkIds = LANDMARK_DATA.filter((entry) => String(entry.region).toUpperCase() === regionKey).map(
      (entry) => entry.id,
    );
    if (this.mapLandmarkIds.length === 0) {
      this.mapLandmarkIds = LANDMARK_DATA.filter((entry) => String(entry.region).toUpperCase() === "JOHTO").map(
        (entry) => entry.id,
      );
    }
    const preferred = this.gameState.wram.pokegear_map_cursor_landmark;
    if (this.mapLandmarkIds.includes(preferred)) {
      this.mapIndex = this.mapLandmarkIds.indexOf(preferred);
    } else if (this.mapLandmarkIds.includes(this.playerLandmarkId)) {
      this.mapIndex = this.mapLandmarkIds.indexOf(this.playerLandmarkId);
    } else {
      this.mapIndex = 0;
    }
    this.cursorLandmarkId = this.mapLandmarkIds[this.mapIndex];
    this.gameState.wram.pokegear_map_cursor_landmark = this.cursorLandmarkId;
  }

  private clampPhoneCursor(): void {
    const numbers = this.phoneNumbers;
    if (numbers.length === 0) {
      this.gameState.wram.pokegear_phone_cursor_position = 0;
      this.gameState.wram.pokegear_phone_scroll_position = 0;
      return;
    }
    const total = numbers.length;
    const cursor = Math.max(0, Math.min(this.gameState.wram.pokegear_phone_cursor_position, total - 1));
    let scroll = Math.max(0, this.gameState.wram.pokegear_phone_scroll_position);
    const maxScroll = Math.max(0, total - PHONE_LIST_LENGTH);
    scroll = Math.min(scroll, maxScroll);
    if (cursor < scroll) {
      scroll = cursor;
    } else if (cursor >= scroll + PHONE_LIST_LENGTH) {
      scroll = Math.max(0, cursor - PHONE_LIST_LENGTH + 1);
    }
    this.gameState.wram.pokegear_phone_cursor_position = cursor;
    this.gameState.wram.pokegear_phone_scroll_position = scroll;
  }

  private clampRadioIndex(): void {
    const raw = this.gameState.wram.pokegear_radio_frequency_raw;
    const index = RADIO_FREQUENCY_DATA.findIndex((entry) => entry.raw === raw);
    if (index === -1) {
      this.radioIndex = 0;
      this.gameState.wram.pokegear_radio_frequency_raw = RADIO_FREQUENCY_DATA[0].raw;
    } else {
      this.radioIndex = index;
    }
  }

  private stationInJohto(): boolean {
    const entry = LANDMARK_ENTRY_BY_ID.get(this.playerLandmarkId);
    if (!entry) {
      return true;
    }
    return entry.region !== "KANTO";
  }

  private resolveStation(handler: string): string | null {
    if (shouldRocketRadioOverride(this.gameState)) {
      return "ROCKET_RADIO";
    }
    if (handler === "PKMNTalkAndPokedexShow") {
      if (!this.stationInJohto()) {
        return null;
      }
      const period = String(this.gameState.wram.time_of_day ?? "day").toLowerCase();
      if (this.playerLandmarkId === FAST_SHIP_LANDMARK) {
        return "POKEDEX_SHOW";
      }
      if (period === "morn" || period === "morning") {
        return "POKEDEX_SHOW";
      }
      return "OAKS_POKEMON_TALK";
    }
    if (handler === "PokemonMusic") {
      return this.stationInJohto() ? "POKEMON_MUSIC" : null;
    }
    if (handler === "LuckyChannel") {
      return this.stationInJohto() ? "LUCKY_CHANNEL" : null;
    }
    if (handler === "BuenasPassword") {
      return this.stationInJohto() ? "BUENAS_PASSWORD" : null;
    }
    if (handler === "RuinsOfAlphRadio") {
      const target = LANDMARK_ID_BY_CONSTANT.get("LANDMARK_RUINS_OF_ALPH");
      return this.playerLandmarkId === target ? "UNOWN_RADIO" : null;
    }
    if (handler === "PlacesAndPeople") {
      const flags = this.gameState.wram.engine_flags;
      if (this.stationInJohto() || !flags["ENGINE_EXPN_CARD"]) {
        return null;
      }
      return "PLACES_AND_PEOPLE";
    }
    if (handler === "LetsAllSing") {
      const flags = this.gameState.wram.engine_flags;
      if (this.stationInJohto() || !flags["ENGINE_EXPN_CARD"]) {
        return null;
      }
      return "LETS_ALL_SING";
    }
    if (handler === "PokeFluteRadio") {
      const flags = this.gameState.wram.engine_flags;
      if (this.stationInJohto() || !flags["ENGINE_EXPN_CARD"]) {
        return null;
      }
      return "POKE_FLUTE_RADIO";
    }
    if (handler === "EvolutionRadio") {
      const flags = this.gameState.wram.engine_flags;
      if (!flags["ENGINE_ROCKET_SIGNAL_ON_CH20"]) {
        return null;
      }
      return EVOLUTION_LANDMARKS.has(this.playerLandmarkId) ? "EVOLUTION_RADIO" : null;
    }
    return null;
  }
}

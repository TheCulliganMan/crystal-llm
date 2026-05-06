import { GameState } from "@pokecrystal/core/core/state";
import { NUM_LINK_BATTLE_RECORDS } from "@pokecrystal/core/core/constants";
import { countPokedexEntries } from "@pokecrystal/core/core/pokedex";
import { SerialConnectionStatus } from "@pokecrystal/core/core/memory/registers";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { showLabelledText } from "@pokecrystal/core/engine/world/story-events/text-helpers";
import type { LinkBattleRecord, Party, Trainer } from "@pokecrystal/core/core/models";
import { Pokemon, toPokemon } from "@pokecrystal/core/core/models/pokemon";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import type { Overworld as BaseOverworld } from "@pokecrystal/core/types/overworld";
import type { OverworldObject } from "@pokecrystal/core/engine/world/overworld/overworld-object";
import type { PhotoSnapshot } from "@pokecrystal/core/core/memory/sram";
import { gameEngine, type GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import { nextFrame } from "@pokecrystal/core/ui/async-loop";
import { NameEntryScreen } from "@pokecrystal/core/ui/screens/name-entry-screen";
import type { ScreenUI } from "@pokecrystal/core/ui/screens/screen-types";
import {
  ScriptRunner,
  ensureRunnerVariables,
  ensureScriptMemory,
  setRunnerValue,
  setSerialConnectionStatus,
} from "./utils";

type NameEntryUi = ScreenUI & { eventQueue?: GameEngineEventQueue | null };
type NameEntryScreenEvent = Parameters<NameEntryScreen["handleInput"]>[0];

type OverworldAccessor = BaseOverworld & {
  data_loader?: DataLoader | null;
  dataLoader?: DataLoader | null;
  get_object_by_id?: (identifier: string) => OverworldObject | null;
  getObjectById?: (identifier: string) => OverworldObject | null;
  audio_engine?: AudioEngine | null;
  audioEngine?: AudioEngine | null;
  ui?: NameEntryUi | null;
  input_capture_active?: boolean;
};

type PlaceholderContext = {
  runner?: ScriptRunner | null;
  overworld?: OverworldAccessor | null;
  event_manager?: EventManager | null;
};

type BattleTowerTrainer = Trainer;

type BattleTowerLoaderResultObject = {
  trainer?: BattleTowerTrainer;
  opponent?: BattleTowerTrainer;
  sprite_constant?: string | number;
  spriteConstant?: string | number;
  sprite?: string | number;
  sprite_id?: string | number;
};

type BattleTowerLoaderResult =
  | BattleTowerLoaderResultObject
  | [BattleTowerTrainer | undefined, string | number | undefined];

type BattleTowerLoader = (
  game_state: GameState,
  data_loader?: DataLoader | null,
  overworld?: OverworldAccessor | null
) => BattleTowerLoaderResult | null | undefined;

type BattleTowerDataLoader = DataLoader & {
  load_battle_tower_opponent?: BattleTowerLoader;
  loadBattleTowerOpponent?: BattleTowerLoader;
  getBattleTowerOpponent?: BattleTowerLoader;
  get_battle_tower_opponent?: BattleTowerLoader;
};

type SpeciesLookupDataLoader = DataLoader & {
  get_pokemon_species?: (id: string) => { id: string; int_id?: number };
  getPokemonSpecies?: (id: string) => { id: string; int_id?: number };
  getSpecies?: (id: string) => { id: string; int_id?: number };
};

type PartyMon = Pokemon | null;

type ScriptMemoryBucket = Record<string, unknown>;
type HramWithPrinter = GameState["hram"] & { hPrinter?: number };

type HaircutOutcome = {
  threshold: number;
  scriptValue: number;
  happinessChangeCode: number;
};

export const LINK_NULL = 0;
export const LINK_TIMECAPSULE = 1;
export const LINK_TRADECENTER = 2;
export const LINK_COLOSSEUM = 3;
export const LINK_MOBILE = 4;

const ensureStringBuffers = (runner?: ScriptRunner | null): Record<string, string> => {
  if (!runner) {
    return {};
  }
  if (!runner.string_buffers) {
    runner.string_buffers = {};
  }
  return runner.string_buffers;
};

export function name_rival(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): string | Promise<string> {
  // ASM: engine/events/specials.asm::NameRival
  // ASM prompt: engine/menus/naming_screen.asm::RivalNameString
  // ASM default name fallback: home/string.asm::InitName ("SILVER")
  void event_manager;
  const variables = ensureRunnerVariables(runner ?? undefined);
  const provided = String(variables._rival_name ?? "").trim();
  const defaultName = "SILVER";
  const isBlankName = (value: string): boolean => {
    for (let idx = 0; idx < value.length; idx += 1) {
      if (value[idx] !== " ") {
        return false;
      }
    }
    return true;
  };
  const finalize = (value: string): string => {
    const rivalName = isBlankName(value) ? defaultName : value;
    game_state.sram.rival_name = rivalName;
    if (runner) {
      runner.last_value = rivalName;
      runner.last_condition_result = true;
    }
    return rivalName;
  };

  if (provided) {
    return finalize(provided);
  }

  const activeOverworld = (overworld ?? (runner?.overworld as OverworldAccessor | null) ?? null) as
    | OverworldAccessor
    | null;
  const ui = (activeOverworld?.ui ?? null) as NameEntryUi | null;
  if (!ui || !ui.screen || typeof ui.clearScreen !== "function") {
    return finalize("");
  }

  const audioEngine =
    activeOverworld?.audio_engine ??
    activeOverworld?.audioEngine ??
    runner?.audio_engine ??
    runner?.audioEngine ??
    null;
  const promptText = "RIVAL'S NAME?";
  const screen = new NameEntryScreen(ui, promptText, audioEngine);
  screen.reset({ prompt: promptText });

  const previousCapture = activeOverworld?.input_capture_active ?? false;
  if (activeOverworld) {
    activeOverworld.input_capture_active = true;
  }

  const runNamingScreen = async (): Promise<string> => {
    try {
      while (!screen.finished) {
        for (const event of gameEngine.event.get(ui.eventQueue ?? undefined)) {
          if (event.type === gameEngine.QUIT) {
            gameEngine.quit();
            throw new Error("Quit requested during naming screen.");
          }
          screen.handleInput(event as NameEntryScreenEvent);
        }
        screen.update();
        screen.draw();
        ui.update?.();
        await nextFrame();
      }
    } finally {
      if (activeOverworld) {
        activeOverworld.input_capture_active = previousCapture;
      }
    }
    return finalize(screen.name);
  };

  return runNamingScreen();
}

export function trainer_house(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): boolean {
  // ASM: engine/events/specials.asm::TrainerHouse
  void overworld;
  void event_manager;
  const enabled = Boolean(game_state.sram.mystery_gift?.trainer_house_flag);
  if (runner) {
    setRunnerValue(runner, enabled ? 1 : 0, { truthy: enabled });
  }
  return enabled;
}

export function diploma(
  _game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): boolean {
  // ASM: engine/events/specials.asm::Diploma
  void overworld;
  if (event_manager?.dispatch) {
    event_manager.dispatch(new Event("show_diploma", { source: "special" }));
  }
  if (runner) {
    runner.last_condition_result = true;
  }
  return true;
}

export function print_diploma(
  _game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): boolean {
  // ASM: engine/events/specials.asm::PrintDiploma
  void overworld;
  if (event_manager?.dispatch) {
    event_manager.dispatch(new Event("print_diploma", { source: "special" }));
  }
  if (runner) {
    runner.last_condition_result = true;
  }
  return true;
}

export function stubbed_trainer_rankings_healings(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): number {
  // ASM: engine/events/specials.asm::StubbedTrainerRankings_Healings
  void overworld;
  void event_manager;
  const value = Number(game_state.wram.trainer_rankings_healings ?? 0);
  if (runner) {
    setRunnerValue(runner, value, { truthy: true });
  }
  return value;
}

export function _touch_state(game_state: GameState, key: string): ScriptMemoryBucket {
  const scriptMemory = ensureScriptMemory(game_state);
  let specials = scriptMemory.specials as Record<string, ScriptMemoryBucket> | undefined;
  if (!specials || typeof specials !== "object") {
    specials = {};
    scriptMemory.specials = specials;
  }
  let bucket = specials[key];
  if (!bucket || typeof bucket !== "object") {
    bucket = {};
    specials[key] = bucket;
  }
  bucket.count = Number(bucket.count ?? 0) + 1;
  return bucket;
}

const normalizeSpecies = (value: unknown, fallback = "MISSINGNO"): string => {
  const text = String(value ?? "").trim().toUpperCase();
  return text || fallback;
};

const HAPPINESS_CHANGE_TABLE: [number, number, number][] = [
  [5, 3, 2],
  [5, 3, 2],
  [1, 1, 0],
  [3, 2, 1],
  [1, 1, 0],
  [-1, -1, -1],
  [-5, -5, -10],
  [-5, -5, -10],
  [1, 1, 1],
  [3, 3, 1],
  [5, 5, 2],
  [1, 1, 1],
  [3, 3, 1],
  [10, 10, 4],
  [-5, -5, -10],
  [-10, -10, -15],
  [-15, -15, -20],
  [3, 3, 1],
  [10, 6, 4],
];

const OLDER_HAIRCUT_OUTCOMES: HaircutOutcome[] = [
  { threshold: 76, scriptValue: 2, happinessChangeCode: 9 },
  { threshold: 204, scriptValue: 3, happinessChangeCode: 10 },
  { threshold: 256, scriptValue: 4, happinessChangeCode: 11 },
];

const YOUNGER_HAIRCUT_OUTCOMES: HaircutOutcome[] = [
  { threshold: 154, scriptValue: 2, happinessChangeCode: 12 },
  { threshold: 230, scriptValue: 3, happinessChangeCode: 13 },
  { threshold: 256, scriptValue: 4, happinessChangeCode: 14 },
];

const DAISY_GROOMING_OUTCOMES: HaircutOutcome[] = [
  { threshold: 256, scriptValue: 2, happinessChangeCode: 18 },
];

const OAK_RATING_THRESHOLDS: Array<[number, string]> = [
  [9, "OakRating01"],
  [19, "OakRating02"],
  [34, "OakRating03"],
  [49, "OakRating04"],
  [64, "OakRating05"],
  [79, "OakRating06"],
  [94, "OakRating07"],
  [109, "OakRating08"],
  [124, "OakRating09"],
  [139, "OakRating10"],
  [154, "OakRating11"],
  [169, "OakRating12"],
  [184, "OakRating13"],
  [199, "OakRating14"],
  [214, "OakRating15"],
  [229, "OakRating16"],
  [239, "OakRating17"],
  [248, "OakRating18"],
  [255, "OakRating19"],
];

const resolveNamedObjectIndex = (
  speciesId: string,
  mon: PartyMon | null,
  runner?: ScriptRunner | null,
  overworld?: OverworldAccessor | null
): number => {
  const monIndex = mon?.species?.int_id;
  if (typeof monIndex === "number" && Number.isFinite(monIndex)) {
    return monIndex;
  }
  const dataLoader = (runner?.data_loader ??
    runner?.dataLoader ??
    overworld?.data_loader ??
    overworld?.dataLoader) as SpeciesLookupDataLoader | null | undefined;
  const lookup = dataLoader?.get_pokemon_species ?? dataLoader?.getPokemonSpecies ?? dataLoader?.getSpecies;
  if (lookup) {
    const species = lookup.call(dataLoader, speciesId);
    if (!species) {
      throw new Error(`Unknown species '${speciesId}'`);
    }
    if (typeof species.int_id !== "number") {
      throw new Error(`Missing numeric species id for '${speciesId}'`);
    }
    return species.int_id;
  }
  const numeric = Number.parseInt(speciesId, 10);
  if (!Number.isFinite(numeric)) {
    throw new Error(`Unable to resolve numeric species id for '${speciesId}'`);
  }
  return numeric;
};

const selectPartyMon = (
  game_state: GameState,
  runner?: ScriptRunner | null
): [PartyMon | null, number | null] => {
  const variables = ensureRunnerVariables(runner ?? undefined);
  if (variables._selection_cancelled) {
    return [null, null];
  }

  const party: Party = game_state.sram.party;
  const slots = [...party.pokemon];
  const candidates = slots
    .map((mon, idx) => ({ idx, mon }))
    .filter(
      (entry): entry is { idx: number; mon: Pokemon } => entry.mon !== null && entry.mon !== undefined
    );
  if (!candidates.length) {
    throw new Error("A party Pokemon is required for this action.");
  }

  let index = Number(variables._selected_party_index ?? 0);
  if (!Number.isFinite(index)) {
    index = 0;
  }
  let item = slots[index];
  let mon: PartyMon = item ? toPokemon(item) : null;
  if (!mon) {
    const fallback = candidates[0];
    index = fallback.idx;
    mon = fallback.mon ? toPokemon(fallback.mon) : null;
  }


  game_state.wram.wCurPartySpecies = String(mon?.species?.id ?? "");
  return [mon, index];
};

const selectPartyMonSlot = (
  game_state: GameState,
  runner?: ScriptRunner | null
): [{ happiness?: number; nickname?: string; species?: { id?: string; int_id?: number } } | null, number | null] => {
  const variables = ensureRunnerVariables(runner ?? undefined);
  if (variables._selection_cancelled) {
    return [null, null];
  }

  const slots = game_state.sram.party.pokemon;
  const candidates = slots
    .map((mon, idx) => ({ idx, mon }))
    .filter((entry) => entry.mon !== null && entry.mon !== undefined);
  if (!candidates.length) {
    throw new Error("A party Pokemon is required for this action.");
  }

  let index = Number(variables._selected_party_index ?? 0);
  if (!Number.isFinite(index)) {
    index = 0;
  }
  let mon = slots[index] as { happiness?: number; nickname?: string; species?: { id?: string; int_id?: number } } | null;
  if (!mon) {
    const fallback = candidates[0];
    index = fallback.idx;
    mon = fallback.mon;
  }

  game_state.wram.wCurPartySpecies = String(mon?.species?.id ?? "");
  return [mon, index];
};

const applyHappinessChange = (
  mon: { happiness?: number },
  happinessChangeCode: number
): void => {
  const row = HAPPINESS_CHANGE_TABLE[happinessChangeCode - 1];
  if (!row) {
    throw new Error(`Unsupported happiness change code ${happinessChangeCode}`);
  }
  const current = Number(mon.happiness ?? 0);
  const delta = current < 100 ? row[0] : current < 200 ? row[1] : row[2];
  mon.happiness = Math.max(0, Math.min(0xff, current + delta));
};

const randomByte = (game_state: GameState, runner?: ScriptRunner | null): number => {
  const variables = ensureRunnerVariables(runner ?? undefined);
  const forced = variables._rng_roll;
  if (forced !== undefined && forced !== null) {
    const value = Number(forced);
    if (!Number.isFinite(value)) {
      throw new Error(`Invalid _rng_roll override '${String(forced)}'`);
    }
    return value & 0xff;
  }
  return new HardwareRNG(game_state).nextByte();
};

const runHaircutOrGrooming = (
  game_state: GameState,
  runner: ScriptRunner | null | undefined,
  outcomes: HaircutOutcome[]
): number => {
  const [mon] = selectPartyMonSlot(game_state, runner);
  if (!mon) {
    return setRunnerValue(runner, 0, { truthy: false });
  }

  const species = normalizeSpecies(mon.species?.id ?? "", "");
  if (species === "EGG") {
    return setRunnerValue(runner, 1, { truthy: true });
  }

  const displayName = String(mon.nickname ?? species).trim() || species;
  const buffers = ensureStringBuffers(runner ?? undefined);
  if (runner) {
    buffers.STRING_BUFFER_1 = displayName;
    buffers.STRING_BUFFER_3 = displayName;
  }

  const roll = randomByte(game_state, runner);
  const outcome = outcomes.find((entry) => roll < entry.threshold);
  if (!outcome) {
    return setRunnerValue(runner, 0, { truthy: false });
  }

  applyHappinessChange(mon, outcome.happinessChangeCode);
  return setRunnerValue(runner, outcome.scriptValue, { truthy: true });
};

const appendPhotoAlbumEntry = (game_state: GameState, snapshot: PhotoSnapshot): void => {
  const current = Array.isArray(game_state.sram.photo_album)
    ? [...game_state.sram.photo_album]
    : [];
  current.push(snapshot);
  if (current.length > 30) {
    current.splice(0, current.length - 30);
  }
  game_state.sram.photo_album = current;
};

const resolveBattleTowerLoader = (dataLoader: DataLoader | null | undefined): BattleTowerLoader => {
  const battleLoader = dataLoader as BattleTowerDataLoader | null | undefined;
  const loader =
    battleLoader?.load_battle_tower_opponent ??
    battleLoader?.loadBattleTowerOpponent ??
    battleLoader?.getBattleTowerOpponent ??
    battleLoader?.get_battle_tower_opponent;
  if (!loader) {
    throw new Error(
      "Battle Tower opponents require a data loader to resolve sprites and parties."
    );
  }
  return loader as BattleTowerLoader;
};

const ensureLuckyNumber = (game_state: GameState): number => {
  const storedDay = Number(game_state.sram.lucky_number_day ?? -1);
  const currentDay = Number(game_state.wram.wCurDay ?? -1) & 0xff;
  let number = Number(game_state.sram.lucky_id_number ?? 0) & 0xffff;
  if (currentDay !== storedDay) {
    const rng = new HardwareRNG(game_state);
    number = ((rng.nextByte() << 8) | rng.nextByte()) & 0xffff;
    game_state.sram.lucky_number_day = currentDay;
    game_state.sram.lucky_id_number = number;
  }
  game_state.wram.script_memory.lucky_id_number = number;
  return number;
};

export function load_opponent_trainer_and_pokemon_with_ot_sprite(
  game_state: GameState,
  context: PlaceholderContext = {}
): unknown {
  const { runner, overworld, event_manager } = context;
  // ASM: engine/events/battle_tower/battle_tower.asm::LoadOpponentTrainerAndPokemonWithOTSprite
  void event_manager;

  const dataLoader =
    overworld?.data_loader ??
    overworld?.dataLoader ??
    runner?.data_loader ??
    runner?.dataLoader ??
    null;
  if (!dataLoader) {
    throw new Error(
      "Battle Tower opponents require a data loader to resolve sprites and parties."
    );
  }

  const loader = resolveBattleTowerLoader(dataLoader);
  const result = loader.call(dataLoader, game_state, dataLoader, overworld);
  if (!result) {
    throw new Error("Battle Tower opponent loader returned no data.");
  }
  let trainer: BattleTowerTrainer | undefined;
  let spriteConstant: string | number | undefined;
  if (Array.isArray(result)) {
    [trainer, spriteConstant] = result;
  } else {
    trainer = result.trainer ?? result.opponent ?? undefined;
    spriteConstant =
      result.sprite_constant ??
      result.spriteConstant ??
      result.sprite ??
      result.sprite_id ??
      undefined;
  }
  if (!trainer) {
    throw new Error("Battle Tower opponent loader returned no trainer data.");
  }

  if (trainer.name === undefined || trainer.name === null || !Array.isArray(trainer.party)) {
    throw new Error("Battle Tower opponent loader returned incomplete trainer data.");
  }

  const state = _touch_state(game_state, "battle_tower");
  const trainerIdText = trainer.trainer_id !== undefined && trainer.trainer_id !== null
    ? String(trainer.trainer_id)
    : "";
  state.loaded_trainer = trainer;
  state.last_sprite_constant = spriteConstant;
  if (game_state.wram.script_memory.wScriptVar === undefined) {
    game_state.wram.script_memory.wScriptVar = 1;
  }

  const wram = game_state.wram;
  wram.other_trainer_class = trainer.trainer_class ?? "";
  wram.other_trainer_id = trainerIdText;
  wram.other_trainer = trainer;
  wram.other_trainer_party = Array.isArray(trainer.party) ? [...trainer.party] : [];

  const runnerVariables = ensureRunnerVariables(runner ?? undefined);
  const targetObject = String(runnerVariables._value ?? "BATTLETOWERBATTLEROOM_YOUNGSTER");
  const npc =
    typeof overworld?.get_object_by_id === "function"
      ? overworld.get_object_by_id(targetObject)
      : typeof overworld?.getObjectById === "function"
      ? overworld.getObjectById(targetObject)
      : null;
  if (npc && spriteConstant) {
    const spriteName = String(spriteConstant);
    if (typeof npc.setSprite === "function") {
      npc.setSprite(spriteName);
    } else if ("set_sprite" in npc && typeof (npc as { set_sprite?: (sprite: string) => void }).set_sprite === "function") {
      (npc as { set_sprite: (sprite: string) => void }).set_sprite(spriteName);
    }
  }

  if (runner) {
    runner.loaded_trainer = trainer;
    runner.loaded_trainer_id = trainerIdText || null;
  }

  return setRunnerValue(runner, "$1", { truthy: true });
}

export function display_coin_case_balance(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): number {
  // ASM: engine/menus/menu_2.asm::DisplayCoinCaseBalance
  void overworld;

  const coins = Math.max(0, Number(game_state.sram.coins ?? 0));
  const overlay = {
    width: 7,
    height: 1,
    x: 11,
    y: 0,
    label: "COIN",
    value: coins,
  };
  if (event_manager?.dispatch) {
    event_manager.dispatch(
      new Event("show_coin_case_balance", { source: "special", overlay })
    );
  }

  const buffers = ensureStringBuffers(runner ?? undefined);
  if (runner) {
    buffers.STRING_BUFFER_1 = String(coins).padStart(4, "0");
  }
  setRunnerValue(runner, coins, { truthy: true });
  return coins;
}

type LinkBattleRecordExtended = LinkBattleRecord & {
  trainer_id?: number;
  trainer_name?: string;
};

type LinkRecordEntry = {
  trainer_id: number;
  trainer_name: string;
  wins: number;
  losses: number;
  draws: number;
  total: number;
};

type LinkRecordPayload = {
  stats: {
    wins: number;
    losses: number;
    draws: number;
    total: number;
  };
  records: LinkRecordEntry[];
};

export function display_link_record(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): LinkRecordPayload {
  // ASM: engine/events/specials.asm::DisplayLinkRecord
  void overworld;
  void event_manager;

  const stats = game_state.sram.link_battle_stats;
  const records: LinkBattleRecordExtended[] = Array.isArray(game_state.sram.link_battle_records)
    ? [...game_state.sram.link_battle_records]
    : [];
  if (records.length !== NUM_LINK_BATTLE_RECORDS) {
    throw new Error(
      `Expected ${NUM_LINK_BATTLE_RECORDS} link battle records, found ${records.length}`
    );
  }

  const serialised = records.map((record) => {
    const wins = Number(record.stats?.wins ?? 0);
    const losses = Number(record.stats?.losses ?? 0);
    const draws = Number(record.stats?.draws ?? 0);
    const total = wins + losses + draws;
    const trainerId = Number(record.trainer_id ?? record.id ?? 0);
    const trainerName = record.trainer_name ?? record.name ?? "---";
    return {
      trainer_id: trainerId,
      trainer_name: trainerName || "---",
      wins,
      losses,
      draws,
      total,
    };
  });

  const statsPayload: LinkRecordPayload["stats"] = {
    wins: Number(stats?.wins ?? 0),
    losses: Number(stats?.losses ?? 0),
    draws: Number(stats?.draws ?? 0),
    total: Number(stats?.wins ?? 0) + Number(stats?.losses ?? 0) + Number(stats?.draws ?? 0),
  };
  const payload: LinkRecordPayload = { stats: statsPayload, records: serialised };

  if (runner) {
    const variables = ensureRunnerVariables(runner);
    variables.link_record_payload = payload;
  }
  setRunnerValue(runner, payload, { truthy: Boolean(statsPayload.total) });
  return payload;
}

export function print_todays_lucky_number(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): string {
  // ASM: engine/events/lucky_number.asm::PrintTodaysLuckyNumber
  void overworld;
  void event_manager;

  const number = ensureLuckyNumber(game_state);
  const formatted = String(number).padStart(5, "0");
  if (runner) {
    const buffers = ensureStringBuffers(runner);
    buffers.STRING_BUFFER_3 = formatted;
  }
  setRunnerValue(runner, formatted, { truthy: true });
  return formatted;
}

export function bills_grandfather(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): string | number {
  // ASM: engine/events/haircut.asm::BillsGrandfather
  void overworld;
  void event_manager;

  const scriptMemory = ensureScriptMemory(game_state);
  const variables = ensureRunnerVariables(runner ?? undefined);
  const manualSpecies = variables._selected_species;
  let mon: PartyMon | null = null;
  let species = "";

  if (manualSpecies) {
    species = normalizeSpecies(manualSpecies);
  } else {
    const [selected, _index] = selectPartyMon(game_state, runner ?? undefined);
    mon = selected;
    species = normalizeSpecies(mon?.species?.id ?? "", "");
  }

  if (!mon && !species) {
    scriptMemory.wScriptVar = 0;
    scriptMemory.wNamedObjectIndex = 0;
    return setRunnerValue(runner, 0, { truthy: false });
  }

  let displayName = species.replace(/_/g, " ");
  const formatter = runner?.formatText ?? runner?.format_text;
  if (typeof formatter === "function") {
    displayName = formatter(displayName);
  }

  scriptMemory.wScriptVar = species;
  scriptMemory.wNamedObjectIndex = resolveNamedObjectIndex(species, mon, runner ?? null, overworld ?? null);
  game_state.wram.wCurPartySpecies = species;

  const buffers = ensureStringBuffers(runner ?? undefined);
  if (runner) {
    buffers.STRING_BUFFER_1 = displayName;
    buffers.STRING_BUFFER_3 = displayName;
  }

  return setRunnerValue(runner, species, { truthy: true });
}

export function celebi_shrine_event(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): boolean {
  // ASM: engine/events/celebi.asm::CelebiShrineEvent
  void overworld;
  void event_manager;

  game_state.wram.battle_type = "BATTLETYPE_CELEBI";
  _touch_state(game_state, "celebi_shrine").triggered = true;
  return Boolean(setRunnerValue(runner, 1, { truthy: true }));
}

export function colosseum(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): void {
  // ASM: engine/link/link.asm::Colosseum
  void runner;
  void overworld;
  void event_manager;
  game_state.wram.wLinkMode = LINK_COLOSSEUM;
}

export function trade_center(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): void {
  // ASM: engine/link/link.asm::TradeCenter
  void runner;
  void overworld;
  void event_manager;
  game_state.wram.wLinkMode = LINK_TRADECENTER;
}

export function failed_link_to_past(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): boolean {
  // ASM: engine/link/link.asm::FailedLinkToPast
  void overworld;
  void event_manager;

  const state = _touch_state(game_state, "link_rooms");
  state.failed_link = true;
  game_state.wram.wLinkMode = LINK_TIMECAPSULE;

  setSerialConnectionStatus(game_state, SerialConnectionStatus.CONNECTION_NOT_ESTABLISHED);

  return Boolean(setRunnerValue(runner, "$0", { truthy: false }));
}

export function move_deletion(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): string {
  // ASM: engine/events/move_deleter.asm::MoveDeletion
  void overworld;
  void event_manager;

  const variables = ensureRunnerVariables(runner ?? undefined);
  if (variables._selection_cancelled) {
    return setRunnerValue(runner, "", { truthy: false });
  }

  const [mon] = selectPartyMon(game_state, runner ?? undefined);
  if (!mon) {
    return setRunnerValue(runner, "", { truthy: false });
  }

  if (normalizeSpecies(mon.species?.id ?? "", "") === "EGG") {
    return setRunnerValue(runner, "", { truthy: false });
  }

  const moves = Array.isArray(mon.moves)
    ? mon.moves.filter(
        (move): move is NonNullable<(typeof mon.moves)[number]> =>
          move !== null && move !== undefined && typeof move.name === "string"
      )
    : [];
  if (moves.length <= 1) {
    return setRunnerValue(runner, "", { truthy: false });
  }

  const selectedMoveIndexRaw = variables._selected_move_index;
  let selectedMoveIndex: number | null = null;
  if (selectedMoveIndexRaw !== undefined && selectedMoveIndexRaw !== null) {
    const numericIndex = Number(selectedMoveIndexRaw);
    if (!Number.isFinite(numericIndex)) {
      throw new Error(`Invalid move deletion index '${String(selectedMoveIndexRaw)}'`);
    }
    selectedMoveIndex = Math.trunc(numericIndex);
  } else if (variables._selected_move !== undefined && variables._selected_move !== null) {
    const selectedMove = String(variables._selected_move).toUpperCase();
    selectedMoveIndex = moves.findIndex((move) => String(move.name).toUpperCase() === selectedMove);
  }

  if (selectedMoveIndex === null) {
    throw new Error("MoveDeletion requires an explicit move selection.");
  }
  if (selectedMoveIndex < 0 || selectedMoveIndex >= moves.length) {
    throw new Error(`MoveDeletion move index ${selectedMoveIndex} is out of range.`);
  }

  const deletedMove = String(moves[selectedMoveIndex].name).toUpperCase();
  mon.moves.splice(selectedMoveIndex, 1);
  return setRunnerValue(runner, deletedMove, { truthy: true });
}

export function older_haircut_brother(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): number {
  // ASM: engine/events/haircut.asm::OlderHaircutBrother
  void overworld;
  void event_manager;

  _touch_state(game_state, "haircut").older_brother = true;
  return runHaircutOrGrooming(game_state, runner, OLDER_HAIRCUT_OUTCOMES);
}

export function younger_haircut_brother(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): number {
  // ASM: engine/events/haircut.asm::YoungerHaircutBrother
  void overworld;
  void event_manager;

  _touch_state(game_state, "haircut").younger_brother = true;
  return runHaircutOrGrooming(game_state, runner, YOUNGER_HAIRCUT_OUTCOMES);
}

export function daisys_grooming(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): number {
  // ASM: engine/events/haircut.asm::DaisysGrooming
  void overworld;
  void event_manager;

  _touch_state(game_state, "haircut").daisy = true;
  return runHaircutOrGrooming(game_state, runner, DAISY_GROOMING_OUTCOMES);
}

export function photo_studio(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): boolean {
  // ASM: engine/events/print_photo.asm::PhotoStudio
  const state = _touch_state(game_state, "photo");
  const hram = game_state.hram as HramWithPrinter;

  const eventManager = event_manager ?? runner?.eventManager ?? runner?.event_manager;
  if (runner && eventManager) {
    showLabelledText(runner, overworld ?? null, eventManager, "WhichMonPhotoText");
  }

  const [mon, index] = selectPartyMon(game_state, runner ?? undefined);
  if (!mon) {
    if (runner && eventManager) {
      showLabelledText(runner, overworld ?? null, eventManager, "NoPhotoText");
    }
    game_state.wram.script_memory.wScriptVar = 0;
    setRunnerValue(runner, 0, { truthy: false });
    return false;
  }
  if (index === null) {
    throw new Error("PhotoStudio requires a selected party slot.");
  }

  const species = normalizeSpecies(mon?.species?.id ?? "");
  const nickname = mon?.nickname || species;
  const paletteId = Number(game_state.wram.player_palette_id ?? 0);
  const namedObjectIndex = resolveNamedObjectIndex(species, mon, runner ?? null, overworld ?? null);
  const snapshot = {
    species,
    nickname,
    palette: paletteId,
    level: Number(mon?.level ?? 0),
    trainer_id: Number(mon?.original_trainer_id ?? 0),
    party_index: index,
    frame: Number(game_state.wram.wFrameCounter ?? 0),
  };

  game_state.wram.script_memory.wNamedObjectIndex = namedObjectIndex;

  if (species === "EGG") {
    if (runner && eventManager) {
      showLabelledText(runner, overworld ?? null, eventManager, "EggPhotoText");
    }
    game_state.wram.script_memory.wScriptVar = 0;
    setRunnerValue(runner, species, { truthy: false });
    return false;
  }

  if (runner && eventManager) {
    showLabelledText(runner, overworld ?? null, eventManager, "HoldStillText");
  }

  if (Number(hram.hPrinter ?? 0) !== 0) {
    if (runner && eventManager) {
      showLabelledText(runner, overworld ?? null, eventManager, "NoPhotoText");
    }
    game_state.wram.script_memory.wScriptVar = 0;
    setRunnerValue(runner, 0, { truthy: false });
    return false;
  }

  appendPhotoAlbumEntry(game_state, snapshot);
  state.snapshots = Number(state.snapshots ?? 0) + 1;

  if (eventManager?.dispatch) {
    eventManager.dispatch(new Event("photo_snapshot", { snapshot }));
  }

  const audioEngine = overworld?.audio_engine ?? overworld?.audioEngine;
  if (audioEngine?.playSound) {
    try {
      audioEngine.playSound("SFX_SHUTTER");
    } catch {
      try {
        audioEngine.playSound("SFX_READ_TEXT_2");
      } catch {
        // Ignore missing sound effect.
      }
    }
  }

  if (runner && eventManager) {
    showLabelledText(runner, overworld ?? null, eventManager, "PrestoAllDoneText");
  }
  game_state.wram.script_memory.wScriptVar = 1;
  setRunnerValue(runner, species, { truthy: true });
  return true;
}

export function prof_oaks_pc_boot(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: PlaceholderContext = {}
): boolean {
  // ASM: engine/events/prof_oaks_pc.asm::ProfOaksPCBoot
  void overworld;
  void event_manager;

  const seen = countPokedexEntries(game_state.sram.pokedex_seen);
  const owned = countPokedexEntries(game_state.sram.pokedex_owned);
  const ratingLabel =
    OAK_RATING_THRESHOLDS.find(([limit]) => owned <= limit)?.[1] ??
    OAK_RATING_THRESHOLDS[OAK_RATING_THRESHOLDS.length - 1][1];
  const state = _touch_state(game_state, "oak_pc");
  const buffers = ensureStringBuffers(runner ?? undefined);
  state.opened = true;
  state.pokedex_seen = seen;
  state.pokedex_owned = owned;
  state.rating_label = ratingLabel;
  if (runner) {
    const variables = ensureRunnerVariables(runner);
    buffers.STRING_BUFFER_3 = String(seen);
    buffers.STRING_BUFFER_4 = String(owned);
    variables._oak_rating_label = ratingLabel;
    variables._oak_seen_count = seen;
    variables._oak_owned_count = owned;
    runner.last_condition_result = true;
  }
  return true;
}

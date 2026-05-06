// ASM mapping: pokecrystal_disassembly/engine/overworld/scripting.asm (standard script helpers).
import { chooseRockSmashEncounter, getRockSetForMap } from "@pokecrystal/assets/content/tree-encounters";
import { TEXTBOX_DELAY_FLAG } from "@pokecrystal/core/core/text-constants";
import { GameState } from "@pokecrystal/core/core/state";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { Event, type EventManager } from "@pokecrystal/core/engine/events/events";
import { getMapMetadataByGroup, getMapMetadataByName, mapConstantToName } from "@pokecrystal/core/engine/world/maps";
import { loadPhoneContactDirectory } from "@pokecrystal/core/ui/menus/pokegear-contacts";
import { PokegearStateMachine } from "@pokecrystal/core/ui/menus/pokegear-state";
import { applyEventFlag } from "../event-flags";
import {
  closeTextBox as closeTextBoxInternal,
  openTextBox as openTextBoxInternal,
  resolveText as resolveTextInternal,
  showLabelledText as showLabelledTextInternal,
  type OverworldContext,
} from "../text-helpers";
import { PHONE_CONTACT_BASE_NAMES_FEMALE, PHONE_CONTACT_BASE_NAMES_MALE } from "../common";
import { FortuneTellerState } from "./fortune";
import { VerboseGiveItemCommand } from "../commands/items";
import type { OverworldObject } from "@pokecrystal/core/engine/world/overworld/overworld-object";

type AudioEngineLike = {
  play_sound?: (name: string) => void;
  playSound?: (name: string) => void;
  play_music?: (name: string, role?: string | { role?: string }) => void;
  playMusic?: (name: string, role: string) => void;
  restart_map_music?: () => void;
  restartMapMusic?: () => void;
};

import { ScriptRunner } from "../runner";

export type RockSmashResult = {
  smashed: boolean;
  encounter: [string, number] | null;
};

export type RockSmashPending = {
  object_id: string | number | null;
  encounter?: [string, number];
};

const getOverworldTextContext = (runner: ScriptRunner): OverworldContext => ({
  dataLoader: runner.overworld?.dataLoader ?? runner.overworld?.data_loader ?? null,
});

export const resolveText = (runner: ScriptRunner, label: string): string => {
  const overworld = getOverworldTextContext(runner);
  return resolveTextInternal(runner, overworld, label);
};

export const openTextBox = (runner: ScriptRunner): void => {
  const eventManager = runner.event_manager;
  if (!eventManager) {
    return;
  }
  openTextBoxInternal(eventManager);
};

export const closeTextBox = (runner: ScriptRunner): void => {
  const eventManager = runner.event_manager;
  if (!eventManager) {
    return;
  }
  closeTextBoxInternal(eventManager);
};

export const showLabelledText = (
  runner: ScriptRunner,
  label: string,
  options: { wait?: boolean; logEvent?: boolean; autoCloseAfterWait?: boolean } = {},
): string => {
  const overworld = getOverworldTextContext(runner);
  const eventManager = runner.event_manager;
  if (!eventManager) {
    return resolveText(runner, label);
  }
  return showLabelledTextInternal(
    runner,
    overworld,
    eventManager,
    label,
    options,
  );
};

export const startFortuneCookie = (runner: ScriptRunner): void => {
  const state = runner.game_state;
  if (!state) {
    return;
  }
  state.wram.wFortuneTellerState = FortuneTellerState.SCROLLING;
  state.wram.wTextboxFlags |= TEXTBOX_DELAY_FLAG;
};

export const finishFortuneCookie = (runner: ScriptRunner): void => {
  const state = runner.game_state;
  if (!state) {
    return;
  }
  state.wram.wFortuneTellerState = FortuneTellerState.READY;
};

export const fortuneCookieIsScrolling = (gameState: GameState): boolean => {
  const state = gameState.wram.wFortuneTellerState ?? 0;
  return state === FortuneTellerState.SCROLLING;
};

export const runPhoneTextScript = (
  runner: ScriptRunner,
  suffix: string,
  options: {
    female?: boolean;
    shouldOpenTextBox?: boolean;
    shouldCloseTextBox?: boolean;
    wait?: boolean;
    playSound?: string | null;
  } = {},
): string => {
  const {
    female = false,
    shouldOpenTextBox = true,
    shouldCloseTextBox = true,
    wait = true,
    playSound = null,
  } = options;

  const contactId = currentPhoneContact(runner);
  const label = resolvePhoneLabel(runner, contactId, suffix, { female });

  if (shouldOpenTextBox) {
    openTextBox(runner);
  }
  const autoCloseAfterWait = shouldCloseTextBox && wait;
  const message = showLabelledText(runner, label, {
    wait,
    autoCloseAfterWait,
  });

  if (playSound) {
    const audioEngine: AudioEngineLike | null = runner.overworld?.audio_engine ?? runner.overworld?.audioEngine ?? null;
    if (audioEngine?.play_sound) {
      audioEngine.play_sound(playSound);
    } else if (audioEngine?.playSound) {
      audioEngine.playSound(playSound);
    }
    runner.last_sound_effect = playSound;
  }

  if (shouldCloseTextBox && !autoCloseAfterWait) {
    closeTextBox(runner);
  }

  runner.variables["_last_phone_contact"] = contactId;
  runner.last_value = {
    phone_text: {
      contact: contactId,
      label,
      message,
      female,
    },
  };
  return label;
};

export const setEventFlag = (runner: ScriptRunner, flag: string, value: boolean): void => {
  const state = runner.game_state;
  if (!state) {
    return;
  }
  applyEventFlag(state, flag, { value, overworld: runner.overworld ?? null });
};

export const setEngineFlag = (runner: ScriptRunner, flag: string, value: boolean): void => {
  const state = runner.game_state;
  if (!state) {
    return;
  }
  state.wram.engine_flags[flag] = Boolean(value);
};

export const queueSpecialPhoneCall = (runner: ScriptRunner, callId: string): void => {
  const state = runner.game_state;
  if (!state) {
    return;
  }
  const queue = state.wram.scheduled_phone_calls;
  if (!queue.includes(callId)) {
    queue.push(callId);
  }
};

export const currentPhoneContact = (runner: ScriptRunner): string => {
  let contact = runner.variables?.VAR_CALLERID;
  if (Array.isArray(contact)) {
    contact = contact[0];
  }

  if (!contact) {
    let last = runner.variables?._last_phone_contact;
    if (Array.isArray(last)) {
      last = last[0];
    }
    if (last) {
      contact = last;
    }
  }

  if (!contact && typeof runner.last_value === "string") {
    contact = runner.last_value;
  }

  if (!contact && runner.last_value && typeof runner.last_value === "object") {
    const details = (runner.last_value as Record<string, unknown>)["phone_text"];
    if (details && typeof details === "object") {
      contact = (details as Record<string, unknown>).contact;
    }
  }

  if (!contact) {
    return "";
  }

  const resolved = loadPhoneContactDirectory().resolveContactId(String(contact)) ?? String(contact);
  runner.variables["_last_phone_contact"] = resolved;
  return resolved;
};

export const resolvePhoneLabel = (
  _runner: ScriptRunner,
  contactId: string,
  suffix: string,
  { female }: { female: boolean },
): string => {
  const mapping = female ? PHONE_CONTACT_BASE_NAMES_FEMALE : PHONE_CONTACT_BASE_NAMES_MALE;
  const base = mapping[contactId];
  if (!base) {
    throw new Error(`Unknown phone contact '${contactId}' for suffix '${suffix}'.`);
  }
  return `${base}${suffix}`;
};

const getMoveIdentifier = (move: unknown): string | null => {
  if (!move) {
    return null;
  }
  if (typeof move === "string") {
    return move;
  }
  if (typeof move === "object" && move !== null && "name" in move) {
    const candidate = (move as Record<string, unknown>).name;
    if (typeof candidate === "string") {
      return candidate;
    }
    if (candidate !== undefined && candidate !== null) {
      return String(candidate);
    }
  }
  return null;
};

export const partyHasMove = (runner: ScriptRunner, moveName: string): boolean => {
  const target = moveName.toUpperCase();
  const state = runner.game_state;
  if (!state) {
    return false;
  }
  for (const pokemon of state.sram.party.pokemon) {
    if (!pokemon) {
      continue;
    }
    for (const move of pokemon.moves ?? []) {
      const normalized = getMoveIdentifier(move);
      if (!normalized) {
        continue;
      }
      if (normalized.toUpperCase() === target) {
        return true;
      }
    }
  }
  return false;
};

export const selectStrengthSpecies = (runner: ScriptRunner): string | null => {
  const state = runner.game_state;
  if (!state) {
    return null;
  }
  for (const pokemon of state.sram.party.pokemon) {
    if (!pokemon) {
      continue;
    }
    for (const move of pokemon.moves ?? []) {
      const normalized = getMoveIdentifier(move);
      if (!normalized) {
        continue;
      }
      if (normalized.toUpperCase() !== "STRENGTH") {
        continue;
      }
      let nickname = String(pokemon.nickname ?? pokemon.species?.id ?? "").trim();
      if (!nickname) {
        nickname = String(pokemon.species?.id ?? "");
      }
      if (runner.string_buffers) {
        runner.string_buffers["STRING_BUFFER_3"] = nickname;
      } else {
        runner.string_buffers = { STRING_BUFFER_3: nickname };
      }
      return pokemon.species?.id ?? null;
    }
  }
  return null;
};

export const setMoveActorBuffer = (
  runner: ScriptRunner,
  moveName: string,
  buffer = "STRING_BUFFER_2",
): string => {
  const overworld = runner.overworld ?? null;
  let nickname = "";
  const moveHolder = (overworld as unknown as { _get_party_move_holder?: (move: string) => unknown } | null)
    ?._get_party_move_holder;
  if (moveHolder) {
    try {
      const result = moveHolder(moveName);
      nickname = Array.isArray(result) ? String(result[1] ?? "") : String(result ?? "");
    } catch {
      nickname = "";
    }
  }
  const cleaned = (nickname || moveName.replace(/_/g, " ")).trim().toUpperCase();
  if (buffer) {
    if (!runner.string_buffers) {
      runner.string_buffers = {};
    }
    runner.string_buffers[buffer] = cleaned;
  }
  return cleaned;
};

export const recordWarpDestination = (
  runner: ScriptRunner,
  mapConstant: string,
  x: number,
  y: number,
): void => {
  const existing = runner.last_value && typeof runner.last_value === "object" ? runner.last_value : {};
  runner.last_value = {
    ...existing,
    warp: {
      map_constant: mapConstant,
      map_name: mapConstantToName(mapConstant),
      x,
      y,
    },
  };
};

export const giveVerboseItem = (runner: ScriptRunner, itemName: string): void => {
  const command = new VerboseGiveItemCommand(itemName);
  command.runner = runner;
  const state = runner.game_state;
  if (!state) {
    return;
  }
  const eventManager = runner.event_manager;
  command.execute(state, eventManager, runner.overworld);
};

export const performRockSmash = (runner: ScriptRunner): RockSmashResult => {
  const overworld = runner.overworld ?? null;
  if (!overworld) {
    return { smashed: false, encounter: null };
  }
  const getter = overworld.get_object_by_id ?? overworld.getObjectById;
  const rock = typeof getter === "function" ? getter.call(overworld, "LAST_TALKED") : null;
  if (!rock) {
    return { smashed: false, encounter: null };
  }
  if (!("event" in rock)) {
    return { smashed: false, encounter: null };
  }
  const npc = rock as OverworldObject;
  const movement = String(npc.event?.spritemovedata ?? "");
  if (!movement || !movement.toUpperCase().includes("SMASHABLE_ROCK")) {
    return { smashed: false, encounter: null };
  }
  let identifier: string | number | null = npc.objectIndex ?? null;
  if (identifier === null || identifier === undefined) {
    identifier = npc.objectId ?? npc.spriteId ?? null;
  }

  const encounter = maybeTriggerRockSmashEncounter(runner);
  const state = runner.game_state;
  if (!state) {
    return { smashed: false, encounter: null };
  }
  const wram = state.wram;
  const fieldMoveOverworld = overworld as unknown as {
    _ROCK_SMASH_BREAK_FRAMES?: number;
    _pending_rock_smash?: RockSmashPending;
    _start_field_move_animation?: (
      animation: string,
      frames: number,
      tile_x: number,
      tile_y: number,
      options?: { metadata?: { source?: string } }
    ) => void;
    startFieldMoveAnimation?: (
      animation: string,
      frames: number,
      tile_x: number,
      tile_y: number,
      options?: { metadata?: { source?: string } }
    ) => void;
    _tick_field_move_states?: () => void;
  };
  wram.wRockSmashState = 1;
  wram.wRockSmashTile = [rock.x, rock.y];
  wram.wRockSmashStepTimer = Number(fieldMoveOverworld._ROCK_SMASH_BREAK_FRAMES ?? 0x0b);
  wram.wRockSmashEncounter = encounter ? { species: encounter[0], level: encounter[1] } : undefined;
  const pending: RockSmashPending = { object_id: identifier };
  if (encounter) {
    pending.encounter = encounter;
  }
  fieldMoveOverworld._pending_rock_smash = pending;

  const startAnimation =
    fieldMoveOverworld._start_field_move_animation ?? fieldMoveOverworld.startFieldMoveAnimation;
  try {
    if (typeof startAnimation === "function") {
      startAnimation.call(
        overworld,
        "ROCK_SMASH",
        wram.wRockSmashStepTimer,
        rock.x,
        rock.y,
        { metadata: { source: "rock_smash" } },
      );
    }
  } catch {
    // Ignore animation failures and allow state to proceed.
  }

  if (wram.wRockSmashStepTimer <= 0 && typeof fieldMoveOverworld._tick_field_move_states === "function") {
    fieldMoveOverworld._tick_field_move_states();
  }
  return { smashed: true, encounter };
};

const maybeTriggerRockSmashEncounter = (runner: ScriptRunner): [string, number] | null => {
  const overworld = runner.overworld ?? null;
  const mapName = overworld?.current_map_name ?? null;
  let metadata = mapName ? getMapMetadataByName(mapName) : undefined;
  const state = runner.game_state;
  if (!metadata && state) {
    metadata = getMapMetadataByGroup(state.wram.current_map_group, state.wram.current_map_id);
  }
  if (!metadata) {
    return null;
  }
  const rockSet = getRockSetForMap(metadata.constant);
  if (!rockSet) {
    return null;
  }
  const rng = new HardwareRNG(state as GameState);
  return chooseRockSmashEncounter(rockSet, (max) => rng.randrange(max));
};

export const bugContestCopyContestants = (runner: ScriptRunner): void => {
  const state = runner.game_state;
  if (!state) {
    return;
  }
  for (let index = 1; index <= 10; index += 1) {
    const flagA = `EVENT_BUG_CATCHING_CONTESTANT_${index}A`;
    const flagB = `EVENT_BUG_CATCHING_CONTESTANT_${index}B`;
    if (!state.wram.event_flags[flagA]) {
      setEventFlag(runner, flagB, false);
    }
  }
};

export const determineBugContestRank = (runner: ScriptRunner): number => {
  let rankValue = runner.variables?._bug_contest_rank ?? null;
  if (rankValue) {
    delete runner.variables._bug_contest_rank;
  } else {
    rankValue = runner.string_buffers?.STRING_BUFFER_3 ?? null;
  }
  let rank = 0;
  if (rankValue !== null && rankValue !== undefined) {
    const parsed = Number(rankValue);
    rank = Number.isFinite(parsed) ? parsed : 0;
  }
  if (rank <= 0) {
    const results = runner.game_state?.wram?.bug_contest_results;
    if (results?.first_place?.winner_id === 1) {
      rank = 1;
    } else if (results?.second_place?.winner_id === 1) {
      rank = 2;
    } else if (results?.third_place?.winner_id === 1) {
      rank = 3;
    } else {
      rank = 4;
    }
  }
  if (!runner.string_buffers) {
    runner.string_buffers = {};
  }
  runner.string_buffers["STRING_BUFFER_3"] = String(rank);
  return rank;
};

export const currentLandmarkName = (runner: ScriptRunner): string => {
  const overworld = runner.overworld ?? null;
  const mapName = overworld?.current_map_name ?? null;
  let metadata = mapName ? getMapMetadataByName(mapName) : undefined;
  const state = runner.game_state;
  if (!metadata && state) {
    metadata = getMapMetadataByGroup(state.wram.current_map_group, state.wram.current_map_id);
  }
  if (!metadata) {
    return "UNKNOWN";
  }
  return String(metadata.name ?? "").replace(/_/g, " ").toUpperCase();
};

export const consumeScriptChoice = (
  runner: ScriptRunner,
  key: string,
  defaultValue: number | null = null,
): number | null => {
  if (!runner.variables || !(key in runner.variables)) {
    return defaultValue;
  }
  const value = runner.variables[key];
  if (Array.isArray(value)) {
    if (!value.length) {
      delete runner.variables[key];
      return defaultValue;
    }
    const choice = value.shift();
    if (!value.length) {
      delete runner.variables[key];
    }
    return choice ?? defaultValue;
  }
  delete runner.variables[key];
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : defaultValue;
};

const RADIO_LINE_DELAY_FRAMES = 45;
const POKEMON_CHANNEL_TRANSCRIPT = [
  "PlayersRadioText1",
  "PlayersRadioText2",
  "PlayersRadioText3",
  "PlayersRadioText4",
];
// ASM mapping: engine/pokegear/radio.asm::LuckyNumberShow1..LuckyNumberShow13.
const LUCKY_CHANNEL_TRANSCRIPT = [
  "LC_Text1",
  "LC_Text2",
  "LC_Text3",
  "LC_Text4",
  "LC_Text5",
  "LC_Text6",
  "LC_Text7",
  "LC_Text8",
  "LC_Text9",
  "LC_Text7",
  "LC_Text8",
  "LC_Text10",
  "LC_Text11",
];
const MAP_RADIO_TRANSCRIPTS: Record<string, string[]> = {
  MAPRADIO_POKEMON_CHANNEL: POKEMON_CHANNEL_TRANSCRIPT,
  OAKS_POKEMON_TALK: POKEMON_CHANNEL_TRANSCRIPT,
  POKEDEX_SHOW: POKEMON_CHANNEL_TRANSCRIPT,
  LUCKY_CHANNEL: LUCKY_CHANNEL_TRANSCRIPT,
  PLACES_AND_PEOPLE: POKEMON_CHANNEL_TRANSCRIPT,
};

type PokegearInternalStateMachine = {
  resolveStation?: (handler: string) => string | null;
  stationInJohto?: () => boolean;
};

const resolveMapRadioTranscript = (
  runner: ScriptRunner,
  varValue: string,
): [string | null, string[]] => {
  const stateMachine = new PokegearStateMachine(
    runner.game_state,
    runner,
  );
  const normalized = varValue.toUpperCase();
  let station: string | null = null;
  const resolverCandidate = (stateMachine as unknown as PokegearInternalStateMachine).resolveStation;
  const stationInJohtoCandidate = (stateMachine as unknown as PokegearInternalStateMachine).stationInJohto;
  const resolver = typeof resolverCandidate === "function" ? resolverCandidate.bind(stateMachine) : null;
  const stationInJohto =
    typeof stationInJohtoCandidate === "function" ? stationInJohtoCandidate.bind(stateMachine) : null;

  if (normalized === "MAPRADIO_POKEMON_CHANNEL") {
    // ASM: engine/pokegear/pokegear.asm::LoadStation_PokemonChannel sends Kanto
    // map radio to Places & People instead of using the standard tuner handler.
    if (stationInJohto && !stationInJohto()) {
      station = "PLACES_AND_PEOPLE";
    } else {
      station = resolver ? resolver("PKMNTalkAndPokedexShow") : null;
    }
  } else if (normalized === "MAPRADIO_POKEMON_MUSIC") {
    station = stationInJohto ? (stationInJohto() ? "POKEMON_MUSIC" : null) : null;
  } else if (normalized === "MAPRADIO_LUCKY_CHANNEL") {
    station = resolver ? resolver("LuckyChannel") : null;
  } else if (normalized === "MAPRADIO_UNOWN") {
    station = resolver ? resolver("RuinsOfAlphRadio") : null;
  } else if (normalized === "MAPRADIO_PLACES_PEOPLE") {
    station = resolver ? resolver("PlacesAndPeople") : null;
  } else if (normalized === "MAPRADIO_LETS_ALL_SING") {
    station = resolver ? resolver("LetsAllSing") : null;
  } else if (normalized === "MAPRADIO_ROCKET") {
    station = "ROCKET_RADIO";
  } else if (normalized === "MAPRADIO_POKEDEX_SHOW") {
    station = "POKEDEX_SHOW";
  } else if (normalized === "MAPRADIO_OAKS_POKEMON_TALK") {
    station = "OAKS_POKEMON_TALK";
  } else {
    station = normalized;
  }

  let transcript = MAP_RADIO_TRANSCRIPTS[station ?? ""] ?? [];
  if (!transcript.length) {
    transcript = MAP_RADIO_TRANSCRIPTS[normalized] ?? [];
  }
  return [station, transcript];
};

export const dispatchRadioChannel = (
  runner: ScriptRunner,
  channel: string,
  varValue: string,
): void => {
  let [station, transcript] = resolveMapRadioTranscript(runner, varValue);
  if (!transcript.length) {
    console.warn(
      `Map radio channel '${channel}' (${varValue}) has no transcript mapping; defaulting to the Pokemon Channel transcript.`,
    );
    transcript = [...POKEMON_CHANNEL_TRANSCRIPT];
    station = station ?? varValue;
  }

  const eventManager = runner.event_manager;
  const durationFrames = Math.max(RADIO_LINE_DELAY_FRAMES * transcript.length, RADIO_LINE_DELAY_FRAMES);
  runner.variables["_value"] = varValue;

  if (eventManager) {
    eventManager.dispatch(
      new Event("play_radio_channel", {
        channel,
        station,
        duration_frames: durationFrames,
        source: "standard_script",
      }),
    );
  }

  const playedSegments: Array<{ label: string; message: string }> = [];
  if (!eventManager) {
    for (const label of transcript) {
      const message = resolveText(runner, label);
      playedSegments.push({ label, message });
    }
    runner.last_value = {
      radio: {
        channel,
        value: varValue,
        station: station ?? varValue,
        transcript: playedSegments,
      },
    };
    return;
  }

  const fastForwardEvents = !runner.overworld;
  const timelineEventManager = eventManager as TimelineEventManager;
  const currentFrame = timelineEventManager._current_frame ?? 0;
  const targetFrame = currentFrame + RADIO_LINE_DELAY_FRAMES * Math.max(transcript.length - 1, 0);

  transcript.forEach((label, offset) => {
    const message = resolveText(runner, label);
    if (!message) {
      return;
    }
    playedSegments.push({ label, message });
    const event = new Event("show_text", { text: message });
    if (offset > 0) {
      eventManager.dispatch(event, { delay: offset * RADIO_LINE_DELAY_FRAMES });
    } else {
      eventManager.dispatch(event);
    }
  });

  if (fastForwardEvents && transcript.length) {
    const advance = eventManager.advance_frame?.bind(eventManager);
    const process = eventManager.process_pending_events?.bind(eventManager);
    if (advance && process) {
      while ((timelineEventManager._current_frame ?? 0) < targetFrame) {
        advance();
      }
      process();
    }
  }

  runner.last_value = {
    radio: {
      channel,
      value: varValue,
      station: station ?? varValue,
      transcript: playedSegments,
    },
  };
};

type TimelineEventManager = EventManager & {
  _current_frame?: number;
};

export const playerHasCoinCase = (runner: ScriptRunner): boolean => {
  const state = runner.game_state;
  if (!state) {
    return false;
  }
  return (state.sram.key_items?.COIN_CASE ?? 0) > 0;
};

export const attemptCoinPurchase = (
  runner: ScriptRunner,
  coins: number,
  price: number,
): Record<string, unknown> => {
  const summary: Record<string, unknown> = { coins, price, status: "ok" };
  const state = runner.game_state;
  if (!state) {
    return summary;
  }
  const maxCoins = 9999;
  if (state.sram.coins + coins > maxCoins) {
    showLabelledText(runner, "CoinVendor_CoinCaseFullText");
    summary.status = "coin_case_full";
    return summary;
  }
  if (state.sram.money < price) {
    showLabelledText(runner, "CoinVendor_NotEnoughMoneyText");
    summary.status = "insufficient_funds";
    return summary;
  }
  state.sram.coins += coins;
  state.sram.money -= price;
  const label = coins === 50 ? "CoinVendor_Buy50CoinsText" : "CoinVendor_Buy500CoinsText";
  showLabelledText(runner, label);
  return summary;
};

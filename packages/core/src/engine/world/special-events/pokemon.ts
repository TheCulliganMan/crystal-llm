import { GameState } from "@pokecrystal/core/core/state";
import { getPokedexFlag, recordPokedexCaught } from "@pokecrystal/core/core/pokedex";
import { Event } from "@pokecrystal/core/engine/events/events";
import { ScriptRunner, ensureRunnerVariables } from "./utils";
import type { Pokemon } from "@pokecrystal/core/core/models";
import { toPokemon } from "@pokecrystal/core/core/models/pokemon";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import type { Overworld as OverworldType } from "@pokecrystal/core/types/overworld";

type SpeciesToken = { id?: string; int_id?: number } | string | number;

type DataLoader = {
  get_pokemon_species?: (id: string) => { id: string; int_id?: number };
  getPokemonSpecies?: (id: string) => { id: string; int_id?: number };
  getSpecies?: (id: string) => { id: string; int_id?: number };
};

type SpecialEventContext = {
  runner?: ScriptRunner;
  overworld?: OverworldType | null;
  event_manager?: EventManager;
};

const getTokenSpeciesId = (token: SpeciesToken): string => {
  if (typeof token === "object" && token !== null) {
    if (typeof token.id === "string" && token.id.trim()) {
      return token.id;
    }
    if (typeof token.int_id === "number") {
      return String(token.int_id);
    }
    return "";
  }
  if (token === null || token === undefined) {
    return "";
  }
  return String(token);
};

const resolveSpeciesId = (
  speciesToken: SpeciesToken,
  dataLoader?: DataLoader | null
): { speciesId: string; numericId: number | null } => {
  let speciesId = getTokenSpeciesId(speciesToken).toUpperCase();
  let numericId: number | null = null;
  if (dataLoader) {
    const lookup =
      dataLoader.get_pokemon_species ??
      dataLoader.getPokemonSpecies ??
      dataLoader.getSpecies;
    if (lookup) {
      const species = lookup.call(dataLoader, speciesId);
      if (!species) {
        throw new Error(`Unknown species '${speciesId}'`);
      }
      speciesId = species.id;
      numericId = species.int_id ?? null;
    }
  }
  return { speciesId, numericId };
};

const requireSpeciesToken = (value: unknown, label: string): SpeciesToken => {
  if (typeof value === "string" || typeof value === "number") {
    return value;
  }
  if (typeof value === "object" && value !== null) {
    const token = value as { id?: unknown; int_id?: unknown };
    if (typeof token.id === "string" || typeof token.int_id === "number") {
      return token as { id?: string; int_id?: number };
    }
  }
  throw new Error(`${label} requires a species identifier via setval.`);
};

const ensureStringBuffers = (runner?: ScriptRunner | null): Record<string, string> => {
  if (!runner) {
    return {};
  }
  if (!runner.string_buffers) {
    runner.string_buffers = {};
  }
  return runner.string_buffers;
};

const partyMembers = (gameState: GameState): Array<Pokemon | null> => {
  const members = gameState.sram.party?.pokemon ?? [];
  let partyCount = gameState.wram.wPartyCount ?? members.length;
  if (!partyCount) {
    partyCount = members.length;
  }
  return members.slice(0, partyCount).map((mon) => mon ? toPokemon(mon) : null);
};

const partyMatchesSpecies = (members: Array<Pokemon | null>, speciesId: string): boolean => {
  for (const mon of members) {
    if (!mon) {
      continue;
    }
    const monSpecies = String(mon.species?.id ?? "").toUpperCase();
    if (monSpecies === speciesId) {
      return true;
    }
  }
  return false;
};

const partyMatchesSpeciesOt = (
  members: Array<Pokemon | null>,
  speciesId: string,
  playerName: string,
  playerId: number
): boolean => {
  for (const mon of members) {
    if (!mon) {
      continue;
    }
    const monSpecies = String(mon.species?.id ?? "").toUpperCase();
    if (monSpecies !== speciesId) {
      continue;
    }
    if (Number(mon.original_trainer_id ?? -1) !== playerId) {
      continue;
    }
    const otName = String(mon.original_trainer_name ?? "").toUpperCase().trim();
    if (otName === playerName) {
      return true;
    }
  }
  return false;
};

export function get_first_pokemon_happiness(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: SpecialEventContext = {}
): number {
  // ASM: engine/events/std_scripts.asm::GetFirstPokemonHappiness
  void overworld;
  void event_manager;

  const buffers = ensureStringBuffers(runner);
  const members = partyMembers(game_state);
  const target = members.find(
    (mon) => mon && String(mon.species?.id ?? "").toUpperCase() !== "EGG"
  );
  if (!target) {
    throw new Error("GetFirstPokemonHappiness requires at least one party Pokemon");
  }

  const nickname = (target.nickname ?? "").trim() || target.species?.id || "";
  buffers.STRING_BUFFER_3 = nickname;
  const happiness = Number(target.happiness ?? 0);

  game_state.wram.wCurPartySpecies = String(target.species?.id ?? "").toUpperCase();
  if (runner) {
    runner.last_value = happiness;
    const variables = ensureRunnerVariables(runner);
    variables._value = happiness;
  }
  return happiness;
}

export function check_first_mon_is_egg(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: SpecialEventContext = {}
): boolean {
  // ASM: engine/events/std_scripts.asm::CheckFirstMonIsEgg
  void overworld;
  void event_manager;

  const buffers = ensureStringBuffers(runner);
  const members = partyMembers(game_state);
  if (!members.length || !members[0]) {
    throw new Error("CheckFirstMonIsEgg requires a populated party slot 0");
  }
  const lead = members[0];
  const speciesId = String(lead.species?.id ?? "").toUpperCase();
  const isEgg = speciesId === "EGG";
  const nickname = (lead.nickname ?? "").trim() || speciesId;
  buffers.STRING_BUFFER_3 = nickname;
  game_state.wram.wCurPartySpecies = speciesId;

  if (runner) {
    const value = isEgg ? 1 : 0;
    runner.last_value = value;
    const variables = ensureRunnerVariables(runner);
    variables._value = value;
    runner.last_condition_result = isEgg;
  }
  return isEgg;
}

export function check_caught_celebi(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: SpecialEventContext = {}
): boolean {
  // ASM: engine/events/celebi.asm::CheckCaughtCelebi
  void overworld;
  void event_manager;

  const result = Number(game_state.wram.battle_result ?? 0);
  const caught = (result & (1 << 6)) !== 0;
  if (runner) {
    runner.last_condition_result = caught;
    const value = caught ? 1 : 0;
    runner.last_value = value;
    const variables = ensureRunnerVariables(runner);
    variables._value = value;
  }
  return caught;
}

export function find_party_mon_that_species(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: SpecialEventContext = {}
): boolean {
  // ASM: engine/events/specials.asm::FindPartyMonThatSpecies
  void overworld;
  void event_manager;
  if (!runner?.variables || !("_value" in runner.variables)) {
    throw new Error("FindPartyMonThatSpecies requires a species identifier via setval.");
  }
  const dataLoader = (runner.data_loader ?? runner.dataLoader) as DataLoader | undefined;
  const token = requireSpeciesToken(runner.variables._value, "FindPartyMonThatSpecies");
  const { speciesId } = resolveSpeciesId(token, dataLoader);
  const found = partyMatchesSpecies(partyMembers(game_state), speciesId);
  if (runner) {
    const value = found ? 1 : 0;
    runner.last_value = value;
    const variables = ensureRunnerVariables(runner);
    variables._value = value;
    runner.last_condition_result = found;
  }
  return found;
}

export function find_party_mon_that_species_your_trainer_id(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: SpecialEventContext = {}
): boolean {
  // ASM: engine/events/specials.asm::FindPartyMonThatSpeciesYourTrainerID
  void overworld;
  void event_manager;
  if (!runner?.variables || !("_value" in runner.variables)) {
    throw new Error("FindPartyMonThatSpeciesYourTrainerID requires a species identifier via setval.");
  }
  const dataLoader = (runner.data_loader ?? runner.dataLoader) as DataLoader | undefined;
  const token = requireSpeciesToken(
    runner.variables._value,
    "FindPartyMonThatSpeciesYourTrainerID"
  );
  const { speciesId } = resolveSpeciesId(token, dataLoader);
  const playerName = String(game_state.sram.player_name ?? "").toUpperCase().trim();
  const playerId = Number(game_state.sram.player_id ?? 0);
  const found = partyMatchesSpeciesOt(partyMembers(game_state), speciesId, playerName, playerId);
  if (runner) {
    const value = found ? 1 : 0;
    runner.last_value = value;
    const variables = ensureRunnerVariables(runner);
    variables._value = value;
    runner.last_condition_result = found;
  }
  return found;
}

export function activate_fishing_swarm(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: SpecialEventContext = {}
): number {
  // ASM: constants/script_constants.asm::ActivateFishingSwarm
  void overworld;
  void event_manager;

  let value: unknown = 0;
  if (runner) {
    value = runner.variables?._value ?? runner.last_value ?? 0;
  }
  if (value === null || value === undefined) {
    value = 0;
  }
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    throw new Error(`Invalid fishing swarm value '${value}'`);
  }
  game_state.wram.wFishingSwarmFlag = numeric & 0xff;
  if (runner) {
    runner.last_value = numeric;
    const variables = ensureRunnerVariables(runner);
    variables._value = numeric;
    runner.last_condition_result = true;
  }
  return numeric;
}

const ownsSpeciesWithOt = (
  game_state: GameState,
  speciesId: string,
  playerName: string,
  playerId: number
): boolean => {
  const matches = (mon: Pokemon | null | undefined): boolean => {
    if (!mon?.species) {
      return false;
    }
    const monSpecies = String(mon.species.id ?? "").toUpperCase();
    if (monSpecies !== speciesId) {
      return false;
    }
    if (Number(mon.original_trainer_id ?? -1) !== playerId) {
      return false;
    }
    const otName = String(mon.original_trainer_name ?? "").toUpperCase().trim();
    return otName === playerName;
  };

  for (const mon of game_state.sram.party?.pokemon ?? []) {
    if (mon && matches(toPokemon(mon))) {
      return true;
    }
  }

  for (const box of game_state.sram.pc_boxes ?? []) {
    for (const mon of box?.pokemon ?? []) {
      if (mon && matches(toPokemon(mon))) {
        return true;
      }
    }
  }

  return false;
};

export function beasts_check(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: SpecialEventContext = {}
): boolean {
  // ASM: engine/events/specials.asm::BeastsCheck
  void overworld;
  void event_manager;

  const dataLoader = (runner?.data_loader ?? runner?.dataLoader) as DataLoader | undefined;
  const playerName = String(game_state.sram.player_name ?? "").toUpperCase().trim();
  const playerId = Number(game_state.sram.player_id ?? 0);

  for (const token of ["RAIKOU", "ENTEI", "SUICUNE"]) {
    const { speciesId } = resolveSpeciesId(token, dataLoader);
    if (!ownsSpeciesWithOt(game_state, speciesId, playerName, playerId)) {
      if (runner) {
        runner.last_value = 0;
        const variables = ensureRunnerVariables(runner);
        variables._value = 0;
        runner.last_condition_result = false;
      }
      return false;
    }
  }

  if (runner) {
    runner.last_value = 1;
    const variables = ensureRunnerVariables(runner);
    variables._value = 1;
    runner.last_condition_result = true;
  }
  return true;
}

export function mon_check(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: SpecialEventContext = {}
): boolean {
  // ASM: engine/events/specials.asm::MonCheck
  void overworld;
  void event_manager;

  if (!runner?.variables || !("_value" in runner.variables)) {
    throw new Error("MonCheck requires a species identifier via setval.");
  }

  const dataLoader = (runner.data_loader ?? runner.dataLoader) as DataLoader | undefined;
  const token = requireSpeciesToken(runner.variables._value, "MonCheck");
  const { speciesId } = resolveSpeciesId(token, dataLoader);
  const playerName = String(game_state.sram.player_name ?? "").toUpperCase().trim();
  const playerId = Number(game_state.sram.player_id ?? 0);

  const owned = ownsSpeciesWithOt(game_state, speciesId, playerName, playerId);
  if (runner) {
    const value = owned ? 1 : 0;
    runner.last_value = value;
    const variables = ensureRunnerVariables(runner);
    variables._value = value;
    runner.last_condition_result = owned;
  }
  return owned;
}

export function game_corner_prize_mon_check_dex(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: SpecialEventContext = {}
): boolean {
  // ASM: engine/events/specials.asm::GameCornerPrizeMonCheckDex
  void overworld;
  void event_manager;
  if (!runner?.variables || !("_value" in runner.variables)) {
    throw new Error("GameCornerPrizeMonCheckDex requires a species identifier via setval.");
  }
  const dataLoader = (runner.data_loader ?? runner.dataLoader) as DataLoader | undefined;
  const token = requireSpeciesToken(runner.variables._value, "GameCornerPrizeMonCheckDex");
  const resolved = resolveSpeciesId(token, dataLoader);
  const numeric =
    resolved.numericId ??
    (typeof token === "number"
      ? token
      : typeof token === "object" && token !== null && typeof token.int_id === "number"
        ? token.int_id
        : Number.parseInt(String(resolved.speciesId), 10));
  if (!Number.isFinite(numeric)) {
    throw new Error(`Unable to resolve numeric species id for ${resolved.speciesId}`);
  }
  const speciesId = Number(numeric);
  const alreadyCaught = getPokedexFlag(game_state, speciesId, "owned");
  if (alreadyCaught) {
    if (runner) {
      runner.last_condition_result = false;
      runner.last_value = resolved.speciesId;
    }
    return false;
  }
  game_state.wram.wCurPartySpecies = resolved.speciesId;
  game_state.wram.wNamedObjectIndex = speciesId;
  recordPokedexCaught(game_state, { int_id: speciesId });
  const manager = event_manager ?? runner?.event_manager ?? null;
  manager?.dispatch(
    new Event("show_pokedex_entry", {
      species_id: resolved.speciesId,
      pokedex_number: speciesId,
      source: "game_corner_prize",
    })
  );
  if (runner) {
    runner.last_condition_result = true;
    runner.last_value = resolved.speciesId;
  }
  return true;
}

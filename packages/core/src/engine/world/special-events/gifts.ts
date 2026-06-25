import { GameState } from "@pokecrystal/core/core/state";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { MoveName } from "@pokecrystal/core/core/enums";
import { moves } from "@pokecrystal/assets/content/moves";
import { createPokemon } from "@pokecrystal/core/engine/systems/pokemon";
import { addPokemon, getFilledSlots } from "@pokecrystal/core/core/models/party";
import { LearnedMove, Pokemon, PokemonSpecies, toPokemon } from "@pokecrystal/core/core/models";
import { ensureRunnerVariables, ScriptRunner } from "./utils";
import type { Overworld } from "@pokecrystal/core/types/overworld";
import type { EventManager } from "@pokecrystal/core/engine/world/events";

type PokemonSpeciesCollection =
  | Map<string, PokemonSpecies>
  | Record<string, PokemonSpecies | undefined>
  | { get: (id: string) => PokemonSpecies | undefined };

type PokemonSpeciesLookup = {
  get_pokemon_species?: (id: string) => PokemonSpecies | null | undefined;
  getPokemonSpecies?: (id: string) => PokemonSpecies | null | undefined;
  getSpecies?: (id: string) => PokemonSpecies | null | undefined;
  pokemonData?: PokemonSpeciesCollection;
  pokemon_data?: PokemonSpeciesCollection;
} & Partial<DataLoader>;

const MANIA_OT_ID = 0x0518;
const SHUCKIE_NICKNAME = "SHUCKIE";
const SHUCKIE_OT = "MANIA";

export const SHUCKIE_WRONG_MON = 0;
export const SHUCKIE_REFUSED = 1;
export const SHUCKIE_RETURNED = 2;
export const SHUCKIE_HAPPY = 3;
export const SHUCKIE_FAINTED = 4;

const DRATINI_GIFT_MOVES = ["WRAP", "THUNDER_WAVE", "TWISTER", "EXTREMESPEED"] as const;
const DRATINI_NORMAL_MOVES = ["WRAP", "LEER", "THUNDER_WAVE", "TWISTER"] as const;

type OverworldDataLoaderCarrier = {
  data_loader?: DataLoader | null;
  dataLoader?: DataLoader | null;
};

const resolveOverworldDataLoader = (overworld?: Overworld | null): DataLoader | null | undefined => {
  if (!overworld || typeof overworld !== "object") {
    return undefined;
  }
  const carrier = overworld as OverworldDataLoaderCarrier;
  return carrier.data_loader ?? carrier.dataLoader;
};

const ensureDataLoader = (runner?: ScriptRunner | null, overworld?: Overworld | null): DataLoader => {
  let loader = runner?.data_loader ?? runner?.dataLoader ?? resolveOverworldDataLoader(overworld);
  if (!loader) {
    loader = new DataLoader();
  }
  return loader as DataLoader;
};

const resolveSpecies = (loader: PokemonSpeciesLookup | null | undefined, speciesId: string): PokemonSpecies | null => {
  const upper = speciesId.toUpperCase();
  const lookup = loader?.get_pokemon_species ?? loader?.getPokemonSpecies ?? loader?.getSpecies;
  if (typeof lookup === "function") {
    return lookup.call(loader, upper) ?? null;
  }
  const collection = loader?.pokemonData ?? loader?.pokemon_data;
  if (collection) {
    if (typeof (collection as { get?: unknown }).get === "function") {
      return (collection as { get: (id: string) => PokemonSpecies | undefined }).get(upper) ?? null;
    }
    return (collection as Record<string, PokemonSpecies | undefined>)[upper] ?? null;
  }
  return null;
};

const partyMembers = (game_state: GameState): [Array<Pokemon | null>, number] => {
  const members = (game_state.sram.party?.pokemon ?? []).map((mon) => (mon ? toPokemon(mon) : null));
  const partyCount = Number(game_state.wram.wPartyCount ?? members.length) || members.length;
  return [members, partyCount];
};

const removePartyMember = (game_state: GameState, index: number): void => {
  const party = game_state.sram.party.pokemon;
  if (index < 0 || index >= party.length) {
    throw new Error(`Party index ${index} out of range.`);
  }
  for (let slot = index; slot < party.length - 1; slot++) {
    party[slot] = party[slot + 1];
  }
  party[party.length - 1] = null;
  game_state.wram.wPartyCount = getFilledSlots(game_state.sram.party);
};

const isManiaShuckie = (mon: Pokemon | null | undefined): boolean => {
  if (!mon || !mon.species) {
    return false;
  }
  const speciesId = String(mon.species.id ?? "").toUpperCase();
  if (speciesId !== "SHUCKLE") {
    return false;
  }
  if (Number(mon.original_trainer_id ?? -1) !== MANIA_OT_ID) {
    return false;
  }
  const otName = String(mon.original_trainer_name ?? "").trim().toUpperCase();
  return otName === SHUCKIE_OT;
};

export function give_shuckle(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): number {
  // ASM: data/events/special_pointers.asm::GiveShuckle
  void event_manager;

  const dataLoader = ensureDataLoader(runner, overworld);
  const species = resolveSpecies(dataLoader, "SHUCKLE");
  if (!species) {
    throw new Error("Could not resolve SHUCKLE species data.");
  }

  const shuckle = createPokemon(game_state, species, 15);
  shuckle.item = "BERRY";
  shuckle.nickname = SHUCKIE_NICKNAME;
  shuckle.original_trainer_name = SHUCKIE_OT;
  shuckle.original_trainer_id = MANIA_OT_ID;

  if (!addPokemon(game_state.sram.party, shuckle)) {
    if (runner) {
      runner.last_condition_result = false;
      runner.last_value = 0;
      ensureRunnerVariables(runner)._value = 0;
    }
    return 0;
  }

  game_state.wram.wPartyCount = getFilledSlots(game_state.sram.party);
  game_state.wram.wCurPartySpecies = "SHUCKLE";
  const flags = game_state.wram.engine_flags as Record<string, boolean>;
  flags["ENGINE_GOT_SHUCKIE_TODAY"] = true;

  if (runner) {
    runner.last_condition_result = true;
    runner.last_value = 1;
    ensureRunnerVariables(runner)._value = 1;
  }
  return 1;
}

export function return_shuckie(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): number {
  // ASM: data/events/special_pointers.asm::ReturnShuckie
  void overworld;
  void event_manager;

  const variables = runner ? ensureRunnerVariables(runner) : {};
  let result: number;

  if (runner && variables._selection_cancelled) {
    result = SHUCKIE_REFUSED;
  } else {
    const [members, partyCount] = partyMembers(game_state);
    const rawSelection = variables._selected_party_index ?? game_state.wram.wCurPartyMon ?? 0;
    let index = Number(rawSelection);
    if (!Number.isFinite(index)) {
      index = 0;
    }

    if (index < 0 || index >= partyCount || !members[index]) {
      result = SHUCKIE_REFUSED;
    } else {
      const selection = toPokemon(members[index] as Pokemon);
      game_state.wram.wCurPartySpecies = String(selection.species?.id ?? "").toUpperCase();

      if (!isManiaShuckie(selection)) {
        result = SHUCKIE_WRONG_MON;
      } else if ((selection.hp ?? 0) <= 0) {
        result = SHUCKIE_FAINTED;
      } else if ((selection.happiness ?? 0) >= 150) {
        result = SHUCKIE_HAPPY;
      } else {
        removePartyMember(game_state, index);
        result = SHUCKIE_RETURNED;
      }
    }
  }

  if (runner) {
    runner.last_value = result;
    runner.last_condition_result = result === SHUCKIE_HAPPY || result === SHUCKIE_RETURNED;
    variables._value = result;
  }
  return result;
}

const buildMoveset = (moveNames: readonly string[]): LearnedMove[] => {
  const learned: LearnedMove[] = [];
  for (const name of moveNames) {
    const upper = name.toUpperCase();
    if (!Object.prototype.hasOwnProperty.call(MoveName, upper)) {
      throw new Error(`Unknown move '${upper}'`);
    }
    const key = MoveName[upper as keyof typeof MoveName];
    const metadata = moves[key];
    if (!metadata) {
      throw new Error(`Unknown move '${upper}'`);
    }
    learned.push({ name: key, current_pp: metadata.pp ?? 0, pp_ups: 0 });
  }
  return learned;
};

export function give_dratini(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): boolean {
  // ASM: data/events/special_pointers.asm::GiveDratini
  void overworld;
  void event_manager;

  const variables = runner ? ensureRunnerVariables(runner) : {};
  const rawValue = variables._value ?? runner?.last_value ?? 0;
  const mode = Number(rawValue);
  if (!Number.isFinite(mode)) {
    throw new Error("GiveDratini expects numeric script accumulator values");
  }

  if (mode >= 2 || mode < 0) {
    if (runner) {
      runner.last_value = mode;
      runner.last_condition_result = false;
    }
    return false;
  }

  const [members, partyCount] = partyMembers(game_state);
  let targetIndex: number | null = null;
  for (let offset = 0; offset < partyCount; offset++) {
    const index = partyCount - 1 - offset;
    const mon = members[index];
    if (!mon || !mon.species) {
      continue;
    }
    if (String(mon.species.id ?? "").toUpperCase() === "DRATINI") {
      targetIndex = index;
      break;
    }
  }

  if (targetIndex === null) {
    if (runner) {
      runner.last_value = mode;
      runner.last_condition_result = false;
    }
    return false;
  }

  const movePool = mode === 0 ? DRATINI_GIFT_MOVES : DRATINI_NORMAL_MOVES;
  const targetMon = members[targetIndex] as Pokemon;
  targetMon.moves = buildMoveset(movePool);
  game_state.wram.wCurPartySpecies = "DRATINI";

  if (runner) {
    runner.last_value = mode;
    variables._value = mode;
    runner.last_condition_result = true;
  }
  return true;
}

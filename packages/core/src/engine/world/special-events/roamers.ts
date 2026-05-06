import { GameState } from "@pokecrystal/core/core/state";
import { getMapMetadataByConstant } from "@pokecrystal/core/engine/world/maps";
import type { ScriptRunner } from "./utils";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import type { EventManager } from "@pokecrystal/core/engine/events/events";

type DataLoader = {
  get_pokemon_species?: (id: string) => { id: string };
  getPokemonSpecies?: (id: string) => { id: string };
  getSpecies?: (id: string) => { id: string };
};

const ROAMER_DATA: Array<[string, string]> = [
  ["RAIKOU", "ROUTE_42"],
  ["ENTEI", "ROUTE_37"],
];

const ROAMER_LEVEL = 40;

type SpeciesToken = { id?: string } | string | number;

const resolveSpeciesId = (token: SpeciesToken, dataLoader?: DataLoader | null): string => {
  const resolvedToken =
    token && typeof token === "object" && "id" in token ? String(token.id ?? "") : String(token);
  const lookup =
    dataLoader?.get_pokemon_species ??
    dataLoader?.getPokemonSpecies ??
    dataLoader?.getSpecies;
  if (lookup) {
    const normalized = String(resolvedToken).toUpperCase();
    const species = lookup.call(dataLoader, normalized);
    if (!species) {
      throw new Error(`Unknown species '${normalized}'.`);
    }
    return String(species.id).toUpperCase();
  }
  return String(resolvedToken).toUpperCase();
};

type RoamingPokemonSlot = {
  species?: string;
  level?: number;
  map_group?: number;
  map_number?: number;
  hp?: number;
  dvs?: number;
};

export function init_roam_mons(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: OverworldEngine; event_manager?: EventManager } = {},
): boolean {
  // ASM: engine/events/roamers.asm::InitRoamMons
  void event_manager;

  const dataLoader = (
    runner?.data_loader ??
    runner?.dataLoader ??
    overworld?.data_loader ??
    overworld?.dataLoader ??
    undefined
  ) as DataLoader | undefined;

  const roamers = (game_state.wram.roaming_pokemon ?? []) as RoamingPokemonSlot[];
  if (roamers.length < ROAMER_DATA.length) {
    throw new Error(
      `Expected at least ${ROAMER_DATA.length} roaming slots, found ${roamers.length}.`
    );
  }

  ROAMER_DATA.forEach(([speciesId, mapConstant], index) => {
    const species = resolveSpeciesId(speciesId, dataLoader);
    const metadata = getMapMetadataByConstant(mapConstant);
    if (!metadata) {
      throw new Error(`Unknown map constant '${mapConstant}'.`);
    }
    const roamer = roamers[index];
    if (!roamer) {
      throw new Error(`Missing roaming slot at index ${index}.`);
    }
    roamer.species = species;
    roamer.level = ROAMER_LEVEL;
    roamer.map_group = metadata.groupId;
    roamer.map_number = metadata.mapId;
    roamer.hp = 0;
    roamer.dvs = 0;
  });

  if (runner) {
    runner.last_condition_result = true;
    runner.last_value = true;
  }
  return true;
}

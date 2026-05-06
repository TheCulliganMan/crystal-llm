import { Pokemon, RoamingPokemon } from "@pokecrystal/core/core/models";
import type { DV, PokemonSpecies } from "@pokecrystal/core/core/models";
import { GameState } from "@pokecrystal/core/core/state";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { getMapMetadataByConstant } from "@pokecrystal/core/engine/world/maps";
import { createPokemon } from "@pokecrystal/core/engine/systems/pokemon";
import { normalizeDvs } from "@pokecrystal/core/core/pokemon-dvs";

export const GROUP_N_A = -1;
export const MAP_N_A = -1;
const NUM_ROAMMON_MAPS = 16;

const ROAM_MAP_CONSTANTS: Array<[string, string[]]> = [
  ["ROUTE_29", ["ROUTE_30", "ROUTE_46"]],
  ["ROUTE_30", ["ROUTE_29", "ROUTE_31"]],
  ["ROUTE_31", ["ROUTE_30", "ROUTE_32", "ROUTE_36"]],
  ["ROUTE_32", ["ROUTE_36", "ROUTE_31", "ROUTE_33"]],
  ["ROUTE_33", ["ROUTE_32", "ROUTE_34"]],
  ["ROUTE_34", ["ROUTE_33", "ROUTE_35"]],
  ["ROUTE_35", ["ROUTE_34", "ROUTE_36"]],
  ["ROUTE_36", ["ROUTE_35", "ROUTE_31", "ROUTE_32", "ROUTE_37"]],
  ["ROUTE_37", ["ROUTE_36", "ROUTE_38", "ROUTE_42"]],
  ["ROUTE_38", ["ROUTE_37", "ROUTE_39", "ROUTE_42"]],
  ["ROUTE_39", ["ROUTE_38"]],
  ["ROUTE_42", ["ROUTE_43", "ROUTE_44", "ROUTE_37", "ROUTE_38"]],
  ["ROUTE_43", ["ROUTE_42", "ROUTE_44"]],
  ["ROUTE_44", ["ROUTE_42", "ROUTE_43", "ROUTE_45"]],
  ["ROUTE_45", ["ROUTE_44", "ROUTE_46"]],
  ["ROUTE_46", ["ROUTE_45", "ROUTE_29"]],
];

type RoamMapEntry = {
  origin: [number, number];
  connections: Array<[number, number]>;
};

type RandomSource = {
  nextByte: () => number;
  randrange: (upperBound: number) => number;
};

const resolveMapConstant = (constant: string): [number, number] => {
  const metadata = getMapMetadataByConstant(constant);
  if (!metadata) {
    throw new Error(`Unknown map constant '${constant}'.`);
  }
  return [metadata.groupId, metadata.mapId];
};

let roamEntryCache: RoamMapEntry[] | null = null;
let roamEntryMap: Map<string, RoamMapEntry> | null = null;

export type RoamerState = RoamingPokemon & {
  map_group?: number;
  map_number?: number;
  hp?: number;
  update_location?: (group: number, number: number) => void;
  set_remaining_hp?: (value: number) => void;
};

const getRoamers = (gameState: GameState): RoamerState[] =>
  (gameState.wram.roaming_pokemon ?? []) as RoamerState[];

const loadRoamEntries = (): RoamMapEntry[] => {
  if (roamEntryCache) {
    return roamEntryCache;
  }
  const entries: RoamMapEntry[] = [];
  ROAM_MAP_CONSTANTS.forEach(([originConstant, connections]) => {
    const origin = resolveMapConstant(originConstant);
    const neighbors = connections.map((name) => resolveMapConstant(name));
    entries.push({ origin, connections: neighbors });
  });
  if (entries.length !== NUM_ROAMMON_MAPS) {
    throw new Error(
      `Expected ${NUM_ROAMMON_MAPS} roaming map entries, found ${entries.length}.`
    );
  }
  roamEntryCache = entries;
  return entries;
};

const entriesByOrigin = (): Map<string, RoamMapEntry> => {
  if (roamEntryMap) {
    return roamEntryMap;
  }
  const mapping = new Map<string, RoamMapEntry>();
  loadRoamEntries().forEach((entry) => {
    mapping.set(`${entry.origin[0]},${entry.origin[1]}`, entry);
  });
  roamEntryMap = mapping;
  return mapping;
};

const ensureRng = (gameState: GameState, rng?: RandomSource): RandomSource => {
  return rng ?? new HardwareRNG(gameState);
};

const locateRoamer = (roamers: RoamerState[], speciesId: string | null): RoamerState | null => {
  if (!speciesId) {
    return null;
  }
  for (const roamer of roamers) {
    if (roamer?.species === speciesId) {
      return roamer;
    }
  }
  return null;
};

const hpLowByte = (pokemon: Pokemon): number => {
  const hp = Math.max(0, Math.min(Number(pokemon.hp), 0xffff));
  return hp & 0xff;
};

export const encodeRoamerDvs = (dvs: Pick<DV, "attack" | "defense" | "speed" | "special">): number => {
  const normalized = normalizeDvs(dvs);
  const attackDefense = ((normalized.attack & 0x0f) << 4) | (normalized.defense & 0x0f);
  const speedSpecial = ((normalized.speed & 0x0f) << 4) | (normalized.special & 0x0f);
  return (attackDefense << 8) | speedSpecial;
};

export const decodeRoamerDvs = (value: number): DV => {
  const word = Math.trunc(value) & 0xffff;
  return normalizeDvs({
    attack: (word >> 12) & 0x0f,
    defense: (word >> 8) & 0x0f,
    speed: (word >> 4) & 0x0f,
    special: word & 0x0f,
  });
};

export function createRoamingBattlePokemon(
  gameState: GameState,
  species: PokemonSpecies,
  level: number
): Pokemon {
  const roamers = getRoamers(gameState);
  const roamer = locateRoamer(roamers, species.id);
  if (!roamer) {
    throw new Error(`Roaming Pokémon '${species.id}' missing from roaming state.`);
  }

  const storedHp = Math.trunc(roamer.hp ?? 0) & 0xff;
  const pokemon = storedHp > 0
    ? createPokemon(gameState, species, level, { dvs: decodeRoamerDvs(roamer.dvs ?? 0) })
    : createPokemon(gameState, species, level);

  if (storedHp > 0) {
    pokemon.hp = Math.min(pokemon.max_hp, storedHp);
  } else {
    roamer.dvs = encodeRoamerDvs(pokemon.dvs);
    roamer.hp = hpLowByte(pokemon);
  }

  return pokemon;
}

const updateRoamerLocation = (
  roamer: RoamerState | null | undefined,
  group: number,
  number: number
): void => {
  if (!roamer) {
    return;
  }
  if (typeof roamer.update_location === "function") {
    roamer.update_location(group, number);
    return;
  }
  roamer.map_group = group;
  roamer.map_number = number;
};

const getRoamerMapCoords = (roamer: RoamerState): [number, number] => [
  roamer.map_group ?? MAP_N_A,
  roamer.map_number ?? MAP_N_A,
];

const chooseRoamTarget = (
  gameState: GameState,
  rng: RandomSource,
  origin: [number, number]
): [number, number] | null => {
  const entry = entriesByOrigin().get(`${origin[0]},${origin[1]}`);
  if (!entry || !entry.connections.length) {
    return origin;
  }
  const lastGroup = gameState.wram.roam_mons_last_map_group;
  const lastNumber = gameState.wram.roam_mons_last_map_number;
  while (true) {
    const value = rng.nextByte() & 0x1f;
    if (value === 0) {
      return jumpToRandomMap(gameState, rng);
    }
    const index = value & 0x03;
    if (index >= entry.connections.length) {
      continue;
    }
    const candidate = entry.connections[index];
    if (candidate[0] === lastGroup && candidate[1] === lastNumber) {
      continue;
    }
    return candidate;
  }
};

const jumpToRandomMap = (
  gameState: GameState,
  rng: RandomSource
): [number, number] => {
  const entries = loadRoamEntries();
  const current: [number, number] = [
    gameState.wram.wMapGroup,
    gameState.wram.wMapNumber,
  ];
  while (true) {
    const index = rng.randrange(entries.length);
    const candidate = entries[index].origin;
    if (candidate[0] !== current[0] || candidate[1] !== current[1]) {
      return candidate;
    }
  }
};

const backupMapIndices = (gameState: GameState): void => {
  const wram = gameState.wram;
  wram.roam_mons_last_map_number = wram.roam_mons_current_map_number;
  wram.roam_mons_last_map_group = wram.roam_mons_current_map_group;
  wram.roam_mons_current_map_number = wram.wMapNumber;
  wram.roam_mons_current_map_group = wram.wMapGroup;
};

const updateAllRoamers = (
  gameState: GameState,
  rng: RandomSource,
  roamers: RoamerState[]
): void => {
  for (const roamer of roamers) {
    const [group, number] = getRoamerMapCoords(roamer);
    if (group === GROUP_N_A || number === MAP_N_A) {
      continue;
    }
    const target = chooseRoamTarget(gameState, rng, [group, number]);
    if (!target) {
      continue;
    }
    updateRoamerLocation(roamer, target[0], target[1]);
  }
  backupMapIndices(gameState);
};

export function updateRoamMons(gameState: GameState, { rng }: { rng?: RandomSource } = {}): void {
  const roamers = getRoamers(gameState);
  const rngSource = ensureRng(gameState, rng);
  updateAllRoamers(gameState, rngSource, roamers);
}

export function jumpRoamMons(gameState: GameState, { rng }: { rng?: RandomSource } = {}): void {
  const roamers = getRoamers(gameState);
  const rngSource = ensureRng(gameState, rng);
  for (const roamer of roamers) {
    const [group, number] = getRoamerMapCoords(roamer);
    if (group === GROUP_N_A || number === MAP_N_A) {
      continue;
    }
    const [targetGroup, targetNumber] = jumpToRandomMap(gameState, rngSource);
    updateRoamerLocation(roamer, targetGroup, targetNumber);
  }
  backupMapIndices(gameState);
}

export function handlePostBattleRoamers(
  gameState: GameState,
  outcome: number,
  enemyPokemon: Pokemon | null,
  { rng }: { rng?: RandomSource } = {}
): void {
  const battleType = gameState.wram.battle_type ?? "BATTLETYPE_NORMAL";
  const roamers = getRoamers(gameState);
  const rngSource = ensureRng(gameState, rng);

  if (battleType === "BATTLETYPE_ROAMING") {
    const speciesId = enemyPokemon?.species?.id ?? null;
    const roamer = locateRoamer(roamers, speciesId);
    if (!roamer) {
      throw new Error(`Roaming Pokémon '${speciesId}' missing from roaming state.`);
    }
    if (outcome === 0) {
      roamer.hp = 0;
      updateRoamerLocation(roamer, GROUP_N_A, MAP_N_A);
      roamer.species = "";
      return;
    }
    if (enemyPokemon) {
      if (typeof roamer.set_remaining_hp === "function") {
        roamer.set_remaining_hp(hpLowByte(enemyPokemon));
      } else {
        roamer.hp = hpLowByte(enemyPokemon);
      }
    }
    updateRoamMons(gameState, { rng: rngSource });
    return;
  }

  if (rngSource.nextByte() & 0x0f) {
    return;
  }
  updateRoamMons(gameState, { rng: rngSource });
}

export function checkRoamingEncounter(
  gameState: GameState,
  { rng, on_water = false }: { rng?: RandomSource; on_water?: boolean } = {}
): [string, number] | null {
  // ASM: engine/overworld/wildmons.asm::CheckEncounterRoamMon
  if (on_water) {
    return null;
  }

  const rngSource = rng ?? new HardwareRNG(gameState);
  const roll = rngSource.nextByte();
  if (roll >= 100) {
    return null;
  }
  const masked = roll & 0x03;
  if (masked === 0) {
    return null;
  }
  const index = masked - 1;
  const wram = gameState.wram;
  const roamers = getRoamers(gameState);
  if (index >= roamers.length) {
    throw new Error(
      `Roamer index ${index} exceeds available slots (${roamers.length}).`
    );
  }
  const roamer = roamers[index];
  const [roamerGroup, roamerNumber] = getRoamerMapCoords(roamer);
  if (
    roamerGroup !== wram.wMapGroup ||
    roamerNumber !== wram.wMapNumber ||
    roamerGroup === GROUP_N_A ||
    roamerNumber === MAP_N_A ||
    !roamer.species
  ) {
    return null;
  }
  return [roamer.species, roamer.level];
}

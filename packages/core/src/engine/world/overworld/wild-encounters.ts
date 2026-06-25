// ASM mapping: pokecrystal_disassembly/engine/overworld/wildmons.asm (TryWildEncounter, ChooseWildEncounter, CheckEncounterRoamMon).
import { WildEncounter, WildEncounterData, WildEncounterTable } from "@pokecrystal/assets/content/wild-encounter-data";
import { Pokemon, PokemonSpecies } from "@pokecrystal/core/core/models";
import { GameState } from "@pokecrystal/core/core/state";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { EventManager, StartBattleEvent } from "@pokecrystal/core/engine/events/events";
import { Item } from "@pokecrystal/core/core/enums/item";
import { checkRoamingEncounter, createRoamingBattlePokemon, RoamerState } from "@pokecrystal/core/engine/world/roamers";
import { describeCollision, resolveCollisionValue, Terrain } from "@pokecrystal/core/engine/world/overworld/collision-data";
import { getCoordCollision } from "@pokecrystal/core/engine/world/overworld/collision-rules";
import { ALL_SWARM_DEFINITIONS, SwarmDefinition } from "@pokecrystal/core/engine/world/overworld/swarm";
import { applySafariBattleType } from "@pokecrystal/core/engine/world/safari-zone";
import { METATILE_WIDTH } from "@pokecrystal/core/core/tileset-data";
import { createPokemon } from "@pokecrystal/core/engine/systems/pokemon";
import { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import type { OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";

type RandomSource = {
  nextByte: () => number;
  randrange: (upperBound: number) => number;
};

export enum EncounterSurface {
  GRASS = "grass",
  WATER = "water",
  ROCK = "rock",
}

const GRASS_SLOT_PROBABILITIES: Array<[number, number]> = [
  [30, 0],
  [60, 1],
  [80, 2],
  [90, 3],
  [95, 4],
  [99, 5],
  [100, 6],
];

const WATER_SLOT_PROBABILITIES: Array<[number, number]> = [
  [60, 0],
  [90, 1],
  [100, 2],
];

const GRASS_COLLISION_TOKENS = [
  "CUT_08",
  "TALL_GRASS",
  "TALL_GRASS_10",
  "LONG_GRASS",
  "LONG_GRASS_1C",
  "CUT_28",
  "GRASS_48",
  "GRASS_49",
  "GRASS_4A",
  "GRASS_4B",
  "GRASS_4C",
];

const GRASS_COLLISION_VALUES = new Set(
  GRASS_COLLISION_TOKENS.map((token) => resolveCollisionValue(token))
);

const ICE_COLLISION_VALUES = new Set([
  resolveCollisionValue("ICE"),
  resolveCollisionValue("ICE_2B"),
]);

const CAVE_TILESETS = new Set(["cave", "dark_cave"]);
const LAND_ENCOUNTER_ENVIRONMENTS = new Set(["CAVE"]);

const TIME_KEYS: Record<string, "morning" | "day" | "night"> = {
  morning: "morning",
  day: "day",
  night: "night",
};

const DOUBLE_ENCOUNTER_MUSIC_TOKENS = new Set([
  "MUSIC_POKEMON_MARCH",
  "MUSIC_RUINS_OF_ALPH_RADIO",
]);
const MUSIC_HALVE_TOKEN = "MUSIC_POKEMON_LULLABY";

const percentToByte = (value: number): number => {
  if (value <= 0) {
    return 0;
  }
  if (value >= 100) {
    return 255;
  }
  return Math.floor((value * 0xff) / 100);
};

const resolveEncounterTimeKey = (timeOfDay: unknown): "morning" | "day" | "night" => {
  if (typeof timeOfDay !== "string") {
    throw new Error(`Unknown wild encounter time of day '${timeOfDay}'.`);
  }
  const key = TIME_KEYS[timeOfDay];
  if (!key) {
    throw new Error(`Unknown wild encounter time of day '${timeOfDay}'.`);
  }
  return key;
};

export type OverworldLike = {
  map: OverworldMap;
  tileset: OverworldTilesetLike;
  current_map_name: string;
  player_x: number;
  player_y: number;
  audio_engine?: {
    get_map_music_token?: () => string | null | undefined;
    getMapMusicToken?: () => string | null | undefined;
  } | null;
};

type DataLoaderLike = {
  wild_encounter_data?: Map<string, WildEncounterData> | Record<string, WildEncounterData>;
  wildEncounterData?: Map<string, WildEncounterData> | Record<string, WildEncounterData>;
  map_attributes?: Map<string, { tileset_name?: string; environment?: string | null; map_constant?: string | null }>;
  mapAttributes?: Map<string, { tileset_name?: string; environment?: string | null; map_constant?: string | null }>;
  pokemonData?: Map<string, PokemonSpecies> | Record<string, PokemonSpecies>;
  ensure_battle_data?: () => void;
};

export class WildEncounterManager {
  private readonly rngFactory: (state: GameState) => RandomSource;
  private stepPending = false;
  private encounterMap: Map<string, WildEncounterData> | null = null;

  constructor(
    public readonly game_state: GameState,
    public readonly data_loader: DataLoaderLike,
    public readonly event_manager: EventManager | null,
    options: {
      rng_factory?: (state: GameState) => RandomSource;
      cooldown_steps?: number;
    } = {}
  ) {
    this.rngFactory = options.rng_factory ?? ((state) => new HardwareRNG(state));
    this.cooldown_steps = options.cooldown_steps ?? 5;
  }

  public cooldown_steps: number;

  on_map_loaded(map_name: string): void {
    this.game_state.wram.wild_encounter_cooldown = this.cooldown_steps;
    this.stepPending = false;
  }

  notify_step_complete(): void {
    this.stepPending = true;
  }

  skip_pending_step(): void {
    this.stepPending = false;
  }

  maybe_trigger_random_encounter(overworld: OverworldLike): void {
    if (!this.stepPending || !this.event_manager) {
      return;
    }
    this.stepPending = false;
    const wram = this.game_state.wram;
    const cooldown = wram.wild_encounter_cooldown ?? 0;
    if (cooldown > 0) {
      wram.wild_encounter_cooldown = cooldown - 1;
      return;
    }

    const encounterData = this._lookup_map_data(overworld.current_map_name);
    if (!encounterData) {
      return;
    }

    const surface = this._resolve_surface(overworld);
    if (!surface) {
      return;
    }

    const table = this._resolve_table(encounterData, surface);
    if (!table.length) {
      return;
    }

    const rng = this.rngFactory(this.game_state);
    if (!this._passes_encounter_roll(encounterData, surface, rng, overworld)) {
      return;
    }

    const roamerCandidate = checkRoamingEncounter(this.game_state, {
      rng,
      on_water: surface === EncounterSurface.WATER,
    });
    if (roamerCandidate) {
      const [speciesName, level] = roamerCandidate;
      this._start_battle(speciesName, level, "BATTLETYPE_ROAMING");
      return;
    }

    const slot = this._choose_slot(surface, table.length, rng);
    if (slot === null) {
      return;
    }

    const encounter = table[slot];
    const level = this._apply_grass_level_variance(encounter.level, surface, rng);
    this._start_battle(encounter.species, level);
  }

  choose_forced_encounter(
    overworld: OverworldLike,
    rng: RandomSource
  ): [WildEncounter, string] | null {
    const data = this._lookup_map_data(overworld.current_map_name);
    if (!data) {
      return null;
    }
    const surface = this._resolve_surface(overworld);
    if (!surface) {
      return null;
    }

    const roamer = this._choose_roamer(surface, rng);
    if (roamer) {
      return roamer;
    }

    const table = this._resolve_table(data, surface);
    if (!table.length) {
      return null;
    }
    const slot = this._choose_slot(surface, table.length, rng);
    if (slot === null) {
      return null;
    }
    const encounter = table[slot];
    const level = this._apply_grass_level_variance(encounter.level, surface, rng);
    this.game_state.wram.battle_type = "BATTLETYPE_NORMAL";
    this.game_state.wram.wTempBattleMonSpecies = encounter.species;
    return [{ level, species: encounter.species }, "BATTLETYPE_NORMAL"];
  }

  public _lookup_map_data(map_name: string): WildEncounterData | null {
    const swarmDefinition = this._resolve_active_swarm_definition();
    if (swarmDefinition) {
      return swarmDefinition.encounterData;
    }
    if (!this.encounterMap) {
      const source =
        this.data_loader.wildEncounterData ?? this.data_loader.wild_encounter_data;
      if (source instanceof Map) {
        this.encounterMap = source;
      } else if (source && typeof source === "object") {
        this.encounterMap = new Map(Object.entries(source));
      } else {
        throw new Error("Wild encounter data loader is missing definitive modpack data.");
      }
    }
    return this.encounterMap.get(map_name) ?? null;
  }

  public _resolve_surface(overworld: OverworldLike): EncounterSurface | null {
    const mapWidth = overworld.map.width * METATILE_WIDTH;
    const mapHeight = overworld.map.height * METATILE_WIDTH;
    const { player_x, player_y } = overworld;
    if (player_x < 0 || player_x >= mapWidth || player_y < 0 || player_y >= mapHeight) {
      return null;
    }
    const permission = getCoordCollision(
      overworld.map,
      overworld.tileset,
      player_x,
      player_y
    );
    const attrs = describeCollision(permission);
    if (attrs.terrain === Terrain.WALL) {
      return null;
    }
    if (GRASS_COLLISION_VALUES.has(attrs.value)) {
      return EncounterSurface.GRASS;
    }
    if (attrs.terrain === Terrain.WATER) {
      return EncounterSurface.WATER;
    }
    if (ICE_COLLISION_VALUES.has(attrs.value)) {
      return null;
    }
    if (
      this._map_allows_land_encounters(overworld)
    ) {
      return EncounterSurface.GRASS;
    }
    return null;
  }

  private _map_allows_land_encounters(overworld: OverworldLike): boolean {
    const mapName = overworld.current_map_name ?? "";
    if (!mapName) {
      return false;
    }
    const mapAttributes =
      this.data_loader.map_attributes ?? this.data_loader.mapAttributes ?? null;
    const attributes = mapAttributes?.get(mapName);
    if (attributes?.environment && LAND_ENCOUNTER_ENVIRONMENTS.has(attributes.environment)) {
      return true;
    }
    const tilesetName = attributes?.tileset_name ?? "";
    return Boolean(tilesetName && CAVE_TILESETS.has(tilesetName));
  }

  public _resolve_table(
    data: WildEncounterData,
    surface: EncounterSurface
  ): WildEncounter[] {
    const encounterTable =
      surface === EncounterSurface.WATER ? data.water : data.grass;
    if (!encounterTable) {
      return [];
    }
    const key = resolveEncounterTimeKey(this.game_state.wram.time_of_day);
    const slots = (encounterTable as WildEncounterTable)[key] ?? [];
    return [...slots];
  }

  private _resolve_active_swarm_definition(): SwarmDefinition | null {
    const wram = this.game_state.wram;
    const swarmFlags = wram.swarm_flags ?? 0;
    if (!swarmFlags) {
      return null;
    }
    const currentGroup = wram.current_map_group || wram.wMapGroup;
    const currentMapId = wram.current_map_id || wram.wMapNumber;
    const wramAny = wram as Record<string, unknown>;
    for (const definition of ALL_SWARM_DEFINITIONS) {
      if (!(swarmFlags & definition.bitMask)) {
        continue;
      }
      const groupValue = wramAny[definition.mapGroupAttr];
      const mapValue = wramAny[definition.mapNumberAttr];
      if (groupValue === currentGroup && mapValue === currentMapId) {
        return definition;
      }
    }
    return null;
  }

  public _passes_encounter_roll(
    data: WildEncounterData,
    surface: EncounterSurface,
    rng: RandomSource,
    overworld: OverworldLike
  ): boolean {
    let threshold = percentToByte(this._get_base_rate(data, surface));
    if (threshold <= 0) {
      return false;
    }
    threshold = this._apply_music_effect(threshold, overworld);
    threshold = this._apply_cleanse_tag_effect(threshold);
    if (threshold <= 0) {
      return false;
    }
    return rng.nextByte() < threshold;
  }

  private _get_base_rate(data: WildEncounterData, surface: EncounterSurface): number {
    if (surface !== EncounterSurface.WATER && data.grass_rates) {
      const key = resolveEncounterTimeKey(this.game_state.wram.time_of_day);
      return data.grass_rates[key] ?? 0;
    }
    if (surface === EncounterSurface.WATER && data.water_rate !== undefined) {
      return data.water_rate ?? 0;
    }
    return 0;
  }

  private _apply_music_effect(threshold: number, overworld: OverworldLike): number {
    const token = this._current_map_music_token(overworld);
    if (token && DOUBLE_ENCOUNTER_MUSIC_TOKENS.has(token)) {
      return (threshold << 1) & 0xff;
    }
    if (token === MUSIC_HALVE_TOKEN) {
      return threshold >> 1;
    }
    return threshold;
  }

  private _current_map_music_token(overworld: OverworldLike): string | null {
    const audioEngine = overworld.audio_engine;
    if (audioEngine) {
      const getToken =
        audioEngine.get_map_music_token ?? audioEngine.getMapMusicToken;
      if (getToken) {
        const token = getToken.call(audioEngine);
        if (token) {
          return token;
        }
      }
    }
    return null;
  }

  private _apply_cleanse_tag_effect(threshold: number): number {
    if (threshold <= 0) {
      return 0;
    }
    const party = this.game_state.sram.party?.pokemon ?? [];
    for (const member of party) {
      if (!member) {
        continue;
      }
      const item = member.item ?? null;
      if (!item) {
        continue;
      }
      const itemValue = item as unknown;
      const itemId =
        typeof itemValue === "string"
          ? itemValue
          : (itemValue as { script_name?: string; id?: string }).script_name ??
            (itemValue as { id?: string }).id;
      if (itemId === Item.CLEANSE_TAG) {
        return threshold >> 1;
      }
    }
    return threshold;
  }

  public _apply_grass_level_variance(
    baseLevel: number,
    surface: EncounterSurface,
    rng: RandomSource
  ): number {
    if (surface !== EncounterSurface.GRASS) {
      return baseLevel;
    }
    const roll = rng.nextByte();
    let extra = 0;
    for (const threshold of [35, 65, 85, 95]) {
      if (roll < percentToByte(threshold)) {
        break;
      }
      extra += 1;
    }
    return baseLevel + extra;
  }

  private _choose_roamer(
    surface: EncounterSurface,
    rng: RandomSource
  ): [WildEncounter, string] | null {
    if (surface === EncounterSurface.WATER) {
      return null;
    }
    const roll = rng.nextByte();
    if (roll >= 100) {
      return null;
    }
    const candidateBits = roll & 0x03;
    if (candidateBits === 0) {
      return null;
    }
    const index = candidateBits - 1;
    const wram = this.game_state.wram;
    const roamers = (wram.roaming_pokemon ?? []) as RoamerState[];
    if (index >= roamers.length) {
      return null;
    }
    const roamer = roamers[index];
    if (!roamer?.species) {
      return null;
    }
    let mapGroup = wram.current_map_group;
    let mapId = wram.current_map_id;
    if (mapGroup == null || mapId == null) {
      mapGroup = wram.wMapGroup ?? 0;
      mapId = wram.wMapNumber ?? 0;
    }
    if (roamer.map_group !== mapGroup || roamer.map_number !== mapId) {
      return null;
    }

    const level = roamer.level ?? 40;
    wram.battle_type = "BATTLETYPE_ROAMING";
    wram.wTempBattleMonSpecies = roamer.species;
    return [{ level, species: roamer.species }, "BATTLETYPE_ROAMING"];
  }

  public _choose_slot(
    surface: EncounterSurface,
    slotCount: number,
    rng: RandomSource
  ): number | null {
    if (slotCount <= 0) {
      return null;
    }
    const probabilities =
      surface === EncounterSurface.WATER ? WATER_SLOT_PROBABILITIES : GRASS_SLOT_PROBABILITIES;
    const roll = this._random_percent(rng);
    for (const [threshold, slot] of probabilities) {
      if (roll <= threshold && slot < slotCount) {
        return slot;
      }
    }
    throw new Error(
      `Encounter roll ${roll} did not resolve for ${surface} slot table with ${slotCount} slots.`
    );
  }

  private _random_percent(rng: RandomSource): number {
    while (true) {
      const value = rng.nextByte();
      if (value < 100) {
        return value + 1;
      }
    }
  }

  private _start_battle(species_name: string, level: number, battle_type?: string): void {
    if (!this.event_manager) {
      throw new Error("Cannot start battle without event manager");
    }
    if (this._is_blocked_by_repel(level)) {
      return;
    }
    if (typeof this.data_loader.ensure_battle_data === "function") {
      this.data_loader.ensure_battle_data();
    }
    const wram = this.game_state.wram;
    const speciesTable = this.data_loader.pokemonData ?? {};
    const species =
      speciesTable instanceof Map
        ? speciesTable.get(species_name)
        : (speciesTable as Record<string, PokemonSpecies | undefined>)[species_name];
    if (!species) {
      throw new Error(`Unknown wild species '${species_name}' in encounter table.`);
    }
    const resolvedBattleType = battle_type ?? "BATTLETYPE_NORMAL";
    const wildPokemon = resolvedBattleType === "BATTLETYPE_ROAMING"
      ? createRoamingBattlePokemon(this.game_state, species, level)
      : createPokemon(this.game_state, species, level);
    wildPokemon.original_trainer_name = "WILD";
    wildPokemon.original_trainer_id = 0;

    const partyCandidates = (this.game_state.sram.party?.pokemon ?? []) as (Pokemon | null)[];
    const party = partyCandidates.filter((member): member is Pokemon => Boolean(member));
    if (!party.length) {
      throw new Error("Cannot start a wild battle without at least one Pokemon.");
    }

    if (wram) {
      wram.other_trainer_class = "";
      wram.other_trainer_id = "";
      wram.other_trainer = undefined;
      wram.other_trainer_party = [];
      wram.wild_pokemon = {
        species: species_name,
        level,
      };
    }
    this.game_state.wram.battle_type = resolvedBattleType;
    if (!battle_type) {
      applySafariBattleType(this.game_state);
    }

    const event = new StartBattleEvent({
      player_pokemon: party[0],
      enemy_pokemon: wildPokemon,
      player_party: party,
      enemy_party: [wildPokemon],
    });
    this.event_manager.dispatch(event);
  }

  private _is_blocked_by_repel(level: number): boolean {
    const repelSteps = this.game_state.wram.repel_steps ?? 0;
    if (repelSteps <= 0) {
      return false;
    }
    const party = this.game_state.sram.party?.pokemon ?? [];
    for (const pokemon of party) {
      if (!pokemon || pokemon.hp <= 0) {
        continue;
      }
      const leadLevel = pokemon.level;
      if (leadLevel === undefined || leadLevel === null) {
        return false;
      }
      return level <= leadLevel;
    }
    return false;
  }
}

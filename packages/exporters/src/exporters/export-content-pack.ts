import fs from "fs";
import { readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir } from "@pokecrystal/core/core/paths";
import { joinPath } from "@pokecrystal/core/core/path-utils";
import type { Move } from "@pokecrystal/core/core/models/move";
import type { PokemonSpecies } from "@pokecrystal/core/core/models/pokemon";
import type { WildEncounterData } from "@pokecrystal/assets/content/wild-encounter-data";
import type { PokedexData } from "@pokecrystal/assets/content/pokedex-data";
import type { Trainer } from "@pokecrystal/core/core/models/trainer";
import type { ExportedItem } from "./export-items";
import type { NpcData } from "./export-npcs";
import { ensureDir, removeMatchingOutputs, writeJsonToTargets } from "./asm-utils";
import type { EggMovesData, LevelUpLearnsets, LevelUpMovesData } from "./export-data";
import type { PokegearLandmarksPayload } from "./export-pokegear-landmarks";
import type { PlayabilityRules } from "./export-playability";
import type { ExportedAudioAsset } from "./export-audio-assets";
import type { ExportedPokemonEvolutionData } from "./export-evolutions";

const CONTENT_PACK_CATEGORIES = [
  "pokemon",
  "moves",
  "learnsets",
  "level_up_moves",
  "egg_moves",
  "evolutions",
  "maps",
  "map_blocks",
  "map_attributes",
  "map_dimensions",
  "wild_encounters",
  "runtime_spawn_points",
  "runtime_map_metadata",
  "flee_mons",
  "fishing",
  "fruit_trees",
  "npcs",
  "pokegear_landmarks",
  "pc_strings",
  "menu_icons",
  "items",
  "marts",
  "currency_constants",
  "trainers",
  "pokedex",
  "pokedex_entries",
  "pokemon_frontpic_anim",
  "initialize_events",
  "story_event_script_constants",
  "story_events",
  "phone_scripts",
  "phone_contacts",
  "permanent_phone_numbers",
  "special_phone_calls",
  "npc_trades",
  "special_routines",
  "asm_text",
  "move_names",
  "battle_animations",
  "battle_animation_table",
  "battle_anim_bundle",
  "sprite_anim_bundle",
  "sprite_palette_defaults",
  "pokegear_town_map_palette_map",
  "pokemon_cries",
  "audio",
  "tilesets",
  "playability",
] as const;

type ContentPackCategory = (typeof CONTENT_PACK_CATEGORIES)[number];
type ContentPackFiles = Record<ContentPackCategory, string[]>;

type ContentPack = {
  id: string;
  enabled: boolean;
  priority: number;
  path: string;
  compiled: string | null;
  files: ContentPackFiles;
};

type ContentPackIndex = {
  version: number;
  packs: ContentPack[];
};

type CompiledContentPack = {
  version: number;
  packId: string;
  categories: Record<ContentPackCategory, unknown[]>;
};

export type CoreExportPayload = {
  pokemonData: PokemonSpecies[];
  movesData: Record<string, Move>;
  learnsetsData: LevelUpLearnsets;
  levelUpMovesData: LevelUpMovesData;
  eggMovesData: EggMovesData;
  evolutions: ExportedPokemonEvolutionData[];
  wildEncounters: WildEncounterData[];
  runtimeSpawnPoints?: unknown;
  runtimeMapMetadata?: unknown;
  fleeMons?: unknown;
  mapDimensions: Record<string, { width: number; height: number }>;
  mapAttributes: Record<string, unknown>;
  items: ExportedItem[];
  marts?: Record<string, string[]>;
  pcStrings?: Record<string, string>;
  menuIcons?: Record<string, string>;
  pokedexEntries?: unknown[];
  pokemonFrontpicAnimations?: Record<string, unknown>;
  initializeEvents?: unknown;
  storyEventScriptConstants?: unknown;
  phoneContacts?: Record<string, unknown>;
  permanentPhoneNumbers?: string[];
  specialPhoneCalls?: string[];
  npcTrades?: string[];
  specialRoutines?: string[];
  asmText?: Record<string, string>;
  moveNames?: string[];
  battleAnimations?: Record<string, string[]>;
  battleAnimationTable?: string[];
  battleAnimBundle?: unknown;
  spriteAnimBundle?: unknown;
  spritePaletteDefaults?: Record<string, number>;
  pokegearTownMapPaletteMap?: Record<string, string[]>;
  pokemonCries?: unknown;
  trainers: Trainer[];
  pokedex: PokedexData[];
  npcData: NpcData;
  pokegearLandmarks: PokegearLandmarksPayload;
  playability?: PlayabilityRules;
  audioAssets?: ExportedAudioAsset[];
};

const CORE_PACK_ID = "core-modular";
const CORE_PACK_PATH = `content-packs/${CORE_PACK_ID}`;
const MODULE_PREFIX = "module";

const assertExactFileStem = (value: string): void => {
  if (value.trim() !== value || value.length === 0) {
    throw new Error(`Content pack file stem must be explicit and untrimmed: ${JSON.stringify(value)}`);
  }
  if (value.includes("/") || value.includes("\\") || value === "." || value === ".." || value.includes("..")) {
    throw new Error(`Content pack file stem must be a single exact path segment: ${JSON.stringify(value)}`);
  }
};

const emptyContentPackFiles = (): ContentPackFiles =>
  Object.fromEntries(CONTENT_PACK_CATEGORIES.map((category) => [category, []])) as unknown as ContentPackFiles;

const asRecord = (value: unknown): Record<string, unknown> | null =>
  value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;

const GENERATED_PARTY_FIELDS = new Set([
  "species",
  "nickname",
  "level",
  "item",
  "moves",
  "hp",
  "max_hp",
  "original_trainer_name",
  "original_trainer_id",
  "experience",
  "happiness",
  "dvs",
  "sleep_turns",
  "flinching",
  "rampage_turns",
  "confusion_turns",
  "perish_song_turns",
  "focus_energy",
  "hp_exp",
  "attack_exp",
  "defense_exp",
  "speed_exp",
  "special_exp",
  "turns_in_battle",
  "stat_boosts",
  "locked_turns_remaining",
  "trapped_turns",
  "leech_seeded",
  "nightmare",
  "cursed",
  "attack",
  "defense",
  "speed",
  "special_attack",
  "special_defense",
  "disable_turns",
  "encore_turns_remaining",
  "destiny_bond_active",
  "pokerus",
  "rage_active",
  "rage_counter",
  "fury_cutter_count",
  "rollout_step",
  "rollout_active",
  "defense_curled",
  "cant_run",
  "bide_active",
  "bide_turns_remaining",
  "bide_damage",
  "protect_active",
  "protect_counter",
  "endure_active",
  "endure_counter",
  "foresight_active",
  "lock_on_active",
  "substitute_hp",
  "transformed",
  "last_damage_taken",
]);

const partySpeciesInfo = (
  partyEntry: Trainer["party"][number],
  trainerLabel: string,
  partyIndex: number
): { speciesId: string } => {
  const source = partyEntry as unknown as Record<string, unknown>;
  const species = source.species;
  if (typeof species === "string" && species.trim()) {
    return { speciesId: species };
  }
  const speciesRecord = asRecord(species) as PokemonSpecies | null;
  if (speciesRecord && typeof speciesRecord.id === "string" && speciesRecord.id.trim()) {
    return {
      speciesId: speciesRecord.id,
    };
  }
  throw new Error(
    `Unable to export trainer ${trainerLabel} party[${partyIndex}] because species is not a species id string or Pokemon species record with an id.`
  );
};

const compactTrainerPartyEntry = (
  partyEntry: Trainer["party"][number],
  trainerLabel: string,
  partyIndex: number
): Record<string, unknown> => {
  const source = partyEntry as unknown as Record<string, unknown>;
  const { speciesId } = partySpeciesInfo(partyEntry, trainerLabel, partyIndex);
  for (const key of ["item", "moves", "dvs"] as const) {
    if (!Object.prototype.hasOwnProperty.call(source, key)) {
      throw new Error(
        `Unable to export trainer ${trainerLabel} party[${partyIndex}] because '${key}' must be explicit modpack data.`
      );
    }
  }
  if (!Array.isArray(source.moves)) {
    throw new Error(
      `Unable to export trainer ${trainerLabel} party[${partyIndex}] because 'moves' must be an explicit array.`
    );
  }
  if (!asRecord(source.dvs)) {
    throw new Error(
      `Unable to export trainer ${trainerLabel} party[${partyIndex}] because 'dvs' must be an explicit object.`
    );
  }
  const compact: Record<string, unknown> = {
    species: speciesId,
    level: partyEntry.level,
    item: source.item,
    moves: source.moves,
    dvs: source.dvs,
  };

  for (const [key, value] of Object.entries(source)) {
    if (key === "species" || key === "level" || key === "item" || key === "moves" || key === "dvs") {
      continue;
    }
    if (GENERATED_PARTY_FIELDS.has(key)) {
      continue;
    }
    throw new Error(
      `Unable to export trainer ${trainerLabel} party[${partyIndex}] because '${key}' is not part of the definitive trainer party schema.`
    );
  }

  return compact;
};

const pokemonSpeciesPackPayload = (pokemon: PokemonSpecies): Omit<PokemonSpecies, "evolutions"> => {
  const { evolutions: _staleEvolutions, ...speciesPayload } = pokemon as PokemonSpecies & {
    evolutions?: unknown;
  };
  return speciesPayload;
};

const compactTrainerForContentPack = (trainer: Trainer): Record<string, unknown> => {
  const trainerLabel = trainer.trainer_id ?? trainer.name ?? "unknown";
  return {
    ...trainer,
    party: Array.isArray(trainer.party)
      ? trainer.party.map((entry, index) => compactTrainerPartyEntry(entry, trainerLabel, index))
      : [],
  };
};

const assertNoEmbeddedOwnedRecords = (
  category: ContentPackCategory,
  relativePath: string,
  payload: unknown
): void => {
  if (category !== "trainers") {
    return;
  }
  const trainer = asRecord(payload);
  const party = Array.isArray(trainer?.party) ? trainer.party : [];
  for (let index = 0; index < party.length; index += 1) {
    const entry = asRecord(party[index]);
    const species = asRecord(entry?.species);
    if (
      species &&
      (typeof species.id === "string" ||
        asRecord(species.base_stats) ||
        Array.isArray(species.tmhm_learnset))
    ) {
      throw new Error(
        `Content pack trainer ${relativePath} embeds a Pokemon species object at party[${index}].species. Use the species id string instead.`
      );
    }
  }
};

const writePackEntry = <T>(
  files: string[],
  category: ContentPackCategory,
  stem: string,
  payload: T
): string => {
  assertExactFileStem(stem);
  const fileName = `${stem}.json`;
  const relativePath = `${CORE_PACK_PATH}/${category}/${fileName}`;
  assertNoEmbeddedOwnedRecords(category, relativePath, payload);
  writeJsonToTargets(relativePath, payload, { indent: 2 });
  files.push(relativePath);
  return relativePath;
};

const SILENT_MIDI_BYTES = Buffer.from(
  "4d546864000000060000000100604d54726b0000000400ff2f00",
  "hex"
);

const clearGeneratedAudioOutputs = (): void => {
  for (const directory of ["music", "sfx", "cries"]) {
    fs.rmSync(joinPath(getDataDir(), CORE_PACK_PATH, directory), { recursive: true, force: true });
  }
};

const writeAudioAssetFile = (
  files: string[],
  asset: ExportedAudioAsset,
  writtenPayloads: Map<string, unknown>
): void => {
  if (!asset.path.endsWith(".mid")) {
    throw new Error(`Audio asset ${asset.id} must use a .mid file: ${asset.path}`);
  }
  if (!asset.path.startsWith(`${CORE_PACK_PATH}/`)) {
    throw new Error(`Audio asset ${asset.id} must live under ${CORE_PACK_PATH}: ${asset.path}`);
  }
  const absolutePath = joinPath(getDataDir(), asset.path);
  ensureDir(absolutePath.split("/").slice(0, -1).join("/"));
  fs.writeFileSync(absolutePath, SILENT_MIDI_BYTES);
  files.push(asset.path);
  writtenPayloads.set(asset.path, asset);
};

const collectJsonFiles = (relativeDir: ContentPackCategory): string[] => {
  const absoluteDir = joinPath(getDataDir(), relativeDir);
  if (!fs.existsSync(absoluteDir)) {
    return [];
  }
  return fs
    .readdirSync(absoluteDir)
    .filter((entry) => entry.endsWith(".json"))
    .sort()
    .map((entry) => `${relativeDir}/${entry}`);
};

const exactMapNameFromPayload = (relativePath: string, payload: unknown): string => {
  const record = asRecord(payload);
  if (!record) {
    throw new Error(`Map asset ${relativePath} must be an object keyed by exact map labels.`);
  }
  const mapNames = new Set<string>();
  for (const key of Object.keys(record)) {
    const match = key.match(/^(.+)_Map(?:Scripts|Events)$/);
    if (match) {
      mapNames.add(match[1]);
    }
  }
  if (mapNames.size !== 1) {
    throw new Error(`Map asset ${relativePath} must declare one exact map name via _MapScripts or _MapEvents labels.`);
  }
  return [...mapNames][0];
};

const writeMapAssetFiles = (
  files: string[],
  sourceRelativeDir: string,
  writtenPayloads?: Map<string, unknown>
): Map<string, string> => {
  const written = new Map<string, string>();
  const absoluteDir = joinPath(getDataDir(), sourceRelativeDir);
  if (!fs.existsSync(absoluteDir)) {
    return written;
  }
  for (const entry of fs
    .readdirSync(absoluteDir)
    .filter((fileName) => fileName.endsWith(".json"))
    .sort()) {
    const stem = entry.replace(/\.json$/i, "");
    const payload = readJsonAssetSync(joinPath(getDataDir(), sourceRelativeDir, entry));
    const relativePath = writePackEntry(files, "maps", stem, payload);
    writtenPayloads?.set(relativePath, payload);
    written.set(exactMapNameFromPayload(relativePath, payload), relativePath);
  }
  return written;
};

const writeMapBlockEntries = (
  files: string[],
  writtenPayloads?: Map<string, unknown>
): Map<string, string> => {
  const written = new Map<string, string>();
  const raw = readJsonAssetSync<unknown>(joinPath(getDataDir(), "map_blocks.json"));
  if (!isRecord(raw)) {
    throw new Error("map_blocks.json must be an object of block labels to encoded block data.");
  }
  for (const [label, encoded] of Object.entries(raw).sort(([a], [b]) => a.localeCompare(b))) {
    if (typeof encoded !== "string") {
      throw new Error(`map_blocks.json entry '${label}' must be encoded block data.`);
    }
    const payload = { [label]: encoded };
    const relativePath = writePackEntry(files, "map_blocks", label, payload);
    writtenPayloads?.set(relativePath, payload);
    written.set(label, relativePath);
  }
  return written;
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === "object" && !Array.isArray(value);

const requireContentPackFiles = (packId: string, value: unknown): ContentPackFiles => {
  if (!isRecord(value)) {
    throw new Error(`Content pack '${packId}' must declare files.`);
  }
  const files = {} as ContentPackFiles;
  for (const category of CONTENT_PACK_CATEGORIES) {
    const entries = value[category];
    if (!Array.isArray(entries) || entries.some((entry) => typeof entry !== "string")) {
      throw new Error(`Content pack '${packId}' must declare files.${category} as a string array.`);
    }
    files[category] = entries;
  }
  return files;
};

const requireContentPack = (value: unknown, index: number): ContentPack => {
  if (!isRecord(value)) {
    throw new Error(`Content pack index entry ${index} must be an object.`);
  }
  const id = value.id;
  if (typeof id !== "string" || id.length === 0) {
    throw new Error(`Content pack index entry ${index} must declare id.`);
  }
  if (typeof value.enabled !== "boolean") {
    throw new Error(`Content pack '${id}' must declare enabled.`);
  }
  if (typeof value.priority !== "number") {
    throw new Error(`Content pack '${id}' must declare priority.`);
  }
  if (typeof value.path !== "string") {
    throw new Error(`Content pack '${id}' must declare path.`);
  }
  if (typeof value.compiled !== "string" && value.compiled !== null) {
    throw new Error(`Content pack '${id}' must declare compiled as a string or null.`);
  }
  return {
    id,
    enabled: value.enabled,
    priority: value.priority,
    path: value.path,
    compiled: value.compiled,
    files: requireContentPackFiles(id, value.files),
  };
};

const readIndex = (): ContentPackIndex => {
  const indexPath = joinPath(getDataDir(), "content-packs/index.json");
  const parsed = readJsonAssetSync<unknown>(indexPath);
  if (!isRecord(parsed)) {
    throw new Error("Content pack index must be an object.");
  }
  if (parsed.version !== 1) {
    throw new Error("Content pack index must declare version 1.");
  }
  if (!Array.isArray(parsed.packs)) {
    throw new Error("Content pack index must declare packs as an array.");
  }
  return {
    version: parsed.version,
    packs: parsed.packs.map(requireContentPack),
  };
};

const parentPathFor = (filePath: string, _category: ContentPackCategory): string =>
  filePath.replace(/\/[^/]+$/, "");

const fileStem = (filePath: string): string =>
  filePath.split("/").pop()?.replace(/\.[^.]+$/i, "") ?? "unknown";

const makeModulePack = (
  id: string,
  path: string,
  files: Partial<ContentPackFiles>
): ContentPack => ({
  id,
  enabled: false,
  priority: 0,
  path,
  compiled: null,
  files: {
    ...emptyContentPackFiles(),
    ...files,
  },
});

const emptyCompiledCategories = (): Record<ContentPackCategory, unknown[]> =>
  Object.fromEntries(CONTENT_PACK_CATEGORIES.map((category) => [category, []])) as unknown as Record<
      ContentPackCategory,
      unknown[]
    >;

const buildCompiledCorePack = (
  files: ContentPackFiles,
  writtenPayloads: Map<string, unknown>
): CompiledContentPack => {
  const categories = emptyCompiledCategories();
  for (const category of CONTENT_PACK_CATEGORIES) {
    categories[category] = files[category].map((relativePath) => {
      if (writtenPayloads.has(relativePath)) {
        return writtenPayloads.get(relativePath);
      }
      return readJsonAssetSync(joinPath(getDataDir(), relativePath));
    });
  }
  return {
    version: 1,
    packId: CORE_PACK_ID,
    categories,
  };
};

export function exportCoreContentPack(payload: CoreExportPayload): void {
  removeMatchingOutputs(CORE_PACK_PATH, ".json");
  clearGeneratedAudioOutputs();

  const files = emptyContentPackFiles();
  const writtenPayloads = new Map<string, unknown>();

  const writeCorePackEntry = <T>(
    categoryFiles: string[],
    category: ContentPackCategory,
    stem: string,
    entryPayload: T
  ): string => {
    const relativePath = writePackEntry(categoryFiles, category, stem, entryPayload);
    writtenPayloads.set(relativePath, entryPayload);
    return relativePath;
  };

  for (const pokemon of payload.pokemonData) {
    writeCorePackEntry(files.pokemon, "pokemon", pokemon.id, pokemonSpeciesPackPayload(pokemon));
  }
  for (const [name, move] of Object.entries(payload.movesData)) {
    writeCorePackEntry(files.moves, "moves", name, move);
  }
  for (const [species, learnset] of Object.entries(payload.learnsetsData)) {
    writeCorePackEntry(files.learnsets, "learnsets", species, {
      species,
      learnset,
    });
  }
  for (const [species, moves] of Object.entries(payload.levelUpMovesData)) {
    writeCorePackEntry(files.level_up_moves, "level_up_moves", species, {
      species,
      moves,
    });
  }
  for (const [species, moves] of Object.entries(payload.eggMovesData)) {
    writeCorePackEntry(files.egg_moves, "egg_moves", species, {
      species,
      moves,
    });
  }
  for (const evolution of payload.evolutions) {
    const key = evolution.species ?? `species_${files.evolutions.length}`;
    writeCorePackEntry(files.evolutions, "evolutions", key, evolution);
  }
  const mapPathByName = writeMapAssetFiles(files.maps, "maps", writtenPayloads);
  const mapBlockPathByLabel = writeMapBlockEntries(files.map_blocks, writtenPayloads);
  const encounterPathByMapName = new Map<string, string>();
  for (const encounter of payload.wildEncounters) {
    const encounterPath = writeCorePackEntry(files.wild_encounters, "wild_encounters", encounter.map_name, encounter);
    encounterPathByMapName.set(encounter.map_name, encounterPath);
  }
  if (payload.runtimeSpawnPoints) {
    writeCorePackEntry(files.runtime_spawn_points, "runtime_spawn_points", "spawn_points", payload.runtimeSpawnPoints);
  }
  if (payload.runtimeMapMetadata) {
    writeCorePackEntry(files.runtime_map_metadata, "runtime_map_metadata", "map_metadata", payload.runtimeMapMetadata);
  }
  if (payload.fleeMons) {
    writeCorePackEntry(files.flee_mons, "flee_mons", "flee_mons", payload.fleeMons);
  }
  for (const audioAsset of payload.audioAssets ?? []) {
    writeAudioAssetFile(files.audio, audioAsset, writtenPayloads);
  }

  const mapAttributePathByName = new Map<string, string>();
  const mapBlockPathByName = new Map<string, string>();
  const mapDimensionKeyByName = new Map<string, string>();
  for (const [mapName, attributes] of Object.entries(payload.mapAttributes)) {
    const mapAttributePath = writeCorePackEntry(files.map_attributes, "map_attributes", mapName, {
      [mapName]: attributes,
    });
    mapAttributePathByName.set(mapName, mapAttributePath);
    const record = asRecord(attributes);
    if (typeof record?.map_constant === "string") {
      mapDimensionKeyByName.set(mapName, record.map_constant);
    }
    const blocksLabel = typeof record?.blocks_label === "string" ? record.blocks_label : `${mapName}_Blocks`;
    const mapBlockPath = mapBlockPathByLabel.get(blocksLabel);
    if (mapBlockPath) {
      mapBlockPathByName.set(mapName, mapBlockPath);
    }
  }

  const mapDimensionPathByName = new Map<string, string>();
  for (const [mapName, dimensions] of Object.entries(payload.mapDimensions)) {
    const mapDimensionPath = writeCorePackEntry(files.map_dimensions, "map_dimensions", mapName, {
      [mapName]: dimensions,
    });
    mapDimensionPathByName.set(mapName, mapDimensionPath);
  }

  const npcPathByName = new Map<string, string>();
  for (const [mapName, entries] of Object.entries(payload.npcData)) {
    const npcPath = writeCorePackEntry(files.npcs, "npcs", mapName, { [mapName]: entries });
    npcPathByName.set(mapName, npcPath);
  }

  for (const item of payload.items) {
    writeCorePackEntry(files.items, "items", item.name, item);
  }
  if (payload.marts && Object.keys(payload.marts).length > 0) {
    writeCorePackEntry(files.marts, "marts", "marts", payload.marts);
  }
  if (payload.pcStrings && Object.keys(payload.pcStrings).length > 0) {
    writeCorePackEntry(files.pc_strings, "pc_strings", "pc_strings", payload.pcStrings);
  }
  if (payload.menuIcons && Object.keys(payload.menuIcons).length > 0) {
    writeCorePackEntry(files.menu_icons, "menu_icons", "menu_icons", payload.menuIcons);
  }
  if (payload.phoneContacts && Object.keys(payload.phoneContacts).length > 0) {
    writeCorePackEntry(files.phone_contacts, "phone_contacts", "contacts", payload.phoneContacts);
  }
  if (payload.permanentPhoneNumbers && payload.permanentPhoneNumbers.length > 0) {
    writeCorePackEntry(
      files.permanent_phone_numbers,
      "permanent_phone_numbers",
      "permanent",
      payload.permanentPhoneNumbers
    );
  }
  if (payload.specialPhoneCalls && payload.specialPhoneCalls.length > 0) {
    writeCorePackEntry(files.special_phone_calls, "special_phone_calls", "calls", payload.specialPhoneCalls);
  }
  if (payload.npcTrades && payload.npcTrades.length > 0) {
    writeCorePackEntry(files.npc_trades, "npc_trades", "trades", payload.npcTrades);
  }
  if (payload.specialRoutines && payload.specialRoutines.length > 0) {
    writeCorePackEntry(files.special_routines, "special_routines", "routines", payload.specialRoutines);
  }
  if (payload.asmText && Object.keys(payload.asmText).length > 0) {
    writeCorePackEntry(files.asm_text, "asm_text", "texts", payload.asmText);
  }
  if (payload.moveNames && payload.moveNames.length > 0) {
    writeCorePackEntry(files.move_names, "move_names", "moves", payload.moveNames);
  }
  if (payload.battleAnimations && Object.keys(payload.battleAnimations).length > 0) {
    writeCorePackEntry(files.battle_animations, "battle_animations", "scripts", payload.battleAnimations);
  }
  if (payload.battleAnimationTable && payload.battleAnimationTable.length > 0) {
    writeCorePackEntry(files.battle_animation_table, "battle_animation_table", "table", payload.battleAnimationTable);
  }
  if (payload.battleAnimBundle) {
    writeCorePackEntry(files.battle_anim_bundle, "battle_anim_bundle", "bundle", payload.battleAnimBundle);
  }
  if (payload.spriteAnimBundle) {
    writeCorePackEntry(files.sprite_anim_bundle, "sprite_anim_bundle", "bundle", payload.spriteAnimBundle);
  }
  if (payload.spritePaletteDefaults && Object.keys(payload.spritePaletteDefaults).length > 0) {
    writeCorePackEntry(files.sprite_palette_defaults, "sprite_palette_defaults", "defaults", payload.spritePaletteDefaults);
  }
  if (payload.pokegearTownMapPaletteMap && Object.keys(payload.pokegearTownMapPaletteMap).length > 0) {
    writeCorePackEntry(
      files.pokegear_town_map_palette_map,
      "pokegear_town_map_palette_map",
      "palettes",
      payload.pokegearTownMapPaletteMap
    );
  }
  if (payload.pokemonCries) {
    writeCorePackEntry(files.pokemon_cries, "pokemon_cries", "cries", payload.pokemonCries);
  }
  for (const trainer of payload.trainers) {
    writeCorePackEntry(
      files.trainers,
      "trainers",
      trainer.trainer_id ?? trainer.name,
      compactTrainerForContentPack(trainer)
    );
  }
  for (const entry of payload.pokedex) {
    writeCorePackEntry(files.pokedex, "pokedex", entry.species, entry);
  }
  if (payload.pokedexEntries && payload.pokedexEntries.length > 0) {
    writeCorePackEntry(files.pokedex_entries, "pokedex_entries", "entries", payload.pokedexEntries);
  }
  if (payload.pokemonFrontpicAnimations && Object.keys(payload.pokemonFrontpicAnimations).length > 0) {
    writeCorePackEntry(
      files.pokemon_frontpic_anim,
      "pokemon_frontpic_anim",
      "programs",
      payload.pokemonFrontpicAnimations
    );
  }
  if (payload.initializeEvents) {
    writeCorePackEntry(files.initialize_events, "initialize_events", "initialize_events", payload.initializeEvents);
  }
  if (payload.storyEventScriptConstants) {
    writeCorePackEntry(
      files.story_event_script_constants,
      "story_event_script_constants",
      "constants",
      payload.storyEventScriptConstants
    );
  }
  writeCorePackEntry(files.pokegear_landmarks, "pokegear_landmarks", "landmarks", payload.pokegearLandmarks);
  if (payload.playability) {
    writeCorePackEntry(files.playability, "playability", "core", payload.playability);
  }
  files.story_events.push(...collectJsonFiles("story_events"));
  files.phone_scripts.push(...collectJsonFiles("phone_scripts"));

  const compiledPath = "content-packs/core-modular.compiled.json";
  writeJsonToTargets(compiledPath, buildCompiledCorePack(files, writtenPayloads), { indent: 0 });

  const index = readIndex();
  const remainingPacks = index.packs.filter(
    (pack) => pack.id !== CORE_PACK_ID && !pack.id.startsWith(`${MODULE_PREFIX}-`)
  );
  remainingPacks.push({
    id: CORE_PACK_ID,
    enabled: true,
    priority: -100,
    path: CORE_PACK_PATH,
    compiled: compiledPath,
    files,
  });

  const singleCategoryModules: Array<{
    category: ContentPackCategory;
    prefix: string;
  }> = [
    { category: "maps", prefix: "map" },
    { category: "map_blocks", prefix: "map-block" },
    { category: "runtime_spawn_points", prefix: "runtime-spawn-points" },
    { category: "runtime_map_metadata", prefix: "runtime-map-metadata" },
    { category: "flee_mons", prefix: "flee-mons" },
    { category: "pokemon", prefix: "pokemon" },
    { category: "moves", prefix: "move" },
    { category: "learnsets", prefix: "learnset" },
    { category: "level_up_moves", prefix: "level-up-move" },
    { category: "egg_moves", prefix: "egg-move" },
    { category: "items", prefix: "item" },
    { category: "pc_strings", prefix: "pc-strings" },
    { category: "menu_icons", prefix: "menu-icons" },
    { category: "trainers", prefix: "trainer" },
    { category: "pokedex", prefix: "pokedex" },
    { category: "pokedex_entries", prefix: "pokedex-entries" },
    { category: "pokemon_frontpic_anim", prefix: "pokemon-frontpic-anim" },
    { category: "initialize_events", prefix: "initialize-events" },
    { category: "story_event_script_constants", prefix: "story-event-script-constants" },
    { category: "pokegear_landmarks", prefix: "pokegear-landmarks" },
    { category: "npcs", prefix: "npc" },
    { category: "story_events", prefix: "story" },
    { category: "phone_scripts", prefix: "phone" },
    { category: "phone_contacts", prefix: "phone-contacts" },
    { category: "permanent_phone_numbers", prefix: "permanent-phone-numbers" },
    { category: "special_phone_calls", prefix: "special-phone-calls" },
    { category: "npc_trades", prefix: "npc-trades" },
    { category: "special_routines", prefix: "special-routines" },
    { category: "asm_text", prefix: "asm-text" },
    { category: "move_names", prefix: "move-names" },
    { category: "battle_animations", prefix: "battle-animations" },
    { category: "battle_animation_table", prefix: "battle-animation-table" },
    { category: "battle_anim_bundle", prefix: "battle-anim-bundle" },
    { category: "sprite_anim_bundle", prefix: "sprite-anim-bundle" },
    { category: "sprite_palette_defaults", prefix: "sprite-palette-defaults" },
    { category: "pokegear_town_map_palette_map", prefix: "pokegear-town-map-palette-map" },
    { category: "pokemon_cries", prefix: "pokemon-cries" },
    { category: "audio", prefix: "audio" },
    { category: "tilesets", prefix: "tileset" },
    { category: "playability", prefix: "playability" },
  ];

  for (const { category, prefix } of singleCategoryModules) {
    for (const filePath of files[category]) {
      const name = fileStem(filePath);
      remainingPacks.push(
        makeModulePack(`${MODULE_PREFIX}-${prefix}-${name}`, parentPathFor(filePath, category), {
          [category]: [filePath],
        })
      );
    }
  }

  for (const [mapName, routePath] of encounterPathByMapName) {
    const name = fileStem(routePath);
    const mapPath = mapPathByName.get(mapName);
    const mapBlockPath = mapBlockPathByName.get(mapName);
    const mapAttributePath = mapAttributePathByName.get(mapName);
    const mapDimensionKey = mapDimensionKeyByName.get(mapName);
    const mapDimensionPath = mapDimensionKey ? mapDimensionPathByName.get(mapDimensionKey) : undefined;
    const npcPath = npcPathByName.get(mapName);
    remainingPacks.push(
      makeModulePack(`${MODULE_PREFIX}-route-${name}`, parentPathFor(routePath, "wild_encounters"), {
        maps: mapPath ? [mapPath] : [],
        map_blocks: mapBlockPath ? [mapBlockPath] : [],
        map_attributes: mapAttributePath ? [mapAttributePath] : [],
        map_dimensions: mapDimensionPath ? [mapDimensionPath] : [],
        wild_encounters: [routePath],
        npcs: npcPath ? [npcPath] : [],
      })
    );
  }

  remainingPacks.sort((a, b) => a.id.localeCompare(b.id));

  writeJsonToTargets("content-packs/index.json", { version: 1, packs: remainingPacks }, { indent: 2 });
}

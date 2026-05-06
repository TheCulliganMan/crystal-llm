import fs from "fs";
import { readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir } from "@pokecrystal/core/core/paths";
import { joinPath } from "@pokecrystal/core/core/path-utils";
import type { Move } from "@pokecrystal/core/core/models/move";
import type { PokemonSpecies } from "@pokecrystal/core/core/models/pokemon";
import type { WildEncounterData } from "@pokecrystal/assets/content/wild-encounter-data";
import type { PokemonEvolutionData } from "@pokecrystal/assets/content/evolution-data";
import type { PokedexData } from "@pokecrystal/assets/content/pokedex-data";
import type { Trainer } from "@pokecrystal/core/core/models/trainer";
import type { ExportedItem } from "./export-items";
import type { NpcData } from "./export-npcs";
import { removeMatchingOutputs, writeJsonToTargets } from "./asm-utils";
import type { EggMovesData, LevelUpLearnsets, LevelUpMovesData } from "./export-data";
import type { PokegearLandmarksPayload } from "./export-pokegear-landmarks";

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
  "npcs",
  "pokegear_landmarks",
  "items",
  "trainers",
  "pokedex",
  "story_events",
  "phone_scripts",
] as const;

type ContentPackCategory = (typeof CONTENT_PACK_CATEGORIES)[number];
type ContentPackFiles = Record<ContentPackCategory, string[]>;

type ContentPack = {
  id: string;
  enabled?: boolean;
  priority?: number;
  path?: string;
  compiled?: string;
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
  evolutions: PokemonEvolutionData[];
  wildEncounters: WildEncounterData[];
  mapDimensions: Record<string, { width: number; height: number }>;
  mapAttributes: Record<string, unknown>;
  items: ExportedItem[];
  trainers: Trainer[];
  pokedex: PokedexData[];
  npcData: NpcData;
  pokegearLandmarks: PokegearLandmarksPayload;
};

const CORE_PACK_ID = "core-modular";
const CORE_PACK_PATH = `content-packs/${CORE_PACK_ID}`;
const MODULE_PREFIX = "module";

const toMapIdentity = (value: string): string => value.toLowerCase().replace(/[^a-z0-9]/g, "");

const normalizeFileName = (value: string): string =>
  value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");

const emptyContentPackFiles = (): ContentPackFiles =>
  Object.fromEntries(CONTENT_PACK_CATEGORIES.map((category) => [category, []])) as unknown as ContentPackFiles;

const asRecord = (value: unknown): Record<string, unknown> | null =>
  value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;

const isDefaultDvs = (value: unknown): boolean => {
  const record = asRecord(value);
  return Boolean(
    record &&
      record.attack === 0 &&
      record.defense === 0 &&
      record.speed === 0 &&
      record.special === 0 &&
      record.hp === 0
  );
};

const isDefaultStatBoosts = (value: unknown): boolean => {
  const record = asRecord(value);
  return Boolean(
    record &&
      record.HP === 0 &&
      record.ATTACK === 0 &&
      record.DEFENSE === 0 &&
      record.SPEED === 0 &&
      record.SPECIAL_ATTACK === 0 &&
      record.SPECIAL_DEFENSE === 0 &&
      record.ACCURACY === 0 &&
      record.EVASION === 0
  );
};

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

const isGeneratedDefaultPartyField = (
  key: string,
  value: unknown,
  speciesId: string,
  speciesHp: number
): boolean => {
  if (value === undefined) {
    return true;
  }
  if (key === "nickname") {
    return value === speciesId;
  }
  if (key === "item") {
    return value === null;
  }
  if (key === "moves") {
    return Array.isArray(value) && value.length === 0;
  }
  if (key === "hp" || key === "max_hp") {
    return value === speciesHp;
  }
  if (key === "original_trainer_name") {
    return value === "Trainer";
  }
  if (key === "original_trainer_id") {
    return value === 0;
  }
  if (key === "dvs") {
    return isDefaultDvs(value);
  }
  if (key === "stat_boosts") {
    return isDefaultStatBoosts(value);
  }
  if (typeof value === "number") {
    return value === 0;
  }
  if (typeof value === "boolean") {
    return value === false;
  }
  return false;
};

const partySpeciesInfo = (
  partyEntry: Trainer["party"][number],
  trainerLabel: string,
  partyIndex: number
): { speciesId: string; speciesHp: number } => {
  const source = partyEntry as unknown as Record<string, unknown>;
  const species = source.species;
  if (typeof species === "string" && species.trim()) {
    const hp = typeof source.hp === "number" ? source.hp : 0;
    return { speciesId: species, speciesHp: hp };
  }
  const speciesRecord = asRecord(species) as PokemonSpecies | null;
  if (speciesRecord && typeof speciesRecord.id === "string" && speciesRecord.id.trim()) {
    return {
      speciesId: speciesRecord.id,
      speciesHp: speciesRecord.base_stats?.hp ?? 0,
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
  const { speciesId, speciesHp } = partySpeciesInfo(partyEntry, trainerLabel, partyIndex);
  const compact: Record<string, unknown> = {
    species: speciesId,
    level: partyEntry.level,
  };

  for (const [key, value] of Object.entries(source)) {
    if (key === "species" || key === "level") {
      continue;
    }
    if (
      GENERATED_PARTY_FIELDS.has(key) &&
      isGeneratedDefaultPartyField(key, value, speciesId, speciesHp)
    ) {
      continue;
    }
    compact[key] = value;
  }

  return compact;
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
  const fileName = `${normalizeFileName(stem)}.json`;
  const relativePath = `${CORE_PACK_PATH}/${category}/${fileName}`;
  assertNoEmbeddedOwnedRecords(category, relativePath, payload);
  writeJsonToTargets(relativePath, payload, { indent: 2 });
  files.push(relativePath);
  return relativePath;
};

const collectJsonFiles = (relativeDir: ContentPackCategory): string[] => {
  const absoluteDir = joinPath(getDataDir(), relativeDir);
  if (!fs.existsSync(absoluteDir)) {
    return [];
  }
  return fs
    .readdirSync(absoluteDir)
    .filter((entry) => entry.toLowerCase().endsWith(".json"))
    .sort()
    .map((entry) => `${relativeDir}/${entry}`);
};

const writeJsonAssetFiles = (
  files: string[],
  category: ContentPackCategory,
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
    .filter((fileName) => fileName.toLowerCase().endsWith(".json"))
    .sort()) {
    const stem = entry.replace(/\.json$/i, "");
    const payload = readJsonAssetSync(joinPath(getDataDir(), sourceRelativeDir, entry));
    const relativePath = writePackEntry(files, category, stem, payload);
    writtenPayloads?.set(relativePath, payload);
    written.set(toMapIdentity(stem), relativePath);
  }
  return written;
};

const writeMapBlockEntries = (
  files: string[],
  writtenPayloads?: Map<string, unknown>
): Map<string, string> => {
  const written = new Map<string, string>();
  let payload: Record<string, string> = {};
  try {
    const raw = readJsonAssetSync<Record<string, string>>(joinPath(getDataDir(), "map_blocks.json"));
    if (raw && typeof raw === "object" && !Array.isArray(raw)) {
      payload = raw;
    }
  } catch {
    return written;
  }
  for (const [label, encoded] of Object.entries(payload).sort(([a], [b]) => a.localeCompare(b))) {
    if (typeof encoded !== "string") {
      continue;
    }
    const payload = { [label]: encoded };
    const relativePath = writePackEntry(files, "map_blocks", label, payload);
    writtenPayloads?.set(relativePath, payload);
    written.set(label, relativePath);
  }
  return written;
};

const normalizePack = (pack: Partial<ContentPack>): ContentPack => ({
  id: String(pack.id ?? ""),
  enabled: pack.enabled,
  priority: pack.priority,
  path: pack.path,
  compiled: pack.compiled,
  files: {
    ...emptyContentPackFiles(),
    ...(pack.files ?? {}),
  },
});

const readIndex = (): ContentPackIndex => {
  const indexPath = joinPath(getDataDir(), "content-packs/index.json");
  try {
    const parsed = readJsonAssetSync<ContentPackIndex>(indexPath);
    if (!parsed || typeof parsed !== "object" || !Array.isArray(parsed.packs)) {
      return { version: 1, packs: [] };
    }
    return {
      version: typeof parsed.version === "number" ? parsed.version : 1,
      packs: parsed.packs.map(normalizePack),
    };
  } catch {
    return { version: 1, packs: [] };
  }
};

const parentPathFor = (filePath: string, category: ContentPackCategory): string =>
  filePath.replace(new RegExp(`/${category}/[^/]+\\.json$`), "");

const fileStem = (filePath: string): string =>
  filePath.split("/").pop()?.replace(/\.json$/i, "") ?? "unknown";

const makeModulePack = (
  id: string,
  path: string,
  files: Partial<ContentPackFiles>
): ContentPack => ({
  id,
  enabled: false,
  priority: 0,
  path,
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
    writeCorePackEntry(files.pokemon, "pokemon", pokemon.id, pokemon);
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
  const mapPathByIdentity = writeJsonAssetFiles(files.maps, "maps", "maps", writtenPayloads);
  const mapBlockPathByLabel = writeMapBlockEntries(files.map_blocks, writtenPayloads);
  for (const encounter of payload.wildEncounters) {
    writeCorePackEntry(files.wild_encounters, "wild_encounters", encounter.map_name, encounter);
  }

  const mapAttributePathByIdentity = new Map<string, string>();
  const mapBlockPathByIdentity = new Map<string, string>();
  for (const [mapName, attributes] of Object.entries(payload.mapAttributes)) {
    const mapAttributePath = writeCorePackEntry(files.map_attributes, "map_attributes", mapName, {
      [mapName]: attributes,
    });
    mapAttributePathByIdentity.set(toMapIdentity(mapName), mapAttributePath);
    const record = asRecord(attributes);
    const blocksLabel = typeof record?.blocks_label === "string" ? record.blocks_label : `${mapName}_Blocks`;
    const mapBlockPath = mapBlockPathByLabel.get(blocksLabel);
    if (mapBlockPath) {
      mapBlockPathByIdentity.set(toMapIdentity(mapName), mapBlockPath);
    }
  }

  const mapDimensionPathByIdentity = new Map<string, string>();
  for (const [mapName, dimensions] of Object.entries(payload.mapDimensions)) {
    const mapDimensionPath = writeCorePackEntry(files.map_dimensions, "map_dimensions", mapName, {
      [mapName]: dimensions,
    });
    mapDimensionPathByIdentity.set(toMapIdentity(mapName), mapDimensionPath);
  }

  const npcPathByIdentity = new Map<string, string>();
  for (const [mapName, entries] of Object.entries(payload.npcData)) {
    const npcPath = writeCorePackEntry(files.npcs, "npcs", mapName, { [mapName]: entries });
    npcPathByIdentity.set(toMapIdentity(mapName), npcPath);
  }

  for (const item of payload.items) {
    writeCorePackEntry(files.items, "items", item.name, item);
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
  writeCorePackEntry(files.pokegear_landmarks, "pokegear_landmarks", "landmarks", payload.pokegearLandmarks);
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
    { category: "pokemon", prefix: "pokemon" },
    { category: "moves", prefix: "move" },
    { category: "learnsets", prefix: "learnset" },
    { category: "level_up_moves", prefix: "level-up-move" },
    { category: "egg_moves", prefix: "egg-move" },
    { category: "items", prefix: "item" },
    { category: "trainers", prefix: "trainer" },
    { category: "pokedex", prefix: "pokedex" },
    { category: "pokegear_landmarks", prefix: "pokegear-landmarks" },
    { category: "npcs", prefix: "npc" },
    { category: "story_events", prefix: "story" },
    { category: "phone_scripts", prefix: "phone" },
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

  for (const routePath of files.wild_encounters) {
    const name = fileStem(routePath);
    const routeIdentity = toMapIdentity(name);
    const mapPath = mapPathByIdentity.get(routeIdentity);
    const mapBlockPath = mapBlockPathByIdentity.get(routeIdentity);
    const mapAttributePath = mapAttributePathByIdentity.get(routeIdentity);
    const mapDimensionPath = mapDimensionPathByIdentity.get(routeIdentity);
    const npcPath = npcPathByIdentity.get(routeIdentity);
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

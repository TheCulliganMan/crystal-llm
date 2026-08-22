import { z } from "zod";
import { readJsonAssetSync } from "./asset-reader";
import { pokemonSpeciesDisplayName } from "./models/pokemon";
import { getDataDir } from "./paths";
import { joinPath, normalizePath } from "./path-utils";
import { assetExists } from "./asset-manifest";

export const CONTENT_PACK_CATEGORIES = [
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
  "fly_destinations",
  "battle_reward_rules",
  "step_event_rules",
  "field_moves",
  "decorations",
  "battle_tower_rules",
  "oak_ratings",
  "odd_egg_definitions",
  "npcs",
  "pokegear_landmarks",
  "items",
  "trainers",
  "pokedex",
  "story_events",
  "phone_scripts",
] as const;

export type ContentPackCategory = (typeof CONTENT_PACK_CATEGORIES)[number];

const ContentPackFilesSchema = z.object({
  pokemon: z.array(z.string()).default([]),
  moves: z.array(z.string()).default([]),
  learnsets: z.array(z.string()).default([]),
  level_up_moves: z.array(z.string()).default([]),
  egg_moves: z.array(z.string()).default([]),
  evolutions: z.array(z.string()).default([]),
  maps: z.array(z.string()).default([]),
  map_blocks: z.array(z.string()).default([]),
  map_attributes: z.array(z.string()).default([]),
  map_dimensions: z.array(z.string()).default([]),
  wild_encounters: z.array(z.string()).default([]),
  fly_destinations: z.array(z.string()).default([]),
  battle_reward_rules: z.array(z.string()).default([]),
  step_event_rules: z.array(z.string()).default([]),
  field_moves: z.array(z.string()).default([]),
  decorations: z.array(z.string()).default([]),
  battle_tower_rules: z.array(z.string()).default([]),
  oak_ratings: z.array(z.string()).default([]),
  odd_egg_definitions: z.array(z.string()).default([]),
  npcs: z.array(z.string()).default([]),
  pokegear_landmarks: z.array(z.string()).default([]),
  items: z.array(z.string()).default([]),
  trainers: z.array(z.string()).default([]),
  pokedex: z.array(z.string()).default([]),
  story_events: z.array(z.string()).default([]),
  phone_scripts: z.array(z.string()).default([]),
});

const ContentPackSchema = z.object({
  id: z.string(),
  enabled: z.boolean().optional().default(true),
  priority: z.number().optional().default(0),
  path: z.string().optional().default(""),
  compiled: z.string().nullable().optional(),
  compiled_json: z.string().nullable().optional(),
  files: ContentPackFilesSchema.default(() => ContentPackFilesSchema.parse({})),
});

const ContentPackIndexSchema = z.object({
  version: z.number().default(1),
  packs: z.array(ContentPackSchema).default([]),
});

const CompiledContentPackCategoriesSchema = z.object({
  pokemon: z.array(z.unknown()).default([]),
  moves: z.array(z.unknown()).default([]),
  learnsets: z.array(z.unknown()).default([]),
  level_up_moves: z.array(z.unknown()).default([]),
  egg_moves: z.array(z.unknown()).default([]),
  evolutions: z.array(z.unknown()).default([]),
  maps: z.array(z.unknown()).default([]),
  map_blocks: z.array(z.unknown()).default([]),
  map_attributes: z.array(z.unknown()).default([]),
  map_dimensions: z.array(z.unknown()).default([]),
  wild_encounters: z.array(z.unknown()).default([]),
  fly_destinations: z.array(z.unknown()).default([]),
  battle_reward_rules: z.array(z.unknown()).default([]),
  step_event_rules: z.array(z.unknown()).default([]),
  field_moves: z.array(z.unknown()).default([]),
  decorations: z.array(z.unknown()).default([]),
  battle_tower_rules: z.array(z.unknown()).default([]),
  oak_ratings: z.array(z.unknown()).default([]),
  odd_egg_definitions: z.array(z.unknown()).default([]),
  npcs: z.array(z.unknown()).default([]),
  pokegear_landmarks: z.array(z.unknown()).default([]),
  items: z.array(z.unknown()).default([]),
  trainers: z.array(z.unknown()).default([]),
  pokedex: z.array(z.unknown()).default([]),
  story_events: z.array(z.unknown()).default([]),
  phone_scripts: z.array(z.unknown()).default([]),
});

const CompiledContentPackSchema = z.object({
  version: z.number(),
  packId: z.string(),
  categories: CompiledContentPackCategoriesSchema.default(() =>
    CompiledContentPackCategoriesSchema.parse({})
  ),
});

type ContentPack = z.infer<typeof ContentPackSchema>;
type ContentPackIndex = z.infer<typeof ContentPackIndexSchema>;
type CompiledContentPack = z.infer<typeof CompiledContentPackSchema>;

const EMPTY_INDEX: ContentPackIndex = {
  version: 1,
  packs: [],
};

const INDEX_RELATIVE_PATH = "content-packs/index.json";
const CORE_PACK_ID = "core-modular";
const CORE_COMPILED_RELATIVE_PATH = "content-packs/core-modular.compiled.json";

let cachedIndex: ContentPackIndex | null = null;
const compiledPackCache = new Map<string, CompiledContentPack>();

const normalizeToken = (value: string): string => String(value ?? "").trim().toUpperCase();

const contentPackIndexPath = (): string => joinPath(getDataDir(), INDEX_RELATIVE_PATH);

const coreCompiledPackPath = (): string => joinPath(getDataDir(), CORE_COMPILED_RELATIVE_PATH);

const hasBundledCoreCompiledPack = (): boolean => assetExists(coreCompiledPackPath());

const implicitCompiledCoreIndex = (): ContentPackIndex => ({
  version: 1,
  packs: [
    {
      id: CORE_PACK_ID,
      enabled: true,
      priority: -100,
      compiled: CORE_COMPILED_RELATIVE_PATH,
      path: "content-packs/core-modular",
      files: ContentPackFilesSchema.parse({}),
    },
  ],
});

const withImplicitCompiledCorePack = (pack: ContentPack): ContentPack => {
  if (pack.compiled || pack.id !== CORE_PACK_ID || pack.enabled === false) {
    return pack;
  }
  return hasBundledCoreCompiledPack()
    ? { ...pack, compiled: CORE_COMPILED_RELATIVE_PATH }
    : pack;
};

const readOptionalJsonSync = (path: string): unknown | null => {
  try {
    return readJsonAssetSync(path);
  } catch {
    return null;
  }
};

const resolvePackFilePath = (entry: string): string => {
  const normalized = normalizePath(String(entry ?? ""));
  if (!normalized) {
    throw new Error("Encountered an empty content-pack file entry.");
  }
  if (normalized.startsWith("/")) {
    return normalized;
  }
  if (normalized.startsWith("assets/data/")) {
    return `/${normalized}`;
  }
  return joinPath(getDataDir(), normalized);
};

const packSort = (a: ContentPack, b: ContentPack): number => {
  const priorityDiff = (a.priority ?? 0) - (b.priority ?? 0);
  if (priorityDiff !== 0) {
    return priorityDiff;
  }
  return a.id.localeCompare(b.id);
};

export const resetContentPackCache = (): void => {
  cachedIndex = null;
  compiledPackCache.clear();
};

export const loadContentPackIndexSync = (): ContentPackIndex => {
  if (cachedIndex) {
    return cachedIndex;
  }
  const raw = readOptionalJsonSync(contentPackIndexPath());
  if (!raw) {
    cachedIndex = hasBundledCoreCompiledPack() ? implicitCompiledCoreIndex() : EMPTY_INDEX;
    return cachedIndex;
  }
  const parsed = ContentPackIndexSchema.parse(raw);
  cachedIndex = {
    version: parsed.version,
    packs: parsed.packs.map(withImplicitCompiledCorePack).sort(packSort),
  };
  return cachedIndex;
};

const enabledPacks = (): ContentPack[] =>
  loadContentPackIndexSync().packs.filter((pack) => pack.enabled !== false);

const loadCompiledPackSync = (pack: ContentPack): CompiledContentPack => {
  const compiledEntry = pack.compiled_json ?? (
    pack.compiled?.endsWith(".crystalpack")
      ? pack.compiled.replace(/\.crystalpack$/, ".compiled.json")
      : pack.compiled
  );
  if (!compiledEntry) {
    throw new Error(`Content pack ${pack.id} does not declare a compiled asset.`);
  }
  const compiledPath = resolvePackFilePath(compiledEntry);
  const cached = compiledPackCache.get(compiledPath);
  if (cached) {
    return cached;
  }
  try {
    const parsed = CompiledContentPackSchema.parse(readJsonAssetSync(compiledPath));
    if (parsed.packId !== pack.id) {
      throw new Error(`expected packId ${pack.id}, received ${parsed.packId}`);
    }
    compiledPackCache.set(compiledPath, parsed);
    return parsed;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(
      `Unable to load compiled content pack ${pack.id} from ${compiledPath}: ${message}`
    );
  }
};

export const listContentPackFilesSync = (category: ContentPackCategory): string[] => {
  const files: string[] = [];
  for (const pack of enabledPacks()) {
    const entries = pack.files[category] ?? [];
    for (const entry of entries) {
      files.push(resolvePackFilePath(entry));
    }
  }
  return files;
};

export const hasEnabledCompiledContentPackSync = (): boolean =>
  hasBundledCoreCompiledPack() || enabledPacks().some((pack) => Boolean(pack.compiled || pack.compiled_json));

export const loadContentPackCategoryJsonSync = (category: ContentPackCategory): unknown[] => {
  const payloads: unknown[] = [];
  for (const pack of enabledPacks()) {
    if (pack.compiled) {
      payloads.push(...loadCompiledPackSync(pack).categories[category]);
      continue;
    }
    const entries = pack.files[category] ?? [];
    for (const entry of entries) {
      const filePath = resolvePackFilePath(entry);
      try {
        payloads.push(readJsonAssetSync(filePath));
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        throw new Error(
          `Unable to load content pack file ${filePath} for category ${category}: ${message}`
        );
      }
    }
  }
  return payloads;
};

const asObject = (value: unknown): Record<string, unknown> | null =>
  value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;

const looksLikeSinglePokemon = (value: Record<string, unknown>): boolean =>
  typeof value.id === "string";

const applyPokemonPayload = (target: Map<string, unknown>, payload: unknown): void => {
  if (Array.isArray(payload)) {
    for (const entry of payload) {
      const record = asObject(entry);
      if (!record || typeof record.id !== "string") {
        continue;
      }
      target.set(normalizeToken(record.id), record);
    }
    return;
  }
  const record = asObject(payload);
  if (!record) {
    return;
  }
  if (looksLikeSinglePokemon(record)) {
    target.set(normalizeToken(String(record.id)), record);
    return;
  }
  for (const [key, value] of Object.entries(record)) {
    const entryRecord = asObject(value);
    if (!entryRecord) {
      continue;
    }
    const id = typeof entryRecord.id === "string" ? entryRecord.id : key;
    target.set(normalizeToken(id), entryRecord);
  }
};

const looksLikeSingleMove = (value: Record<string, unknown>): boolean =>
  typeof value.name === "string" && typeof value.type === "string";

const applyMovePayload = (target: Map<string, unknown>, payload: unknown): void => {
  if (Array.isArray(payload)) {
    for (const entry of payload) {
      const record = asObject(entry);
      if (!record || typeof record.name !== "string") {
        continue;
      }
      target.set(normalizeToken(record.name), record);
    }
    return;
  }
  const record = asObject(payload);
  if (!record) {
    return;
  }
  if (looksLikeSingleMove(record)) {
    target.set(normalizeToken(String(record.name)), record);
    return;
  }
  for (const [key, value] of Object.entries(record)) {
    const moveRecord = asObject(value);
    if (!moveRecord) {
      continue;
    }
    const moveName = typeof moveRecord.name === "string" ? moveRecord.name : key;
    target.set(normalizeToken(moveName), moveRecord);
  }
};

const applyLearnsetPayload = (target: Map<string, unknown>, payload: unknown): void => {
  const single = asObject(payload);
  if (single && typeof single.species === "string" && Array.isArray(single.learnset)) {
    target.set(normalizeToken(single.species), single.learnset);
    return;
  }
  if (single) {
    for (const [key, value] of Object.entries(single)) {
      if (Array.isArray(value)) {
        target.set(normalizeToken(key), value);
      }
    }
    return;
  }
  if (Array.isArray(payload)) {
    for (const entry of payload) {
      const record = asObject(entry);
      if (!record || typeof record.species !== "string" || !Array.isArray(record.learnset)) {
        continue;
      }
      target.set(normalizeToken(record.species), record.learnset);
    }
  }
};

const applyLevelUpMovesPayload = (target: Map<string, unknown>, payload: unknown): void => {
  const single = asObject(payload);
  if (single && typeof single.species === "string" && Array.isArray(single.moves)) {
    target.set(normalizeToken(single.species), single.moves);
    return;
  }
  if (single) {
    for (const [key, value] of Object.entries(single)) {
      if (Array.isArray(value)) {
        target.set(normalizeToken(key), value);
      }
    }
    return;
  }
  if (Array.isArray(payload)) {
    for (const entry of payload) {
      const record = asObject(entry);
      if (!record || typeof record.species !== "string" || !Array.isArray(record.moves)) {
        continue;
      }
      target.set(normalizeToken(record.species), record.moves);
    }
  }
};

const applyEggMovesPayload = (target: Map<string, unknown>, payload: unknown): void => {
  const single = asObject(payload);
  if (single && typeof single.species === "string" && Array.isArray(single.moves)) {
    target.set(normalizeToken(single.species), single.moves);
    return;
  }
  if (!single) {
    return;
  }
  for (const [key, value] of Object.entries(single)) {
    if (Array.isArray(value)) {
      target.set(normalizeToken(key), value);
    }
  }
};

const applyEvolutionsPayload = (target: Map<string, unknown>, payload: unknown): void => {
  if (Array.isArray(payload)) {
    for (const entry of payload) {
      const record = asObject(entry);
      if (!record || typeof record.species !== "string") {
        continue;
      }
      target.set(normalizeToken(record.species), record);
    }
    return;
  }
  const record = asObject(payload);
  if (!record) {
    return;
  }
  if (typeof record.species === "string") {
    target.set(normalizeToken(record.species), record);
    return;
  }
  for (const [key, value] of Object.entries(record)) {
    if (Array.isArray(value)) {
      target.set(normalizeToken(key), { species: key, evolutions: value });
      continue;
    }
    const entry = asObject(value);
    if (entry) {
      const species = typeof entry.species === "string" ? entry.species : key;
      target.set(normalizeToken(species), { ...entry, species });
    }
  }
};

const mapNameFromRecord = (record: Record<string, unknown>): string | null => {
  const candidates = [record.map_name, record.mapName, record.name, record.map];
  for (const candidate of candidates) {
    if (typeof candidate === "string" && candidate.trim()) {
      return candidate;
    }
  }
  return null;
};

const applyMapPayload = (target: Map<string, unknown>, payload: unknown): void => {
  if (Array.isArray(payload)) {
    for (const entry of payload) {
      const record = asObject(entry);
      if (!record) {
        continue;
      }
      const mapName = mapNameFromRecord(record);
      if (mapName) {
        target.set(mapName, record);
      }
    }
    return;
  }
  const record = asObject(payload);
  if (!record) {
    return;
  }
  const singleMapName = mapNameFromRecord(record);
  if (singleMapName && typeof record.tileset_name === "string") {
    target.set(singleMapName, record);
    return;
  }
  for (const [key, value] of Object.entries(record)) {
    const entryRecord = asObject(value);
    if (!entryRecord) {
      continue;
    }
    target.set(key, entryRecord);
  }
};

const applyMapDimensionsPayload = (target: Map<string, unknown>, payload: unknown): void => {
  if (Array.isArray(payload)) {
    for (const entry of payload) {
      const record = asObject(entry);
      if (!record) {
        continue;
      }
      const mapName = mapNameFromRecord(record);
      if (mapName) {
        target.set(mapName, record);
      }
    }
    return;
  }
  const record = asObject(payload);
  if (!record) {
    return;
  }
  const singleMapName = mapNameFromRecord(record);
  if (singleMapName && ("width" in record || "height" in record)) {
    target.set(singleMapName, record);
    return;
  }
  for (const [key, value] of Object.entries(record)) {
    const entryRecord = asObject(value);
    if (!entryRecord) {
      continue;
    }
    target.set(key, entryRecord);
  }
};

const applyMapBlocksPayload = (target: Map<string, string>, payload: unknown): void => {
  const record = asObject(payload);
  if (!record) {
    return;
  }
  for (const [label, encoded] of Object.entries(record)) {
    if (typeof encoded === "string") {
      target.set(label, encoded);
    }
  }
};

const applyWildEncounterPayload = (target: Map<string, unknown>, payload: unknown): void => {
  if (Array.isArray(payload)) {
    for (const entry of payload) {
      const record = asObject(entry);
      if (!record || typeof record.map_name !== "string") {
        continue;
      }
      target.set(record.map_name, record);
    }
    return;
  }
  const record = asObject(payload);
  if (record && typeof record.map_name === "string") {
    target.set(record.map_name, record);
  }
};

const applyItemsPayload = (target: Map<string, unknown>, payload: unknown): void => {
  if (Array.isArray(payload)) {
    for (const entry of payload) {
      const record = asObject(entry);
      if (!record || typeof record.name !== "string") {
        continue;
      }
      target.set(normalizeToken(record.name), record);
    }
    return;
  }
  const record = asObject(payload);
  if (record && typeof record.name === "string") {
    target.set(normalizeToken(record.name), record);
    return;
  }
  if (record) {
    for (const [key, value] of Object.entries(record)) {
      const item = asObject(value);
      if (!item) {
        continue;
      }
      const itemName = typeof item.name === "string" ? item.name : key;
      target.set(normalizeToken(itemName), item);
    }
  }
};

const trainerToken = (record: Record<string, unknown>): string | null => {
  if (typeof record.trainer_id === "string" && record.trainer_id.trim()) {
    return normalizeToken(record.trainer_id);
  }
  if (typeof record.name === "string" && record.name.trim()) {
    return normalizeToken(record.name);
  }
  return null;
};

type SpeciesRecord = Record<string, unknown> & {
  id?: string;
  base_stats?: {
    hp?: number;
  };
};

type PokemonDataByIdResolver = () => Record<string, unknown>;

const DEFAULT_DVS = { attack: 0, defense: 0, speed: 0, special: 0, hp: 0 };

const DEFAULT_STAT_BOOSTS = {
  HP: 0,
  ATTACK: 0,
  DEFENSE: 0,
  SPEED: 0,
  SPECIAL_ATTACK: 0,
  SPECIAL_DEFENSE: 0,
  ACCURACY: 0,
  EVASION: 0,
};

const compactPokemonDefaults = (species: SpeciesRecord, level: number): Record<string, unknown> => {
  const hp = typeof species.base_stats?.hp === "number" ? species.base_stats.hp : 0;
  return {
    species,
    nickname: pokemonSpeciesDisplayName(species),
    level,
    item: null,
    moves: [],
    hp,
    max_hp: hp,
    original_trainer_name: "Trainer",
    original_trainer_id: 0,
    experience: 0,
    happiness: 0,
    dvs: { ...DEFAULT_DVS },
    sleep_turns: 0,
    flinching: false,
    rampage_turns: 0,
    confusion_turns: 0,
    perish_song_turns: 0,
    focus_energy: false,
    hp_exp: 0,
    attack_exp: 0,
    defense_exp: 0,
    speed_exp: 0,
    special_exp: 0,
    turns_in_battle: 0,
    stat_boosts: { ...DEFAULT_STAT_BOOSTS },
    locked_turns_remaining: 0,
    trapped_turns: 0,
    leech_seeded: false,
    nightmare: false,
    cursed: false,
    attack: 0,
    defense: 0,
    speed: 0,
    special_attack: 0,
    special_defense: 0,
    disable_turns: 0,
    encore_turns_remaining: 0,
    destiny_bond_active: false,
    pokerus: false,
    rage_active: false,
    rage_counter: 0,
    fury_cutter_count: 0,
    rollout_step: 0,
    rollout_active: false,
    defense_curled: false,
    cant_run: false,
    bide_active: false,
    bide_turns_remaining: 0,
    bide_damage: 0,
    protect_active: false,
    protect_counter: 0,
    endure_active: false,
    endure_counter: 0,
    foresight_active: false,
    lock_on_active: false,
    substitute_hp: 0,
    transformed: false,
    last_damage_taken: 0,
  };
};

const resolveTrainerPartySpecies = (
  speciesId: string,
  trainerLabel: string,
  pokemonDataById: PokemonDataByIdResolver
): SpeciesRecord => {
  const normalizedSpeciesId = normalizeToken(speciesId);
  const species = asObject(pokemonDataById()[normalizedSpeciesId]) as SpeciesRecord | null;
  if (!species) {
    throw new Error(
      `Unable to resolve species ${speciesId} for trainer ${trainerLabel} content-pack party entry.`
    );
  }
  return species;
};

const hydrateTrainerPartyEntry = (
  entry: unknown,
  trainerLabel: string,
  pokemonDataById: PokemonDataByIdResolver
): unknown => {
  const record = asObject(entry);
  if (!record || typeof record.species !== "string") {
    return entry;
  }
  const species = resolveTrainerPartySpecies(record.species, trainerLabel, pokemonDataById);
  const level = typeof record.level === "number" ? record.level : 1;
  return {
    ...compactPokemonDefaults(species, level),
    ...record,
    species,
  };
};

const hydrateTrainerRecord = (
  record: Record<string, unknown>,
  pokemonDataById: PokemonDataByIdResolver
): Record<string, unknown> => {
  if (!Array.isArray(record.party)) {
    return record;
  }
  const trainerLabel = trainerToken(record) ?? "unknown";
  return {
    ...record,
    party: record.party.map((entry) => hydrateTrainerPartyEntry(entry, trainerLabel, pokemonDataById)),
  };
};

const applyTrainerPayload = (
  target: Map<string, unknown>,
  payload: unknown,
  pokemonDataById: PokemonDataByIdResolver
): void => {
  if (Array.isArray(payload)) {
    for (const entry of payload) {
      const record = asObject(entry);
      if (!record) {
        continue;
      }
      const key = trainerToken(record);
      if (key) {
        target.set(key, hydrateTrainerRecord(record, pokemonDataById));
      }
    }
    return;
  }
  const record = asObject(payload);
  if (!record) {
    return;
  }
  const singleKey = trainerToken(record);
  if (singleKey) {
    target.set(singleKey, hydrateTrainerRecord(record, pokemonDataById));
    return;
  }
  for (const [key, value] of Object.entries(record)) {
    const trainerRecord = asObject(value);
    if (!trainerRecord) {
      continue;
    }
    const token = trainerToken(trainerRecord) ?? normalizeToken(key);
    target.set(token, hydrateTrainerRecord(trainerRecord, pokemonDataById));
  }
};

const applyPokedexPayload = (target: Map<string, unknown>, payload: unknown): void => {
  if (Array.isArray(payload)) {
    for (const entry of payload) {
      const record = asObject(entry);
      if (!record || typeof record.species !== "string") {
        continue;
      }
      target.set(normalizeToken(record.species), record);
    }
    return;
  }
  const record = asObject(payload);
  if (!record) {
    return;
  }
  if (typeof record.species === "string") {
    target.set(normalizeToken(record.species), record);
    return;
  }
  for (const [key, value] of Object.entries(record)) {
    const entry = asObject(value);
    if (!entry) {
      continue;
    }
    const species = typeof entry.species === "string" ? entry.species : key;
    target.set(normalizeToken(species), entry);
  }
};

const applyNpcPayload = (target: Map<string, unknown>, payload: unknown): void => {
  const record = asObject(payload);
  if (!record) {
    return;
  }
  if (typeof record.map_name === "string" && Array.isArray(record.npcs)) {
    target.set(record.map_name, record.npcs);
    return;
  }
  for (const [mapName, entries] of Object.entries(record)) {
    if (Array.isArray(entries)) {
      target.set(mapName, entries);
    }
  }
};

const loadBaseRecord = (fileName: string): Map<string, unknown> => {
  const source = readOptionalJsonSync(joinPath(getDataDir(), fileName));
  const mapped = new Map<string, unknown>();
  const record = asObject(source);
  if (!record) {
    return mapped;
  }
  for (const [key, value] of Object.entries(record)) {
    mapped.set(key, value);
  }
  return mapped;
};

export const loadMergedPokemonDataSync = (): Record<string, unknown> => {
  const merged = new Map<string, unknown>();
  applyPokemonPayload(merged, readOptionalJsonSync(joinPath(getDataDir(), "pokemon_data.json")));
  for (const payload of loadContentPackCategoryJsonSync("pokemon")) {
    applyPokemonPayload(merged, payload);
  }
  return Object.fromEntries(merged.entries());
};

export const loadMergedMovesDataSync = (): Record<string, unknown> => {
  const merged = new Map<string, unknown>();
  applyMovePayload(merged, readOptionalJsonSync(joinPath(getDataDir(), "moves_data.json")));
  for (const payload of loadContentPackCategoryJsonSync("moves")) {
    applyMovePayload(merged, payload);
  }
  return Object.fromEntries(merged.entries());
};

export const loadMergedLearnsetsSync = (): Record<string, unknown> => {
  const merged = new Map<string, unknown>();
  applyLearnsetPayload(merged, readOptionalJsonSync(joinPath(getDataDir(), "learnsets.json")));
  for (const payload of loadContentPackCategoryJsonSync("learnsets")) {
    applyLearnsetPayload(merged, payload);
  }
  return Object.fromEntries(merged.entries());
};

export const loadMergedLevelUpMovesSync = (): Record<string, unknown> => {
  const merged = new Map<string, unknown>();
  applyLevelUpMovesPayload(
    merged,
    readOptionalJsonSync(joinPath(getDataDir(), "level_up_moves.json"))
  );
  for (const payload of loadContentPackCategoryJsonSync("level_up_moves")) {
    applyLevelUpMovesPayload(merged, payload);
  }
  return Object.fromEntries(merged.entries());
};

export const loadMergedEggMovesSync = (): Record<string, unknown> => {
  const merged = new Map<string, unknown>();
  applyEggMovesPayload(merged, readOptionalJsonSync(joinPath(getDataDir(), "egg_moves.json")));
  for (const payload of loadContentPackCategoryJsonSync("egg_moves")) {
    applyEggMovesPayload(merged, payload);
  }
  return Object.fromEntries(merged.entries());
};

export const loadMergedEvolutionsSync = (): unknown[] => {
  const merged = new Map<string, unknown>();
  applyEvolutionsPayload(merged, readOptionalJsonSync(joinPath(getDataDir(), "evolutions.json")));
  for (const payload of loadContentPackCategoryJsonSync("evolutions")) {
    applyEvolutionsPayload(merged, payload);
  }
  return Array.from(merged.values());
};

export const loadMergedMapBlocksSync = (): Record<string, string> => {
  const merged = new Map<string, string>();
  applyMapBlocksPayload(merged, readOptionalJsonSync(joinPath(getDataDir(), "map_blocks.json")));
  for (const payload of loadContentPackCategoryJsonSync("map_blocks")) {
    applyMapBlocksPayload(merged, payload);
  }
  return Object.fromEntries(merged.entries());
};

export const loadMergedMapAttributesSync = (): Record<string, unknown> => {
  const merged = loadBaseRecord("map_attributes.json");
  for (const payload of loadContentPackCategoryJsonSync("map_attributes")) {
    applyMapPayload(merged, payload);
  }
  return Object.fromEntries(merged.entries());
};

export const loadMergedMapDimensionsSync = (): Record<string, unknown> => {
  const merged = loadBaseRecord("map_dimensions.json");
  for (const payload of loadContentPackCategoryJsonSync("map_dimensions")) {
    applyMapDimensionsPayload(merged, payload);
  }
  return Object.fromEntries(merged.entries());
};

export const loadMergedWildEncountersSync = (): unknown[] => {
  const merged = new Map<string, unknown>();
  applyWildEncounterPayload(
    merged,
    readOptionalJsonSync(joinPath(getDataDir(), "wild_encounters.json"))
  );
  for (const payload of loadContentPackCategoryJsonSync("wild_encounters")) {
    applyWildEncounterPayload(merged, payload);
  }
  return Array.from(merged.values());
};

export const loadMergedNpcDataSync = (): Record<string, unknown> => {
  const merged = loadBaseRecord("npcs.json");
  for (const payload of loadContentPackCategoryJsonSync("npcs")) {
    applyNpcPayload(merged, payload);
  }
  return Object.fromEntries(merged.entries());
};

export const loadMergedItemsSync = (): unknown[] => {
  const merged = new Map<string, unknown>();
  applyItemsPayload(merged, readOptionalJsonSync(joinPath(getDataDir(), "items.json")));
  for (const payload of loadContentPackCategoryJsonSync("items")) {
    applyItemsPayload(merged, payload);
  }
  return Array.from(merged.values());
};

export const loadMergedTrainersSync = (): unknown[] => {
  const merged = new Map<string, unknown>();
  let cachedPokemonData: Record<string, unknown> | null = null;
  const pokemonDataById = (): Record<string, unknown> => {
    if (!cachedPokemonData) {
      cachedPokemonData = loadMergedPokemonDataSync();
    }
    return cachedPokemonData;
  };
  applyTrainerPayload(merged, readOptionalJsonSync(joinPath(getDataDir(), "trainers.json")), pokemonDataById);
  for (const payload of loadContentPackCategoryJsonSync("trainers")) {
    applyTrainerPayload(merged, payload, pokemonDataById);
  }
  return Array.from(merged.values());
};

export const loadMergedPokedexSync = (): unknown[] => {
  const merged = new Map<string, unknown>();
  applyPokedexPayload(merged, readOptionalJsonSync(joinPath(getDataDir(), "pokedex.json")));
  for (const payload of loadContentPackCategoryJsonSync("pokedex")) {
    applyPokedexPayload(merged, payload);
  }
  return Array.from(merged.values());
};

type PokegearLandmarkPayload = {
  landmarks: Array<Record<string, unknown>>;
  map_to_landmark: Record<string, string>;
};

const normalizeLandmarkKey = (entry: Record<string, unknown>): string => {
  const constant = typeof entry.constant === "string" ? normalizeToken(entry.constant) : "";
  if (constant) {
    return `constant:${constant}`;
  }
  const id = typeof entry.id === "number" ? entry.id : Number.NaN;
  if (!Number.isNaN(id)) {
    return `id:${id}`;
  }
  return `json:${JSON.stringify(entry)}`;
};

export const mergePokegearLandmarksPayload = (base: unknown): PokegearLandmarkPayload => {
  const defaultBase: PokegearLandmarkPayload = {
    landmarks: [],
    map_to_landmark: {},
  };
  const baseRecord = asObject(base);
  const merged: PokegearLandmarkPayload = {
    landmarks: Array.isArray(baseRecord?.landmarks)
      ? (baseRecord?.landmarks as Array<Record<string, unknown>>)
      : defaultBase.landmarks,
    map_to_landmark:
      baseRecord && asObject(baseRecord.map_to_landmark)
        ? Object.fromEntries(
            Object.entries(baseRecord.map_to_landmark as Record<string, unknown>).map(
              ([key, value]) => [key, String(value)]
            )
          )
        : defaultBase.map_to_landmark,
  };

  const landmarkMap = new Map<string, Record<string, unknown>>();
  for (const entry of merged.landmarks) {
    landmarkMap.set(normalizeLandmarkKey(entry), entry);
  }

  for (const payload of loadContentPackCategoryJsonSync("pokegear_landmarks")) {
    const record = asObject(payload);
    if (!record) {
      continue;
    }
    const landmarks = Array.isArray(record.landmarks) ? record.landmarks : [];
    for (const entry of landmarks) {
      const landmark = asObject(entry);
      if (!landmark) {
        continue;
      }
      landmarkMap.set(normalizeLandmarkKey(landmark), landmark);
    }
    const mapping = asObject(record.map_to_landmark);
    if (mapping) {
      for (const [mapName, landmark] of Object.entries(mapping)) {
        merged.map_to_landmark[mapName] = String(landmark);
      }
    }
  }

  merged.landmarks = Array.from(landmarkMap.values());
  return merged;
};

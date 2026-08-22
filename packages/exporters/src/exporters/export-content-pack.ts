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
import {
  ensureDir,
  removeMatchingOutputs,
  writeJsonToTargets,
} from "./asm-utils";
import type {
  EggMovesData,
  GrowthRateCurveData,
  LevelUpLearnsets,
  LevelUpMovesData,
} from "./export-data";
import type { PokegearLandmarksPayload } from "./export-pokegear-landmarks";
import type { PlayabilityRules } from "./export-playability";
import type { ExportedAudioAsset } from "./export-audio-assets";
import type { ExportedPokemonEvolutionData } from "./export-evolutions";
import type { ExportedFieldEncounterData } from "./export-field-encounters";
import type { ExportedFishingCatalog } from "./export-fishing";
import type { ExportedFieldMoveCatalog } from "./export-field-moves";
import type { ExportedFieldBoxItemRule } from "./export-field-box-items";
import type { ExportedDecorationCatalog } from "./export-decorations";
import type { ExportedFlyDestinationTable } from "./export-fly-destinations";
import type { ExportedBattleRewardRules } from "./export-battle-reward-rules";
import type { ExportedBattleEscapeRules } from "./export-battle-escape-rules";
import type { ExportedStepEventRules } from "./export-step-event-rules";
import type { ExportedCaptureRules } from "./export-capture-rules";
import type { ExportedFruitTreeCatalog } from "./export-fruit-trees";
import type { CurrencyConstantsPayload } from "./export-currency-constants";
import type {
  BugContestConfig,
  NpcTradeDefinition,
  PermanentPhoneNumberDefinition,
  SpecialPhoneCallDefinition,
} from "./export-runtime-assets";

const CONTENT_PACK_CATEGORIES = [
  "pokemon",
  "moves",
  "growth_rates",
  "learnsets",
  "level_up_moves",
  "egg_moves",
  "evolutions",
  "maps",
  "map_scripts",
  "map_blocks",
  "map_attributes",
  "map_dimensions",
  "wild_encounters",
  "field_encounters",
  "runtime_spawn_points",
  "fly_destinations",
  "runtime_map_metadata",
  "flee_mons",
  "roaming_pokemon",
  "buena_password_categories",
  "buena_prizes",
  "kurt_apricorn_recipes",
  "shuckie_gift",
  "dratini_move_sets",
  "bug_contest_config",
  "battle_tower_rules",
  "oak_ratings",
  "odd_egg_definitions",
  "magikarp_lengths",
  "happiness_data",
  "encounter_slot_tables",
  "encounter_music_modifiers",
  "battle_stat_multipliers",
  "capture_wobble_probabilities",
  "capture_rules",
  "move_priorities",
  "type_categories",
  "type_effectiveness",
  "weather_modifiers",
  "battle_reward_rules",
  "battle_escape_rules",
  "step_event_rules",
  "fishing",
  "field_moves",
  "field_box_items",
  "decorations",
  "runtime_title_screen",
  "fruit_trees",
  "npcs",
  "pokegear_landmarks",
  "pc_strings",
  "menu_icons",
  "items",
  "marts",
  "currency_constants",
  "trainers",
  "trainer_class_names",
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

type ExportedRuntimeTitleScreen = {
  title_music: string | null;
};

type StoryEventScriptConstantsPayload = {
  global?: Record<string, unknown>;
  maps?: Record<string, Record<string, unknown>>;
};

export type CoreExportPayload = {
  pokemonData: PokemonSpecies[];
  movesData: Record<string, Move>;
  growthRatesData?: GrowthRateCurveData[];
  learnsetsData: LevelUpLearnsets;
  levelUpMovesData: LevelUpMovesData;
  eggMovesData: EggMovesData;
  evolutions: ExportedPokemonEvolutionData[];
  wildEncounters: WildEncounterData[];
  fieldEncounters?: ExportedFieldEncounterData[];
  fishing?: ExportedFishingCatalog;
  fieldMoves?: ExportedFieldMoveCatalog;
  fieldBoxItems?: Record<string, ExportedFieldBoxItemRule>;
  decorations?: ExportedDecorationCatalog;
  runtimeTitleScreen?: ExportedRuntimeTitleScreen;
  flyDestinations?: ExportedFlyDestinationTable;
  fruitTrees?: ExportedFruitTreeCatalog;
  runtimeSpawnPoints?: unknown;
  runtimeMapMetadata?: unknown;
  fleeMons?: unknown;
  roamingPokemon?: unknown;
  buenaPasswordCategories?: unknown;
  buenaPrizes?: unknown;
  kurtApricornRecipes?: unknown;
  shuckieGift?: unknown;
  dratiniMoveSets?: unknown;
  bugContestConfig?: BugContestConfig;
  battleTowerRules?: unknown;
  oakRatings?: unknown;
  oddEggDefinitions?: unknown;
  magikarpLengths?: unknown;
  happinessData?: unknown;
  encounterSlotTables?: unknown;
  encounterMusicModifiers?: unknown;
  battleStatMultipliers?: unknown;
  captureWobbleProbabilities?: unknown;
  captureRules?: ExportedCaptureRules;
  movePriorities?: unknown;
  typeCategories?: unknown;
  typeEffectiveness?: unknown;
  weatherModifiers?: unknown;
  battleRewardRules?: ExportedBattleRewardRules;
  battleEscapeRules?: ExportedBattleEscapeRules;
  stepEventRules?: ExportedStepEventRules;
  mapDimensions: Record<string, { width: number; height: number }>;
  mapAttributes: Record<string, unknown>;
  items: ExportedItem[];
  marts?: Record<string, string[]>;
  currencyConstants?: CurrencyConstantsPayload;
  pcStrings?: Record<string, string>;
  menuIcons?: Record<string, string>;
  pokedexEntries?: Record<string, unknown>;
  pokemonFrontpicAnimations?: Record<string, unknown>;
  initializeEvents?: unknown;
  storyEventScriptConstants?: unknown;
  phoneContacts?: Record<string, unknown>;
  permanentPhoneNumbers?: Record<string, PermanentPhoneNumberDefinition>;
  specialPhoneCalls?: Record<string, SpecialPhoneCallDefinition>;
  npcTrades?: Record<string, NpcTradeDefinition>;
  specialRoutines?: Record<string, Record<string, never>>;
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
  trainerClassNames: Record<string, string>;
  pokedex: PokedexData[];
  npcData: NpcData;
  pokegearLandmarks: PokegearLandmarksPayload;
  playability?: PlayabilityRules;
  audioAssets?: Record<string, ExportedAudioAsset>;
};

function storyEventConstantsWithFieldBoxFlags(
  constants: unknown,
  fieldBoxItems: Record<string, ExportedFieldBoxItemRule> | undefined,
): unknown {
  if (!fieldBoxItems || Object.keys(fieldBoxItems).length === 0) {
    return constants;
  }
  const payload =
    constants && typeof constants === "object"
      ? (constants as StoryEventScriptConstantsPayload)
      : {};
  const global = { ...(payload.global ?? {}) };
  for (const rule of Object.values(fieldBoxItems).sort((a, b) =>
    a.item_id.localeCompare(b.item_id),
  )) {
    if (rule.decoration_flag && global[rule.decoration_flag] === undefined) {
      global[rule.decoration_flag] = 0;
    }
  }
  return {
    ...payload,
    global,
    maps: payload.maps ?? {},
  };
}

const CORE_PACK_ID = "core-modular";
const CORE_PACK_PATH = `content-packs/${CORE_PACK_ID}`;
const CORE_RUNTIME_PACK_PATH = "content-packs/core-modular.crystalpack";
const CORE_GENERATED_PACK_MANIFEST_PATH =
  "content-packs/core-modular.generated.json";
const MODULE_PREFIX = "module";

type RuntimeSpawnPointPayload = {
  identifier: number;
  mapConstant: string;
  mapName: string;
  groupId: number;
  mapId: number;
  tileX: number;
  tileY: number;
  groupName: string;
  metatileX: number;
  metatileY: number;
  subtileX: number;
  subtileY: number;
};

const requireIntegerField = (
  spawn: Record<string, unknown>,
  key: keyof RuntimeSpawnPointPayload,
  id: string,
): number => {
  const value = spawn[key];
  if (!Number.isInteger(value)) {
    throw new Error(`Runtime spawn point ${id} requires integer ${key}`);
  }
  return value as number;
};

const requireStringField = (
  spawn: Record<string, unknown>,
  key: keyof RuntimeSpawnPointPayload,
  id: string,
): string => {
  const value = spawn[key];
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`Runtime spawn point ${id} requires exact ${key}`);
  }
  return value;
};

export const alignRuntimeSpawnPoints = (
  runtimeSpawnPoints: unknown,
): Record<string, RuntimeSpawnPointPayload> => {
  if (
    typeof runtimeSpawnPoints !== "object" ||
    runtimeSpawnPoints === null ||
    Array.isArray(runtimeSpawnPoints)
  ) {
    throw new Error("Runtime spawn points payload must be an object map");
  }
  const aligned: Record<string, RuntimeSpawnPointPayload> = {};
  for (const [id, rawSpawn] of Object.entries(runtimeSpawnPoints)) {
    if (
      typeof rawSpawn !== "object" ||
      rawSpawn === null ||
      Array.isArray(rawSpawn)
    ) {
      throw new Error(`Runtime spawn point ${id} must be an object`);
    }
    const spawn = rawSpawn as Record<string, unknown>;
    const identifier = requireIntegerField(spawn, "identifier", id);
    if (String(identifier) !== id) {
      throw new Error(
        `Runtime spawn point key ${id} does not match identifier ${identifier}`,
      );
    }
    const mapConstant = requireStringField(spawn, "mapConstant", id);
    const mapName = requireStringField(spawn, "mapName", id);
    const groupName = requireStringField(spawn, "groupName", id);
    const groupId = requireIntegerField(spawn, "groupId", id);
    const mapId = requireIntegerField(spawn, "mapId", id);
    const tileX = requireIntegerField(spawn, "tileX", id);
    const tileY = requireIntegerField(spawn, "tileY", id);
    const metatileX = requireIntegerField(spawn, "metatileX", id);
    const metatileY = requireIntegerField(spawn, "metatileY", id);
    const subtileX = requireIntegerField(spawn, "subtileX", id);
    const subtileY = requireIntegerField(spawn, "subtileY", id);
    if (
      mapConstant === "N_A" &&
      mapName === "N_A" &&
      groupName === "N_A" &&
      groupId === -1 &&
      mapId === -1 &&
      tileX === -1 &&
      tileY === -1 &&
      metatileX === -1 &&
      metatileY === -1 &&
      subtileX === -1 &&
      subtileY === -1
    ) {
      continue;
    }
    const expectedTileX = metatileX * 2 + subtileX;
    const expectedTileY = metatileY * 2 + subtileY;
    if (tileX !== expectedTileX || tileY !== expectedTileY) {
      throw new Error(
        `Runtime spawn point ${id} tile (${tileX}, ${tileY}) must match metatile/subtile-derived tile (${expectedTileX}, ${expectedTileY})`,
      );
    }
    // Keep the exact ASM spawn tile.  The subtile is part of the spawn
    // coordinate, not metadata that can be normalized away: the player starts
    // at (3, 3) in PLAYERS_HOUSE_2F, while the containing metatile is (1, 1).
    // Snapping to metatileX * 2 moved every new-game/fly/whiteout destination
    // one tile up and left in the Rust runtime.
    aligned[id] = {
      identifier,
      mapConstant,
      mapName,
      groupId,
      mapId,
      tileX,
      tileY,
      groupName,
      metatileX,
      metatileY,
      subtileX,
      subtileY,
    };
  }
  return aligned;
};

const buildCoreRuntimeTitleScreen = (
  audioAssets?: Record<string, ExportedAudioAsset>,
): ExportedRuntimeTitleScreen => {
  const title = audioAssets?.MUSIC_TITLE;
  if (!title || title.id !== "MUSIC_TITLE" || title.kind !== "music") {
    throw new Error("Core title screen requires exact MUSIC_TITLE music asset");
  }
  return { title_music: title.id };
};

const assertExactFileStem = (value: string): void => {
  if (value.trim() !== value || value.length === 0) {
    throw new Error(
      `Content pack file stem must be explicit and untrimmed: ${JSON.stringify(value)}`,
    );
  }
  if (
    value.includes("/") ||
    value.includes("\\") ||
    value === "." ||
    value === ".." ||
    value.includes("..")
  ) {
    throw new Error(
      `Content pack file stem must be a single exact path segment: ${JSON.stringify(value)}`,
    );
  }
};

const emptyContentPackFiles = (): ContentPackFiles =>
  Object.fromEntries(
    CONTENT_PACK_CATEGORIES.map((category) => [category, []]),
  ) as unknown as ContentPackFiles;

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
  partyIndex: number,
): { speciesId: string } => {
  const source = partyEntry as unknown as Record<string, unknown>;
  const species = source.species;
  if (typeof species === "string" && species.trim()) {
    return { speciesId: species };
  }
  const speciesRecord = asRecord(species) as PokemonSpecies | null;
  if (
    speciesRecord &&
    typeof speciesRecord.id === "string" &&
    speciesRecord.id.trim()
  ) {
    return {
      speciesId: speciesRecord.id,
    };
  }
  throw new Error(
    `Unable to export trainer ${trainerLabel} party[${partyIndex}] because species is not a species id string or Pokemon species record with an id.`,
  );
};

const compactTrainerPartyEntry = (
  partyEntry: Trainer["party"][number],
  trainerLabel: string,
  partyIndex: number,
): Record<string, unknown> => {
  const source = partyEntry as unknown as Record<string, unknown>;
  const { speciesId } = partySpeciesInfo(partyEntry, trainerLabel, partyIndex);
  for (const key of ["item", "moves", "dvs"] as const) {
    if (!Object.prototype.hasOwnProperty.call(source, key)) {
      throw new Error(
        `Unable to export trainer ${trainerLabel} party[${partyIndex}] because '${key}' must be explicit modpack data.`,
      );
    }
  }
  if (!Array.isArray(source.moves)) {
    throw new Error(
      `Unable to export trainer ${trainerLabel} party[${partyIndex}] because 'moves' must be an explicit array.`,
    );
  }
  if (!asRecord(source.dvs)) {
    throw new Error(
      `Unable to export trainer ${trainerLabel} party[${partyIndex}] because 'dvs' must be an explicit object.`,
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
    if (
      key === "species" ||
      key === "level" ||
      key === "item" ||
      key === "moves" ||
      key === "dvs"
    ) {
      continue;
    }
    if (GENERATED_PARTY_FIELDS.has(key)) {
      continue;
    }
    throw new Error(
      `Unable to export trainer ${trainerLabel} party[${partyIndex}] because '${key}' is not part of the definitive trainer party schema.`,
    );
  }

  return compact;
};

const pokemonSpeciesPackPayload = (
  pokemon: PokemonSpecies,
): Omit<PokemonSpecies, "evolutions"> => {
  const { evolutions: _staleEvolutions, ...speciesPayload } =
    pokemon as PokemonSpecies & {
      evolutions?: unknown;
    };
  return speciesPayload;
};

const compactTrainerForContentPack = (
  trainer: Trainer,
): Record<string, unknown> => {
  const trainerLabel = trainer.trainer_id ?? trainer.name ?? "unknown";
  return {
    ...trainer,
    party: Array.isArray(trainer.party)
      ? trainer.party.map((entry, index) =>
          compactTrainerPartyEntry(entry, trainerLabel, index),
        )
      : [],
  };
};

const assertNoEmbeddedOwnedRecords = (
  category: ContentPackCategory,
  relativePath: string,
  payload: unknown,
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
        `Content pack trainer ${relativePath} embeds a Pokemon species object at party[${index}].species. Use the species id string instead.`,
      );
    }
  }
};

const writePackEntry = <T>(
  files: string[],
  category: ContentPackCategory,
  stem: string,
  payload: T,
): string => {
  assertExactFileStem(stem);
  const fileName = `${stem}.json`;
  const relativePath = `${CORE_PACK_PATH}/${category}/${fileName}`;
  assertNoEmbeddedOwnedRecords(category, relativePath, payload);
  writeJsonToTargets(relativePath, payload, { indent: 2 });
  files.push(relativePath);
  return relativePath;
};

const clearGeneratedAudioOutputs = (): void => {
  for (const directory of ["music", "sfx", "cries"]) {
    removeMatchingOutputs(`${CORE_PACK_PATH}/${directory}`, ".json");
  }
};

const assertExactAudioId = (audioId: string): void => {
  if (
    !/^(MUSIC|SFX|CRY)_[A-Z0-9_]+$/.test(audioId) ||
    audioId.includes("FALLBACK") ||
    audioId.includes("LEGACY")
  ) {
    throw new Error(`Audio asset id ${audioId} must be an exact pack audio id`);
  }
};

const audioDirectoryForKind = (kind: ExportedAudioAsset["kind"]): string => {
  switch (kind) {
    case "music":
      return "music";
    case "sound_effect":
      return "sfx";
    case "cry":
      return "cries";
  }
};

const writeAudioAssetFile = (
  files: string[],
  audioId: string,
  asset: ExportedAudioAsset,
  writtenPayloads: Map<string, unknown>,
): void => {
  assertExactAudioId(audioId);
  assertExactAudioId(asset.id);
  if (asset.source !== "pcm") {
    throw new Error(`Audio asset ${asset.id} must use PCM source`);
  }
  if (audioId !== asset.id) {
    throw new Error(
      `Audio asset key ${audioId} does not match record id ${asset.id}`,
    );
  }
  if (!asset.path.endsWith(".pcm")) {
    throw new Error(
      `Audio asset ${asset.id} must use a .pcm file: ${asset.path}`,
    );
  }
  if (!asset.path.startsWith(`${CORE_PACK_PATH}/`)) {
    throw new Error(
      `Audio asset ${asset.id} must live under ${CORE_PACK_PATH}: ${asset.path}`,
    );
  }
  const pathSegments = asset.path.split("/");
  const expectedDirectory = audioDirectoryForKind(asset.kind);
  const actualDirectory = pathSegments.at(-2);
  if (actualDirectory !== expectedDirectory) {
    throw new Error(
      `Audio asset ${asset.id} must live under ${expectedDirectory}: ${asset.path}`,
    );
  }
  const fileName = pathSegments.at(-1) ?? "";
  const expectedFileName = `${asset.id}.pcm`;
  if (fileName !== expectedFileName) {
    throw new Error(
      `Audio asset ${asset.id} path must end with ${expectedFileName}: ${asset.path}`,
    );
  }
  const pcmFormat = asset.pcm_format;
  if (
    !pcmFormat ||
    pcmFormat.sample_rate_hz <= 0 ||
    pcmFormat.channels !== 2 ||
    pcmFormat.bits_per_sample !== 16
  ) {
    throw new Error(`Audio asset ${asset.id} must declare exact stereo 16-bit PCM format`);
  }
  const absolutePath = joinPath(getDataDir(), asset.path);
  if (!fs.existsSync(absolutePath)) {
    throw new Error(
      `Audio asset ${asset.id} is missing generated PCM file: ${asset.path}`,
    );
  }
  const bytes = fs.readFileSync(absolutePath);
  const frameBytes = pcmFormat.channels * (pcmFormat.bits_per_sample / 8);
  if (bytes.length === 0 || bytes.length % frameBytes !== 0) {
    throw new Error(
      `Audio asset ${asset.id} generated PCM file must contain complete frames: ${asset.path}`,
    );
  }
  const metadataPath = asset.path.replace(/\.pcm$/, ".json");
  writeJsonToTargets(metadataPath, { [asset.id]: asset }, { indent: 2 });
  files.push(metadataPath);
  writtenPayloads.set(metadataPath, { [asset.id]: asset });
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
    .map((entry) => {
      const relativePath = `${CORE_PACK_PATH}/${relativeDir}/${entry}`;
      const payload = JSON.parse(
        fs.readFileSync(joinPath(absoluteDir, entry), "utf8"),
      );
      writeJsonToTargets(relativePath, payload, { indent: 2 });
      return relativePath;
    });
};

const exactMapNameFromPayload = (
  relativePath: string,
  payload: unknown,
): string => {
  const record = asRecord(payload);
  if (!record) {
    throw new Error(
      `Map asset ${relativePath} must be an object keyed by exact map labels.`,
    );
  }
  const mapNames = new Set<string>();
  for (const key of Object.keys(record)) {
    const match = key.match(/^(.+)_Map(?:Scripts|Events)$/);
    if (match) {
      mapNames.add(match[1]);
    }
  }
  if (mapNames.size !== 1) {
    throw new Error(
      `Map asset ${relativePath} must declare one exact map name via _MapScripts or _MapEvents labels.`,
    );
  }
  return [...mapNames][0];
};

const writeMapScriptAssetFiles = (
  files: string[],
  sourceRelativeDir: string,
  writtenPayloads?: Map<string, unknown>,
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
    const payload = readJsonAssetSync(
      joinPath(getDataDir(), sourceRelativeDir, entry),
    );
    const relativePath = writePackEntry(files, "map_scripts", stem, payload);
    writtenPayloads?.set(relativePath, payload);
    written.set(exactMapNameFromPayload(relativePath, payload), relativePath);
  }
  return written;
};

const writeMapBlockEntries = (
  files: string[],
  writtenPayloads?: Map<string, unknown>,
): Map<string, string> => {
  const written = new Map<string, string>();
  const raw = readJsonAssetSync<unknown>(
    joinPath(getDataDir(), "map_blocks.json"),
  );
  if (!isRecord(raw)) {
    throw new Error(
      "map_blocks.json must be an object of block labels to encoded block data.",
    );
  }
  for (const [label, encoded] of Object.entries(raw).sort(([a], [b]) =>
    a.localeCompare(b),
  )) {
    if (typeof encoded !== "string") {
      throw new Error(
        `map_blocks.json entry '${label}' must be encoded block data.`,
      );
    }
    const payload = { [label]: encoded };
    const relativePath = writePackEntry(files, "map_blocks", label, payload);
    writtenPayloads?.set(relativePath, payload);
    written.set(label, relativePath);
  }
  return written;
};

const assertExactTilesetId = (tilesetId: string): void => {
  if (!/^[a-z0-9_]+$/.test(tilesetId)) {
    throw new Error(
      `Tileset id '${tilesetId}' must be an exact lowercase asset id.`,
    );
  }
};

const writeTilesetAssetFiles = (
  files: string[],
  mapAttributes: Record<string, unknown>,
  writtenPayloads?: Map<string, unknown>,
): void => {
  const relativeDir = "tilesets";
  const tilesetIds = Array.from(
    new Set(
      Object.values(mapAttributes)
        .map((attributes) => asRecord(attributes)?.tileset_name)
        .filter(
          (tilesetName): tilesetName is string =>
            typeof tilesetName === "string",
        ),
    ),
  ).sort();
  if (tilesetIds.length === 0) {
    return;
  }
  const absoluteDir = joinPath(getDataDir(), relativeDir);
  if (!fs.existsSync(absoluteDir)) {
    throw new Error("Referenced tilesets require assets/data/tilesets.");
  }
  const entries = fs
    .readdirSync(absoluteDir)
    .filter((fileName) => fileName.endsWith(".json"))
    .sort();
  for (const tilesetId of tilesetIds) {
    assertExactTilesetId(tilesetId);
    const collisionFile = `${tilesetId}.json`;
    const paletteFile = `${tilesetId}_palette_map.json`;
    if (!entries.includes(collisionFile)) {
      throw new Error(`Tileset '${tilesetId}' must declare ${collisionFile}.`);
    }
    if (!entries.includes(paletteFile)) {
      throw new Error(`Tileset '${tilesetId}' must declare ${paletteFile}.`);
    }
    const collision = readJsonAssetSync(
      joinPath(getDataDir(), relativeDir, collisionFile),
    );
    const paletteMap = readJsonAssetSync(
      joinPath(getDataDir(), relativeDir, paletteFile),
    );
    if (!isRecord(collision)) {
      throw new Error(
        `Tileset '${tilesetId}' collision payload must be an object.`,
      );
    }
    if (!Array.isArray(paletteMap)) {
      throw new Error(
        `Tileset '${tilesetId}' palette_map payload must be an array.`,
      );
    }
    const payload = {
      [tilesetId]: {
        collision,
        palette_map: paletteMap,
      },
    };
    const relativePath = writePackEntry(files, "tilesets", tilesetId, payload);
    writtenPayloads?.set(relativePath, payload);
  }
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === "object" && !Array.isArray(value);

const requireContentPackFiles = (
  packId: string,
  value: unknown,
): ContentPackFiles => {
  if (!isRecord(value)) {
    throw new Error(`Content pack '${packId}' must declare files.`);
  }
  const files = {} as ContentPackFiles;
  for (const category of CONTENT_PACK_CATEGORIES) {
    const entries = value[category];
    if (
      !Array.isArray(entries) ||
      entries.some((entry) => typeof entry !== "string")
    ) {
      throw new Error(
        `Content pack '${packId}' must declare files.${category} as a string array.`,
      );
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
    throw new Error(
      `Content pack '${id}' must declare compiled as a string or null.`,
    );
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

const readPreservedPacks = (): ContentPack[] => {
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
  return parsed.packs
    .filter((pack) => {
      const id = isRecord(pack) && typeof pack.id === "string" ? pack.id : "";
      return id !== CORE_PACK_ID && !id.startsWith(`${MODULE_PREFIX}-`);
    })
    .map((pack, index) => {
      if (isRecord(pack)) {
        return requireContentPack(
          {
            ...pack,
            compiled: "compiled" in pack ? pack.compiled : null,
            files: {
              ...emptyContentPackFiles(),
              ...(isRecord(pack.files) ? pack.files : {}),
            },
          },
          index,
        );
      }
      return requireContentPack(pack, index);
    });
};

const parentPathFor = (
  filePath: string,
  _category: ContentPackCategory,
): string => filePath.replace(/\/[^/]+$/, "");

const fileStem = (filePath: string): string =>
  filePath
    .split("/")
    .pop()
    ?.replace(/\.[^.]+$/i, "") ?? "unknown";

const makeModulePack = (
  id: string,
  path: string,
  files: Partial<ContentPackFiles>,
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
  Object.fromEntries(
    CONTENT_PACK_CATEGORIES.map((category) => [category, []]),
  ) as unknown as Record<ContentPackCategory, unknown[]>;

const buildCompiledCorePack = (
  files: ContentPackFiles,
  writtenPayloads: Map<string, unknown>,
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
  const remainingPacks = readPreservedPacks();

  removeMatchingOutputs(CORE_PACK_PATH, ".json");
  clearGeneratedAudioOutputs();

  const files = emptyContentPackFiles();
  const writtenPayloads = new Map<string, unknown>();

  const writeCorePackEntry = <T>(
    categoryFiles: string[],
    category: ContentPackCategory,
    stem: string,
    entryPayload: T,
  ): string => {
    const relativePath = writePackEntry(
      categoryFiles,
      category,
      stem,
      entryPayload,
    );
    writtenPayloads.set(relativePath, entryPayload);
    return relativePath;
  };

  for (const pokemon of payload.pokemonData) {
    writeCorePackEntry(files.pokemon, "pokemon", pokemon.id, {
      [pokemon.id]: pokemonSpeciesPackPayload(pokemon),
    });
  }
  for (const [name, move] of Object.entries(payload.movesData)) {
    writeCorePackEntry(files.moves, "moves", name, { [name]: move });
  }
  if (payload.growthRatesData && payload.growthRatesData.length > 0) {
    writeCorePackEntry(
      files.growth_rates,
      "growth_rates",
      "growth_rates",
      Object.fromEntries(
        payload.growthRatesData.map((curve) => [curve.id, curve]),
      ),
    );
  }
  for (const [species, learnset] of Object.entries(payload.learnsetsData)) {
    writeCorePackEntry(files.learnsets, "learnsets", species, {
      [species]: {
        species,
        learnset,
      },
    });
  }
  for (const [species, moves] of Object.entries(payload.levelUpMovesData)) {
    writeCorePackEntry(files.level_up_moves, "level_up_moves", species, {
      [species]: {
        species,
        moves,
      },
    });
  }
  for (const [species, moves] of Object.entries(payload.eggMovesData)) {
    writeCorePackEntry(files.egg_moves, "egg_moves", species, {
      [species]: {
        species,
        moves,
      },
    });
  }
  for (const evolution of payload.evolutions) {
    const key = evolution.species ?? `species_${files.evolutions.length}`;
    writeCorePackEntry(files.evolutions, "evolutions", key, {
      [key]: evolution,
    });
  }
  const mapScriptPathByName = writeMapScriptAssetFiles(
    files.map_scripts,
    "maps",
    writtenPayloads,
  );
  const mapBlockPathByLabel = writeMapBlockEntries(
    files.map_blocks,
    writtenPayloads,
  );
  const encounterPathByMapName = new Map<string, string>();
  for (const encounter of payload.wildEncounters) {
    const encounterPath = writeCorePackEntry(
      files.wild_encounters,
      "wild_encounters",
      encounter.map_name,
      {
        [encounter.map_name]: encounter,
      },
    );
    encounterPathByMapName.set(encounter.map_name, encounterPath);
  }
  const fieldEncounterPathByMapName = new Map<string, string>();
  for (const encounter of payload.fieldEncounters ?? []) {
    const encounterPath = writeCorePackEntry(
      files.field_encounters,
      "field_encounters",
      encounter.map_name,
      {
        [encounter.map_name]: encounter,
      },
    );
    fieldEncounterPathByMapName.set(encounter.map_name, encounterPath);
  }
  if (payload.runtimeSpawnPoints) {
    writeCorePackEntry(
      files.runtime_spawn_points,
      "runtime_spawn_points",
      "spawn_points",
      alignRuntimeSpawnPoints(payload.runtimeSpawnPoints),
    );
  }
  if (payload.flyDestinations) {
    writeCorePackEntry(
      files.fly_destinations,
      "fly_destinations",
      "fly_destinations",
      payload.flyDestinations,
    );
  }
  if (payload.runtimeMapMetadata) {
    writeCorePackEntry(
      files.runtime_map_metadata,
      "runtime_map_metadata",
      "map_metadata",
      payload.runtimeMapMetadata,
    );
  }
  if (payload.fleeMons) {
    writeCorePackEntry(
      files.flee_mons,
      "flee_mons",
      "flee_mons",
      payload.fleeMons,
    );
  }
  if (payload.roamingPokemon) {
    writeCorePackEntry(
      files.roaming_pokemon,
      "roaming_pokemon",
      "roaming_pokemon",
      payload.roamingPokemon,
    );
  }
  if (payload.buenaPasswordCategories) {
    writeCorePackEntry(
      files.buena_password_categories,
      "buena_password_categories",
      "buena_password_categories",
      payload.buenaPasswordCategories,
    );
  }
  if (payload.buenaPrizes) {
    writeCorePackEntry(
      files.buena_prizes,
      "buena_prizes",
      "buena_prizes",
      payload.buenaPrizes,
    );
  }
  if (payload.kurtApricornRecipes) {
    writeCorePackEntry(
      files.kurt_apricorn_recipes,
      "kurt_apricorn_recipes",
      "kurt_apricorn_recipes",
      payload.kurtApricornRecipes,
    );
  }
  if (payload.shuckieGift) {
    writeCorePackEntry(
      files.shuckie_gift,
      "shuckie_gift",
      "shuckie_gift",
      payload.shuckieGift,
    );
  }
  if (payload.dratiniMoveSets) {
    writeCorePackEntry(
      files.dratini_move_sets,
      "dratini_move_sets",
      "dratini_move_sets",
      payload.dratiniMoveSets,
    );
  }
  if (payload.bugContestConfig) {
    writeCorePackEntry(
      files.bug_contest_config,
      "bug_contest_config",
      "bug_contest_config",
      payload.bugContestConfig,
    );
  }
  if (payload.battleTowerRules) {
    writeCorePackEntry(
      files.battle_tower_rules,
      "battle_tower_rules",
      "battle_tower_rules",
      payload.battleTowerRules,
    );
  }
  if (payload.oakRatings) {
    writeCorePackEntry(
      files.oak_ratings,
      "oak_ratings",
      "oak_ratings",
      payload.oakRatings,
    );
  }
  if (payload.oddEggDefinitions) {
    writeCorePackEntry(
      files.odd_egg_definitions,
      "odd_egg_definitions",
      "odd_egg_definitions",
      payload.oddEggDefinitions,
    );
  }
  if (payload.magikarpLengths) {
    writeCorePackEntry(
      files.magikarp_lengths,
      "magikarp_lengths",
      "magikarp_lengths",
      payload.magikarpLengths,
    );
  }
  if (payload.happinessData) {
    writeCorePackEntry(
      files.happiness_data,
      "happiness_data",
      "happiness_data",
      payload.happinessData,
    );
  }
  if (payload.encounterSlotTables) {
    writeCorePackEntry(
      files.encounter_slot_tables,
      "encounter_slot_tables",
      "encounter_slot_tables",
      payload.encounterSlotTables,
    );
  }
  if (payload.encounterMusicModifiers) {
    writeCorePackEntry(
      files.encounter_music_modifiers,
      "encounter_music_modifiers",
      "encounter_music_modifiers",
      payload.encounterMusicModifiers,
    );
  }
  if (payload.battleStatMultipliers) {
    writeCorePackEntry(
      files.battle_stat_multipliers,
      "battle_stat_multipliers",
      "battle_stat_multipliers",
      payload.battleStatMultipliers,
    );
  }
  if (payload.captureWobbleProbabilities) {
    writeCorePackEntry(
      files.capture_wobble_probabilities,
      "capture_wobble_probabilities",
      "capture_wobble_probabilities",
      payload.captureWobbleProbabilities,
    );
  }
  if (payload.captureRules) {
    writeCorePackEntry(
      files.capture_rules,
      "capture_rules",
      "rules",
      payload.captureRules,
    );
  }
  if (payload.movePriorities) {
    writeCorePackEntry(
      files.move_priorities,
      "move_priorities",
      "move_priorities",
      payload.movePriorities,
    );
  }
  if (payload.typeCategories) {
    writeCorePackEntry(
      files.type_categories,
      "type_categories",
      "type_categories",
      payload.typeCategories,
    );
  }
  if (payload.typeEffectiveness) {
    writeCorePackEntry(
      files.type_effectiveness,
      "type_effectiveness",
      "type_effectiveness",
      payload.typeEffectiveness,
    );
  }
  if (payload.weatherModifiers) {
    writeCorePackEntry(
      files.weather_modifiers,
      "weather_modifiers",
      "weather_modifiers",
      payload.weatherModifiers,
    );
  }
  if (payload.battleRewardRules) {
    writeCorePackEntry(
      files.battle_reward_rules,
      "battle_reward_rules",
      "rules",
      payload.battleRewardRules,
    );
  }
  if (payload.battleEscapeRules) {
    writeCorePackEntry(
      files.battle_escape_rules,
      "battle_escape_rules",
      "rules",
      payload.battleEscapeRules,
    );
  }
  if (payload.stepEventRules) {
    writeCorePackEntry(
      files.step_event_rules,
      "step_event_rules",
      "rules",
      payload.stepEventRules,
    );
  }
  if (payload.fishing) {
    writeCorePackEntry(files.fishing, "fishing", "fishing", payload.fishing);
  }
  if (payload.fieldMoves) {
    writeCorePackEntry(
      files.field_moves,
      "field_moves",
      "field_moves",
      payload.fieldMoves,
    );
  }
  if (payload.fieldBoxItems && Object.keys(payload.fieldBoxItems).length > 0) {
    writeCorePackEntry(
      files.field_box_items,
      "field_box_items",
      "field_box_items",
      payload.fieldBoxItems,
    );
  }
  if (payload.decorations) {
    writeCorePackEntry(
      files.decorations,
      "decorations",
      "decorations",
      payload.decorations,
    );
  }
  if (payload.fruitTrees && Object.keys(payload.fruitTrees).length > 0) {
    writeCorePackEntry(
      files.fruit_trees,
      "fruit_trees",
      "fruit_trees",
      payload.fruitTrees,
    );
  }
  for (const [audioId, audioAsset] of Object.entries(payload.audioAssets ?? {})) {
    writeAudioAssetFile(files.audio, audioId, audioAsset, writtenPayloads);
  }
  const mapAttributePathByName = new Map<string, string>();
  const mapBlockPathByName = new Map<string, string>();
  const mapDimensionKeyByName = new Map<string, string>();
  for (const [mapName, attributes] of Object.entries(payload.mapAttributes)) {
    const mapAttributePath = writeCorePackEntry(
      files.map_attributes,
      "map_attributes",
      mapName,
      {
        [mapName]: attributes,
      },
    );
    mapAttributePathByName.set(mapName, mapAttributePath);
    const record = asRecord(attributes);
    if (typeof record?.map_constant === "string") {
      mapDimensionKeyByName.set(mapName, record.map_constant);
    }
    const blocksLabel =
      typeof record?.blocks_label === "string"
        ? record.blocks_label
        : `${mapName}_Blocks`;
    const mapBlockPath = mapBlockPathByLabel.get(blocksLabel);
    if (mapBlockPath) {
      mapBlockPathByName.set(mapName, mapBlockPath);
    }
  }

  const mapDimensionPathByName = new Map<string, string>();
  for (const [mapName, dimensions] of Object.entries(payload.mapDimensions)) {
    const mapDimensionPath = writeCorePackEntry(
      files.map_dimensions,
      "map_dimensions",
      mapName,
      {
        [mapName]: dimensions,
      },
    );
    mapDimensionPathByName.set(mapName, mapDimensionPath);
  }

  const npcPathByName = new Map<string, string>();
  for (const [mapName, entries] of Object.entries(payload.npcData)) {
    const npcPath = writeCorePackEntry(files.npcs, "npcs", mapName, {
      [mapName]: entries,
    });
    npcPathByName.set(mapName, npcPath);
  }

  for (const item of payload.items) {
    if (item.script_name === "$00") {
      continue;
    }
    writeCorePackEntry(files.items, "items", item.script_name, {
      [item.script_name]: item,
    });
  }
  if (payload.marts && Object.keys(payload.marts).length > 0) {
    writeCorePackEntry(files.marts, "marts", "marts", payload.marts);
  }
  if (
    payload.currencyConstants &&
    Object.keys(payload.currencyConstants).length > 0
  ) {
    writeCorePackEntry(
      files.currency_constants,
      "currency_constants",
      "constants",
      payload.currencyConstants,
    );
  }
  if (payload.pcStrings && Object.keys(payload.pcStrings).length > 0) {
    writeCorePackEntry(
      files.pc_strings,
      "pc_strings",
      "pc_strings",
      payload.pcStrings,
    );
  }
  if (payload.menuIcons && Object.keys(payload.menuIcons).length > 0) {
    writeCorePackEntry(
      files.menu_icons,
      "menu_icons",
      "menu_icons",
      payload.menuIcons,
    );
  }
  if (payload.phoneContacts && Object.keys(payload.phoneContacts).length > 0) {
    writeCorePackEntry(
      files.phone_contacts,
      "phone_contacts",
      "contacts",
      payload.phoneContacts,
    );
  }
  if (
    payload.permanentPhoneNumbers &&
    Object.keys(payload.permanentPhoneNumbers).length > 0
  ) {
    writeCorePackEntry(
      files.permanent_phone_numbers,
      "permanent_phone_numbers",
      "permanent",
      payload.permanentPhoneNumbers,
    );
  }
  if (
    payload.specialPhoneCalls &&
    Object.keys(payload.specialPhoneCalls).length > 0
  ) {
    writeCorePackEntry(
      files.special_phone_calls,
      "special_phone_calls",
      "calls",
      payload.specialPhoneCalls,
    );
  }
  if (payload.npcTrades && Object.keys(payload.npcTrades).length > 0) {
    writeCorePackEntry(
      files.npc_trades,
      "npc_trades",
      "trades",
      payload.npcTrades,
    );
  }
  if (
    payload.specialRoutines &&
    Object.keys(payload.specialRoutines).length > 0
  ) {
    writeCorePackEntry(
      files.special_routines,
      "special_routines",
      "routines",
      payload.specialRoutines,
    );
  }
  if (payload.asmText && Object.keys(payload.asmText).length > 0) {
    writeCorePackEntry(files.asm_text, "asm_text", "texts", payload.asmText);
  }
  if (payload.moveNames && payload.moveNames.length > 0) {
    writeCorePackEntry(
      files.move_names,
      "move_names",
      "moves",
      payload.moveNames,
    );
  }
  if (
    payload.battleAnimations &&
    Object.keys(payload.battleAnimations).length > 0
  ) {
    writeCorePackEntry(
      files.battle_animations,
      "battle_animations",
      "scripts",
      payload.battleAnimations,
    );
  }
  if (payload.battleAnimationTable && payload.battleAnimationTable.length > 0) {
    writeCorePackEntry(
      files.battle_animation_table,
      "battle_animation_table",
      "table",
      payload.battleAnimationTable,
    );
  }
  if (payload.battleAnimBundle) {
    writeCorePackEntry(
      files.battle_anim_bundle,
      "battle_anim_bundle",
      "bundle",
      payload.battleAnimBundle,
    );
  }
  if (payload.spriteAnimBundle) {
    writeCorePackEntry(
      files.sprite_anim_bundle,
      "sprite_anim_bundle",
      "bundle",
      payload.spriteAnimBundle,
    );
  }
  if (
    payload.spritePaletteDefaults &&
    Object.keys(payload.spritePaletteDefaults).length > 0
  ) {
    writeCorePackEntry(
      files.sprite_palette_defaults,
      "sprite_palette_defaults",
      "defaults",
      payload.spritePaletteDefaults,
    );
  }
  if (
    payload.pokegearTownMapPaletteMap &&
    Object.keys(payload.pokegearTownMapPaletteMap).length > 0
  ) {
    writeCorePackEntry(
      files.pokegear_town_map_palette_map,
      "pokegear_town_map_palette_map",
      "palettes",
      payload.pokegearTownMapPaletteMap,
    );
  }
  if (payload.pokemonCries) {
    writeCorePackEntry(
      files.pokemon_cries,
      "pokemon_cries",
      "cries",
      payload.pokemonCries,
    );
  }
  for (const trainer of payload.trainers) {
    const trainerKey = trainer.trainer_id ?? trainer.name;
    writeCorePackEntry(files.trainers, "trainers", trainerKey, {
      [trainerKey]: compactTrainerForContentPack(trainer),
    });
  }
  writeCorePackEntry(
    files.trainer_class_names,
    "trainer_class_names",
    "classes",
    payload.trainerClassNames,
  );
  for (const entry of payload.pokedex) {
    writeCorePackEntry(files.pokedex, "pokedex", entry.species, {
      [entry.species]: entry,
    });
  }
  if (
    payload.pokedexEntries &&
    Object.keys(payload.pokedexEntries).length > 0
  ) {
    writeCorePackEntry(
      files.pokedex_entries,
      "pokedex_entries",
      "entries",
      payload.pokedexEntries,
    );
  }
  if (
    payload.pokemonFrontpicAnimations &&
    Object.keys(payload.pokemonFrontpicAnimations).length > 0
  ) {
    writeCorePackEntry(
      files.pokemon_frontpic_anim,
      "pokemon_frontpic_anim",
      "programs",
      payload.pokemonFrontpicAnimations,
    );
  }
  if (payload.initializeEvents) {
    writeCorePackEntry(
      files.initialize_events,
      "initialize_events",
      "initialize_events",
      payload.initializeEvents,
    );
  }
  if (payload.storyEventScriptConstants) {
    writeCorePackEntry(
      files.story_event_script_constants,
      "story_event_script_constants",
      "constants",
      storyEventConstantsWithFieldBoxFlags(
        payload.storyEventScriptConstants,
        payload.fieldBoxItems,
      ),
    );
  }
  writeCorePackEntry(
    files.pokegear_landmarks,
    "pokegear_landmarks",
    "landmarks",
    payload.pokegearLandmarks,
  );
  if (payload.playability) {
    writeCorePackEntry(
      files.playability,
      "playability",
      "core",
      payload.playability,
    );
  }
  writeCorePackEntry(
    files.runtime_title_screen,
    "runtime_title_screen",
    "title",
    payload.runtimeTitleScreen ??
      buildCoreRuntimeTitleScreen(payload.audioAssets),
  );
  files.story_events.push(...collectJsonFiles("story_events"));
  files.phone_scripts.push(...collectJsonFiles("phone_scripts"));
  writeTilesetAssetFiles(
    files.tilesets,
    payload.mapAttributes,
    writtenPayloads,
  );

  const compiledPath = "content-packs/core-modular.compiled.json";
  writeJsonToTargets(
    compiledPath,
    buildCompiledCorePack(files, writtenPayloads),
    { indent: 0 },
  );

  const generatedCorePack = {
    id: CORE_PACK_ID,
    enabled: true,
    priority: -100,
    path: CORE_PACK_PATH,
    compiled: null,
    files,
  };
  writeJsonToTargets(CORE_GENERATED_PACK_MANIFEST_PATH, generatedCorePack, {
    indent: 2,
  });

  remainingPacks.push({
    id: CORE_PACK_ID,
    enabled: true,
    priority: -100,
    path: CORE_PACK_PATH,
    compiled: CORE_RUNTIME_PACK_PATH,
    files: emptyContentPackFiles(),
  });

  const singleCategoryModules: Array<{
    category: ContentPackCategory;
    prefix: string;
  }> = [
    { category: "maps", prefix: "map" },
    { category: "map_scripts", prefix: "map-script" },
    { category: "map_blocks", prefix: "map-block" },
    { category: "runtime_spawn_points", prefix: "runtime-spawn-points" },
    { category: "fly_destinations", prefix: "fly-destinations" },
    { category: "runtime_map_metadata", prefix: "runtime-map-metadata" },
    { category: "flee_mons", prefix: "flee-mons" },
    { category: "roaming_pokemon", prefix: "roaming-pokemon" },
    {
      category: "buena_password_categories",
      prefix: "buena-password-categories",
    },
    { category: "buena_prizes", prefix: "buena-prizes" },
    { category: "kurt_apricorn_recipes", prefix: "kurt-apricorn-recipes" },
    { category: "shuckie_gift", prefix: "shuckie-gift" },
    { category: "dratini_move_sets", prefix: "dratini-move-sets" },
    { category: "bug_contest_config", prefix: "bug-contest-config" },
    { category: "battle_tower_rules", prefix: "battle-tower-rules" },
    { category: "oak_ratings", prefix: "oak-ratings" },
    { category: "odd_egg_definitions", prefix: "odd-egg-definitions" },
    { category: "magikarp_lengths", prefix: "magikarp-lengths" },
    { category: "happiness_data", prefix: "happiness-data" },
    { category: "encounter_slot_tables", prefix: "encounter-slot-tables" },
    { category: "battle_stat_multipliers", prefix: "battle-stat-multipliers" },
    {
      category: "capture_wobble_probabilities",
      prefix: "capture-wobble-probabilities",
    },
    { category: "capture_rules", prefix: "capture-rules" },
    { category: "move_priorities", prefix: "move-priorities" },
    { category: "type_categories", prefix: "type-categories" },
    { category: "type_effectiveness", prefix: "type-effectiveness" },
    { category: "weather_modifiers", prefix: "weather-modifiers" },
    { category: "battle_reward_rules", prefix: "battle-reward-rules" },
    { category: "battle_escape_rules", prefix: "battle-escape-rules" },
    { category: "step_event_rules", prefix: "step-event-rules" },
    { category: "fishing", prefix: "fishing" },
    { category: "field_moves", prefix: "field-moves" },
    { category: "field_box_items", prefix: "field-box-items" },
    { category: "decorations", prefix: "decorations" },
    { category: "runtime_title_screen", prefix: "runtime-title-screen" },
    { category: "fruit_trees", prefix: "fruit-trees" },
    { category: "pokemon", prefix: "pokemon" },
    { category: "moves", prefix: "move" },
    { category: "learnsets", prefix: "learnset" },
    { category: "level_up_moves", prefix: "level-up-move" },
    { category: "egg_moves", prefix: "egg-move" },
    { category: "items", prefix: "item" },
    { category: "currency_constants", prefix: "currency-constants" },
    { category: "pc_strings", prefix: "pc-strings" },
    { category: "menu_icons", prefix: "menu-icons" },
    { category: "trainers", prefix: "trainer" },
    { category: "trainer_class_names", prefix: "trainer-class-names" },
    { category: "pokedex", prefix: "pokedex" },
    { category: "pokedex_entries", prefix: "pokedex-entries" },
    { category: "pokemon_frontpic_anim", prefix: "pokemon-frontpic-anim" },
    { category: "initialize_events", prefix: "initialize-events" },
    {
      category: "story_event_script_constants",
      prefix: "story-event-script-constants",
    },
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
    {
      category: "pokegear_town_map_palette_map",
      prefix: "pokegear-town-map-palette-map",
    },
    { category: "pokemon_cries", prefix: "pokemon-cries" },
    { category: "audio", prefix: "audio" },
    { category: "tilesets", prefix: "tileset" },
    { category: "playability", prefix: "playability" },
  ];

  for (const { category, prefix } of singleCategoryModules) {
    for (const filePath of files[category]) {
      const name = fileStem(filePath);
      remainingPacks.push(
        makeModulePack(
          `${MODULE_PREFIX}-${prefix}-${name}`,
          parentPathFor(filePath, category),
          {
            [category]: [filePath],
          },
        ),
      );
    }
  }

  const routeMapNames = new Set([
    ...encounterPathByMapName.keys(),
    ...fieldEncounterPathByMapName.keys(),
  ]);
  for (const mapName of [...routeMapNames].sort((a, b) => a.localeCompare(b))) {
    const routePath = encounterPathByMapName.get(mapName);
    const fieldEncounterPath = fieldEncounterPathByMapName.get(mapName);
    const name = fileStem(routePath ?? fieldEncounterPath ?? mapName);
    const mapScriptPath = mapScriptPathByName.get(mapName);
    const mapBlockPath = mapBlockPathByName.get(mapName);
    const mapAttributePath = mapAttributePathByName.get(mapName);
    const mapDimensionKey = mapDimensionKeyByName.get(mapName);
    const mapDimensionPath = mapDimensionKey
      ? mapDimensionPathByName.get(mapDimensionKey)
      : undefined;
    const npcPath = npcPathByName.get(mapName);
    const parentPath = routePath
      ? parentPathFor(routePath, "wild_encounters")
      : parentPathFor(fieldEncounterPath!, "field_encounters");
    remainingPacks.push(
      makeModulePack(`${MODULE_PREFIX}-route-${name}`, parentPath, {
        map_scripts: mapScriptPath ? [mapScriptPath] : [],
        map_blocks: mapBlockPath ? [mapBlockPath] : [],
        map_attributes: mapAttributePath ? [mapAttributePath] : [],
        map_dimensions: mapDimensionPath ? [mapDimensionPath] : [],
        wild_encounters: routePath ? [routePath] : [],
        field_encounters: fieldEncounterPath ? [fieldEncounterPath] : [],
        npcs: npcPath ? [npcPath] : [],
      }),
    );
  }

  remainingPacks.sort((a, b) => a.id.localeCompare(b.id));

  writeJsonToTargets(
    "content-packs/index.json",
    { version: 1, packs: remainingPacks },
    { indent: 2 },
  );
}

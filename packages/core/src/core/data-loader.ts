import fs from "fs";
import { promises as fsPromises } from "fs";
import path from "path";
import { z } from "zod";
import { PokemonSpeciesSchema, MoveSchema, ItemSchema, Item } from "./models";
import { asmTextLoader } from "./asm-text-loader";
import { preloadTextAssets, readJsonAssetSync, readTextAssetSync } from "./asset-reader";
import { getDataDir, getDisassemblyRoot } from "./paths";
import { MoveName } from "./enums";
import {
  loadMergedMapAttributesSync,
  loadMergedMapDimensionsSync,
  loadMergedMovesDataSync,
  loadMergedItemsSync,
  loadMergedPokemonDataSync,
  loadMergedTrainersSync,
  loadMergedWildEncountersSync,
  loadMergedNpcDataSync,
  loadContentPackCategoryJsonSync,
  hasEnabledCompiledContentPackSync,
} from "./content-packs";
import {
  MapAttributes as MapAttributesSchema,
  MapEvents as MapEventsSchema,
  type MapAttributes as MapAttributesType,
  type MapEvents as MapEventsType,
  type ObjectEvent,
} from "./models/map";
import { TrainerSchema, type Trainer } from './models/trainer';
import { mapConstantToName } from "@pokecrystal/core/engine/world/maps";
import type { OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";
import { METATILE_WIDTH } from "@pokecrystal/core/core/tileset-data";
import { WildEncounterDataSchema, type WildEncounterData } from "@pokecrystal/assets/content/wild-encounter-data";
import { listAssetDir } from "./asset-manifest";

type PokemonSpecies = z.infer<typeof PokemonSpeciesSchema>;
type Move = z.infer<typeof MoveSchema>;

const speciesData: Record<string, PokemonSpecies> = {};
const moveData: Record<string, Move> = {};
const itemData: Record<string, Item> = {};
const isBrowser = typeof window !== "undefined";
type PreloadMode = "none" | "core" | "all" | "all_with_phone";

type ScriptArgument =
  | string
  | number
  | boolean
  | null
  | (string | number | boolean | null)[]
  | Record<string, string | number | boolean | null>;

export interface ScriptEntry {
  command?: string;
  args?: ScriptArgument[];
  text?: string | null;
  [key: string]: ScriptArgument | ScriptArgument[] | undefined;
}

export type ScriptData = ScriptEntry[];

export type StoryEvents = Record<string, ScriptData | Record<string, ScriptData>>;

const isScriptEntry = (value: unknown): value is ScriptEntry =>
  typeof value === "object" && value !== null;

const getEntryCommand = (entry: ScriptEntry | unknown, options?: { trim?: boolean; lower?: boolean }): string => {
  if (!isScriptEntry(entry)) {
    return "";
  }
  const value = entry.command;
  let result = String(value ?? "");
  if (options?.trim) {
    result = result.trim();
  }
  if (options?.lower) {
    result = result.toLowerCase();
  }
  return result;
};

const getEntryArgs = (entry: ScriptEntry): unknown[] | null =>
  Array.isArray(entry.args) ? entry.args : null;

const getEntryText = (entry: ScriptEntry): unknown | null => {
  if (entry.text !== undefined) {
    return entry.text;
  }
  if (typeof entry.args === "string") {
    return entry.args;
  }
  const args = getEntryArgs(entry);
  return args && args.length ? args[0] : null;
};

type TilesetConstructor = new (tilesetName: string, timeOfDay: string) => OverworldTilesetLike;

const DATA_DIR = getDataDir();
const STORY_EVENTS_PATH = path.join(DATA_DIR, "story_events.json");
const MAP_ATTRIBUTES_PATH = path.join(DATA_DIR, "map_attributes.json");
const MAP_DIMENSIONS_PATH = path.join(DATA_DIR, "map_dimensions.json");
const NPC_DATA_PATH = path.join(DATA_DIR, "npcs.json");
const MAP_SCRIPTS_DIR = path.join(DATA_DIR, "maps");
const PHONE_SCRIPTS_DIR = path.join(DATA_DIR, "phone_scripts");
const WILD_ENCOUNTERS_PATH = path.join(DATA_DIR, "wild_encounters.json");
const STORY_EVENTS_DIR = path.join(DATA_DIR, "story_events");
const MARTS_PATH = path.join(DATA_DIR, "marts.json");
const PC_STRINGS_PATH = path.join(DATA_DIR, "pc_strings.json");
const ITEMS_PATH = path.join(DATA_DIR, "items.json");
const POKEMON_DATA_PATH = path.join(DATA_DIR, "pokemon_data.json");
const POKEGEAR_LANDMARKS_PATH = path.join(DATA_DIR, "pokegear_landmarks.json");
const PHONE_CONTACTS_PATH = path.join(DATA_DIR, "phone_contacts.json");
const PERMANENT_PHONE_NUMBERS_PATH = path.join(DATA_DIR, "permanent_phone_numbers.json");
const INITIALIZE_EVENTS_PATH = path.join(DATA_DIR, "initialize_events.json");

let storyEventsCache: StoryEvents | null = null;
let storyScriptIndex: Record<string, ScriptData> | null = null;
let storyLabelToMap: Record<string, string> | null = null;
let storyTextIndex: Record<string, string> | null = null;
let storyScriptSuccessors: Record<string, [string | null, string]> | null = null;
let storyLocalScripts: Record<string, Record<string, ScriptData>> | null = null;
let storyEventsLoadedFromCompiledContentPack = false;
let phoneScriptsLoaded = false;
let pcStringsLoaded = false;
const mapScriptsCache: Map<string, Record<string, ScriptData> | null> = new Map();
const mapScriptSuccessorsCache: Map<string, Record<string, [string | null, string]>> = new Map();

const resetStoryEventsCache = (): void => {
  storyEventsCache = null;
  storyScriptIndex = null;
  storyLabelToMap = null;
  storyTextIndex = null;
  storyScriptSuccessors = null;
  storyLocalScripts = null;
  storyEventsLoadedFromCompiledContentPack = false;
  phoneScriptsLoaded = false;
  pcStringsLoaded = false;
  mapScriptsCache.clear();
  mapScriptSuccessorsCache.clear();
};

const normalizeLabel = (label: string): string => label.trim().replace(/:$/, "");

const readJsonSync = (filePath: string): unknown => {
  return readJsonAssetSync(filePath);
};

const readAsmLines = (filePath: string): string[] => {
  try {
    return readTextAssetSync(filePath).split(/\r?\n/);
  } catch {
    return [];
  }
};

const stripAsmComment = (line: string): string => line.split(";")[0].trim();

const listJsonAssets = (dirPath: string): string[] => {
  const entries = isBrowser
    ? listAssetDir(dirPath)
    : fs.existsSync(dirPath)
      ? (fs.readdirSync(dirPath, { withFileTypes: true }) as Array<fs.Dirent> | string[])
      : [];
  const jsonEntries: string[] = [];
  for (const entry of entries) {
    const entryName =
      typeof entry === "string"
        ? entry
        : typeof entry?.name === "string"
          ? entry.name
          : "";
    if (!entryName.toLowerCase().endsWith(".json")) {
      continue;
    }
    if (typeof entry !== "string" && typeof entry?.isFile === "function" && !entry.isFile()) {
      continue;
    }
    jsonEntries.push(entryName);
  }
  return jsonEntries;
};

const assetExists = (filePath: string): boolean => {
  if (fs.existsSync(filePath)) {
    return true;
  }
  const directory = path.dirname(filePath);
  const filename = path.basename(filePath);
  try {
    return listAssetDir(directory).includes(filename);
  } catch {
    return false;
  }
};

const resolvePreloadMode = (value?: string): PreloadMode => {
  const normalized = String(value ?? "none").trim().toLowerCase();
  if (
    normalized === "all" ||
    normalized === "all_with_phone" ||
    normalized === "core" ||
    normalized === "none"
  ) {
    return normalized as PreloadMode;
  }
  return "none";
};

export const preloadCoreDataAssets = async (
  mode?: PreloadMode,
  options?: { onProgress?: (completed: number, total: number, path?: string) => void }
): Promise<void> => {
  if (!isBrowser) {
    return;
  }
  const resolvedMode = mode ?? resolvePreloadMode(process.env.NEXT_PUBLIC_DATA_PREFETCH);
  if (resolvedMode === "none") {
    return;
  }
  const corePaths = [
    MAP_ATTRIBUTES_PATH,
    MAP_DIMENSIONS_PATH,
    NPC_DATA_PATH,
    WILD_ENCOUNTERS_PATH,
    MARTS_PATH,
    PC_STRINGS_PATH,
    ITEMS_PATH,
    POKEMON_DATA_PATH,
    POKEGEAR_LANDMARKS_PATH,
    PHONE_CONTACTS_PATH,
    PERMANENT_PHONE_NUMBERS_PATH,
    INITIALIZE_EVENTS_PATH,
  ];
  if (resolvedMode === "all" || resolvedMode === "all_with_phone") {
    const storyPaths = listJsonAssets(STORY_EVENTS_DIR).map((entry) =>
      path.join(STORY_EVENTS_DIR, entry)
    );
    const phonePaths =
      resolvedMode === "all_with_phone"
        ? listJsonAssets(PHONE_SCRIPTS_DIR).map((entry) => path.join(PHONE_SCRIPTS_DIR, entry))
        : [];
    await preloadTextAssets([STORY_EVENTS_PATH, ...corePaths, ...storyPaths, ...phonePaths], {
      onProgress: options?.onProgress,
    });
    return;
  }
  await preloadTextAssets(corePaths, { onProgress: options?.onProgress });
};

let trainerClassBaseRewardsCache: Record<string, number> | null = null;

const isAsmNumericToken = (token: string): boolean => {
  const trimmed = token.trim();
  if (!trimmed) {
    return false;
  }
  if (trimmed.startsWith("$") || trimmed.startsWith("%")) {
    return true;
  }
  if (trimmed.toLowerCase().startsWith("0x")) {
    return true;
  }
  return /^[0-9]/.test(trimmed);
};

const parseTrainerClassBaseRewards = (): Record<string, number> => {
  if (isBrowser) {
    return {};
  }
  const constantsPath = path.join(getDisassemblyRoot(), "constants", "trainer_constants.asm");
  const lines = readAsmLines(constantsPath);
  const classOrder: string[] = [];
  for (const raw of lines) {
    const line = stripAsmComment(raw).trim();
    if (!line.startsWith("trainerclass ")) {
      continue;
    }
    const tokens = line.split(/\s+/);
    const className = tokens[1];
    if (!className || className === "TRAINER_NONE") {
      continue;
    }
    classOrder.push(className);
  }
  if (!classOrder.length) {
    return {};
  }
  const attributesPath = path.join(getDisassemblyRoot(), "data", "trainers", "attributes.asm");
  const attributeLines = readAsmLines(attributesPath);
  const rewards: number[] = [];
  for (const raw of attributeLines) {
    const line = stripAsmComment(raw).trim();
    if (!line.startsWith("db ")) {
      continue;
    }
    const token = line.slice(3).trim().split(/[,\s]+/)[0];
    if (!token || !isAsmNumericToken(token)) {
      continue;
    }
    rewards.push(parseScriptInt(token));
  }
  if (!rewards.length) {
    return {};
  }
  if (rewards.length !== classOrder.length) {
    throw new Error(
      `Trainer class base rewards (${rewards.length}) do not match trainer classes (${classOrder.length}).`
    );
  }
  const result: Record<string, number> = {};
  for (let i = 0; i < classOrder.length; i += 1) {
    result[classOrder[i]] = rewards[i];
  }
  return result;
};

const getTrainerClassBaseRewards = (): Record<string, number> => {
  if (!trainerClassBaseRewardsCache) {
    // ASM: pokecrystal_disassembly/data/trainers/attributes.asm::TrainerClassAttributes
    const parsed = parseTrainerClassBaseRewards();
    if (Object.keys(parsed).length) {
      trainerClassBaseRewardsCache = parsed;
    }
    return parsed;
  }
  return trainerClassBaseRewardsCache;
};

const normalizeScriptName = (label: string): string => {
  const cleaned = String(label ?? "").split(";", 1)[0].trim();
  return normalizeLabel(cleaned);
};

const loadPcStrings = (): Record<string, string> => {
  if (!assetExists(PC_STRINGS_PATH)) {
    throw new Error("ASM-backed PC string data is required; pc_strings.json is missing.");
  }
  const raw = readJsonSync(PC_STRINGS_PATH);
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error("ASM-backed PC string data is required; pc_strings.json must contain an object.");
  }
  const texts: Record<string, string> = {};
  for (const [label, text] of Object.entries(raw as Record<string, unknown>)) {
    const normalized = normalizeScriptName(label);
    if (!normalized) {
      continue;
    }
    if (typeof text !== "string") {
      throw new Error(
        `ASM-backed PC string data is required; pc_strings.json entry ${label} must be a string.`
      );
    }
    texts[normalized] = text;
  }
  return texts;
};

const trainerAliases = (trainer: Trainer): string[] => {
  const aliases = new Set<string>();
  const push = (value: string | null | undefined): void => {
    const normalized = String(value ?? "").trim();
    if (!normalized) {
      return;
    }
    aliases.add(normalized);
  };
  push(trainer.trainer_id);
  push(trainer.name);
  push(String(trainer.name ?? "").replace(/@+$/g, "").trim());
  return Array.from(aliases);
};

const normalizeMapName = (mapName: string): string => {
  if (!mapName) {
    return mapName;
  }
  if (mapName.includes("_") && mapName.toUpperCase() === mapName) {
    return mapConstantToName(mapName);
  }
  return mapName;
};

const loadMapScripts = (mapName: string): Record<string, ScriptData> | null => {
  const normalized = normalizeMapName(String(mapName ?? "").trim());
  if (!normalized) {
    return null;
  }
  if (mapScriptsCache.has(normalized)) {
    return mapScriptsCache.get(normalized) ?? null;
  }
  const cachedStoryMap = storyEventsCache?.[normalized];
  if (
    storyEventsLoadedFromCompiledContentPack &&
    cachedStoryMap &&
    typeof cachedStoryMap === "object" &&
    !Array.isArray(cachedStoryMap)
  ) {
    const scripts = cachedStoryMap as Record<string, ScriptData>;
    mapScriptsCache.set(normalized, scripts);
    return scripts;
  }
  const mapPath = path.join(MAP_SCRIPTS_DIR, `${normalized}.json`);
  if (!assetExists(mapPath)) {
    mapScriptsCache.set(normalized, null);
    return null;
  }
  try {
    const raw = readJsonSync(mapPath);
    if (raw && typeof raw === "object") {
      const scripts = raw as Record<string, ScriptData>;
      mapScriptsCache.set(normalized, scripts);
      return scripts;
    }
    throw new Error(`Map script payload at ${mapPath} must be a JSON object.`);
  } catch (error) {
    throw new Error(
      `ASM-backed map scripts are required for ${normalized}; failed to load ${mapPath}`,
      { cause: error }
    );
  }
};

const extractTextBlocks = (scriptData: ScriptData): string | null => {
  const chunks: string[] = [];
  for (const entry of scriptData) {
    if (!isScriptEntry(entry)) {
      continue;
    }
    const command = getEntryCommand(entry, { lower: true });
    if (
      command !== "text_block" &&
      command !== "text" &&
      command !== "line" &&
      command !== "para" &&
      command !== "cont"
    ) {
      continue;
    }
    const text = getEntryText(entry);
    if (text !== null && text !== undefined) {
      chunks.push(
        String(text)
          .replace(/^"/, "")
          .replace(/"$/, "")
          .replace(/@$/, "")
      );
    }
  }
  if (!chunks.length) {
    return null;
  }
  return chunks.join("\n");
};

const FALLTHROUGH_TERMINATORS = new Set([
  "end",
  "endcallback",
  "trainer",
  "itemball",
  "hiddenitem",
  "fruittree",
  "prompt",
  "next",
  "done",
  "step_end",
  "db",
  "dw",
  "dbw",
  "dbb",
  "dbbw",
  "dba",
  "dn",
  "menu_coords",
  "cmdqueue",
  "stonetable",
]);
const FALLTHROUGH_JUMPS = new Set([
  "sjump",
  "jump",
  "jumptext",
  "jumptextfaceplayer",
  "jumpstd",
]);
const TEXT_SCRIPT_COMMANDS = new Set([
  "text",
  "line",
  "para",
  "cont",
  "done",
  "text_ram",
  "text_start",
  "text_block",
  "text_decimal",
  "text_promptbutton",
  "text_end",
  "prompt",
  "next",
]);
const MOVEMENT_DATA_COMMANDS = new Set([
  "step",
  "slow_step",
  "slow_jump_step",
  "slow_slide_step",
  "jump_step",
  "slide_step",
  "big_step",
  "fast_jump_step",
  "fast_slide_step",
  "turn_head",
  "step_sleep",
  "tree_shake",
  "fix_facing",
  "remove_fixed_facing",
  "set_sliding",
  "remove_sliding",
  "skyfall_top",
  "teleport_from",
  "teleport_to",
  "step_end",
]);
const NON_SCRIPT_DATA_COMMANDS = new Set([
  "db",
  "dw",
  "dbw",
  "dbb",
  "dbbw",
  "dba",
  "dn",
  "menu_coords",
  "cmdqueue",
  "stonetable",
]);

const extractLastCommand = (scriptData: ScriptData): string | null => {
  for (let i = scriptData.length - 1; i >= 0; i -= 1) {
    const entry = scriptData[i];
    if (!isScriptEntry(entry)) {
      continue;
    }
    const command = getEntryCommand(entry, { trim: true, lower: true });
    if (command) {
      return command;
    }
  }
  return null;
};

const isTextScript = (scriptData: ScriptData): boolean => {
  if (!Array.isArray(scriptData) || !scriptData.length) {
    return false;
  }
  return scriptData.every((entry) => {
    if (!isScriptEntry(entry)) {
      return false;
    }
    const command = getEntryCommand(entry, { trim: true, lower: true });
    return Boolean(command) && TEXT_SCRIPT_COMMANDS.has(command);
  });
};

const isMovementDataScript = (scriptData: ScriptData): boolean => {
  if (!Array.isArray(scriptData) || !scriptData.length) {
    return false;
  }
  return scriptData.every((entry) => {
    if (!isScriptEntry(entry)) {
      return false;
    }
    const command = getEntryCommand(entry, { trim: true, lower: true });
    return Boolean(command) && MOVEMENT_DATA_COMMANDS.has(command);
  });
};

const isNonScriptDataScript = (scriptData: ScriptData): boolean => {
  if (!Array.isArray(scriptData) || !scriptData.length) {
    return false;
  }
  return scriptData.every((entry) => {
    if (!isScriptEntry(entry)) {
      return false;
    }
    const command = getEntryCommand(entry, { trim: true, lower: true });
    return Boolean(command) && NON_SCRIPT_DATA_COMMANDS.has(command);
  });
};

const shouldAllowFallthrough = (scriptData: ScriptData): boolean => {
  if (!scriptData.length) {
    return false;
  }
  const lastCommand = extractLastCommand(scriptData);
  if (!lastCommand) {
    return true;
  }
  if (FALLTHROUGH_TERMINATORS.has(lastCommand)) {
    return false;
  }
  if (FALLTHROUGH_JUMPS.has(lastCommand)) {
    return false;
  }
  return true;
};

const successorKey = (parent: string | null | undefined, name: string): string => {
  const parentKey = parent ?? "";
  return `${parentKey}::${name}`;
};

const splitLocalLabel = (
  label: string,
  currentBase: string | null
): { localName: string; parentName: string | null } | null => {
  if (!label.startsWith(".")) {
    return null;
  }
  const atIndex = label.indexOf("@");
  if (atIndex !== -1) {
    const localName = label.slice(0, atIndex);
    const parentName = label.slice(atIndex + 1) || null;
    return { localName, parentName };
  }
  return { localName: label, parentName: currentBase };
};

type StoryIndexes = {
  scripts: Record<string, ScriptData>;
  labelToMap: Record<string, string>;
  texts: Record<string, string>;
  successors: Record<string, [string | null, string]>;
  localScripts: Record<string, Record<string, ScriptData>>;
};

const asStoryMapScriptsRecord = (
  value: unknown
): Record<string, ScriptData> | null => {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  const entries = Object.entries(value as Record<string, unknown>);
  if (entries.length === 0) {
    return null;
  }
  let mapNameFromScripts: string | null = null;
  for (const [label, scriptData] of entries) {
    if (!Array.isArray(scriptData)) {
      return null;
    }
    if (label.endsWith("_MapScripts")) {
      mapNameFromScripts = label.slice(0, -"_MapScripts".length).trim() || null;
    }
  }
  if (!mapNameFromScripts) {
    return null;
  }
  return value as Record<string, ScriptData>;
};

const mergeMapScriptPayload = (
  cache: StoryEvents,
  payload: unknown,
  sourceLabel: string
): void => {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    throw new Error(`${sourceLabel} must contain an object.`);
  }
  const flatMapScripts = asStoryMapScriptsRecord(payload);
  if (flatMapScripts) {
    const [mapScriptsLabel] = Object.keys(flatMapScripts).filter((label) =>
      label.endsWith("_MapScripts")
    );
    const inferredMapName = normalizeMapName(
      mapScriptsLabel.slice(0, -"_MapScripts".length).trim()
    );
    const existing = cache[inferredMapName];
    if (existing && typeof existing === "object" && !Array.isArray(existing)) {
      cache[inferredMapName] = {
        ...(existing as Record<string, ScriptData>),
        ...flatMapScripts,
      };
    } else {
      cache[inferredMapName] = flatMapScripts;
    }
    return;
  }
  for (const [mapName, scriptsPayload] of Object.entries(payload as Record<string, unknown>)) {
    const normalizedMapName = normalizeMapName(String(mapName ?? "").trim());
    if (!normalizedMapName) {
      continue;
    }
    if (!scriptsPayload || typeof scriptsPayload !== "object" || Array.isArray(scriptsPayload)) {
      throw new Error(`${sourceLabel} entry ${mapName} must contain an object.`);
    }
    const existing = cache[normalizedMapName];
    if (existing && typeof existing === "object" && !Array.isArray(existing)) {
      cache[normalizedMapName] = {
        ...(existing as Record<string, ScriptData>),
        ...(scriptsPayload as Record<string, ScriptData>),
      };
    } else {
      cache[normalizedMapName] = scriptsPayload as ScriptData | Record<string, ScriptData>;
    }
  }
};

type StoryIndexState = {
  lastTopLevel: { name: string; script: ScriptData } | null;
};

const indexStoryRoot = (
  rootKey: string,
  rootValue: unknown,
  indexes: StoryIndexes,
  state: StoryIndexState
): void => {
  const { scripts, labelToMap, texts, successors, localScripts } = indexes;
  if (Array.isArray(rootValue)) {
    const normalized = normalizeLabel(rootKey);
    const text = extractTextBlocks(rootValue as ScriptData);
    if (text) {
      texts[normalized] = text;
    }
    if (
      isTextScript(rootValue as ScriptData) ||
      isMovementDataScript(rootValue as ScriptData) ||
      isNonScriptDataScript(rootValue as ScriptData)
    ) {
      scripts[normalized] = rootValue as ScriptData;
      state.lastTopLevel = null;
    } else {
      scripts[normalized] = rootValue as ScriptData;
      if (state.lastTopLevel && shouldAllowFallthrough(state.lastTopLevel.script)) {
        successors[successorKey(null, state.lastTopLevel.name)] = [null, normalized];
      }
      state.lastTopLevel = { name: normalized, script: rootValue as ScriptData };
    }
    return;
  }
  if (!rootValue || typeof rootValue !== "object") {
    return;
  }
  const mapOrder = Object.entries(rootValue as Record<string, unknown>);
  for (const [label, script] of mapOrder) {
    if (!Array.isArray(script)) {
      throw new Error(
        `ASM-backed story events are required; ${rootKey}.${label} must be an array.`
      );
    }
  }
  let currentBase: string | null = null;
  const lastInBase: Record<
    string,
    { name: string; parent: string | null; script: ScriptData } | null
  > = {};
  for (const [label, script] of mapOrder) {
    const normalized = normalizeLabel(label);
    if (normalized.endsWith("_MapScripts") || normalized.endsWith("_MapEvents")) {
      continue;
    }
    const isText = isTextScript(script as ScriptData);
    const isMovementData = isMovementDataScript(script as ScriptData);
    const isStaticData = isNonScriptDataScript(script as ScriptData);
    const isDataLabel = isText || isMovementData || isStaticData;
    const isLocal = normalized.startsWith(".");
    const previousBase = currentBase;
    const localInfo: { localName: string; parentName: string | null } | null =
      isLocal ? splitLocalLabel(normalized, currentBase) : null;
    if (!isLocal) {
      const previousLocal = previousBase ? lastInBase[previousBase] : null;
      if (!isDataLabel && previousLocal && shouldAllowFallthrough(previousLocal.script)) {
        successors[successorKey(previousLocal.parent, previousLocal.name)] = [
          null,
          normalized,
        ];
      }
      currentBase = normalized;
    } else if (!currentBase) {
      currentBase = localInfo?.parentName ?? normalized;
    }
    const parentContext = isLocal ? localInfo?.parentName ?? currentBase : null;
    const entryName = localInfo?.localName ?? normalized;
    const baseKey = parentContext ?? "";
    if (isLocal) {
      if (localInfo?.parentName && !localInfo.parentName.startsWith(".")) {
        if (!localScripts[localInfo.parentName]) {
          localScripts[localInfo.parentName] = {};
        }
        localScripts[localInfo.parentName][localInfo.localName] = script as ScriptData;
      }
    }
    if (isDataLabel) {
      lastInBase[baseKey] = null;
      continue;
    }
    const prevEntry = lastInBase[baseKey];
    if (prevEntry) {
      if (shouldAllowFallthrough(prevEntry.script)) {
        successors[successorKey(prevEntry.parent, prevEntry.name)] = [
          parentContext,
          entryName,
        ];
      }
    }
    lastInBase[baseKey] = { name: entryName, parent: parentContext, script: script as ScriptData };
  }
  for (const [label, script] of Object.entries(rootValue as Record<string, unknown>)) {
    const normalized = normalizeLabel(label);
    const text = extractTextBlocks(script as ScriptData);
    if (text) {
      texts[normalized] = text;
    }
    if (isTextScript(script as ScriptData)) {
      continue;
    }
    if (!normalized.startsWith(".")) {
      scripts[normalized] = script as ScriptData;
      labelToMap[normalized] = rootKey;
    }
  }
};

const ensureStoryEventsLoaded = (): void => {
  if (storyScriptIndex) {
    return;
  }
  const scripts: Record<string, ScriptData> = {};
  const labelToMap: Record<string, string> = {};
  const texts: Record<string, string> = {};
  const successors: Record<string, [string | null, string]> = {};
  const localScripts: Record<string, Record<string, ScriptData>> = {};

  try {
    storyEventsCache = readJsonSync(STORY_EVENTS_PATH) as StoryEvents;
  } catch {
    storyEventsCache = {};
  }

  const hasCompiledContentPack = hasEnabledCompiledContentPackSync();
  storyEventsLoadedFromCompiledContentPack = hasCompiledContentPack;
  if (!hasCompiledContentPack) {
    const orderedMapEntries = listJsonAssets(MAP_SCRIPTS_DIR).sort((a, b) =>
      a.localeCompare(b)
    );
    for (const entryName of orderedMapEntries) {
      const filePath = path.join(MAP_SCRIPTS_DIR, entryName);
      const payload = readJsonSync(filePath);
      storyEventsCache = storyEventsCache ?? {};
      mergeMapScriptPayload(
        storyEventsCache,
        payload,
        `ASM-backed map scripts are required; ${filePath}`
      );
    }
  }

  if (!hasCompiledContentPack) {
    const orderedStoryEntries = listJsonAssets(STORY_EVENTS_DIR).sort((a, b) =>
      a.localeCompare(b)
    );
    for (const entryName of orderedStoryEntries) {
      const filePath = path.join(STORY_EVENTS_DIR, entryName);
      const payload = readJsonSync(filePath);
      storyEventsCache = storyEventsCache ?? {};
      mergeMapScriptPayload(
        storyEventsCache,
        payload,
        `ASM-backed story events are required; ${filePath}`
      );
    }
  }

  for (const payload of loadContentPackCategoryJsonSync("maps")) {
    storyEventsCache = storyEventsCache ?? {};
    mergeMapScriptPayload(storyEventsCache, payload, "Map content pack payload");
  }

  for (const payload of loadContentPackCategoryJsonSync("story_events")) {
    storyEventsCache = storyEventsCache ?? {};
    mergeMapScriptPayload(storyEventsCache, payload, "Story-event content pack payload");
  }

  const state: StoryIndexState = { lastTopLevel: null };
  const indexes: StoryIndexes = { scripts, labelToMap, texts, successors, localScripts };
  for (const [rootKey, rootValue] of Object.entries(storyEventsCache ?? {})) {
    indexStoryRoot(rootKey, rootValue, indexes, state);
  }

  storyScriptIndex = scripts;
  storyLabelToMap = labelToMap;
  storyTextIndex = texts;
  storyScriptSuccessors = successors;
  storyLocalScripts = localScripts;
};

const ensurePcStringsLoaded = (): void => {
  ensureStoryEventsLoaded();
  if (pcStringsLoaded) {
    return;
  }
  const pcStrings = loadPcStrings();
  const texts = storyTextIndex ?? {};
  for (const [label, text] of Object.entries(pcStrings)) {
    if (!label) {
      continue;
    }
    texts[label] = text;
  }
  storyTextIndex = texts;
  pcStringsLoaded = true;
};

const ensurePhoneScriptsLoaded = (): void => {
  if (phoneScriptsLoaded) {
    return;
  }
  ensureStoryEventsLoaded();
  phoneScriptsLoaded = true;
  const orderedPhoneEntries = listJsonAssets(PHONE_SCRIPTS_DIR).sort((a, b) =>
    a.localeCompare(b)
  );
  if (!orderedPhoneEntries.length) {
    return;
  }
  const phoneScripts: Record<string, ScriptData> = {};
  for (const entryName of orderedPhoneEntries) {
    const filePath = path.join(PHONE_SCRIPTS_DIR, entryName);
    const payload = readJsonSync(filePath);
    if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
      throw new Error(
        `ASM-backed phone scripts are required; ${filePath} must contain an object.`
      );
    }
    for (const [label, script] of Object.entries(payload as Record<string, unknown>)) {
      if (!Array.isArray(script)) {
        throw new Error(
          `ASM-backed phone scripts are required; ${filePath} entry ${label} must be an array.`
        );
      }
      const normalized = normalizeLabel(label);
      phoneScripts[normalized] = script as ScriptData;
    }
  }
  for (const payload of loadContentPackCategoryJsonSync("phone_scripts")) {
    if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
      throw new Error("Phone-script content pack payloads must be objects keyed by script labels.");
    }
    for (const [label, script] of Object.entries(payload as Record<string, unknown>)) {
      if (!Array.isArray(script)) {
        throw new Error(`Phone-script content pack entry ${label} must be an array.`);
      }
      const normalized = normalizeLabel(label);
      phoneScripts[normalized] = script as ScriptData;
    }
  }
  if (!Object.keys(phoneScripts).length) {
    return;
  }
  storyEventsCache = storyEventsCache ?? {};
  if (!("PhoneScripts" in storyEventsCache)) {
    storyEventsCache["PhoneScripts"] = phoneScripts;
  } else if (storyEventsCache["PhoneScripts"] && typeof storyEventsCache["PhoneScripts"] === "object") {
    storyEventsCache["PhoneScripts"] = {
      ...(storyEventsCache["PhoneScripts"] as Record<string, ScriptData>),
      ...phoneScripts,
    };
  } else {
    storyEventsCache["PhoneScripts"] = phoneScripts;
  }
  if (
    !storyScriptIndex ||
    !storyLabelToMap ||
    !storyTextIndex ||
    !storyScriptSuccessors ||
    !storyLocalScripts
  ) {
    return;
  }
  const indexes: StoryIndexes = {
    scripts: storyScriptIndex,
    labelToMap: storyLabelToMap,
    texts: storyTextIndex,
    successors: storyScriptSuccessors,
    localScripts: storyLocalScripts,
  };
  const state: StoryIndexState = { lastTopLevel: null };
  indexStoryRoot("PhoneScripts", storyEventsCache["PhoneScripts"], indexes, state);
};

const buildMapScriptSuccessors = (
  mapScripts: Record<string, ScriptData> | null,
): Record<string, [string | null, string]> => {
  const successors: Record<string, [string | null, string]> = {};
  if (!mapScripts) {
    return successors;
  }
  const mapOrder = Object.entries(mapScripts);
  let currentBase: string | null = null;
  const lastInBase: Record<
    string,
    { name: string; parent: string | null; script: ScriptData } | null
  > = {};
  for (const [label, script] of mapOrder) {
    if (!Array.isArray(script)) {
      continue;
    }
    const normalized = normalizeLabel(label);
    if (normalized.endsWith("_MapScripts") || normalized.endsWith("_MapEvents")) {
      continue;
    }
    const isText = isTextScript(script as ScriptData);
    const isMovementData = isMovementDataScript(script as ScriptData);
    const isStaticData = isNonScriptDataScript(script as ScriptData);
    const isDataLabel = isText || isMovementData || isStaticData;
    const isLocal = normalized.startsWith(".");
    const previousBase = currentBase;
    const localInfo: { localName: string; parentName: string | null } | null =
      isLocal ? splitLocalLabel(normalized, currentBase) : null;
    if (!isLocal) {
      const previousLocal = previousBase ? lastInBase[previousBase] : null;
      if (!isDataLabel && previousLocal && shouldAllowFallthrough(previousLocal.script)) {
        successors[successorKey(previousLocal.parent, previousLocal.name)] = [
          null,
          normalized,
        ];
      }
      currentBase = normalized;
    } else if (!currentBase) {
      currentBase = localInfo?.parentName ?? normalized;
    }
    const parentContext = isLocal ? localInfo?.parentName ?? currentBase : null;
    const entryName = localInfo?.localName ?? normalized;
    const baseKey = parentContext ?? "";
    if (isDataLabel) {
      lastInBase[baseKey] = null;
      continue;
    }
    const prevEntry = lastInBase[baseKey];
    if (prevEntry) {
      if (shouldAllowFallthrough(prevEntry.script)) {
        successors[successorKey(prevEntry.parent, prevEntry.name)] = [parentContext, entryName];
      }
    }
    lastInBase[baseKey] = { name: entryName, parent: parentContext, script: script as ScriptData };
  }
  return successors;
};

type MapDimensions = { width: number; height: number };

const getScriptCommand = (entry: ScriptEntry): string => String(entry.command ?? "");

const normalizeMapKey = (mapName: string): string => {
  const result: string[] = [];
  let previous = "";
  for (let i = 0; i < mapName.length; i += 1) {
    const char = mapName[i];
    if (char === "_") {
      if (result.length > 0 && result[result.length - 1] !== "_") {
        result.push("_");
      }
      previous = char;
      continue;
    }
    if (char >= "A" && char <= "Z") {
      if (i > 0 && result.length > 0 && result[result.length - 1] !== "_") {
        if (
          (previous >= "a" && previous <= "z") ||
          (previous >= "0" && previous <= "9")
        ) {
          if (!((previous >= "0" && previous <= "9") && char === "F")) {
            result.push("_");
          }
        }
      }
      result.push(char);
    } else if (char >= "0" && char <= "9") {
      if (i > 0 && result.length > 0 && result[result.length - 1] !== "_") {
        if (previous >= "a" && previous <= "z") {
          result.push("_");
        }
      }
      result.push(char);
    } else {
      if (i > 0 && result.length > 0 && result[result.length - 1] !== "_") {
        if (previous >= "0" && previous <= "9") {
          result.push("_");
        }
      }
      result.push(char.toUpperCase());
    }
    previous = char;
  }
  return result.join("").replace(/_+/g, "_").replace(/^_|_$/g, "");
};

const parseScriptInt = (token: string): number => {
  const normalized = token.trim();
  if (!normalized) {
    throw new Error("Cannot parse an empty numeric token.");
  }
  let sign = 1;
  let valueToken = normalized;
  if (valueToken.startsWith("-")) {
    sign = -1;
    valueToken = valueToken.slice(1);
  } else if (valueToken.startsWith("+")) {
    valueToken = valueToken.slice(1);
  }
  let base = 10;
  if (valueToken.startsWith("$")) {
    base = 16;
    valueToken = valueToken.slice(1);
  } else if (valueToken.toLowerCase().startsWith("0x")) {
    base = 16;
    valueToken = valueToken.slice(2);
  } else if (valueToken.startsWith("%")) {
    base = 2;
    valueToken = valueToken.slice(1);
  }
  if (!valueToken) {
    throw new Error(`Numeric token '${token}' does not contain digits.`);
  }
  const parsed = parseInt(valueToken, base);
  if (Number.isNaN(parsed)) {
    throw new Error(`Numeric token '${token}' could not be parsed.`);
  }
  return sign * parsed;
};

const parseArgsList = (raw: unknown): string[] => {
  if (raw == null) {
    return [];
  }
  if (Array.isArray(raw)) {
    return raw
      .map((value) => String(value).split(";", 1)[0].trim())
      .filter((value) => value.length > 0);
  }
  const cleaned = String(raw).split(";", 1)[0].trim();
  if (!cleaned) {
    return [];
  }
  return cleaned
    .split(",")
    .map((value) => value.trim())
    .filter((value) => value.length > 0);
};

const POKECENTER_MAP_SUFFIX = "pokecenter1f";
const POKECENTER_BG_COORDINATES: Record<string, [number, number][]> = {
    "default": [[9, 1]],
    "indigoplateaupokecenter1f": [[7, 7]],
};

const PC_SCRIPT_NAME = "PCScript";

/**
 * Injects PC background events into Pokémon Center maps.
 * The original map data dumps are missing PC events, so we inject them here to match in-game behavior.
 */
const maybeAddPokecenterEvents = (mapName: string, bg_events: MapEventsType["bg_events"]): void => {
    if (typeof mapName !== 'string') {
        return;
    }

    const normalized = mapName.trim().toLowerCase();
    if (!normalized.endsWith(POKECENTER_MAP_SUFFIX)) {
        return;
    }

    const coords = POKECENTER_BG_COORDINATES[normalized] ?? POKECENTER_BG_COORDINATES["default"];

    for (const [x, y] of coords) {
        const eventExists = bg_events.some(
            event =>
                event.x === x &&
                event.y === y &&
                event.event_type === "BGEVENT_UP" &&
                event.script.toUpperCase() === PC_SCRIPT_NAME.toUpperCase()
        );

        if (!eventExists) {
            bg_events.push({
                x,
                y,
                event_type: "BGEVENT_UP",
                script: PC_SCRIPT_NAME,
            });
        }
    }
};

const parseMapEvents = (
  mapName: string,
  rows: ScriptData,
  mapDimensions: Map<string, MapDimensions>
): MapEventsType => {
  const warps: MapEventsType["warps"] = [];
  const coord_events: MapEventsType["coord_events"] = [];
  const bg_events: MapEventsType["bg_events"] = [];
  let section: "warps" | "coord_events" | "bg_events" | "object_events" | null = null;
  let warp_index = 0;
  const mapKey = normalizeMapKey(mapName);
  const dimensions = mapDimensions.get(mapKey) ?? null;
  const width = dimensions?.width ?? null;
  const height = dimensions?.height ?? null;
  const eventStride = Math.max(1, Math.floor(METATILE_WIDTH / 2));
  const maxEventX = typeof width === "number" ? width * eventStride - 1 : null;
  const maxEventY = typeof height === "number" ? height * eventStride - 1 : null;

  for (const entry of rows as ScriptEntry[]) {
    if (!entry || typeof entry !== "object") {
      continue;
    }
    const command = String(entry.command ?? "").trim();
    const argsList = parseArgsList(entry.args);
    if (command === "def_warp_events") {
      section = "warps";
      warp_index = 0;
      continue;
    }
    if (command === "def_coord_events") {
      section = "coord_events";
      continue;
    }
    if (command === "def_bg_events") {
      section = "bg_events";
      continue;
    }
    if (command === "def_object_events") {
      section = "object_events";
      continue;
    }

    if (section === "warps" && command === "warp_event") {
      if (argsList.length !== 4) {
        throw new Error(
          `Malformed warp_event in ${mapName}: expected 4 args, found ${argsList.length}.`
        );
      }
      const raw_x = argsList[0];
      const raw_y = argsList[1];
      const target_map_constant = String(argsList[2]).replace(/,+$/, "");
      const raw_warp = argsList[3];
      let x = parseScriptInt(raw_x);
      let y = parseScriptInt(raw_y);
      if (typeof maxEventX === "number" && typeof maxEventY === "number") {
        if (x > maxEventX) {
          x = maxEventX;
        }
        if (y > maxEventY) {
          y = maxEventY;
        }
      }
      warp_index += 1;
      warps.push({
        index: warp_index,
        x,
        y,
        target_map_constant,
        target_map: mapConstantToName(target_map_constant),
        target_warp_id: parseScriptInt(raw_warp),
      });
      continue;
    }

    if (section === "coord_events" && command === "coord_event") {
      if (argsList.length !== 4) {
        throw new Error(
          `Malformed coord_event in ${mapName}: expected 4 args, found ${argsList.length}.`
        );
      }
      coord_events.push({
        x: parseScriptInt(argsList[0]),
        y: parseScriptInt(argsList[1]),
        scene_id: argsList[2],
        script_name: argsList[3],
      });
      continue;
    }

    if (section === "bg_events" && command === "bg_event") {
      if (argsList.length !== 4) {
        throw new Error(
          `Malformed bg_event in ${mapName}: expected 4 args, found ${argsList.length}.`
        );
      }
      bg_events.push({
        x: parseScriptInt(argsList[0]),
        y: parseScriptInt(argsList[1]),
        event_type: String(argsList[2]).trim().toUpperCase(),
        script: String(argsList[3]).trim(),
      });
    }
  }

  maybeAddPokecenterEvents(mapName, bg_events);
  return MapEventsSchema.parse({ warps, coord_events, bg_events });
};

const parseMapScripts = (rows: ScriptData): {
  callbacks: Array<[string, string]>;
  sceneMap: Record<string, string>;
  sceneOrder: string[];
  defaultScene: string | null;
} => {
  const callbacks: Array<[string, string]> = [];
  const sceneMap: Record<string, string> = {};
  const sceneOrder: string[] = [];
  let defaultScene: string | null = null;
  let lastSceneScript: string | null = null;

  for (const entry of rows as ScriptEntry[]) {
    if (!entry || typeof entry !== "object") {
      continue;
    }
    const command = String(entry.command ?? "").trim();
    const argsList = parseArgsList(entry.args);
    if (command === "callback") {
      if (argsList.length >= 2) {
        callbacks.push([argsList[0], argsList[1]]);
      }
    } else if (command === "scene_script") {
      if (argsList.length >= 2) {
        const script_ref = argsList[0];
        const scene_name = argsList[1];
        sceneMap[scene_name] = script_ref;
        sceneOrder.push(scene_name);
        lastSceneScript = script_ref;
        if (!defaultScene) {
          defaultScene = scene_name;
        }
      }
    } else if (command === "scene_const") {
      if (argsList.length >= 1 && lastSceneScript) {
        const scene_name = argsList[0];
        sceneMap[scene_name] = lastSceneScript;
        sceneOrder.push(scene_name);
        if (!defaultScene) {
          defaultScene = scene_name;
        }
      }
    }
  }

  return { callbacks, sceneMap, sceneOrder, defaultScene };
};

export const speciesMap: Map<string, PokemonSpecies> = new Map();
export const movesMap: Map<MoveName, Move> = new Map();
export const itemsMap: Map<string, Item> = new Map();

const storeSpecies = (key: string, species: PokemonSpecies): void => {
  const normalized = String(key ?? species.id ?? "").toUpperCase();
  if (!normalized) {
    return;
  }
  speciesData[normalized] = species;
  speciesMap.set(normalized, species);
};

export async function loadSpecies(): Promise<void> {
  const data = loadMergedPokemonDataSync();
  if (Array.isArray(data)) {
    for (const entry of data) {
      const species = PokemonSpeciesSchema.parse(entry);
      storeSpecies(species.id, species);
    }
    return;
  }
  for (const [key, value] of Object.entries(data as Record<string, unknown>)) {
    const species = PokemonSpeciesSchema.parse(value);
    storeSpecies(species.id ?? key, species);
  }
}

export function getSpecies(name: string): PokemonSpecies {
  return speciesData[String(name ?? "").toUpperCase()];
}

export function loadAllSpecies(): Map<string, PokemonSpecies> {
  if (speciesMap.size > 0) {
    return speciesMap;
  }
  const data = loadMergedPokemonDataSync();
  if (Array.isArray(data)) {
    for (const entry of data) {
      const species = PokemonSpeciesSchema.parse(entry);
      storeSpecies(species.id, species);
    }
    return speciesMap;
  }
  for (const [key, value] of Object.entries(data as Record<string, unknown>)) {
    const species = PokemonSpeciesSchema.parse(value);
    storeSpecies(species.id ?? key, species);
  }
  return speciesMap;
}

export async function loadMoves(): Promise<void> {
  const data = loadMergedMovesDataSync();
  for (const key in data) {
    const move = MoveSchema.parse(data[key]);
    const token = String(move.name ?? key).toUpperCase();
    moveData[token] = move;
    movesMap.set(token as MoveName, move);
  }
}

export function loadAllMoves(): Map<MoveName, Move> {
  const hasMoveRecord = Object.keys(moveData).length > 0;
  if (movesMap.size > 0 && hasMoveRecord) {
    return movesMap;
  }
  const data = loadMergedMovesDataSync();
  if (!data || typeof data !== "object") {
    throw new Error("moves_data.json did not load move metadata.");
  }
  for (const [key, value] of Object.entries(data as Record<string, unknown>)) {
    const move = MoveSchema.parse(value);
    const token = String(move.name ?? key).toUpperCase();
    moveData[token] = move;
    movesMap.set(token as MoveName, move);
  }
  return movesMap;
}

export function getMove(name: string): Move {
  return moveData[name];
}

export async function loadItems(): Promise<void> {
  const dataPath = path.join(DATA_DIR, "items.json");
  try {
    const baseRaw = await fsPromises.readFile(dataPath, "utf-8");
    const baseData = JSON.parse(baseRaw);
    if (!Array.isArray(baseData)) {
      throw new Error(`ASM-backed item data is required; failed to load ${dataPath}.`);
    }
    const data = loadMergedItemsSync();
    for (const itemObj of data as unknown[]) {
      const item = ItemSchema.parse(itemObj) as Item;
      itemsMap.set(item.name, item);
    }
  } catch (error) {
    throw new Error(
      "ASM-backed item data is required; failed to load merged item data.",
      { cause: error }
    );
  }
}

export function getItem(name: string): Item | undefined {
  return itemsMap.get(name);
}

export function loadAllItems(): Map<string, Item> {
    if (itemsMap.size > 0) {
        return itemsMap;
    }
    const dataPath = path.join(DATA_DIR, "items.json");
    try {
        const baseRaw = fs.readFileSync(dataPath, "utf-8");
        const baseData = JSON.parse(baseRaw);
        if (!Array.isArray(baseData)) {
            throw new Error(`ASM-backed item data is required; failed to load ${dataPath}.`);
        }
        const data = loadMergedItemsSync();
        for (const itemObj of data as unknown[]) {
            const item = ItemSchema.parse(itemObj) as Item;
            itemsMap.set(item.name, item);
        }
    } catch (error: unknown) {
        throw new Error(
            "ASM-backed item data is required; failed to load merged item data.",
            { cause: error }
        );
    }
    return itemsMap;
}

export class DataLoader {
    public pokemonData: Map<string, PokemonSpecies> = speciesMap;
    public moveData: Map<MoveName, Move> = movesMap;
    public itemData: Map<string, Item> = itemsMap;
    public trainer_data: Map<string, Trainer> = new Map();
    public trainer_data_by_name: Map<string, Trainer> = new Map();
    public hidden_item_event_flags: Record<string, string> = {};
    public trainer_event_flags: Record<string, string> = {};
    public martData: Map<string, string[]> | undefined;
    public map_attributes: Map<string, MapAttributesType> = new Map();
    public map_dimensions: Map<string, MapDimensions> = new Map();
    public npc_data: Map<string, ObjectEvent[]> = new Map();
    public map_events: Map<string, MapEventsType> = new Map();
    public map_callbacks: Map<string, Array<[string, string]>> = new Map();
    public map_scene_scripts: Map<string, Record<string, string>> = new Map();
    public map_scene_order: Map<string, string[]> = new Map();
    public map_default_scene: Record<string, string> = {};
    public wild_encounter_data?: Map<string, WildEncounterData>;
    public wildEncounterData?: Map<string, WildEncounterData>;
    public mapAttributes = this.map_attributes;
    public mapEvents = this.map_events;
    public mapCallbacks = this.map_callbacks;
    public mapSceneScripts = this.map_scene_scripts;
    public mapSceneOrder = this.map_scene_order;
    public Tileset?: TilesetConstructor;

    private _loaded_categories = new Set<string>();
    private _loaded_map_scripts = new Set<string>();
    private _bg_event_script_flags: Map<string, string | null> = new Map();
    private _script_event_flags: Map<string, Set<string>> = new Map();

    private _set_map_entry<T>(map: Map<string, T>, key: string, value: T): void {
        map.set(key, value);
    }

    public ensureMenuData(): void {
        // Mirror menu-time asset loads expected by ASM-driven UI flows.
        loadAllSpecies();
        loadAllMoves();
        loadAllItems();
    }

    public loadMartData(): void {
        if (this.martData) {
            return;
        }
        // ASM mapping: pokecrystal_disassembly/data/items/marts.asm (Marts table).
        const raw = readJsonSync(MARTS_PATH);
        if (!raw || typeof raw !== "object") {
            throw new Error("marts.json did not load mart data.");
        }
        const data = raw as Record<string, unknown>;
        const entries = new Map<string, string[]>();
        for (const [martKey, items] of Object.entries(data)) {
            if (!Array.isArray(items)) {
                throw new Error(`marts.json entry ${martKey} is not an item list.`);
            }
            const normalizedItems = items.map((item) => {
                if (typeof item !== "string") {
                    throw new Error(`marts.json entry ${martKey} contains a non-string item.`);
                }
                return item;
            });
            entries.set(martKey.trim().toUpperCase(), normalizedItems);
        }
        this.martData = entries;
    }

    public get_hidden_item_event_flag(scriptName: string | null): string | null {
        if (!scriptName) {
            return null;
        }
        const normalized = normalizeScriptName(scriptName);
        if (!normalized) {
            return null;
        }
        const lookupKey = normalized.toUpperCase();
        if (lookupKey in this.hidden_item_event_flags) {
            return this.hidden_item_event_flags[lookupKey] ?? null;
        }
        const scriptData = this.get_script(normalized);
        const eventFlag = this._extract_hidden_item_event_flag(scriptData);
        if (eventFlag) {
            this.hidden_item_event_flags[lookupKey] = eventFlag;
        }
        return eventFlag ?? null;
    }

    public get_bg_event_script_flag(scriptName: string | null): string | null {
        if (!scriptName) {
            return null;
        }
        const normalized = normalizeScriptName(scriptName);
        if (!normalized) {
            return null;
        }
        const lookupKey = normalized.toUpperCase();
        if (this._bg_event_script_flags.has(lookupKey)) {
            return this._bg_event_script_flags.get(lookupKey) ?? null;
        }
        const scriptData = this.get_script(normalized);
        const eventFlag = this._extract_bg_event_script_flag(scriptData);
        this._bg_event_script_flags.set(lookupKey, eventFlag);
        return eventFlag ?? null;
    }

    public get_pokemon_species(name: string): PokemonSpecies | null {
        if (this.pokemonData.size === 0) {
            loadAllSpecies();
        }
        const normalized = String(name ?? "").toUpperCase();
        if (!normalized) {
            return null;
        }
        return this.pokemonData.get(normalized) ?? speciesData[normalized] ?? null;
    }

    public getPokemonSpecies(name: string): PokemonSpecies | null {
        return this.get_pokemon_species(name);
    }

    public getSpecies(name: string): PokemonSpecies | null {
        return this.get_pokemon_species(name);
    }

    public get_item(name: string): Item | null {
        if (this.itemData.size === 0) {
            loadAllItems();
        }
        return this.itemData.get(name) ?? null;
    }

    public getItem(name: string): Item | null {
        return this.get_item(name);
    }

    public ensure_battle_data(): void {
        if (this.pokemonData.size === 0) {
            loadAllSpecies();
        }
        if (this.moveData.size === 0) {
            loadAllMoves();
        }
        if (this.itemData.size === 0) {
            loadAllItems();
        }
    }

    public ensure_overworld_data({ map_name }: { map_name?: string } = {}): void {
        this.load_map_attributes();
        this.load_map_dimensions();
        this.load_npc_data();
        this.load_wild_encounter_data();
        if (map_name) {
            this.ensure_map_scripts(map_name);
        } else {
            this.load_map_scripts();
        }
    }

    public ensure_map_scripts(map_name: string): void {
        const normalized = normalizeMapName(String(map_name ?? "").trim());
        if (!normalized) {
            return;
        }
        if (this._loaded_map_scripts.has(normalized)) {
            return;
        }
        this._loaded_map_scripts.add(normalized);
        ensureStoryEventsLoaded();
        const storyMap = storyEventsCache?.[normalized];
        if (!storyMap || typeof storyMap !== "object") {
            return;
        }
        for (const [label, scriptData] of Object.entries(storyMap as Record<string, unknown>)) {
            if (!Array.isArray(scriptData)) {
                continue;
            }
            if (label.endsWith("_MapScripts")) {
                const { callbacks, sceneMap, sceneOrder, defaultScene } = parseMapScripts(scriptData);
                if (callbacks.length) {
                    this._set_map_entry(this.map_callbacks, normalized, callbacks);
                }
                if (Object.keys(sceneMap).length) {
                    this._set_map_entry(this.map_scene_scripts, normalized, sceneMap);
                    this._set_map_entry(this.map_scene_order, normalized, sceneOrder);
                    if (defaultScene) {
                        this.map_default_scene[normalized] = defaultScene;
                    }
                }
            } else if (label.endsWith("_MapEvents")) {
                const events = parseMapEvents(normalized, scriptData, this.map_dimensions);
                this._set_map_entry(this.map_events, normalized, events);
            }
        }
    }

    public load_map_attributes(): void {
        if (this._loaded_categories.has("map_attributes")) {
            return;
        }
        const raw = loadMergedMapAttributesSync();
        if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
            throw new Error(
                "ASM-backed map attributes are required; merged map attributes must be an object."
            );
        }
        for (const [mapName, attributes] of Object.entries(raw as Record<string, unknown>)) {
            const parsed = MapAttributesSchema.parse(attributes);
            this._set_map_entry(this.map_attributes, mapName, parsed);
        }
        this._loaded_categories.add("map_attributes");
    }

    public load_map_dimensions(): void {
        if (this._loaded_categories.has("map_dimensions")) {
            return;
        }
        const raw = loadMergedMapDimensionsSync();
        if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
            throw new Error(
                "ASM-backed map dimensions are required; merged map dimensions must be an object."
            );
        }
        for (const [mapKey, dims] of Object.entries(raw as Record<string, unknown>)) {
            const payload =
              dims && typeof dims === "object" && !Array.isArray(dims)
                ? (dims as Record<string, unknown>)
                : null;
            const width = Number(payload?.width ?? 0);
            const height = Number(payload?.height ?? 0);
            if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) {
                throw new Error(
                    `ASM-backed map dimensions are required; ${mapKey} must provide positive numeric width and height.`
                );
            }
            this._set_map_entry(this.map_dimensions, mapKey, { width, height });
        }
        this._loaded_categories.add("map_dimensions");
    }

    public load_npc_data(): void {
        if (this._loaded_categories.has("npc_data")) {
            return;
        }
        const raw = loadMergedNpcDataSync();
        if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
            throw new Error("ASM-backed NPC data is required; merged NPC data must contain an object.");
        }
        for (const [mapName, entries] of Object.entries(raw as Record<string, unknown>)) {
            if (!Array.isArray(entries)) {
                throw new Error(
                    `ASM-backed NPC data is required; merged NPC data entry ${mapName} must be an array.`
                );
            }
            this._set_map_entry(this.npc_data, mapName, entries as ObjectEvent[]);
        }
        this._loaded_categories.add("npc_data");
    }

    public load_map_scripts(): void {
        if (this._loaded_categories.has("map_scripts")) {
            return;
        }
        ensureStoryEventsLoaded();
        for (const [mapName, payload] of Object.entries(storyEventsCache ?? {})) {
            if (!payload || typeof payload !== "object") {
                continue;
            }
            this.ensure_map_scripts(mapName);
        }
        this._loaded_categories.add("map_scripts");
    }

    public load_wild_encounter_data(): void {
        if (this._loaded_categories.has("wild_encounters")) {
            return;
        }
        const raw = loadMergedWildEncountersSync();
        if (!Array.isArray(raw)) {
            throw new Error(
                "ASM-backed wild encounter data is required; merged encounter payload must be an array."
            );
        }
        const entries = new Map<string, WildEncounterData>();
        for (const entry of raw) {
            const parsed = WildEncounterDataSchema.parse(entry);
            entries.set(parsed.map_name, parsed);
        }
        if (entries.size) {
            this.wild_encounter_data = entries;
            this.wildEncounterData = entries;
        }
        this._loaded_categories.add("wild_encounters");
    }

    public get_script(scriptName: string, parentScript?: string): ScriptData | null {
        if (!scriptName) {
            return null;
        }
        const lookup = (): ScriptData | null => {
            ensureStoryEventsLoaded();
            const normalized = normalizeLabel(scriptName);
            const scripts = storyScriptIndex ?? {};
            const parentNormalized = parentScript ? normalizeLabel(parentScript) : "";
            if (normalized.startsWith(".") && parentNormalized) {
                const localName = normalized.includes("@") ? normalized.split("@", 1)[0] : normalized;
                const explicitParent = normalized.includes("@")
                  ? normalizeLabel(normalized.slice(normalized.indexOf("@") + 1))
                  : "";
                const parentKey = explicitParent ? normalizeLabel(explicitParent) : parentNormalized;
                const locals = storyLocalScripts ?? {};
                const localScript = locals[parentKey]?.[localName];
                if (localScript) {
                  return localScript;
                }
                const mapName = storyLabelToMap?.[parentKey] ?? storyLabelToMap?.[parentNormalized] ?? null;
                if (mapName) {
                  const mapScripts = loadMapScripts(mapName);
                  const mapLocalKey = `${localName}@${parentKey}`;
                  const mapLocal = mapScripts?.[mapLocalKey];
                  if (Array.isArray(mapLocal)) {
                    return mapLocal as ScriptData;
                  }
                }
                return null;
            }
            const direct = scripts[normalized];
            if (direct) {
                const mapName = storyLabelToMap?.[normalized] ?? null;
                if (mapName) {
                    const mapScripts = loadMapScripts(mapName);
                    const mapScript = mapScripts?.[normalized];
                    if (Array.isArray(mapScript)) {
                        return mapScript as ScriptData;
                    }
                }
                return direct;
            }
            if (parentScript) {
                const mapName = storyLabelToMap?.[parentNormalized];
                if (mapName) {
                    const mapScripts = storyEventsCache?.[mapName];
                    if (mapScripts && typeof mapScripts === "object") {
                        const scoped = (mapScripts as Record<string, unknown>)[normalized];
                        if (Array.isArray(scoped)) {
                            if (!normalized.startsWith(".")) {
                                scripts[normalized] = scoped as ScriptData;
                            }
                            return scoped as ScriptData;
                        }
                    }
                }
            }
            const mapName = storyLabelToMap?.[normalized];
            if (mapName) {
                const mapScripts = loadMapScripts(mapName);
                const mapScript = mapScripts?.[normalized];
                if (Array.isArray(mapScript)) {
                    return mapScript as ScriptData;
                }
            }
            return null;
        };
        let result = lookup();
        if (!result && !phoneScriptsLoaded) {
            ensurePhoneScriptsLoaded();
            result = lookup();
        }
        if (!result && storyScriptIndex) {
            const normalized = normalizeLabel(scriptName);
            if (!storyScriptIndex[normalized]) {
                resetStoryEventsCache();
                return lookup();
            }
        }
        if (result) {
            const normalized = normalizeScriptName(scriptName);
            if (normalized) {
                this._register_trainer_event_flag(normalized, result);
                this._register_hidden_item_event_flag(normalized, result);
                this._register_bg_event_script_flag(normalized, result);
            }
        }
        return result;
    }

    public get_script_successor(
        scriptName: string,
        parentScript?: string | null,
    ): [string | null, string] | null {
        if (!scriptName) {
            return null;
        }
        const lookup = (): [string | null, string] | null => {
            ensureStoryEventsLoaded();
            const normalized = normalizeLabel(scriptName);
            const parentNormalized = parentScript ? normalizeLabel(parentScript) : null;
            const successors = storyScriptSuccessors ?? {};
            const direct = successors[successorKey(parentNormalized, normalized)];
            if (direct) {
                return direct;
            }
            const fallback = successors[successorKey(null, normalized)];
            if (fallback) {
                return fallback;
            }
            const mapName = storyLabelToMap?.[normalized]
              ?? (parentNormalized ? storyLabelToMap?.[parentNormalized] : null)
              ?? null;
            if (!mapName) {
                return null;
            }
            const cached = mapScriptSuccessorsCache.get(mapName);
            const mapSuccessors = cached ?? buildMapScriptSuccessors(loadMapScripts(mapName));
            if (!cached) {
                mapScriptSuccessorsCache.set(mapName, mapSuccessors);
            }
            const mapDirect = mapSuccessors[successorKey(parentNormalized, normalized)];
            if (mapDirect) {
                return mapDirect;
            }
            return mapSuccessors[successorKey(null, normalized)] ?? null;
        };
        let result = lookup();
        if (!result && !phoneScriptsLoaded) {
            ensurePhoneScriptsLoaded();
            result = lookup();
        }
        return result;
    }

    public getScript(scriptName: string, parentScript?: string): ScriptData | null {
        return this.get_script(scriptName, parentScript);
    }

    public getScriptByLabel(scriptName: string, parentScript?: string): ScriptData | null {
        return this.get_script(scriptName, parentScript);
    }

    public get_script_event_flags(scriptName: string): string[] {
        if (!scriptName) {
            return [];
        }
        const normalized = normalizeScriptName(scriptName);
        if (!normalized) {
            return [];
        }
        const cached = this._script_event_flags.get(normalized);
        if (cached) {
            return Array.from(cached);
        }
        const flags = this._collect_event_flag_names(normalized, new Set());
        this._script_event_flags.set(normalized, flags);
        return Array.from(flags);
    }

    public get_text(label: string): string | null {
        if (!label) {
            return null;
        }
        const lookup = (): string | null => {
            ensureStoryEventsLoaded();
            const normalized = normalizeLabel(label);
            const fromStory = storyTextIndex?.[normalized];
            if (fromStory) {
                return fromStory;
            }
            let asm = asmTextLoader.get(normalized);
            if (!asm && !normalized.startsWith("_")) {
                // ASM text_far pointers (e.g., CaughtAskNicknameText -> _CaughtAskNicknameText).
                asm = asmTextLoader.get(`_${normalized}`);
            }
            if (asm) {
                return asm;
            }
            ensurePcStringsLoaded();
            return storyTextIndex?.[normalized] ?? null;
        };
        let result = lookup();
        if (!result && !phoneScriptsLoaded) {
            ensurePhoneScriptsLoaded();
            result = lookup();
        }
        if (!result && storyTextIndex) {
            resetStoryEventsCache();
            return lookup();
        }
        return result;
    }

    public getText(label: string): string | null {
        return this.get_text(label);
    }

    public getTextByLabel(label: string): string | null {
        return this.get_text(label);
    }

    public reload_story_events(): void {
        resetStoryEventsCache();
        this.hidden_item_event_flags = {};
        this.trainer_event_flags = {};
        this._bg_event_script_flags.clear();
        this._script_event_flags.clear();
        ensureStoryEventsLoaded();
    }

    public reloadStoryEvents(): void {
        this.reload_story_events();
    }

    public load_trainer_data(): void {
        if (this._loaded_categories.has("trainers")) {
            return;
        }
        const raw = loadMergedTrainersSync();
        if (!Array.isArray(raw)) {
            throw new Error("ASM-backed trainer data is required; merged trainer data must be an array.");
        }
        const trainers: Trainer[] = [];
        for (const entry of raw) {
            let trainer: Trainer;
            try {
                trainer = TrainerSchema.parse(entry);
            } catch (error) {
                throw new Error("ASM-backed trainer data is required; merged trainer data contains an invalid trainer entry.", { cause: error });
            }
            trainers.push(trainer);
        }
        this.trainer_data.clear();
        this.trainer_data_by_name.clear();
        const classBaseRewards = getTrainerClassBaseRewards();
        const hasClassRewards = Object.keys(classBaseRewards).length > 0;
        const applyBaseReward = (trainer: Trainer): void => {
            const trainerClass = String(trainer.trainer_class ?? "").trim();
            if (!trainerClass) {
                return;
            }
            if (!hasClassRewards) {
                return;
            }
            if (!(trainerClass in classBaseRewards)) {
                throw new Error(`Missing trainer base reward for class ${trainerClass}.`);
            }
            trainer.base_reward = classBaseRewards[trainerClass];
        };
        for (const trainer of trainers) {
            applyBaseReward(trainer);
            for (const alias of trainerAliases(trainer)) {
                if (!this.trainer_data.has(alias)) {
                    this.trainer_data.set(alias, trainer);
                }
            }
            if (!this.trainer_data_by_name.has(trainer.name)) {
                this.trainer_data_by_name.set(trainer.name, trainer);
            }
        }
        this._loaded_categories.add("trainers");
    }

    public get_trainer(name: string): Trainer | undefined {
        if (!this._loaded_categories.has("trainers")) {
            this.load_trainer_data();
        }
        return this.trainer_data.get(name) ?? this.trainer_data_by_name.get(name);
    }

    public get_trainer_base_reward(name: string): number {
        const trainer = this.get_trainer(name);
        return trainer?.base_reward ?? 0;
    }

    public getTrainerBaseReward(name: string): number {
        return this.get_trainer_base_reward(name);
    }

    private _extract_hidden_item_event_flag(scriptData: ScriptData | null): string | null {
        if (!scriptData) {
            return null;
        }
        // ASM mapping: pokecrystal_disassembly/engine/events/hidden_item.asm (HiddenItem script).
        for (const entry of scriptData) {
            if (!isScriptEntry(entry)) {
                continue;
            }
            const command = getEntryCommand(entry, { lower: true });
            if (command !== "hiddenitem") {
                continue;
            }
            const args = parseArgsList(entry.args);
            const eventFlag = String(args[1] ?? "").trim();
            return eventFlag || null;
        }
        return null;
    }

    private _extract_bg_event_script_flag(scriptData: ScriptData | null): string | null {
        if (!scriptData || !scriptData.length) {
            return null;
        }
        const firstEntry = scriptData[0];
        if (!isScriptEntry(firstEntry)) {
            return null;
        }
        const command = getEntryCommand(firstEntry, { lower: true });
        const args = parseArgsList(firstEntry.args);
        if (command === "conditional_event" || command === "checkevent") {
            const eventFlag = String(args[0] ?? "").trim();
            return eventFlag || null;
        }
        if (command === "hiddenitem") {
            const eventFlag = String(args[1] ?? "").trim();
            return eventFlag || null;
        }
        return null;
    }

    private _register_hidden_item_event_flag(scriptName: string, scriptData: ScriptData | null): void {
        const normalized = normalizeScriptName(scriptName);
        if (!normalized || !scriptData) {
            return;
        }
        const lookupKey = normalized.toUpperCase();
        if (lookupKey in this.hidden_item_event_flags) {
            return;
        }
        const flag = this._extract_hidden_item_event_flag(scriptData);
        if (flag) {
            this.hidden_item_event_flags[lookupKey] = flag;
        }
    }

    private _register_bg_event_script_flag(scriptName: string, scriptData: ScriptData | null): void {
        const normalized = normalizeScriptName(scriptName);
        if (!normalized || !scriptData) {
            return;
        }
        const lookupKey = normalized.toUpperCase();
        if (this._bg_event_script_flags.has(lookupKey)) {
            return;
        }
        const flag = this._extract_bg_event_script_flag(scriptData);
        this._bg_event_script_flags.set(lookupKey, flag ?? null);
    }

    private _register_trainer_event_flag(scriptName: string, scriptData: ScriptData | null): void {
        const normalized = normalizeScriptName(scriptName);
        if (!normalized || !scriptData) {
            return;
        }
        if (this.trainer_event_flags[normalized]) {
            return;
        }
        // ASM mapping: pokecrystal_disassembly/home/trainers.asm (CheckTrainerBattle event flag).
        for (const entry of scriptData) {
            if (!isScriptEntry(entry)) {
                continue;
            }
            const command = getEntryCommand(entry, { lower: true });
            if (command !== "trainer") {
                continue;
            }
            const args = parseArgsList(entry.args);
            const flag = String(args[2] ?? "").trim();
            if (!flag || flag === "0" || flag === "-1") {
                return;
            }
            this.trainer_event_flags[normalized] = flag;
            return;
        }
    }

    private _collect_event_flag_names(
        scriptName: string,
        visited: Set<string>,
    ): Set<string> {
        const normalized = normalizeScriptName(scriptName);
        if (!normalized || visited.has(normalized)) {
            return new Set();
        }
        visited.add(normalized);
        const scriptData = this.get_script(normalized);
        if (!scriptData) {
            return new Set();
        }
        const flags = new Set<string>();
        for (const entry of scriptData) {
            if (!isScriptEntry(entry)) {
                continue;
            }
            const command = getEntryCommand(entry, { lower: true });
            const args = parseArgsList(entry.args);
            if (!args.length) {
                continue;
            }
            if (command === "setevent" || command === "setflag") {
                const flag = String(args[0] ?? "").trim();
                if (flag) {
                    flags.add(flag);
                }
                continue;
            }
            if (command === "conditional_event" || command === "checkevent") {
                const flag = String(args[0] ?? "").trim();
                if (flag) {
                    flags.add(flag);
                }
                continue;
            }
            if (command === "sjump") {
                const target = String(args[0] ?? "").trim();
                if (target) {
                    const nested = this._collect_event_flag_names(target, visited);
                    nested.forEach((flag) => flags.add(flag));
                }
            }
        }
        const successor = this.get_script_successor(normalized, null);
        const successorName = successor ? successor[1] : null;
        if (successorName) {
            const nested = this._collect_event_flag_names(successorName, visited);
            nested.forEach((flag) => flags.add(flag));
        }
        return flags;
    }
}

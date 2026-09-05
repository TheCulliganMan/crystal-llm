import fs from "fs";
import path from "path";
import type { Move } from "@pokecrystal/core/core/models/move";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import {
  parseAsmNumber,
  splitAsmArgs,
  stripAsmComment,
  writeJsonToTargets,
} from "./asm-utils";

type FleeMonsData = {
  buckets: Record<string, string[]>;
};

export type EncounterMusicModifiers = {
  modifiers: Record<
    string,
    {
      numerator: number;
      denominator: number;
    }
  >;
};

export type RoamingMapLocation = {
  mapGroup: number;
  mapNumber: number;
};

export type RoamingPokemonInitWrite = RoamingMapLocation & {
  slot: number;
  species: string;
  level: number;
  hp: number;
};

export type RoamingPokemonRoute = RoamingMapLocation & {
  connections: RoamingMapLocation[];
};

export type RoamingPokemonCatalog = {
  slotCount: 3;
  inactiveMap: RoamingMapLocation;
  initWrites: RoamingPokemonInitWrite[];
  routes: RoamingPokemonRoute[];
  jumpMask: 15;
};

export type BuenaPrizeDefinitions = Record<string, number>;

export type BuenaPasswordCategoryDefinition = {
  categoryType: string;
  points: number;
  options: string[];
};
export type BuenaPasswordCategories = {
  order: string[];
  categories: Record<string, BuenaPasswordCategoryDefinition>;
};

export type KurtApricornRecipes = Record<string, string>;

export type ShuckieGiftDefinition = {
  species: string;
  level: number;
  heldItem: string;
  nickname: string;
  originalTrainerName: string;
  originalTrainerId: number;
  gotTodayEngineFlag: string;
};

export type DratiniMoveSets = Record<string, string[]>;

export type BugContestEncounter = {
  weight: number;
  species: string;
  minLevel: number;
  maxLevel: number;
};

export type BugContestConfig = {
  parkBalls: number;
  timerMinutes: number;
  timerSeconds: number;
  selectedContestantCount: number;
  contestantFlags: string[];
  encounters: BugContestEncounter[];
};

export type BattleTowerRules = {
  bannedSpecies: Record<string, Record<string, never>>;
  requiredPartyCount: number;
  challengeStreakLength: number;
  rewardCandidates: string[];
  excludedRewardItems: string[];
  rewardQuantity: number;
  rewardFailureSentinel: string;
  rewardItemValues: Record<string, number>;
  minimumLevelGroup: number;
  maximumLevelGroup: number;
  levelGroupSize: number;
  partyCountFailureText: string;
  duplicateSpeciesFailureText: string;
  duplicateHeldItemFailureText: string;
  eggFailureText: string;
  trainers: BattleTowerTrainerDefinition[];
  monGroups: BattleTowerMonDefinition[][];
};

export type BattleTowerTrainerDefinition = {
  index: number;
  trainerClass: string;
  name: string;
  spriteConstant: string;
  female: boolean;
};

export type BattleTowerMonDefinition = {
  species: string;
  item: string | null;
  moves: string[];
  originalTrainerId: number;
  experience: number;
  statExp: number[];
  dvs: number[];
  pp: number[];
  happiness: number;
  pokerus: number[];
  level: number;
  status: number[];
  stats: number[];
  nickname: string;
};

export type OakRatingEntry = {
  caughtCountLimit: number;
  fanfare: string;
  textLabel: string;
};

export type OddEggDefinition = {
  species: string;
  moves: string[];
  originalTrainerId: number;
  dvs: [number, number, number, number];
  probability: number;
  level: number;
  experience: number;
  hatchCycles: number;
  nickname: string;
  originalTrainerName: string;
};

export type MagikarpLengthEntry = {
  threshold: number;
  divisor: number;
};

export type HappinessData = {
  changes: Record<string, HappinessChangeEntry>;
  services: Record<string, HappinessServiceOutcome[]>;
};

export type HappinessChangeEntry = {
  code: string;
  low: number;
  mid: number;
  high: number;
};

export type HappinessServiceOutcome = {
  rollWeight: number;
  scriptValue: number;
  changeCode: number;
};

export type EncounterSlotTables = {
  tables: EncounterSlotTableMap;
};

export type EncounterSlotTableMap = {
  grass: EncounterSlotChance[];
  water: EncounterSlotChance[];
};

export type EncounterSlotChance = {
  threshold: number;
  slot: number;
};

export type BattleStatMultiplierTables = {
  stat: BattleStatMultiplier[];
  accuracy: BattleStatMultiplier[];
};

export type BattleStatMultiplier = {
  numerator: number;
  denominator: number;
};

export type TypeMultiplier = BattleStatMultiplier;

export type CaptureWobbleProbability = {
  catch_rate: number;
  chance: number;
};

export type WeatherModifiers = {
  type_modifiers: Record<string, Record<string, TypeMultiplier>>;
  move_effect_modifiers: Record<string, Record<string, TypeMultiplier>>;
};

export type TypeEffectivenessTable = {
  matchups: TypeEffectivenessEntry[];
  foresight_matchups: TypeEffectivenessEntry[];
};

export type TypeEffectivenessEntry = {
  attacker: string;
  defender: string;
  multiplier: BattleStatMultiplier;
};

export type TypeCategories = {
  physical: string[];
  special: string[];
};

const typeCategoriesFromAsm = (): TypeCategories => {
  const physical: string[] = [];
  const special: string[] = [];
  let section: "physical" | "special" | null = null;
  for (const raw of readAsmLines(
    path.join("constants", "type_constants.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    if (line === "DEF PHYSICAL EQU const_value") {
      section = "physical";
      continue;
    }
    if (line === "DEF UNUSED_TYPES EQU const_value") {
      section = null;
      continue;
    }
    if (line === "DEF SPECIAL EQU const_value") {
      section = "special";
      continue;
    }
    if (line === "DEF TYPES_END EQU const_value") {
      section = null;
      continue;
    }
    if (!section) {
      continue;
    }
    const constant = line.match(/^const\s+([A-Z0-9_]+)$/);
    if (constant) {
      if (section === "physical") {
        physical.push(constant[1]);
      } else {
        special.push(constant[1]);
      }
      continue;
    }
    if (line.startsWith("const") || line.startsWith("DEF ")) {
      throw new Error(`Unexpected type category row: ${raw}`);
    }
  }
  if (!physical.length || !special.length) {
    throw new Error("Could not export complete type categories");
  }
  const seenTypes = new Set<string>();
  for (const type of [...physical, ...special]) {
    if (seenTypes.has(type)) {
      throw new Error(`Duplicate type category constant '${type}'.`);
    }
    seenTypes.add(type);
  }
  return { physical, special };
};

export type MovePriorityTable = {
  base_priority: number;
  effect_priorities: Record<string, number>;
  move_priorities: MovePriorityOverride[];
};

export type MoveEffectPriority = {
  move_effect: string;
  priority: number;
};

export type MovePriorityOverride = {
  move: string;
  priority: number;
};

type RuntimeMapMetadataRecord = {
  groupId: number;
  mapId: number;
};

type PokedexEntryData = {
  species: string;
  classification: string;
  heightDigits: number;
  weightDigits: number;
  pages: string[];
};

type FrontpicAnimCommand =
  | { kind: "frame"; frame: number; duration: number }
  | { kind: "setrepeat"; count: number }
  | { kind: "dorepeat"; target: number }
  | { kind: "endanim" };

type FrontpicAnimProgram = {
  commands: FrontpicAnimCommand[];
};

type PhoneContactRecord = {
  contactId: string;
  trainerClass: string | null;
  trainerLabel: string | null;
  lines: string[];
  primaryLabel: string;
  mapConstant: string | null;
  calleeTimeMask: number;
  calleeScript: string | null;
  callerTimeMask: number;
  callerScript: string | null;
};

export type PermanentPhoneNumberDefinition = {
  listIndex: number;
};

export type SpecialPhoneCallDefinition = {
  value: number;
  condition: string;
  contactId: string;
  callerScript: string;
};

const CONTROL_CODE_REPLACEMENTS: Record<string, string> = {
  "<PLAYER>": "PLAYER",
  "<PKMN>": "PKMN",
  "<PC>": "PC",
  "<PARA>": "\n",
  "<NEXT>": "\n",
  "<LINE>": "\n",
  "<CONT>": "",
  "<DONE>": "",
  "<PROMPT>": "",
};

const TIME_MASKS: Record<string, number> = {
  MORN: 0x1,
  DAY: 0x2,
  NITE: 0x4,
  DARKNESS: 0x8,
};

const NON_TRAINER_RE = /^\.(\w+):\s*db\s+"(.+)"/;

const readAsmLines = (relativePath: string): string[] =>
  fs
    .readFileSync(path.join(getDisassemblyRoot(), relativePath), "utf8")
    .split(/\r?\n/);

const parsePokemonConstants = (): string[] => {
  const species: string[] = [];
  const seenSpecies = new Set<string>();
  for (const rawLine of readAsmLines(
    path.join("constants", "pokemon_constants.asm"),
  )) {
    const line = stripAsmComment(rawLine);
    const match = line.match(/^const\s+([A-Z0-9_]+)$/);
    if (!match) {
      continue;
    }
    if (seenSpecies.has(match[1])) {
      throw new Error(`Duplicate Pokemon species constant '${match[1]}'.`);
    }
    seenSpecies.add(match[1]);
    species.push(match[1]);
  }
  return species;
};

const exactSpeciesIdMap = (): Map<string, string> => {
  const speciesByFileStem = new Map<string, string>();
  for (const species of parsePokemonConstants()) {
    const fileStem = species.toLowerCase();
    if (speciesByFileStem.has(fileStem)) {
      throw new Error(
        `Duplicate runtime species file stem '${fileStem}' from pokemon constants.`,
      );
    }
    speciesByFileStem.set(fileStem, species);
  }
  return speciesByFileStem;
};

const exactSpeciesFromFileStem = (
  fileStem: string,
  speciesByFileStem: Map<string, string>,
  sourcePath: string,
): string => {
  const species = speciesByFileStem.get(fileStem);
  if (!species) {
    throw new Error(
      `Unknown or case-changed runtime species file stem '${fileStem}' in ${sourcePath}.`,
    );
  }
  return species;
};

const decodePhoneText = (payload: string): string => {
  let result = String(payload ?? "")
    .replace(/<LF>/g, "\n")
    .replace(/@/g, "")
    .replace(/#/g, "POKé");
  for (const [token, replacement] of Object.entries(
    CONTROL_CODE_REPLACEMENTS,
  )) {
    result = result.split(token).join(replacement);
  }
  return result;
};

const nullSentinel = (token: string): string | null => {
  const value = token.trim();
  return value === "" || value === "0" || value === "N_A" ? null : value;
};

export const timeTokenToMask = (token: string): number => {
  const value = token.trim();
  if (!value || value === "0" || value === "NONE") {
    return 0;
  }
  if (value === "ANYTIME") {
    return TIME_MASKS.MORN | TIME_MASKS.DAY | TIME_MASKS.NITE;
  }
  let mask = 0;
  for (const part of value
    .split("|")
    .map((part) => part.trim())
    .filter(Boolean)) {
    const partMask = TIME_MASKS[part];
    if (partMask === undefined) {
      if (/^[+-]?\d+$/.test(part)) {
        mask |= Number.parseInt(part, 10);
        continue;
      }
      throw new Error(`Unknown phone time mask token '${part}' in '${token}'.`);
    }
    mask |= partMask;
  }
  if (mask !== 0) {
    return mask;
  }
  if (/^[+-]?\d+$/.test(value)) {
    return Number.parseInt(value, 10);
  }
  throw new Error(`Unknown phone time mask token '${token}'.`);
};

const contactDisplayName = (contactId: string): string => {
  const parts = contactId.split("_");
  return parts[parts.length - 1] || contactId;
};

const parseDbSymbolList = (content: string, label: string): string[] => {
  const result: string[] = [];
  let inBlock = false;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (!inBlock) {
      if (line === `${label}:`) {
        inBlock = true;
      }
      continue;
    }
    if (!line) {
      continue;
    }
    if (/^[A-Za-z_][A-Za-z0-9_]*:$/.test(line)) {
      break;
    }
    const match = line.match(/^db\s+(.+)$/);
    if (!match) {
      continue;
    }
    const value = match[1].split(",", 1)[0].trim();
    if (value === "-1") {
      break;
    }
    result.push(value);
  }
  return result;
};

const parseRequiredDbSymbolList = (
  content: string,
  label: string,
  sourcePath: string,
): string[] => {
  const values = parseDbSymbolList(content, label);
  if (!values.length) {
    throw new Error(
      `Could not parse required ${label} table from ${sourcePath}.`,
    );
  }
  return values;
};

export const exportFleeMons = (): FleeMonsData => {
  const sourcePath = path.join(
    getDisassemblyRoot(),
    "data",
    "wild",
    "flee_mons.asm",
  );
  const content = fs.readFileSync(sourcePath, "utf8");
  const payload: FleeMonsData = {
    buckets: {
      always: parseRequiredDbSymbolList(content, "AlwaysFleeMons", sourcePath),
      often: parseRequiredDbSymbolList(content, "OftenFleeMons", sourcePath),
      sometimes: parseRequiredDbSymbolList(
        content,
        "SometimesFleeMons",
        sourcePath,
      ),
    },
  };
  writeJsonToTargets("flee_mons.json", payload, { indent: 2 });
  return payload;
};

export const exportMarts = (): Record<string, string[]> => {
  const sourcePath = path.join(
    getDisassemblyRoot(),
    "data",
    "items",
    "marts.asm",
  );
  const content = fs.readFileSync(sourcePath, "utf8");
  const marts: Record<string, string[]> = {};
  let currentMart: string | null = null;
  let expectedCount: number | null = null;

  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    const labelMatch = line.match(/^(Mart[A-Za-z0-9_]+):$/);
    if (labelMatch) {
      if (labelMatch[1] === "Marts") {
        currentMart = null;
        expectedCount = null;
        continue;
      }
      currentMart = martLabelToScriptId(labelMatch[1]);
      if (Object.prototype.hasOwnProperty.call(marts, currentMart)) {
        throw new Error(`Duplicate mart table '${currentMart}'.`);
      }
      marts[currentMart] = [];
      expectedCount = null;
      continue;
    }
    if (!currentMart) {
      continue;
    }
    const dbMatch = line.match(/^db\s+(.+)$/);
    if (!dbMatch) {
      continue;
    }
    const value = dbMatch[1].split(",", 1)[0].trim();
    if (value === "-1") {
      if (
        expectedCount !== null &&
        marts[currentMart].length !== expectedCount
      ) {
        throw new Error(
          `${currentMart} declared ${expectedCount} mart items but exported ${marts[currentMart].length}.`,
        );
      }
      currentMart = null;
      expectedCount = null;
      continue;
    }
    if (expectedCount === null && /^\d+$/.test(value)) {
      expectedCount = Number.parseInt(value, 10);
      continue;
    }
    marts[currentMart].push(value);
  }

  writeJsonToTargets("marts.json", marts, { indent: 2 });
  return marts;
};

const martLabelToScriptId = (label: string): string => {
  if (!label.startsWith("Mart") || label === "Mart") {
    throw new Error(`Invalid mart label ${label}.`);
  }
  return `MART_${label
    .slice("Mart".length)
    .replace(/([a-z])([A-Z0-9])/g, "$1_$2")
    .replace(/([0-9][A-Z])([0-9])/g, "$1_$2")
    .toUpperCase()}`;
};

export const exportPcStrings = (): Record<string, string> => {
  const sourcePath = path.join(
    getDisassemblyRoot(),
    "engine",
    "pokemon",
    "bills_pc.asm",
  );
  const content = fs.readFileSync(sourcePath, "utf8");
  const strings: Record<string, string> = {};
  for (const match of content.matchAll(
    /^(PCString_[A-Za-z0-9_]+):\s+db\s+"([^"]*)@"/gm,
  )) {
    if (Object.prototype.hasOwnProperty.call(strings, match[1])) {
      throw new Error(`Duplicate PC string '${match[1]}'.`);
    }
    if (!match[2].trim()) {
      throw new Error(`PC string '${match[1]}' must be nonempty.`);
    }
    strings[match[1]] = match[2];
  }
  writeJsonToTargets("pc_strings.json", strings, { indent: 2 });
  return strings;
};

export const exportMenuIcons = (): Record<string, string> => {
  const sourcePath = path.join(
    getDisassemblyRoot(),
    "data",
    "pokemon",
    "menu_icons.asm",
  );
  const content = fs.readFileSync(sourcePath, "utf8");
  const icons: Record<string, string> = {};
  for (const rawLine of content.split(/\r?\n/)) {
    const match = rawLine
      .trim()
      .match(/^db\s+(ICON_[A-Z0-9_]+)\s*;\s*([A-Z0-9_]+)$/);
    if (!match) {
      continue;
    }
    if (Object.prototype.hasOwnProperty.call(icons, match[2])) {
      throw new Error(`Duplicate menu icon species '${match[2]}'.`);
    }
    icons[match[2]] = match[1];
  }
  if (Object.prototype.hasOwnProperty.call(icons, "EGG")) {
    throw new Error("Menu icon table must not declare built-in EGG icon.");
  }
  icons.EGG = "ICON_EGG";
  writeJsonToTargets("menu_icons.json", icons, { indent: 2 });
  return icons;
};

const parseDexEntryFile = (
  filePath: string,
  speciesByFileStem: Map<string, string>,
): PokedexEntryData => {
  const fileStem = path.basename(filePath, ".asm");
  const species = exactSpeciesFromFileStem(
    fileStem,
    speciesByFileStem,
    filePath,
  );
  const content = fs.readFileSync(filePath, "utf8");
  const classificationMatch = content.match(/db\s+"([^"]*)@"/);
  const sizeMatch = content.match(/dw\s+(\d+),\s*(\d+)\s*;\s*height,\s*weight/);
  if (!classificationMatch || !sizeMatch) {
    throw new Error(
      `Could not parse complete Pokedex entry for ${species} in ${filePath}.`,
    );
  }
  if (!classificationMatch[1].trim()) {
    throw new Error(
      `Pokedex entry for ${species} in ${filePath} must declare a nonempty classification.`,
    );
  }
  const heightDigits = Number.parseInt(sizeMatch[1], 10);
  const weightDigits = Number.parseInt(sizeMatch[2], 10);
  if (heightDigits > 99999 || weightDigits > 99999) {
    throw new Error(
      `Pokedex entry for ${species} in ${filePath} has size digits outside supported display range.`,
    );
  }
  const pages: string[] = [];
  let currentPage: string[] = [];
  let inEntryText = false;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (!line) {
      continue;
    }
    if (!inEntryText) {
      if (line.match(/^dw\s+\d+,\s*\d+/)) {
        inEntryText = true;
      }
      continue;
    }
    const textMatch = line.match(/^(db|next|page)\s+"([^"]*)"/);
    if (!textMatch) {
      throw new Error(
        `Malformed Pokedex entry text row for ${species} in ${filePath}: ${rawLine}`,
      );
    }
    const opcode = textMatch[1];
    if (opcode === "page") {
      const pageText = currentPage.join(" @ ").trim();
      if (!pageText) {
        throw new Error(
          `Pokedex entry for ${species} in ${filePath} has an empty text page before page break.`,
        );
      }
      pages.push(pageText);
      currentPage = [];
    }
    currentPage.push(textMatch[2].replace(/@$/, ""));
  }
  if (currentPage.length) {
    pages.push(currentPage.join(" @ ").trim());
  }
  if (!pages.length || pages.some((page) => !page.trim())) {
    throw new Error(
      `Pokedex entry for ${species} in ${filePath} must declare nonempty text pages.`,
    );
  }
  return {
    species,
    classification: classificationMatch[1],
    heightDigits,
    weightDigits,
    pages,
  };
};

export const exportPokedexEntries = (): Record<string, PokedexEntryData> => {
  const dexEntriesDir = path.join(
    getDisassemblyRoot(),
    "data",
    "pokemon",
    "dex_entries",
  );
  const speciesByFileStem = exactSpeciesIdMap();
  const entries: Record<string, PokedexEntryData> = {};
  for (const entry of fs
    .readdirSync(dexEntriesDir)
    .filter((entry) => entry.endsWith(".asm"))
    .sort()) {
    const parsed = parseDexEntryFile(
      path.join(dexEntriesDir, entry),
      speciesByFileStem,
    );
    if (Object.prototype.hasOwnProperty.call(entries, parsed.species)) {
      throw new Error(`duplicate Pokedex entry for species ${parsed.species}`);
    }
    entries[parsed.species] = parsed;
  }
  writeJsonToTargets("pokedex_entries.json", entries, { indent: 2 });
  return entries;
};

const parsePhoneConstants = (): Array<string | null> => {
  const entries: Array<string | null> = [];
  const seenContactIds = new Set<string>();
  for (const raw of readAsmLines(
    path.join("constants", "phone_constants.asm"),
  )) {
    const line = raw.trim();
    if (!line || line.startsWith(";")) {
      continue;
    }
    if (line.startsWith("const_skip")) {
      entries.push(null);
      continue;
    }
    if (line.startsWith("const ")) {
      const contactId = line.split(/\s+/)[1] ?? null;
      if (contactId) {
        if (seenContactIds.has(contactId)) {
          throw new Error(
            `Phone contact constant '${contactId}' is declared more than once.`,
          );
        }
        seenContactIds.add(contactId);
      }
      entries.push(contactId);
      continue;
    }
    if (line.startsWith("DEF NUM_PHONE_CONTACTS")) {
      break;
    }
  }
  return entries;
};

const parsePhoneContactRows = (): Array<{
  trainerClass: string | null;
  trainerLabel: string | null;
  mapConstant: string | null;
  calleeTimeMask: number;
  calleeScript: string | null;
  callerTimeMask: number;
  callerScript: string | null;
}> => {
  const entries = [];
  for (const raw of readAsmLines(
    path.join("data", "phone", "phone_contacts.asm"),
  )) {
    const line = raw.trim();
    if (!line || line.startsWith(";") || !line.startsWith("phone ")) {
      continue;
    }
    const tokens = line
      .slice("phone ".length)
      .split(";")[0]
      .split(",")
      .map((token) => token.trim());
    if (tokens.length !== 7) {
      throw new Error(`Malformed phone contact row: ${line}`);
    }
    entries.push({
      trainerClass: nullSentinel(tokens[0]),
      trainerLabel: nullSentinel(tokens[1]),
      mapConstant: nullSentinel(tokens[2]),
      calleeTimeMask: timeTokenToMask(tokens[3]),
      calleeScript: nullSentinel(tokens[4]),
      callerTimeMask: timeTokenToMask(tokens[5]),
      callerScript: nullSentinel(tokens[6]),
    });
  }
  return entries;
};

const parseNonTrainerNames = (): Record<string, string[]> => {
  const entries: Record<string, string[]> = {};
  for (const raw of readAsmLines(
    path.join("data", "phone", "non_trainer_names.asm"),
  )) {
    const line = raw.trim();
    if (!line || line.startsWith(";")) {
      continue;
    }
    const match = NON_TRAINER_RE.exec(line);
    if (!match) {
      continue;
    }
    const [, label, text] = match;
    const contactLabel = `PHONECONTACT_${label.toUpperCase()}`;
    if (Object.prototype.hasOwnProperty.call(entries, contactLabel)) {
      throw new Error(`Duplicate non-trainer phone label '${contactLabel}'.`);
    }
    const segments = decodePhoneText(text)
      .split("\n")
      .map((segment) => segment.trimEnd());
    entries[contactLabel] = [segments[0] ?? "", ...segments.slice(1)];
  }
  return entries;
};

export const exportTrainerClassNames = (): Record<string, string> => {
  let classIds: string[] = [];
  for (const raw of readAsmLines(
    path.join("constants", "trainer_constants.asm"),
  )) {
    const line = raw.trim();
    if (line.startsWith("trainerclass ")) {
      const classId = line.split(/\s+/)[1];
      if (classIds.includes(classId)) {
        throw new Error(`Duplicate trainer class id '${classId}'.`);
      }
      classIds.push(classId);
    }
  }
  if (classIds[0] !== "TRAINER_NONE") {
    throw new Error(
      `Trainer class constant table must begin with exact TRAINER_NONE, found '${classIds[0] ?? "<missing>"}'.`,
    );
  }
  classIds = classIds.slice(1);
  const classNames: string[] = [];
  for (const raw of readAsmLines(
    path.join("data", "trainers", "class_names.asm"),
  )) {
    const line = raw.trim();
    if (!line.startsWith("li ")) {
      continue;
    }
    const [, remainder] = line.split('"', 2);
    if (!remainder) {
      continue;
    }
    classNames.push(decodePhoneText(remainder.split('"')[0] ?? ""));
  }
  if (classIds.length !== classNames.length) {
    throw new Error(
      `Trainer class id count ${classIds.length} does not match class name count ${classNames.length}.`,
    );
  }
  const mapping: Record<string, string> = {};
  for (let index = 0; index < classIds.length; index += 1) {
    mapping[classIds[index]] = classNames[index];
  }
  writeJsonToTargets("trainer_class_names.json", mapping, { indent: 2 });
  return mapping;
};

export const exportPhoneContacts = (
  classNames = exportTrainerClassNames(),
): Record<string, PhoneContactRecord> => {
  const phoneConstants = parsePhoneConstants();
  const phoneRows = parsePhoneContactRows();
  const nonTrainerLines = parseNonTrainerNames();
  if (phoneConstants.length !== phoneRows.length) {
    throw new Error(
      `Phone constant count ${phoneConstants.length} does not match contact table ${phoneRows.length}`,
    );
  }
  const records: Record<string, PhoneContactRecord> = {};
  for (let index = 0; index < phoneConstants.length; index += 1) {
    const contactId = phoneConstants[index];
    const row = phoneRows[index];
    if (!contactId || contactId === "PHONE_00") {
      continue;
    }
    let lines: string[];
    if (row.trainerClass === "TRAINER_NONE") {
      lines = nonTrainerLines[row.trainerLabel ?? ""] ?? [];
    } else {
      if (!row.trainerClass) {
        throw new Error(
          `Phone contact ${contactId} is missing a trainer class.`,
        );
      }
      const className = classNames[row.trainerClass];
      if (!className) {
        throw new Error(
          `Phone contact ${contactId} references trainer class '${row.trainerClass}' without an exported class name.`,
        );
      }
      lines = [`${contactDisplayName(contactId)}:`, `   ${className}`];
    }
    if (!lines.length) {
      throw new Error(`Phone contact ${contactId} has no display lines`);
    }
    const primaryLabel = String(lines[0] ?? "")
      .replace(/:$/, "")
      .trim();
    if (!primaryLabel) {
      throw new Error(
        `Phone contact ${contactId} has an empty primary display label.`,
      );
    }
    records[contactId] = {
      contactId,
      trainerClass: row.trainerClass,
      trainerLabel: row.trainerLabel,
      lines,
      primaryLabel,
      mapConstant: row.mapConstant,
      calleeTimeMask: row.calleeTimeMask,
      calleeScript: row.calleeScript,
      callerTimeMask: row.callerTimeMask,
      callerScript: row.callerScript,
    };
  }
  writeJsonToTargets("phone_contacts.json", records, { indent: 2 });
  return records;
};

export const exportPermanentPhoneNumbers = (): Record<
  string,
  PermanentPhoneNumberDefinition
> => {
  const phoneConstants = parsePhoneConstants();
  const phoneRows = parsePhoneContactRows();
  const declaredContactIds = new Set(
    phoneConstants.filter((contactId): contactId is string =>
      Boolean(contactId),
    ),
  );
  const contactIdByTrainerLabel = new Map<string, string>();
  for (
    let index = 0;
    index < Math.min(phoneConstants.length, phoneRows.length);
    index += 1
  ) {
    const contactId = phoneConstants[index];
    const trainerLabel = phoneRows[index].trainerLabel;
    if (contactId && trainerLabel) {
      contactIdByTrainerLabel.set(trainerLabel, contactId);
    }
  }
  const numbers: Record<string, PermanentPhoneNumberDefinition> = {};
  for (const raw of readAsmLines(
    path.join("data", "phone", "permanent_numbers.asm"),
  )) {
    const cleaned = stripAsmComment(raw);
    if (!cleaned.startsWith("db ")) {
      continue;
    }
    const token = cleaned.slice("db ".length).split(",", 1)[0]?.trim();
    if (!token || token.startsWith("-1") || token.startsWith("$FF")) {
      break;
    }
    const resolvedContactId =
      contactIdByTrainerLabel.get(token) ??
      (declaredContactIds.has(token) ? token : null);
    if (!resolvedContactId) {
      throw new Error(
        `Permanent phone number '${token}' does not match a declared phone contact id or trainer label.`,
      );
    }
    if (Object.hasOwn(numbers, resolvedContactId)) {
      throw new Error(
        `Permanent phone number '${resolvedContactId}' is exported more than once.`,
      );
    }
    numbers[resolvedContactId] = {
      listIndex: Object.keys(numbers).length,
    };
  }
  writeJsonToTargets("permanent_phone_numbers.json", numbers, { indent: 2 });
  return numbers;
};

export const exportSpecialPhoneCalls = (): Record<
  string,
  SpecialPhoneCallDefinition
> => {
  const callIds: Array<{ id: string; value: number }> = [];
  let inSpecialCalls = false;
  let specialCallValue = 0;
  for (const raw of readAsmLines(
    path.join("constants", "phone_constants.asm"),
  )) {
    const line = raw.trim();
    if (line.startsWith("; SpecialPhoneCallList")) {
      inSpecialCalls = true;
      continue;
    }
    if (!inSpecialCalls || !line || line.startsWith(";")) {
      continue;
    }
    if (line === "const_def") {
      specialCallValue = 0;
      continue;
    }
    if (line.startsWith("const_skip")) {
      specialCallValue += 1;
      continue;
    }
    if (line.startsWith("const ")) {
      const callId = line.split(/\s+/)[1];
      if (callIds.some((call) => call.id === callId)) {
        throw new Error(
          `Special phone call '${callId}' is exported more than once.`,
        );
      }
      if (callId !== "SPECIALCALL_NONE") {
        callIds.push({ id: callId, value: specialCallValue });
      }
      specialCallValue += 1;
      continue;
    }
    if (line.startsWith("DEF NUM_SPECIALCALLS")) {
      break;
    }
  }
  if (!callIds.length) {
    throw new Error(
      "No special phone calls were exported from constants/phone_constants.asm",
    );
  }

  const phoneConstants = parsePhoneConstants();
  const phoneRows = parsePhoneContactRows();
  const contactIdByTrainerLabel = new Map<string, string>();
  for (
    let index = 0;
    index < Math.min(phoneConstants.length, phoneRows.length);
    index += 1
  ) {
    const contactId = phoneConstants[index];
    const trainerLabel = phoneRows[index].trainerLabel;
    if (contactId && trainerLabel) {
      contactIdByTrainerLabel.set(trainerLabel, contactId);
    }
  }
  const declaredContactIds = new Set(
    phoneConstants.filter((contactId): contactId is string =>
      Boolean(contactId),
    ),
  );
  const rows: Array<{
    condition: string;
    contactId: string;
    callerScript: string;
  }> = [];
  for (const raw of readAsmLines(path.join("data", "phone", "special_calls.asm"))) {
    const line = stripAsmComment(raw);
    if (!line.startsWith("specialcall ")) {
      continue;
    }
    const tokens = line
      .slice("specialcall ".length)
      .split(",")
      .map((token) => token.trim());
    if (tokens.length !== 3 || tokens.some((token) => !token)) {
      throw new Error(`Malformed special phone call row: ${line}`);
    }
    const [condition, contactToken, callerScript] = tokens;
    if (
      condition !== "SpecialCallOnlyWhenOutside" &&
      condition !== "SpecialCallWhereverYouAre"
    ) {
      throw new Error(
        `Special phone call row has unsupported condition '${condition}'.`,
      );
    }
    const contactId =
      contactIdByTrainerLabel.get(contactToken) ??
      (declaredContactIds.has(contactToken) ? contactToken : null);
    if (!contactId) {
      throw new Error(
        `Special phone call contact '${contactToken}' does not match a declared phone contact id or trainer label.`,
      );
    }
    rows.push({ condition, contactId, callerScript });
  }
  if (rows.length !== callIds.length) {
    throw new Error(
      `Special phone call constant count ${callIds.length} does not match table ${rows.length}.`,
    );
  }
  const calls: Record<string, SpecialPhoneCallDefinition> = {};
  for (let index = 0; index < callIds.length; index += 1) {
    calls[callIds[index].id] = {
      value: callIds[index].value,
      ...rows[index],
    };
  }
  writeJsonToTargets("special_phone_calls.json", calls, { indent: 2 });
  return calls;
};

export type NpcTradeDefinition = {
  dialogSet: string;
  requestedSpecies: string;
  offeredSpecies: string;
  nickname: string;
  dvs: [number, number];
  heldItem: string;
  originalTrainerId: number;
  originalTrainerName: string;
  genderRequirement: string;
};

export const exportNpcTrades = (): Record<string, NpcTradeDefinition> => {
  const trades: Record<string, NpcTradeDefinition> = {};
  const constants = readAsmLines(
    path.join("constants", "npc_trade_constants.asm"),
  );
  const ids = new Set(
    constants
      .map(stripAsmComment)
      .filter((line) => line.startsWith("const NPC_TRADE_"))
      .map((line) => line.split(/\s+/)[1]),
  );
  for (const raw of readAsmLines(path.join("data", "events", "npc_trades.asm"))) {
    const line = stripAsmComment(raw);
    if (!line.startsWith("npctrade ")) {
      continue;
    }
    const args = splitAsmArgs(line.slice("npctrade ".length));
    if (args.length !== 10) {
      throw new Error(`NPC trade row has ${args.length} fields, expected 10: ${line}`);
    }
    const index = Object.keys(trades).length;
    const tradeId = [...ids][index];
    if (!tradeId) {
      throw new Error(`NPC trade row ${index} has no canonical constant id.`);
    }
    if (Object.hasOwn(trades, tradeId)) {
      throw new Error(`NPC trade '${tradeId}' is exported more than once.`);
    }
    const unquote = (value: string) => value.replace(/^"|"$/g, "");
    trades[tradeId] = {
      dialogSet: args[0],
      requestedSpecies: args[1],
      offeredSpecies: args[2],
      nickname: unquote(args[3]),
      dvs: [parseAsmNumber(args[4]), parseAsmNumber(args[5])],
      heldItem: args[6],
      originalTrainerId: parseAsmNumber(args[7]),
      originalTrainerName: unquote(args[8]),
      genderRequirement: args[9],
    };
  }
  if (Object.keys(trades).length !== ids.size) {
    throw new Error(`Exported ${Object.keys(trades).length} NPC trades, expected ${ids.size}.`);
  }
  if (!Object.keys(trades).length) {
    throw new Error(
      "No NPC trade ids were exported from constants/npc_trade_constants.asm",
    );
  }
  writeJsonToTargets("npc_trades.json", trades, { indent: 2 });
  return trades;
};

export const exportSpecialRoutines = (): Record<
  string,
  Record<string, never>
> => {
  const routines: Record<string, Record<string, never>> = {};
  for (const raw of readAsmLines(
    path.join("data", "events", "special_pointers.asm"),
  )) {
    const line = stripAsmComment(raw);
    if (!line.startsWith("add_special ")) {
      continue;
    }
    const routine = line.slice("add_special ".length).trim();
    if (!routine) {
      throw new Error(`Malformed special pointer row: ${raw}`);
    }
    if (Object.hasOwn(routines, routine)) {
      throw new Error(
        `Special routine '${routine}' is exported more than once.`,
      );
    }
    routines[routine] = {};
  }
  if (!Object.keys(routines).length) {
    throw new Error(
      "No special routines were exported from data/events/special_pointers.asm",
    );
  }
  writeJsonToTargets("special_routines.json", routines, { indent: 2 });
  return routines;
};

const ROAMING_SLOT_COUNT = 3;
const CANONICAL_ROAMING_INIT_WRITES = [
  { slot: 0, field: "species", value: "RAIKOU" },
  { slot: 1, field: "species", value: "ENTEI" },
  { slot: 0, field: "level", value: 40 },
  { slot: 1, field: "level", value: 40 },
  { slot: 0, field: "mapGroup", value: "ROUTE_42" },
  { slot: 0, field: "mapNumber", value: "ROUTE_42" },
  { slot: 1, field: "mapGroup", value: "ROUTE_37" },
  { slot: 1, field: "mapNumber", value: "ROUTE_37" },
  { slot: 0, field: "hp", value: 0 },
  { slot: 1, field: "hp", value: 0 },
] as const;

const CANONICAL_ROAMING_ROUTE_ROWS = [
  ["ROUTE_29", "ROUTE_30", "ROUTE_46"],
  ["ROUTE_30", "ROUTE_29", "ROUTE_31"],
  ["ROUTE_31", "ROUTE_30", "ROUTE_32", "ROUTE_36"],
  ["ROUTE_32", "ROUTE_36", "ROUTE_31", "ROUTE_33"],
  ["ROUTE_33", "ROUTE_32", "ROUTE_34"],
  ["ROUTE_34", "ROUTE_33", "ROUTE_35"],
  ["ROUTE_35", "ROUTE_34", "ROUTE_36"],
  ["ROUTE_36", "ROUTE_35", "ROUTE_31", "ROUTE_32", "ROUTE_37"],
  ["ROUTE_37", "ROUTE_36", "ROUTE_38", "ROUTE_42"],
  ["ROUTE_38", "ROUTE_37", "ROUTE_39", "ROUTE_42"],
  ["ROUTE_39", "ROUTE_38"],
  ["ROUTE_42", "ROUTE_43", "ROUTE_44", "ROUTE_37", "ROUTE_38"],
  ["ROUTE_43", "ROUTE_42", "ROUTE_44"],
  ["ROUTE_44", "ROUTE_42", "ROUTE_43", "ROUTE_45"],
  ["ROUTE_45", "ROUTE_44", "ROUTE_46"],
  ["ROUTE_46", "ROUTE_45", "ROUTE_29"],
] as const;

type RoamingInitField = "species" | "level" | "mapGroup" | "mapNumber" | "hp";

type ParsedRoamingInitWrite = {
  slot: number;
  field: RoamingInitField;
  value: string | number;
};

type SymbolicRoamingInitWrite = {
  slot: number;
  species: string;
  level: number;
  mapConstant: string;
  hp: number;
};

type SymbolicRoamingRoute = {
  origin: string;
  connections: string[];
};

const parseInitRoamMons = (): SymbolicRoamingInitWrite[] => {
  const lines = readAsmLines(path.join("engine", "overworld", "wildmons.asm"));
  const start = lines.findIndex(
    (line) => stripAsmComment(line) === "InitRoamMons:",
  );
  if (start < 0) {
    throw new Error(
      "Unable to find InitRoamMons in engine/overworld/wildmons.asm",
    );
  }
  const end = lines.findIndex(
    (line, index) =>
      index > start &&
      stripAsmComment(line).startsWith("CheckEncounterRoamMon:"),
  );
  if (end < 0) {
    throw new Error(
      "Unable to find end of InitRoamMons in engine/overworld/wildmons.asm",
    );
  }
  const block = lines
    .slice(start + 1, end)
    .map((line) => stripAsmComment(line).trim())
    .filter(Boolean);
  let register:
    | { kind: "symbol"; value: string }
    | { kind: "number"; value: number }
    | { kind: "mapGroup"; value: string }
    | { kind: "mapNumber"; value: string }
    | null = null;
  const writes: ParsedRoamingInitWrite[] = [];
  for (const line of block) {
    let match = line.match(/^ld a, GROUP_([A-Z0-9_]+)$/);
    if (match) {
      register = { kind: "mapGroup", value: match[1] };
      continue;
    }
    match = line.match(/^ld a, MAP_([A-Z0-9_]+)$/);
    if (match) {
      register = { kind: "mapNumber", value: match[1] };
      continue;
    }
    match = line.match(/^ld a, ([0-9]+)$/);
    if (match) {
      register = { kind: "number", value: Number.parseInt(match[1], 10) };
      continue;
    }
    match = line.match(/^ld a, ([A-Z0-9_]+)$/);
    if (match) {
      register = { kind: "symbol", value: match[1] };
      continue;
    }
    if (line === "xor a") {
      register = { kind: "number", value: 0 };
      continue;
    }
    match = line.match(/^ld \[wRoamMon([0-9]+)([A-Za-z0-9_]+)\],\s*a$/);
    if (match) {
      const slot = Number.parseInt(match[1], 10) - 1;
      if (slot < 0 || slot >= ROAMING_SLOT_COUNT) {
        throw new Error(
          `InitRoamMons writes roaming slot ${slot}, outside exact 0..2 range.`,
        );
      }
      const fieldByRamName: Record<string, RoamingInitField | undefined> = {
        Species: "species",
        Level: "level",
        MapGroup: "mapGroup",
        MapNumber: "mapNumber",
        HP: "hp",
      };
      const field = fieldByRamName[match[2]];
      if (!field) {
        throw new Error(
          `InitRoamMons writes unsupported roaming field '${match[2]}'.`,
        );
      }
      const requiredRegisterKind: Record<
        RoamingInitField,
        "symbol" | "number" | "mapGroup" | "mapNumber"
      > = {
        species: "symbol",
        level: "number",
        mapGroup: "mapGroup",
        mapNumber: "mapNumber",
        hp: "number",
      };
      if (!register || register.kind !== requiredRegisterKind[field]) {
        throw new Error(
          `InitRoamMons ${field} store '${line}' has invalid register state.`,
        );
      }
      writes.push({ slot, field, value: register.value });
    }
  }
  if (writes.length !== CANONICAL_ROAMING_INIT_WRITES.length) {
    throw new Error(
      `InitRoamMons has ${writes.length} roaming field writes, expected exactly ${CANONICAL_ROAMING_INIT_WRITES.length}.`,
    );
  }
  for (
    let index = 0;
    index < CANONICAL_ROAMING_INIT_WRITES.length;
    index += 1
  ) {
    const actual = writes[index];
    const expected = CANONICAL_ROAMING_INIT_WRITES[index];
    if (
      actual.slot !== expected.slot ||
      actual.field !== expected.field ||
      actual.value !== expected.value
    ) {
      throw new Error(
        `InitRoamMons field write ${index} is slot ${actual.slot} ${actual.field}=${actual.value}, expected slot ${expected.slot} ${expected.field}=${expected.value}.`,
      );
    }
  }
  const valueFor = (slot: number, field: RoamingInitField): string | number => {
    const write = writes.find(
      (candidate) => candidate.slot === slot && candidate.field === field,
    );
    if (!write) {
      throw new Error(
        `InitRoamMons slot ${slot} is missing exact ${field} write.`,
      );
    }
    return write.value;
  };
  return [0, 1].map((slot) => {
    const mapGroup = String(valueFor(slot, "mapGroup"));
    const mapNumber = String(valueFor(slot, "mapNumber"));
    if (mapGroup !== mapNumber) {
      throw new Error(
        `InitRoamMons slot ${slot} map group '${mapGroup}' does not match map number '${mapNumber}'.`,
      );
    }
    return {
      slot,
      species: String(valueFor(slot, "species")),
      level: Number(valueFor(slot, "level")),
      mapConstant: mapGroup,
      hp: Number(valueFor(slot, "hp")),
    };
  });
};

const parseNumRoamMonMaps = (): number => {
  const values = readAsmLines(
    path.join("constants", "pokemon_data_constants.asm"),
  )
    .map((line) => stripAsmComment(line).trim())
    .map((line) => line.match(/^DEF NUM_ROAMMON_MAPS EQU (.+)$/)?.[1])
    .filter((value): value is string => Boolean(value));
  if (values.length !== 1) {
    throw new Error(
      `Expected exactly one NUM_ROAMMON_MAPS definition, found ${values.length}.`,
    );
  }
  const count = parseAsmNumber(values[0]);
  if (count !== CANONICAL_ROAMING_ROUTE_ROWS.length) {
    throw new Error(
      `NUM_ROAMMON_MAPS is ${count}, expected exact source count ${CANONICAL_ROAMING_ROUTE_ROWS.length}.`,
    );
  }
  if ((count & (count - 1)) !== 0) {
    throw new Error(
      `NUM_ROAMMON_MAPS ${count} cannot define a maskbits jump mask.`,
    );
  }
  return count;
};

const parseInactiveRoamingMap = (): RoamingMapLocation => {
  const values = new Map<string, number>();
  for (const raw of readAsmLines(
    path.join("constants", "map_data_constants.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    const match = line.match(/^DEF (GROUP_N_A|MAP_N_A)\s+EQU\s+(.+)$/);
    if (!match) {
      continue;
    }
    const [, name, valueToken] = match;
    if (values.has(name)) {
      throw new Error(
        `Inactive roaming sentinel '${name}' is defined more than once.`,
      );
    }
    values.set(name, parseAsmNumber(valueToken));
  }
  const exactSentinel = (name: "GROUP_N_A" | "MAP_N_A"): number => {
    const value = values.get(name);
    if (value === undefined) {
      throw new Error(`Inactive roaming sentinel '${name}' is missing.`);
    }
    if (value !== -1) {
      throw new Error(
        `${name} is ${value}, expected exact inactive sentinel -1.`,
      );
    }
    return value & 0xff;
  };
  return {
    mapGroup: exactSentinel("GROUP_N_A"),
    mapNumber: exactSentinel("MAP_N_A"),
  };
};

const validateJumpRoamMonMask = (): void => {
  const lines = readAsmLines(path.join("engine", "overworld", "wildmons.asm"));
  const start = lines.findIndex(
    (line) => stripAsmComment(line).trim() === "JumpRoamMon:",
  );
  if (start < 0) {
    throw new Error(
      "Unable to find JumpRoamMon in engine/overworld/wildmons.asm",
    );
  }
  const end = lines.findIndex(
    (line, index) =>
      index > start && stripAsmComment(line).trim() === "_BackUpMapIndices:",
  );
  const block = lines
    .slice(start + 1, end < 0 ? lines.length : end)
    .map((line) => stripAsmComment(line).trim())
    .filter(Boolean);
  const maskOperands = block
    .map((line) => line.match(/^maskbits (.+)$/)?.[1])
    .filter((value): value is string => Boolean(value));
  if (maskOperands.length !== 1 || maskOperands[0] !== "NUM_ROAMMON_MAPS") {
    throw new Error("JumpRoamMon must mask with exact NUM_ROAMMON_MAPS.");
  }
  if (block.filter((line) => line === "cp NUM_ROAMMON_MAPS").length !== 1) {
    throw new Error(
      "JumpRoamMon must compare against exact NUM_ROAMMON_MAPS once.",
    );
  }
};

const parseRoamingRoutes = (): SymbolicRoamingRoute[] => {
  const lines = readAsmLines(path.join("data", "wild", "roammon_maps.asm"));
  const start = lines.findIndex(
    (line) => stripAsmComment(line).trim() === "RoamMaps:",
  );
  if (start < 0) {
    throw new Error("Unable to find RoamMaps in data/wild/roammon_maps.asm");
  }
  const rows: SymbolicRoamingRoute[] = [];
  const origins = new Set<string>();
  let foundEnd = false;
  for (const raw of lines.slice(start + 1)) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    if (line === "db -1") {
      foundEnd = true;
      break;
    }
    if (!line.startsWith("roam_map ")) {
      throw new Error(`Unexpected RoamMaps source row '${line}'.`);
    }
    const args = splitAsmArgs(line.slice("roam_map ".length));
    const connectionCount = args.length - 1;
    if (connectionCount < 1 || connectionCount > 4) {
      throw new Error(
        `RoamMaps row ${rows.length} must declare 1..4 connections, found ${connectionCount}.`,
      );
    }
    if (args.some((value) => !/^[A-Z][A-Z0-9_]*$/.test(value))) {
      throw new Error(
        `RoamMaps row ${rows.length} has malformed map constants.`,
      );
    }
    const [origin, ...connections] = args;
    if (origins.has(origin)) {
      throw new Error(`RoamMaps repeats origin '${origin}'.`);
    }
    origins.add(origin);
    rows.push({ origin, connections });
  }
  if (!foundEnd) {
    throw new Error("RoamMaps is missing exact db -1 terminator.");
  }
  if (rows.length !== CANONICAL_ROAMING_ROUTE_ROWS.length) {
    throw new Error(
      `RoamMaps has ${rows.length} rows, expected exactly ${CANONICAL_ROAMING_ROUTE_ROWS.length}.`,
    );
  }
  for (let index = 0; index < CANONICAL_ROAMING_ROUTE_ROWS.length; index += 1) {
    const actual = rows[index];
    const [expectedOrigin, ...expectedConnections] =
      CANONICAL_ROAMING_ROUTE_ROWS[index];
    if (actual.origin !== expectedOrigin) {
      throw new Error(
        `RoamMaps row ${index} origin '${actual.origin}' does not match canonical '${expectedOrigin}'.`,
      );
    }
    if (
      actual.connections.length !== expectedConnections.length ||
      actual.connections.some(
        (connection, connectionIndex) =>
          connection !== expectedConnections[connectionIndex],
      )
    ) {
      throw new Error(
        `RoamMaps row ${index} connections '${actual.connections.join(", ")}' do not match canonical '${expectedConnections.join(", ")}'.`,
      );
    }
  }
  return rows;
};

const resolveRoamingMapLocations = (
  runtimeMapMetadata: Record<string, RuntimeMapMetadataRecord>,
): Map<string, RoamingMapLocation> => {
  const constants = new Set<string>();
  for (const row of CANONICAL_ROAMING_ROUTE_ROWS) {
    for (const mapConstant of row) {
      constants.add(mapConstant);
    }
  }
  for (const initWrite of CANONICAL_ROAMING_INIT_WRITES) {
    if (initWrite.field === "mapGroup" || initWrite.field === "mapNumber") {
      constants.add(initWrite.value);
    }
  }
  const locations = new Map<string, RoamingMapLocation>();
  const constantsByLocation = new Map<string, string>();
  for (const mapConstant of constants) {
    const metadata = runtimeMapMetadata[mapConstant];
    if (!metadata) {
      throw new Error(
        `Roaming map '${mapConstant}' is missing from runtime map metadata.`,
      );
    }
    if (
      !Number.isInteger(metadata.groupId) ||
      metadata.groupId < 1 ||
      metadata.groupId > 0xff
    ) {
      throw new Error(
        `Roaming map '${mapConstant}' group ${metadata.groupId} is outside exact nonzero byte range.`,
      );
    }
    if (
      !Number.isInteger(metadata.mapId) ||
      metadata.mapId < 1 ||
      metadata.mapId > 0xff
    ) {
      throw new Error(
        `Roaming map '${mapConstant}' number ${metadata.mapId} is outside exact nonzero byte range.`,
      );
    }
    const key = `${metadata.groupId}:${metadata.mapId}`;
    const duplicate = constantsByLocation.get(key);
    if (duplicate) {
      throw new Error(
        `Roaming maps '${duplicate}' and '${mapConstant}' resolve to duplicate location ${key}.`,
      );
    }
    constantsByLocation.set(key, mapConstant);
    locations.set(mapConstant, {
      mapGroup: metadata.groupId,
      mapNumber: metadata.mapId,
    });
  }
  return locations;
};

export const exportRoamingPokemon = (
  runtimeMapMetadata: Record<string, RuntimeMapMetadataRecord>,
): RoamingPokemonCatalog => {
  const count = parseNumRoamMonMaps();
  const inactiveMap = parseInactiveRoamingMap();
  const inactiveGroupCollision = Object.entries(runtimeMapMetadata).find(
    ([, metadata]) => metadata.groupId === inactiveMap.mapGroup,
  );
  if (inactiveGroupCollision) {
    throw new Error(
      `Inactive roaming map group ${inactiveMap.mapGroup} collides with runtime map '${inactiveGroupCollision[0]}'.`,
    );
  }
  validateJumpRoamMonMask();
  const symbolicInitWrites = parseInitRoamMons();
  const symbolicRoutes = parseRoamingRoutes();
  const locations = resolveRoamingMapLocations(runtimeMapMetadata);
  const locationFor = (mapConstant: string): RoamingMapLocation => {
    const location = locations.get(mapConstant);
    if (!location) {
      throw new Error(`Roaming map '${mapConstant}' was not resolved.`);
    }
    return { ...location };
  };
  const jumpMask = count - 1;
  if (jumpMask !== 15) {
    throw new Error(`Roaming jump mask is ${jumpMask}, expected exact 15.`);
  }
  const catalog: RoamingPokemonCatalog = {
    slotCount: 3,
    inactiveMap,
    initWrites: symbolicInitWrites.map((write) => ({
      slot: write.slot,
      species: write.species,
      level: write.level,
      ...locationFor(write.mapConstant),
      hp: write.hp,
    })),
    routes: symbolicRoutes.map((route) => ({
      ...locationFor(route.origin),
      connections: route.connections.map(locationFor),
    })),
    jumpMask,
  };
  writeJsonToTargets("roaming_pokemon.json", catalog, { indent: 2 });
  return catalog;
};

export const exportBuenaPrizes = (): BuenaPrizeDefinitions => {
  const prizes: BuenaPrizeDefinitions = {};
  let inTable = false;
  for (const raw of readAsmLines(
    path.join("data", "items", "buena_prizes.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    if (line === "BuenaPrizeItems:") {
      inTable = true;
      continue;
    }
    if (!inTable) {
      continue;
    }
    if (line.startsWith("assert_table_length ")) {
      break;
    }
    if (line.startsWith("table_width ")) {
      continue;
    }
    const match = line.match(/^db\s+([A-Z0-9_]+),\s*([0-9]+)$/);
    if (!match) {
      throw new Error(`Malformed Buena prize row: ${raw}`);
    }
    if (Object.prototype.hasOwnProperty.call(prizes, match[1])) {
      throw new Error(`Duplicate Buena prize item '${match[1]}'.`);
    }
    const cost = Number.parseInt(match[2], 10);
    if (cost > 0xff) {
      throw new Error(`Buena prize cost ${cost} is outside byte range.`);
    }
    prizes[match[1]] = cost;
  }
  if (!Object.keys(prizes).length) {
    throw new Error(
      "No Buena prize definitions were exported from data/items/buena_prizes.asm",
    );
  }
  writeJsonToTargets("buena_prizes.json", prizes, { indent: 2 });
  return prizes;
};

const parseBuenaPasswordOption = (token: string): string => {
  const trimmed = token.trim();
  const stringMatch = trimmed.match(/^"(.+)@"$/);
  if (stringMatch) {
    return stringMatch[1];
  }
  if (!/^[A-Z0-9_]+$/.test(trimmed)) {
    throw new Error(`Malformed Buena password option '${token}'`);
  }
  return trimmed;
};

export const exportBuenaPasswordCategories = (): BuenaPasswordCategories => {
  const lines = readAsmLines(
    path.join("data", "radio", "buenas_passwords.asm"),
  );
  const categoryOrder: string[] = [];
  const rows = new Map<string, BuenaPasswordCategoryDefinition>();
  let inPointerTable = false;
  for (const raw of lines) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    if (line === "BuenasPasswordTable:") {
      inPointerTable = true;
      continue;
    }
    if (inPointerTable) {
      if (line.startsWith("table_width ")) {
        continue;
      }
      if (line.startsWith("assert_table_length ")) {
        inPointerTable = false;
        continue;
      }
      const pointerMatch = line.match(/^dw\s+\.([A-Za-z0-9_]+)$/);
      if (!pointerMatch) {
        throw new Error(`Malformed Buena password pointer row: ${raw}`);
      }
      categoryOrder.push(pointerMatch[1]);
      continue;
    }
    const rowMatch = line.match(
      /^\.([A-Za-z0-9_]+):\s+db\s+([A-Z0-9_]+),\s*([0-9]+),\s*(.+)$/,
    );
    if (!rowMatch) {
      continue;
    }
    if (rows.has(rowMatch[1])) {
      throw new Error(`Duplicate Buena password category '${rowMatch[1]}'.`);
    }
    const options = rowMatch[4].split(/,\s*/).map(parseBuenaPasswordOption);
    if (options.length !== 3) {
      throw new Error(
        `Buena password category ${rowMatch[1]} must declare exactly three options`,
      );
    }
    const points = Number.parseInt(rowMatch[3], 10);
    if (points > 0xff) {
      throw new Error(
        `Buena password category ${rowMatch[1]} points ${points} is outside byte range.`,
      );
    }
    rows.set(rowMatch[1], {
      categoryType: rowMatch[2],
      points,
      options,
    });
  }
  if (!categoryOrder.length) {
    throw new Error(
      "No Buena password category pointers were exported from data/radio/buenas_passwords.asm",
    );
  }
  const categories: Record<string, BuenaPasswordCategoryDefinition> = {};
  for (const id of categoryOrder) {
    const row = rows.get(id);
    if (!row) {
      throw new Error(
        `Buena password pointer references missing category row '${id}'`,
      );
    }
    if (Object.prototype.hasOwnProperty.call(categories, id)) {
      throw new Error(`Duplicate Buena password category id '${id}'.`);
    }
    categories[id] = row;
  }
  const catalog = { order: categoryOrder, categories };
  writeJsonToTargets("buena_password_categories.json", catalog, { indent: 2 });
  return catalog;
};

export const exportKurtApricornRecipes = (): KurtApricornRecipes => {
  const recipes: KurtApricornRecipes = {};
  let inTable = false;
  for (const raw of readAsmLines(
    path.join("data", "items", "apricorn_balls.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    if (line === "ApricornBalls:") {
      inTable = true;
      continue;
    }
    if (!inTable) {
      continue;
    }
    if (line === "db -1") {
      break;
    }
    const match = line.match(/^db\s+([A-Z0-9_]+),\s*([A-Z0-9_]+)$/);
    if (!match) {
      throw new Error(`Malformed Kurt apricorn recipe row: ${raw}`);
    }
    if (Object.prototype.hasOwnProperty.call(recipes, match[1])) {
      throw new Error(`Duplicate Kurt apricorn recipe '${match[1]}'.`);
    }
    recipes[match[1]] = match[2];
  }
  if (!Object.keys(recipes).length) {
    throw new Error(
      "No Kurt apricorn recipes were exported from data/items/apricorn_balls.asm",
    );
  }
  writeJsonToTargets("kurt_apricorn_recipes.json", recipes, { indent: 2 });
  return recipes;
};

export const exportShuckieGift = (): ShuckieGiftDefinition => {
  const lines = readAsmLines(path.join("engine", "events", "shuckle.asm"));
  let originalTrainerId: number | null = null;
  let species: string | null = null;
  let level: number | null = null;
  let heldItem: string | null = null;
  let nickname: string | null = null;
  let originalTrainerName: string | null = null;
  let gotTodayEngineFlag: string | null = null;
  let inGiveShuckle = false;
  let pendingNameLabel: "SpecialShuckleOT" | "SpecialShuckleNickname" | null =
    null;
  for (const raw of lines) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    const otIdMatch = line.match(/^DEF\s+MANIA_OT_ID\s+EQU\s+([0-9]+)$/);
    if (otIdMatch) {
      originalTrainerId = Number.parseInt(otIdMatch[1], 10);
      continue;
    }
    if (line === "GiveShuckle:") {
      inGiveShuckle = true;
      continue;
    }
    if (line === "ReturnShuckie:") {
      inGiveShuckle = false;
      continue;
    }
    if (line === "SpecialShuckleOT:") {
      pendingNameLabel = "SpecialShuckleOT";
      continue;
    }
    if (line === "SpecialShuckleNickname:") {
      pendingNameLabel = "SpecialShuckleNickname";
      continue;
    }
    if (pendingNameLabel) {
      const nameMatch = line.match(/^db\s+"([^@]+)@"$/);
      if (!nameMatch) {
        throw new Error(`Malformed ${pendingNameLabel} row: ${raw}`);
      }
      if (pendingNameLabel === "SpecialShuckleOT") {
        originalTrainerName = nameMatch[1];
      } else {
        nickname = nameMatch[1];
      }
      pendingNameLabel = null;
      continue;
    }
    if (!inGiveShuckle) {
      continue;
    }
    let match = line.match(/^ld a,\s*([A-Z0-9_]+)$/);
    if (match?.[1] === "SHUCKLE") {
      species = match[1];
      continue;
    }
    match = line.match(/^ld a,\s*([0-9]+)$/);
    if (match && species && level === null) {
      level = Number.parseInt(match[1], 10);
      continue;
    }
    match = line.match(/^ld \[hl\],\s*([A-Z0-9_]+)$/);
    if (match) {
      heldItem = match[1];
      continue;
    }
    match = line.match(/^set\s+([A-Z0-9_]+)_F,\s*\[hl\]$/);
    if (match) {
      gotTodayEngineFlag = `ENGINE_${match[1].replace(/^DAILYFLAGS[0-9]_/, "")}`;
    }
  }
  if (
    !species ||
    level === null ||
    !heldItem ||
    !nickname ||
    !originalTrainerName ||
    originalTrainerId === null ||
    !gotTodayEngineFlag
  ) {
    throw new Error(
      "Could not export complete Shuckie gift definition from engine/events/shuckle.asm",
    );
  }
  if (level < 1 || level > 100) {
    throw new Error(
      `Shuckie gift level ${level} is outside Pokemon level range.`,
    );
  }
  if (originalTrainerId < 0 || originalTrainerId > 0xffff) {
    throw new Error(
      `Shuckie gift original trainer id ${originalTrainerId} is outside word range.`,
    );
  }
  const gift = {
    species,
    level,
    heldItem,
    nickname,
    originalTrainerName,
    originalTrainerId,
    gotTodayEngineFlag,
  };
  writeJsonToTargets("shuckie_gift.json", gift, { indent: 2 });
  return gift;
};

export const exportDratiniMoveSets = (): DratiniMoveSets => {
  const moveSets: DratiniMoveSets = {};
  let currentMode: number | null = null;
  let currentMoves: string[] = [];
  for (const raw of readAsmLines(
    path.join("engine", "events", "dratini.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    const labelMatch = line.match(/^\.Moveset([0-9]+):$/);
    if (labelMatch) {
      if (currentMode !== null) {
        throw new Error(
          `Dratini moveset ${currentMode} is missing zero terminator`,
        );
      }
      currentMode = Number.parseInt(labelMatch[1], 10);
      currentMoves = [];
      continue;
    }
    if (currentMode === null) {
      continue;
    }
    const dbMatch = line.match(/^db\s+([A-Z0-9_]+|0)$/);
    if (!dbMatch) {
      throw new Error(`Malformed Dratini moveset row: ${raw}`);
    }
    if (dbMatch[1] === "0") {
      if (!currentMoves.length) {
        throw new Error(`Dratini moveset ${currentMode} must not be empty.`);
      }
      if (currentMoves.length > 4) {
        throw new Error(
          `Dratini moveset ${currentMode} has ${currentMoves.length} moves, exceeding party move limit.`,
        );
      }
      if (Object.prototype.hasOwnProperty.call(moveSets, String(currentMode))) {
        throw new Error(`Duplicate Dratini moveset ${currentMode}.`);
      }
      moveSets[String(currentMode)] = currentMoves;
      currentMode = null;
      currentMoves = [];
      continue;
    }
    currentMoves.push(dbMatch[1]);
  }
  if (currentMode !== null) {
    throw new Error(
      `Dratini moveset ${currentMode} is missing zero terminator`,
    );
  }
  if (!Object.keys(moveSets).length) {
    throw new Error(
      "No Dratini move sets were exported from engine/events/dratini.asm",
    );
  }
  writeJsonToTargets("dratini_move_sets.json", moveSets, { indent: 2 });
  return moveSets;
};

const parseScriptConstantNumber = (name: string): number => {
  for (const raw of readAsmLines(
    path.join("constants", "script_constants.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    const match = line.match(new RegExp(`^DEF\\s+${name}\\s+EQU\\s+([0-9]+)$`));
    if (match) {
      return Number.parseInt(match[1], 10);
    }
  }
  throw new Error(`Could not parse required script constant ${name}.`);
};

const parseBugContestSelectedContestantCount = (): number => {
  let inRoutine = false;
  let afterClearLoop = false;
  for (const raw of readAsmLines(
    path.join("engine", "events", "bug_contest", "contest_2.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    if (line === "SelectRandomBugContestContestants:") {
      inRoutine = true;
      continue;
    }
    if (!inRoutine) {
      continue;
    }
    if (line === "CheckBugContestContestantFlag:") {
      break;
    }
    if (line === ".loop2") {
      break;
    }
    if (line === ".loop1") {
      afterClearLoop = true;
      continue;
    }
    if (!afterClearLoop) {
      continue;
    }
    const match = line.match(/^ld c,\s*([0-9]+)$/);
    if (match) {
      return Number.parseInt(match[1], 10);
    }
  }
  throw new Error(
    "Could not parse selected Bug-Catching Contest contestant count.",
  );
};

const parseBugContestContestantFlags = (): string[] => {
  const flags: string[] = [];
  let inTable = false;
  for (const raw of readAsmLines(
    path.join("data", "events", "bug_contest_flags.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    if (line === "BugCatchingContestantEventFlagTable:") {
      inTable = true;
      continue;
    }
    if (!inTable) {
      continue;
    }
    if (line.startsWith("assert_table_length ")) {
      break;
    }
    if (line.startsWith("table_width ")) {
      continue;
    }
    const match = line.match(/^dw\s+(EVENT_[A-Z0-9_]+)$/);
    if (!match) {
      throw new Error(
        `Malformed Bug-Catching Contest contestant flag row: ${raw}`,
      );
    }
    flags.push(match[1]);
  }
  if (!flags.length) {
    throw new Error(
      "No Bug-Catching Contest contestant flags were exported from data/events/bug_contest_flags.asm",
    );
  }
  return flags;
};

type SourceBugContestEncounter = Omit<BugContestEncounter, "weight"> & {
  sourceWeight: number;
};

const BUG_CONTEST_ENCOUNTER_SOURCE = path.join(
  "data",
  "wild",
  "bug_contest_mons.asm",
);

const BUG_CONTEST_ENCOUNTER_CERTIFICATE: readonly SourceBugContestEncounter[] =
  [
    {
      sourceWeight: 20,
      species: "CATERPIE",
      minLevel: 7,
      maxLevel: 18,
    },
    {
      sourceWeight: 20,
      species: "WEEDLE",
      minLevel: 7,
      maxLevel: 18,
    },
    {
      sourceWeight: 10,
      species: "METAPOD",
      minLevel: 9,
      maxLevel: 18,
    },
    {
      sourceWeight: 10,
      species: "KAKUNA",
      minLevel: 9,
      maxLevel: 18,
    },
    {
      sourceWeight: 5,
      species: "BUTTERFREE",
      minLevel: 12,
      maxLevel: 15,
    },
    {
      sourceWeight: 5,
      species: "BEEDRILL",
      minLevel: 12,
      maxLevel: 15,
    },
    {
      sourceWeight: 10,
      species: "VENONAT",
      minLevel: 10,
      maxLevel: 16,
    },
    {
      sourceWeight: 10,
      species: "PARAS",
      minLevel: 10,
      maxLevel: 17,
    },
    {
      sourceWeight: 5,
      species: "SCYTHER",
      minLevel: 13,
      maxLevel: 14,
    },
    {
      sourceWeight: 5,
      species: "PINSIR",
      minLevel: 13,
      maxLevel: 14,
    },
    {
      sourceWeight: -1,
      species: "VENOMOTH",
      minLevel: 30,
      maxLevel: 40,
    },
  ];

const parseBugContestEncounters = (): BugContestEncounter[] => {
  let rawLines: string[];
  try {
    rawLines = readAsmLines(BUG_CONTEST_ENCOUNTER_SOURCE);
  } catch {
    throw new Error(
      "Required Bug-Catching Contest encounter source data/wild/bug_contest_mons.asm could not be read.",
    );
  }

  const lines = rawLines
    .map((raw) => ({ raw, line: stripAsmComment(raw).trim() }))
    .filter(({ line }) => line.length > 0);
  if (lines[0]?.line !== "ContestMons:") {
    throw new Error(
      "Bug-Catching Contest encounter source must begin with exact ContestMons: label.",
    );
  }

  const encounters = lines.slice(1).map(({ raw, line }, index) => {
    const match = line.match(
      /^db\s+(-?[0-9]+)\s*,\s*([A-Z][A-Z0-9_]*)\s*,\s*([0-9]+)\s*,\s*([0-9]+)$/,
    );
    if (!match) {
      throw new Error(
        `Malformed Bug-Catching Contest encounter row ${index + 1}: ${raw}`,
      );
    }
    const sourceWeight = Number.parseInt(match[1], 10);
    const minLevel = Number.parseInt(match[3], 10);
    const maxLevel = Number.parseInt(match[4], 10);
    if (sourceWeight !== -1 && (sourceWeight < 1 || sourceWeight > 0xfe)) {
      throw new Error(
        `Bug-Catching Contest encounter row ${index + 1} weight ${sourceWeight} is outside weighted byte range.`,
      );
    }
    if (minLevel < 1 || minLevel > 100 || maxLevel < 1 || maxLevel > 100) {
      throw new Error(
        `Bug-Catching Contest encounter row ${index + 1} levels ${minLevel}-${maxLevel} are outside Pokemon level range.`,
      );
    }
    if (minLevel > maxLevel) {
      throw new Error(
        `Bug-Catching Contest encounter row ${index + 1} minimum level ${minLevel} exceeds maximum level ${maxLevel}.`,
      );
    }
    return {
      sourceWeight,
      weight: sourceWeight === -1 ? 0xff : sourceWeight,
      species: match[2],
      minLevel,
      maxLevel,
    };
  });

  if (encounters.length !== BUG_CONTEST_ENCOUNTER_CERTIFICATE.length) {
    throw new Error(
      `Bug-Catching Contest encounter table must contain exactly 10 weighted rows and one final sentinel row; found ${encounters.length} rows.`,
    );
  }
  if (
    encounters.at(-1)?.sourceWeight !== -1 ||
    encounters.slice(0, -1).some(({ sourceWeight }) => sourceWeight === -1)
  ) {
    throw new Error(
      "Bug-Catching Contest encounter sentinel must be the final row after 10 weighted rows.",
    );
  }

  encounters.forEach((encounter, index) => {
    const canonical = BUG_CONTEST_ENCOUNTER_CERTIFICATE[index];
    if (
      encounter.sourceWeight !== canonical.sourceWeight ||
      encounter.species !== canonical.species ||
      encounter.minLevel !== canonical.minLevel ||
      encounter.maxLevel !== canonical.maxLevel
    ) {
      throw new Error(
        `Bug-Catching Contest encounter row ${index + 1} changed canonical order or values.`,
      );
    }
  });

  return encounters.map(
    ({ weight, species, minLevel, maxLevel }): BugContestEncounter => ({
      weight,
      species,
      minLevel,
      maxLevel,
    }),
  );
};

export const exportBugContestConfig = (): BugContestConfig => {
  const config = {
    parkBalls: parseScriptConstantNumber("BUG_CONTEST_BALLS"),
    timerMinutes: parseScriptConstantNumber("BUG_CONTEST_MINUTES"),
    timerSeconds: parseScriptConstantNumber("BUG_CONTEST_SECONDS"),
    selectedContestantCount: parseBugContestSelectedContestantCount(),
    contestantFlags: parseBugContestContestantFlags(),
    encounters: parseBugContestEncounters(),
  };
  if (config.parkBalls < 1 || config.parkBalls > 0xff) {
    throw new Error(
      `Bug-Catching Contest park ball count ${config.parkBalls} is outside byte count range.`,
    );
  }
  if (config.timerMinutes > 0xff) {
    throw new Error(
      `Bug-Catching Contest timer minutes ${config.timerMinutes} is outside byte range.`,
    );
  }
  if (config.timerSeconds > 59) {
    throw new Error(
      `Bug-Catching Contest timer seconds ${config.timerSeconds} is outside clock second range.`,
    );
  }
  if (
    config.selectedContestantCount < 1 ||
    config.selectedContestantCount > 0xff
  ) {
    throw new Error(
      `Bug-Catching Contest selected contestant count ${config.selectedContestantCount} is outside byte count range.`,
    );
  }
  const expectedContestants = parseScriptConstantNumber("NUM_BUG_CONTESTANTS");
  if (expectedContestants < 1 || expectedContestants > 0xff) {
    throw new Error(
      `NUM_BUG_CONTESTANTS ${expectedContestants} is outside byte count range.`,
    );
  }
  if (config.contestantFlags.length !== expectedContestants) {
    throw new Error(
      `Bug-Catching Contest flag count ${config.contestantFlags.length} does not match NUM_BUG_CONTESTANTS ${expectedContestants}.`,
    );
  }
  if (config.selectedContestantCount > expectedContestants) {
    throw new Error(
      `Bug-Catching Contest selected contestant count ${config.selectedContestantCount} exceeds contestant flags ${expectedContestants}.`,
    );
  }
  writeJsonToTargets("bug_contest_config.json", config, { indent: 2 });
  return config;
};

const parsePokemonConstantOrder = (): string[] => {
  const species: string[] = [];
  let inPokemonConstants = false;
  for (const raw of readAsmLines(
    path.join("constants", "pokemon_constants.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    if (/^const_def(?:\s+[0-9]+)?$/.test(line)) {
      inPokemonConstants = true;
      continue;
    }
    if (!inPokemonConstants) {
      continue;
    }
    if (line.startsWith("DEF NUM_POKEMON")) {
      break;
    }
    const match = line.match(/^const\s+([A-Z0-9_]+)$/);
    if (match) {
      species.push(match[1]);
    }
  }
  if (!species.length) {
    throw new Error(
      "No Pokemon constants were exported from constants/pokemon_constants.asm",
    );
  }
  return species;
};

const exportBattleTowerDefinitions = (): {
  trainers: BattleTowerTrainerDefinition[];
  monGroups: BattleTowerMonDefinition[][];
} => {
  const requiredPaths = [
    path.join("constants", "trainer_constants.asm"),
    path.join("data", "trainers", "genders.asm"),
    path.join("data", "trainers", "sprites.asm"),
    path.join("data", "battle_tower", "classes.asm"),
    path.join("data", "battle_tower", "parties.asm"),
  ];
  for (const source of requiredPaths) {
    if (!fs.existsSync(path.join(getDisassemblyRoot(), source))) {
      throw new Error(`Required Battle Tower source is missing: ${source}`);
    }
  }
  const trainerConstants = readAsmLines(path.join("constants", "trainer_constants.asm"));
  const trainerClasses = trainerConstants
    .map((line) => stripAsmComment(line).trim().match(/^trainerclass\s+([A-Z0-9_]+)$/)?.[1])
    .filter((value): value is string => Boolean(value) && value !== "TRAINER_NONE");
  const spriteLines = readAsmLines(path.join("data", "trainers", "sprites.asm"))
    .map((line) => stripAsmComment(line).trim())
    .filter((line) => line.startsWith("db "))
    .flatMap((line) => splitAsmArgs(line.slice(3)));
  const classToSprite = new Map<string, string>();
  trainerClasses.forEach((trainerClass, index) => {
    const sprite = spriteLines[index];
    if (sprite) classToSprite.set(trainerClass, sprite);
  });
  const classToFemale = new Map<string, boolean>();
  for (const line of readAsmLines(path.join("data", "trainers", "genders.asm"))) {
    const match = line.match(/^\s*db\s+(MALE|FEMALE)\s*;\s*([A-Z0-9_]+)/);
    if (match) classToFemale.set(match[2], match[1] === "FEMALE");
  }
  let trainerIndex = 0;
  const trainers = readAsmLines(path.join("data", "battle_tower", "classes.asm"))
    .map((line) => {
      const match = stripAsmComment(line).trim().match(/^bt_trainer\s+([A-Z0-9_]+),\s*"([^"]+)"/);
      if (!match) return undefined;
      const [, trainerClass, name] = match;
      const spriteConstant = classToSprite.get(trainerClass);
      if (!spriteConstant) {
        throw new Error(`Battle Tower trainer class '${trainerClass}' has no sprite mapping`);
      }
      const female = classToFemale.get(trainerClass);
      if (female === undefined) {
        throw new Error(`Battle Tower trainer class '${trainerClass}' has no gender mapping`);
      }
      return { index: trainerIndex++, trainerClass, name, spriteConstant, female };
    })
    .filter((value): value is BattleTowerTrainerDefinition => Boolean(value));
  if (!trainers.length) throw new Error("Battle Tower trainer roster is empty");

  const groups: BattleTowerMonDefinition[][] = [];
  let current: BattleTowerMonDefinition[] = [];
  let mon: Partial<BattleTowerMonDefinition> | undefined;
  let stage = "";
  let statExp: number[] = [];
  let statValues: number[] = [];
  const finishMon = () => {
    if (!mon) return;
    current.push({
      species: String(mon.species ?? ""),
      item: mon.item ?? null,
      moves: mon.moves ?? [],
      originalTrainerId: Number(mon.originalTrainerId ?? 0),
      experience: Number(mon.experience ?? 0),
      statExp: mon.statExp ?? [],
      dvs: mon.dvs ?? [0, 0, 0, 0],
      pp: mon.pp ?? [0, 0, 0, 0],
      happiness: Number(mon.happiness ?? 0),
      pokerus: mon.pokerus ?? [0, 0, 0],
      level: Number(mon.level ?? 1),
      status: mon.status ?? [0, 0],
      stats: mon.stats ?? [],
      nickname: String(mon.nickname ?? ""),
    });
    mon = undefined;
    stage = "";
  };
  const number = (token: string): number => parseAsmNumber(token.trim());
  for (const raw of readAsmLines(path.join("data", "battle_tower", "parties.asm"))) {
    if (raw.trim().startsWith("; BattleTowerMons group")) {
      if (current.length) groups.push(current);
      current = [];
      continue;
    }
    const stripped = stripAsmComment(raw).trim();
    if (!stripped) continue;
    if (stripped.startsWith("db ") && !mon) {
      mon = { species: splitAsmArgs(stripped.slice(3))[0] };
      stage = "item";
      continue;
    }
    if (!mon) continue;
    if (stage === "item" && stripped.startsWith("db ")) {
      const item = splitAsmArgs(stripped.slice(3))[0];
      mon.item = item === "NO_ITEM" || item === "0" ? null : item;
      stage = "moves";
      continue;
    }
    if (stage === "moves" && stripped.startsWith("db ")) {
      mon.moves = [...(mon.moves ?? []), ...splitAsmArgs(stripped.slice(3))].slice(0, 4);
      continue;
    }
    if (stage === "moves" && stripped.startsWith("dw ")) {
      mon.originalTrainerId = number(stripped.slice(3));
      stage = "experience";
      continue;
    }
    if (stage === "experience" && stripped.startsWith("bigdt ")) {
      mon.experience = number(stripped.split(/\s+/)[1]);
      statExp = [];
      stage = "statExp";
      continue;
    }
    if (stage === "statExp" && stripped.startsWith("bigdw ")) {
      statExp.push(number(stripped.split(/\s+/)[1]));
      if (statExp.length >= 5) { mon.statExp = statExp; stage = "dvs"; }
      continue;
    }
    if (stage === "dvs" && stripped.startsWith("dn ")) {
      mon.dvs = splitAsmArgs(stripped.slice(3)).map(number);
      stage = "pp";
      continue;
    }
    if (stage === "pp" && stripped.startsWith("db ")) {
      mon.pp = splitAsmArgs(stripped.slice(3)).map(number);
      stage = "happiness";
      continue;
    }
    if (stage === "happiness" && stripped.startsWith("db ")) {
      mon.happiness = number(stripped.slice(3));
      stage = "pokerus";
      continue;
    }
    if (stage === "pokerus" && stripped.startsWith("db ")) {
      mon.pokerus = splitAsmArgs(stripped.slice(3)).map(number);
      stage = "level";
      continue;
    }
    if (stage === "level" && stripped.startsWith("db ")) {
      mon.level = number(stripped.slice(3));
      stage = "status";
      continue;
    }
    if (stage === "status" && stripped.startsWith("db ")) {
      mon.status = splitAsmArgs(stripped.slice(3)).map(number);
      statValues = [];
      stage = "stats";
      continue;
    }
    if (stage === "stats" && stripped.startsWith("bigdw ")) {
      statValues.push(number(stripped.split(/\s+/)[1]));
      if (statValues.length >= 7) { mon.stats = statValues; stage = "nickname"; }
      continue;
    }
    if (stage === "nickname" && stripped.startsWith("dname ")) {
      const nickname = stripped.match(/"([^"]+)"/)?.[1];
      if (!nickname) throw new Error("Battle Tower mon nickname is missing");
      mon.nickname = nickname;
      finishMon();
    }
  }
  if (mon) throw new Error("Battle Tower party parser ended mid-entry");
  if (current.length) groups.push(current);
  if (!groups.length || groups.some((group) => group.length === 0)) {
    throw new Error("Battle Tower parties contain an empty group");
  }
  return { trainers, monGroups: groups };
};

export const exportBattleTowerRules = (): BattleTowerRules => {
  const constantsLines = readAsmLines(
    path.join("constants", "battle_tower_constants.asm"),
  ).map((raw) => stripAsmComment(raw).trim());
  const parseBattleTowerConstant = (name: string): number => {
    const line = constantsLines.find((candidate) =>
      candidate.startsWith(`DEF ${name} EQU `),
    );
    const match = line?.match(
      new RegExp(`^DEF\\s+${name}\\s+EQU\\s+([0-9]+)$`),
    );
    if (!match) {
      throw new Error(
        `Could not parse ${name} from battle_tower_constants.asm`,
      );
    }
    return Number(match[1]);
  };
  const requiredPartyCount = parseBattleTowerConstant(
    "BATTLETOWER_PARTY_LENGTH",
  );
  const challengeStreakLength = parseBattleTowerConstant(
    "BATTLETOWER_STREAK_LENGTH",
  );
  if (requiredPartyCount < 1 || requiredPartyCount > 6) {
    throw new Error(
      `Battle Tower party length ${requiredPartyCount} is outside party size range.`,
    );
  }
  if (challengeStreakLength < 1 || challengeStreakLength > 0xff) {
    throw new Error(
      `Battle Tower streak length ${challengeStreakLength} is outside byte count range.`,
    );
  }
  const parseBattleTowerRewards = () => {
    const parseBattleTowerTokenConstant = (name: string): string => {
    const line = constantsLines.find((candidate) =>
      candidate.startsWith(`DEF ${name} EQU `),
    );
    const match = line?.match(
      new RegExp(`^DEF\\s+${name}\\s+EQU\\s+([A-Z0-9_]+)$`),
    );
    if (!match) {
      throw new Error(
        `Could not parse ${name} from battle_tower_constants.asm`,
      );
    }
    return match[1];
    };
    const rewardQuantity = parseBattleTowerConstant(
    "BATTLETOWER_REWARD_QUANTITY",
  );
  const minimumReward = parseBattleTowerTokenConstant(
    "BATTLETOWER_MIN_REWARD",
  );
  const maximumReward = parseBattleTowerTokenConstant(
    "BATTLETOWER_MAX_REWARD",
  );
  const itemOrder = readAsmLines(path.join("constants", "item_constants.asm"))
    .map((raw) => stripAsmComment(raw).trim())
    .map((line) => line.match(/^const\s+([A-Z0-9_]+)$/)?.[1] ?? null)
    .filter((item): item is string => item !== null);
  const minimumRewardIndex = itemOrder.indexOf(minimumReward);
  const maximumRewardIndex = itemOrder.indexOf(maximumReward);
  if (minimumRewardIndex < 0 || maximumRewardIndex < minimumRewardIndex) {
    throw new Error(
      `Battle Tower reward range ${minimumReward}..${maximumReward} is missing or unordered in item_constants.asm`,
    );
  }
  const rewardCandidates = itemOrder.slice(
    minimumRewardIndex,
    maximumRewardIndex + 1,
  );
  const battleTowerRoutineLines = readAsmLines(
    path.join("engine", "events", "battle_tower", "battle_tower.asm"),
  ).map((raw) => stripAsmComment(raw).trim());
  const chooseRewardStart = battleTowerRoutineLines.indexOf(
    "BattleTower_RandomlyChooseReward:",
  );
  const chooseRewardEndOffset = battleTowerRoutineLines
    .slice(chooseRewardStart + 1)
    .findIndex((line) => /^[A-Za-z0-9_]+:$/.test(line));
  if (chooseRewardStart < 0) {
    throw new Error("Could not parse BattleTower_RandomlyChooseReward");
  }
  const chooseRewardBody = battleTowerRoutineLines.slice(
    chooseRewardStart + 1,
    chooseRewardEndOffset < 0
      ? undefined
      : chooseRewardStart + 1 + chooseRewardEndOffset,
  );
  const excludedRewardItems = chooseRewardBody.flatMap((line, index) => {
    const match = line.match(/^cp\s+([A-Z0-9_]+)$/);
    return match && chooseRewardBody[index + 1] === "jr z, .loop"
      ? [match[1]]
      : [];
  });
  if (
    excludedRewardItems.length !== 1 ||
    !rewardCandidates.includes(excludedRewardItems[0])
  ) {
    throw new Error(
      "Battle Tower reward exclusion must name exactly one candidate item",
    );
  }
  const giveRewardStart = battleTowerRoutineLines.indexOf(
    "BattleTower_GiveReward:",
  );
  const wonStateStart = battleTowerRoutineLines.indexOf(
    "BattleTowerAction_1C:",
  );
  if (giveRewardStart < 0 || wonStateStart <= giveRewardStart) {
    throw new Error("Could not parse BattleTower_GiveReward");
  }
  const rewardFailureSentinel = battleTowerRoutineLines
    .slice(giveRewardStart + 1, wonStateStart)
    .flatMap((line) => line.match(/^ld\s+a,\s+([A-Z0-9_]+)$/)?.[1] ?? [])
    .at(-1);
    if (!rewardFailureSentinel) {
      throw new Error("Could not parse Battle Tower reward failure sentinel");
    }
    const rewardFailureSentinelValue = itemOrder.indexOf(rewardFailureSentinel);
    if (rewardFailureSentinelValue < 0) {
      throw new Error(
        `Battle Tower reward failure sentinel ${rewardFailureSentinel} is missing from item_constants.asm`,
      );
    }
    const rewardItemValues = Object.fromEntries([
      ...rewardCandidates.map((item, index) => [
        item,
        minimumRewardIndex + index,
      ]),
      [rewardFailureSentinel, rewardFailureSentinelValue],
    ]);
    return {
      rewardCandidates,
      excludedRewardItems,
      rewardQuantity,
      rewardFailureSentinel,
      rewardItemValues,
    };
  };
  const directSpecies: string[] = [];
  let rangeStart: string | null = null;
  const routineLines = readAsmLines(path.join("mobile", "mobile_46.asm")).map(
    (raw) => stripAsmComment(raw).trim(),
  );
  for (let index = 0; index < routineLines.length; index += 1) {
    const line = routineLines[index];
    if (line === "BattleTower_UbersCheck:") {
      const body = routineLines.slice(index + 1);
      for (let bodyIndex = 0; bodyIndex < body.length; bodyIndex += 1) {
        const bodyLine = body[bodyIndex];
        if (bodyLine === ".uber") {
          break;
        }
        const cpMatch = bodyLine.match(/^cp\s+([A-Z0-9_]+)$/);
        if (!cpMatch) {
          continue;
        }
        const species = cpMatch[1];
        const nextLine = body
          .slice(bodyIndex + 1)
          .find((candidate) => candidate.length > 0);
        if (nextLine === "jr c, .next") {
          rangeStart = species;
          continue;
        }
        if (species !== "NUM_POKEMON") {
          directSpecies.push(species);
        }
      }
      continue;
    }
  }
  if (!rangeStart) {
    throw new Error(
      "Could not parse Battle Tower banned species range from mobile/mobile_46.asm",
    );
  }
  const pokemonOrder = parsePokemonConstantOrder();
  const rangeStartIndex = pokemonOrder.indexOf(rangeStart);
  if (rangeStartIndex < 0) {
    throw new Error(
      `Battle Tower banned species range starts with unknown species '${rangeStart}'`,
    );
  }
  const bannedSpeciesList = [
    ...directSpecies,
    ...pokemonOrder.slice(rangeStartIndex),
  ];
  if (!bannedSpeciesList.length) {
    throw new Error(
      "No Battle Tower banned species were exported from mobile/mobile_46.asm",
    );
  }
  const bannedSpecies = Object.fromEntries(
    bannedSpeciesList.map((species) => [species, {}]),
  );
  const ruleLines = readAsmLines(
    path.join("engine", "events", "battle_tower", "rules.asm"),
  ).map((raw) => stripAsmComment(raw).trim());
  const checkStart = ruleLines.indexOf("_CheckForBattleTowerRules:");
  if (checkStart < 0) {
    throw new Error(
      "Could not find _CheckForBattleTowerRules in battle_tower/rules.asm",
    );
  }
  const requiredPartyCountLine = ruleLines
    .slice(checkStart)
    .find((line) => /^ld\s+\[hl\],\s*'[0-9]'$/.test(line));
  const requiredPartyCountMatch = requiredPartyCountLine?.match(
    /^ld\s+\[hl\],\s*'([0-9])'$/,
  );
  if (!requiredPartyCountMatch) {
    throw new Error("Could not parse Battle Tower required party count");
  }
  const rulePartyCount = Number(requiredPartyCountMatch[1]);
  if (rulePartyCount !== requiredPartyCount) {
    throw new Error(
      `Battle Tower rule party count ${rulePartyCount} does not match BATTLETOWER_PARTY_LENGTH ${requiredPartyCount}`,
    );
  }
  const levelCheckStart = routineLines.indexOf("BattleTower_LevelCheck:");
  if (levelCheckStart < 0) {
    throw new Error(
      "Could not find BattleTower_LevelCheck in mobile/mobile_46.asm",
    );
  }
  const levelGroupSizeLine = routineLines
    .slice(levelCheckStart)
    .find((line) => /^ld\s+c,\s*[0-9]+$/.test(line));
  const levelGroupSizeMatch = levelGroupSizeLine?.match(/^ld\s+c,\s*([0-9]+)$/);
  if (!levelGroupSizeMatch) {
    throw new Error("Could not parse Battle Tower level group size");
  }
  const levelGroupSize = Number(levelGroupSizeMatch[1]);
  if (levelGroupSize < 1 || levelGroupSize > 100) {
    throw new Error(
      `Battle Tower level group size ${levelGroupSize} is outside Pokemon level range.`,
    );
  }
  const levelMenuStart = routineLines.indexOf("Strings_L10ToL100:");
  if (levelMenuStart < 0) {
    throw new Error("Could not find Battle Tower full level menu");
  }
  const levelGroups: number[] = [];
  for (const line of routineLines.slice(levelMenuStart + 1)) {
    if (!line.startsWith("db ")) {
      break;
    }
    if (line.includes("CANCEL")) {
      break;
    }
    const match = line.match(/^db\s+" L:([0-9]+)\s*@@"/);
    if (!match) {
      throw new Error(
        `Could not parse Battle Tower level group menu entry '${line}'`,
      );
    }
    const displayedLevel = Number(match[1]);
    if (displayedLevel < 1 || displayedLevel > 100) {
      throw new Error(
        `Battle Tower level menu entry ${displayedLevel} is outside Pokemon level range.`,
      );
    }
    levelGroups.push(displayedLevel / levelGroupSize);
  }
  if (
    !levelGroups.length ||
    !levelGroups.every((group) => Number.isInteger(group))
  ) {
    throw new Error(
      "Battle Tower level menu did not export exact integer level groups",
    );
  }
  const {
    rewardCandidates,
    excludedRewardItems,
    rewardQuantity,
    rewardFailureSentinel,
    rewardItemValues,
  } = parseBattleTowerRewards();
  const textPointersStart = ruleLines.indexOf(".TextPointers:", checkStart);
  if (textPointersStart < 0) {
    throw new Error("Could not parse Battle Tower rule text pointer table");
  }
  const textLabels = ruleLines
    .slice(textPointersStart + 1)
    .filter((line) => line.startsWith("dw "))
    .map((line) => line.replace(/^dw\s+/, ""))
    .slice(0, 5);
  if (textLabels.length !== 5) {
    throw new Error(
      "Battle Tower rule text pointer table must contain five text labels",
    );
  }
  const [
    ,
    partyCountFailureText,
    duplicateSpeciesFailureText,
    duplicateHeldItemFailureText,
    eggFailureText,
  ] = textLabels;
  const definitions = exportBattleTowerDefinitions();
  const rules = {
    bannedSpecies,
    requiredPartyCount,
    challengeStreakLength,
    rewardCandidates,
    excludedRewardItems,
    rewardQuantity,
    rewardFailureSentinel,
    rewardItemValues,
    minimumLevelGroup: Math.min(...levelGroups),
    maximumLevelGroup: Math.max(...levelGroups),
    levelGroupSize,
    partyCountFailureText,
    duplicateSpeciesFailureText,
    duplicateHeldItemFailureText,
    eggFailureText,
    ...definitions,
  };
  writeJsonToTargets("battle_tower_rules.json", rules, { indent: 2 });
  return rules;
};

export const exportOakRatings = (): OakRatingEntry[] => {
  const entries = readAsmLines(
    path.join("data", "events", "pokedex_ratings.asm"),
  )
    .map((raw) => stripAsmComment(raw).trim())
    .map((line) =>
      line.match(/^rating\s+([0-9]+),\s*([A-Z0-9_]+),\s*([A-Za-z0-9_]+)$/),
    )
    .filter((match): match is RegExpMatchArray => Boolean(match))
    .map((match) => ({
      caughtCountLimit: Number(match[1]),
      fanfare: match[2],
      textLabel: match[3],
    }));
  if (!entries.length) {
    throw new Error(
      "No Oak rating entries were exported from data/events/pokedex_ratings.asm",
    );
  }
  for (let index = 1; index < entries.length; index += 1) {
    if (
      entries[index].caughtCountLimit <= entries[index - 1].caughtCountLimit
    ) {
      throw new Error(
        "Oak rating caught-count limits must be strictly increasing",
      );
    }
  }
  for (const entry of entries) {
    if (entry.caughtCountLimit > 0xff) {
      throw new Error(
        `Oak rating caught-count limit ${entry.caughtCountLimit} is outside byte range.`,
      );
    }
  }
  writeJsonToTargets("oak_ratings.json", entries, { indent: 2 });
  return entries;
};

const parseOddEggOriginalTrainerName = (): string => {
  for (const raw of readAsmLines(
    path.join("engine", "events", "odd_egg.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    const match = line.match(/^dname\s+"([^"]+)",\s*MON_NAME_LENGTH\s*\+\s*1$/);
    if (match) {
      return match[1];
    }
  }
  throw new Error(
    "Could not parse Odd Egg original trainer name from engine/events/odd_egg.asm",
  );
};

const parseOddEggProbabilities = (lines: string[]): number[] => {
  const probabilities: number[] = [];
  let inTable = false;
  for (const line of lines) {
    if (line === "OddEggProbabilities:") {
      inTable = true;
      continue;
    }
    if (!inTable) {
      continue;
    }
    if (line === "OddEggs:") {
      break;
    }
    const match = line.match(/^odd_egg_prob\s+([0-9]+)$/);
    if (match) {
      const probability = Number.parseInt(match[1], 10);
      if (probability > 100) {
        throw new Error(
          `Odd Egg probability ${probability} is outside percent range.`,
        );
      }
      probabilities.push(probability);
    }
  }
  if (!probabilities.length) {
    throw new Error(
      "No Odd Egg probabilities were exported from data/events/odd_eggs.asm",
    );
  }
  return probabilities;
};

const parseOddEggMoveList = (line: string): string[] => {
  const match = line.match(/^db\s+(.+)$/);
  if (!match) {
    throw new Error(`Malformed Odd Egg move row: ${line}`);
  }
  const moves = match[1]
    .split(",")
    .map((part) => part.trim())
    .filter((move) => move !== "0");
  if (!moves.length) {
    throw new Error("Odd Egg move list must not be empty.");
  }
  if (moves.length > 4) {
    throw new Error(
      `Odd Egg move list has ${moves.length} moves, exceeding party move limit.`,
    );
  }
  return moves;
};

const parseOddEggDvs = (line: string): [number, number, number, number] => {
  const match = line.match(
    /^dn\s+([0-9]+),\s*([0-9]+),\s*([0-9]+),\s*([0-9]+)$/,
  );
  if (!match) {
    throw new Error(`Malformed Odd Egg DVs row: ${line}`);
  }
  const dvs: [number, number, number, number] = [
    Number.parseInt(match[1], 10),
    Number.parseInt(match[2], 10),
    Number.parseInt(match[3], 10),
    Number.parseInt(match[4], 10),
  ];
  for (const value of dvs) {
    if (value > 15) {
      throw new Error(`Odd Egg DV '${value}' is outside nibble range.`);
    }
  }
  return dvs;
};

const parseOddEggDefinitionsFromLines = (
  lines: string[],
  probabilities: number[],
  originalTrainerName: string,
): OddEggDefinition[] => {
  const definitions: OddEggDefinition[] = [];
  const start = lines.indexOf("OddEggs:");
  if (start < 0) {
    throw new Error("Could not find OddEggs table in data/events/odd_eggs.asm");
  }
  let index = start + 1;
  while (index < lines.length) {
    const line = lines[index];
    if (line.startsWith("assert_table_length ")) {
      break;
    }
    if (!line || line.startsWith("table_width ")) {
      index += 1;
      continue;
    }
    const speciesMatch = line.match(/^db\s+([A-Z0-9_]+)$/);
    if (!speciesMatch) {
      throw new Error(`Malformed Odd Egg species row: ${line}`);
    }
    const species = speciesMatch[1];
    index += 1;
    if (lines[index] !== "db NO_ITEM") {
      throw new Error(`Odd Egg ${species} must declare NO_ITEM before moves`);
    }
    index += 1;
    const moves = parseOddEggMoveList(lines[index]);
    index += 1;
    const otMatch = lines[index].match(/^dw\s+([0-9]+)$/);
    if (!otMatch) {
      throw new Error(`Malformed Odd Egg OT id row: ${lines[index]}`);
    }
    const originalTrainerId = Number.parseInt(otMatch[1], 10);
    if (originalTrainerId > 0xffff) {
      throw new Error(
        `Odd Egg ${species} original trainer id ${originalTrainerId} is outside word range.`,
      );
    }
    index += 1;
    const expMatch = lines[index].match(/^bigdt\s+([0-9]+)$/);
    if (!expMatch) {
      throw new Error(`Malformed Odd Egg experience row: ${lines[index]}`);
    }
    const experience = Number.parseInt(expMatch[1], 10);
    index += 1;
    while (index < lines.length && !lines[index].startsWith("dn ")) {
      index += 1;
    }
    const dvs = parseOddEggDvs(lines[index]);
    index += 1;
    if (!lines[index].startsWith("db ")) {
      throw new Error(`Malformed Odd Egg PP row: ${lines[index]}`);
    }
    index += 1;
    const hatchMatch = lines[index].match(/^db\s+([0-9]+)$/);
    if (!hatchMatch) {
      throw new Error(`Malformed Odd Egg hatch cycles row: ${lines[index]}`);
    }
    const hatchCycles = Number.parseInt(hatchMatch[1], 10);
    if (hatchCycles > 0xff) {
      throw new Error(
        `Odd Egg ${species} hatch cycles ${hatchCycles} is outside byte range.`,
      );
    }
    index += 1;
    if (!/^db\s+0,\s*0,\s*0$/.test(lines[index])) {
      throw new Error(
        `Malformed Odd Egg Pokerus/caught data row: ${lines[index]}`,
      );
    }
    index += 1;
    const levelMatch = lines[index].match(/^db\s+([0-9]+)$/);
    if (!levelMatch) {
      throw new Error(`Malformed Odd Egg level row: ${lines[index]}`);
    }
    const level = Number.parseInt(levelMatch[1], 10);
    if (level < 1 || level > 100) {
      throw new Error(
        `Odd Egg ${species} level ${level} is outside Pokemon level range.`,
      );
    }
    index += 1;
    while (index < lines.length && !lines[index].startsWith("dname ")) {
      index += 1;
    }
    const nicknameMatch = lines[index].match(
      /^dname\s+"([^"]+)",\s*MON_NAME_LENGTH$/,
    );
    if (!nicknameMatch) {
      throw new Error(`Malformed Odd Egg nickname row: ${lines[index]}`);
    }
    definitions.push({
      species,
      moves,
      originalTrainerId,
      dvs,
      probability: probabilities[definitions.length],
      level,
      experience,
      hatchCycles,
      nickname: nicknameMatch[1],
      originalTrainerName,
    });
    index += 1;
  }
  if (definitions.length !== probabilities.length) {
    throw new Error(
      `Odd Egg definition count ${definitions.length} does not match probability count ${probabilities.length}`,
    );
  }
  return definitions;
};

export const exportOddEggDefinitions = (): OddEggDefinition[] => {
  const lines = readAsmLines(path.join("data", "events", "odd_eggs.asm"))
    .map((raw) => stripAsmComment(raw).trim())
    .filter((line) => line.length > 0);
  const definitions = parseOddEggDefinitionsFromLines(
    lines,
    parseOddEggProbabilities(lines),
    parseOddEggOriginalTrainerName(),
  );
  const totalProbability = definitions.reduce(
    (total, definition) => total + definition.probability,
    0,
  );
  if (totalProbability !== 100) {
    throw new Error(
      `Odd Egg probabilities sum to ${totalProbability}%, not 100%.`,
    );
  }
  writeJsonToTargets("odd_egg_definitions.json", definitions, { indent: 2 });
  return definitions;
};

export const exportMagikarpLengths = (): MagikarpLengthEntry[] => {
  const entries: MagikarpLengthEntry[] = [];
  let inTable = false;
  for (const raw of readAsmLines(
    path.join("data", "events", "magikarp_lengths.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    if (line === "MagikarpLengths:") {
      inTable = true;
      continue;
    }
    if (!inTable) {
      continue;
    }
    const match = line.match(/^dwb\s+([0-9]+),\s*([0-9]+)$/);
    if (!match) {
      throw new Error(`Malformed Magikarp length row: ${raw}`);
    }
    const threshold = Number.parseInt(match[1], 10);
    const divisor = Number.parseInt(match[2], 10);
    if (threshold > 0xffff) {
      throw new Error(
        `Magikarp length threshold ${threshold} is outside word range.`,
      );
    }
    if (divisor === 0 || divisor > 0xff) {
      throw new Error(
        `Magikarp length divisor ${divisor} is outside byte divisor range.`,
      );
    }
    entries.push({
      threshold,
      divisor,
    });
  }
  if (!entries.length) {
    throw new Error(
      "No Magikarp length entries were exported from data/events/magikarp_lengths.asm",
    );
  }
  writeJsonToTargets("magikarp_lengths.json", entries, { indent: 2 });
  return entries;
};

const parseHappinessChangeConstants = (): Map<string, number> => {
  const constants = new Map<string, number>();
  let sawHappinessHeader = false;
  let inBlock = false;
  let nextValue = 0;
  for (const raw of readAsmLines(
    path.join("constants", "pokemon_data_constants.asm"),
  )) {
    if (raw.includes("ChangeHappiness")) {
      sawHappinessHeader = true;
      continue;
    }
    const line = stripAsmComment(raw).trim();
    if (sawHappinessHeader && line === "const_def 1") {
      inBlock = true;
      nextValue = 1;
      continue;
    }
    if (!inBlock) {
      continue;
    }
    if (line.startsWith("DEF NUM_HAPPINESS_CHANGES")) {
      break;
    }
    const match = line.match(/^const\s+(HAPPINESS_[A-Z0-9_]+)$/);
    if (match) {
      if (constants.has(match[1])) {
        throw new Error(`Duplicate happiness change constant '${match[1]}'.`);
      }
      constants.set(match[1], nextValue);
      nextValue += 1;
    }
  }
  if (!constants.size) {
    throw new Error(
      "No HAPPINESS_* constants were exported from constants/pokemon_data_constants.asm",
    );
  }
  return constants;
};

const parseSignedDbNumber = (token: string): number => {
  const trimmed = token.trim();
  if (!/^[+-]?[0-9]+$/.test(trimmed)) {
    throw new Error(`Malformed signed db number '${token}'`);
  }
  const value = Number.parseInt(trimmed, 10);
  if (value < -128 || value > 127) {
    throw new Error(`Signed db number '${token}' is outside signed byte range`);
  }
  return value;
};

const parsePercentExpression = (token: string): number => {
  const trimmed = token.trim();
  if (trimmed === "-1") {
    return 255;
  }
  const match = trimmed.match(/^([0-9]+)\s+percent(?:\s*([+-])\s*([0-9]+))?$/);
  if (!match) {
    if (/^[0-9]+$/.test(trimmed)) {
      return Number.parseInt(trimmed, 10);
    }
    throw new Error(`Malformed percent expression '${token}'`);
  }
  let value = Math.floor((Number.parseInt(match[1], 10) * 0xff) / 100);
  if (match[2] && match[3]) {
    value +=
      match[2] === "+"
        ? Number.parseInt(match[3], 10)
        : -Number.parseInt(match[3], 10);
  }
  if (value < 0 || value > 255) {
    throw new Error(
      `Percent expression '${token}' resolved outside byte range`,
    );
  }
  return value;
};

export const exportHappinessData = (): HappinessData => {
  const constants = parseHappinessChangeConstants();
  const changes: Record<string, HappinessChangeEntry> = {};
  for (const raw of readAsmLines(
    path.join("data", "events", "happiness_changes.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    if (
      !line ||
      line === "HappinessChanges:" ||
      line.startsWith("table_width ")
    ) {
      continue;
    }
    if (line.startsWith("assert_table_length ")) {
      break;
    }
    const match = line.match(
      /^db\s+([+-]?[0-9]+),\s*([+-]?[0-9]+),\s*([+-]?[0-9]+)$/,
    );
    if (!match) {
      throw new Error(`Malformed happiness change row: ${raw}`);
    }
    const index = Object.keys(changes).length + 1;
    const code = [...constants.entries()].find(
      ([, value]) => value === index,
    )?.[0];
    if (!code) {
      throw new Error(
        `Happiness change row ${index} has no matching HAPPINESS_* constant`,
      );
    }
    changes[String(index)] = {
      code,
      low: parseSignedDbNumber(match[1]),
      mid: parseSignedDbNumber(match[2]),
      high: parseSignedDbNumber(match[3]),
    };
  }
  if (Object.keys(changes).length !== constants.size) {
    throw new Error(
      `Happiness change row count ${Object.keys(changes).length} does not match constants ${constants.size}`,
    );
  }

  const labelToRoutine: Record<string, string> = {
    HappinessData_OlderHaircutBrother: "OlderHaircutBrother",
    HappinessData_YoungerHaircutBrother: "YoungerHaircutBrother",
    HappinessData_DaisysGrooming: "DaisysGrooming",
  };
  const services: Record<string, HappinessServiceOutcome[]> = {};
  let current: HappinessServiceOutcome[] | null = null;
  for (const raw of readAsmLines(
    path.join("data", "events", "happiness_probabilities.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    const labelMatch = line.match(/^(HappinessData_[A-Za-z0-9_]+):$/);
    if (labelMatch) {
      const routine = labelToRoutine[labelMatch[1]];
      if (!routine) {
        throw new Error(
          `Unknown happiness probability table '${labelMatch[1]}'`,
        );
      }
      if (Object.prototype.hasOwnProperty.call(services, routine)) {
        throw new Error(`Duplicate happiness probability table '${routine}'.`);
      }
      current = [];
      services[routine] = current;
      continue;
    }
    if (!current) {
      continue;
    }
    const rowMatch = line.match(
      /^db\s+(.+?),\s*([0-9]+),\s*(HAPPINESS_[A-Z0-9_]+)$/,
    );
    if (!rowMatch) {
      throw new Error(`Malformed happiness probability row: ${raw}`);
    }
    const changeCode = constants.get(rowMatch[3]);
    if (!changeCode) {
      throw new Error(
        `Happiness probability row references unknown ${rowMatch[3]}`,
      );
    }
    const scriptValue = Number.parseInt(rowMatch[2], 10);
    if (scriptValue > 0xff) {
      throw new Error(
        `Happiness probability script value ${scriptValue} is outside byte range.`,
      );
    }
    current.push({
      rollWeight: parsePercentExpression(rowMatch[1]),
      scriptValue,
      changeCode,
    });
  }
  if (
    Object.keys(services).length !== Object.keys(labelToRoutine).length ||
    Object.values(services).some((outcomes) => !outcomes.length)
  ) {
    throw new Error(
      "Could not export complete happiness service probability tables",
    );
  }
  const payload = { changes, services };
  writeJsonToTargets("happiness_data.json", payload, { indent: 2 });
  return payload;
};

export const exportEncounterSlotTables = (): EncounterSlotTables => {
  const labelToKey: Record<string, keyof EncounterSlotTableMap> = {
    GrassMonProbTable: "grass",
    WaterMonProbTable: "water",
  };
  const tables: EncounterSlotTableMap = { grass: [], water: [] };
  let current: keyof EncounterSlotTableMap | null = null;
  const lastThresholds: Record<keyof EncounterSlotTableMap, number> = {
    grass: 0,
    water: 0,
  };
  const seenSlots: Record<keyof EncounterSlotTableMap, Set<number>> = {
    grass: new Set(),
    water: new Set(),
  };
  for (const raw of readAsmLines(
    path.join("data", "wild", "probabilities.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    const label = line.match(/^([A-Za-z0-9_]+):$/);
    if (label) {
      current = labelToKey[label[1]] ?? null;
      continue;
    }
    if (
      !current ||
      line.startsWith("table_width ") ||
      line.startsWith("assert_table_length ")
    ) {
      continue;
    }
    const row = line.match(/^mon_prob\s+([0-9]+),\s*([0-9]+)$/);
    if (!row) {
      throw new Error(`Malformed encounter slot probability row: ${raw}`);
    }
    const threshold = Number.parseInt(row[1], 10);
    const slot = Number.parseInt(row[2], 10);
    if (threshold < 1 || threshold > 100) {
      throw new Error(
        `Encounter slot threshold ${threshold} is outside 1..=100`,
      );
    }
    if (slot > 0xff) {
      throw new Error(`Encounter slot ${slot} is outside byte range.`);
    }
    if (threshold <= lastThresholds[current]) {
      throw new Error(
        `Encounter slot table ${current} threshold ${threshold} must be greater than ${lastThresholds[current]}`,
      );
    }
    if (seenSlots[current].has(slot)) {
      throw new Error(`Encounter slot table ${current} repeats slot ${slot}.`);
    }
    lastThresholds[current] = threshold;
    seenSlots[current].add(slot);
    tables[current].push({ threshold, slot });
  }
  if (!tables.grass.length || !tables.water.length) {
    throw new Error(
      "Could not export complete encounter slot probability tables",
    );
  }
  for (const key of Object.keys(tables) as Array<keyof EncounterSlotTableMap>) {
    const finalThreshold = tables[key].at(-1)?.threshold;
    if (finalThreshold !== 100) {
      throw new Error(
        `Encounter slot table ${key} must end at threshold 100, found ${finalThreshold}.`,
      );
    }
  }
  const payload = { tables };
  writeJsonToTargets("encounter_slot_tables.json", payload, { indent: 2 });
  return payload;
};

const parseBattleStatMultiplierRows = (
  relativePath: string,
): BattleStatMultiplier[] => {
  const rows: BattleStatMultiplier[] = [];
  for (const raw of readAsmLines(relativePath)) {
    const line = stripAsmComment(raw).trim();
    if (!line || line.endsWith(":")) {
      continue;
    }
    const row = line.match(/^db\s+([0-9]+),\s*([0-9]+)$/);
    if (!row) {
      throw new Error(
        `Malformed battle stat multiplier row in ${relativePath}: ${raw}`,
      );
    }
    const numerator = Number.parseInt(row[1], 10);
    const denominator = Number.parseInt(row[2], 10);
    if (numerator > 255) {
      throw new Error(
        `Battle stat multiplier in ${relativePath} has numerator ${numerator} outside byte range`,
      );
    }
    if (denominator <= 0) {
      throw new Error(
        `Battle stat multiplier in ${relativePath} has invalid denominator ${denominator}`,
      );
    }
    if (denominator > 255) {
      throw new Error(
        `Battle stat multiplier in ${relativePath} has denominator ${denominator} outside byte range`,
      );
    }
    rows.push({ numerator, denominator });
  }
  if (rows.length !== 13) {
    throw new Error(
      `Expected 13 battle stat multiplier rows in ${relativePath}, found ${rows.length}`,
    );
  }
  return rows;
};

export const exportBattleStatMultipliers = (): BattleStatMultiplierTables => {
  const payload = {
    stat: parseBattleStatMultiplierRows(
      path.join("data", "battle", "stat_multipliers.asm"),
    ),
    accuracy: parseBattleStatMultiplierRows(
      path.join("data", "battle", "accuracy_multipliers.asm"),
    ),
  };
  writeJsonToTargets("battle_stat_multipliers.json", payload, { indent: 2 });
  return payload;
};

export const exportCaptureWobbleProbabilities =
  (): CaptureWobbleProbability[] => {
    const rows: CaptureWobbleProbability[] = [];
    const seenCatchRates = new Set<number>();
    let lastCatchRate = 0;
    let inTable = false;
    for (const raw of readAsmLines(
      path.join("data", "battle", "wobble_probabilities.asm"),
    )) {
      const line = stripAsmComment(raw).trim();
      if (!line) {
        continue;
      }
      if (line === "WobbleProbabilities:") {
        inTable = true;
        continue;
      }
      if (!inTable) {
        continue;
      }
      const row = line.match(/^db\s+([0-9]+),\s*([0-9]+)$/);
      if (!row) {
        throw new Error(`Malformed capture wobble probability row: ${raw}`);
      }
      const catchRate = Number.parseInt(row[1], 10);
      const chance = Number.parseInt(row[2], 10);
      if (catchRate < 1 || catchRate > 255) {
        throw new Error(
          `Capture wobble catch rate ${catchRate} is outside 1..=255`,
        );
      }
      if (seenCatchRates.has(catchRate)) {
        throw new Error(`Duplicate capture wobble catch rate ${catchRate}.`);
      }
      if (catchRate <= lastCatchRate) {
        throw new Error(
          `Capture wobble catch rate ${catchRate} must be greater than ${lastCatchRate}.`,
        );
      }
      if (chance > 255) {
        throw new Error(`Capture wobble chance ${chance} is outside 0..=255`);
      }
      seenCatchRates.add(catchRate);
      lastCatchRate = catchRate;
      rows.push({ catch_rate: catchRate, chance });
    }
    if (!rows.length) {
      throw new Error("Could not export capture wobble probabilities");
    }
    writeJsonToTargets("capture_wobble_probabilities.json", rows, {
      indent: 2,
    });
    return rows;
  };

const weatherEffectivenessMultiplier = (
  token: string,
): BattleStatMultiplier => {
  switch (token) {
    case "MORE_EFFECTIVE":
      return { numerator: 3, denominator: 2 };
    case "NOT_VERY_EFFECTIVE":
      return { numerator: 1, denominator: 2 };
    default:
      throw new Error(`Unknown weather effectiveness token '${token}'`);
  }
};

const typeEffectivenessMultiplier = (token: string): BattleStatMultiplier => {
  switch (token) {
    case "SUPER_EFFECTIVE":
      return { numerator: 2, denominator: 1 };
    case "NOT_VERY_EFFECTIVE":
      return { numerator: 1, denominator: 2 };
    case "NO_EFFECT":
      return { numerator: 0, denominator: 1 };
    default:
      throw new Error(`Unknown type effectiveness token '${token}'`);
  }
};

export const exportTypeEffectivenessTable = (): TypeEffectivenessTable => {
  const sparseMatchups: TypeEffectivenessEntry[] = [];
  const foresightMatchups: TypeEffectivenessEntry[] = [];
  let inTable = false;
  let section: "normal" | "foresight" = "normal";
  for (const raw of readAsmLines(
    path.join("data", "types", "type_matchups.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    if (line === "TypeMatchups:") {
      inTable = true;
      continue;
    }
    if (!inTable) {
      continue;
    }
    if (line === "db -2") {
      section = "foresight";
      continue;
    }
    if (line === "db -1") {
      inTable = false;
      continue;
    }
    const row = line.match(
      /^db\s+([A-Z0-9_]+),\s*([A-Z0-9_]+),\s*([A-Z0-9_]+)$/,
    );
    if (!row) {
      throw new Error(`Malformed type effectiveness row: ${raw}`);
    }
    const [, attacker, defender, effectiveness] = row;
    const entry = {
      attacker,
      defender,
      multiplier: typeEffectivenessMultiplier(effectiveness),
    };
    if (section === "normal") {
      sparseMatchups.push(entry);
    } else {
      foresightMatchups.push(entry);
    }
  }
  if (!sparseMatchups.length || !foresightMatchups.length) {
    throw new Error("Could not export complete type effectiveness table");
  }
  const assertUniqueTypeMatchups = (
    entries: TypeEffectivenessEntry[],
    tableName: string,
  ): void => {
    const pairs = new Set<string>();
    for (const entry of entries) {
      const pair = `${entry.attacker}\u0000${entry.defender}`;
      if (pairs.has(pair)) {
        throw new Error(
          `Duplicate ${tableName} type effectiveness matchup '${entry.attacker}/${entry.defender}'.`,
        );
      }
      pairs.add(pair);
    }
  };
  assertUniqueTypeMatchups(sparseMatchups, "normal");
  assertUniqueTypeMatchups(foresightMatchups, "foresight");
  const categories = typeCategoriesFromAsm();
  const types = [...categories.physical, ...categories.special];
  const typeSet = new Set(types);
  for (const entry of [...sparseMatchups, ...foresightMatchups]) {
    if (!typeSet.has(entry.attacker)) {
      throw new Error(
        `Type effectiveness matchup references unknown attacker type '${entry.attacker}'.`,
      );
    }
    if (!typeSet.has(entry.defender)) {
      throw new Error(
        `Type effectiveness matchup references unknown defender type '${entry.defender}'.`,
      );
    }
  }
  const payload = {
    matchups: sparseMatchups,
    foresight_matchups: foresightMatchups,
  };
  writeJsonToTargets("type_effectiveness.json", payload, { indent: 2 });
  return payload;
};

export const exportTypeCategories = (): TypeCategories => {
  const payload = typeCategoriesFromAsm();
  writeJsonToTargets("type_categories.json", payload, { indent: 2 });
  return payload;
};

const moveEffectSchemaId = (asmEffect: string): string => {
  if (!asmEffect.startsWith("EFFECT_")) {
    throw new Error(
      `Move effect priority id '${asmEffect}' must use an exact EFFECT_ token`,
    );
  }
  const effect = asmEffect.slice("EFFECT_".length);
  const statChange = effect.match(/^([A-Z_]+)_(UP|DOWN)_?(\d)?_?(HIT)?$/);
  if (!statChange) return effect;
  const [, stat, direction, amount, hit] = statChange;
  const canonicalStat =
    stat === "SP_ATK" ? "SPECIAL_ATTACK" : stat === "SP_DEF" ? "SPECIAL_DEFENSE" : stat;
  return `${canonicalStat}_${direction}${amount ? `_${amount}` : ""}${hit ? "_HIT" : ""}`;
};

export const exportMovePriorityTable = (
  movesData: Record<string, Move> = {},
): MovePriorityTable => {
  let basePriority: number | null = null;
  for (const raw of readAsmLines(
    path.join("constants", "battle_constants.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    const row = line.match(/^DEF\s+BASE_PRIORITY\s+EQU\s+([0-9]+)$/);
    if (row) {
      basePriority = Number.parseInt(row[1], 10);
      break;
    }
  }
  if (basePriority === null) {
    throw new Error("Could not export BASE_PRIORITY");
  }
  if (basePriority > 0xff) {
    throw new Error(`BASE_PRIORITY ${basePriority} is outside byte range.`);
  }

  const sparseEffectPriorities: MoveEffectPriority[] = [];
  let inTable = false;
  for (const raw of readAsmLines(
    path.join("data", "moves", "effects_priorities.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    if (line === "MoveEffectPriorities:") {
      inTable = true;
      continue;
    }
    if (!inTable) {
      continue;
    }
    if (line === "db -1") {
      inTable = false;
      continue;
    }
    const row = line.match(/^db\s+([A-Z0-9_]+),\s*([0-9]+)$/);
    if (!row) {
      throw new Error(`Malformed move priority row: ${raw}`);
    }
    const priority = Number.parseInt(row[2], 10);
    if (priority > 0xff) {
      throw new Error(
        `Move effect priority '${row[1]}' value ${priority} is outside byte range.`,
      );
    }
    sparseEffectPriorities.push({
      move_effect: moveEffectSchemaId(row[1]),
      priority,
    });
  }
  if (!sparseEffectPriorities.length) {
    throw new Error("Could not export move effect priorities");
  }
  const sparseByEffect = new Map<string, number>();
  for (const entry of sparseEffectPriorities) {
    if (sparseByEffect.has(entry.move_effect)) {
      throw new Error(`Duplicate move effect priority '${entry.move_effect}'.`);
    }
    sparseByEffect.set(entry.move_effect, entry.priority);
  }
  const effects = [
    ...new Set(
      Object.values(movesData)
        .map((move) => move.effect)
        .filter(Boolean),
    ),
  ].sort();
  const effectPriorityEntries =
    effects.length > 0
      ? effects.map((moveEffect) => ({
          move_effect: moveEffect,
          priority: sparseByEffect.get(moveEffect) ?? basePriority,
        }))
      : sparseEffectPriorities;
  const effectPriorities = Object.fromEntries(
    effectPriorityEntries.map((entry) => [entry.move_effect, entry.priority]),
  );

  const core = readAsmLines(path.join("engine", "battle", "core.asm"))
    .map((raw) => stripAsmComment(raw).trim())
    .filter(Boolean);
  const vitalThrowIndex = core.findIndex((line) => line === "cp VITAL_THROW");
  if (
    vitalThrowIndex < 0 ||
    core[vitalThrowIndex + 1] !== "ld a, 0" ||
    core[vitalThrowIndex + 2] !== "ret z"
  ) {
    throw new Error(
      "Could not export VITAL_THROW priority override from GetMovePriority",
    );
  }

  const payload = {
    base_priority: basePriority,
    effect_priorities: effectPriorities,
    move_priorities: [{ move: "VITAL_THROW", priority: 0 }],
  };
  writeJsonToTargets("move_priorities.json", payload, { indent: 2 });
  return payload;
};

export const exportWeatherModifiers = (): WeatherModifiers => {
  const typeModifiers: Record<string, Record<string, TypeMultiplier>> = {};
  const moveEffectModifiers: Record<
    string,
    Record<string, TypeMultiplier>
  > = {};
  let section: "type" | "move_effect" | null = null;
  for (const raw of readAsmLines(
    path.join("data", "battle", "weather_modifiers.asm"),
  )) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    if (line === "WeatherTypeModifiers:") {
      section = "type";
      continue;
    }
    if (line === "WeatherMoveModifiers:") {
      section = "move_effect";
      continue;
    }
    if (line === "db -1") {
      section = null;
      continue;
    }
    if (!section) {
      throw new Error(`Weather modifier row outside known section: ${raw}`);
    }
    const row = line.match(
      /^db\s+([A-Z0-9_]+),\s*([A-Z0-9_]+),\s*([A-Z0-9_]+)$/,
    );
    if (!row) {
      throw new Error(`Malformed weather modifier row: ${raw}`);
    }
    const [, weather, target, effectiveness] = row;
    const multiplier = weatherEffectivenessMultiplier(effectiveness);
    if (section === "type") {
      if (
        Object.prototype.hasOwnProperty.call(
          typeModifiers[weather] ?? {},
          target,
        )
      ) {
        throw new Error(
          `Duplicate weather type modifier '${weather}/${target}'.`,
        );
      }
      typeModifiers[weather] = {
        ...(typeModifiers[weather] ?? {}),
        [target]: multiplier,
      };
    } else {
      const moveEffect = moveEffectSchemaId(target);
      if (
        Object.prototype.hasOwnProperty.call(
          moveEffectModifiers[weather] ?? {},
          moveEffect,
        )
      ) {
        throw new Error(
          `Duplicate weather move-effect modifier '${weather}/${moveEffect}'.`,
        );
      }
      moveEffectModifiers[weather] = {
        ...(moveEffectModifiers[weather] ?? {}),
        [moveEffect]: multiplier,
      };
    }
  }
  if (
    !Object.keys(typeModifiers).length ||
    !Object.keys(moveEffectModifiers).length
  ) {
    throw new Error("Could not export complete weather modifier tables");
  }
  const payload = {
    type_modifiers: typeModifiers,
    move_effect_modifiers: moveEffectModifiers,
  };
  writeJsonToTargets("weather_modifiers.json", payload, { indent: 2 });
  return payload;
};

const parseFrontpicAnimNumber = (token: string): number => {
  const cleaned = token.trim();
  if (!cleaned) {
    throw new Error("Missing frontpic animation numeric operand.");
  }
  let value: number;
  if (cleaned.startsWith("$")) {
    value = Number.parseInt(cleaned.slice(1), 16);
    if (!Number.isFinite(value)) {
      throw new Error(`Invalid frontpic animation numeric operand '${token}'.`);
    }
  } else if (/^[+-]?\d+$/.test(cleaned)) {
    value = Number.parseInt(cleaned, 10);
  } else {
    throw new Error(`Invalid frontpic animation numeric operand '${token}'.`);
  }
  if (value < 0 || value > 255) {
    throw new Error(
      `Frontpic animation numeric operand '${token}' is outside byte range`,
    );
  }
  return value;
};

const parseFrontpicAnimScript = (source: string): FrontpicAnimProgram => {
  const commands: FrontpicAnimCommand[] = [];
  for (const rawLine of source.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (!line) {
      continue;
    }
    const parts = line.split(/[\s,]+/).filter(Boolean);
    const opcode = parts[0];
    if (opcode === "frame") {
      if (parts.length !== 3) {
        throw new Error(`Malformed frontpic animation frame row: ${rawLine}`);
      }
      commands.push({
        kind: "frame",
        frame: parseFrontpicAnimNumber(parts[1]),
        duration: parseFrontpicAnimNumber(parts[2]),
      });
      continue;
    }
    if (opcode === "setrepeat") {
      if (parts.length !== 2) {
        throw new Error(
          `Malformed frontpic animation setrepeat row: ${rawLine}`,
        );
      }
      const count = parseFrontpicAnimNumber(parts[1]);
      if (count === 0) {
        throw new Error("Frontpic animation setrepeat count must be nonzero.");
      }
      commands.push({
        kind: "setrepeat",
        count,
      });
      continue;
    }
    if (opcode === "dorepeat") {
      if (parts.length !== 2) {
        throw new Error(
          `Malformed frontpic animation dorepeat row: ${rawLine}`,
        );
      }
      const target = parseFrontpicAnimNumber(parts[1]);
      if (target >= commands.length) {
        throw new Error(
          `Frontpic animation dorepeat target ${target} does not reference an earlier command.`,
        );
      }
      commands.push({
        kind: "dorepeat",
        target,
      });
      continue;
    }
    if (opcode === "endanim") {
      if (parts.length !== 1) {
        throw new Error(`Malformed frontpic animation endanim row: ${rawLine}`);
      }
      commands.push({ kind: "endanim" });
      continue;
    }
    throw new Error(
      `Unknown frontpic animation opcode '${opcode}' in row: ${rawLine}`,
    );
  }
  const endIndex = commands.findIndex((command) => command.kind === "endanim");
  if (endIndex < 0) {
    throw new Error("Frontpic animation program is missing endanim.");
  }
  if (endIndex !== commands.length - 1) {
    throw new Error("Frontpic animation program has commands after endanim.");
  }
  for (const [index, command] of commands.entries()) {
    if (command.kind !== "dorepeat") {
      continue;
    }
    const hasPriorSetrepeat = commands
      .slice(0, index)
      .some((candidate) => candidate.kind === "setrepeat");
    if (!hasPriorSetrepeat) {
      throw new Error("Frontpic animation dorepeat requires setrepeat.");
    }
  }
  for (const [index, command] of commands.entries()) {
    if (command.kind !== "setrepeat") {
      continue;
    }
    const nextSetrepeatIndex = commands
      .slice(index + 1)
      .findIndex((candidate) => candidate.kind === "setrepeat");
    const repeatWindow =
      nextSetrepeatIndex < 0
        ? commands.slice(index + 1)
        : commands.slice(index + 1, index + 1 + nextSetrepeatIndex);
    const hasDorepeat = repeatWindow.some(
      (candidate) => candidate.kind === "dorepeat",
    );
    if (!hasDorepeat && nextSetrepeatIndex < 0) {
      throw new Error("Frontpic animation setrepeat is missing dorepeat.");
    }
  }
  return { commands };
};

export const exportPokemonFrontpicAnimations = (): Record<
  string,
  FrontpicAnimProgram
> => {
  const pokemonGfxDir = path.join(getDisassemblyRoot(), "gfx", "pokemon");
  const entries: Record<string, FrontpicAnimProgram> = {};
  const speciesByFileStem = exactSpeciesIdMap();
  if (fs.existsSync(pokemonGfxDir)) {
    for (const entry of fs
      .readdirSync(pokemonGfxDir, { withFileTypes: true })
      .sort((a, b) => a.name.localeCompare(b.name))) {
      if (!entry.isDirectory()) {
        continue;
      }
      const animPath = path.join(pokemonGfxDir, entry.name, "anim.asm");
      if (!fs.existsSync(animPath)) {
        continue;
      }
      const species = exactSpeciesFromFileStem(
        entry.name,
        speciesByFileStem,
        animPath,
      );
      let program: FrontpicAnimProgram;
      try {
        program = parseFrontpicAnimScript(fs.readFileSync(animPath, "utf8"));
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        throw new Error(
          `Failed to parse frontpic animation for ${species} in ${animPath}: ${message}`,
        );
      }
      if (program.commands.length) {
        entries[species] = program;
      }
    }
  }
  writeJsonToTargets("pokemon_frontpic_anim.json", entries, { indent: 2 });
  return entries;
};

export const exportEncounterMusicModifiers = (): EncounterMusicModifiers => {
  const modifiers: EncounterMusicModifiers = {
    modifiers: {
      MUSIC_POKEMON_MARCH: { numerator: 2, denominator: 1 },
      MUSIC_RUINS_OF_ALPH_RADIO: { numerator: 2, denominator: 1 },
      MUSIC_POKEMON_LULLABY: { numerator: 1, denominator: 2 },
    },
  };
  writeJsonToTargets("encounter_music_modifiers.json", modifiers, {
    indent: 2,
  });
  return modifiers;
};

export const exportRuntimeAssets = (): {
  fleeMons: FleeMonsData;
  encounterMusicModifiers: EncounterMusicModifiers;
  pcStrings: Record<string, string>;
  menuIcons: Record<string, string>;
  pokedexEntries: Record<string, PokedexEntryData>;
  pokemonFrontpicAnimations: Record<string, FrontpicAnimProgram>;
  marts: Record<string, string[]>;
  trainerClassNames: Record<string, string>;
  phoneContacts: Record<string, PhoneContactRecord>;
  permanentPhoneNumbers: Record<string, PermanentPhoneNumberDefinition>;
  specialPhoneCalls: Record<string, SpecialPhoneCallDefinition>;
  npcTrades: Record<string, NpcTradeDefinition>;
  specialRoutines: Record<string, Record<string, never>>;
} => {
  const fleeMons = exportFleeMons();
  const encounterMusicModifiers = exportEncounterMusicModifiers();
  const pcStrings = exportPcStrings();
  const menuIcons = exportMenuIcons();
  const pokedexEntries = exportPokedexEntries();
  const pokemonFrontpicAnimations = exportPokemonFrontpicAnimations();
  const marts = exportMarts();
  const trainerClassNames = exportTrainerClassNames();
  const phoneContacts = exportPhoneContacts(trainerClassNames);
  const permanentPhoneNumbers = exportPermanentPhoneNumbers();
  const specialPhoneCalls = exportSpecialPhoneCalls();
  const npcTrades = exportNpcTrades();
  const specialRoutines = exportSpecialRoutines();
  return {
    fleeMons,
    encounterMusicModifiers,
    pcStrings,
    menuIcons,
    pokedexEntries,
    pokemonFrontpicAnimations,
    marts,
    trainerClassNames,
    phoneContacts,
    permanentPhoneNumbers,
    specialPhoneCalls,
    npcTrades,
    specialRoutines,
  };
};

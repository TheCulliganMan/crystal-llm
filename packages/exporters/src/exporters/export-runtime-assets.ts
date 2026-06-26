import fs from "fs";
import path from "path";
import type { Move } from "@pokecrystal/core/core/models/move";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripAsmComment, writeJsonToTargets } from "./asm-utils";

type FleeMonsData = {
  always: string[];
  often: string[];
  sometimes: string[];
};

export type EncounterMusicModifiers = {
  modifiers: Array<{
    music_id: string;
    numerator: number;
    denominator: number;
  }>;
};

export type RoamingPokemonDefinition = {
  species: string;
  level: number;
  mapGroup: number;
  mapNumber: number;
};

export type BuenaPrizeDefinition = {
  itemId: string;
  cost: number;
};

export type BuenaPasswordCategoryDefinition = {
  id: string;
  categoryType: string;
  points: number;
  options: string[];
};

export type KurtApricornRecipe = {
  apricorn: string;
  ball: string;
};

export type ShuckieGiftDefinition = {
  species: string;
  level: number;
  heldItem: string;
  nickname: string;
  originalTrainerName: string;
  originalTrainerId: number;
  gotTodayEngineFlag: string;
};

export type DratiniMoveSetDefinition = {
  mode: number;
  moves: string[];
};

export type BugContestConfig = {
  parkBalls: number;
  timerMinutes: number;
  timerSeconds: number;
  selectedContestantCount: number;
  contestantFlags: string[];
};

export type BattleTowerRules = {
  bannedSpecies: string[];
  requiredPartyCount: number;
  challengeStreakLength: number;
  minimumLevelGroup: number;
  maximumLevelGroup: number;
  levelGroupSize: number;
  partyCountFailureText: string;
  duplicateSpeciesFailureText: string;
  duplicateHeldItemFailureText: string;
  eggFailureText: string;
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
  changes: HappinessChangeEntry[];
  services: HappinessServiceTable[];
};

export type HappinessChangeEntry = {
  code: string;
  changeCode: number;
  low: number;
  mid: number;
  high: number;
};

export type HappinessServiceTable = {
  routine: string;
  outcomes: HappinessServiceOutcome[];
};

export type HappinessServiceOutcome = {
  rollWeight: number;
  scriptValue: number;
  changeCode: number;
};

export type EncounterSlotTables = {
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

export type CaptureWobbleProbability = {
  catch_rate: number;
  chance: number;
};

export type WeatherModifiers = {
  type_modifiers: WeatherTypeModifier[];
  move_effect_modifiers: WeatherMoveEffectModifier[];
};

export type WeatherTypeModifier = {
  weather: string;
  move_type: string;
  multiplier: BattleStatMultiplier;
};

export type WeatherMoveEffectModifier = {
  weather: string;
  move_effect: string;
  multiplier: BattleStatMultiplier;
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
  for (const raw of readAsmLines(path.join("constants", "type_constants.asm"))) {
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
  return { physical, special };
};

export type MovePriorityTable = {
  base_priority: number;
  effect_priorities: MoveEffectPriority[];
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
  fs.readFileSync(path.join(getDisassemblyRoot(), relativePath), "utf8").split(/\r?\n/);

const parsePokemonConstants = (): string[] => {
  const species: string[] = [];
  for (const rawLine of readAsmLines(path.join("constants", "pokemon_constants.asm"))) {
    const line = stripAsmComment(rawLine);
    const match = line.match(/^const\s+([A-Z0-9_]+)$/);
    if (!match) {
      continue;
    }
    species.push(match[1]);
  }
  return species;
};

const exactSpeciesIdMap = (): Map<string, string> => {
  const speciesByFileStem = new Map<string, string>();
  for (const species of parsePokemonConstants()) {
    const fileStem = species.toLowerCase();
    if (speciesByFileStem.has(fileStem)) {
      throw new Error(`Duplicate runtime species file stem '${fileStem}' from pokemon constants.`);
    }
    speciesByFileStem.set(fileStem, species);
  }
  return speciesByFileStem;
};

const exactSpeciesFromFileStem = (
  fileStem: string,
  speciesByFileStem: Map<string, string>,
  sourcePath: string
): string => {
  const species = speciesByFileStem.get(fileStem);
  if (!species) {
    throw new Error(`Unknown or case-changed runtime species file stem '${fileStem}' in ${sourcePath}.`);
  }
  return species;
};

const decodePhoneText = (payload: string): string => {
  let result = String(payload ?? "").replace(/<LF>/g, "\n").replace(/@/g, "");
  for (const [token, replacement] of Object.entries(CONTROL_CODE_REPLACEMENTS)) {
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
  for (const part of value.split("|").map((part) => part.trim()).filter(Boolean)) {
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

const parseRequiredDbSymbolList = (content: string, label: string, sourcePath: string): string[] => {
  const values = parseDbSymbolList(content, label);
  if (!values.length) {
    throw new Error(`Could not parse required ${label} table from ${sourcePath}.`);
  }
  return values;
};

export const exportFleeMons = (): FleeMonsData => {
  const sourcePath = path.join(getDisassemblyRoot(), "data", "wild", "flee_mons.asm");
  const content = fs.readFileSync(sourcePath, "utf8");
  const payload: FleeMonsData = {
    always: parseRequiredDbSymbolList(content, "AlwaysFleeMons", sourcePath),
    often: parseRequiredDbSymbolList(content, "OftenFleeMons", sourcePath),
    sometimes: parseRequiredDbSymbolList(content, "SometimesFleeMons", sourcePath),
  };
  writeJsonToTargets("flee_mons.json", payload, { indent: 2 });
  return payload;
};

export const exportMarts = (): Record<string, string[]> => {
  const sourcePath = path.join(getDisassemblyRoot(), "data", "items", "marts.asm");
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
      if (expectedCount !== null && marts[currentMart].length !== expectedCount) {
        throw new Error(
          `${currentMart} declared ${expectedCount} mart items but exported ${marts[currentMart].length}.`
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
  const sourcePath = path.join(getDisassemblyRoot(), "engine", "pokemon", "bills_pc.asm");
  const content = fs.readFileSync(sourcePath, "utf8");
  const strings: Record<string, string> = {};
  for (const match of content.matchAll(/^(PCString_[A-Za-z0-9_]+):\s+db\s+"([^"]*)@"/gm)) {
    strings[match[1]] = match[2];
  }
  writeJsonToTargets("pc_strings.json", strings, { indent: 2 });
  return strings;
};

export const exportMenuIcons = (): Record<string, string> => {
  const sourcePath = path.join(getDisassemblyRoot(), "data", "pokemon", "menu_icons.asm");
  const content = fs.readFileSync(sourcePath, "utf8");
  const icons: Record<string, string> = {};
  for (const rawLine of content.split(/\r?\n/)) {
    const match = rawLine.trim().match(/^db\s+(ICON_[A-Z0-9_]+)\s*;\s*([A-Z0-9_]+)$/);
    if (!match) {
      continue;
    }
    icons[match[2]] = match[1];
  }
  icons.EGG = "ICON_EGG";
  writeJsonToTargets("menu_icons.json", icons, { indent: 2 });
  return icons;
};

const parseDexEntryFile = (
  filePath: string,
  speciesByFileStem: Map<string, string>
): PokedexEntryData => {
  const fileStem = path.basename(filePath, ".asm");
  const species = exactSpeciesFromFileStem(fileStem, speciesByFileStem, filePath);
  const content = fs.readFileSync(filePath, "utf8");
  const classificationMatch = content.match(/db\s+"([^"]*)@"/);
  const sizeMatch = content.match(/dw\s+(\d+),\s*(\d+)\s*;\s*height,\s*weight/);
  if (!classificationMatch || !sizeMatch) {
    throw new Error(`Could not parse complete Pokedex entry for ${species} in ${filePath}.`);
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
      continue;
    }
    const opcode = textMatch[1];
    if (opcode === "page") {
      if (currentPage.length) {
        pages.push(currentPage.join(" @ ").trim());
      }
      currentPage = [];
    }
    currentPage.push(textMatch[2].replace(/@$/, ""));
  }
  if (currentPage.length) {
    pages.push(currentPage.join(" @ ").trim());
  }
  return {
    species,
    classification: classificationMatch[1],
    heightDigits: Number.parseInt(sizeMatch[1], 10),
    weightDigits: Number.parseInt(sizeMatch[2], 10),
    pages,
  };
};

export const exportPokedexEntries = (): PokedexEntryData[] => {
  const dexEntriesDir = path.join(getDisassemblyRoot(), "data", "pokemon", "dex_entries");
  const speciesByFileStem = exactSpeciesIdMap();
  const entries = fs
    .readdirSync(dexEntriesDir)
    .filter((entry) => entry.endsWith(".asm"))
    .sort()
    .map((entry) => parseDexEntryFile(path.join(dexEntriesDir, entry), speciesByFileStem));
  writeJsonToTargets("pokedex_entries.json", entries, { indent: 2 });
  return entries;
};

const parsePhoneConstants = (): Array<string | null> => {
  const entries: Array<string | null> = [];
  for (const raw of readAsmLines(path.join("constants", "phone_constants.asm"))) {
    const line = raw.trim();
    if (!line || line.startsWith(";")) {
      continue;
    }
    if (line.startsWith("const_skip")) {
      entries.push(null);
      continue;
    }
    if (line.startsWith("const ")) {
      entries.push(line.split(/\s+/)[1] ?? null);
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
  for (const raw of readAsmLines(path.join("data", "phone", "phone_contacts.asm"))) {
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
  for (const raw of readAsmLines(path.join("data", "phone", "non_trainer_names.asm"))) {
    const line = raw.trim();
    if (!line || line.startsWith(";")) {
      continue;
    }
    const match = NON_TRAINER_RE.exec(line);
    if (!match) {
      continue;
    }
    const [, label, text] = match;
    const segments = decodePhoneText(text).split("\n").map((segment) => segment.trimEnd());
    entries[`PHONECONTACT_${label.toUpperCase()}`] = [segments[0] ?? "", ...segments.slice(1)];
  }
  return entries;
};

const parseTrainerClassNames = (): Record<string, string> => {
  let classIds: string[] = [];
  for (const raw of readAsmLines(path.join("constants", "trainer_constants.asm"))) {
    const line = raw.trim();
    if (line.startsWith("trainerclass ")) {
      classIds.push(line.split(/\s+/)[1]);
    }
  }
  if (classIds[0] === "TRAINER_NONE") {
    classIds = classIds.slice(1);
  }
  const classNames: string[] = [];
  for (const raw of readAsmLines(path.join("data", "trainers", "class_names.asm"))) {
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
  const mapping: Record<string, string> = {};
  for (let index = 0; index < Math.min(classIds.length, classNames.length); index += 1) {
    mapping[classIds[index]] = classNames[index];
  }
  return mapping;
};

export const exportPhoneContacts = (): Record<string, PhoneContactRecord> => {
  const phoneConstants = parsePhoneConstants();
  const phoneRows = parsePhoneContactRows();
  const nonTrainerLines = parseNonTrainerNames();
  const classNames = parseTrainerClassNames();
  if (phoneConstants.length !== phoneRows.length) {
    throw new Error(`Phone constant count ${phoneConstants.length} does not match contact table ${phoneRows.length}`);
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
        throw new Error(`Phone contact ${contactId} is missing a trainer class.`);
      }
      const className = classNames[row.trainerClass];
      if (!className) {
        throw new Error(`Phone contact ${contactId} references trainer class '${row.trainerClass}' without an exported class name.`);
      }
      lines = [`${contactDisplayName(contactId)}:`, `   ${className}`];
    }
    if (!lines.length) {
      throw new Error(`Phone contact ${contactId} has no display lines`);
    }
    const primaryLabel = String(lines[0] ?? "").replace(/:$/, "").trim();
    if (!primaryLabel) {
      throw new Error(`Phone contact ${contactId} has an empty primary display label.`);
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

export const exportPermanentPhoneNumbers = (): string[] => {
  const phoneConstants = parsePhoneConstants();
  const phoneRows = parsePhoneContactRows();
  const declaredContactIds = new Set(phoneConstants.filter((contactId): contactId is string => Boolean(contactId)));
  const contactIdByTrainerLabel = new Map<string, string>();
  for (let index = 0; index < Math.min(phoneConstants.length, phoneRows.length); index += 1) {
    const contactId = phoneConstants[index];
    const trainerLabel = phoneRows[index].trainerLabel;
    if (contactId && trainerLabel) {
      contactIdByTrainerLabel.set(trainerLabel, contactId);
    }
  }
  const numbers: string[] = [];
  for (const raw of readAsmLines(path.join("data", "phone", "permanent_numbers.asm"))) {
    const cleaned = stripAsmComment(raw);
    if (!cleaned.startsWith("db ")) {
      continue;
    }
    const token = cleaned.slice("db ".length).split(",", 1)[0]?.trim();
    if (!token || token.startsWith("-1") || token.startsWith("$FF")) {
      break;
    }
    const resolvedContactId = contactIdByTrainerLabel.get(token) ?? (declaredContactIds.has(token) ? token : null);
    if (!resolvedContactId) {
      throw new Error(`Permanent phone number '${token}' does not match a declared phone contact id or trainer label.`);
    }
    numbers.push(resolvedContactId);
  }
  const deduped = Array.from(new Set(numbers));
  writeJsonToTargets("permanent_phone_numbers.json", deduped, { indent: 2 });
  return deduped;
};

export const exportSpecialPhoneCalls = (): string[] => {
  const calls: string[] = [];
  let inSpecialCalls = false;
  for (const raw of readAsmLines(path.join("constants", "phone_constants.asm"))) {
    const line = raw.trim();
    if (line.startsWith("; SpecialPhoneCallList")) {
      inSpecialCalls = true;
      continue;
    }
    if (!inSpecialCalls || !line || line.startsWith(";") || line === "const_def") {
      continue;
    }
    if (line.startsWith("const ")) {
      calls.push(line.split(/\s+/)[1]);
      continue;
    }
    if (line.startsWith("DEF NUM_SPECIALCALLS")) {
      break;
    }
  }
  if (!calls.length) {
    throw new Error("No special phone calls were exported from constants/phone_constants.asm");
  }
  writeJsonToTargets("special_phone_calls.json", calls, { indent: 2 });
  return calls;
};

export const exportNpcTrades = (): string[] => {
  const trades: string[] = [];
  for (const raw of readAsmLines(path.join("constants", "npc_trade_constants.asm"))) {
    const line = stripAsmComment(raw);
    if (!line.startsWith("const NPC_TRADE_")) {
      continue;
    }
    trades.push(line.split(/\s+/)[1]);
  }
  if (!trades.length) {
    throw new Error("No NPC trade ids were exported from constants/npc_trade_constants.asm");
  }
  writeJsonToTargets("npc_trades.json", trades, { indent: 2 });
  return trades;
};

export const exportSpecialRoutines = (): string[] => {
  const routines: string[] = [];
  for (const raw of readAsmLines(path.join("data", "events", "special_pointers.asm"))) {
    const line = stripAsmComment(raw);
    if (!line.startsWith("add_special ")) {
      continue;
    }
    const routine = line.slice("add_special ".length).trim();
    if (!routine) {
      throw new Error(`Malformed special pointer row: ${raw}`);
    }
    routines.push(routine);
  }
  if (!routines.length) {
    throw new Error("No special routines were exported from data/events/special_pointers.asm");
  }
  writeJsonToTargets("special_routines.json", routines, { indent: 2 });
  return routines;
};

const parseInitRoamMons = (): Array<{ species: string; level: number; mapConstant: string }> => {
  const lines = readAsmLines(path.join("engine", "overworld", "wildmons.asm"));
  const start = lines.findIndex((line) => stripAsmComment(line) === "InitRoamMons:");
  if (start < 0) {
    throw new Error("Unable to find InitRoamMons in engine/overworld/wildmons.asm");
  }
  const end = lines.findIndex(
    (line, index) => index > start && stripAsmComment(line).startsWith("CheckEncounterRoamMon:")
  );
  if (end < 0) {
    throw new Error("Unable to find end of InitRoamMons in engine/overworld/wildmons.asm");
  }
  const block = lines.slice(start + 1, end).map((line) => stripAsmComment(line).trim()).filter(Boolean);
  const speciesBySlot = new Map<number, string>();
  const levelBySlot = new Map<number, number>();
  const mapBySlot = new Map<number, string>();
  let register:
    | { kind: "species"; value: string }
    | { kind: "level"; value: number }
    | { kind: "map"; value: string }
    | { kind: "other" }
    | null = null;
  for (const line of block) {
    let match = line.match(/^ld a, GROUP_([A-Z0-9_]+)$/);
    if (match) {
      register = { kind: "map", value: match[1] };
      continue;
    }
    match = line.match(/^ld a, MAP_([A-Z0-9_]+)$/);
    if (match) {
      register = { kind: "other" };
      continue;
    }
    match = line.match(/^ld a, ([0-9]+)$/);
    if (match) {
      register = { kind: "level", value: Number.parseInt(match[1], 10) };
      continue;
    }
    match = line.match(/^ld a, ([A-Z0-9_]+)$/);
    if (match) {
      register = { kind: "species", value: match[1] };
      continue;
    }
    match = line.match(/^ld \[wRoamMon([0-9]+)Species\], a$/);
    if (match) {
      if (register?.kind !== "species") {
        throw new Error(`Roaming Pokemon species store '${line}' has invalid register state.`);
      }
      speciesBySlot.set(Number.parseInt(match[1], 10), register.value);
      continue;
    }
    match = line.match(/^ld \[wRoamMon([0-9]+)Level\], a$/);
    if (match) {
      if (register?.kind !== "level") {
        throw new Error(`Roaming Pokemon level store '${line}' has invalid register state.`);
      }
      levelBySlot.set(Number.parseInt(match[1], 10), register.value);
      continue;
    }
    match = line.match(/^ld \[wRoamMon([0-9]+)MapGroup\], a$/);
    if (match) {
      if (register?.kind !== "map") {
        throw new Error(`Roaming Pokemon map group store '${line}' has invalid register state.`);
      }
      mapBySlot.set(Number.parseInt(match[1], 10), register.value);
    }
  }
  return [...speciesBySlot.keys()].sort((left, right) => left - right).map((slot) => {
    const species = speciesBySlot.get(slot);
    const level = levelBySlot.get(slot);
    const mapConstant = mapBySlot.get(slot);
    if (!species || level === undefined || !mapConstant) {
      throw new Error(`InitRoamMons slot ${slot} is missing species, level, or map group data.`);
    }
    return { species, level, mapConstant };
  });
};

export const exportRoamingPokemon = (
  runtimeMapMetadata: Record<string, RuntimeMapMetadataRecord>
): RoamingPokemonDefinition[] => {
  const definitions = parseInitRoamMons().map((definition) => {
    const metadata = runtimeMapMetadata[definition.mapConstant];
    if (!metadata) {
      throw new Error(`InitRoamMons references missing runtime map metadata '${definition.mapConstant}'.`);
    }
    return {
      species: definition.species,
      level: definition.level,
      mapGroup: metadata.groupId,
      mapNumber: metadata.mapId,
    };
  });
  if (!definitions.length) {
    throw new Error("No roaming Pokemon definitions were exported from InitRoamMons.");
  }
  writeJsonToTargets("roaming_pokemon.json", definitions, { indent: 2 });
  return definitions;
};

export const exportBuenaPrizes = (): BuenaPrizeDefinition[] => {
  const prizes: BuenaPrizeDefinition[] = [];
  let inTable = false;
  for (const raw of readAsmLines(path.join("data", "items", "buena_prizes.asm"))) {
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
    prizes.push({
      itemId: match[1],
      cost: Number.parseInt(match[2], 10),
    });
  }
  if (!prizes.length) {
    throw new Error("No Buena prize definitions were exported from data/items/buena_prizes.asm");
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

export const exportBuenaPasswordCategories = (): BuenaPasswordCategoryDefinition[] => {
  const lines = readAsmLines(path.join("data", "radio", "buenas_passwords.asm"));
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
    const rowMatch = line.match(/^\.([A-Za-z0-9_]+):\s+db\s+([A-Z0-9_]+),\s*([0-9]+),\s*(.+)$/);
    if (!rowMatch) {
      continue;
    }
    const options = rowMatch[4].split(/,\s*/).map(parseBuenaPasswordOption);
    if (options.length !== 3) {
      throw new Error(`Buena password category ${rowMatch[1]} must declare exactly three options`);
    }
    rows.set(rowMatch[1], {
      id: rowMatch[1],
      categoryType: rowMatch[2],
      points: Number.parseInt(rowMatch[3], 10),
      options,
    });
  }
  if (!categoryOrder.length) {
    throw new Error("No Buena password category pointers were exported from data/radio/buenas_passwords.asm");
  }
  const categories = categoryOrder.map((id) => {
    const row = rows.get(id);
    if (!row) {
      throw new Error(`Buena password pointer references missing category row '${id}'`);
    }
    return row;
  });
  writeJsonToTargets("buena_password_categories.json", categories, { indent: 2 });
  return categories;
};

export const exportKurtApricornRecipes = (): KurtApricornRecipe[] => {
  const recipes: KurtApricornRecipe[] = [];
  let inTable = false;
  for (const raw of readAsmLines(path.join("data", "items", "apricorn_balls.asm"))) {
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
    recipes.push({ apricorn: match[1], ball: match[2] });
  }
  if (!recipes.length) {
    throw new Error("No Kurt apricorn recipes were exported from data/items/apricorn_balls.asm");
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
  let pendingNameLabel: "SpecialShuckleOT" | "SpecialShuckleNickname" | null = null;
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
    throw new Error("Could not export complete Shuckie gift definition from engine/events/shuckle.asm");
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

export const exportDratiniMoveSets = (): DratiniMoveSetDefinition[] => {
  const moveSets: DratiniMoveSetDefinition[] = [];
  let currentMode: number | null = null;
  let currentMoves: string[] = [];
  for (const raw of readAsmLines(path.join("engine", "events", "dratini.asm"))) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    const labelMatch = line.match(/^\.Moveset([0-9]+):$/);
    if (labelMatch) {
      if (currentMode !== null) {
        throw new Error(`Dratini moveset ${currentMode} is missing zero terminator`);
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
      moveSets.push({ mode: currentMode, moves: currentMoves });
      currentMode = null;
      currentMoves = [];
      continue;
    }
    currentMoves.push(dbMatch[1]);
  }
  if (currentMode !== null) {
    throw new Error(`Dratini moveset ${currentMode} is missing zero terminator`);
  }
  if (!moveSets.length) {
    throw new Error("No Dratini move sets were exported from engine/events/dratini.asm");
  }
  writeJsonToTargets("dratini_move_sets.json", moveSets, { indent: 2 });
  return moveSets;
};

const parseScriptConstantNumber = (name: string): number => {
  for (const raw of readAsmLines(path.join("constants", "script_constants.asm"))) {
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
  for (const raw of readAsmLines(path.join("engine", "events", "bug_contest", "contest_2.asm"))) {
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
  throw new Error("Could not parse selected Bug-Catching Contest contestant count.");
};

const parseBugContestContestantFlags = (): string[] => {
  const flags: string[] = [];
  let inTable = false;
  for (const raw of readAsmLines(path.join("data", "events", "bug_contest_flags.asm"))) {
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
      throw new Error(`Malformed Bug-Catching Contest contestant flag row: ${raw}`);
    }
    flags.push(match[1]);
  }
  if (!flags.length) {
    throw new Error("No Bug-Catching Contest contestant flags were exported from data/events/bug_contest_flags.asm");
  }
  return flags;
};

export const exportBugContestConfig = (): BugContestConfig => {
  const config = {
    parkBalls: parseScriptConstantNumber("BUG_CONTEST_BALLS"),
    timerMinutes: parseScriptConstantNumber("BUG_CONTEST_MINUTES"),
    timerSeconds: parseScriptConstantNumber("BUG_CONTEST_SECONDS"),
    selectedContestantCount: parseBugContestSelectedContestantCount(),
    contestantFlags: parseBugContestContestantFlags(),
  };
  const expectedContestants = parseScriptConstantNumber("NUM_BUG_CONTESTANTS");
  if (config.contestantFlags.length !== expectedContestants) {
    throw new Error(
      `Bug-Catching Contest flag count ${config.contestantFlags.length} does not match NUM_BUG_CONTESTANTS ${expectedContestants}.`
    );
  }
  writeJsonToTargets("bug_contest_config.json", config, { indent: 2 });
  return config;
};

const parsePokemonConstantOrder = (): string[] => {
  const species: string[] = [];
  let inPokemonConstants = false;
  for (const raw of readAsmLines(path.join("constants", "pokemon_constants.asm"))) {
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
    throw new Error("No Pokemon constants were exported from constants/pokemon_constants.asm");
  }
  return species;
};

export const exportBattleTowerRules = (): BattleTowerRules => {
  const constantsLines = readAsmLines(path.join("constants", "battle_tower_constants.asm")).map(
    (raw) => stripAsmComment(raw).trim()
  );
  const parseBattleTowerConstant = (name: string): number => {
    const line = constantsLines.find((candidate) => candidate.startsWith(`DEF ${name} EQU `));
    const match = line?.match(new RegExp(`^DEF\\s+${name}\\s+EQU\\s+([0-9]+)$`));
    if (!match) {
      throw new Error(`Could not parse ${name} from battle_tower_constants.asm`);
    }
    return Number(match[1]);
  };
  const requiredPartyCount = parseBattleTowerConstant("BATTLETOWER_PARTY_LENGTH");
  const challengeStreakLength = parseBattleTowerConstant("BATTLETOWER_STREAK_LENGTH");
  const directSpecies: string[] = [];
  let rangeStart: string | null = null;
  const routineLines = readAsmLines(path.join("mobile", "mobile_46.asm")).map((raw) =>
    stripAsmComment(raw).trim()
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
        const nextLine = body.slice(bodyIndex + 1).find((candidate) => candidate.length > 0);
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
    throw new Error("Could not parse Battle Tower banned species range from mobile/mobile_46.asm");
  }
  const pokemonOrder = parsePokemonConstantOrder();
  const rangeStartIndex = pokemonOrder.indexOf(rangeStart);
  if (rangeStartIndex < 0) {
    throw new Error(`Battle Tower banned species range starts with unknown species '${rangeStart}'`);
  }
  const bannedSpecies = [...directSpecies, ...pokemonOrder.slice(rangeStartIndex)];
  if (!bannedSpecies.length) {
    throw new Error("No Battle Tower banned species were exported from mobile/mobile_46.asm");
  }
  const ruleLines = readAsmLines(path.join("engine", "events", "battle_tower", "rules.asm")).map(
    (raw) => stripAsmComment(raw).trim()
  );
  const checkStart = ruleLines.indexOf("_CheckForBattleTowerRules:");
  if (checkStart < 0) {
    throw new Error("Could not find _CheckForBattleTowerRules in battle_tower/rules.asm");
  }
  const requiredPartyCountLine = ruleLines
    .slice(checkStart)
    .find((line) => /^ld\s+\[hl\],\s*'[0-9]'$/.test(line));
  const requiredPartyCountMatch = requiredPartyCountLine?.match(/^ld\s+\[hl\],\s*'([0-9])'$/);
  if (!requiredPartyCountMatch) {
    throw new Error("Could not parse Battle Tower required party count");
  }
  const rulePartyCount = Number(requiredPartyCountMatch[1]);
  if (rulePartyCount !== requiredPartyCount) {
    throw new Error(
      `Battle Tower rule party count ${rulePartyCount} does not match BATTLETOWER_PARTY_LENGTH ${requiredPartyCount}`
    );
  }
  const levelCheckStart = routineLines.indexOf("BattleTower_LevelCheck:");
  if (levelCheckStart < 0) {
    throw new Error("Could not find BattleTower_LevelCheck in mobile/mobile_46.asm");
  }
  const levelGroupSizeLine = routineLines
    .slice(levelCheckStart)
    .find((line) => /^ld\s+c,\s*[0-9]+$/.test(line));
  const levelGroupSizeMatch = levelGroupSizeLine?.match(/^ld\s+c,\s*([0-9]+)$/);
  if (!levelGroupSizeMatch) {
    throw new Error("Could not parse Battle Tower level group size");
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
      throw new Error(`Could not parse Battle Tower level group menu entry '${line}'`);
    }
    levelGroups.push(Number(match[1]) / Number(levelGroupSizeMatch[1]));
  }
  if (!levelGroups.length || !levelGroups.every((group) => Number.isInteger(group))) {
    throw new Error("Battle Tower level menu did not export exact integer level groups");
  }
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
    throw new Error("Battle Tower rule text pointer table must contain five text labels");
  }
  const [
    ,
    partyCountFailureText,
    duplicateSpeciesFailureText,
    duplicateHeldItemFailureText,
    eggFailureText,
  ] = textLabels;
  const rules = {
    bannedSpecies,
    requiredPartyCount,
    challengeStreakLength,
    minimumLevelGroup: Math.min(...levelGroups),
    maximumLevelGroup: Math.max(...levelGroups),
    levelGroupSize: Number(levelGroupSizeMatch[1]),
    partyCountFailureText,
    duplicateSpeciesFailureText,
    duplicateHeldItemFailureText,
    eggFailureText,
  };
  writeJsonToTargets("battle_tower_rules.json", rules, { indent: 2 });
  return rules;
};

export const exportOakRatings = (): OakRatingEntry[] => {
  const entries = readAsmLines(path.join("data", "events", "pokedex_ratings.asm"))
    .map((raw) => stripAsmComment(raw).trim())
    .map((line) => line.match(/^rating\s+([0-9]+),\s*([A-Z0-9_]+),\s*([A-Za-z0-9_]+)$/))
    .filter((match): match is RegExpMatchArray => Boolean(match))
    .map((match) => ({
      caughtCountLimit: Number(match[1]),
      fanfare: match[2],
      textLabel: match[3],
    }));
  if (!entries.length) {
    throw new Error("No Oak rating entries were exported from data/events/pokedex_ratings.asm");
  }
  for (let index = 1; index < entries.length; index += 1) {
    if (entries[index].caughtCountLimit <= entries[index - 1].caughtCountLimit) {
      throw new Error("Oak rating caught-count limits must be strictly increasing");
    }
  }
  writeJsonToTargets("oak_ratings.json", entries, { indent: 2 });
  return entries;
};

const parseOddEggOriginalTrainerName = (): string => {
  for (const raw of readAsmLines(path.join("engine", "events", "odd_egg.asm"))) {
    const line = stripAsmComment(raw).trim();
    const match = line.match(/^dname\s+"([^"]+)",\s*MON_NAME_LENGTH\s*\+\s*1$/);
    if (match) {
      return match[1];
    }
  }
  throw new Error("Could not parse Odd Egg original trainer name from engine/events/odd_egg.asm");
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
      probabilities.push(Number.parseInt(match[1], 10));
    }
  }
  if (!probabilities.length) {
    throw new Error("No Odd Egg probabilities were exported from data/events/odd_eggs.asm");
  }
  return probabilities;
};

const parseOddEggMoveList = (line: string): string[] => {
  const match = line.match(/^db\s+(.+)$/);
  if (!match) {
    throw new Error(`Malformed Odd Egg move row: ${line}`);
  }
  return match[1]
    .split(",")
    .map((part) => part.trim())
    .filter((move) => move !== "0");
};

const parseOddEggDvs = (line: string): [number, number, number, number] => {
  const match = line.match(/^dn\s+([0-9]+),\s*([0-9]+),\s*([0-9]+),\s*([0-9]+)$/);
  if (!match) {
    throw new Error(`Malformed Odd Egg DVs row: ${line}`);
  }
  return [
    Number.parseInt(match[1], 10),
    Number.parseInt(match[2], 10),
    Number.parseInt(match[3], 10),
    Number.parseInt(match[4], 10),
  ];
};

const parseOddEggDefinitionsFromLines = (lines: string[], probabilities: number[], originalTrainerName: string): OddEggDefinition[] => {
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
    index += 1;
    if (!/^db\s+0,\s*0,\s*0$/.test(lines[index])) {
      throw new Error(`Malformed Odd Egg Pokerus/caught data row: ${lines[index]}`);
    }
    index += 1;
    const levelMatch = lines[index].match(/^db\s+([0-9]+)$/);
    if (!levelMatch) {
      throw new Error(`Malformed Odd Egg level row: ${lines[index]}`);
    }
    const level = Number.parseInt(levelMatch[1], 10);
    index += 1;
    while (index < lines.length && !lines[index].startsWith("dname ")) {
      index += 1;
    }
    const nicknameMatch = lines[index].match(/^dname\s+"([^"]+)",\s*MON_NAME_LENGTH$/);
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
      `Odd Egg definition count ${definitions.length} does not match probability count ${probabilities.length}`
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
    parseOddEggOriginalTrainerName()
  );
  const totalProbability = definitions.reduce((total, definition) => total + definition.probability, 0);
  if (totalProbability !== 100) {
    throw new Error(`Odd Egg probabilities sum to ${totalProbability}%, not 100%.`);
  }
  writeJsonToTargets("odd_egg_definitions.json", definitions, { indent: 2 });
  return definitions;
};

export const exportMagikarpLengths = (): MagikarpLengthEntry[] => {
  const entries: MagikarpLengthEntry[] = [];
  let inTable = false;
  for (const raw of readAsmLines(path.join("data", "events", "magikarp_lengths.asm"))) {
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
    entries.push({
      threshold: Number.parseInt(match[1], 10),
      divisor: Number.parseInt(match[2], 10),
    });
  }
  if (!entries.length) {
    throw new Error("No Magikarp length entries were exported from data/events/magikarp_lengths.asm");
  }
  writeJsonToTargets("magikarp_lengths.json", entries, { indent: 2 });
  return entries;
};

const parseHappinessChangeConstants = (): Map<string, number> => {
  const constants = new Map<string, number>();
  let sawHappinessHeader = false;
  let inBlock = false;
  let nextValue = 0;
  for (const raw of readAsmLines(path.join("constants", "pokemon_data_constants.asm"))) {
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
      constants.set(match[1], nextValue);
      nextValue += 1;
    }
  }
  if (!constants.size) {
    throw new Error("No HAPPINESS_* constants were exported from constants/pokemon_data_constants.asm");
  }
  return constants;
};

const parseSignedDbNumber = (token: string): number => {
  const trimmed = token.trim();
  if (!/^[+-]?[0-9]+$/.test(trimmed)) {
    throw new Error(`Malformed signed db number '${token}'`);
  }
  return Number.parseInt(trimmed, 10);
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
    value += match[2] === "+" ? Number.parseInt(match[3], 10) : -Number.parseInt(match[3], 10);
  }
  if (value < 0 || value > 255) {
    throw new Error(`Percent expression '${token}' resolved outside byte range`);
  }
  return value;
};

export const exportHappinessData = (): HappinessData => {
  const constants = parseHappinessChangeConstants();
  const changes: HappinessChangeEntry[] = [];
  for (const raw of readAsmLines(path.join("data", "events", "happiness_changes.asm"))) {
    const line = stripAsmComment(raw).trim();
    if (!line || line === "HappinessChanges:" || line.startsWith("table_width ")) {
      continue;
    }
    if (line.startsWith("assert_table_length ")) {
      break;
    }
    const match = line.match(/^db\s+([+-]?[0-9]+),\s*([+-]?[0-9]+),\s*([+-]?[0-9]+)$/);
    if (!match) {
      throw new Error(`Malformed happiness change row: ${raw}`);
    }
    const index = changes.length + 1;
    const code = [...constants.entries()].find(([, value]) => value === index)?.[0];
    if (!code) {
      throw new Error(`Happiness change row ${index} has no matching HAPPINESS_* constant`);
    }
    changes.push({
      code,
      changeCode: index,
      low: parseSignedDbNumber(match[1]),
      mid: parseSignedDbNumber(match[2]),
      high: parseSignedDbNumber(match[3]),
    });
  }
  if (changes.length !== constants.size) {
    throw new Error(`Happiness change row count ${changes.length} does not match constants ${constants.size}`);
  }

  const labelToRoutine: Record<string, string> = {
    HappinessData_OlderHaircutBrother: "OlderHaircutBrother",
    HappinessData_YoungerHaircutBrother: "YoungerHaircutBrother",
    HappinessData_DaisysGrooming: "DaisysGrooming",
  };
  const services: HappinessServiceTable[] = [];
  let current: HappinessServiceTable | null = null;
  for (const raw of readAsmLines(path.join("data", "events", "happiness_probabilities.asm"))) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    const labelMatch = line.match(/^(HappinessData_[A-Za-z0-9_]+):$/);
    if (labelMatch) {
      const routine = labelToRoutine[labelMatch[1]];
      if (!routine) {
        throw new Error(`Unknown happiness probability table '${labelMatch[1]}'`);
      }
      current = { routine, outcomes: [] };
      services.push(current);
      continue;
    }
    if (!current) {
      continue;
    }
    const rowMatch = line.match(/^db\s+(.+?),\s*([0-9]+),\s*(HAPPINESS_[A-Z0-9_]+)$/);
    if (!rowMatch) {
      throw new Error(`Malformed happiness probability row: ${raw}`);
    }
    const changeCode = constants.get(rowMatch[3]);
    if (!changeCode) {
      throw new Error(`Happiness probability row references unknown ${rowMatch[3]}`);
    }
    current.outcomes.push({
      rollWeight: parsePercentExpression(rowMatch[1]),
      scriptValue: Number.parseInt(rowMatch[2], 10),
      changeCode,
    });
  }
  if (services.length !== Object.keys(labelToRoutine).length || services.some((service) => !service.outcomes.length)) {
    throw new Error("Could not export complete happiness service probability tables");
  }
  const payload = { changes, services };
  writeJsonToTargets("happiness_data.json", payload, { indent: 2 });
  return payload;
};

export const exportEncounterSlotTables = (): EncounterSlotTables => {
  const labelToKey: Record<string, keyof EncounterSlotTables> = {
    GrassMonProbTable: "grass",
    WaterMonProbTable: "water",
  };
  const tables: EncounterSlotTables = { grass: [], water: [] };
  let current: keyof EncounterSlotTables | null = null;
  for (const raw of readAsmLines(path.join("data", "wild", "probabilities.asm"))) {
    const line = stripAsmComment(raw).trim();
    if (!line) {
      continue;
    }
    const label = line.match(/^([A-Za-z0-9_]+):$/);
    if (label) {
      current = labelToKey[label[1]] ?? null;
      continue;
    }
    if (!current || line.startsWith("table_width ") || line.startsWith("assert_table_length ")) {
      continue;
    }
    const row = line.match(/^mon_prob\s+([0-9]+),\s*([0-9]+)$/);
    if (!row) {
      throw new Error(`Malformed encounter slot probability row: ${raw}`);
    }
    const threshold = Number.parseInt(row[1], 10);
    const slot = Number.parseInt(row[2], 10);
    if (threshold < 1 || threshold > 100) {
      throw new Error(`Encounter slot threshold ${threshold} is outside 1..=100`);
    }
    tables[current].push({ threshold, slot });
  }
  if (!tables.grass.length || !tables.water.length) {
    throw new Error("Could not export complete encounter slot probability tables");
  }
  const payload = { grass: tables.grass, water: tables.water };
  writeJsonToTargets("encounter_slot_tables.json", payload, { indent: 2 });
  return payload;
};

const parseBattleStatMultiplierRows = (relativePath: string): BattleStatMultiplier[] => {
  const rows: BattleStatMultiplier[] = [];
  for (const raw of readAsmLines(relativePath)) {
    const line = stripAsmComment(raw).trim();
    if (!line || line.endsWith(":")) {
      continue;
    }
    const row = line.match(/^db\s+([0-9]+),\s*([0-9]+)$/);
    if (!row) {
      throw new Error(`Malformed battle stat multiplier row in ${relativePath}: ${raw}`);
    }
    const numerator = Number.parseInt(row[1], 10);
    const denominator = Number.parseInt(row[2], 10);
    if (denominator <= 0) {
      throw new Error(`Battle stat multiplier in ${relativePath} has invalid denominator ${denominator}`);
    }
    rows.push({ numerator, denominator });
  }
  if (rows.length !== 13) {
    throw new Error(`Expected 13 battle stat multiplier rows in ${relativePath}, found ${rows.length}`);
  }
  return rows;
};

export const exportBattleStatMultipliers = (): BattleStatMultiplierTables => {
  const payload = {
    stat: parseBattleStatMultiplierRows(path.join("data", "battle", "stat_multipliers.asm")),
    accuracy: parseBattleStatMultiplierRows(path.join("data", "battle", "accuracy_multipliers.asm")),
  };
  writeJsonToTargets("battle_stat_multipliers.json", payload, { indent: 2 });
  return payload;
};

export const exportCaptureWobbleProbabilities = (): CaptureWobbleProbability[] => {
  const rows: CaptureWobbleProbability[] = [];
  let inTable = false;
  for (const raw of readAsmLines(path.join("data", "battle", "wobble_probabilities.asm"))) {
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
      throw new Error(`Capture wobble catch rate ${catchRate} is outside 1..=255`);
    }
    if (chance > 255) {
      throw new Error(`Capture wobble chance ${chance} is outside 0..=255`);
    }
    rows.push({ catch_rate: catchRate, chance });
  }
  if (!rows.length) {
    throw new Error("Could not export capture wobble probabilities");
  }
  writeJsonToTargets("capture_wobble_probabilities.json", rows, { indent: 2 });
  return rows;
};

const weatherEffectivenessMultiplier = (token: string): BattleStatMultiplier => {
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
  for (const raw of readAsmLines(path.join("data", "types", "type_matchups.asm"))) {
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
    const row = line.match(/^db\s+([A-Z0-9_]+),\s*([A-Z0-9_]+),\s*([A-Z0-9_]+)$/);
    if (!row) {
      throw new Error(`Malformed type effectiveness row: ${raw}`);
    }
    const [, attacker, defender, effectiveness] = row;
    const entry = { attacker, defender, multiplier: typeEffectivenessMultiplier(effectiveness) };
    if (section === "normal") {
      sparseMatchups.push(entry);
    } else {
      foresightMatchups.push(entry);
    }
  }
  if (!sparseMatchups.length || !foresightMatchups.length) {
    throw new Error("Could not export complete type effectiveness table");
  }
  const categories = typeCategoriesFromAsm();
  const types = [...categories.physical, ...categories.special];
  const sparseByPair = new Map(
    sparseMatchups.map((entry) => [`${entry.attacker}\u0000${entry.defender}`, entry.multiplier])
  );
  const matchups = types.flatMap((attacker) =>
    types.map((defender) => ({
      attacker,
      defender,
      multiplier: sparseByPair.get(`${attacker}\u0000${defender}`) ?? { numerator: 1, denominator: 1 },
    }))
  );
  const payload = { matchups, foresight_matchups: foresightMatchups };
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
    throw new Error(`Move effect priority id '${asmEffect}' must use an exact EFFECT_ token`);
  }
  return asmEffect.slice("EFFECT_".length);
};

export const exportMovePriorityTable = (movesData: Record<string, Move> = {}): MovePriorityTable => {
  let basePriority: number | null = null;
  for (const raw of readAsmLines(path.join("constants", "battle_constants.asm"))) {
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

  const sparseEffectPriorities: MoveEffectPriority[] = [];
  let inTable = false;
  for (const raw of readAsmLines(path.join("data", "moves", "effects_priorities.asm"))) {
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
    sparseEffectPriorities.push({
      move_effect: moveEffectSchemaId(row[1]),
      priority: Number.parseInt(row[2], 10),
    });
  }
  if (!sparseEffectPriorities.length) {
    throw new Error("Could not export move effect priorities");
  }
  const sparseByEffect = new Map(sparseEffectPriorities.map((entry) => [entry.move_effect, entry.priority]));
  const effects = [...new Set(Object.values(movesData).map((move) => move.effect).filter(Boolean))].sort();
  const effectPriorities =
    effects.length > 0
      ? effects.map((moveEffect) => ({
          move_effect: moveEffect,
          priority: sparseByEffect.get(moveEffect) ?? basePriority,
        }))
      : sparseEffectPriorities;

  const core = readAsmLines(path.join("engine", "battle", "core.asm"))
    .map((raw) => stripAsmComment(raw).trim())
    .filter(Boolean);
  const vitalThrowIndex = core.findIndex((line) => line === "cp VITAL_THROW");
  if (
    vitalThrowIndex < 0 ||
    core[vitalThrowIndex + 1] !== "ld a, 0" ||
    core[vitalThrowIndex + 2] !== "ret z"
  ) {
    throw new Error("Could not export VITAL_THROW priority override from GetMovePriority");
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
  const typeModifiers: WeatherTypeModifier[] = [];
  const moveEffectModifiers: WeatherMoveEffectModifier[] = [];
  let section: "type" | "move_effect" | null = null;
  for (const raw of readAsmLines(path.join("data", "battle", "weather_modifiers.asm"))) {
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
    const row = line.match(/^db\s+([A-Z0-9_]+),\s*([A-Z0-9_]+),\s*([A-Z0-9_]+)$/);
    if (!row) {
      throw new Error(`Malformed weather modifier row: ${raw}`);
    }
    const [, weather, target, effectiveness] = row;
    const multiplier = weatherEffectivenessMultiplier(effectiveness);
    if (section === "type") {
      typeModifiers.push({ weather, move_type: target, multiplier });
    } else {
      moveEffectModifiers.push({ weather, move_effect: moveEffectSchemaId(target), multiplier });
    }
  }
  if (!typeModifiers.length || !moveEffectModifiers.length) {
    throw new Error("Could not export complete weather modifier tables");
  }
  const payload = { type_modifiers: typeModifiers, move_effect_modifiers: moveEffectModifiers };
  writeJsonToTargets("weather_modifiers.json", payload, { indent: 2 });
  return payload;
};

const parseFrontpicAnimNumber = (token: string): number => {
  const cleaned = token.trim();
  if (!cleaned) {
    throw new Error("Missing frontpic animation numeric operand.");
  }
  if (cleaned.startsWith("$")) {
    const value = Number.parseInt(cleaned.slice(1), 16);
    if (!Number.isFinite(value)) {
      throw new Error(`Invalid frontpic animation numeric operand '${token}'.`);
    }
    return value;
  }
  if (!/^[+-]?\d+$/.test(cleaned)) {
    throw new Error(`Invalid frontpic animation numeric operand '${token}'.`);
  }
  return Number.parseInt(cleaned, 10);
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
        throw new Error(`Malformed frontpic animation setrepeat row: ${rawLine}`);
      }
      commands.push({ kind: "setrepeat", count: parseFrontpicAnimNumber(parts[1]) });
      continue;
    }
    if (opcode === "dorepeat") {
      if (parts.length !== 2) {
        throw new Error(`Malformed frontpic animation dorepeat row: ${rawLine}`);
      }
      commands.push({ kind: "dorepeat", target: parseFrontpicAnimNumber(parts[1]) });
      continue;
    }
    if (opcode === "endanim") {
      if (parts.length !== 1) {
        throw new Error(`Malformed frontpic animation endanim row: ${rawLine}`);
      }
      commands.push({ kind: "endanim" });
      continue;
    }
    throw new Error(`Unknown frontpic animation opcode '${opcode}' in row: ${rawLine}`);
  }
  return { commands };
};

export const exportPokemonFrontpicAnimations = (): Record<string, FrontpicAnimProgram> => {
  const pokemonGfxDir = path.join(getDisassemblyRoot(), "gfx", "pokemon");
  const entries: Record<string, FrontpicAnimProgram> = {};
  const speciesByFileStem = exactSpeciesIdMap();
  if (fs.existsSync(pokemonGfxDir)) {
    for (const entry of fs.readdirSync(pokemonGfxDir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      if (!entry.isDirectory()) {
        continue;
      }
      const animPath = path.join(pokemonGfxDir, entry.name, "anim.asm");
      if (!fs.existsSync(animPath)) {
        continue;
      }
      const species = exactSpeciesFromFileStem(entry.name, speciesByFileStem, animPath);
      const program = parseFrontpicAnimScript(fs.readFileSync(animPath, "utf8"));
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
    modifiers: [
      { music_id: "MUSIC_POKEMON_MARCH", numerator: 2, denominator: 1 },
      { music_id: "MUSIC_RUINS_OF_ALPH_RADIO", numerator: 2, denominator: 1 },
      { music_id: "MUSIC_POKEMON_LULLABY", numerator: 1, denominator: 2 },
    ],
  };
  writeJsonToTargets("encounter_music_modifiers.json", modifiers, { indent: 2 });
  return modifiers;
};

export const exportRuntimeAssets = (): {
  fleeMons: FleeMonsData;
  encounterMusicModifiers: EncounterMusicModifiers;
  pcStrings: Record<string, string>;
  menuIcons: Record<string, string>;
  pokedexEntries: PokedexEntryData[];
  pokemonFrontpicAnimations: Record<string, FrontpicAnimProgram>;
  marts: Record<string, string[]>;
  phoneContacts: Record<string, PhoneContactRecord>;
  permanentPhoneNumbers: string[];
  specialPhoneCalls: string[];
  npcTrades: string[];
  specialRoutines: string[];
} => {
  const fleeMons = exportFleeMons();
  const encounterMusicModifiers = exportEncounterMusicModifiers();
  const pcStrings = exportPcStrings();
  const menuIcons = exportMenuIcons();
  const pokedexEntries = exportPokedexEntries();
  const pokemonFrontpicAnimations = exportPokemonFrontpicAnimations();
  const marts = exportMarts();
  const phoneContacts = exportPhoneContacts();
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
    phoneContacts,
    permanentPhoneNumbers,
    specialPhoneCalls,
    npcTrades,
    specialRoutines,
  };
};

import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripAsmComment, writeJsonToTargets } from "./asm-utils";

type FleeMonsData = {
  always: string[];
  often: string[];
  sometimes: string[];
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

export const exportRuntimeAssets = (): {
  fleeMons: FleeMonsData;
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

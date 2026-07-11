#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");

const DEFAULT_PROJECT_ROOT = path.resolve(__dirname, "..");
const DEFAULT_REPO_ROOT = path.resolve(DEFAULT_PROJECT_ROOT, "..", "..");
const DEFAULT_DISASSEMBLY_ROOT = path.join(DEFAULT_REPO_ROOT, "vendor", "pokecrystal");
const DEFAULT_OUT_DIR = path.join(DEFAULT_PROJECT_ROOT, "assets", "data");
const GENERATED_FILES = [
  "runtime_map_metadata.json",
  "runtime_spawn_points.json",
  "battle_animation_table.json",
  "battle_anim_bundle.json",
  "asm_text.json",
  "move_names.json",
  "map_blocks.json",
  "phone_contacts.json",
  "permanent_phone_numbers.json",
  "initialize_events.json",
  "story_event_script_constants.json",
  "sprite_palette_defaults.json",
  path.join("collision", "collision_permissions.json"),
  path.join("collision", "collision_stdscripts.json"),
  "sprite_anim_bundle.json",
];
const GENERATED_DIRECTORIES = ["tilesets", "collision"];
const REQUIRED_ASSET_ONLY_FILES = [
  "runtime_map_metadata.json",
  "runtime_spawn_points.json",
  "battle_animation_table.json",
  "battle_anim_bundle.json",
  "asm_text.json",
  "move_names.json",
  "map_blocks.json",
  "phone_contacts.json",
  "sprite_palette_defaults.json",
  path.join("collision", "collision_permissions.json"),
  path.join("collision", "collision_stdscripts.json"),
  "sprite_anim_bundle.json",
  path.join("tilesets", "johto.json"),
  path.join("tilesets", "johto_metatiles.bin"),
  path.join("tilesets", "johto_palette_map.json"),
  path.join("tilesets", "players_room.json"),
  path.join("tilesets", "players_room_metatiles.bin"),
  path.join("tilesets", "players_room_palette_map.json"),
];
const REQUIRED_ASSET_ONLY_DIRECTORIES = ["tilesets"];

const CONSTANT_TO_NAME_OVERRIDES = {
  GOLDENROD_PP_SPEECH_HOUSE: "GoldenrodPPSpeechHouse",
  WHIRL_ISLAND_NW: "WhirlIslandNW",
  WHIRL_ISLAND_NE: "WhirlIslandNE",
  WHIRL_ISLAND_SW: "WhirlIslandSW",
  WHIRL_ISLAND_SE: "WhirlIslandSE",
  FAST_SHIP_CABINS_SE_SSE_CAPTAINS_CABIN: "FastShipCabins_SE_SSE_CaptainsCabin",
};

const DIRECTIONAL_SUFFIXES = new Set([
  "N",
  "S",
  "E",
  "W",
  "NE",
  "NW",
  "SE",
  "SW",
  "NNE",
  "NNW",
  "SSE",
  "SSW",
]);

const PALETTE_CONSTANTS = {
  PALETTE_AUTO: 0,
  PALETTE_DAY: 1,
  PALETTE_NITE: 2,
  PALETTE_MORN: 3,
  PALETTE_DARK: 4,
};
const NON_TRAINER_RE = /^\.(\w+):\s*db\s+"(.+)"/;
const CONTROL_CODE_REPLACEMENTS = {
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
const TEXT_DIRS = [path.join("data", "text"), path.join("data", "phone", "text")];
const TEXT_JSON_FILENAME = "asm_text.json";
const MOVE_NAMES_JSON_FILENAME = "move_names.json";
const SILENT_TEXT_TOKENS = new Set([
  "INCLUDE",
  "SECTION",
  "db",
  "dw",
  "else",
  "endc",
  "if",
  "sound_caught_mon",
  "sound_dex_fanfare_50_79",
  "sound_dex_fanfare_80_109",
  "sound_item",
  "sound_slot_machine_start",
  "text_low",
]);
const TEXT_TERMINATORS = new Set(["prompt", "text_promptbutton", "text_end", "done"]);
const TEXT_LINE_BREAK_TOKENS = new Set(["line", "cont", "next"]);
const NON_TRAINER_CONTACT_IDS = {
  PHONECONTACT_MOM: "PHONE_MOM",
  PHONECONTACT_BIKESHOP: "PHONE_OAK",
  PHONECONTACT_BILL: "PHONE_BILL",
  PHONECONTACT_ELM: "PHONE_ELM",
  PHONECONTACT_BUENA: "PHONE_BUENA",
};
const CONTACT_ID_ALIASES = {
  ...NON_TRAINER_CONTACT_IDS,
  PHONE_BIKESHOP: "PHONE_OAK",
  PHONE_BIKE_SHOP: "PHONE_OAK",
};
const TIME_MASKS = {
  MORN: 0x1,
  DAY: 0x2,
  NITE: 0x4,
  DARKNESS: 0x8,
};
const PAL_OW_LABELS = {
  PAL_OW_RED: 0,
  PAL_OW_BLUE: 1,
  PAL_OW_GREEN: 2,
  PAL_OW_BROWN: 3,
  PAL_OW_PINK: 4,
  PAL_OW_EMOTE: 5,
  PAL_OW_TREE: 6,
  PAL_OW_ROCK: 7,
};
const SPRITE_PALETTE_LINE_PATTERN =
  /overworld_sprite\s+[^,]+,\s*[^,]+,\s*[^,]+,\s*(PAL_OW_[A-Z0-9_]+)/i;
const INT_RE = /^([-+]?(?:0x[0-9a-fA-F]+|\$[0-9a-fA-F]+|0b[01_]+|%[01_]+|\d+))$/;
const FLAG_SYMBOLS = {
  ABSOLUTE_X: 0x00,
  RELATIVE_X: 0x01,
  B_OAM_XFLIP: 0x20,
  B_OAM_YFLIP: 0x40,
  OAM_XFLIP: 0x20,
  OAM_YFLIP: 0x40,
  OAM_PRIO: 0x80,
};

let disassemblyRoot = DEFAULT_DISASSEMBLY_ROOT;
let outDir = DEFAULT_OUT_DIR;

const readLines = (filePath) => fs.readFileSync(filePath, "utf8").split(/\r?\n/);
const normalizeIdentifier = (value) => String(value ?? "").trim().toUpperCase();
const parseIntToken = (token) => {
  const trimmed = String(token ?? "").trim();
  const match = INT_RE.exec(trimmed);
  if (!match) {
    throw new Error(`Cannot parse integer token: ${token}`);
  }
  const text = match[1];
  const prefix = text.slice(0, 2).toLowerCase();
  if (text.startsWith("$")) {
    return Number.parseInt(text.slice(1), 16);
  }
  if (text.startsWith("%")) {
    return Number.parseInt(text.slice(1).replace(/_/g, ""), 2);
  }
  if (prefix === "0x") {
    return Number.parseInt(text, 16);
  }
  if (prefix === "0b") {
    return Number.parseInt(text.slice(2).replace(/_/g, ""), 2);
  }
  return Number.parseInt(text, 10);
};
const extractTextString = (argument) => {
  if (!argument) {
    return "";
  }
  if (argument.includes('"')) {
    const start = argument.indexOf('"');
    const end = argument.lastIndexOf('"');
    if (start !== -1 && end > start) {
      return argument.substring(start + 1, end).replace(/@+$/g, "");
    }
  }
  return String(argument).trim().replace(/@+$/g, "");
};
const parseTextDigitCount = (argument) => {
  const tokens = String(argument)
    .split(",")
    .map((token) => token.trim().replace(/,$/, ""));
  for (const token of tokens.reverse()) {
    if (!token) {
      continue;
    }
    const digits = Number(token);
    if (!Number.isNaN(digits)) {
      return Math.max(1, digits);
    }
  }
  return 1;
};
const parseTextAsmFile = (filePath) => {
  const results = {};
  let label = null;
  const buffer = [];

  const flush = () => {
    if (label === null) {
      buffer.length = 0;
      return;
    }
    const chunk = buffer.join("").trim();
    buffer.length = 0;
    if (!chunk) {
      return;
    }
    const existing = results[label];
    results[label] = existing ? `${existing}\n\n${chunk}` : chunk;
  };

  const content = fs.readFileSync(filePath, "utf8");
  for (const raw of content.split(/\r?\n/)) {
    const line = raw.split(";", 1)[0].trimEnd();
    const stripped = line.trim();
    if (!stripped) {
      continue;
    }
    if (stripped.endsWith(":")) {
      flush();
      label = stripped.replace(/:+$/, "");
      continue;
    }
    if (label === null) {
      continue;
    }
    const firstSpaceIndex = stripped.search(/\s/);
    const token = firstSpaceIndex === -1 ? stripped : stripped.substring(0, firstSpaceIndex);
    const argument = firstSpaceIndex === -1 ? "" : stripped.substring(firstSpaceIndex).trim();

    if (SILENT_TEXT_TOKENS.has(token)) {
      continue;
    }
    if (TEXT_TERMINATORS.has(token)) {
      flush();
      continue;
    }
    if (token === "text") {
      buffer.push(extractTextString(argument));
      continue;
    }
    if (TEXT_LINE_BREAK_TOKENS.has(token)) {
      buffer.push("\n");
      buffer.push(extractTextString(argument));
      continue;
    }
    if (token === "para") {
      buffer.push("\n\n");
      buffer.push(extractTextString(argument));
      continue;
    }
    if (token === "text_start") {
      buffer.push("\n\n");
      continue;
    }
    if (token === "text_ram") {
      const match = /wStringBuffer(\d+)/i.exec(argument);
      buffer.push(match ? `<STRING_BUFFER_${match[1]}>` : "@");
      continue;
    }
    if (token === "text_decimal") {
      buffer.push("@".repeat(parseTextDigitCount(argument)));
      continue;
    }
    if (token === "text_today") {
      buffer.push("<TODAY>");
      continue;
    }
    if (token === "text_pause") {
      buffer.push("…");
    }
  }
  flush();
  return results;
};
const parseMoveNamesAsmFile = (filePath) => {
  const names = [];
  const content = fs.readFileSync(filePath, "utf8");
  for (const raw of content.split(/\r?\n/)) {
    const line = raw.split(";", 1)[0].trimEnd();
    const stripped = line.trim();
    if (!stripped.startsWith("li ")) {
      continue;
    }
    const start = stripped.indexOf('"');
    const end = stripped.lastIndexOf('"');
    if (start === -1 || end <= start) {
      continue;
    }
    names.push(stripped.substring(start + 1, end));
  }
  return names;
};
const parseFlagExpression = (raw) => {
  const parts = String(raw ?? "").split("|").map((part) => part.trim()).filter(Boolean);
  let value = 0;
  for (const part of parts) {
    if (Object.prototype.hasOwnProperty.call(FLAG_SYMBOLS, part)) {
      value |= FLAG_SYMBOLS[part];
      continue;
    }
    value |= parseIntToken(part);
  }
  return value;
};
const parseSpriteAttrExpression = (raw) => {
  const parts = String(raw ?? "").replace(/,/g, "|").split("|").map((part) => part.trim()).filter(Boolean);
  let value = 0;
  for (const part of parts) {
    if (Object.prototype.hasOwnProperty.call(PAL_OW_LABELS, part)) {
      value |= PAL_OW_LABELS[part];
      continue;
    }
    if (part === "OAM_BANK1") {
      value |= 0x08;
      continue;
    }
    if (part === "OAM_PAL0" || part === "OAM_PAL1") {
      continue;
    }
    if (Object.prototype.hasOwnProperty.call(FLAG_SYMBOLS, part)) {
      value |= FLAG_SYMBOLS[part];
      continue;
    }
    value |= parseIntToken(part);
  }
  return value;
};
const parseBattleAnimOamFlags = (raw) => {
  const tokens = String(raw ?? "")
    .split("|")
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => normalizeIdentifier(part));
  return {
    xflip: tokens.includes("OAM_XFLIP"),
    yflip: tokens.includes("OAM_YFLIP"),
  };
};
const TILESET_PALETTE_LABELS = {
  GRAY: 0,
  RED: 1,
  GREEN: 2,
  WATER: 3,
  YELLOW: 4,
  BROWN: 5,
  ROOF: 6,
  TEXT: 7,
};
const parseTilesetPaletteMap = (content) => {
  const paletteIndices = [];
  let repeatCount = 0;
  let repeatValues = null;

  const pushValues = (values) => {
    for (const value of values) {
      const low = value & 0x0f;
      const high = (value >> 4) & 0x0f;
      paletteIndices.push(low, high);
    }
  };

  for (const rawLine of String(content ?? "").split(/\r?\n/)) {
    const trimmed = rawLine.split(";")[0].trim();
    if (!trimmed) {
      continue;
    }
    const lower = trimmed.toLowerCase();
    if (lower.startsWith("rept")) {
      const parts = trimmed.split(/\s+/);
      repeatCount = Number(parts[1] ?? 0);
      repeatValues = null;
      continue;
    }
    if (lower.startsWith("endr")) {
      if (repeatCount > 0 && repeatValues) {
        for (let index = 0; index < repeatCount; index += 1) {
          pushValues(repeatValues);
        }
      }
      repeatCount = 0;
      repeatValues = null;
      continue;
    }
    if (lower.startsWith("db")) {
      const values = trimmed
        .replace(/^db/i, "")
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean)
        .map((value) => Number(value.replace("$", "0x")));
      if (values.some((value) => Number.isNaN(value))) {
        throw new Error(`Invalid tileset palette byte line '${trimmed}'.`);
      }
      if (repeatCount > 0) {
        repeatValues = values;
      } else {
        pushValues(values);
      }
      continue;
    }
    if (lower.startsWith("tilepal")) {
      const tokens = trimmed
        .replace(/^tilepal/i, "")
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean);
      if (tokens.length < 2) {
        continue;
      }
      const bank = Number(tokens[0] ?? 0);
      const paletteTokens = tokens.slice(1);
      const indices = paletteTokens.map((token) => {
        const value = TILESET_PALETTE_LABELS[String(token).toUpperCase()];
        if (value === undefined) {
          throw new Error(`Unknown palette token '${token}'.`);
        }
        return ((Number.isNaN(bank) ? 0 : bank) << 3) | value;
      });
      paletteIndices.push(...indices);
    }
  }

  return paletteIndices;
};
const firstExistingPath = (candidates) => candidates.find((candidate) => fs.existsSync(candidate)) ?? null;
const hasCompleteRuntimeDisassembly = (root) => {
  const requiredPaths = [
    path.join(root, "constants", "map_constants.asm"),
    path.join(root, "data", "maps", "maps.asm"),
    path.join(root, "data", "maps", "spawn_points.asm"),
    path.join(root, "data", "moves", "animations.asm"),
  ];
  return requiredPaths.every((candidate) => fs.existsSync(candidate));
};
const parseBattleAnimFramesetCommand = (line) => {
  const stripped = String(line ?? "").trim();
  if (!stripped) {
    return null;
  }
  if (stripped.startsWith("oamwait")) {
    const [, rawDuration = "0"] = stripped.split(/\s+/, 2);
    return {
      command: "wait",
      oam_set: null,
      duration: Math.max(0, parseIntToken(rawDuration.replace(/,$/, ""))),
      xflip: false,
      yflip: false,
    };
  }
  if (stripped === "oamend" || stripped === "oamrestart" || stripped === "oamdelete") {
    return {
      command: stripped === "oamend" ? "end" : stripped === "oamrestart" ? "restart" : "delete",
      oam_set: null,
      duration: 0,
      xflip: false,
      yflip: false,
    };
  }
  if (!stripped.startsWith("oamframe")) {
    return null;
  }
  const args = stripped
    .slice("oamframe".length)
    .split(",")
    .map((token) => token.trim())
    .filter(Boolean);
  if (args.length < 2) {
    throw new Error(`Invalid battle animation frameset entry: ${line}`);
  }
  const flags = args.slice(2).map((flag) => normalizeIdentifier(flag));
  return {
    command: "frame",
    oam_set: normalizeIdentifier(args[0]),
    duration: Math.max(0, parseIntToken(args[1])),
    xflip: flags.includes("B_OAM_XFLIP"),
    yflip: flags.includes("B_OAM_YFLIP"),
  };
};
const parseBattleAnimFramesets = (filePath) => {
  const lines = readLines(filePath);
  const tablePattern = /dw\s+([.\w]+)\s*;\s*(BATTLE_ANIM_FRAMESET_[A-Z0-9_]+)/;
  const labelPattern = /^\.([A-Za-z0-9_]+):$/;
  const labelMap = new Map();
  for (const raw of lines) {
    const match = tablePattern.exec(raw);
    if (match) {
      labelMap.set(normalizeIdentifier(match[2]), match[1].replace(/^\./, "").trim());
    }
  }

  const byLabel = new Map();
  const labelOrder = [];
  let currentLabel = null;
  let pending = [];
  for (const raw of lines) {
    const stripped = raw.split(";", 1)[0].trim();
    if (!stripped) {
      continue;
    }
    const labelMatch = labelPattern.exec(stripped);
    if (labelMatch) {
      if (currentLabel) {
        byLabel.set(currentLabel, pending);
      }
      currentLabel = labelMatch[1];
      labelOrder.push(currentLabel);
      pending = [];
      continue;
    }
    if (!currentLabel) {
      continue;
    }
    const entry = parseBattleAnimFramesetCommand(stripped);
    if (entry) {
      pending.push(entry);
    }
  }
  if (currentLabel) {
    byLabel.set(currentLabel, pending);
  }

  const hasTerminator = (entries) =>
    entries.some((entry) => entry.command === "delete" || entry.command === "restart" || entry.command === "end");
  const labelIndex = new Map();
  labelOrder.forEach((label, index) => labelIndex.set(label, index));
  const expandedByLabel = new Map();
  const expandLabel = (label) => {
    if (expandedByLabel.has(label)) {
      return expandedByLabel.get(label);
    }
    const start = labelIndex.get(label);
    if (start === undefined) {
      throw new Error(`Missing battle animation frameset label: ${label}`);
    }
    const merged = [];
    for (let index = start; index < labelOrder.length; index += 1) {
      const entries = byLabel.get(labelOrder[index]) ?? [];
      merged.push(...entries.map((entry) => ({ ...entry })));
      if (hasTerminator(entries)) {
        break;
      }
    }
    expandedByLabel.set(label, merged);
    return merged;
  };

  const framesets = {};
  for (const [constName, label] of labelMap) {
    framesets[constName] = expandLabel(label);
  }
  return framesets;
};
const parseBattleAnimOamSets = (filePath) => {
  const labelData = new Map();
  const ensureLabel = (label) => {
    const normalized = String(label).trim().replace(/^\./, "");
    if (!labelData.has(normalized)) {
      labelData.set(normalized, []);
    }
    return labelData.get(normalized);
  };
  const lines = readLines(filePath);
  let currentLabel = null;
  for (const raw of lines) {
    const body = raw.split(";", 1)[0].trimEnd();
    const stripped = body.trim();
    if (!stripped) {
      continue;
    }
    if (stripped.endsWith(":")) {
      currentLabel = stripped.slice(0, -1).trim().replace(/^\./, "");
      continue;
    }
    if (!currentLabel) {
      continue;
    }
    if (stripped.startsWith("dbsprite")) {
      const args = stripped
        .slice("dbsprite".length)
        .split(",")
        .map((token) => token.trim())
        .filter(Boolean);
      if (args.length < 6) {
        continue;
      }
      const { xflip, yflip } = parseBattleAnimOamFlags(args[5]);
      ensureLabel(currentLabel).push({
        x: parseIntToken(args[0]) * 8 + parseIntToken(args[2]),
        y: parseIntToken(args[1]) * 8 + parseIntToken(args[3]),
        tile_id: parseIntToken(args[4]),
        xflip,
        yflip,
      });
      continue;
    }
    if (!stripped.startsWith("db ")) {
      continue;
    }
    const args = stripped
      .slice("db".length)
      .split(",")
      .map((token) => token.trim())
      .filter(Boolean);
    if (args.length === 1) {
      continue;
    }
    if (args.length < 4) {
      continue;
    }
    const { xflip, yflip } = parseBattleAnimOamFlags(args[3]);
    ensureLabel(currentLabel).push({
      x: parseIntToken(args[0]),
      y: parseIntToken(args[1]),
      tile_id: parseIntToken(args[2]),
      xflip,
      yflip,
    });
  }

  const oamSets = {};
  for (const raw of lines) {
    const stripped = raw.split(";", 1)[0].trim();
    if (!stripped || !stripped.startsWith("battleanimoam")) {
      continue;
    }
    const parts = stripped.slice("battleanimoam".length).split(",");
    if (parts.length < 3) {
      continue;
    }
    const comment = raw.split(";", 2)[1]?.trim();
    if (!comment || !comment.startsWith("BATTLE_ANIM_OAMSET_")) {
      continue;
    }
    const name = normalizeIdentifier(comment);
    const dataLabel = parts[2].trim().replace(/^\./, "");
    const entryCount = parseIntToken(parts[1]);
    const entries = labelData.get(dataLabel) ?? [];
    oamSets[name] = {
      name,
      tile_offset: parseIntToken(parts[0]),
      entries: entryCount > 0 ? entries.slice(0, entryCount) : [...entries],
    };
  }
  return oamSets;
};
const parseIncbinLine = (rawLine) => {
  const body = String(rawLine ?? "").split(";", 1)[0].trim();
  if (!body.includes("INCBIN")) {
    return null;
  }
  const match = /^(.*?)\bINCBIN\s+"([^"]+)"/.exec(body);
  if (!match) {
    return null;
  }
  const labelPrefix = match[1] ?? "";
  const relPath = match[2];
  const labels = [...labelPrefix.matchAll(/([A-Za-z0-9_]+):/g)].map((labelMatch) => labelMatch[1]);
  if (!labels.length) {
    return null;
  }
  return { labels, relPath };
};
const parseBattleAnimObjectConstants = (constantsPath) => {
  const constants = [];
  let capture = false;
  for (const raw of readLines(constantsPath)) {
    if (!capture && raw.includes("BattleAnimObjects indexes")) {
      capture = true;
      continue;
    }
    const line = raw.split(";", 1)[0].trim();
    if (!line) {
      if (capture && constants.length) {
        break;
      }
      continue;
    }
    const match = /^const\s+(BATTLE_ANIM_OBJ_[A-Z0-9_]+)/.exec(line);
    if (match) {
      capture = true;
      constants.push(normalizeIdentifier(match[1]));
      continue;
    }
    if (!capture) {
      continue;
    }
    if (
      constants.length &&
      (line.includes("BattleAnimGFX indexes") ||
        /^const\s+(BATTLE_ANIM_GFX_[A-Z0-9_]+)/.test(line) ||
        line.startsWith("DEF NUM_BATTLE_ANIM_GFX"))
    ) {
      break;
    }
    if (constants.length && !line.startsWith("const_def") && !line.startsWith("const ")) {
      break;
    }
  }
  if (!constants.length) {
    throw new Error(`Could not parse battle animation object constants from ${constantsPath}`);
  }
  return constants;
};
const parseBattleAnimGfxConstants = (constantsPath) => {
  const constants = [];
  let capture = false;
  let currentValue = 0;
  for (const raw of readLines(constantsPath)) {
    const line = raw.split(";", 1)[0].trim();
    if (!line) {
      if (capture && constants.length) {
        break;
      }
      continue;
    }
    const constDefMatch = /^const_def(?:\s+(.+))?$/.exec(line);
    if (constDefMatch) {
      if (!capture && constants.length) {
        break;
      }
      currentValue = constDefMatch[1] ? parseIntToken(constDefMatch[1]) : 0;
      if (!capture && line.includes("BattleAnimGFX indexes")) {
        capture = true;
      }
      continue;
    }
    if (!capture && line.includes("BattleAnimGFX indexes")) {
      capture = true;
      continue;
    }
    const constMatch = /^const\s+(BATTLE_ANIM_GFX_[A-Z0-9_]+)/.exec(line);
    if (constMatch) {
      capture = true;
      constants.push({
        name: normalizeIdentifier(constMatch[1]),
        value: currentValue,
      });
      currentValue += 1;
      continue;
    }
    if (!capture) {
      continue;
    }
    if (/^const_skip\b/.test(line)) {
      currentValue += 1;
      continue;
    }
    if (line.startsWith("DEF NUM_BATTLE_ANIM_GFX")) {
      break;
    }
    if (constants.length) {
      break;
    }
  }
  if (!constants.length) {
    throw new Error(`Could not parse battle animation gfx constants from ${constantsPath}`);
  }
  return constants;
};
const parseBattleAnimGfxEntries = (objectGfxPath) => {
  const entries = new Map();
  let entryIndex = 0;
  for (const raw of readLines(objectGfxPath)) {
    const line = raw.split(";", 1)[0].trim();
    if (!line) {
      continue;
    }
    if (line.startsWith("assert_table_length")) {
      break;
    }
    const match = /^anim_obj_gfx\s+([^,]+)\s*,\s*([A-Za-z0-9_]+)/.exec(line);
    if (!match) {
      continue;
    }
    entries.set(entryIndex, [parseIntToken(match[1]), match[2].trim()]);
    entryIndex += 1;
  }
  if (!entries.size) {
    throw new Error(`Could not parse battle animation gfx table from ${objectGfxPath}`);
  }
  return entries;
};

const resetGeneratedTargets = () => {
  for (const fileName of GENERATED_FILES) {
    fs.rmSync(path.join(outDir, fileName), { force: true });
  }
  for (const directoryName of GENERATED_DIRECTORIES) {
    fs.rmSync(path.join(outDir, directoryName), { recursive: true, force: true });
  }
};

const assertGeneratedTargetsExist = () => {
  const missing = [];
  for (const fileName of REQUIRED_ASSET_ONLY_FILES) {
    if (!fs.existsSync(path.join(outDir, fileName))) {
      missing.push(fileName);
    }
  }
  for (const directoryName of REQUIRED_ASSET_ONLY_DIRECTORIES) {
    if (!fs.existsSync(path.join(outDir, directoryName))) {
      missing.push(`${directoryName}/`);
    }
  }
  if (missing.length) {
    throw new Error(
      `Missing generated runtime assets in ${outDir}: ${missing.join(", ")}`
    );
  }
};

const applyTextReplacements = (text) => {
  let result = String(text ?? "");
  for (const [token, replacement] of Object.entries(CONTROL_CODE_REPLACEMENTS)) {
    result = result.split(token).join(replacement);
  }
  return result;
};

const mapConstantToName = (constant) => {
  const override = CONSTANT_TO_NAME_OVERRIDES[constant];
  if (override) {
    return override;
  }
  const parts = constant.split("_");
  let suffixIndex = null;
  for (let i = 0; i < parts.length; i += 1) {
    if (DIRECTIONAL_SUFFIXES.has(parts[i])) {
      suffixIndex = i;
      break;
    }
  }
  const baseParts = suffixIndex !== null ? parts.slice(0, suffixIndex) : parts;
  const suffixParts = suffixIndex !== null ? parts.slice(suffixIndex) : [];
  const baseName = baseParts
    .map((part) => {
      if (/^\d+$/.test(part)) {
        return part;
      }
      if (/\d/.test(part) && /^[A-Z0-9]+$/.test(part)) {
        return part.toUpperCase();
      }
      return part.charAt(0) + part.slice(1).toLowerCase();
    })
    .join("");
  if (suffixIndex !== null) {
    return `${baseName}_${suffixParts.join("_")}`;
  }
  return baseName;
};

const loadMapEnvironmentMap = () => {
  const mapsPath = path.join(disassemblyRoot, "data", "maps", "maps.asm");
  const environments = {};
  for (const raw of readLines(mapsPath)) {
    const cleaned = raw.split(";", 1)[0].trim();
    if (!cleaned.startsWith("map ")) {
      continue;
    }
    const parts = cleaned.slice("map ".length).split(",").map((part) => part.trim());
    if (parts.length < 3) {
      continue;
    }
    environments[parts[0]] = parts[2];
  }
  return environments;
};

const loadMapPhoneServiceMap = () => {
  const mapsPath = path.join(disassemblyRoot, "data", "maps", "maps.asm");
  const phoneServices = {};
  for (const raw of readLines(mapsPath)) {
    const cleaned = raw.split(";", 1)[0].trim();
    if (!cleaned.startsWith("map ")) {
      continue;
    }
    const parts = cleaned.split(",").map((part) => part.trim());
    if (parts.length < 8) {
      continue;
    }
    const tokens = parts[0].split(/\s+/);
    if (tokens.length < 2) {
      continue;
    }
    const mapName = tokens[1];
    const phoneFlagToken = parts[5].toUpperCase();
    const phoneFlag =
      phoneFlagToken === "TRUE" ? 1 : phoneFlagToken === "FALSE" ? 0 : Number.parseInt(parts[5], 0);
    const paletteValue = PALETTE_CONSTANTS[parts[6]];
    if (!mapName || Number.isNaN(phoneFlag) || paletteValue === undefined) {
      continue;
    }
    phoneServices[mapName] = (phoneFlag << 4) | paletteValue;
  }
  return phoneServices;
};

const exportRuntimeMapMetadata = () => {
  const constantsPath = path.join(disassemblyRoot, "constants", "map_constants.asm");
  const environments = loadMapEnvironmentMap();
  const phoneServices = loadMapPhoneServiceMap();
  const metadata = {};
  let currentGroup = null;
  let groupId = 0;

  for (const raw of readLines(constantsPath)) {
    const mapIdMatch = raw.match(/;\s*(\d+)\s*$/);
    const mapId = mapIdMatch ? Number(mapIdMatch[1]) : null;
    const line = raw.split(";", 1)[0].trim();
    if (!line) {
      continue;
    }
    if (line.startsWith("newgroup")) {
      currentGroup = line.split(/\s+/)[1] ?? null;
      groupId += 1;
      continue;
    }
    if (!line.startsWith("map_const") || !currentGroup) {
      continue;
    }
    const tokens = line.split(",").map((token) => token.trim());
    const header = tokens[0].split(/\s+/);
    const constant = header[1];
    const width = Number(tokens[1]);
    const height = Number(tokens[2]);
    if (!constant || mapId === null || Number.isNaN(width) || Number.isNaN(height)) {
      continue;
    }
    const name = mapConstantToName(constant);
    metadata[constant] = {
      constant,
      name,
      groupName: currentGroup,
      groupId,
      mapId,
      width,
      height,
      environment: environments[constant] ?? environments[name] ?? null,
      phoneService: phoneServices[name] ?? 0,
    };
  }

  fs.writeFileSync(
    path.join(outDir, "runtime_map_metadata.json"),
    `${JSON.stringify(metadata, null, 2)}\n`
  );
};

const exportRuntimeSpawnPoints = () => {
  const metadata = JSON.parse(
    fs.readFileSync(path.join(outDir, "runtime_map_metadata.json"), "utf8")
  );
  const spawnPointsPath = path.join(disassemblyRoot, "data", "maps", "spawn_points.asm");
  const spawnPoints = {};
  let index = 0;
  for (const raw of readLines(spawnPointsPath)) {
    const line = raw.split(";", 1)[0].trim();
    if (!line.startsWith("spawn ")) {
      continue;
    }
    const tokens = line
      .slice("spawn".length)
      .split(",")
      .map((token) => token.trim())
      .filter(Boolean);
    const mapConstant = tokens[0];
    const tileX = Number(tokens[1]);
    const tileY = Number(tokens[2]);
    const entry = metadata[mapConstant];
    const blockStride = 2;
    spawnPoints[index] = {
      identifier: index,
      mapConstant,
      mapName: entry?.name ?? mapConstant,
      groupId: entry?.groupId ?? -1,
      mapId: entry?.mapId ?? -1,
      tileX,
      tileY,
      groupName: entry?.groupName ?? "N_A",
      metatileX: tileX >= 0 ? Math.trunc(tileX / blockStride) : -1,
      metatileY: tileY >= 0 ? Math.trunc(tileY / blockStride) : -1,
      subtileX: tileX >= 0 ? tileX % blockStride : -1,
      subtileY: tileY >= 0 ? tileY % blockStride : -1,
    };
    index += 1;
  }

  fs.writeFileSync(
    path.join(outDir, "runtime_spawn_points.json"),
    `${JSON.stringify(spawnPoints, null, 2)}\n`
  );
};

const exportBattleAnimationTable = () => {
  const animationsPath = path.join(disassemblyRoot, "data", "moves", "animations.asm");
  const table = [];
  let capture = false;
  for (const raw of readLines(animationsPath)) {
    const line = raw.split(";", 1)[0].trim();
    if (!capture) {
      if (line.startsWith("BattleAnimations::")) {
        capture = true;
      }
      continue;
    }
    if (line.startsWith("assert_table_length")) {
      break;
    }
    if (!line.startsWith("dw")) {
      continue;
    }
    const token = line.split(/\s+/)[1];
    if (token) {
      table.push(token);
    }
  }
  fs.writeFileSync(
    path.join(outDir, "battle_animation_table.json"),
    `${JSON.stringify(table, null, 2)}\n`
  );
};

const exportBattleAnimationRuntimeSources = () => {
  const battleAnimSourceDir = path.join(disassemblyRoot, "data", "battle_anims");
  const battleAnimTargetDir = path.join(outDir, "battle_anims");
  const constantsSourcePath = path.join(
    disassemblyRoot,
    "constants",
    "battle_anim_constants.asm"
  );
  const constantsTargetDir = path.join(outDir, "constants");

  fs.mkdirSync(battleAnimTargetDir, { recursive: true });
  fs.mkdirSync(constantsTargetDir, { recursive: true });
  for (const entry of ["objects.asm", "object_gfx.asm", "framesets.asm", "oam.asm"]) {
    fs.copyFileSync(
      path.join(battleAnimSourceDir, entry),
      path.join(battleAnimTargetDir, entry)
    );
  }
  fs.copyFileSync(
    constantsSourcePath,
    path.join(outDir, "battle_anim_constants.asm")
  );
  fs.copyFileSync(
    constantsSourcePath,
    path.join(constantsTargetDir, "battle_anim_constants.asm")
  );
};

const exportBattleAnimationBundle = ({
  projectRoot = DEFAULT_PROJECT_ROOT,
  includeOutDirSources = true,
} = {}) => {
  const standaloneDisassemblyRoot = path.join(
    projectRoot,
    ".next-electron",
    "standalone",
    "apps",
    "web",
    "public",
    "disassembly"
  );
  const objectsPath = firstExistingPath([
    path.join(disassemblyRoot, "data", "battle_anims", "objects.asm"),
    path.join(standaloneDisassemblyRoot, "data", "battle_anims", "objects.asm"),
    ...(includeOutDirSources ? [path.join(outDir, "battle_anims", "objects.asm")] : []),
  ]);
  const framesetsPath = firstExistingPath([
    path.join(disassemblyRoot, "data", "battle_anims", "framesets.asm"),
    path.join(standaloneDisassemblyRoot, "data", "battle_anims", "framesets.asm"),
    ...(includeOutDirSources ? [path.join(outDir, "battle_anims", "framesets.asm")] : []),
  ]);
  const oamPath = firstExistingPath([
    path.join(disassemblyRoot, "data", "battle_anims", "oam.asm"),
    path.join(standaloneDisassemblyRoot, "data", "battle_anims", "oam.asm"),
    ...(includeOutDirSources ? [path.join(outDir, "battle_anims", "oam.asm")] : []),
  ]);
  const objectGfxPath = firstExistingPath([
    path.join(disassemblyRoot, "data", "battle_anims", "object_gfx.asm"),
    path.join(standaloneDisassemblyRoot, "data", "battle_anims", "object_gfx.asm"),
    ...(includeOutDirSources ? [path.join(outDir, "battle_anims", "object_gfx.asm")] : []),
  ]);
  const constantsPath = firstExistingPath([
    path.join(disassemblyRoot, "constants", "battle_anim_constants.asm"),
    path.join(standaloneDisassemblyRoot, "constants", "battle_anim_constants.asm"),
    ...(includeOutDirSources
      ? [
          path.join(outDir, "battle_anim_constants.asm"),
          path.join(outDir, "constants", "battle_anim_constants.asm"),
        ]
      : []),
  ]);
  const gfxSourceTablePath = firstExistingPath([
    path.join(projectRoot, "assets", "gfx", "battle_anims.asm"),
    path.join(disassemblyRoot, "gfx", "battle_anims.asm"),
    path.join(standaloneDisassemblyRoot, "gfx", "battle_anims.asm"),
    ...(includeOutDirSources ? [path.join(outDir, "battle_anims_gfx.asm")] : []),
  ]);

  const required = [
    objectsPath,
    framesetsPath,
    oamPath,
    objectGfxPath,
    constantsPath,
    gfxSourceTablePath,
  ];
  if (required.some((value) => !value)) {
    throw new Error("Missing battle animation sources needed to build battle_anim_bundle.json");
  }

  const bundle = {
    objects: {},
    framesets: {},
    oam_sets: {},
    gfx_table: {},
    gfx_sources: {},
  };

  const objectConstants = parseBattleAnimObjectConstants(constantsPath);
  let objectIndex = 0;
  for (const raw of readLines(objectsPath)) {
    const line = raw.split(";", 1)[0].trim();
    if (!line || !line.startsWith("battleanimobj")) {
      continue;
    }
    const objectId = objectConstants[objectIndex];
    if (!objectId) {
      throw new Error(
        `Found more battleanimobj entries than BATTLE_ANIM_OBJ_* constants while parsing ${objectsPath}`
      );
    }
    const args = line
      .slice("battleanimobj".length)
      .split(",")
      .map((token) => token.trim())
      .filter(Boolean);
    if (args.length < 6) {
      continue;
    }
    bundle.objects[objectId] = {
      object_id: objectId,
      flags: parseFlagExpression(args[0]),
      fix_y: parseIntToken(args[1]),
      function: normalizeIdentifier(args[3]) === "BATTLE_ANIM_FUNC_NULL"
        ? null
        : normalizeIdentifier(args[3]),
      frameset: normalizeIdentifier(args[2]),
      palette: normalizeIdentifier(args[4]),
      gfx_id: normalizeIdentifier(args[5]),
    };
    objectIndex += 1;
  }
  if (objectIndex !== objectConstants.length) {
    throw new Error(
      `Parsed ${objectIndex} battle animation objects from ${objectsPath}, expected ${objectConstants.length} from ${constantsPath}`
    );
  }

  bundle.framesets = parseBattleAnimFramesets(framesetsPath);

  bundle.oam_sets = parseBattleAnimOamSets(oamPath);

  const gfxEntries = parseBattleAnimGfxEntries(objectGfxPath);
  const gfxConstants = parseBattleAnimGfxConstants(constantsPath);
  const zeroEntry = gfxEntries.get(0);
  if (zeroEntry) {
    bundle.gfx_table.BATTLE_ANIM_GFX_0 = zeroEntry;
  }
  for (const { name, value } of gfxConstants) {
    const entry = gfxEntries.get(value);
    if (!entry) {
      throw new Error(
        `Missing battle animation gfx table entry ${value} for ${name} while parsing ${objectGfxPath}`
      );
    }
    bundle.gfx_table[name] = entry;
  }

  for (const raw of readLines(gfxSourceTablePath)) {
    const parsed = parseIncbinLine(raw);
    if (!parsed) {
      continue;
    }
    for (const label of parsed.labels) {
      bundle.gfx_sources[label.trim()] = parsed.relPath;
    }
  }

  fs.writeFileSync(
    path.join(outDir, "battle_anim_bundle.json"),
    `${JSON.stringify(bundle, null, 2)}\n`
  );
};

const exportBattleAnimationGfxSourceTable = ({
  projectRoot = DEFAULT_PROJECT_ROOT,
} = {}) => {
  const standaloneDisassemblyRoot = path.join(
    projectRoot,
    ".next-electron",
    "standalone",
    "apps",
    "web",
    "public",
    "disassembly"
  );
  const sourceCandidates = [
    path.join(projectRoot, "assets", "gfx", "battle_anims.asm"),
    path.join(disassemblyRoot, "gfx", "battle_anims.asm"),
    path.join(standaloneDisassemblyRoot, "gfx", "battle_anims.asm"),
    path.join(outDir, "battle_anims_gfx.asm"),
  ];
  const sourcePath = sourceCandidates.find((candidate) => fs.existsSync(candidate));
  if (!sourcePath) {
    throw new Error(
      `Missing battle animation gfx source table: tried ${sourceCandidates.join(", ")}`
    );
  }
  const assetsRoot = path.join(projectRoot, "assets");
  const normalized = readLines(sourcePath).map((line) => {
    const match = line.match(/^(.*INCBIN\s+")([^"]+)(".*)$/);
    if (!match) {
      return line;
    }
    const [, prefix, relPath, suffix] = match;
    const directPath = path.join(assetsRoot, relPath);
    if (fs.existsSync(directPath)) {
      return line;
    }
    if (relPath.endsWith(".lz")) {
      const uncompressedPath = relPath.slice(0, -3);
      if (fs.existsSync(path.join(assetsRoot, uncompressedPath))) {
        return `${prefix}${uncompressedPath}${suffix}`;
      }
    }
    return line;
  });
  fs.writeFileSync(path.join(outDir, "battle_anims_gfx.asm"), `${normalized.join("\n")}\n`);
};

const iterSections = (lines, prefix) => {
  let currentName = null;
  let buffer = [];
  const sections = [];
  for (const rawLine of lines) {
    const line = rawLine.split(";", 1)[0].trim();
    if (!line) {
      continue;
    }
    if (line.startsWith(prefix)) {
      if (currentName !== null) {
        sections.push([currentName, buffer]);
        buffer = [];
      }
      currentName = line.split(":", 1)[0].replace(/^\./, "");
    } else if (currentName !== null) {
      buffer.push(line);
    }
  }
  if (currentName !== null) {
    sections.push([currentName, buffer]);
  }
  return sections;
};

const parseSpriteAnimBundle = (spriteAnimsDir, constantsPath) => {
  const oamLines = readLines(path.join(spriteAnimsDir, "oam.asm"));
  const framesetLines = readLines(path.join(spriteAnimsDir, "framesets.asm"));
  const objectLines = readLines(path.join(spriteAnimsDir, "objects.asm"));
  const constantsLines = readLines(constantsPath);

  const bundle = {
    oam_sets: {},
    framesets: {},
    objects: {},
  };

  const oamSections = Object.fromEntries(iterSections(oamLines, ".OAMData_"));
  const oamRefs = {};
  for (const rawLine of oamLines) {
    const stripped = rawLine.trimStart();
    if (!stripped.startsWith("spriteanimoam")) {
      continue;
    }
    const [mainPart, commentPart] = stripped.split(";", 2);
    const comment = String(commentPart ?? "").trim();
    const parts = mainPart
      .slice("spriteanimoam".length)
      .split(",")
      .map((part) => part.trim())
      .filter(Boolean);
    if (parts.length !== 2 || !comment.startsWith("SPRITE_ANIM_OAMSET_")) {
      continue;
    }
    oamRefs[comment] = {
      tile_offset: parseIntToken(parts[0]),
      data_label: parts[1].replace(/^\./, ""),
    };
  }
  for (const [name, ref] of Object.entries(oamRefs)) {
    const body = oamSections[ref.data_label] ?? [];
    const pieces = [];
    for (const entry of body) {
      if (!entry.startsWith("dbsprite")) {
        continue;
      }
      const args = entry
        .slice("dbsprite".length)
        .split(",")
        .map((arg) => arg.trim());
      if (args.length !== 6) {
        continue;
      }
      pieces.push({
        x: parseIntToken(args[0]) * 8 + parseIntToken(args[2]),
        y: parseIntToken(args[1]) * 8 + parseIntToken(args[3]),
        tile: parseIntToken(args[4]),
        attributes: parseSpriteAttrExpression(args[5]),
      });
    }
    bundle.oam_sets[name] = {
      name,
      tile_offset: ref.tile_offset,
      pieces,
    };
  }

  const pointerLabels = [];
  let inTable = false;
  for (const rawLine of framesetLines) {
    const stripped = rawLine.trim();
    if (!stripped) {
      continue;
    }
    if (stripped.startsWith("table_width")) {
      inTable = true;
      continue;
    }
    if (inTable && stripped.startsWith("dw")) {
      const parts = stripped.split(/\s+/);
      if (parts[1]) {
        pointerLabels.push(parts[1].replace(/^\./, ""));
      }
      continue;
    }
    if (inTable && stripped.startsWith("assert_table_length")) {
      break;
    }
  }
  const constantNames = [];
  for (const raw of constantsLines) {
    const stripped = raw.trim();
    if (stripped.startsWith("const SPRITE_ANIM_FRAMESET_")) {
      constantNames.push(stripped.split(/\s+/)[1]);
    } else if (stripped.startsWith("DEF NUM_SPRITE_ANIM_FRAMESETS")) {
      break;
    }
  }
  for (const [name, body] of iterSections(framesetLines, ".Frameset_")) {
    const steps = [];
    for (const entry of body) {
      if (entry.startsWith("oamframe")) {
        const args = entry
          .slice("oamframe".length)
          .split(",")
          .map((part) => part.trim())
          .filter(Boolean);
        if (args.length < 2) {
          continue;
        }
        steps.push({
          oam_set: args[0],
          duration: parseIntToken(args[1]),
          attr_flags: args.length > 2 ? parseSpriteAttrExpression(args.slice(2).join(",")) : 0,
          command: "frame",
        });
      } else if (entry.startsWith("oamwait")) {
        const args = entry
          .slice("oamwait".length)
          .split(",")
          .map((part) => part.trim())
          .filter(Boolean);
        steps.push({ oam_set: null, duration: args.length ? parseIntToken(args[0]) : 0, attr_flags: 0, command: "wait" });
      } else if (entry.startsWith("oamrestart")) {
        steps.push({ oam_set: null, duration: 0, attr_flags: 0, command: "restart" });
      } else if (entry.startsWith("oamdelete")) {
        steps.push({ oam_set: null, duration: 0, attr_flags: 0, command: "delete" });
      } else if (entry.startsWith("oamend")) {
        steps.push({ oam_set: null, duration: 0, attr_flags: 0, command: "end" });
      }
    }
    bundle.framesets[name] = { name, steps };
  }
  for (let i = 0; i < Math.min(constantNames.length, pointerLabels.length); i += 1) {
    const constName = constantNames[i];
    const label = pointerLabels[i];
    if (bundle.framesets[label]) {
      bundle.framesets[constName] = bundle.framesets[label];
    }
  }

  let pendingName = null;
  for (const rawLine of objectLines) {
    const stripped = rawLine.trim();
    if (stripped.startsWith("; SPRITE_ANIM_OBJ_")) {
      pendingName = stripped.replace(/^;\s*/, "");
      continue;
    }
    if (pendingName && stripped.startsWith("db ")) {
      const args = stripped
        .slice("db ".length)
        .split(",")
        .map((part) => part.trim());
      if (args.length === 3) {
        bundle.objects[pendingName] = {
          name: pendingName,
          frameset: args[0],
          function: args[1],
          dictionary: args[2],
        };
      }
      pendingName = null;
    }
  }

  return bundle;
};

const exportSpriteAnimationBundle = ({
  projectRoot = DEFAULT_PROJECT_ROOT,
} = {}) => {
  const standaloneDisassemblyRoot = path.join(
    projectRoot,
    ".next-electron",
    "standalone",
    "apps",
    "web",
    "public",
    "disassembly"
  );
  const spriteAnimsDir = firstExistingPath([
    path.join(disassemblyRoot, "data", "sprite_anims"),
    path.join(standaloneDisassemblyRoot, "data", "sprite_anims"),
  ]);
  const constantsPath = firstExistingPath([
    path.join(disassemblyRoot, "constants", "sprite_anim_constants.asm"),
    path.join(standaloneDisassemblyRoot, "constants", "sprite_anim_constants.asm"),
  ]);
  if (!spriteAnimsDir || !constantsPath) {
    throw new Error("Missing sprite animation sources needed to build sprite_anim_bundle.json");
  }
  const bundle = parseSpriteAnimBundle(spriteAnimsDir, constantsPath);
  fs.writeFileSync(
    path.join(outDir, "sprite_anim_bundle.json"),
    `${JSON.stringify(bundle, null, 2)}\n`
  );
};

const exportMapBlocks = () => {
  const blocksAsmPath = path.join(disassemblyRoot, "data", "maps", "blocks.asm");
  const bundle = {};
  const pendingLabels = [];
  for (const raw of readLines(blocksAsmPath)) {
    const labelMatch = raw.match(/^\s*([A-Za-z0-9_]+):/);
    if (labelMatch) {
      pendingLabels.push(labelMatch[1]);
    }
    const incbinMatch = raw.match(/INCBIN\s+"([^"]+\.blk)"/i);
    if (!incbinMatch || !pendingLabels.length) {
      continue;
    }
    const bytes = fs.readFileSync(path.join(disassemblyRoot, incbinMatch[1]));
    const encoded = bytes.toString("base64");
    for (const label of pendingLabels) {
      bundle[label] = encoded;
    }
    pendingLabels.length = 0;
  }
  fs.writeFileSync(
    path.join(outDir, "map_blocks.json"),
    `${JSON.stringify(bundle, null, 2)}\n`
  );
};

const exportTilesetRuntimeData = () => {
  const targetDir = path.join(outDir, "tilesets");
  const standaloneDisassemblyRoot = path.join(
    DEFAULT_PROJECT_ROOT,
    ".next-electron",
    "standalone",
    "apps",
    "web",
    "public",
    "disassembly"
  );
  const metatileSourceDir = firstExistingPath([
    path.join(disassemblyRoot, "data", "tilesets"),
    path.join(standaloneDisassemblyRoot, "data", "tilesets"),
  ]);
  const paletteSourceDir = firstExistingPath([
    path.join(DEFAULT_PROJECT_ROOT, "assets", "gfx", "tilesets"),
    path.join(disassemblyRoot, "gfx", "tilesets"),
    path.join(standaloneDisassemblyRoot, "gfx", "tilesets"),
  ]);
  if (!metatileSourceDir) {
    throw new Error("Missing tileset metatile/collision sources needed to build bundled tileset assets");
  }
  if (!paletteSourceDir) {
    throw new Error("Missing tileset palette map sources needed to build bundled tileset assets");
  }
  fs.mkdirSync(targetDir, { recursive: true });
  for (const entry of fs.readdirSync(metatileSourceDir, { withFileTypes: true })) {
    if (!entry.isFile()) {
      continue;
    }
    const metatileMatch = /^(.*)_metatiles\.bin$/i.exec(entry.name);
    if (metatileMatch) {
      fs.copyFileSync(
        path.join(metatileSourceDir, entry.name),
        path.join(targetDir, entry.name)
      );
      continue;
    }
    const collisionMatch = /^(.*)_collision\.asm$/i.exec(entry.name);
    if (!collisionMatch) {
      continue;
    }
    const tilesetName = collisionMatch[1];
    const collisionMap = {};
    for (const rawLine of readLines(path.join(metatileSourceDir, entry.name))) {
      if (!rawLine.toLowerCase().includes("tilecoll")) {
        continue;
      }
      const indexMatch = rawLine.match(/;\s*([0-9a-fA-F]{2})\b/);
      if (!indexMatch) {
        continue;
      }
      const tileIndex = Number.parseInt(indexMatch[1], 16);
      if (Number.isNaN(tileIndex)) {
        continue;
      }
      const codePart = rawLine.split(";")[0].trim();
      const match = codePart.match(/tilecoll\s+(.*)/i);
      if (!match) {
        continue;
      }
      const tokens = match[1]
        .split(",")
        .map((token) => token.trim())
        .filter(Boolean);
      if (tokens.length !== 4) {
        throw new Error(`Expected 4 collision tokens for ${entry.name}:${indexMatch[1]}, got ${tokens.length}.`);
      }
      collisionMap[indexMatch[1].toLowerCase()] = tokens;
    }
    fs.writeFileSync(
      path.join(targetDir, `${tilesetName}.json`),
      `${JSON.stringify(collisionMap, null, 2)}\n`
    );
  }
  for (const entry of fs.readdirSync(paletteSourceDir, { withFileTypes: true })) {
    if (!entry.isFile() || !/_palette_map\.asm$/i.test(entry.name)) {
      continue;
    }
    const tilesetName = entry.name.replace(/_palette_map\.asm$/i, "");
    const paletteMap = parseTilesetPaletteMap(
      fs.readFileSync(path.join(paletteSourceDir, entry.name), "utf8")
    );
    fs.writeFileSync(
      path.join(targetDir, `${tilesetName}_palette_map.json`),
      `${JSON.stringify(paletteMap, null, 2)}\n`
    );
  }
};

const exportCollisionStdScripts = ({
  projectRoot = DEFAULT_PROJECT_ROOT,
} = {}) => {
  const targetDir = path.join(outDir, "collision");
  const standaloneDisassemblyRoot = path.join(
    projectRoot,
    ".next-electron",
    "standalone",
    "apps",
    "web",
    "public",
    "disassembly"
  );
  const permissionsPath = firstExistingPath([
    path.join(disassemblyRoot, "data", "collision", "collision_permissions.asm"),
    path.join(standaloneDisassemblyRoot, "data", "collision", "collision_permissions.asm"),
  ]);
  const sourcePath = firstExistingPath([
    path.join(disassemblyRoot, "data", "collision", "collision_stdscripts.asm"),
    path.join(standaloneDisassemblyRoot, "data", "collision", "collision_stdscripts.asm"),
  ]);
  if (!permissionsPath || !sourcePath) {
    throw new Error("Missing collision source files needed to build bundled collision assets");
  }
  fs.mkdirSync(targetDir, { recursive: true });
  const collisionPermissions = [];
  let index = 0;
  for (const rawLine of readLines(permissionsPath)) {
    const stripped = rawLine.trim();
    if (!stripped || stripped.startsWith(";") || stripped.toLowerCase().startsWith("table_width")) {
      continue;
    }
    if (!stripped.toLowerCase().startsWith("db")) {
      continue;
    }
    const [code, comment] = rawLine.split(";", 2).map((s) => s.trim());
    const expr = code.substring(2).trim();
    if (!expr) {
      continue;
    }
    const tokens = new Set(expr.split("|").map((token) => token.trim()));
    let terrain = "land";
    if (tokens.has("WALL_TILE")) {
      terrain = "wall";
    } else if (tokens.has("WATER_TILE")) {
      terrain = "water";
    }
    collisionPermissions.push({
      value: index,
      terrain,
      talk: tokens.has("TALK"),
      raw_expr: expr,
      comment: comment || null,
    });
    index += 1;
  }
  fs.writeFileSync(
    path.join(targetDir, "collision_permissions.json"),
    `${JSON.stringify(collisionPermissions, null, 2)}\n`
  );
  const mapping = {};
  for (const rawLine of readLines(sourcePath)) {
    const line = rawLine.split(";", 1)[0].trim();
    if (!line) {
      continue;
    }
    const match = /^(?:stdcoll|std_collision)\s+([A-Za-z0-9_]+),\s*([.A-Za-z0-9_]+)/.exec(line);
    if (!match) {
      continue;
    }
    const [, constant, scriptName] = match;
    mapping[normalizeIdentifier(constant)] = scriptName.replace(/^\./, "").trim();
  }
  fs.writeFileSync(
    path.join(targetDir, "collision_stdscripts.json"),
    `${JSON.stringify(mapping, null, 2)}\n`
  );
};

const parseSpriteConstants = () => {
  const constantsPath = path.join(disassemblyRoot, "constants", "sprite_constants.asm");
  const constants = [];
  for (const rawLine of readLines(constantsPath)) {
    const trimmed = rawLine.split(";", 1)[0].trim();
    if (!trimmed) {
      continue;
    }
    if (trimmed.startsWith("DEF NUM_OVERWORLD_SPRITES")) {
      break;
    }
    if (trimmed.startsWith("const SPRITE_")) {
      const parts = trimmed.split(/\s+/);
      if (parts.length >= 2) {
        constants.push(parts[1].toUpperCase());
      }
    }
  }
  return constants;
};

const parseSpritePaletteTokens = () => {
  const spritesPath = path.join(disassemblyRoot, "data", "sprites", "sprites.asm");
  const tokens = [];
  for (const rawLine of readLines(spritesPath)) {
    const trimmed = rawLine.split(";", 1)[0].trim();
    if (!trimmed) {
      continue;
    }
    const match = trimmed.match(SPRITE_PALETTE_LINE_PATTERN);
    if (match) {
      tokens.push(match[1].toUpperCase());
    }
  }
  return tokens;
};

const exportSpritePaletteDefaults = () => {
  const spriteConstants = parseSpriteConstants().filter((value) => value !== "SPRITE_NONE");
  const paletteTokens = parseSpritePaletteTokens();
  if (spriteConstants.length !== paletteTokens.length) {
    throw new Error(
      `Sprite palette map mismatch: ${spriteConstants.length} constants, ${paletteTokens.length} palettes.`
    );
  }
  const mapping = {};
  for (let index = 0; index < paletteTokens.length; index += 1) {
    const spriteConstant = spriteConstants[index];
    const paletteToken = paletteTokens[index];
    const paletteId = PAL_OW_LABELS[paletteToken];
    if (paletteId === undefined) {
      throw new Error(`Unknown sprite palette token '${paletteToken}'.`);
    }
    mapping[spriteConstant] = paletteId;
  }
  fs.writeFileSync(
    path.join(outDir, "sprite_palette_defaults.json"),
    `${JSON.stringify(mapping, null, 2)}\n`
  );
};

const decodePhoneText = (payload) =>
  applyTextReplacements(String(payload ?? "").replace(/<LF>/g, "\n").replace(/@/g, ""));

const normalizeContactId = (contactId) => {
  const normalized = String(contactId ?? "").trim().replace(/,$/, "");
  return CONTACT_ID_ALIASES[normalized] ?? normalized;
};

const normalizeMapConstant = (token) => {
  const normalized = String(token ?? "").trim();
  if (!normalized || normalized === "0" || normalized === "N_A") {
    return null;
  }
  return normalized;
};

const normalizeScriptLabel = (token) => {
  const normalized = String(token ?? "").trim();
  if (!normalized || normalized === "0") {
    return null;
  }
  return normalized;
};

const timeTokenToMask = (token) => {
  const normalized = String(token ?? "").trim().toUpperCase();
  if (!normalized || normalized === "0" || normalized === "NONE") {
    return 0;
  }
  if (normalized === "ANYTIME") {
    return TIME_MASKS.MORN | TIME_MASKS.DAY | TIME_MASKS.NITE;
  }
  const parts = normalized.split("|").map((part) => part.trim()).filter(Boolean);
  if (!parts.length) {
    return 0;
  }
  let mask = 0;
  for (const part of parts) {
    mask |= TIME_MASKS[part] ?? 0;
  }
  if (mask === 0) {
    const parsed = Number(normalized);
    if (!Number.isNaN(parsed)) {
      mask = parsed;
    }
  }
  return mask;
};

const contactDisplayName = (contactId) => {
  if (!String(contactId).includes("_")) {
    return contactId;
  }
  const parts = String(contactId).split("_");
  const tail = parts[parts.length - 1];
  return tail || contactId;
};

const TOKEN_RE = /\s*(<<|>>|\$[0-9A-Fa-f]+|%[01]+|0x[0-9A-Fa-f]+|0b[01]+|[()+\-*/|&^%]|\d+|[A-Za-z_][A-Za-z0-9_]*)/y;

const tokenizeExpression = (expr) => {
  const tokens = [];
  const trimmed = String(expr ?? "").trim();
  let index = 0;
  while (index < trimmed.length) {
    TOKEN_RE.lastIndex = index;
    const match = TOKEN_RE.exec(trimmed);
    if (!match) {
      throw new Error(`Unsupported token in expression: ${expr}`);
    }
    const token = match[1];
    index = TOKEN_RE.lastIndex;
    if (token) {
      tokens.push(token);
    }
  }
  return tokens;
};

const resolveExpression = (expr, constants) => {
  const tokens = tokenizeExpression(expr);
  if (!tokens.length) {
    throw new Error("Expression cannot be empty.");
  }
  const normalized = [];
  for (const token of tokens) {
    if (["<<", ">>", "+", "-", "*", "/", "%", "|", "&", "^", "(", ")"].includes(token)) {
      normalized.push(token);
      continue;
    }
    if (token.startsWith("$")) {
      normalized.push(parseInt(token.slice(1), 16));
      continue;
    }
    if (token.startsWith("%")) {
      normalized.push(parseInt(token.slice(1), 2));
      continue;
    }
    if (token.toLowerCase().startsWith("0x")) {
      normalized.push(parseInt(token, 16));
      continue;
    }
    if (token.toLowerCase().startsWith("0b")) {
      normalized.push(parseInt(token, 2));
      continue;
    }
    if (/^\d+$/.test(token)) {
      normalized.push(parseInt(token, 10));
      continue;
    }
    if (Object.prototype.hasOwnProperty.call(constants, token)) {
      normalized.push(constants[token]);
      continue;
    }
    throw new Error(`Unknown constant '${token}' in expression '${expr}'.`);
  }

  let index = 0;
  const peek = () => normalized[index];
  const consume = () => normalized[index++];

  const parsePrimary = () => {
    const token = consume();
    if (token === "(") {
      const value = parseExpression();
      const closing = consume();
      if (closing !== ")") {
        throw new Error(`Expected ')' in expression '${expr}'.`);
      }
      return value;
    }
    if (typeof token === "number") {
      return token;
    }
    throw new Error(`Unexpected token '${token}' in expression '${expr}'.`);
  };

  const parseUnary = () => {
    const token = peek();
    if (token === "+" || token === "-") {
      consume();
      const value = parseUnary();
      return token === "-" ? -value : value;
    }
    return parsePrimary();
  };

  const parseMul = () => {
    let value = parseUnary();
    while (true) {
      const token = peek();
      if (token === "*" || token === "/" || token === "%") {
        consume();
        const rhs = parseUnary();
        if (token === "*") {
          value *= rhs;
        } else if (token === "/") {
          value = Math.floor(value / rhs);
        } else {
          value -= Math.floor(value / rhs) * rhs;
        }
      } else {
        break;
      }
    }
    return value;
  };

  const parseAdd = () => {
    let value = parseMul();
    while (true) {
      const token = peek();
      if (token === "+" || token === "-") {
        consume();
        const rhs = parseMul();
        value = token === "+" ? value + rhs : value - rhs;
      } else {
        break;
      }
    }
    return value;
  };

  const parseShift = () => {
    let value = parseAdd();
    while (true) {
      const token = peek();
      if (token === "<<" || token === ">>") {
        consume();
        const rhs = parseAdd();
        value = token === "<<" ? value << rhs : value >> rhs;
      } else {
        break;
      }
    }
    return value;
  };

  const parseAnd = () => {
    let value = parseShift();
    while (peek() === "&") {
      consume();
      value &= parseShift();
    }
    return value;
  };

  const parseXor = () => {
    let value = parseAnd();
    while (peek() === "^") {
      consume();
      value ^= parseAnd();
    }
    return value;
  };

  const parseExpression = () => {
    let value = parseXor();
    while (peek() === "|") {
      consume();
      value |= parseXor();
    }
    return value;
  };

  const result = parseExpression();
  if (index < normalized.length) {
    throw new Error(`Unexpected token '${normalized[index]}' in expression '${expr}'.`);
  }
  return result;
};

const parseDefConstants = (filePath, baseConstants = {}) => {
  const constants = {};
  if (!fs.existsSync(filePath)) {
    return constants;
  }
  const known = { ...baseConstants };
  let constValue = 0;
  for (const rawLine of readLines(filePath)) {
    const line = rawLine.split(";", 1)[0].trim();
    if (!line) {
      continue;
    }
    const parts = line.split(/\s+/);
    if (parts[0] === "const_def") {
      const expr = parts.slice(1).join(" ");
      try {
        constValue = expr ? resolveExpression(expr, known) : 0;
      } catch {
        constValue = 0;
      }
      known.const_value = constValue;
      continue;
    }
    if (parts[0] === "const" && parts[1]) {
      const name = parts[1];
      constants[name] = constValue;
      known[name] = constValue;
      constValue += 1;
      known.const_value = constValue;
      continue;
    }
    if (parts[0] === "const_skip") {
      const expr = parts.slice(1).join(" ");
      try {
        constValue += expr ? resolveExpression(expr, known) : 1;
      } catch {
        constValue += 1;
      }
      known.const_value = constValue;
      continue;
    }
    if (parts.length < 4 || parts[0] !== "DEF" || parts[2] !== "EQU") {
      continue;
    }
    const name = parts[1];
    const expr = parts.slice(3).join(" ");
    try {
      const value = resolveExpression(expr, known);
      constants[name] = value;
      known[name] = value;
    } catch {
      continue;
    }
  }
  return constants;
};

const parsePhoneConstants = () => {
  const filePath = path.join(disassemblyRoot, "constants", "phone_constants.asm");
  const entries = [];
  for (const raw of readLines(filePath)) {
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

const parsePhoneContacts = () => {
  const filePath = path.join(disassemblyRoot, "data", "phone", "phone_contacts.asm");
  const entries = [];
  for (const raw of readLines(filePath)) {
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
      continue;
    }
    entries.push({
      trainer_class: tokens[0] || null,
      trainer_label: tokens[1] || null,
      map_constant: normalizeMapConstant(tokens[2]),
      callee_time_mask: timeTokenToMask(tokens[3]),
      callee_script: normalizeScriptLabel(tokens[4]),
      caller_time_mask: timeTokenToMask(tokens[5]),
      caller_script: normalizeScriptLabel(tokens[6]),
    });
  }
  return entries;
};

const exportPermanentPhoneNumbers = () => {
  const filePath = path.join(disassemblyRoot, "data", "phone", "permanent_numbers.asm");
  const numbers = [];
  for (const raw of readLines(filePath)) {
    const cleaned = raw.split(";", 1)[0].trim();
    if (!cleaned.startsWith("db ")) {
      continue;
    }
    let token = cleaned.slice("db ".length).trim();
    if (!token) {
      continue;
    }
    token = token.split(",", 1)[0].trim();
    if (!token || token.startsWith("-1") || token.toUpperCase().startsWith("$FF")) {
      break;
    }
    numbers.push(normalizeContactId(token));
  }
  fs.writeFileSync(
    path.join(outDir, "permanent_phone_numbers.json"),
    `${JSON.stringify(Array.from(new Set(numbers)), null, 2)}\n`
  );
};

const exportInitializeEvents = ({
  projectRoot = DEFAULT_PROJECT_ROOT,
} = {}) => {
  const standaloneDisassemblyRoot = path.join(
    projectRoot,
    ".next-electron",
    "standalone",
    "apps",
    "web",
    "public",
    "disassembly"
  );
  const scriptPath = firstExistingPath([
    path.join(disassemblyRoot, "engine", "events", "std_scripts.asm"),
    path.join(standaloneDisassemblyRoot, "engine", "events", "std_scripts.asm"),
  ]);
  if (!scriptPath) {
    throw new Error("Missing std_scripts.asm needed to build initialize_events.json");
  }
  const lines = readLines(scriptPath);
  let startIndex = -1;
  for (let index = 0; index < lines.length; index += 1) {
    if (lines[index].trim().startsWith("InitializeEventsScript:")) {
      startIndex = index + 1;
      break;
    }
  }
  if (startIndex === -1) {
    throw new Error("Failed to locate InitializeEventsScript in std_scripts.asm");
  }

  const eventFlags = [];
  const engineFlags = [];
  const variableSprites = {};
  for (const raw of lines.slice(startIndex)) {
    const line = raw.split(";", 1)[0].trim();
    if (!line) {
      continue;
    }
    if (line.startsWith("endcallback")) {
      break;
    }
    if (line.startsWith("setevent")) {
      const parts = line.split(/\s+/);
      if (parts.length > 1) {
        eventFlags.push(parts[1]);
      }
      continue;
    }
    if (line.startsWith("setflag")) {
      const parts = line.split(/\s+/);
      if (parts.length > 1) {
        engineFlags.push(parts[1]);
      }
      continue;
    }
    if (line.startsWith("variablesprite")) {
      const parts = line.split(/\s+/).filter(Boolean);
      if (parts.length >= 3) {
        variableSprites[parts[1].replace(/,$/, "")] = parts[2].replace(/,$/, "");
      }
    }
  }
  const payload = {
    eventFlags: Array.from(new Set(eventFlags)),
    engineFlags: Array.from(new Set(engineFlags)),
    variableSprites,
  };
  fs.writeFileSync(
    path.join(outDir, "initialize_events.json"),
    `${JSON.stringify(payload, null, 2)}\n`
  );
};

const exportStoryEventScriptConstants = () => {
  const global = parseDefConstants(
    path.join(disassemblyRoot, "constants", "misc_constants.asm"),
    {}
  );
  Object.assign(
    global,
    parseDefConstants(path.join(disassemblyRoot, "constants", "pokemon_constants.asm"), global),
    parseDefConstants(path.join(disassemblyRoot, "constants", "battle_constants.asm"), global),
    parseDefConstants(path.join(disassemblyRoot, "constants", "ram_constants.asm"), global)
  );
  const maps = {};
  const mapsDir = path.join(disassemblyRoot, "maps");
  for (const entry of fs.readdirSync(mapsDir, { withFileTypes: true })) {
    if (!entry.isFile() || !entry.name.endsWith(".asm")) {
      continue;
    }
    const mapName = entry.name.slice(0, -4);
    const constants = parseDefConstants(path.join(mapsDir, entry.name), global);
    if (Object.keys(constants).length) {
      maps[mapName] = constants;
    }
  }
  fs.writeFileSync(
    path.join(outDir, "story_event_script_constants.json"),
    `${JSON.stringify({ global, maps }, null, 2)}\n`
  );
};

const parseNonTrainerNames = () => {
  const filePath = path.join(disassemblyRoot, "data", "phone", "non_trainer_names.asm");
  const entries = {};
  for (const raw of readLines(filePath)) {
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

const parseTrainerClassNames = () => {
  const constantsPath = path.join(disassemblyRoot, "constants", "trainer_constants.asm");
  const namesPath = path.join(disassemblyRoot, "data", "trainers", "class_names.asm");
  let classIds = [];
  for (const raw of readLines(constantsPath)) {
    const line = raw.trim();
    if (line.startsWith("trainerclass ")) {
      classIds.push(line.split(/\s+/)[1]);
    }
  }
  if (classIds[0] === "TRAINER_NONE") {
    classIds = classIds.slice(1);
  }
  const classNames = [];
  for (const raw of readLines(namesPath)) {
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
  const mapping = {};
  for (let index = 0; index < Math.min(classIds.length, classNames.length); index += 1) {
    mapping[classIds[index]] = classNames[index];
  }
  return mapping;
};

const exportPhoneContacts = () => {
  const phoneConstants = parsePhoneConstants();
  const phoneEntries = parsePhoneContacts();
  const nonTrainerLines = parseNonTrainerNames();
  const classNames = parseTrainerClassNames();
  if (phoneConstants.length !== phoneEntries.length) {
    throw new Error(
      `Phone constant count ${phoneConstants.length} does not match contact table ${phoneEntries.length}`
    );
  }
  const records = {};
  for (let index = 0; index < phoneConstants.length; index += 1) {
    const contactId = phoneConstants[index];
    const entry = phoneEntries[index];
    if (!contactId || contactId === "PHONE_00") {
      continue;
    }
    const trainerClass = entry.trainer_class;
    const trainerLabel = entry.trainer_label;
    let lines = null;
    if (trainerClass === "TRAINER_NONE") {
      const nonTrainer = trainerLabel ? nonTrainerLines[trainerLabel] : undefined;
      if (!nonTrainer) {
        continue;
      }
      lines = nonTrainer;
    } else {
      const classLabel = trainerClass ? classNames[trainerClass] ?? trainerClass : "";
      const name = contactDisplayName(contactId);
      lines = [`${name}:`, `   ${classLabel}`.trimEnd()];
    }
    const primary = String(lines[0] ?? "").replace(/:$/, "").trim() || String(contactId).replace(/_/g, " ");
    records[normalizeContactId(contactId)] = {
      contactId: normalizeContactId(contactId),
      trainerClass: trainerClass ?? null,
      trainerLabel: trainerLabel ?? null,
      lines,
      primaryLabel: primary,
      mapConstant: entry.map_constant,
      calleeTimeMask: entry.callee_time_mask ?? 0,
      calleeScript: entry.callee_script,
      callerTimeMask: entry.caller_time_mask ?? 0,
      callerScript: entry.caller_script,
    };
  }
  fs.writeFileSync(
    path.join(outDir, "phone_contacts.json"),
    `${JSON.stringify(records, null, 2)}\n`
  );
};

const exportAsmText = () => {
  const payload = {};
  for (const relativeDir of TEXT_DIRS) {
    const absoluteDir = path.join(disassemblyRoot, relativeDir);
    if (!fs.existsSync(absoluteDir)) {
      continue;
    }
    const files = fs.readdirSync(absoluteDir)
      .filter((file) => file.endsWith(".asm"))
      .sort();
    for (const file of files) {
      Object.assign(payload, parseTextAsmFile(path.join(absoluteDir, file)));
    }
  }
  if (!Object.keys(payload).length) {
    throw new Error(`Missing text sources needed to build ${TEXT_JSON_FILENAME}`);
  }
  fs.writeFileSync(
    path.join(outDir, TEXT_JSON_FILENAME),
    `${JSON.stringify(payload, null, 2)}\n`
  );
};

const exportMoveNames = () => {
  const filePath = path.join(disassemblyRoot, "data", "moves", "names.asm");
  if (!fs.existsSync(filePath)) {
    throw new Error(`Missing move names source needed to build ${MOVE_NAMES_JSON_FILENAME}`);
  }
  const names = parseMoveNamesAsmFile(filePath);
  if (!names.length) {
    throw new Error(`Missing move names source needed to build ${MOVE_NAMES_JSON_FILENAME}`);
  }
  fs.writeFileSync(
    path.join(outDir, MOVE_NAMES_JSON_FILENAME),
    `${JSON.stringify(names, null, 2)}\n`
  );
};

const exportRuntimeAssets = ({
  projectRoot = DEFAULT_PROJECT_ROOT,
  repoRoot = path.resolve(projectRoot, "..", ".."),
  disassemblyRoot: disassemblyOverride = path.join(repoRoot, "vendor", "pokecrystal"),
  outDir: outDirOverride = path.join(projectRoot, "assets", "data"),
  strict = process.env.CRYSTAL_CANONICAL_EXPORT === "1",
} = {}) => {
  disassemblyRoot = disassemblyOverride;
  outDir = outDirOverride;
  fs.mkdirSync(outDir, { recursive: true });
  if (!fs.existsSync(disassemblyRoot) || !hasCompleteRuntimeDisassembly(disassemblyRoot)) {
    if (strict) {
      throw new Error(
        `Canonical runtime export requires a complete ASM checkout at ${disassemblyRoot}`
      );
    }
    if (
      !fs.existsSync(path.join(outDir, "collision", "collision_permissions.json")) ||
      !fs.existsSync(path.join(outDir, "collision", "collision_stdscripts.json"))
    ) {
      try {
        exportCollisionStdScripts({ projectRoot });
      } catch {
        // In asset-only mode, require the committed collision dataset if source files are unavailable.
      }
    }
    if (!fs.existsSync(path.join(outDir, "battle_anim_bundle.json"))) {
      try {
        exportBattleAnimationBundle({ projectRoot, includeOutDirSources: false });
      } catch {
        // In asset-only mode, require the committed bundle rather than raw ASM sidecar files.
      }
    }
    if (!fs.existsSync(path.join(outDir, "sprite_anim_bundle.json"))) {
      try {
        exportSpriteAnimationBundle({ projectRoot });
      } catch {
        // In asset-only mode, require the committed sprite animation bundle.
      }
    }
    if (!fs.existsSync(path.join(outDir, "initialize_events.json"))) {
      try {
        exportInitializeEvents({ projectRoot });
      } catch {
        // In asset-only mode, require the committed initialize events dataset if source files are unavailable.
      }
    }
    if (!fs.existsSync(path.join(outDir, TEXT_JSON_FILENAME))) {
      try {
        exportAsmText();
      } catch {
        // In asset-only mode, require the committed asm_text bundle if source files are unavailable.
      }
    }
    if (!fs.existsSync(path.join(outDir, MOVE_NAMES_JSON_FILENAME))) {
      try {
        exportMoveNames();
      } catch {
        // In asset-only mode, require the committed move names bundle if source files are unavailable.
      }
    }
    if (
      !fs.existsSync(path.join(outDir, "tilesets", "johto.json")) ||
      !fs.existsSync(path.join(outDir, "tilesets", "johto_palette_map.json")) ||
      !fs.existsSync(path.join(outDir, "tilesets", "players_room.json")) ||
      !fs.existsSync(path.join(outDir, "tilesets", "players_room_palette_map.json"))
    ) {
      try {
        exportTilesetRuntimeData();
      } catch {
        // In asset-only mode, require the committed tileset runtime dataset if sources are unavailable.
      }
    }
    assertGeneratedTargetsExist();
    console.log("[runtime-assets] using committed runtime assets from", outDir);
    return;
  }
  resetGeneratedTargets();
  exportRuntimeMapMetadata();
  exportRuntimeSpawnPoints();
  exportBattleAnimationTable();
  exportBattleAnimationRuntimeSources();
  exportBattleAnimationGfxSourceTable({ projectRoot });
  exportBattleAnimationBundle({ projectRoot });
  exportSpriteAnimationBundle({ projectRoot });
  exportMapBlocks();
  exportPermanentPhoneNumbers();
  exportInitializeEvents({ projectRoot });
  exportStoryEventScriptConstants();
  exportPhoneContacts();
  exportAsmText();
  exportMoveNames();
  exportTilesetRuntimeData();
  exportCollisionStdScripts({ projectRoot });
  exportSpritePaletteDefaults();
  console.log("[runtime-assets] exported runtime assets to", outDir);
};

if (require.main === module) {
  exportRuntimeAssets();
}

module.exports = { exportRuntimeAssets };

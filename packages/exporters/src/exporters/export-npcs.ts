import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import type { ObjectEvent } from "@pokecrystal/core/core/models/map";
import { parseAsmNumber, splitAsmArgs, stripAsmComment, writeJsonToTargets } from "./asm-utils";

export type ExportedNpcEvent = Omit<ObjectEvent, "label"> & {
  label: string | null;
};
export type NpcData = Record<string, ExportedNpcEvent[]>;

const TIME_OF_DAY_VALUES: Record<string, number> = {
  MORN: 1,
  DAY: 2,
  NITE: 4,
  DARKNESS: 8,
};

export function parseSpriteFacings(
  constantsFilePath: string,
  spritesFilePath: string
): Record<string, boolean> {
  if (!fs.existsSync(constantsFilePath)) {
    throw new Error(`Missing sprite constants file ${constantsFilePath}.`);
  }
  if (!fs.existsSync(spritesFilePath)) {
    throw new Error(`Missing overworld sprite table ${spritesFilePath}.`);
  }
  const constants: string[] = [];
  let inOverworldSprites = false;
  for (const rawLine of fs.readFileSync(constantsFilePath, "utf8").split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (line === "const SPRITE_NONE") {
      inOverworldSprites = true;
      continue;
    }
    if (inOverworldSprites && line.startsWith("const SPRITE_")) {
      constants.push(line.split(/\s+/)[1]);
    }
  }
  const spriteTypes = fs
    .readFileSync(spritesFilePath, "utf8")
    .split(/\r?\n/)
    .map(stripAsmComment)
    .filter((line) => line.startsWith("overworld_sprite "))
    .map((line) => {
      const args = splitAsmArgs(line.slice("overworld_sprite".length));
      if (args.length !== 4) {
        throw new Error(`overworld_sprite requires 4 args, found ${args.length}: ${args.join(", ")}`);
      }
      return args[2];
    });
  if (constants.length < spriteTypes.length) {
    throw new Error(
      `Overworld sprite constant/table length mismatch: ${constants.length} < ${spriteTypes.length}`
    );
  }
  return Object.fromEntries(
    constants.map((sprite, index) => [
      sprite,
      index >= spriteTypes.length || spriteTypes[index] !== "STILL_SPRITE",
    ])
  );
}

const parseConstDefValue = (value: string | undefined): number => {
  const trimmed = String(value ?? "").trim();
  if (!trimmed) {
    return 0;
  }
  const shiftMatch = trimmed.match(/^(.+?)\s*<<\s*(.+)$/);
  if (shiftMatch) {
    return parseAsmNumber(shiftMatch[1]) << parseAsmNumber(shiftMatch[2]);
  }
  return parseAsmNumber(trimmed);
};

const isAsmNumberToken = (value: string): boolean => /^[+-]?(?:\d+|\$[0-9A-Fa-f]+|%[01]+)$/.test(value.trim());

const parseRequiredAsmNumber = (token: string, expression: string): number => {
  if (!isAsmNumberToken(token)) {
    throw new Error(`Unknown numeric expression token '${token}' in '${expression}'.`);
  }
  const parsed = parseAsmNumber(token);
  if (!Number.isFinite(parsed)) {
    throw new Error(`Invalid numeric expression token '${token}' in '${expression}'.`);
  }
  return parsed;
};

export function parseNumericExpression(
  value: string,
  constants: Record<string, number> = {}
): number {
  const trimmed = value.trim();
  if (!trimmed) {
    return 0;
  }
  return trimmed
    .split("|")
    .map((part) => part.trim())
    .filter(Boolean)
    .reduce((total, token) => {
      const mapped = constants[token] ?? TIME_OF_DAY_VALUES[token];
      return total | (mapped ?? parseRequiredAsmNumber(token, trimmed));
    }, 0);
}

export function parseNpcConstants(constantsFilePath: string): Record<string, number> {
  if (!fs.existsSync(constantsFilePath)) {
    throw new Error(`Missing NPC constants file ${constantsFilePath}.`);
  }

  const values: Record<string, number> = {};
  let nextValue = 0;
  for (const rawLine of fs.readFileSync(constantsFilePath, "utf8").split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (!line) {
      continue;
    }
    const parts = line.split(/\s+/);
    if (parts[0] === "const_def") {
      nextValue = parseConstDefValue(line.slice("const_def".length));
      continue;
    }
    if (parts[0] === "const" && parts[1]) {
      values[parts[1]] = nextValue;
      nextValue += 1;
    }
  }
  return values;
}

export function parseObjectConstantLabels(content: string): string[] {
  const labels: string[] = [];
  let inObjectConstants = false;

  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (!line) {
      continue;
    }
    if (line === "object_const_def") {
      inObjectConstants = true;
      continue;
    }
    if (!inObjectConstants) {
      continue;
    }
    const parts = line.split(/\s+/);
    if (parts[0] !== "const" || !parts[1]) {
      break;
    }
    labels.push(parts[1]);
  }

  return labels;
}

const parseObjectEvent = (
  args: string[],
  objectIdentifier: string | null,
  constants: Record<string, number>,
  trainerEventFlags: Record<string, string>,
  spriteFacings: Record<string, boolean>
): ExportedNpcEvent => {
  if (args.length !== 13) {
    throw new Error(`object_event requires 13 args, found ${args.length}: ${args.join(", ")}`);
  }
  const objectType = args[9];
  const sprite = args[2];
  const spriteHasFacings = spriteFacings[sprite];
  if (spriteHasFacings === undefined) {
    throw new Error(`Unknown overworld sprite '${sprite}' in object_event.`);
  }
  const script = args[11];
  const eventFlag =
    objectType === "OBJECTTYPE_TRAINER" && trainerEventFlags[script]
      ? trainerEventFlags[script]
      : args[12];
  return {
    x: parseNumericExpression(args[0], constants),
    y: parseNumericExpression(args[1], constants),
    sprite,
    sprite_has_facings: spriteHasFacings,
    spritemovedata: args[3],
    move_range_x: parseNumericExpression(args[4], constants),
    move_range_y: parseNumericExpression(args[5], constants),
    hram_x: parseNumericExpression(args[6], constants),
    hram_y: parseNumericExpression(args[7], constants),
    pal: parseNumericExpression(args[8], constants),
    object_type: objectType,
    radius: parseNumericExpression(args[10], constants),
    script,
    label: null,
    event_flag: eventFlag,
    object_identifier: objectIdentifier,
    sightline_direction_override: null,
  };
};

function parseTrainerEventFlags(content: string): Record<string, string> {
  const flags: Record<string, string> = {};
  let currentLabel: string | null = null;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    const label = line.match(/^([A-Za-z_][A-Za-z0-9_]*):$/);
    if (label) {
      currentLabel = label[1];
      continue;
    }
    if (!currentLabel || !line.startsWith("trainer")) {
      continue;
    }
    const args = splitAsmArgs(line.slice("trainer".length));
    if (args.length < 3) {
      throw new Error(`trainer command requires at least 3 args for ${currentLabel}: ${args.join(", ")}`);
    }
    flags[currentLabel] = args[2];
  }
  return flags;
}

export function parseNpcDataFromMapFile(
  mapName: string,
  filePath: string,
  constants: Record<string, number>,
  spriteFacings: Record<string, boolean>
): ExportedNpcEvent[] {
  const content = fs.readFileSync(filePath, "utf8");
  const labels = parseObjectConstantLabels(content);
  const trainerEventFlags = parseTrainerEventFlags(content);
  const events: ExportedNpcEvent[] = [];

  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (!line.startsWith("object_event")) {
      continue;
    }
    const args = splitAsmArgs(line.slice("object_event".length));
    events.push(
      parseObjectEvent(
        args,
        labels[events.length] ?? null,
        constants,
        trainerEventFlags,
        spriteFacings
      )
    );
  }

  if (labels.length > 0 && labels.length !== events.length) {
    throw new Error(
      `Object constant count does not match object_event count for ${mapName}: ${labels.length} != ${events.length}`
    );
  }

  return events;
}

export function parseNpcData(
  mapsDir: string,
  constants: Record<string, number>,
  spriteFacings: Record<string, boolean>
): NpcData {
  const data: NpcData = {};
  for (const entry of fs.readdirSync(mapsDir).filter((name) => name.endsWith(".asm")).sort()) {
    const mapName = path.basename(entry, ".asm");
    const events = parseNpcDataFromMapFile(
      mapName,
      path.join(mapsDir, entry),
      constants,
      spriteFacings
    );
    data[mapName] = events;
  }
  return data;
}

export function exportNpcData(): NpcData {
  const root = getDisassemblyRoot();
  const constants = parseNpcConstants(path.join(root, "constants", "sprite_data_constants.asm"));
  const spriteFacings = parseSpriteFacings(
    path.join(root, "constants", "sprite_constants.asm"),
    path.join(root, "data", "sprites", "sprites.asm")
  );
  const npcData = parseNpcData(path.join(root, "maps"), constants, spriteFacings);
  writeJsonToTargets("npcs.json", npcData, { indent: 2 });
  return npcData;
}

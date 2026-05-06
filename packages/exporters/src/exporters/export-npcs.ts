import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import type { ObjectEvent } from "@pokecrystal/core/core/models/map";
import { parseAsmNumber, splitAsmArgs, stripAsmComment, writeJsonToTargets } from "./asm-utils";

export type ExportedNpcEvent = Omit<ObjectEvent, "sightline_direction_override"> & {
  sightline_direction_override?: string | null;
};
export type NpcData = Record<string, ExportedNpcEvent[]>;

const TIME_OF_DAY_VALUES: Record<string, number> = {
  MORN: 1,
  DAY: 2,
  NITE: 4,
  DARKNESS: 8,
};

const DEFAULT_CONSTANT_VALUES: Record<string, number> = {
  PAL_NPC_RED: 8,
  PAL_NPC_BLUE: 9,
  PAL_NPC_GREEN: 10,
  PAL_NPC_BROWN: 11,
  PAL_NPC_PINK: 12,
  PAL_NPC_EMOTE: 13,
  PAL_NPC_TREE: 14,
  PAL_NPC_ROCK: 15,
};

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
      return total | (mapped ?? parseAsmNumber(token));
    }, 0);
}

export function parseNpcConstants(constantsFilePath: string): Record<string, number> {
  if (!fs.existsSync(constantsFilePath)) {
    return { ...DEFAULT_CONSTANT_VALUES };
  }

  const values: Record<string, number> = { ...DEFAULT_CONSTANT_VALUES };
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
  constants: Record<string, number>
): ExportedNpcEvent => {
  if (args.length !== 13) {
    throw new Error(`object_event requires 13 args, found ${args.length}: ${args.join(", ")}`);
  }
  return {
    x: parseNumericExpression(args[0], constants),
    y: parseNumericExpression(args[1], constants),
    sprite: args[2],
    spritemovedata: args[3],
    move_range_x: parseNumericExpression(args[4], constants),
    move_range_y: parseNumericExpression(args[5], constants),
    hram_x: parseNumericExpression(args[6], constants),
    hram_y: parseNumericExpression(args[7], constants),
    pal: parseNumericExpression(args[8], constants),
    object_type: args[9],
    radius: parseNumericExpression(args[10], constants),
    script: args[11],
    event_flag: args[12],
    object_identifier: objectIdentifier,
  };
};

export function parseNpcDataFromMapFile(
  mapName: string,
  filePath: string,
  constants: Record<string, number>
): ExportedNpcEvent[] {
  const content = fs.readFileSync(filePath, "utf8");
  const labels = parseObjectConstantLabels(content);
  const events: ExportedNpcEvent[] = [];

  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (!line.startsWith("object_event")) {
      continue;
    }
    const args = splitAsmArgs(line.slice("object_event".length));
    events.push(parseObjectEvent(args, labels[events.length] ?? null, constants));
  }

  if (labels.length > 0 && labels.length !== events.length) {
    throw new Error(
      `Object constant count does not match object_event count for ${mapName}: ${labels.length} != ${events.length}`
    );
  }

  return events;
}

export function parseNpcData(mapsDir: string, constants: Record<string, number>): NpcData {
  const data: NpcData = {};
  for (const entry of fs.readdirSync(mapsDir).filter((name) => name.endsWith(".asm")).sort()) {
    const mapName = path.basename(entry, ".asm");
    const events = parseNpcDataFromMapFile(mapName, path.join(mapsDir, entry), constants);
    if (events.length > 0) {
      data[mapName] = events;
    }
  }
  return data;
}

export function exportNpcData(): NpcData {
  const root = getDisassemblyRoot();
  const constants = parseNpcConstants(path.join(root, "constants", "sprite_data_constants.asm"));
  const npcData = parseNpcData(path.join(root, "maps"), constants);
  writeJsonToTargets("npcs.json", npcData, { indent: 2 });
  return npcData;
}

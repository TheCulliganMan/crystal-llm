import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { parseMapDefinitions } from "./export-map-attributes";
import { parseAsmNumber, splitAsmArgs, stripAsmComment, writeJsonToTargets } from "./asm-utils";

export interface LandmarkEntry {
  id: number;
  constant: string;
  label: string;
  name: string;
  x: number;
  y: number;
  region: string;
}

interface LandmarkConstant {
  id: number;
  constant: string;
}

const decodeLandmarkName = (value: string): string => {
  return value
    .replace(/^"/, "")
    .replace(/@.*$/, "")
    .replace(/<BSP>/g, "\n")
    .replace(/#/g, "Poke")
    .split("\n")
    .map((line) => line.replace(/[ \t]+/g, " ").trim())
    .join(" ")
    .trim();
};

const parseLandmarkConstants = (
  constantsPath: string
): {
  constants: LandmarkConstant[];
  kantoStart: number;
  otherStart: number;
} => {
  const constants: LandmarkConstant[] = [];
  let currentValue = 0;
  let kantoStart = -1;
  let otherStart = -1;

  for (const rawLine of fs.readFileSync(constantsPath, "utf8").split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (!line) {
      continue;
    }
    const johtoMatch = /^DEF\s+KANTO_LANDMARK\s+EQU\s+const_value\b/.exec(line);
    if (johtoMatch) {
      kantoStart = currentValue;
      continue;
    }
    const otherMatch = /^DEF\s+OTHER_LANDMARK\s+EQU\s+const_value\b/.exec(line);
    if (otherMatch) {
      otherStart = currentValue;
      continue;
    }
    if (/^DEF\s+NUM_LANDMARKS\s+EQU\s+const_value\b/.test(line)) {
      break;
    }
    const constMatch = /^const\s+(LANDMARK_[A-Z0-9_]+)/.exec(line);
    if (!constMatch) {
      continue;
    }
    constants.push({ id: currentValue, constant: constMatch[1] });
    currentValue += 1;
  }

  if (!constants.length || kantoStart < 0 || otherStart < 0) {
    throw new Error(`Could not parse landmark constants from ${constantsPath}`);
  }

  return { constants, kantoStart, otherStart };
};

const parseLandmarkNames = (landmarksPath: string): Record<string, string> => {
  const names: Record<string, string> = {};
  for (const rawLine of fs.readFileSync(landmarksPath, "utf8").split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    const match = /^([A-Za-z0-9_]+):\s+db\s+(.+)$/.exec(line);
    if (!match) {
      continue;
    }
    names[match[1]] = decodeLandmarkName(match[2]);
  }
  return names;
};

const parseLandmarkRows = (
  landmarksPath: string
): Array<{ x: number; y: number; nameLabel: string }> => {
  const rows: Array<{ x: number; y: number; nameLabel: string }> = [];
  for (const rawLine of fs.readFileSync(landmarksPath, "utf8").split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    const match = /^landmark\s+(.+)$/.exec(line);
    if (!match) {
      continue;
    }
    const args = splitAsmArgs(match[1]);
    if (args.length < 3) {
      throw new Error(`Invalid landmark row in ${landmarksPath}: ${line}`);
    }
    rows.push({
      x: parseAsmNumber(args[0]) + 8,
      y: parseAsmNumber(args[1]) + 16,
      nameLabel: args[2],
    });
  }
  return rows;
};

const getRegion = (id: number, kantoStart: number, otherStart: number): string => {
  if (id >= kantoStart && id < otherStart) {
    return "KANTO";
  }
  return "JOHTO";
};

export type PokegearLandmarksPayload = {
  landmarks: LandmarkEntry[];
  map_to_landmark: Record<string, string>;
};

export function exportPokegearLandmarks(): PokegearLandmarksPayload {
  const disassemblyRoot = getDisassemblyRoot();
  const constantsPath = path.join(disassemblyRoot, "constants", "landmark_constants.asm");
  const landmarksPath = path.join(disassemblyRoot, "data", "maps", "landmarks.asm");
  const { constants, kantoStart, otherStart } = parseLandmarkConstants(constantsPath);
  const namesByLabel = parseLandmarkNames(landmarksPath);
  const rows = parseLandmarkRows(landmarksPath);

  if (rows.length !== constants.length) {
    throw new Error(
      `Landmark table length mismatch: ${rows.length} rows in ${landmarksPath}, ${constants.length} constants in ${constantsPath}`
    );
  }

  const landmarks = constants.map((entry, index) => {
    const row = rows[index];
    const label = entry.constant.replace(/^LANDMARK_/, "");
    const name = namesByLabel[row.nameLabel];
    if (name === undefined) {
      throw new Error(`Missing landmark name label '${row.nameLabel}' for ${entry.constant}.`);
    }
    return {
      id: entry.id,
      constant: entry.constant,
      label,
      name,
      x: row.x,
      y: row.y,
      region: getRegion(entry.id, kantoStart, otherStart),
    };
  });

  const map_to_landmark = Object.fromEntries(
    Object.entries(parseMapDefinitions())
      .filter(([, definition]) => typeof definition.location === "string")
      .map(([mapName, definition]) => [mapName, String(definition.location)])
  );

  writeJsonToTargets("pokegear_landmarks.json", { landmarks, map_to_landmark });
  return { landmarks, map_to_landmark };
}

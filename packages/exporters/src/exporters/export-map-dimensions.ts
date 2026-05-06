import fs from "fs";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripAsmComment, writeJsonToTargets } from "./asm-utils";

const MAP_CONST_RE = /^map_const\s+([A-Z0-9_]+)\s*,\s*(\d+)\s*,\s*(\d+)\s*$/;

export function parseMapDimensions(constantsPath: string): Record<string, { width: number; height: number }> {
  const mapDimensions: Record<string, { width: number; height: number }> = {};
  const lines = fs.readFileSync(constantsPath, "utf8").split(/\r?\n/);

  for (const rawLine of lines) {
    const line = stripAsmComment(rawLine);
    if (!line || !line.startsWith("map_const")) {
      continue;
    }
    const match = line.match(MAP_CONST_RE);
    if (!match) {
      throw new Error(`Malformed map_const line: ${rawLine.trimEnd()}`);
    }
    mapDimensions[match[1]] = {
      width: Number.parseInt(match[2], 10),
      height: Number.parseInt(match[3], 10),
    };
  }

  return mapDimensions;
}

export function exportMapDimensions(): Record<string, { width: number; height: number }> {
  const constantsPath = `${getDisassemblyRoot()}/constants/map_constants.asm`;
  const dimensions = parseMapDimensions(constantsPath);
  writeJsonToTargets("map_dimensions.json", dimensions, { indent: 4 });
  return dimensions;
}


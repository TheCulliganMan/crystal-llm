import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import type { MapAttributes, MapConnection } from "@pokecrystal/core/core/models/map";
import { parseAsmNumber, stripAsmComment, writeJsonToTargets } from "./asm-utils";
import { parseMapDimensions } from "./export-map-dimensions";

const PALETTE_CONSTANTS: Record<string, number> = {
  PALETTE_AUTO: 0,
  PALETTE_DAY: 1,
  PALETTE_NITE: 2,
  PALETTE_MORN: 3,
  PALETTE_DARK: 4,
};

const TIME_OF_DAY_BY_PALETTE: Record<string, string | null> = {
  PALETTE_AUTO: null,
  PALETTE_DAY: "day",
  PALETTE_NITE: "nite",
  PALETTE_MORN: "morn",
  PALETTE_DARK: "dark",
};

function requireMappingValue<T>(mapping: Record<string, T | undefined>, key: string, kind: string): T {
  const value = mapping[key];
  if (value === undefined || value === null) {
    throw new Error(`Missing ${kind} for '${key}'.`);
  }
  return value;
}

function requireExistingKey<T>(mapping: Record<string, T>, key: string, kind: string): T {
  if (!Object.prototype.hasOwnProperty.call(mapping, key)) {
    throw new Error(`Missing ${kind} for '${key}'.`);
  }
  return mapping[key];
}

function requireStringField(mapping: Record<string, string | boolean>, key: string, mapName: string): string {
  const value = mapping[key];
  if (typeof value !== "string" || !value) {
    throw new Error(`Missing ${key} for map '${mapName}'.`);
  }
  return value;
}

export function resolveMapMusicAssetId(token: string, mapName: string): string {
  const value = token.trim();
  if (!value) {
    throw new Error(`Missing music for map '${mapName}'.`);
  }
  if (value === "MUSIC_MAHOGANY_MART") {
    return "MUSIC_CHERRYGROVE_CITY";
  }
  if (value === "RADIO_TOWER_MUSIC | MUSIC_GOLDENROD_CITY") {
    return "MUSIC_GOLDENROD_CITY";
  }
  if (/^MUSIC_[A-Z0-9_]+$/.test(value)) {
    return value;
  }
  throw new Error(`Unsupported map music token '${token}' for map '${mapName}'.`);
}

export function parseMapPhoneFlag(token: string): number {
  const value = token.trim();
  if (value === "TRUE") return 1;
  if (value === "FALSE") return 0;
  if (/^[+-]?\d+$/.test(value)) {
    return Number.parseInt(value, 10);
  }
  throw new Error(`Unknown map phone flag token '${token}'.`);
}

export function mapNameToConstant(name: string): string {
  const tokenize = (segment: string): string[] => {
    const tokens = segment.match(/B?\d+F(?=[A-Z]|$)|[A-Z]+(?=[A-Z][a-z]|\d|$)|[A-Z]?[a-z]+|\d+/g);
    return tokens ?? [segment];
  };

  const parts: string[] = [];
  for (const segment of name.split("_")) {
    if (!segment) continue;
    parts.push(...tokenize(segment));
  }
  return parts.map((part) => part.toUpperCase()).join("_");
}

export function parseMapDefinitions(): Record<string, Record<string, string | boolean>> {
  const mapsPath = path.join(getDisassemblyRoot(), "data", "maps", "maps.asm");
  const definitions: Record<string, Record<string, string | boolean>> = {};
  for (const rawLine of fs.readFileSync(mapsPath, "utf8").split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (!line || !line.startsWith("map ")) continue;
    const parts = line.split(",").map((part) => part.trim());
    const nameTokens = parts[0].split(/\s+/);
    const mapName = nameTokens[1];
    if (!mapName || parts.length !== 8 || parts.some((part) => !part)) {
      throw new Error(`Malformed map definition in ${mapsPath}: ${rawLine.trimEnd()}`);
    }
    definitions[mapName] = {
      tileset_constant: parts[1],
      environment: parts[2],
      location: parts[3],
      music: parts[4],
      phone_flag: parseMapPhoneFlag(parts[5]) === 1,
      palette: parts[6],
      fishing_group: parts[7],
    };
  }
  return definitions;
}

const parseMapConstantDimension = (token: string | undefined, mapConstant: string, dimension: "width" | "height"): number => {
  const value = token?.trim();
  if (!value) {
    throw new Error(`Missing ${dimension} for map_const '${mapConstant}'.`);
  }
  if (!/^[+-]?\d+$/.test(value)) {
    throw new Error(`Invalid ${dimension} '${value}' for map_const '${mapConstant}'.`);
  }
  return Number.parseInt(value, 10);
};

export function parseMapConstants(): Record<string, { group_constant: string; width: number; height: number }> {
  const constantsPath = path.join(getDisassemblyRoot(), "constants", "map_constants.asm");
  const metadata: Record<string, { group_constant: string; width: number; height: number }> = {};
  let currentGroup: string | null = null;

  for (const rawLine of fs.readFileSync(constantsPath, "utf8").split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (!line) continue;
    if (line.startsWith("newgroup ")) {
      currentGroup = line.split(/\s+/)[1] ?? null;
      continue;
    }
    if (!line.startsWith("map_const ")) continue;
    const segments = line.split(",").map((segment) => segment.trim());
    const mapConstant = segments[0].split(/\s+/)[1];
    if (!mapConstant) {
      throw new Error(`Malformed map_const row in ${constantsPath}: ${line}`);
    }
    if (!currentGroup) {
      throw new Error(`map_const '${mapConstant}' appears before a newgroup declaration.`);
    }
    metadata[mapConstant] = {
      group_constant: currentGroup,
      width: parseMapConstantDimension(segments[1], mapConstant, "width"),
      height: parseMapConstantDimension(segments[2], mapConstant, "height"),
    };
  }
  return metadata;
}

function loadMapTilesetMapping(definitions: Record<string, Record<string, string | boolean>>): Record<string, string> {
  const mapping: Record<string, string> = {};
  for (const [mapName, data] of Object.entries(definitions)) {
    const tilesetConstant = requireMappingValue(
      data as Record<string, string | undefined>,
      "tileset_constant",
      `tileset constant for map '${mapName}'`
    );
    mapping[mapName] = tilesetConstant.replace("TILESET_", "").toLowerCase();
  }
  return mapping;
}

function loadMapPaletteValues(): [Record<string, number>, Record<string, string | null>] {
  const mapsPath = path.join(getDisassemblyRoot(), "data", "maps", "maps.asm");
  const paletteData: Record<string, number> = {};
  const timeOfDayData: Record<string, string | null> = {};

  for (const rawLine of fs.readFileSync(mapsPath, "utf8").split(/\r?\n/)) {
    const cleaned = stripAsmComment(rawLine);
    if (!cleaned || !cleaned.startsWith("map")) continue;
    const parts = cleaned.split(",").map((part) => part.trim());
    const tokens = parts[0].split(/\s+/);
    const mapName = tokens[1];
    if (!mapName || parts.length !== 8 || parts.some((part) => !part)) {
      throw new Error(`Malformed map definition in ${mapsPath}: ${rawLine.trimEnd()}`);
    }
    const phoneFlag = parseMapPhoneFlag(parts[5]);
    const paletteToken = parts[6];
    const paletteValue = PALETTE_CONSTANTS[paletteToken];
    if (paletteValue === undefined) {
      throw new Error(`Unknown palette token '${paletteToken}' for map '${mapName}'.`);
    }
    paletteData[mapName] = (phoneFlag << 4) | paletteValue;
    timeOfDayData[mapName] = TIME_OF_DAY_BY_PALETTE[paletteToken];
  }

  return [paletteData, timeOfDayData];
}

function parseConnections(lines: string[], index: number, connectionFlags: string): [MapConnection[], number] {
  const connections: MapConnection[] = [];
  const numConnections = connectionFlags === "0" ? 0 : connectionFlags.split("|").length;
  let i = 0;
  while (i < numConnections && index + i < lines.length) {
    const line = lines[index + i].trim();
    if (!line.startsWith("connection")) {
      throw new Error(`Expected ${numConnections} connection rows after map_attributes, found ${i}.`);
    }
    const parts = line.split(",").map((part) => part.trim());
    if (parts.length < 4) {
      throw new Error(`Malformed connection row: ${line}`);
    }
    const direction = parts[0].split(/\s+/)[1];
    if (!direction || !parts[1]) {
      throw new Error(`Malformed connection row: ${line}`);
    }
    const offset = parts.length > 4 ? Number.parseInt(parts[3], 10) - Number.parseInt(parts[4], 10) : Number.parseInt(parts[3], 10);
    if (!Number.isFinite(offset)) {
      throw new Error(`Invalid connection offset in row: ${line}`);
    }
    connections.push({
      direction,
      target_map: parts[1],
      offset,
    });
    i += 1;
  }
  return [connections, index + i];
}

export function exportMapAttributes(): Record<string, MapAttributes> {
  const attributesPath = path.join(getDisassemblyRoot(), "data", "maps", "attributes.asm");
  const dimensionsPath = path.join(getDisassemblyRoot(), "constants", "map_constants.asm");
  const mapDefinitions = parseMapDefinitions();
  const mapToTilesetMap = loadMapTilesetMapping(mapDefinitions);
  const mapDimensions = parseMapDimensions(dimensionsPath);
  const [paletteData, timeOfDayData] = loadMapPaletteValues();
  const mapConstantsMeta = parseMapConstants();

  const lines = fs.readFileSync(attributesPath, "utf8").split(/\r?\n/);
  const mapAttributes: Record<string, MapAttributes> = {};

  let index = 0;
  while (index < lines.length) {
    const line = lines[index].trim();
    if (!line.startsWith("map_attributes")) {
      index += 1;
      continue;
    }
    const parts = line.split(",").map((part) => part.trim());
    const mapName = parts[0].split(/\s+/)[1];
    if (!mapName || parts.length < 4 || parts.some((part) => !part)) {
      throw new Error(`Malformed map_attributes row in ${attributesPath}: ${line}`);
    }
    const tilesetName = requireMappingValue(mapToTilesetMap, mapName, `tileset mapping for map '${mapName}'`);
    const mapConstant = mapNameToConstant(mapName);
    const mapMeta = requireMappingValue(mapDefinitions, mapName, `map definition for '${mapName}'`);
    const constantMeta = requireMappingValue(mapConstantsMeta, mapConstant, `map constant metadata for '${mapConstant}'`);
    const dimensionEntry = requireMappingValue(mapDimensions, mapConstant, `dimension entry for '${mapConstant}'`);
    const borderBlock = parseAsmNumber(parts[2]);
    const connectionFlags = parts[3];
    const [connections, newIndex] = parseConnections(lines, index + 1, connectionFlags);
    index = newIndex;

    mapAttributes[mapName] = {
      tileset_name: tilesetName,
      border_block: borderBlock,
      width: dimensionEntry.width,
      height: dimensionEntry.height,
      connections,
      time_of_day: requireExistingKey(timeOfDayData, mapName, `time-of-day palette for map '${mapName}'`),
      phone_service: requireMappingValue(paletteData, mapName, `phone service palette byte for map '${mapName}'`),
      phone_flag: Boolean(mapMeta.phone_flag),
      environment: requireStringField(mapMeta, "environment", mapName),
      location: requireStringField(mapMeta, "location", mapName),
      music: resolveMapMusicAssetId(requireStringField(mapMeta, "music", mapName), mapName),
      palette: requireStringField(mapMeta, "palette", mapName),
      fishing_group: requireStringField(mapMeta, "fishing_group", mapName),
      map_constant: mapConstant,
      map_group_constant: constantMeta.group_constant,
      blocks_label: `${mapName}_Blocks`,
      map_scripts_label: `${mapName}_MapScripts`,
      map_events_label: `${mapName}_MapEvents`,
      connection_flags: connectionFlags,
    };
  }

  writeJsonToTargets("map_attributes.json", mapAttributes, { indent: 4 });
  return mapAttributes;
}

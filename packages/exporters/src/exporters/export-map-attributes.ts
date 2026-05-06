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

function parseMapPhoneFlag(token: string): number {
  const normalized = token.trim().toUpperCase();
  if (normalized === "TRUE") return 1;
  if (normalized === "FALSE") return 0;
  return Number.parseInt(normalized, 10);
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
    if (parts.length < 8) continue;
    const nameTokens = parts[0].split(/\s+/);
    if (nameTokens.length < 2) continue;
    definitions[nameTokens[1]] = {
      tileset_constant: parts[1],
      environment: parts[2],
      location: parts[3],
      music: parts[4],
      phone_flag: parts[5].toUpperCase() === "TRUE",
      palette: parts[6],
      fishing_group: parts[7],
    };
  }
  return definitions;
}

export function parseMapConstants(): Record<string, { group_constant: string | null; width: number; height: number }> {
  const constantsPath = path.join(getDisassemblyRoot(), "constants", "map_constants.asm");
  const metadata: Record<string, { group_constant: string | null; width: number; height: number }> = {};
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
    metadata[mapConstant] = {
      group_constant: currentGroup,
      width: Number.parseInt(segments[1] ?? "0", 10),
      height: Number.parseInt((segments[2] ?? "0").trim(), 10),
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
    if (parts.length < 8) continue;
    const tokens = parts[0].split(/\s+/);
    if (tokens.length < 2) continue;
    const mapName = tokens[1];
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
    if (!line.startsWith("connection")) break;
    const parts = line.split(",").map((part) => part.trim());
    const offset = parts.length > 4 ? Number.parseInt(parts[3], 10) - Number.parseInt(parts[4], 10) : Number.parseInt(parts[3], 10);
    connections.push({
      direction: parts[0].split(/\s+/)[1],
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
    const tilesetName = requireMappingValue(mapToTilesetMap, mapName, `tileset mapping for map '${mapName}'`);
    const mapConstant = mapNameToConstant(mapName);
    const mapMeta = requireMappingValue(mapDefinitions, mapName, `map definition for '${mapName}'`);
    const constantMeta = requireMappingValue(mapConstantsMeta, mapConstant, `map constant metadata for '${mapConstant}'`);
    const dimensionEntry = requireMappingValue(mapDimensions, mapConstant, `dimension entry for '${mapConstant}'`);
    const borderBlock = parseAsmNumber(parts[2]);
    const connectionFlags = parts[3] ?? "0";
    const [connections, newIndex] = parseConnections(lines, index + 1, connectionFlags);
    index = newIndex;

    mapAttributes[mapName] = {
      tileset_name: tilesetName,
      border_block: borderBlock,
      width: dimensionEntry.width,
      height: dimensionEntry.height,
      connections,
      time_of_day: timeOfDayData[mapName] ?? null,
      phone_service: paletteData[mapName],
      phone_flag: Boolean(mapMeta.phone_flag),
      environment: String(mapMeta.environment ?? ""),
      location: String(mapMeta.location ?? ""),
      music: String(mapMeta.music ?? ""),
      palette: String(mapMeta.palette ?? ""),
      fishing_group: String(mapMeta.fishing_group ?? ""),
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


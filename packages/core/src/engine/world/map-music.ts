import path from "path";
import { readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir } from "@pokecrystal/core/core/paths";

const DEFAULT_MAP_TOKENS: Record<string, string> = {
  NewBarkTown: "MUSIC_NEW_BARK_TOWN",
  PlayersHouse1F: "MUSIC_NEW_BARK_TOWN",
  PlayersHouse2F: "MUSIC_NEW_BARK_TOWN",
  PlayersNeighborsHouse: "MUSIC_NEW_BARK_TOWN",
  ElmsLab: "MUSIC_ELMS_LAB",
};

const NORMALIZED_DEFAULT_MAP_TOKENS: Record<string, string> = {};

const MAP_ATTRIBUTES_PATH = path.join(getDataDir(), "map_attributes.json");

function normalizeMapKey(name: string): string {
  return name.replace(/[^A-Za-z0-9]/g, "").toUpperCase();
}

let mapMusicTable: Map<string, string> | null = null;

function loadMapMusicTable(): Map<string, string> {
  if (mapMusicTable) {
    return mapMusicTable;
  }

  const table = new Map<string, string>();
  let attributes: Record<string, { music?: unknown }>;
  try {
    attributes = readJsonAssetSync<Record<string, { music?: unknown }>>(MAP_ATTRIBUTES_PATH);
  } catch {
    attributes = {};
  }
  for (const [mapName, entry] of Object.entries(attributes)) {
    const musicToken = typeof entry?.music === "string" ? entry.music : "";
    if (!mapName || !musicToken) {
      continue;
    }
    table.set(mapName, musicToken);
    const normalized = normalizeMapKey(mapName);
    if (!table.has(normalized)) {
      table.set(normalized, musicToken);
    }
  }

  mapMusicTable = table;
  return table;
}

if (Object.keys(NORMALIZED_DEFAULT_MAP_TOKENS).length === 0) {
  for (const [name, token] of Object.entries(DEFAULT_MAP_TOKENS)) {
    NORMALIZED_DEFAULT_MAP_TOKENS[normalizeMapKey(name)] = token;
  }
}

export function defaultMusicTokenForMap(mapName: string): string {
  const key = String(mapName ?? "").trim();
  if (!key) {
    throw new Error("Cannot resolve music for empty map name.");
  }

  const normalizedKey = normalizeMapKey(key);

  if (key in DEFAULT_MAP_TOKENS) {
    return DEFAULT_MAP_TOKENS[key];
  }
  if (normalizedKey in NORMALIZED_DEFAULT_MAP_TOKENS) {
    return NORMALIZED_DEFAULT_MAP_TOKENS[normalizedKey];
  }

  const table = loadMapMusicTable();
  const direct = table.get(key);
  if (direct) {
    return direct;
  }
  const normalized = table.get(normalizedKey);
  if (normalized) {
    return normalized;
  }

  throw new Error(`No default music mapping for map '${mapName}'.`);
}

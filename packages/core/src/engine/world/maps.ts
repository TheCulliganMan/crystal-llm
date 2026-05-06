
import path from "path";
import { readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir } from "@pokecrystal/core/core/paths";
import { GameState } from "@pokecrystal/core/core/state";

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

const CONSTANT_TO_NAME_OVERRIDES: { [key: string]: string } = {
  GOLDENROD_PP_SPEECH_HOUSE: "GoldenrodPPSpeechHouse",
  WHIRL_ISLAND_NW: "WhirlIslandNW",
  WHIRL_ISLAND_NE: "WhirlIslandNE",
  WHIRL_ISLAND_SW: "WhirlIslandSW",
  WHIRL_ISLAND_SE: "WhirlIslandSE",
  FAST_SHIP_CABINS_SE_SSE_CAPTAINS_CABIN: "FastShipCabins_SE_SSE_CaptainsCabin",
};

const RUNTIME_MAP_METADATA_PATH = path.join(getDataDir(), "runtime_map_metadata.json");
const RUNTIME_SPAWN_POINTS_PATH = path.join(getDataDir(), "runtime_spawn_points.json");

export function mapConstantToName(constant: string): string {
  const override = CONSTANT_TO_NAME_OVERRIDES[constant];
  if (override) {
    return override;
  }

  const parts = constant.split("_");
  let suffixIndex: number | null = null;
  for (let i = 0; i < parts.length; i++) {
    if (DIRECTIONAL_SUFFIXES.has(parts[i])) {
      suffixIndex = i;
      break;
    }
  }

  const baseParts = suffixIndex !== null ? parts.slice(0, suffixIndex) : parts;
  const suffixParts = suffixIndex !== null ? parts.slice(suffixIndex) : [];

  const format = (part: string) => {
    if (!isNaN(parseInt(part))) {
      return part;
    }
    if (/\d/.test(part) && /^[A-Z0-9]+$/.test(part)) {
      return part.toUpperCase();
    }
    return part.charAt(0).toUpperCase() + part.slice(1).toLowerCase();
  };

  const baseName = baseParts.map(format).join("");
  if (suffixIndex !== null) {
    return `${baseName}_${suffixParts.join("_")}`;
  }
  return baseName;
}

export interface MapMetadata {
  constant: string;
  name: string;
  groupName: string;
  groupId: number;
  mapId: number;
  width: number;
  height: number;
  environment?: string;
  phoneService: number;
}

export interface SpawnPoint {
  identifier: Spawn;
  mapConstant: string;
  mapName: string;
  groupId: number;
  mapId: number;
  tileX: number;
  tileY: number;
  groupName: string;
  metatileX: number;
  metatileY: number;
  subtileX: number;
  subtileY: number;
}

export enum Spawn {
  N_A = -1,
  HOME = 0,
  DEBUG = 1,
  PALLET = 2,
  VIRIDIAN = 3,
  PEWTER = 4,
  CERULEAN = 5,
  ROCK_TUNNEL = 6,
  VERMILION = 7,
  LAVENDER = 8,
  SAFFRON = 9,
  CELADON = 10,
  FUCHSIA = 11,
  CINNABAR = 12,
  INDIGO = 13,
  NEW_BARK = 14,
  CHERRYGROVE = 15,
  VIOLET = 16,
  UNION_CAVE = 17,
  AZALEA = 18,
  CIANWOOD = 19,
  GOLDENROD = 20,
  OLIVINE = 21,
  ECRUTEAK = 22,
  MAHOGANY = 23,
  LAKE_OF_RAGE = 24,
  BLACKTHORN = 25,
  MT_SILVER = 26,
  FAST_SHIP = 27,
}

let mapMetadataCache: Record<string, MapMetadata> | null = null;
let mapMetadataByNameCache: Record<string, MapMetadata> | null = null;
let mapMetadataByGroupCache: Record<string, MapMetadata> | null = null;

const readRequiredRuntimeAsset = <T>(
  assetPath: string,
  description: string
): T => {
  try {
    return readJsonAssetSync<T>(assetPath);
  } catch {
    throw new Error(
      `${description} is required for the asset-only runtime: missing or invalid ${assetPath}.`
    );
  }
};

const loadMapMetadata = (): Record<string, MapMetadata> => {
  if (mapMetadataCache) {
    return mapMetadataCache;
  }
  const bundled = readRequiredRuntimeAsset<Record<string, MapMetadata>>(
    RUNTIME_MAP_METADATA_PATH,
    "Runtime map metadata"
  );
  if (!bundled || !Object.keys(bundled).length) {
    throw new Error(
      `Runtime map metadata is required for the asset-only runtime: missing or invalid ${RUNTIME_MAP_METADATA_PATH}.`
    );
  }
  mapMetadataCache = bundled;
  return mapMetadataCache;
};

const loadMapMetadataByName = (): Record<string, MapMetadata> => {
  if (mapMetadataByNameCache) {
    return mapMetadataByNameCache;
  }
  const byName: Record<string, MapMetadata> = {};
  for (const entry of Object.values(loadMapMetadata())) {
    byName[entry.name] = entry;
  }
  mapMetadataByNameCache = byName;
  return byName;
};

const loadMapMetadataByGroup = (): Record<string, MapMetadata> => {
  if (mapMetadataByGroupCache) {
    return mapMetadataByGroupCache;
  }
  const byGroup: Record<string, MapMetadata> = {};
  for (const entry of Object.values(loadMapMetadata())) {
    byGroup[`${entry.groupId}:${entry.mapId}`] = entry;
  }
  mapMetadataByGroupCache = byGroup;
  return byGroup;
};

const spawnPoints: { [key in Spawn]?: SpawnPoint } = {};
const spawnPointByGroupMap = new Map<string, Spawn>();
let spawnPointsLoaded = false;

const loadSpawnPoints = (): { [key in Spawn]?: SpawnPoint } => {
  if (spawnPointsLoaded) {
    return spawnPoints;
  }
  const bundled = readRequiredRuntimeAsset<Record<string, SpawnPoint>>(
    RUNTIME_SPAWN_POINTS_PATH,
    "Runtime spawn points"
  );
  if (!bundled || !Object.keys(bundled).length) {
    throw new Error(
      `Runtime spawn points are required for the asset-only runtime: missing or invalid ${RUNTIME_SPAWN_POINTS_PATH}.`
    );
  }
  spawnPointsLoaded = true;
  for (const [id, entry] of Object.entries(bundled)) {
    const numericId = Number(id) as Spawn;
    spawnPoints[numericId] = entry;
    if (entry.groupId !== -1 && entry.mapId !== -1) {
      spawnPointByGroupMap.set(`${entry.groupId}:${entry.mapId}`, numericId);
    }
  }
  return spawnPoints;
};

export function getMapMetadataByConstant(
  constant: string
): MapMetadata | undefined {
  return loadMapMetadata()[constant];
}

export function getMapMetadataByName(name: string): MapMetadata | undefined {
  return loadMapMetadataByName()[name];
}

export function getMapEnvironment(mapNameOrConstant: string): string | undefined {
  const metadata =
    getMapMetadataByName(mapNameOrConstant) ??
    getMapMetadataByConstant(mapNameOrConstant);
  return metadata?.environment;
}

export function getMapMetadataByGroup(
  groupId: number,
  mapId: number
): MapMetadata | undefined {
  return loadMapMetadataByGroup()[`${groupId}:${mapId}`];
}

export function getSpawnPoint(identifier: Spawn): SpawnPoint {
  loadSpawnPoints();
  const spawnPoint = spawnPoints[identifier];
  if (!spawnPoint) {
    throw new Error(`Unknown spawn identifier ${identifier}`);
  }
  return spawnPoint;
}

export function findSpawnForMap(
  groupId: number,
  mapId: number
): [Spawn, SpawnPoint] | undefined {
  loadSpawnPoints();
  const cached = spawnPointByGroupMap.get(`${groupId}:${mapId}`);
  if (cached !== undefined) {
    const spawnPoint = spawnPoints[cached];
    if (spawnPoint) {
      return [cached, spawnPoint];
    }
  }
  for (const [id, spawn] of Object.entries(spawnPoints)) {
    if (spawn && spawn.groupId === groupId && spawn.mapId === mapId) {
      return [Number(id) as Spawn, spawn];
    }
  }
  return undefined;
}

export function applySpawn(gameState: GameState, identifier: Spawn): void {
  const spawn = getSpawnPoint(identifier);
  gameState.sram.last_spawn_map_group = spawn.groupId;
  gameState.sram.last_spawn_map_number = spawn.mapId;
  gameState.wram.wLastSpawnMapGroup = spawn.groupId;
  gameState.wram.wLastSpawnMapNumber = spawn.mapId;
  gameState.wram.wMapGroup = spawn.groupId;
  gameState.wram.wMapNumber = spawn.mapId;
  gameState.wram.current_map_group = spawn.groupId;
  gameState.wram.current_map_id = spawn.mapId;
  gameState.wram.wXCoord = spawn.tileX;
  gameState.wram.wYCoord = spawn.tileY;
  gameState.wram.player_x = spawn.metatileX;
  gameState.wram.player_y = spawn.metatileY;
  gameState.wram.player_subtile_x = spawn.subtileX;
  gameState.wram.player_subtile_y = spawn.subtileY;
  gameState.wram.wDefaultSpawnpoint = identifier;
  gameState.wram.scene_name = "";
}

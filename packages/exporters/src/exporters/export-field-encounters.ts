import { readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir } from "@pokecrystal/core/core/paths";
import { joinPath } from "@pokecrystal/core/core/path-utils";
import {
  ROCK_MON_MAPS,
  TREE_MON_MAPS,
  TREE_MON_SETS,
  type TreeMonEntry,
  type TreeMonSet,
} from "@pokecrystal/assets/content/tree-encounters";
import { writeJsonToTargets } from "./asm-utils";

export type ExportedFieldEncounterEntry = {
  weight: number;
  species: string;
  level: number;
};

export type ExportedFieldEncounterTable = {
  common: ExportedFieldEncounterEntry[];
  rare: ExportedFieldEncounterEntry[];
};

export type ExportedFieldEncounterData = {
  map_name: string;
  tables: Record<string, ExportedFieldEncounterTable>;
};

type RuntimeMapMetadata = {
  name: string;
};

const cloneEntry = (entry: TreeMonEntry): ExportedFieldEncounterEntry => ({
  weight: entry.weight,
  species: entry.species,
  level: entry.level,
});

const cloneSet = (set: TreeMonSet): ExportedFieldEncounterTable => ({
  common: set.common.map(cloneEntry),
  rare: set.rare.map(cloneEntry),
});

const mapNameForConstant = (
  metadata: Record<string, RuntimeMapMetadata>,
  mapConstant: string
): string => {
  const map = metadata[mapConstant];
  if (!map?.name) {
    throw new Error(`Field encounter map ${mapConstant} is missing runtime map metadata.`);
  }
  return map.name;
};

const resolveSet = (setName: string, mapConstant: string): ExportedFieldEncounterTable | null => {
  if (setName === "TREEMON_SET_NONE") {
    return null;
  }
  const set = TREE_MON_SETS[setName];
  if (!set) {
    throw new Error(`Field encounter map ${mapConstant} references missing ${setName}.`);
  }
  return cloneSet(set);
};

export function exportFieldEncounters(): ExportedFieldEncounterData[] {
  const metadata = readJsonAssetSync(
    joinPath(getDataDir(), "runtime_map_metadata.json")
  ) as Record<string, RuntimeMapMetadata>;
  const byMapName = new Map<string, ExportedFieldEncounterData>();

  for (const [mapConstant, setName] of Object.entries(TREE_MON_MAPS)) {
    const headbutt = resolveSet(setName, mapConstant);
    if (!headbutt) {
      continue;
    }
    const mapName = mapNameForConstant(metadata, mapConstant);
    byMapName.set(mapName, {
      map_name: mapName,
      tables: {
        headbutt,
      },
    });
  }

  for (const [mapConstant, setName] of Object.entries(ROCK_MON_MAPS)) {
    const rockSmash = resolveSet(setName, mapConstant);
    if (!rockSmash) {
      continue;
    }
    const mapName = mapNameForConstant(metadata, mapConstant);
    const existing = byMapName.get(mapName);
    byMapName.set(mapName, {
      map_name: mapName,
      tables: {
        ...(existing?.tables ?? {}),
        rock_smash: rockSmash,
      },
    });
  }

  const fieldEncounters = [...byMapName.values()].sort((a, b) =>
    a.map_name.localeCompare(b.map_name)
  );
  writeJsonToTargets("field_encounters.json", fieldEncounters, { indent: 2 });
  return fieldEncounters;
}

import fs from "fs";
import path from "path";
import { readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir, getDisassemblyRoot } from "@pokecrystal/core/core/paths";
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
  sleep_turns_by_time: Partial<Record<TreeSleepTime, number>>;
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

export type TreeSleepTime = "morning" | "day" | "night";

export type TreeSleepRules = {
  sleepTurns: number;
  speciesByTime: Record<TreeSleepTime, Set<string>>;
};

const TREE_SLEEP_LABELS = {
  AsleepTreeMonsMorn: "morning",
  AsleepTreeMonsDay: "day",
  AsleepTreeMonsNite: "night",
} as const;

const parseAsmInteger = (token: string): number => {
  const value = token.startsWith("$")
    ? Number.parseInt(token.slice(1), 16)
    : Number.parseInt(token, 10);
  if (!Number.isSafeInteger(value)) {
    throw new Error(`Invalid ASM integer ${token}.`);
  }
  return value;
};

export function parseTreeSleepRules(
  asleepTreeMonsSource: string,
  battleConstantsSource: string
): TreeSleepRules {
  const sleepTurnsMatch = battleConstantsSource.match(
    /^\s*DEF\s+TREEMON_SLEEP_TURNS\s+EQU\s+(\$[0-9a-f]+|\d+)\s*(?:;.*)?$/im
  );
  if (!sleepTurnsMatch) {
    throw new Error("Missing TREEMON_SLEEP_TURNS in constants/battle_constants.asm.");
  }
  const sleepTurns = parseAsmInteger(sleepTurnsMatch[1]);
  if (sleepTurns < 1 || sleepTurns > 7) {
    throw new Error(
      `TREEMON_SLEEP_TURNS must fit the Gen 2 sleep counter (1..7), found ${sleepTurns}.`
    );
  }

  const speciesByTime: TreeSleepRules["speciesByTime"] = {
    morning: new Set<string>(),
    day: new Set<string>(),
    night: new Set<string>(),
  };
  const terminated = new Set<keyof typeof speciesByTime>();
  let currentTime: keyof typeof speciesByTime | null = null;
  for (const rawLine of asleepTreeMonsSource.split(/\r?\n/)) {
    const line = rawLine.split(";", 1)[0].trim();
    if (!line) {
      continue;
    }
    const labelMatch = line.match(/^([A-Za-z_][A-Za-z0-9_]*):$/);
    if (labelMatch) {
      currentTime = TREE_SLEEP_LABELS[labelMatch[1] as keyof typeof TREE_SLEEP_LABELS] ?? null;
      continue;
    }
    if (!currentTime) {
      continue;
    }
    const dataMatch = line.match(/^db\s+([^,\s]+)\s*$/i);
    if (!dataMatch) {
      throw new Error(`Unsupported sleeping tree-mon row: ${rawLine.trim()}`);
    }
    const token = dataMatch[1];
    if (token === "-1") {
      terminated.add(currentTime);
      currentTime = null;
      continue;
    }
    if (!/^[A-Z][A-Z0-9_]*$/.test(token)) {
      throw new Error(`Invalid sleeping tree-mon species token ${token}.`);
    }
    if (speciesByTime[currentTime].has(token)) {
      throw new Error(`Duplicate ${token} in the ${currentTime} sleeping tree-mon table.`);
    }
    speciesByTime[currentTime].add(token);
  }
  for (const time of Object.values(TREE_SLEEP_LABELS)) {
    if (!terminated.has(time)) {
      throw new Error(`Missing terminated ${time} sleeping tree-mon table.`);
    }
  }
  return { sleepTurns, speciesByTime };
}

const sleepTurnsByTime = (
  species: string,
  rules: TreeSleepRules
): Partial<Record<TreeSleepTime, number>> =>
  Object.fromEntries(
    (["morning", "day", "night"] as const)
      .filter((time) => rules.speciesByTime[time].has(species))
      .map((time) => [time, rules.sleepTurns])
  );

const cloneEntry = (
  entry: TreeMonEntry,
  sleepRules: TreeSleepRules | null
): ExportedFieldEncounterEntry => ({
  weight: entry.weight,
  species: entry.species,
  level: entry.level,
  sleep_turns_by_time: sleepRules ? sleepTurnsByTime(entry.species, sleepRules) : {},
});

const cloneSet = (
  set: TreeMonSet,
  sleepRules: TreeSleepRules | null
): ExportedFieldEncounterTable => ({
  common: set.common.map((entry) => cloneEntry(entry, sleepRules)),
  rare: set.rare.map((entry) => cloneEntry(entry, sleepRules)),
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

const resolveSet = (
  setName: string,
  mapConstant: string,
  sleepRules: TreeSleepRules | null
): ExportedFieldEncounterTable | null => {
  if (setName === "TREEMON_SET_NONE") {
    return null;
  }
  const set = TREE_MON_SETS[setName];
  if (!set) {
    throw new Error(`Field encounter map ${mapConstant} references missing ${setName}.`);
  }
  return cloneSet(set, sleepRules);
};

export function exportFieldEncounters(): ExportedFieldEncounterData[] {
  const root = getDisassemblyRoot();
  const sleepRules = parseTreeSleepRules(
    fs.readFileSync(path.join(root, "data", "wild", "treemons_asleep.asm"), "utf8"),
    fs.readFileSync(path.join(root, "constants", "battle_constants.asm"), "utf8")
  );
  const metadata = readJsonAssetSync(
    joinPath(getDataDir(), "runtime_map_metadata.json")
  ) as Record<string, RuntimeMapMetadata>;
  const byMapName = new Map<string, ExportedFieldEncounterData>();

  for (const [mapConstant, setName] of Object.entries(TREE_MON_MAPS)) {
    const headbutt = resolveSet(setName, mapConstant, sleepRules);
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
    const rockSmash = resolveSet(setName, mapConstant, null);
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

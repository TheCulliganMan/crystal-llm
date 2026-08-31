import fs from "fs";
import path from "path";
import type {
  WildEncounter,
  WildEncounterData,
  WildEncounterSwarmOverride,
  WildEncounterTable,
} from "@pokecrystal/assets/content/wild-encounter-data";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { mapConstantToName } from "@pokecrystal/core/engine/world/maps";
import { writeJsonToTargets } from "./asm-utils";

function hasEncounters(table: WildEncounterTable | null | undefined): boolean {
  return Boolean(table && (table.morning.length || table.day.length || table.night.length));
}

function hasSurfaceData(rate: WildEncounterData["grass_rates"] | WildEncounterData["water_rate"], table: WildEncounterTable | null | undefined): boolean {
  return rate !== null || table !== null || hasEncounters(table);
}

export type WildEncounterSwarmDeclaration = {
  swarm_token: string;
  engine_flag: string;
  map_constant: string;
};

export type ParsedWildEncounterSwarm = {
  map_name: string;
  swarm_token: string;
  override: WildEncounterSwarmOverride;
};

export function parseWildEncounterSwarmDeclarations(filePaths: Iterable<string>): WildEncounterSwarmDeclaration[] {
  const declarations = new Map<string, WildEncounterSwarmDeclaration>();
  for (const filePath of filePaths) {
    let precedingEngineFlag: string | null = null;
    for (const rawLine of fs.readFileSync(filePath, "utf8").split(/\r?\n/)) {
      const line = rawLine.replace(/;.*/, "").trim();
      const flag = line.match(/^setflag\s+(ENGINE_[A-Z0-9_]+_SWARM)$/)?.[1];
      if (flag) {
        precedingEngineFlag = flag;
        continue;
      }
      const swarm = line.match(/^swarm\s+(SWARM_[A-Z0-9_]+),\s*([A-Z0-9_]+)$/);
      if (!swarm) {
        continue;
      }
      if (!precedingEngineFlag) {
        throw new Error(`Swarm declaration ${swarm[1]} in ${filePath} has no preceding engine flag.`);
      }
      const declaration = {
        swarm_token: swarm[1],
        engine_flag: precedingEngineFlag,
        map_constant: swarm[2],
      };
      const existing = declarations.get(declaration.swarm_token);
      if (existing && JSON.stringify(existing) !== JSON.stringify(declaration)) {
        throw new Error(`Conflicting declarations for ${declaration.swarm_token}.`);
      }
      declarations.set(declaration.swarm_token, declaration);
      precedingEngineFlag = null;
    }
  }
  return [...declarations.values()];
}

export function parseWildEncounterSwarms(
  filePath: string,
  declarations: Iterable<WildEncounterSwarmDeclaration>,
): ParsedWildEncounterSwarm[] {
  const declarationByMap = new Map(
    [...declarations].map((declaration) => [declaration.map_constant, declaration]),
  );
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/);
  const starts = lines
    .map((line, index) => ({ index, map: line.trim().match(/^map_id\s+([A-Z0-9_]+)$/)?.[1] }))
    .filter((entry): entry is { index: number; map: string } => Boolean(entry.map));
  const parsed: ParsedWildEncounterSwarm[] = [];
  for (const [recordIndex, start] of starts.entries()) {
    const declaration = declarationByMap.get(start.map);
    if (!declaration) {
      throw new Error(`Swarm table map ${start.map} in ${filePath} has no source script declaration.`);
    }
    const end = starts[recordIndex + 1]?.index ?? lines.length;
    const body = lines.slice(start.index + 1, end);
    const rateLine = body.find((line) => /\bpercent\b/.test(line));
    const rates = [...(rateLine ?? "").matchAll(/(\d+)\s+percent/g)].map((match) => Number.parseInt(match[1], 10));
    if (rates.length !== 3) {
      throw new Error(`Could not parse three grass swarm rates for ${start.map} in ${filePath}.`);
    }
    const table: WildEncounterTable = { morning: [], day: [], night: [] };
    let time: keyof WildEncounterTable | null = null;
    for (const rawLine of body) {
      const trimmed = rawLine.trim();
      if (trimmed === "; morn") time = "morning";
      else if (trimmed === "; day") time = "day";
      else if (trimmed === "; nite") time = "night";
      else {
        const row = trimmed.match(/^db\s+(\d+),\s*([A-Z0-9_]+)$/);
        if (row && time) {
          table[time].push({ level: Number.parseInt(row[1], 10), species: row[2] });
        }
      }
    }
    for (const timeKey of ["morning", "day", "night"] as const) {
      if (table[timeKey].length !== 7) {
        throw new Error(`Grass swarm ${start.map} has ${table[timeKey].length} ${timeKey} slots, expected 7.`);
      }
    }
    parsed.push({
      map_name: mapConstantToName(start.map),
      swarm_token: declaration.swarm_token,
      override: {
        engine_flag: declaration.engine_flag,
        grass_rates: { morning: rates[0], day: rates[1], night: rates[2] },
        grass: table,
      },
    });
  }
  return parsed;
}

export function parseWildEncounters(filePath: string): WildEncounterData[] {
  const content = fs.readFileSync(filePath, "utf8");
  const wildData: WildEncounterData[] = [];
  const blocks = content.split(/def_grass_wildmons|def_water_wildmons/g).slice(1);

  for (const block of blocks) {
    const lines = block.split(/\r?\n/).map((line) => line.trimEnd()).filter((line) => line.trim());
    if (lines.length === 0) continue;
    const mapName = mapConstantToName(lines[0].trim());
    const isWater = path.basename(filePath).includes("water");
    const rateLine = lines[1] ?? "";
    const rates = [...rateLine.matchAll(/(\d+)\s+percent/g)].map((match) => Number.parseInt(match[1], 10));

    const morningEncounters: WildEncounter[] = [];
    const dayEncounters: WildEncounter[] = [];
    const nightEncounters: WildEncounter[] = [];
    const waterEncounters: WildEncounter[] = [];
    let grassRates: WildEncounterData["grass_rates"] = null;
    let waterRate: number | null = null;

    if (isWater) {
      if (rates.length === 0) {
        throw new Error(`Could not parse water encounter rate for ${mapName} in ${filePath}`);
      }
      waterRate = rates[0];
    } else {
      if (rates.length < 3) {
        throw new Error(`Could not parse grass encounter rates for ${mapName} in ${filePath}`);
      }
      grassRates = { morning: rates[0], day: rates[1], night: rates[2] };
    }

    let timeOfDay: "morn" | "day" | "nite" | null = null;
    for (const rawLine of lines.slice(2)) {
      const line = rawLine.trim();
      if (!line) continue;
      if (line.startsWith("end_")) break;
      if (isWater) {
        if (line.startsWith("db ") && !line.includes("; end")) {
          const parts = line.split("db", 2)[1].trim().split(",").map((part) => part.trim());
          waterEncounters.push({ level: Number.parseInt(parts[0], 10), species: parts[1] });
        }
        continue;
      }
      if (line.startsWith("; morn")) {
        timeOfDay = "morn";
        continue;
      }
      if (line.startsWith("; day")) {
        timeOfDay = "day";
        continue;
      }
      if (line.startsWith("; nite")) {
        timeOfDay = "nite";
        continue;
      }
      if (line.includes("db") && !line.includes("; end")) {
        const parts = line.split("db", 2)[1].trim().split(",").map((part) => part.trim());
        const encounter = { level: Number.parseInt(parts[0], 10), species: parts[1] };
        if (timeOfDay === "morn") morningEncounters.push(encounter);
        else if (timeOfDay === "day") dayEncounters.push(encounter);
        else if (timeOfDay === "nite") nightEncounters.push(encounter);
        else throw new Error(`Encounter line appeared before a time-of-day marker in ${mapName}: ${line}`);
      }
    }

    wildData.push({
      map_name: mapName,
      grass_rates: grassRates,
      water_rate: waterRate,
      grass: isWater ? null : { morning: morningEncounters, day: dayEncounters, night: nightEncounters },
      water: isWater ? { morning: waterEncounters, day: [...waterEncounters], night: [...waterEncounters] } : null,
    });
  }

  return wildData;
}

export function mergeWildEncounterData(collections: Iterable<Iterable<WildEncounterData>>): WildEncounterData[] {
  const merged = new Map<string, WildEncounterData>();
  for (const collection of collections) {
    for (const entry of collection) {
      const existing = merged.get(entry.map_name);
      if (!existing) {
        merged.set(entry.map_name, entry);
        continue;
      }
      if (hasSurfaceData(entry.grass_rates, entry.grass) && hasSurfaceData(existing.grass_rates, existing.grass)) {
        throw new Error(`Duplicate grass wild encounter data for ${entry.map_name}.`);
      }
      if (hasSurfaceData(entry.water_rate, entry.water) && hasSurfaceData(existing.water_rate, existing.water)) {
        throw new Error(`Duplicate water wild encounter data for ${entry.map_name}.`);
      }
      merged.set(entry.map_name, {
        map_name: entry.map_name,
        grass_rates: entry.grass_rates ?? existing.grass_rates,
        water_rate: entry.water_rate ?? existing.water_rate,
        grass: entry.grass ?? existing.grass,
        water: entry.water ?? existing.water,
        swarm_overrides: {
          ...(existing.swarm_overrides ?? {}),
          ...(entry.swarm_overrides ?? {}),
        },
      });
    }
  }
  return [...merged.values()];
}

export function exportWildEncounters(): WildEncounterData[] {
  const root = path.join(getDisassemblyRoot(), "data", "wild");
  const collections = ["johto_grass.asm", "johto_water.asm", "kanto_grass.asm", "kanto_water.asm"]
    .map((name) => {
      const filePath = path.join(root, name);
      if (!fs.existsSync(filePath)) {
        throw new Error(`Missing required wild encounter source ${filePath}.`);
      }
      return filePath;
    })
    .map((filePath) => parseWildEncounters(filePath));
  const merged = mergeWildEncounterData(collections);
  const phoneScriptsRoot = path.join(getDisassemblyRoot(), "engine", "phone", "scripts");
  const phoneScriptPaths = fs.readdirSync(phoneScriptsRoot)
    .filter((name) => name.endsWith(".asm"))
    .map((name) => path.join(phoneScriptsRoot, name));
  const swarmDeclarations = parseWildEncounterSwarmDeclarations(phoneScriptPaths);
  const swarms = parseWildEncounterSwarms(path.join(root, "swarm_grass.asm"), swarmDeclarations);
  const byMap = new Map(merged.map((entry) => [entry.map_name, entry]));
  for (const swarm of swarms) {
    const entry = byMap.get(swarm.map_name);
    if (!entry) {
      throw new Error(`Swarm ${swarm.swarm_token} references map ${swarm.map_name} without normal wild data.`);
    }
    entry.swarm_overrides = {
      ...(entry.swarm_overrides ?? {}),
      [swarm.swarm_token]: swarm.override,
    };
  }
  writeJsonToTargets("wild_encounters.json", merged, { indent: 2 });
  return merged;
}

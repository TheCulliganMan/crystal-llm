import fs from "fs";
import path from "path";
import type { WildEncounter, WildEncounterData, WildEncounterTable } from "@pokecrystal/assets/content/wild-encounter-data";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { mapConstantToName } from "@pokecrystal/core/engine/world/maps";
import { writeJsonToTargets } from "./asm-utils";

function emptyTable(): WildEncounterTable {
  return { morning: [], day: [], night: [] };
}

function hasEncounters(table: WildEncounterTable | null | undefined): boolean {
  return Boolean(table && (table.morning.length || table.day.length || table.night.length));
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
      grass: isWater ? emptyTable() : { morning: morningEncounters, day: dayEncounters, night: nightEncounters },
      water: isWater ? { morning: waterEncounters, day: [...waterEncounters], night: [...waterEncounters] } : emptyTable(),
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
      if ((entry.grass_rates !== null || hasEncounters(entry.grass)) && (existing.grass_rates !== null || hasEncounters(existing.grass))) {
        throw new Error(`Duplicate grass wild encounter data for ${entry.map_name}.`);
      }
      if ((entry.water_rate !== null || hasEncounters(entry.water)) && (existing.water_rate !== null || hasEncounters(existing.water))) {
        throw new Error(`Duplicate water wild encounter data for ${entry.map_name}.`);
      }
      merged.set(entry.map_name, {
        map_name: entry.map_name,
        grass_rates: entry.grass_rates ?? existing.grass_rates,
        water_rate: entry.water_rate ?? existing.water_rate,
        grass: hasEncounters(entry.grass) ? entry.grass : existing.grass,
        water: hasEncounters(entry.water) ? entry.water : existing.water,
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
  writeJsonToTargets("wild_encounters.json", merged, { indent: 2 });
  return merged;
}

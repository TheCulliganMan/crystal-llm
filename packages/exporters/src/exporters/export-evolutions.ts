import fs from "fs";
import path from "path";
import type { PokemonEvolutionData, EvolutionData } from "@pokecrystal/assets/content/evolution-data";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripAsmComment, writeJsonToTargets } from "./asm-utils";

const BLOCK_LABEL_RE = /^(?<species>[A-Za-z0-9_]+)EvosAttacks:\s*$/;

export type ExportedEvolutionData = Omit<EvolutionData, "level" | "item" | "held_item" | "happiness" | "stat_ratio"> & {
  level: number | null;
  item: string | null;
  held_item: string | null;
  happiness: string | null;
  stat_ratio: string | null;
};

export type ExportedPokemonEvolutionData = Omit<PokemonEvolutionData, "evolutions"> & {
  evolutions: ExportedEvolutionData[];
};

function splitArgs(args: string): string[] {
  return args.split(",").map((part) => part.trim()).filter(Boolean);
}

function completeEvolution(evolution: EvolutionData): ExportedEvolutionData {
  return {
    method: evolution.method,
    species: evolution.species,
    level: evolution.level ?? null,
    item: evolution.item ?? null,
    held_item: evolution.held_item ?? null,
    happiness: evolution.happiness ?? null,
    stat_ratio: evolution.stat_ratio ?? null,
  };
}

function parseEvolutionLine(line: string): ExportedEvolutionData {
  const body = stripAsmComment(line);
  if (!body.startsWith("db EVOLVE_")) {
    throw new Error(`Expected evolution line, got: ${line}`);
  }
  const [methodToken, ...rest] = body.slice(3).split(",");
  if (!methodToken || rest.length === 0) {
    throw new Error(`Malformed evolution line: ${line}`);
  }
  const method = methodToken.trim().replace("EVOLVE_", "") as EvolutionData["method"];
  const args = splitArgs(rest.join(","));

  if (method === "LEVEL") {
    if (args.length !== 2) throw new Error(`LEVEL evolution requires 2 args: ${line}`);
    return completeEvolution({ method, level: Number.parseInt(args[0], 10), species: args[1] });
  }
  if (method === "ITEM") {
    if (args.length !== 2) throw new Error(`ITEM evolution requires 2 args: ${line}`);
    return completeEvolution({ method, item: args[0], species: args[1] });
  }
  if (method === "TRADE") {
    if (args.length !== 2) throw new Error(`TRADE evolution requires 2 args: ${line}`);
    return completeEvolution({ method, held_item: args[0], species: args[1] });
  }
  if (method === "HAPPINESS") {
    if (args.length !== 2) throw new Error(`HAPPINESS evolution requires 2 args: ${line}`);
    return completeEvolution({ method, happiness: args[0], species: args[1] });
  }
  if (method === "STAT") {
    if (args.length !== 3) throw new Error(`STAT evolution requires 3 args: ${line}`);
    return completeEvolution({
      method,
      level: Number.parseInt(args[0], 10),
      stat_ratio: args[1],
      species: args[2],
    });
  }
  throw new Error(`Unhandled evolution method: ${method}`);
}

function asmLabelForSpeciesId(speciesId: string): string {
  return speciesId
    .toLowerCase()
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join("");
}

export function parseSpeciesConstants(constantsPath: string): string[] {
  const species: string[] = [];
  for (const rawLine of fs.readFileSync(constantsPath, "utf8").split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    const match = line.match(/^const\s+([A-Z0-9_]+)$/);
    if (!match) {
      continue;
    }
    const speciesId = match[1];
    if (speciesId.startsWith("UNOWN_") || speciesId === "EGG") {
      continue;
    }
    species.push(speciesId);
  }
  return species;
}

export function buildSpeciesLabelMap(speciesIds: string[]): Map<string, string> {
  const labels = new Map<string, string>();
  for (const speciesId of speciesIds) {
    const label = asmLabelForSpeciesId(speciesId);
    if (labels.has(label)) {
      throw new Error(`Duplicate ASM evolution label '${label}' from species constants.`);
    }
    labels.set(label, speciesId);
  }
  return labels;
}

function requireSpeciesLabel(label: string, speciesLabels: Map<string, string>): string {
  const species = speciesLabels.get(label);
  if (!species) {
    throw new Error(`Unknown or case-changed evolution species label '${label}'.`);
  }
  return species;
}

function validateEvolutionTargets(evolution: ExportedEvolutionData, speciesIds: Set<string>): ExportedEvolutionData {
  if (!speciesIds.has(evolution.species)) {
    throw new Error(`Unknown evolution target species '${evolution.species}'.`);
  }
  return evolution;
}

export function parseEvolutions(filePath: string, speciesIds: string[]): ExportedPokemonEvolutionData[] {
  const speciesLabels = buildSpeciesLabelMap(speciesIds);
  const speciesIdSet = new Set(speciesIds);
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/);
  const evolutions: ExportedPokemonEvolutionData[] = [];
  const seenSpecies = new Set<string>();
  let currentSpecies: string | null = null;
  let currentEvolutions: ExportedEvolutionData[] = [];
  let inAttackSection = false;

  const flushCurrent = () => {
    if (currentSpecies !== null) {
      const species = requireSpeciesLabel(currentSpecies, speciesLabels);
      if (seenSpecies.has(species)) {
        throw new Error(`Duplicate evolution block for species '${species}'.`);
      }
      seenSpecies.add(species);
      evolutions.push({
        species,
        evolutions: currentEvolutions.map((evolution) => validateEvolutionTargets(evolution, speciesIdSet)),
      });
    }
    currentSpecies = null;
    currentEvolutions = [];
    inAttackSection = false;
  };

  for (const rawLine of lines) {
    const stripped = stripAsmComment(rawLine);
    if (!stripped) continue;
    const labelMatch = stripped.match(BLOCK_LABEL_RE);
    if (labelMatch?.groups?.species) {
      flushCurrent();
      currentSpecies = labelMatch.groups.species;
      continue;
    }
    if (currentSpecies === null) continue;
    if (stripped === "db 0") {
      inAttackSection = true;
      continue;
    }
    if (inAttackSection) continue;
    if (stripped.startsWith("db EVOLVE_")) {
      currentEvolutions.push(parseEvolutionLine(stripped));
    }
  }
  flushCurrent();
  for (const species of speciesIds) {
    if (!seenSpecies.has(species)) {
      throw new Error(`Missing evolution block for species '${species}'.`);
    }
  }
  return evolutions;
}

export function exportEvolutions(): ExportedPokemonEvolutionData[] {
  const root = getDisassemblyRoot();
  const speciesIds = parseSpeciesConstants(path.join(root, "constants", "pokemon_constants.asm"));
  const data = parseEvolutions(path.join(root, "data", "pokemon", "evos_attacks.asm"), speciesIds);
  writeJsonToTargets("evolutions.json", data, { indent: 2 });
  return data;
}

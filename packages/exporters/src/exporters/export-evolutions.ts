import fs from "fs";
import path from "path";
import type { PokemonEvolutionData, EvolutionData } from "@pokecrystal/assets/content/evolution-data";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripAsmComment, writeJsonToTargets } from "./asm-utils";

const BLOCK_LABEL_RE = /^(?<species>[A-Za-z0-9_]+)EvosAttacks:\s*$/;
const SPECIES_LABEL_ALIASES: Record<string, string> = {
  NIDORANF: "NIDORAN_F",
  NIDORANM: "NIDORAN_M",
  FARFETCHD: "FARFETCH_D",
  MRMIME: "MR__MIME",
  HOOH: "HO_OH",
};

function splitArgs(args: string): string[] {
  return args.split(",").map((part) => part.trim()).filter(Boolean);
}

function parseEvolutionLine(line: string): EvolutionData {
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
    return { method, level: Number.parseInt(args[0], 10), species: args[1] };
  }
  if (method === "ITEM") {
    if (args.length !== 2) throw new Error(`ITEM evolution requires 2 args: ${line}`);
    return { method, item: args[0], species: args[1] };
  }
  if (method === "TRADE") {
    if (args.length !== 2) throw new Error(`TRADE evolution requires 2 args: ${line}`);
    return { method, held_item: args[0], species: args[1] };
  }
  if (method === "HAPPINESS") {
    if (args.length !== 2) throw new Error(`HAPPINESS evolution requires 2 args: ${line}`);
    return { method, happiness: args[0], species: args[1] };
  }
  if (method === "STAT") {
    if (args.length !== 3) throw new Error(`STAT evolution requires 3 args: ${line}`);
    return {
      method,
      level: Number.parseInt(args[0], 10),
      stat_ratio: args[1],
      species: args[2],
    };
  }
  throw new Error(`Unhandled evolution method: ${method}`);
}

function normalizeSpeciesLabel(label: string): string {
  const normalized = label.toUpperCase().replace(/[^A-Z0-9]/g, "");
  return SPECIES_LABEL_ALIASES[normalized] ?? label.toUpperCase();
}

export function parseEvolutions(filePath: string): PokemonEvolutionData[] {
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/);
  const evolutions: PokemonEvolutionData[] = [];
  let currentSpecies: string | null = null;
  let currentEvolutions: EvolutionData[] = [];
  let inAttackSection = false;

  const flushCurrent = () => {
    if (currentSpecies !== null && currentEvolutions.length > 0) {
      evolutions.push({ species: normalizeSpeciesLabel(currentSpecies), evolutions: [...currentEvolutions] });
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
  return evolutions;
}

export function exportEvolutions(): PokemonEvolutionData[] {
  const data = parseEvolutions(path.join(getDisassemblyRoot(), "data", "pokemon", "evos_attacks.asm"));
  writeJsonToTargets("evolutions.json", data, { indent: 2 });
  return data;
}

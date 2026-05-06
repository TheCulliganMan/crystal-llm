import fs from "fs";
import path from "path";
import type { PokedexData } from "@pokecrystal/assets/content/pokedex-data";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripAsmComment, writeJsonToTargets } from "./asm-utils";

const TEXT_LITERAL_RE = /"([^"]*)"/;
const POKEDEX_INCLUDE_RE = /INCLUDE\s+"data\/pokemon\/dex_entries\/(?<name>[^"]+\.asm)"/;
const INCHES_TO_METERS = 0.0254;
const POUNDS_TO_KG = 0.45359237;

function extractTextLiteral(line: string): string {
  const match = line.match(TEXT_LITERAL_RE);
  return match?.[1] ?? "";
}

function cleanLine(line: string): string {
  return line.replace(/@/g, "").trim();
}

function* iterEntryLines(filePath: string): Generator<string> {
  for (const rawLine of fs.readFileSync(filePath, "utf8").split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith(";")) continue;
    yield line;
  }
}

function assertPokedexDisassemblySource(root: string): void {
  const requiredPaths = [
    path.join(root, "data", "pokemon", "dex_entries.asm"),
    path.join(root, "data", "pokemon", "dex_entries"),
    path.join(root, "engine", "pokedex", "pokedex.asm"),
  ];
  const missing = requiredPaths.filter((sourcePath) => !fs.existsSync(sourcePath));
  if (missing.length) {
    throw new Error(
      [
        "Pokédex export requires a complete pret/pokecrystal disassembly checkout.",
        `Resolved root: ${root}`,
        `Missing: ${missing.join(", ")}`,
        "Set POKECRYSTAL_DISASSEMBLY_ROOT or clone pret/pokecrystal into vendor/pokecrystal before exporting Pokédex data.",
      ].join(" ")
    );
  }
}

export function iterPokedexEntryPaths(): string[] {
  const root = getDisassemblyRoot();
  assertPokedexDisassemblySource(root);
  const dexEntriesPath = path.join(root, "data", "pokemon", "dex_entries.asm");
  const baseDir = path.join(root, "data", "pokemon", "dex_entries");
  const entryPaths: string[] = [];

  for (const rawLine of fs.readFileSync(dexEntriesPath, "utf8").split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    const match = line.match(POKEDEX_INCLUDE_RE);
    if (!match?.groups?.name) continue;
    const entryPath = path.join(baseDir, match.groups.name);
    if (!fs.existsSync(entryPath)) {
      throw new Error(`Missing Pokédex entry referenced by ASM include list: ${entryPath}`);
    }
    entryPaths.push(entryPath);
  }
  if (entryPaths.length === 0) {
    throw new Error(`Could not find any Pokédex entry includes in ${dexEntriesPath}`);
  }
  return entryPaths;
}

function mergeTextLines(lines: string[]): string {
  const parts: string[] = [];
  for (const line of lines) {
    if (!line) continue;
    if (parts.length > 0 && parts[parts.length - 1].endsWith("-")) {
      parts[parts.length - 1] = `${parts[parts.length - 1].slice(0, -1)}${line.trimStart()}`;
    } else {
      parts.push(line);
    }
  }
  return parts.join(" ").trim();
}

function heightToMeters(heightDigits: number): number {
  const feet = Math.floor(heightDigits / 100);
  const inches = heightDigits % 100;
  return Number(((feet * 12 + inches) * INCHES_TO_METERS).toFixed(2));
}

function weightToKg(weightDigits: number): number {
  return Number(((weightDigits / 10) * POUNDS_TO_KG).toFixed(2));
}

export function parsePokedexEntry(filePath: string): PokedexData {
  let classification: string | null = null;
  let heightDigits: number | null = null;
  let weightDigits: number | null = null;
  const textLines: string[] = [];
  let textStarted = false;

  for (const line of iterEntryLines(filePath)) {
    const command = line.split(/\s+/, 1)[0];
    if (command === "dw") {
      const numbers = line.match(/\d+/g) ?? [];
      if (numbers.length < 2) {
        throw new Error("Could not find height/weight in pokedex data");
      }
      const [heightToken, weightToken] = numbers;
      if (!heightToken || !weightToken) {
        throw new Error("Could not find height/weight in pokedex data");
      }
      heightDigits = Number.parseInt(heightToken, 10);
      weightDigits = Number.parseInt(weightToken, 10);
      textStarted = true;
      continue;
    }
    if (classification === null && command === "db") {
      const literal = extractTextLiteral(line);
      if (literal) {
        classification = cleanLine(literal);
      }
      continue;
    }
    if (textStarted && (command === "db" || command === "next" || command === "page")) {
      const literal = extractTextLiteral(line);
      if (literal) {
        textLines.push(cleanLine(literal));
      }
    }
  }

  if (classification === null || heightDigits === null || weightDigits === null) {
    throw new Error("Could not parse pokedex entry");
  }

  return {
    species: path.basename(filePath, ".asm").toUpperCase(),
    classification,
    height: heightToMeters(heightDigits),
    weight: weightToKg(weightDigits),
    text: mergeTextLines(textLines),
  };
}

export function exportPokedex(): PokedexData[] {
  const pokedexData = iterPokedexEntryPaths().map((entryPath) => parsePokedexEntry(entryPath));
  writeJsonToTargets("pokedex.json", pokedexData, { indent: 2 });
  return pokedexData;
}

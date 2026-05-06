import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripAsmComment, writeJsonToTargets } from "./asm-utils";

type FleeMonsData = {
  always: string[];
  often: string[];
  sometimes: string[];
};

type PokedexEntryData = {
  species: string;
  classification: string;
  heightDigits: number;
  weightDigits: number;
  pages: string[];
};

type FrontpicAnimCommand =
  | { kind: "frame"; frame: number; duration: number }
  | { kind: "setrepeat"; count: number }
  | { kind: "dorepeat"; target: number }
  | { kind: "endanim" };

type FrontpicAnimProgram = {
  commands: FrontpicAnimCommand[];
};

const normalizeSpecies = (value: string): string =>
  value.trim().toUpperCase().replace(/[^A-Z0-9]+/g, "_").replace(/^_+|_+$/g, "");

const parseDbSymbolList = (content: string, label: string): string[] => {
  const result: string[] = [];
  let inBlock = false;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (!inBlock) {
      if (line === `${label}:`) {
        inBlock = true;
      }
      continue;
    }
    if (!line) {
      continue;
    }
    if (/^[A-Za-z_][A-Za-z0-9_]*:$/.test(line)) {
      break;
    }
    const match = line.match(/^db\s+(.+)$/);
    if (!match) {
      continue;
    }
    const value = match[1].split(",", 1)[0].trim();
    if (value === "-1") {
      break;
    }
    result.push(value);
  }
  return result;
};

const parseRequiredDbSymbolList = (content: string, label: string, sourcePath: string): string[] => {
  const values = parseDbSymbolList(content, label);
  if (!values.length) {
    throw new Error(`Could not parse required ${label} table from ${sourcePath}.`);
  }
  return values;
};

export const exportFleeMons = (): FleeMonsData => {
  const sourcePath = path.join(getDisassemblyRoot(), "data", "wild", "flee_mons.asm");
  const content = fs.readFileSync(sourcePath, "utf8");
  const payload: FleeMonsData = {
    always: parseRequiredDbSymbolList(content, "AlwaysFleeMons", sourcePath),
    often: parseRequiredDbSymbolList(content, "OftenFleeMons", sourcePath),
    sometimes: parseRequiredDbSymbolList(content, "SometimesFleeMons", sourcePath),
  };
  writeJsonToTargets("flee_mons.json", payload, { indent: 2 });
  return payload;
};

export const exportMarts = (): Record<string, string[]> => {
  const sourcePath = path.join(getDisassemblyRoot(), "data", "items", "marts.asm");
  const content = fs.readFileSync(sourcePath, "utf8");
  const marts: Record<string, string[]> = {};
  let currentMart: string | null = null;
  let expectedCount: number | null = null;

  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    const labelMatch = line.match(/^(Mart[A-Za-z0-9_]+):$/);
    if (labelMatch) {
      currentMart = labelMatch[1];
      marts[currentMart] = [];
      expectedCount = null;
      continue;
    }
    if (!currentMart) {
      continue;
    }
    const dbMatch = line.match(/^db\s+(.+)$/);
    if (!dbMatch) {
      continue;
    }
    const value = dbMatch[1].split(",", 1)[0].trim();
    if (value === "-1") {
      if (expectedCount !== null && marts[currentMart].length !== expectedCount) {
        throw new Error(
          `${currentMart} declared ${expectedCount} mart items but exported ${marts[currentMart].length}.`
        );
      }
      currentMart = null;
      expectedCount = null;
      continue;
    }
    if (expectedCount === null && /^\d+$/.test(value)) {
      expectedCount = Number.parseInt(value, 10);
      continue;
    }
    marts[currentMart].push(value);
  }

  writeJsonToTargets("marts.json", marts, { indent: 2 });
  return marts;
};

export const exportPcStrings = (): Record<string, string> => {
  const sourcePath = path.join(getDisassemblyRoot(), "engine", "pokemon", "bills_pc.asm");
  const content = fs.readFileSync(sourcePath, "utf8");
  const strings: Record<string, string> = {};
  for (const match of content.matchAll(/^(PCString_[A-Za-z0-9_]+):\s+db\s+"([^"]*)@"/gm)) {
    strings[match[1]] = match[2];
  }
  writeJsonToTargets("pc_strings.json", strings, { indent: 2 });
  return strings;
};

export const exportMenuIcons = (): Record<string, string> => {
  const sourcePath = path.join(getDisassemblyRoot(), "data", "pokemon", "menu_icons.asm");
  const content = fs.readFileSync(sourcePath, "utf8");
  const icons: Record<string, string> = {};
  for (const rawLine of content.split(/\r?\n/)) {
    const match = rawLine.trim().match(/^db\s+(ICON_[A-Z0-9_]+)\s*;\s*([A-Z0-9_]+)$/);
    if (!match) {
      continue;
    }
    icons[normalizeSpecies(match[2])] = match[1];
  }
  icons.EGG = "ICON_EGG";
  writeJsonToTargets("menu_icons.json", icons, { indent: 2 });
  return icons;
};

const parseDexEntryFile = (filePath: string): PokedexEntryData | null => {
  const species = normalizeSpecies(path.basename(filePath, ".asm"));
  if (!species || species === "EGG") {
    return null;
  }
  const content = fs.readFileSync(filePath, "utf8");
  const classificationMatch = content.match(/db\s+"([^"]*)@"/);
  const sizeMatch = content.match(/dw\s+(\d+),\s*(\d+)\s*;\s*height,\s*weight/);
  if (!classificationMatch || !sizeMatch) {
    return null;
  }
  const pages: string[] = [];
  let currentPage: string[] = [];
  let inEntryText = false;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (!line) {
      continue;
    }
    if (!inEntryText) {
      if (line.match(/^dw\s+\d+,\s*\d+/)) {
        inEntryText = true;
      }
      continue;
    }
    const textMatch = line.match(/^(db|next|page)\s+"([^"]*)"/);
    if (!textMatch) {
      continue;
    }
    const opcode = textMatch[1];
    if (opcode === "page") {
      if (currentPage.length) {
        pages.push(currentPage.join(" @ ").trim());
      }
      currentPage = [];
    }
    currentPage.push(textMatch[2].replace(/@$/, ""));
  }
  if (currentPage.length) {
    pages.push(currentPage.join(" @ ").trim());
  }
  return {
    species,
    classification: classificationMatch[1],
    heightDigits: Number.parseInt(sizeMatch[1], 10),
    weightDigits: Number.parseInt(sizeMatch[2], 10),
    pages,
  };
};

export const exportPokedexEntries = (): PokedexEntryData[] => {
  const dexEntriesDir = path.join(getDisassemblyRoot(), "data", "pokemon", "dex_entries");
  const entries = fs
    .readdirSync(dexEntriesDir)
    .filter((entry) => entry.endsWith(".asm"))
    .sort()
    .map((entry) => parseDexEntryFile(path.join(dexEntriesDir, entry)))
    .filter((entry): entry is PokedexEntryData => entry !== null);
  writeJsonToTargets("pokedex_entries.json", entries, { indent: 2 });
  return entries;
};

const parseFrontpicAnimNumber = (token: string): number => {
  const cleaned = token.trim();
  if (!cleaned) {
    return 0;
  }
  if (cleaned.startsWith("$")) {
    return Number.parseInt(cleaned.slice(1), 16);
  }
  return Number.parseInt(cleaned, 10);
};

const parseFrontpicAnimScript = (source: string): FrontpicAnimProgram => {
  const commands: FrontpicAnimCommand[] = [];
  for (const rawLine of source.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (!line) {
      continue;
    }
    const parts = line.split(/[\s,]+/).filter(Boolean);
    const opcode = parts[0]?.toLowerCase();
    if (opcode === "frame" && parts.length >= 3) {
      commands.push({
        kind: "frame",
        frame: parseFrontpicAnimNumber(parts[1]),
        duration: parseFrontpicAnimNumber(parts[2]),
      });
      continue;
    }
    if (opcode === "setrepeat" && parts.length >= 2) {
      commands.push({ kind: "setrepeat", count: parseFrontpicAnimNumber(parts[1]) });
      continue;
    }
    if (opcode === "dorepeat" && parts.length >= 2) {
      commands.push({ kind: "dorepeat", target: parseFrontpicAnimNumber(parts[1]) });
      continue;
    }
    if (opcode === "endanim") {
      commands.push({ kind: "endanim" });
    }
  }
  return { commands };
};

export const exportPokemonFrontpicAnimations = (): Record<string, FrontpicAnimProgram> => {
  const pokemonGfxDir = path.join(getDisassemblyRoot(), "gfx", "pokemon");
  const entries: Record<string, FrontpicAnimProgram> = {};
  if (fs.existsSync(pokemonGfxDir)) {
    for (const entry of fs.readdirSync(pokemonGfxDir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      if (!entry.isDirectory()) {
        continue;
      }
      const animPath = path.join(pokemonGfxDir, entry.name, "anim.asm");
      if (!fs.existsSync(animPath)) {
        continue;
      }
      const program = parseFrontpicAnimScript(fs.readFileSync(animPath, "utf8"));
      if (program.commands.length) {
        entries[entry.name.toLowerCase()] = program;
      }
    }
  }
  writeJsonToTargets("pokemon_frontpic_anim.json", entries, { indent: 2 });
  return entries;
};

export const exportRuntimeAssets = (): void => {
  exportFleeMons();
  exportMarts();
  exportPcStrings();
  exportMenuIcons();
  exportPokedexEntries();
  exportPokemonFrontpicAnimations();
};

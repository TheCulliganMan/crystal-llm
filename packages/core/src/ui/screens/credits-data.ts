import fs from "fs";
import path from "path";
import { z } from "zod";
import { getAssetPath, getAssetsRoot, getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";
import { buildDefaultCharMap } from "@pokecrystal/core/ui/text/glyph-map";

export type RGBColor = [number, number, number];
export type Palette = [RGBColor, RGBColor, RGBColor, RGBColor];
export type PaletteSet = [Palette, Palette, Palette];

export type CreditsOp = {
  kind: string;
  value?: string | number | null;
  lineIndex?: number | null;
  byteLength: number;
};

const RGBColorSchema = z.tuple([z.number(), z.number(), z.number()]);
const PaletteSchema = z.tuple([RGBColorSchema, RGBColorSchema, RGBColorSchema, RGBColorSchema]);
const PaletteSetSchema = z.tuple([PaletteSchema, PaletteSchema, PaletteSchema]);

const DISASSEMBLY_ROOT = getDisassemblyRoot();

const stripComment = (line: string): string => line.split(";", 1)[0].trim();

const splitDirective = (line: string): [string, string] => {
  const match = line.match(/^(\S+)\s*(.*)$/);
  return match ? [match[1], match[2] ?? ""] : ["", ""];
};

const findRepoPublicDisassemblyFile = (parts: readonly string[]): string | null => {
  let current = path.resolve(process.cwd());
  for (let depth = 0; depth < 12; depth += 1) {
    const candidate = path.join(current, "apps", "web", "public", "disassembly", ...parts);
    if (fs.existsSync(candidate)) {
      return candidate;
    }
    const parent = path.dirname(current);
    if (parent === current) {
      break;
    }
    current = parent;
  }
  return null;
};

const resolveDisassemblyFile = (...parts: string[]): string => {
  const candidates = [
    path.join(DISASSEMBLY_ROOT, ...parts),
    path.join(getAssetsRoot(), "disassembly", ...parts),
    path.join(getAssetsRoot(), "..", "public", "disassembly", ...parts),
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  const repoPublicFile = findRepoPublicDisassemblyFile(parts);
  if (repoPublicFile) {
    return repoPublicFile;
  }
  return candidates[0];
};

const parseNumericToken = (token: string): number => {
  let normalized = token.trim();
  if (!normalized) {
    throw new Error("Empty numeric token encountered.");
  }
  let sign = 1;
  if (normalized.startsWith("-")) {
    sign = -1;
    normalized = normalized.slice(1);
  } else if (normalized.startsWith("+")) {
    normalized = normalized.slice(1);
  }
  let base = 10;
  if (normalized.startsWith("$")) {
    base = 16;
    normalized = normalized.slice(1);
  } else if (normalized.toLowerCase().startsWith("0x")) {
    base = 16;
    normalized = normalized.slice(2);
  }
  if (!normalized) {
    throw new Error("Numeric token missing digits.");
  }
  return sign * Number.parseInt(normalized, base);
};

const assertByte = (value: number, label: string): void => {
  if (!Number.isInteger(value) || value < 0 || value > 0xff) {
    throw new Error(`${label} must be an 8-bit value, got ${value}.`);
  }
};

export function loadCreditConstantIndices(): Record<string, number> {
  const constantsFile = resolveDisassemblyFile("constants", "credits_constants.asm");
  if (!fs.existsSync(constantsFile)) {
    throw new Error(`Missing credits constants file: ${constantsFile}`);
  }

  const constants: Record<string, number> = {};
  let currentValue = 0;

  for (const rawLine of fs.readFileSync(constantsFile, "utf8").split(/\r?\n/)) {
    const line = stripComment(rawLine);
    if (!line) {
      continue;
    }
    if (line.startsWith("DEF NUM_CREDITS_STRINGS")) {
      break;
    }
    if (line.startsWith("const_def")) {
      currentValue = 0;
      continue;
    }
    if (!line.startsWith("const ")) {
      continue;
    }
    const [, name] = line.split(/\s+/, 2);
    if (!name) {
      continue;
    }
    const trimmed = name.trim();
    if (trimmed in constants) {
      throw new Error(`Duplicate credits constant: ${trimmed}`);
    }
    constants[trimmed] = currentValue;
    currentValue += 1;
  }

  if (!Object.keys(constants).length) {
    throw new Error("Failed to parse credits constants; no entries found.");
  }
  return constants;
}

const parsePointerTable = (lines: readonly string[]): string[] => {
  const labels: string[] = [];
  for (const rawLine of lines) {
    const line = stripComment(rawLine);
    if (!line) {
      continue;
    }
    if (line.startsWith("assert_table_length")) {
      break;
    }
    if (line.startsWith("dw")) {
      const entries = line.slice("dw".length).split(",");
      for (const entry of entries) {
        const normalized = entry.trim();
        if (!normalized) {
          continue;
        }
        labels.push(normalized.replace(/^\./, ""));
      }
    }
  }
  return labels;
};

const parseStringBlocks = (lines: readonly string[]): Record<string, string> => {
  const strings: Record<string, string> = {};
  let currentLabel: string | null = null;
  let buffer: string[] = [];

  const appendDirective = (line: string): void => {
    if (!line.startsWith("db") && !line.startsWith("next")) {
      return;
    }
    const [directive, remainder] = splitDirective(line);
    const text = remainder.trim().replace(/^"|"$/g, "");
    if (directive.toLowerCase() === "next" && buffer.length) {
      buffer.push("\n");
    }
    buffer.push(text);
  };

  for (const rawLine of lines) {
    const line = stripComment(rawLine);
    if (!line) {
      continue;
    }
    if (line.startsWith(".")) {
      if (currentLabel !== null) {
        strings[currentLabel] = buffer.join("").replace(/@/g, "");
      }
      const colonIndex = line.indexOf(":");
      currentLabel = (colonIndex === -1 ? line : line.slice(0, colonIndex)).replace(/^\./, "");
      buffer = [];
      const inlineDirective = colonIndex === -1 ? "" : line.slice(colonIndex + 1).trim();
      if (inlineDirective) {
        appendDirective(inlineDirective);
      }
      continue;
    }
    appendDirective(line);
  }

  if (currentLabel !== null) {
    strings[currentLabel] = buffer.join("").replace(/@/g, "");
  }
  return strings;
};

const encodeTextTiles = (text: string, charMap: Record<string, number>): number[] => {
  const stripped = text.replace(/@/g, "");
  const tiles: number[] = [];
  for (const char of stripped) {
    const tileId = charMap[char];
    if (tileId === undefined) {
      throw new Error(`Credits glyph ${JSON.stringify(char)} is not defined.`);
    }
    tiles.push(tileId);
  }
  return tiles;
};

const parseStringBlocksAsTiles = (lines: readonly string[]): Record<string, number[][]> => {
  const strings: Record<string, number[][]> = {};
  let currentLabel: string | null = null;
  let currentLines: number[][] = [];
  const charMap = buildDefaultCharMap();

  const appendDirective = (line: string): void => {
    if (!line.startsWith("db") && !line.startsWith("next")) {
      return;
    }
    const [directive, remainder] = splitDirective(line);
    const trimmed = remainder.trim();
    if (!trimmed) {
      return;
    }
    if (trimmed === '"@"' || trimmed === "@") {
      return;
    }
    if (directive.toLowerCase() === "next" || currentLines.length === 0) {
      currentLines.push([]);
    }
    const lineTiles = currentLines[currentLines.length - 1];
    if (trimmed.startsWith('"')) {
      const text = trimmed.replace(/^"|"$/g, "");
      lineTiles.push(...encodeTextTiles(text, charMap));
      return;
    }
    for (const token of trimmed.split(",")) {
      const cleaned = token.trim();
      if (!cleaned || cleaned === '"@"' || cleaned === "@") {
        continue;
      }
      const value = parseNumericToken(cleaned);
      if (value < 0 || value > 0xff) {
        throw new Error(`Credits tile index ${value} is outside the 8-bit range.`);
      }
      lineTiles.push(value);
    }
  };

  for (const rawLine of lines) {
    const line = stripComment(rawLine);
    if (!line) {
      continue;
    }
    if (line.startsWith(".")) {
      if (currentLabel !== null) {
        strings[currentLabel] = currentLines;
      }
      const colonIndex = line.indexOf(":");
      currentLabel = (colonIndex === -1 ? line : line.slice(0, colonIndex)).replace(/^\./, "");
      currentLines = [];
      const inlineDirective = colonIndex === -1 ? "" : line.slice(colonIndex + 1).trim();
      if (inlineDirective) {
        appendDirective(inlineDirective);
      }
      continue;
    }
    appendDirective(line);
  }

  if (currentLabel !== null) {
    strings[currentLabel] = currentLines;
  }
  return strings;
};

export function loadCreditsStrings(): string[] {
  const stringsPath = resolveDisassemblyFile("data", "credits_strings.asm");
  if (!fs.existsSync(stringsPath)) {
    throw new Error(`Missing credits strings file: ${stringsPath}`);
  }

  const lines = fs.readFileSync(stringsPath, "utf8").split(/\r?\n/);
  const pointerLabels = parsePointerTable(lines);
  const stringBlocks = parseStringBlocks(lines);

  if (!pointerLabels.length) {
    throw new Error("Credits pointer table is empty.");
  }

  const resolved: string[] = [];
  for (const label of pointerLabels) {
    if (!(label in stringBlocks)) {
      throw new Error(`Missing credits string for label '${label}'.`);
    }
    resolved.push(stringBlocks[label]);
  }
  return resolved;
}

export function loadCreditsStringTiles(): number[][][] {
  const stringsPath = resolveDisassemblyFile("data", "credits_strings.asm");
  if (!fs.existsSync(stringsPath)) {
    throw new Error(`Missing credits strings file: ${stringsPath}`);
  }

  const lines = fs.readFileSync(stringsPath, "utf8").split(/\r?\n/);
  const pointerLabels = parsePointerTable(lines);
  const stringBlocks = parseStringBlocksAsTiles(lines);

  if (!pointerLabels.length) {
    throw new Error("Credits pointer table is empty.");
  }

  const resolved: number[][][] = [];
  for (const label of pointerLabels) {
    const block = stringBlocks[label];
    if (!block) {
      throw new Error(`Missing credits string for label '${label}'.`);
    }
    resolved.push(block);
  }
  return resolved;
}

export function loadCreditsScript(): CreditsOp[] {
  const scriptPath = resolveDisassemblyFile("data", "credits_script.asm");
  if (!fs.existsSync(scriptPath)) {
    throw new Error(`Missing credits script file: ${scriptPath}`);
  }

  const tokens: string[] = [];
  for (const rawLine of fs.readFileSync(scriptPath, "utf8").split(/\r?\n/)) {
    const line = stripComment(rawLine);
    if (!line || !line.startsWith("db")) {
      continue;
    }
    for (const token of line.slice("db".length).split(",")) {
      const cleaned = token.trim();
      if (cleaned) {
        tokens.push(cleaned);
      }
    }
  }

  const ops: CreditsOp[] = [];
  let index = 0;
  while (index < tokens.length) {
    const token = tokens[index];
    if (token === "CREDITS_END") {
      ops.push({ kind: "end", byteLength: 1 });
      index += 1;
    } else if (token === "CREDITS_WAIT") {
      const durationToken = tokens[index + 1];
      if (durationToken === undefined) {
        throw new Error(`CREDITS_WAIT is missing a duration argument at token ${index}`);
      }
      const duration = parseNumericToken(durationToken);
      assertByte(duration, "CREDITS_WAIT duration");
      ops.push({ kind: "wait", value: duration, byteLength: 2, lineIndex: null });
      index += 2;
    } else if (token === "CREDITS_WAIT2") {
      const durationToken = tokens[index + 1];
      if (durationToken === undefined) {
        throw new Error(`CREDITS_WAIT2 is missing a duration argument at token ${index}`);
      }
      const duration = parseNumericToken(durationToken);
      assertByte(duration, "CREDITS_WAIT2 duration");
      ops.push({ kind: "wait2", value: duration, byteLength: 2, lineIndex: null });
      index += 2;
    } else if (token === "CREDITS_SCENE") {
      const sceneToken = tokens[index + 1];
      if (sceneToken === undefined) {
        throw new Error(`CREDITS_SCENE is missing a scene argument at token ${index}`);
      }
      const scene = parseNumericToken(sceneToken);
      assertByte(scene, "CREDITS_SCENE index");
      ops.push({ kind: "scene", value: scene, byteLength: 2 });
      index += 2;
    } else if (token === "CREDITS_CLEAR") {
      ops.push({ kind: "clear", byteLength: 1 });
      index += 1;
    } else if (token === "CREDITS_MUSIC") {
      ops.push({ kind: "music", byteLength: 1 });
      index += 1;
    } else if (token === "CREDITS_THEEND") {
      ops.push({ kind: "theend", byteLength: 1 });
      index += 1;
    } else {
      const stringArgToken = tokens[index + 1];
      if (stringArgToken === undefined) {
        throw new Error(`Truncated credits string entry at token ${token}`);
      }
      const lineIndex = parseNumericToken(stringArgToken);
      assertByte(lineIndex, "string line index");
      ops.push({
        kind: "string",
        value: token,
        lineIndex,
        byteLength: 2,
      });
      index += 2;
    }
  }
  return ops;
}

export function loadCreditsPalettes(): PaletteSet[] {
  const palettePath = getAssetPath("gfx", "credits", "credits.pal");
  if (!fs.existsSync(palettePath)) {
    throw new Error(`Missing credits palette file: ${palettePath}`);
  }

  const palettes: Palette[] = [];
  const lines = fs.readFileSync(palettePath, "utf8").split(/\r?\n/);
  for (const rawLine of lines) {
    const line = stripComment(rawLine);
    if (!line || !line.startsWith("RGB")) {
      continue;
    }
    const components = line
      .slice(3)
      .split(",")
      .map((part) => part.trim())
      .filter(Boolean)
      .map((part) => Number.parseInt(part, 10));
    if (components.length !== 12) {
      throw new Error(`Unexpected palette line: ${rawLine.trim()}`);
    }
    const colours: RGBColor[] = [];
    for (let offset = 0; offset < 12; offset += 3) {
      const [r, g, b] = components.slice(offset, offset + 3);
      colours.push([gbc5To8(r), gbc5To8(g), gbc5To8(b)]);
    }
    palettes.push(PaletteSchema.parse(colours) as Palette);
  }

  if (palettes.length % 3 !== 0) {
    throw new Error("Credits palettes should be grouped in threes per scene.");
  }

  const result: PaletteSet[] = [];
  for (let i = 0; i < palettes.length; i += 3) {
    const group = PaletteSetSchema.parse([
      palettes[i],
      palettes[i + 1],
      palettes[i + 2],
    ]);
    result.push(group);
  }

  if (!result.length) {
    throw new Error("No credits palettes parsed from credits.pal.");
  }
  return result;
}

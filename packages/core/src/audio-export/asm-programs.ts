import * as fs from "fs";
import * as path from "path";

export type AsmAudioProgramKind = "music" | "sfx" | "cry";

export type AsmAudioProgram = {
  cacheKey: string;
  source: string;
};

const LEGACY_SLUG_OVERRIDES = new Map([
  ["NIDORAN_M", "nidoran_m"],
  ["Nidoran_M", "nidoran_m"],
  ["nidoran_m", "nidoran_m"],
  ["NIDORAN_F", "nidoran_f"],
  ["Nidoran_F", "nidoran_f"],
  ["nidoran_f", "nidoran_f"],
  ["UNKNOWN5F", "unused"],
  ["Unknown5F", "unused"],
  ["unknown5f", "unused"],
]);

export const normalizeAsmSlug = (value: string): string =>
  LEGACY_SLUG_OVERRIDES.get(value) ?? value.replace(/[^A-Za-z0-9]/g, "").toLowerCase();

export const normalizeStandaloneLocalLabels = (sourceText: string): string =>
  sourceText.replace(/^(\s*)(\.[A-Za-z0-9_]+)\s*$/gm, "$1$2:");

export function extractAsmProgram(sourceText: string, entryLabel: string): string | null {
  const lines = normalizeStandaloneLocalLabels(sourceText).split(/\r?\n/);
  const labelIndex = new Map<string, number>();
  for (let i = 0; i < lines.length; i += 1) {
    const match = lines[i].match(/^([A-Za-z0-9_.]+):\s*$/);
    if (match) {
      labelIndex.set(match[1], i);
    }
  }

  const readBlock = (label: string): string[] | null => {
    const start = labelIndex.get(label);
    if (start === undefined) {
      return null;
    }
    let end = lines.length;
    let hasCommands = false;
    for (let i = start + 1; i < lines.length; i += 1) {
      if (/^[A-Za-z0-9_]+:\s*$/.test(lines[i])) {
        if (hasCommands) {
          end = i;
          break;
        }
        continue;
      }
      const command = lines[i].split(";", 1)[0].trim();
      if (command && !command.startsWith("assert")) {
        hasCommands = true;
      }
    }
    return lines.slice(start, end);
  };

  const queue = [entryLabel];
  const seen = new Set<string>();
  const blocks: string[] = [];

  while (queue.length > 0) {
    const label = queue.shift();
    if (!label || seen.has(label)) {
      continue;
    }
    seen.add(label);
    const block = readBlock(label);
    if (!block) {
      continue;
    }
    blocks.push(block.join("\n"));
    const blockText = block.join("\n");

    for (const match of blockText.matchAll(/^\s*channel\s+\d+\s*,\s*([A-Za-z0-9_.]+)/gm)) {
      queue.push(match[1]);
    }

    const owner = label.startsWith(".") ? null : label;
    for (const match of blockText.matchAll(/^\s*sound_call\s+([A-Za-z0-9_.]+)/gm)) {
      const raw = match[1];
      const target = raw.startsWith(".") && owner ? `${owner}${raw}` : raw;
      queue.push(target);
      if (target.includes(".")) {
        queue.push(target.split(".", 1)[0]);
      }
    }
  }

  return blocks.length > 0 ? blocks.join("\n\n") : null;
}

function loadAsmCollectionSource(
  root: string,
  collectionFile: string,
  requestStem: string,
): string | null {
  const filePath = path.join(root, collectionFile);
  let sourceText: string;
  try {
    sourceText = normalizeStandaloneLocalLabels(fs.readFileSync(filePath, "utf8"));
  } catch {
    return null;
  }
  const requestedSlug = normalizeAsmSlug(requestStem);
  const labels = Array.from(sourceText.matchAll(/^([A-Za-z0-9_]+):\s*$/gm))
    .map((match) => match[1]);
  const entryLabel = labels.find((label) =>
    normalizeAsmSlug(label.replace(/^(Sfx|Cry)_?/, "")) === requestedSlug,
  );
  if (!entryLabel) {
    return null;
  }
  return extractAsmProgram(sourceText, entryLabel);
}

function loadMusicProgram(root: string, requestStem: string): string | null {
  const musicPath = path.join(root, "music", `${requestStem}.asm`);
  let source: string;
  try {
    source = normalizeStandaloneLocalLabels(fs.readFileSync(musicPath, "utf8"));
  } catch {
    return null;
  }

  const musicRoot = path.join(root, "music");
  const sourcesByLabel = new Map<string, string>();
  for (const fileName of fs.readdirSync(musicRoot).filter((entry) => entry.endsWith(".asm")).sort()) {
    const filePath = path.join(musicRoot, fileName);
    const text = normalizeStandaloneLocalLabels(fs.readFileSync(filePath, "utf8"));
    for (const match of text.matchAll(/^([A-Za-z_][A-Za-z0-9_]*):\s*$/gm)) {
      sourcesByLabel.set(match[1], text);
    }
  }

  const blocks = [source];
  const includedLabels = new Set<string>();
  for (let index = 0; index < blocks.length; index += 1) {
    for (const match of blocks[index].matchAll(/^\s*sound_(?:call|jump)\s+([A-Za-z_][A-Za-z0-9_]*)\b/gm)) {
      const label = match[1];
      if (includedLabels.has(label)) {
        continue;
      }
      includedLabels.add(label);
      const referencedSource = sourcesByLabel.get(label);
      if (!referencedSource || referencedSource === source) {
        continue;
      }
      const extracted = extractAsmProgram(referencedSource, label);
      if (extracted) {
        blocks.push(extracted);
      }
    }
  }
  return blocks.join("\n\n");
}

export function buildAsmAudioProgram(
  root: string,
  kind: AsmAudioProgramKind,
  stem: string,
): AsmAudioProgram | null {
  const normalizedStem = normalizeAsmSlug(stem);
  if (!normalizedStem) {
    return null;
  }

  if (kind === "music") {
    const source = loadMusicProgram(root, normalizedStem);
    if (!source) {
      return null;
    }
    return {
      cacheKey: `music:${path.join(root, "music", `${normalizedStem}.asm`)}`,
      source,
    };
  }

  if (kind === "sfx") {
    const source = loadAsmCollectionSource(root, "sfx.asm", normalizedStem)
      ?? loadAsmCollectionSource(root, "sfx_crystal.asm", normalizedStem);
    return source ? { cacheKey: `sfx:${normalizedStem}`, source } : null;
  }

  const source = loadAsmCollectionSource(root, "cries.asm", normalizedStem);
  return source ? { cacheKey: `cry:${normalizedStem}`, source } : null;
}

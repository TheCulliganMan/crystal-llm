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
    for (let i = start + 1; i < lines.length; i += 1) {
      if (/^[A-Za-z0-9_]+:\s*$/.test(lines[i])) {
        end = i;
        break;
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
      queue.push(raw.startsWith(".") && owner ? `${owner}${raw}` : raw);
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
  const entryLabel = labels.find((label) => normalizeAsmSlug(label.replace(/^(Sfx|Cry)_/, "")) === requestedSlug);
  if (!entryLabel) {
    return null;
  }
  return extractAsmProgram(sourceText, entryLabel);
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
    const musicPath = path.join(root, "music", `${normalizedStem}.asm`);
    try {
      const source = normalizeStandaloneLocalLabels(fs.readFileSync(musicPath, "utf8"));
      return {
        cacheKey: `music:${musicPath}`,
        source,
      };
    } catch {
      return null;
    }
  }

  if (kind === "sfx") {
    const source = loadAsmCollectionSource(root, "sfx.asm", normalizedStem)
      ?? loadAsmCollectionSource(root, "sfx_crystal.asm", normalizedStem);
    return source ? { cacheKey: `sfx:${normalizedStem}`, source } : null;
  }

  const source = loadAsmCollectionSource(root, "cries.asm", normalizedStem);
  return source ? { cacheKey: `cry:${normalizedStem}`, source } : null;
}

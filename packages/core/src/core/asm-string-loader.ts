/**
 * Parse inline db strings for getstring targets that are not in JSON assets.
 */
import { assetExists, listAssetDir } from "./asset-manifest";
import { readJsonAssetSync, readTextAssetSync } from "./asset-reader";
import { joinPath } from "./path-utils";
import { getDataDir, getDisassemblyRoot } from "./paths";

const STRING_DIRS = ["engine/phone/scripts", "engine/events"];
const STRING_JSON_FILENAME = "asm_strings.json";

const stripComment = (line: string): string => line.split(";", 1)[0].trim();

const extractQuotedSegments = (value: string): string[] => {
  const segments: string[] = [];
  const regex = /"([^"]*)"/g;
  let match: RegExpExecArray | null = null;
  while ((match = regex.exec(value))) {
    segments.push(match[1]);
  }
  return segments;
};

const parseDbLine = (line: string): { text: string; terminated: boolean } | null => {
  const trimmed = line.trim();
  if (!trimmed.startsWith("db ")) {
    return null;
  }
  const payload = trimmed.slice(3).trim();
  if (!payload) {
    return null;
  }
  const segments = extractQuotedSegments(payload);
  if (!segments.length) {
    return null;
  }
  let buffer = "";
  for (const segment of segments) {
    const terminated = segment.includes("@");
    buffer += segment.replace(/@+$/g, "");
    if (terminated) {
      return { text: buffer, terminated: true };
    }
  }
  return { text: buffer, terminated: false };
};

const parseAsmStrings = (filePath: string): Record<string, string> => {
  const results: Record<string, string> = {};
  let label: string | null = null;
  let buffer = "";

  const flush = () => {
    if (!label) {
      buffer = "";
      return;
    }
    if (buffer) {
      results[label] = buffer;
    }
    buffer = "";
  };

  const content = readTextAssetSync(filePath);
  for (const raw of content.split(/\r?\n/)) {
    const cleaned = stripComment(raw);
    if (!cleaned) {
      continue;
    }
    const labelMatch = /^([A-Za-z0-9_\.]+):\s*(.*)$/.exec(cleaned);
    if (labelMatch) {
      flush();
      label = labelMatch[1];
      const remainder = labelMatch[2]?.trim();
      if (remainder) {
        const parsed = parseDbLine(remainder);
        if (parsed) {
          buffer += parsed.text;
          if (parsed.terminated) {
            results[label] = buffer;
            buffer = "";
            label = null;
          }
        }
      }
      continue;
    }
    if (!label) {
      continue;
    }
    const parsed = parseDbLine(cleaned);
    if (!parsed) {
      continue;
    }
    buffer += parsed.text;
    if (parsed.terminated) {
      results[label] = buffer;
      buffer = "";
      label = null;
    }
  }
  flush();
  return results;
};

export class AsmStringLoader {
  private root: string;
  private dataRoot: string;
  private cache: Record<string, string> | null = null;

  constructor(disassemblyRoot?: string, dataRoot?: string) {
    this.root = disassemblyRoot || getDisassemblyRoot();
    this.dataRoot = dataRoot || getDataDir();
  }

  public get(label: string): string {
    if (!label) {
      return "";
    }
    const cache = this.cache ?? this.load();
    return cache[label] || "";
  }

  private load(): Record<string, string> {
    const cache: Record<string, string> = {};
    const jsonPath = joinPath(this.dataRoot, STRING_JSON_FILENAME);
    try {
      const payload = readJsonAssetSync<Record<string, string>>(jsonPath);
      this.cache = payload;
      return payload;
    } catch {
      // Fall back to parsing ASM files when JSON is unavailable.
    }
    for (const relative of STRING_DIRS) {
      const base = joinPath(this.root, relative);
      if (!assetExists(base)) {
        continue;
      }
      const files = listAssetDir(base)
        .filter((file) => file.endsWith(".asm"))
        .sort();
      for (const file of files) {
        const filePath = joinPath(base, file);
        const entries = parseAsmStrings(filePath);
        Object.assign(cache, entries);
      }
    }
    this.cache = cache;
    return cache;
  }
}

export const asmStringLoader = new AsmStringLoader();

/**
 * Parse dialogue text directly from the disassembly for faithful rendering.
 */
import { assetExists, listAssetDir } from "./asset-manifest";
import { readJsonAssetSync, readTextAssetSync } from "./asset-reader";
import { joinPath } from "./path-utils";
import { getDataDir, getDisassemblyRoot } from "./paths";

const TEXT_DIRS = ["data/text", "data/phone/text"];
const TEXT_JSON_FILENAME = "asm_text.json";

const SILENT_TOKENS = new Set([
  "INCLUDE",
  "SECTION",
  "db",
  "dw",
  "else",
  "endc",
  "if",
  "sound_caught_mon",
  "sound_dex_fanfare_50_79",
  "sound_dex_fanfare_80_109",
  "sound_item",
  "sound_slot_machine_start",
  "text_low",
]);

const TERMINATORS = new Set(["prompt", "text_promptbutton", "text_end", "done"]);
const LINE_BREAK_TOKENS = new Set(["line", "cont", "next"]);

function extractString(argument: string): string {
  if (!argument) {
    return "";
  }
  if (argument.includes('"')) {
    const start = argument.indexOf('"');
    const end = argument.lastIndexOf('"');
    if (start !== -1 && end > start) {
      const raw = argument.substring(start + 1, end);
      return raw.replace(/@+$/g, "");
    }
  }
  return argument.trim().replace(/@+$/g, "");
}

function parseTextFile(filePath: string): Record<string, string> {
  const results: Record<string, string> = {};
  let label: string | null = null;
  const buffer: string[] = [];

  const flush = () => {
    if (label === null) {
      buffer.length = 0;
      return;
    }
    const chunk = buffer.join("").trim();
    buffer.length = 0;
    if (!chunk) {
      return;
    }
    const existing = results[label];
    if (existing) {
      results[label] = `${existing}\n\n${chunk}`;
    } else {
      results[label] = chunk;
    }
  };

  const content = readTextAssetSync(filePath);
  for (const raw of content.split("\n")) {
    const line = raw.split(";", 1)[0].trimEnd();
    const stripped = line.trim();
    if (!stripped) {
      continue;
    }

    if (stripped.endsWith(":")) {
      flush();
      label = stripped.replace(/:+$/, "");
      continue;
    }

    if (label === null) {
      continue;
    }

    const firstSpaceIndex = stripped.search(/\s/);
    const token = firstSpaceIndex === -1 ? stripped : stripped.substring(0, firstSpaceIndex);
    const argument = firstSpaceIndex === -1 ? "" : stripped.substring(firstSpaceIndex).trim();

    if (SILENT_TOKENS.has(token)) {
      continue;
    }
    if (TERMINATORS.has(token)) {
      flush();
      continue;
    }
    if (token === "text") {
      buffer.push(extractString(argument));
      continue;
    }
    if (LINE_BREAK_TOKENS.has(token)) {
      buffer.push("\n");
      buffer.push(extractString(argument));
      continue;
    }
    if (token === "para") {
      buffer.push("\n\n");
      buffer.push(extractString(argument));
      continue;
    }
    if (token === "text_start") {
      continue;
    }
    if (token === "text_ram") {
      const match = /wStringBuffer(\d+)/i.exec(argument);
      if (match) {
        buffer.push(`<STRING_BUFFER_${match[1]}>`);
      } else {
        buffer.push(`<RAM:${argument.trim()}>`);
      }
      continue;
    }
    if (token === "text_decimal") {
      buffer.push(`<DECIMAL:${argument.trim()}>`);
      continue;
    }
    if (token === "text_today") {
      buffer.push("<TODAY>");
      continue;
    }
    if (token === "text_pause") {
      buffer.push("…");
      continue;
    }
  }
  flush();
  return results;
}

export class AsmTextLoader {
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
    const jsonPath = joinPath(this.dataRoot, TEXT_JSON_FILENAME);
    try {
      const payload = readJsonAssetSync<Record<string, string>>(jsonPath);
      this.cache = payload;
      return payload;
    } catch {
      // Fall back to parsing ASM files when JSON is unavailable.
    }
    for (const relative of TEXT_DIRS) {
      const base = joinPath(this.root, relative);
      if (!assetExists(base)) {
        continue;
      }
      const files = listAssetDir(base)
        .filter(file => file.endsWith(".asm"))
        .sort();

      for (const file of files) {
        const filePath = joinPath(base, file);
        const newEntries = parseTextFile(filePath);
        Object.assign(cache, newEntries);
      }
    }
    this.cache = cache;
    return cache;
  }
}

export const asmTextLoader = new AsmTextLoader();

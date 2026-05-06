import fs from "fs";
import path from "path";
import { getAssetsRoot } from "@pokecrystal/core/core/paths";

export function stripInlineComment(value: string): string {
  let inQuotes = false;
  for (let index = 0; index < value.length; index += 1) {
    const char = value[index];
    if (char === '"') {
      inQuotes = !inQuotes;
      continue;
    }
    if (char === ";" && !inQuotes) {
      return value.slice(0, index);
    }
  }
  return value;
}

export function stripAsmComment(line: string): string {
  return stripInlineComment(line).trim();
}

export function splitAsmArgs(args: string): string[] {
  const stripped = stripInlineComment(args).trim();
  if (!stripped) {
    return [];
  }
  if (stripped.startsWith('"') && stripped.endsWith('"')) {
    return [stripped];
  }
  if (stripped.includes(",")) {
    return stripped
      .split(",")
      .map((part) => part.trim())
      .filter(Boolean)
      .map((part) => part.replace(/,$/, ""));
  }
  return stripped
    .split(/\s+/)
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => part.replace(/,$/, ""));
}

export function parseAsmNumber(value: string): number {
  const trimmed = value.trim();
  if (!trimmed) {
    return 0;
  }
  const sign = trimmed.startsWith("-") ? -1 : 1;
  const raw = trimmed.replace(/^[+-]/, "");
  if (raw.startsWith("$")) {
    return sign * Number.parseInt(raw.slice(1), 16);
  }
  if (raw.startsWith("%")) {
    return sign * Number.parseInt(raw.slice(1), 2);
  }
  return sign * Number.parseInt(raw, 10);
}

export function toSnakeCase(name: string): string {
  const step1 = name.replace(/(.)([A-Z][a-z]+)/g, "$1_$2");
  return step1.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
}

export function getTypeScriptDataDir(): string {
  return path.join(getAssetsRoot(), "data");
}

export function ensureDir(dirPath: string): void {
  fs.mkdirSync(dirPath, { recursive: true });
}

export function writeJsonToTargets(
  relativePath: string,
  payload: unknown,
  options?: { indent?: number }
): void {
  const indent = options?.indent ?? 2;
  const contents = `${JSON.stringify(payload, null, indent)}\n`;
  const targetPath = path.join(getTypeScriptDataDir(), relativePath);

  ensureDir(path.dirname(targetPath));
  fs.writeFileSync(targetPath, contents, "utf8");
}

export function removeMatchingOutputs(relativeDir: string, extension = ".json"): void {
  const dirPath = path.join(getTypeScriptDataDir(), relativeDir);
  if (!fs.existsSync(dirPath)) {
    return;
  }
  const removeFromDir = (currentDir: string): void => {
    for (const entry of fs.readdirSync(currentDir, { withFileTypes: true })) {
      const entryPath = path.join(currentDir, entry.name);
      if (entry.isDirectory()) {
        removeFromDir(entryPath);
        continue;
      }
      if (entry.isFile() && entry.name.endsWith(extension)) {
        fs.unlinkSync(entryPath);
      }
    }
  };
  removeFromDir(dirPath);
}

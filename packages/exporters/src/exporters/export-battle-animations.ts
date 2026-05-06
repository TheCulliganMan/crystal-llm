import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripAsmComment, writeJsonToTargets } from "./asm-utils";

export function exportBattleAnimations(): Record<string, string[]> {
  const sourcePath = path.join(getDisassemblyRoot(), "data", "moves", "animations.asm");
  const animations: Record<string, string[]> = {};
  let currentName: string | null = null;
  let currentLines: string[] = [];

  const flush = (): void => {
    if (currentName && currentLines.length) {
      animations[currentName] = currentLines;
    }
    currentName = null;
    currentLines = [];
  };

  for (const rawLine of fs.readFileSync(sourcePath, "utf8").split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    const labelMatch = /^(BattleAnim_[A-Za-z0-9_]+):$/.exec(line);
    if (labelMatch) {
      flush();
      currentName = labelMatch[1];
      currentLines = [];
      continue;
    }
    if (!currentName || !line) {
      continue;
    }
    currentLines.push(line);
  }
  flush();

  if (!Object.keys(animations).length) {
    throw new Error(`Could not parse battle animation scripts from ${sourcePath}`);
  }

  writeJsonToTargets("animations.json", animations);
  return animations;
}

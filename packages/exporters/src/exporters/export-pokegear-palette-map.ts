import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { splitAsmArgs, stripAsmComment, writeJsonToTargets } from "./asm-utils";

const PALETTE_TOKENS = new Set(["BORDER", "EARTH", "MOUNTAIN", "CITY", "POI", "POI_MTN"]);

export function exportPokegearPaletteMap(): Record<string, string[]> {
  const sourcePath = path.join(
    getDisassemblyRoot(),
    "gfx",
    "pokegear",
    "town_map_palette_map.asm"
  );
  const payload: Record<string, string[]> = {};
  let currentKey: string | null = null;

  for (const rawLine of fs.readFileSync(sourcePath, "utf8").split(/\r?\n/)) {
    const commentMatch = /^\s*;\s*gfx\/pokegear\/([^.\s]+)\.png/.exec(rawLine);
    if (commentMatch) {
      currentKey = commentMatch[1];
      payload[currentKey] = [];
      continue;
    }

    const line = stripAsmComment(rawLine);
    const match = /^townmappals\s+(.+)$/.exec(line);
    if (!match || !currentKey) {
      continue;
    }

    for (const token of splitAsmArgs(match[1])) {
      const normalized = token.toUpperCase();
      if (!PALETTE_TOKENS.has(normalized)) {
        throw new Error(`Unknown Pokégear town map palette token '${token}' in ${sourcePath}`);
      }
      payload[currentKey].push(normalized);
    }
  }

  if (!payload.town_map?.length || !payload.pokegear?.length) {
    throw new Error(`Could not parse Pokégear town map palette map from ${sourcePath}`);
  }

  writeJsonToTargets("pokegear_town_map_palette_map.json", payload);
  return payload;
}

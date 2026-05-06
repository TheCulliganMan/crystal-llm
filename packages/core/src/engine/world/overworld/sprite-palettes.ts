import path from "path";
import { readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir } from "@pokecrystal/core/core/paths";

let spritePaletteDefaults: Map<string, number> | null = null;
const SPRITE_PALETTE_DEFAULTS_JSON_PATH = path.join(getDataDir(), "sprite_palette_defaults.json");

const loadSpritePaletteDefaults = (): Map<string, number> => {
  if (spritePaletteDefaults) {
    return spritePaletteDefaults;
  }
  let bundled: Record<string, number>;
  try {
    bundled = readJsonAssetSync<Record<string, number>>(SPRITE_PALETTE_DEFAULTS_JSON_PATH);
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    throw new Error(
      `Missing bundled sprite palette defaults at ${SPRITE_PALETTE_DEFAULTS_JSON_PATH}: ${reason}`
    );
  }
  const bundledMapping = new Map<string, number>();
  for (const [spriteConstant, paletteId] of Object.entries(bundled)) {
    if (typeof paletteId !== "number" || Number.isNaN(paletteId)) {
      continue;
    }
    bundledMapping.set(spriteConstant.toUpperCase(), paletteId);
  }
  if (!bundledMapping.size) {
    throw new Error(`Missing bundled sprite palette defaults at ${SPRITE_PALETTE_DEFAULTS_JSON_PATH}.`);
  }
  spritePaletteDefaults = bundledMapping;
  return bundledMapping;
};

export const resolveNpcPaletteId = (
  spriteConstant: string,
  paletteOverride?: number | null
): number => {
  const override = paletteOverride ?? 0;
  if (override !== 0) {
    return override;
  }
  const key = spriteConstant.trim().toUpperCase();
  const defaults = loadSpritePaletteDefaults();
  return defaults.get(key) ?? 0;
};

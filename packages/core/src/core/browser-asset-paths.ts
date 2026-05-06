import { normalizePath } from "./path-utils";

type BundledBrowserAssetPathOptions = {
  assetBase?: string;
  disassemblyBase?: string;
};

const LEGACY_DISASSEMBLY_BASE = "/pokecrystal_disassembly";

const normalizeBase = (value: string): string => {
  const normalized = normalizePath(value);
  if (normalized.length > 1) {
    return normalized.replace(/\/+$/, "");
  }
  return normalized;
};

export const canonicalizeBundledBrowserAssetPath = (
  filePath: string,
  options: BundledBrowserAssetPathOptions = {},
): string => {
  const assetBase = normalizeBase(options.assetBase ?? "/assets");
  const disassemblyBase = normalizeBase(options.disassemblyBase ?? "/disassembly");
  const normalized = normalizeBase(filePath);
  const disassemblyPrefixes = [`${disassemblyBase}/gfx/`, `${LEGACY_DISASSEMBLY_BASE}/gfx/`];

  for (const prefix of disassemblyPrefixes) {
    if (normalized.startsWith(prefix)) {
      return `${assetBase}/gfx/${normalized.slice(prefix.length)}`;
    }
  }

  return normalized;
};

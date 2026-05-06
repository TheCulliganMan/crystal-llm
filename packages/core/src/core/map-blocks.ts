import fs from "fs";
import { loadMergedMapBlocksSync } from "./content-packs";
import { joinPath } from "./path-utils";
import { getDataDir, getDisassemblyRoot } from "./paths";

const MAP_BLOCKS_JSON_PATH = joinPath(getDataDir(), "map_blocks.json");
const MAP_BLOCKS_ASM_PATH = (): string => joinPath(getDisassemblyRoot(), "data", "maps", "blocks.asm");
let cachedBundledBlocks: Map<string, Buffer> | null = null;
let cachedBlockPaths: Map<string, string> | null = null;

const missingMapBlocksAssetError = (): Error =>
  new Error(
    `Bundled map block asset is required for the asset-only runtime: missing or invalid ${MAP_BLOCKS_JSON_PATH}.`
  );

const readBundledMapBlocksPayload = (): Record<string, string> => {
  try {
    const payload = loadMergedMapBlocksSync();
    if (!payload || typeof payload !== "object" || !Object.keys(payload).length) {
      throw new Error("empty payload");
    }
    return payload;
  } catch {
    throw missingMapBlocksAssetError();
  }
};

const loadBundledBlocks = (): Map<string, Buffer> => {
  if (cachedBundledBlocks) {
    return cachedBundledBlocks;
  }
  cachedBundledBlocks = new Map();
  const payload = readBundledMapBlocksPayload();
  for (const [label, encoded] of Object.entries(payload)) {
    if (typeof encoded !== "string" || !encoded.length) {
      continue;
    }
    cachedBundledBlocks.set(label, Buffer.from(encoded, "base64"));
  }
  return cachedBundledBlocks;
};

const loadBlockPaths = (): Map<string, string> => {
  if (cachedBlockPaths) {
    return cachedBlockPaths;
  }
  cachedBlockPaths = new Map();
  const asmPath = MAP_BLOCKS_ASM_PATH();
  const lines = fs.existsSync(asmPath) ? fs.readFileSync(asmPath, "utf8").split(/\r?\n/) : [];
  const pendingLabels: string[] = [];
  for (const rawLine of lines) {
    const line = rawLine.split(";")[0].trim();
    if (!line) {
      continue;
    }
    const labelMatch = /^([A-Za-z0-9_]+):$/.exec(line);
    if (labelMatch) {
      pendingLabels.push(labelMatch[1]);
      continue;
    }
    const incbinMatch = /^INCBIN\s+"([^"]+)"/i.exec(line);
    if (!incbinMatch || pendingLabels.length === 0) {
      continue;
    }
    const blockPath = joinPath(getDisassemblyRoot(), incbinMatch[1]);
    for (const label of pendingLabels.splice(0)) {
      cachedBlockPaths.set(label, blockPath);
    }
  }
  return cachedBlockPaths;
};

export const resolveMapBlockPath = (
  mapName: string,
  blocksLabel?: string | null
): string => {
  const label =
    blocksLabel && blocksLabel.trim()
      ? blocksLabel.trim()
      : `${mapName}_Blocks`;
  const blockPath = loadBlockPaths().get(label);
  if (blockPath) {
    return blockPath;
  }
  throw new Error(`Missing map block path for ${label} in ${MAP_BLOCKS_ASM_PATH()}.`);
};

export const readMapBlockBytes = (
  mapName: string,
  blocksLabel?: string | null
): Buffer => {
  const label =
    blocksLabel && blocksLabel.trim()
      ? blocksLabel.trim()
      : `${mapName}_Blocks`;
  const bundled = loadBundledBlocks().get(label);
  if (bundled) {
    return bundled;
  }
  throw new Error(
    `Bundled map block entry is required for the asset-only runtime: missing ${label} in ${MAP_BLOCKS_JSON_PATH}.`
  );
};

export const readMapBlockBytesAsync = async (
  mapName: string,
  blocksLabel?: string | null
): Promise<Buffer> => {
  const label =
    blocksLabel && blocksLabel.trim()
      ? blocksLabel.trim()
      : `${mapName}_Blocks`;
  const bundled = loadBundledBlocks().get(label);
  if (bundled) {
    return bundled;
  }
  throw new Error(
    `Bundled map block entry is required for the asset-only runtime: missing ${label} in ${MAP_BLOCKS_JSON_PATH}.`
  );
};

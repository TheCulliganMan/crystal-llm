#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");
const { exportRuntimeAssets } = require("./export-runtime-fallbacks");
const { syncAssetManifest } = require("./sync-asset-manifest");
const { syncContentPackIndex } = require("./sync-content-pack-index");
const { PNG } = require("pngjs");

const TILE_SIZE = 8;
const UNOWN_PUZZLE_FILES = [
  "aerodactyl",
  "cursor",
  "hooh",
  "kabuto",
  "omanyte",
  "start_cancel",
  "tile_borders",
];

const readPngLevels = (filePath) => {
  const image = PNG.sync.read(fs.readFileSync(filePath));
  if (image.width % TILE_SIZE !== 0 || image.height % TILE_SIZE !== 0) {
    throw new Error(`PNG tileset ${filePath} must align to ${TILE_SIZE}x${TILE_SIZE} tiles.`);
  }
  const tiles = [];
  const tilesWide = image.width / TILE_SIZE;
  const tilesHigh = image.height / TILE_SIZE;
  for (let tileY = 0; tileY < tilesHigh; tileY += 1) {
    for (let tileX = 0; tileX < tilesWide; tileX += 1) {
      const levels = [];
      for (let y = 0; y < TILE_SIZE; y += 1) {
        for (let x = 0; x < TILE_SIZE; x += 1) {
          const sourceX = tileX * TILE_SIZE + x;
          const sourceY = tileY * TILE_SIZE + y;
          const offset = (sourceY * image.width + sourceX) * 4;
          const r = image.data[offset] ?? 0;
          const g = image.data[offset + 1] ?? 0;
          const b = image.data[offset + 2] ?? 0;
          const a = image.data[offset + 3] ?? 255;
          if (a === 0) {
            levels.push(0);
            continue;
          }
          const gray = Math.round((r + g + b) / 3);
          levels.push(Math.round(((255 - gray) / 255) * 3));
        }
      }
      tiles.push(levels);
    }
  }
  return tiles;
};

const encode2bpp = (levelsByTile) => {
  const bytes = [];
  for (const levels of levelsByTile) {
    for (let y = 0; y < TILE_SIZE; y += 1) {
      let lo = 0;
      let hi = 0;
      for (let x = 0; x < TILE_SIZE; x += 1) {
        const level = levels[y * TILE_SIZE + x] ?? 0;
        const mask = 1 << (7 - x);
        if (level & 1) {
          lo |= mask;
        }
        if (level & 2) {
          hi |= mask;
        }
      }
      bytes.push(lo, hi);
    }
  }
  return Buffer.from(bytes);
};

const syncUnownPuzzleAssets = ({ projectRoot, disassemblySource, assetsSource }) => {
  const targetGfxDir = path.join(assetsSource, "gfx", "unown_puzzle");
  const sourceGfxDir = path.join(disassemblySource, "gfx", "unown_puzzle");
  fs.mkdirSync(targetGfxDir, { recursive: true });
  for (const stem of UNOWN_PUZZLE_FILES) {
    const sourcePng = path.join(sourceGfxDir, `${stem}.png`);
    const targetPng = path.join(targetGfxDir, `${stem}.png`);
    if (fs.existsSync(sourcePng)) {
      fs.copyFileSync(sourcePng, targetPng);
    }
    if (fs.existsSync(targetPng)) {
      fs.writeFileSync(
        path.join(targetGfxDir, `${stem}.2bpp`),
        encode2bpp(readPngLevels(targetPng))
      );
    }
  }

  const sourceDataDir = path.resolve(
    __dirname,
    "..",
    "..",
    "..",
    "packages",
    "assets",
    "src",
    "content",
    "data",
    "unown-puzzles"
  );
  const targetDataDir = path.join(assetsSource, "data", "unown_puzzles");
  fs.mkdirSync(targetDataDir, { recursive: true });
  for (const name of ["coordinates.json", "layouts.json"]) {
    const sourcePath = path.join(sourceDataDir, name);
    if (fs.existsSync(sourcePath)) {
      fs.copyFileSync(sourcePath, path.join(targetDataDir, name));
    }
  }
};

const syncPokemonCriesAsset = ({ disassemblySource, assetsSource }) => {
  const sourcePath = path.join(disassemblySource, "data", "pokemon", "cries.asm");
  if (!fs.existsSync(sourcePath)) {
    return;
  }

  const cries = {};
  const source = fs.readFileSync(sourcePath, "utf8");
  for (const line of source.split(/\r?\n/)) {
    const match = line.match(/^\s*mon_cry\s+([A-Z0-9_]+)\s*,\s*([0-9]+)\s*,\s*([0-9]+)\s*;\s*([A-Z0-9_]+)/);
    if (!match) {
      continue;
    }
    const [, cry, pitch, length, species] = match;
    cries[species] = {
      cry,
      pitch: Number(pitch),
      length: Number(length),
    };
  }

  if (Object.keys(cries).length === 0) {
    return;
  }

  const targetPath = path.join(assetsSource, "data", "pokemon_cries.json");
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.writeFileSync(targetPath, `${JSON.stringify(cries, null, 2)}\n`);
};

const preparePublic = ({
  projectRoot = path.resolve(__dirname, ".."),
  disassemblySource = process.env.POKECRYSTAL_DISASSEMBLY_ROOT
    ? path.resolve(process.env.POKECRYSTAL_DISASSEMBLY_ROOT)
    : path.resolve(projectRoot, "..", "..", "vendor", "pokecrystal"),
  assetsSource = path.join(projectRoot, "assets"),
} = {}) => {
  const publicAssetsMirror = path.join(projectRoot, "public", "assets");
  const requiredRuntimeExportInput = path.join(disassemblySource, "data", "maps", "maps.asm");

  // The canonical browser asset source is apps/web/assets. A mirrored
  // public/assets tree only creates ambiguity during development.
  if (fs.existsSync(publicAssetsMirror)) {
    fs.rmSync(publicAssetsMirror, { recursive: true, force: true });
  }

  if (fs.existsSync(requiredRuntimeExportInput)) {
    exportRuntimeAssets({
      projectRoot,
      disassemblyRoot: disassemblySource,
      outDir: path.join(assetsSource, "data"),
    });
  } else {
    console.log(
      `[runtime-assets] Export inputs not found at ${requiredRuntimeExportInput}; using committed asset-only runtime data.`
    );
  }

  syncUnownPuzzleAssets({ projectRoot, disassemblySource, assetsSource });
  syncPokemonCriesAsset({ disassemblySource, assetsSource });

  // Build a deterministic content-pack file index so runtime loaders can
  // discover drop-in extension files in both Node and browser environments.
  try {
    syncContentPackIndex({ projectRoot, assetsSource });
  } catch (error) {
    console.warn("[content-packs] failed to sync index:", error);
  }

  const manifest = syncAssetManifest({ projectRoot, assetsSource });
  console.log(
    `[asset-manifest] indexed ${manifest.files} files and ${manifest.directories} directories`
  );

  return { skipped: false, mode: "assets-only" };
};

if (require.main === module) {
  preparePublic();
}

module.exports = { preparePublic };

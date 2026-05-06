const fs = require("node:fs");
const path = require("node:path");

const requiredAssets = [
  "assets/data/battle_animation_table.json",
  "assets/data/battle_anim_bundle.json",
  "assets/data/asm_text.json",
  "assets/data/move_names.json",
  "assets/data/phone_contacts.json",
  "assets/data/permanent_phone_numbers.json",
  "assets/data/initialize_events.json",
  "assets/data/pokemon_cries.json",
  "assets/data/runtime_map_metadata.json",
  "assets/data/runtime_spawn_points.json",
  "assets/data/map_blocks.json",
  "assets/data/sprite_palette_defaults.json",
  "assets/data/unown_puzzles/coordinates.json",
  "assets/data/unown_puzzles/layouts.json",
  "assets/data/collision/collision_permissions.json",
  "assets/data/collision/collision_stdscripts.json",
  "assets/data/content-packs/core-modular.compiled.json",
  "assets/data/sprite_anim_bundle.json",
  "assets/data/tilesets/johto.json",
  "assets/data/tilesets/johto_metatiles.bin",
  "assets/data/tilesets/johto_palette_map.json",
  "assets/data/tilesets/players_room.json",
  "assets/data/tilesets/players_room_metatiles.bin",
  "assets/data/tilesets/players_room_palette_map.json",
  "assets/gfx/unown_puzzle/aerodactyl.2bpp",
  "assets/gfx/unown_puzzle/cursor.2bpp",
  "assets/gfx/unown_puzzle/hooh.2bpp",
  "assets/gfx/unown_puzzle/kabuto.2bpp",
  "assets/gfx/unown_puzzle/omanyte.2bpp",
  "assets/gfx/unown_puzzle/start_cancel.2bpp",
  "assets/gfx/unown_puzzle/tile_borders.2bpp",
];

const MANIFEST_PATHS = [
  path.resolve(__dirname, "..", "assets.manifest.json"),
  path.resolve(__dirname, "..", "..", "..", "packages", "core", "assets.manifest.json"),
  path.resolve(__dirname, "..", "..", "..", "packages", "assets", "dist", "core", "assets.manifest.json"),
];

const LITERAL_ASSET_REFERENCE_ROOTS = [
  path.resolve(__dirname, "..", "src"),
  path.resolve(__dirname, "..", "..", "..", "packages", "core", "src"),
  path.resolve(__dirname, "..", "..", "..", "packages", "assets", "src"),
];

const listAssetEntries = (rootDir) => {
  const ignoredRelativeRoots = new Set([
    "audio",
    path.join("data", "content-packs", "core-modular"),
  ]);
  const shouldIgnore = (absolutePath) => {
    const relative = path.relative(rootDir, absolutePath);
    return Array.from(ignoredRelativeRoots).some(
      (ignoredRoot) => relative === ignoredRoot || relative.startsWith(`${ignoredRoot}${path.sep}`),
    );
  };
  const visit = (dir) => {
    const entries = fs
      .readdirSync(dir, { withFileTypes: true })
      .filter((entry) => entry.name !== ".DS_Store")
      .sort((a, b) => a.name.localeCompare(b.name));
    const results = [];
    for (const entry of entries) {
      const absolutePath = path.join(dir, entry.name);
      if (/ 2\.json$/.test(entry.name)) {
        continue;
      }
      if (shouldIgnore(absolutePath)) {
        continue;
      }
      results.push({ absolutePath, name: entry.name, isDirectory: entry.isDirectory() });
      if (entry.isDirectory()) {
        results.push(...visit(absolutePath));
      }
    }
    return results;
  };
  return visit(rootDir);
};

const collectManifestGaps = (manifest, webRoot, assetRoot) => {
  const missingFiles = [];
  const missingDirectoryEntries = [];
  for (const entry of listAssetEntries(assetRoot)) {
    const relativePath = `/${path.relative(webRoot, entry.absolutePath).replace(/\\/g, "/")}`;
    const parentDir = `/${path.relative(webRoot, path.dirname(entry.absolutePath)).replace(/\\/g, "/")}`;
    const parentEntries = manifest.directories[parentDir] ?? [];
    if (!parentEntries.includes(entry.name)) {
      missingDirectoryEntries.push(`${parentDir} -> ${entry.name}`);
    }
    if (!entry.isDirectory && !manifest.files.includes(relativePath)) {
      missingFiles.push(relativePath);
    }
  }
  return { missingFiles, missingDirectoryEntries };
};

const walkSourceFiles = (rootDir) => {
  const results = [];
  const visit = (dir) => {
    if (!fs.existsSync(dir)) {
      return;
    }
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const absolutePath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        visit(absolutePath);
      } else if (/\.(?:ts|tsx|js|jsx)$/.test(entry.name)) {
        results.push(absolutePath);
      }
    }
  };
  visit(rootDir);
  return results;
};

const extractLiteralGetAssetPathReferences = (source) => {
  const references = [];
  for (const match of source.matchAll(/getAssetPath\(([^)]*)\)/g)) {
    const parts = [];
    let isLiteralOnly = true;
    for (const rawArg of match[1].split(",")) {
      const literal = rawArg.trim().match(/^['"]([^'"]+)['"]$/);
      if (!literal) {
        isLiteralOnly = false;
        break;
      }
      parts.push(literal[1]);
    }
    if (isLiteralOnly && parts.length > 0) {
      references.push({
        line: source.slice(0, match.index).split("\n").length,
        relativePath: path.join(...parts),
      });
    }
  }
  return references;
};

describe("critical runtime assets", () => {
  test.each(requiredAssets)("%s exists and is non-empty", (relativePath) => {
    const absolutePath = path.resolve(__dirname, "..", relativePath);
    expect(fs.existsSync(absolutePath)).toBe(true);
    expect(fs.statSync(absolutePath).size).toBeGreaterThan(0);
  });

  test("battle_anim_bundle.json remains in the required generated asset set", () => {
    const exportScriptPath = path.resolve(__dirname, "..", "scripts", "export-runtime-fallbacks.js");
    const exportScript = fs.readFileSync(exportScriptPath, "utf8");

    expect(exportScript).toMatch(/"battle_anim_bundle\.json"/);
  });

  test("story-event runtime assets remain listed in the committed web asset manifest", () => {
    const manifestPath = path.resolve(__dirname, "..", "assets.manifest.json");
    const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
    const dataDirEntries = manifest.directories["/assets/data"] ?? [];

    expect(dataDirEntries).toContain("permanent_phone_numbers.json");
    expect(dataDirEntries).toContain("initialize_events.json");
    expect(manifest.files).toContain("/assets/data/permanent_phone_numbers.json");
    expect(manifest.files).toContain("/assets/data/initialize_events.json");
  });

  test("committed asset manifests stay synchronized with the web asset tree", () => {
    const webRoot = path.resolve(__dirname, "..");
    const assetRoot = path.join(webRoot, "assets");

    for (const manifestPath of MANIFEST_PATHS) {
      if (!fs.existsSync(manifestPath)) {
        continue;
      }
      const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
      const { missingFiles, missingDirectoryEntries } = collectManifestGaps(
        manifest,
        webRoot,
        assetRoot,
      );

      expect({
        manifestPath: path.relative(webRoot, manifestPath),
        missingFiles,
        missingDirectoryEntries,
      }).toEqual({
        manifestPath: path.relative(webRoot, manifestPath),
        missingFiles: [],
        missingDirectoryEntries: [],
      });
    }
  });

  test("literal runtime asset references point at bundled assets", () => {
    const assetRoot = path.resolve(__dirname, "..", "assets");
    const missingReferences = [];

    for (const sourcePath of LITERAL_ASSET_REFERENCE_ROOTS.flatMap(walkSourceFiles)) {
      const source = fs.readFileSync(sourcePath, "utf8");
      for (const reference of extractLiteralGetAssetPathReferences(source)) {
        const absoluteAssetPath = path.join(assetRoot, reference.relativePath);
        if (!fs.existsSync(absoluteAssetPath)) {
          missingReferences.push(
            `${path.relative(path.resolve(__dirname, "..", "..", ".."), sourcePath)}:${reference.line} -> ${reference.relativePath}`
          );
        }
      }
    }

    expect(missingReferences).toEqual([]);
  });
});

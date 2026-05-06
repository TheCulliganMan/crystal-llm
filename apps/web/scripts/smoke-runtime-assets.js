#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");

const projectRoot = path.resolve(__dirname, "..");
const candidateRoots = [
  path.join(projectRoot, "public", "assets"),
  path.join(projectRoot, "assets"),
];

const IGNORED_BASENAMES = new Set([".DS_Store"]);
const REQUIRED_RUNTIME_JSON_PATHS = [
  "data/animations.json",
  "data/asm_text.json",
  "data/battle_animation_table.json",
  "data/battle_anim_bundle.json",
  "data/content-packs/index.json",
  "data/content-packs/core-modular.compiled.json",
  "data/egg_moves.json",
  "data/evolutions.json",
  "data/flee_mons.json",
  "data/initialize_events.json",
  "data/items.json",
  "data/learnsets.json",
  "data/level_up_moves.json",
  "data/map_blocks.json",
  "data/map_attributes.json",
  "data/map_dimensions.json",
  "data/menu_icons.json",
  "data/marts.json",
  "data/move_names.json",
  "data/moves_data.json",
  "data/npcs.json",
  "data/pc_strings.json",
  "data/permanent_phone_numbers.json",
  "data/phone_contacts.json",
  "data/pokemon_cries.json",
  "data/pokedex.json",
  "data/pokedex_entries.json",
  "data/pokegear_landmarks.json",
  "data/pokegear_town_map_palette_map.json",
  "data/pokemon_data.json",
  "data/runtime_map_metadata.json",
  "data/runtime_spawn_points.json",
  "data/sprite_anim_bundle.json",
  "data/sprite_palette_defaults.json",
  "data/unown_puzzles/coordinates.json",
  "data/unown_puzzles/layouts.json",
  "data/story_event_script_constants.json",
  "data/trainers.json",
  "data/wild_encounters.json",
  "data/collision/collision_permissions.json",
  "data/collision/collision_stdscripts.json",
  "data/tilesets/johto.json",
  "data/tilesets/johto_palette_map.json",
  "data/tilesets/players_room.json",
  "data/tilesets/players_room_palette_map.json",
];

const TEXT_TYPES = new Set([".asm", ".pal", ".mk", ".dimensions"]);
const BINARY_TYPES = new Set([
  ".2bpp",
  ".2bpp.lz",
  ".1bpp",
  ".gbcpal",
  ".bin",
  ".tilemap",
  ".attrmap",
  ".rle",
  ".mp3",
]);

const inferType = (relativePath) => {
  if (relativePath.endsWith(".2bpp.lz")) {
    return ".2bpp.lz";
  }
  if (relativePath.endsWith(".2bpp")) {
    return ".2bpp";
  }
  if (relativePath.endsWith(".1bpp")) {
    return ".1bpp";
  }
  if (relativePath.endsWith(".gbcpal")) {
    return ".gbcpal";
  }
  return path.extname(relativePath);
};

const walkFiles = (rootDir) => {
  const files = [];
  const visit = (dir) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      if (IGNORED_BASENAMES.has(entry.name)) {
        continue;
      }
      const absolutePath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        visit(absolutePath);
        continue;
      }
      files.push(absolutePath);
    }
  };
  visit(rootDir);
  return files;
};

const validateJson = (absolutePath) => {
  const raw = fs.readFileSync(absolutePath, "utf8");
  if (!raw.trim()) {
    throw new Error("empty JSON file");
  }
  JSON.parse(raw);
};

const validateText = (absolutePath) => {
  const raw = fs.readFileSync(absolutePath, "utf8");
  if (!raw.trim()) {
    throw new Error("empty text asset");
  }
};

const validatePng = (absolutePath) => {
  const raw = fs.readFileSync(absolutePath);
  if (raw.length < 8) {
    throw new Error("truncated PNG");
  }
  const signature = "89504e470d0a1a0a";
  if (raw.subarray(0, 8).toString("hex") !== signature) {
    throw new Error("invalid PNG signature");
  }
};

const validateBinary = (absolutePath) => {
  const raw = fs.readFileSync(absolutePath);
  if (!raw.length) {
    throw new Error("empty binary asset");
  }
};

const validateAsset = (absolutePath, type) => {
  if (type === ".json") {
    validateJson(absolutePath);
    return;
  }
  if (type === ".png") {
    validatePng(absolutePath);
    return;
  }
  if (TEXT_TYPES.has(type)) {
    validateText(absolutePath);
    return;
  }
  if (BINARY_TYPES.has(type)) {
    validateBinary(absolutePath);
    return;
  }
  throw new Error(`unsupported asset type '${type || "(none)"}'`);
};

const smokeRoot = (rootDir) => {
  if (!fs.existsSync(rootDir)) {
    throw new Error(`missing asset root: ${rootDir}`);
  }
  const files = walkFiles(rootDir);
  if (!files.length) {
    throw new Error(`no files found under ${rootDir}`);
  }

  const counts = new Map();
  const examples = new Map();
  for (const absolutePath of files) {
    const relativePath = path.relative(rootDir, absolutePath).replace(/\\/g, "/");
    const type = inferType(relativePath);
    validateAsset(absolutePath, type);
    counts.set(type, (counts.get(type) ?? 0) + 1);
    if (!examples.has(type)) {
      examples.set(type, relativePath);
    }
  }

  const missingRuntimeJson = REQUIRED_RUNTIME_JSON_PATHS.filter(
    (relativePath) => !fs.existsSync(path.join(rootDir, relativePath))
  );
  if (missingRuntimeJson.length) {
    throw new Error(`missing required runtime JSON assets: ${missingRuntimeJson.join(", ")}`);
  }

  const lines = Array.from(counts.entries())
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([type, count]) => `  ${type || "(none)"}: ${count} (${examples.get(type)})`);

  return {
    rootDir,
    fileCount: files.length,
    lines,
  };
};

const roots = candidateRoots.filter((rootDir, index, list) => {
  const resolved = fs.existsSync(rootDir) ? fs.realpathSync(rootDir) : rootDir;
  return index === list.findIndex((candidate) => {
    const otherResolved = fs.existsSync(candidate) ? fs.realpathSync(candidate) : candidate;
    return otherResolved === resolved;
  });
});

const main = () => {
  let failures = 0;
  let checkedRoots = 0;
  for (const rootDir of roots) {
    if (!fs.existsSync(rootDir)) {
      console.log(`[asset-smoke] SKIP ${rootDir} (missing optional asset root)`);
      continue;
    }
    try {
      const result = smokeRoot(rootDir);
      checkedRoots += 1;
      console.log(`[asset-smoke] PASS ${result.rootDir} (${result.fileCount} files)`);
      for (const line of result.lines) {
        console.log(line);
      }
    } catch (error) {
      failures += 1;
      const message = error instanceof Error ? error.message : String(error);
      console.error(`[asset-smoke] FAIL ${rootDir}: ${message}`);
    }
  }

  if (checkedRoots === 0) {
    console.error("[asset-smoke] FAIL no asset roots found");
    failures += 1;
  }

  if (failures) {
    process.exit(1);
  }
};

if (require.main === module) {
  main();
} else {
  module.exports = {
    REQUIRED_RUNTIME_JSON_PATHS,
    main,
    smokeRoot,
    validateAsset,
    validateJson,
  };
}

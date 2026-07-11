#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");

const CONTENT_PACK_ROOT_RELATIVE = "content-packs";
const PACK_CONFIG_FILE = "pack.json";

const CATEGORY_NAMES = [
  "pokemon",
  "moves",
  "learnsets",
  "level_up_moves",
  "egg_moves",
  "evolutions",
  "maps",
  "map_blocks",
  "map_attributes",
  "map_dimensions",
  "wild_encounters",
  "npcs",
  "pokegear_landmarks",
  "items",
  "trainers",
  "pokedex",
  "story_events",
  "phone_scripts",
];

const listDirectories = (rootDir) => {
  if (!fs.existsSync(rootDir)) {
    return [];
  }
  return fs
    .readdirSync(rootDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort((a, b) => a.localeCompare(b));
};

const listJsonFilesRecursive = (rootDir) => {
  if (!fs.existsSync(rootDir)) {
    return [];
  }
  const results = [];
  const queue = [rootDir];
  while (queue.length > 0) {
    const current = queue.shift();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const absolutePath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        queue.push(absolutePath);
        continue;
      }
      if (entry.isFile() && entry.name.toLowerCase().endsWith(".json")) {
        results.push(absolutePath);
      }
    }
  }
  return results.sort((a, b) => a.localeCompare(b));
};

const readPackConfig = (packRoot, defaultId) => {
  const configPath = path.join(packRoot, PACK_CONFIG_FILE);
  if (!fs.existsSync(configPath)) {
    return {
      id: defaultId,
      enabled: true,
      priority: 0,
    };
  }
  try {
    const parsed = JSON.parse(fs.readFileSync(configPath, "utf8"));
    return {
      id: typeof parsed.id === "string" && parsed.id.trim() ? parsed.id.trim() : defaultId,
      enabled: parsed.enabled !== false,
      priority: Number.isFinite(parsed.priority) ? Number(parsed.priority) : 0,
    };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Invalid content pack config at ${configPath}: ${message}`);
  }
};

const pathToDataRelative = (dataRoot, absoluteFile) =>
  path
    .relative(dataRoot, absoluteFile)
    .split(path.sep)
    .join("/");

const gatherPackFiles = (dataRoot, packRootRelative) => {
  const packRoot = path.join(dataRoot, packRootRelative);
  const files = {};
  for (const category of CATEGORY_NAMES) {
    const categoryDir = path.join(packRoot, category);
    const categoryFiles = listJsonFilesRecursive(categoryDir).map((filePath) =>
      pathToDataRelative(dataRoot, filePath)
    );

    const rootFile = path.join(packRoot, `${category}.json`);
    if (fs.existsSync(rootFile)) {
      categoryFiles.unshift(pathToDataRelative(dataRoot, rootFile));
    }

    files[category] = Array.from(new Set(categoryFiles));
  }
  return files;
};

const readExistingCompiledEntries = (indexPath) => {
  if (!fs.existsSync(indexPath)) {
    return new Map();
  }
  try {
    const parsed = JSON.parse(fs.readFileSync(indexPath, "utf8"));
    const packs = Array.isArray(parsed?.packs) ? parsed.packs : [];
    return new Map(
      packs
        .filter((pack) => typeof pack?.id === "string" && typeof pack?.compiled === "string")
        .map((pack) => [pack.id, pack.compiled])
    );
  } catch {
    return new Map();
  }
};

const buildIndexPayload = ({ dataRoot }) => {
  const contentPackRoot = path.join(dataRoot, CONTENT_PACK_ROOT_RELATIVE);
  const existingCompiledById = readExistingCompiledEntries(
    path.join(contentPackRoot, "index.json")
  );
  const packDirectories = listDirectories(contentPackRoot);
  const packs = packDirectories.map((packDir) => {
    const packRootRelative = path.join(CONTENT_PACK_ROOT_RELATIVE, packDir);
    const packRoot = path.join(dataRoot, packRootRelative);
    const config = readPackConfig(packRoot, packDir);
    const pack = {
      id: config.id,
      enabled: config.enabled,
      priority: config.priority,
      path: packRootRelative.split(path.sep).join("/"),
      files: gatherPackFiles(dataRoot, packRootRelative),
    };
    const defaultCompiled = path.join(
      CONTENT_PACK_ROOT_RELATIVE,
      `${packDir}.compiled.json`
    ).split(path.sep).join("/");
    const compiled = existingCompiledById.get(pack.id) ??
      (fs.existsSync(path.join(dataRoot, defaultCompiled)) ? defaultCompiled : null);
    if (compiled) {
      pack.compiled = compiled;
    }
    return pack;
  });

  packs.sort((a, b) => {
    if (a.priority !== b.priority) {
      return a.priority - b.priority;
    }
    return a.id.localeCompare(b.id);
  });

  return {
    version: 1,
    packs,
  };
};

const syncContentPackIndex = ({
  projectRoot = path.resolve(__dirname, ".."),
  assetsSource = path.join(projectRoot, "assets"),
} = {}) => {
  const dataRoot = path.join(assetsSource, "data");
  if (!fs.existsSync(dataRoot)) {
    return {
      skipped: true,
      reason: `missing data directory: ${dataRoot}`,
    };
  }

  // The Rust exporter owns the v2 index for compiled `.crystalpack` artifacts.
  // The legacy web synchronizer must never reinterpret that schema or replace
  // it with its smaller JSON-only view.
  const existingIndexPath = path.join(dataRoot, CONTENT_PACK_ROOT_RELATIVE, "index.json");
  if (fs.existsSync(existingIndexPath)) {
    try {
      const existing = JSON.parse(fs.readFileSync(existingIndexPath, "utf8"));
      if (existing?.version >= 2 || existing?.packs?.some((pack) =>
        typeof pack?.compiled === "string" && pack.compiled.endsWith(".crystalpack")
      )) {
        return {
          skipped: true,
          reason: "compiled Rust content-pack index is authoritative",
          outputPath: existingIndexPath,
          outputPaths: [existingIndexPath],
        };
      }
    } catch {
      // The normal path below reports malformed JSON through its own rewrite.
    }
  }

  const payload = buildIndexPayload({ dataRoot });
  const nextText = `${JSON.stringify(payload, null, 2)}\n`;
  const outputPaths = [path.join(dataRoot, CONTENT_PACK_ROOT_RELATIVE, "index.json")];

  let changed = false;
  for (const outputPath of outputPaths) {
    fs.mkdirSync(path.dirname(outputPath), { recursive: true });
    const previousText = fs.existsSync(outputPath)
      ? fs.readFileSync(outputPath, "utf8")
      : null;
    if (previousText === nextText) {
      continue;
    }
    fs.writeFileSync(outputPath, nextText);
    changed = true;
  }

  return {
    skipped: false,
    changed,
    outputPath: outputPaths[0],
    outputPaths,
  };
};

if (require.main === module) {
  const result = syncContentPackIndex();
  if (result?.skipped) {
    console.warn(`[content-packs] skipped: ${result.reason}`);
  } else if (result?.changed) {
    console.log(`[content-packs] wrote ${result.outputPath}`);
  } else {
    console.log("[content-packs] index already up to date");
  }
}

module.exports = { syncContentPackIndex };

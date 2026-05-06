#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");

const listEntries = (rootDir) => {
  if (!fs.existsSync(rootDir)) {
    return [];
  }
  const results = [];
  const visit = (dir) => {
    const entries = fs
      .readdirSync(dir, { withFileTypes: true })
      .filter((entry) => entry.name !== ".DS_Store")
      .sort((a, b) => a.name.localeCompare(b.name));
    for (const entry of entries) {
      const absolutePath = path.join(dir, entry.name);
      results.push({ absolutePath, isDirectory: entry.isDirectory(), name: entry.name });
      if (entry.isDirectory()) {
        visit(absolutePath);
      }
    }
  };
  visit(rootDir);
  return results;
};

const toManifestPath = (projectRoot, absolutePath) =>
  `/${path.relative(projectRoot, absolutePath).split(path.sep).join("/")}`;

const buildAssetManifest = ({ projectRoot, assetsSource }) => {
  const directories = {};
  const files = [];

  if (fs.existsSync(assetsSource)) {
    directories["/assets"] = fs
      .readdirSync(assetsSource, { withFileTypes: true })
      .filter((entry) => entry.name !== ".DS_Store")
      .map((entry) => entry.name)
      .sort((a, b) => a.localeCompare(b));
  }

  for (const entry of listEntries(assetsSource)) {
    const manifestPath = toManifestPath(projectRoot, entry.absolutePath);
    if (entry.isDirectory) {
      directories[manifestPath] = fs
        .readdirSync(entry.absolutePath, { withFileTypes: true })
        .filter((child) => child.name !== ".DS_Store")
        .map((child) => child.name)
        .sort((a, b) => a.localeCompare(b));
    } else {
      files.push(manifestPath);
    }
  }

  files.sort((a, b) => a.localeCompare(b));

  return {
    generatedAt: new Date(0).toISOString(),
    directories,
    files,
  };
};

const writeIfChanged = (filePath, content) => {
  if (fs.existsSync(filePath) && fs.readFileSync(filePath, "utf8") === content) {
    return false;
  }
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content);
  return true;
};

const syncManifestLink = ({ manifestPath, webManifestPath }) => {
  const relativeTarget = path.relative(path.dirname(manifestPath), webManifestPath);
  fs.mkdirSync(path.dirname(manifestPath), { recursive: true });
  fs.rmSync(manifestPath, { force: true });
  try {
    fs.symlinkSync(relativeTarget, manifestPath);
  } catch {
    fs.copyFileSync(webManifestPath, manifestPath);
  }
  return manifestPath;
};

const syncManifestMirrors = ({ repoRoot, webManifestPath }) => {
  const mirrorPaths = [
    path.join(repoRoot, "packages", "core", "assets.manifest.json"),
    path.join(repoRoot, "packages", "assets", "dist", "core", "assets.manifest.json"),
  ];
  return mirrorPaths.map((manifestPath) => syncManifestLink({ manifestPath, webManifestPath }));
};

const syncAssetManifest = ({
  projectRoot = path.resolve(__dirname, ".."),
  repoRoot = path.resolve(projectRoot, "..", ".."),
  assetsSource = path.join(projectRoot, "assets"),
} = {}) => {
  const manifest = buildAssetManifest({ projectRoot, assetsSource });
  const webManifestPath = path.join(projectRoot, "assets.manifest.json");
  const changed = writeIfChanged(
    webManifestPath,
    `${JSON.stringify(manifest, null, 2)}\n`
  );
  const mirrorManifestPaths = syncManifestMirrors({ repoRoot, webManifestPath });

  return {
    changed,
    files: manifest.files.length,
    directories: Object.keys(manifest.directories).length,
    webManifestPath,
    coreManifestPath: mirrorManifestPaths[0],
    mirrorManifestPaths,
  };
};

if (require.main === module) {
  const result = syncAssetManifest();
  console.log(
    `[asset-manifest] indexed ${result.files} files and ${result.directories} directories`
  );
}

module.exports = { buildAssetManifest, syncAssetManifest };

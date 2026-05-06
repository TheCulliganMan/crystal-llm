#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const repoRoot = path.resolve(__dirname, "..", "..", "..");
const nodeBin = process.execPath;
const tscBin = path.join(repoRoot, "node_modules", "typescript", "bin", "tsc");

const packages = [
  {
    name: "@pokecrystal/core",
    dir: path.join(repoRoot, "packages", "core"),
    srcDirs: ["src"],
    project: "tsconfig.build.json",
    requiredOutputs: ["dist/core/src/index.js", "dist/core/src/ui/menus/pokegear-labels.js"],
  },
  {
    name: "@pokecrystal/assets",
    dir: path.join(repoRoot, "packages", "assets"),
    srcDirs: ["src"],
    project: "tsconfig.build.json",
    requiredOutputs: ["dist/assets/src/index.js", "dist/assets/src/content/pokegear.js"],
  },
  {
    name: "@pokecrystal/exporters",
    dir: path.join(repoRoot, "packages", "exporters"),
    srcDirs: ["src"],
    project: "tsconfig.build.json",
    requiredOutputs: ["dist/exporters/src/index.js"],
  },
];

const walkLatestMtime = (targetPath) => {
  if (!fs.existsSync(targetPath)) {
    return 0;
  }
  const stat = fs.statSync(targetPath);
  if (!stat.isDirectory()) {
    return stat.mtimeMs;
  }

  let latest = stat.mtimeMs;
  for (const entry of fs.readdirSync(targetPath, { withFileTypes: true })) {
    if (entry.name === "node_modules" || entry.name === ".git" || entry.name === ".next") {
      continue;
    }
    latest = Math.max(latest, walkLatestMtime(path.join(targetPath, entry.name)));
  }
  return latest;
};

const packageNeedsBuild = ({ dir, srcDirs, requiredOutputs = [] }) => {
  const distDir = path.join(dir, "dist");
  if (!fs.existsSync(distDir)) {
    return true;
  }
  for (const output of requiredOutputs) {
    if (!fs.existsSync(path.join(dir, output))) {
      return true;
    }
  }

  const sourceLatest = Math.max(
    walkLatestMtime(path.join(dir, "package.json")),
    ...srcDirs.map((srcDir) => walkLatestMtime(path.join(dir, srcDir)))
  );
  const distLatest = walkLatestMtime(distDir);
  return sourceLatest > distLatest;
};

const runBuild = ({ name, dir, project }) => {
  fs.rmSync(path.join(dir, "dist"), { recursive: true, force: true });
  const result = spawnSync(nodeBin, [tscBin, "-p", project, "--noCheck", "--declaration", "false", "--emitDeclarationOnly", "false"], {
    cwd: dir,
    env: process.env,
    stdio: "inherit",
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`Failed to build ${name}`);
  }
};

const syncWorkspaceRuntime = (
  workspacePackages = packages,
  deps = { packageNeedsBuild, runBuild, log: console.log }
) => {
  for (const pkg of workspacePackages) {
    if (deps.packageNeedsBuild(pkg)) {
      deps.log(`[sync-workspace-runtime] building ${pkg.name}`);
      deps.runBuild(pkg);
    } else {
      deps.log(`[sync-workspace-runtime] ${pkg.name} already up to date`);
    }
  }
};

if (require.main === module) {
  try {
    syncWorkspaceRuntime();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}

module.exports = {
  packages,
  walkLatestMtime,
  packageNeedsBuild,
  runBuild,
  syncWorkspaceRuntime,
};

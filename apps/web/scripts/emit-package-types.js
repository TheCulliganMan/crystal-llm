#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const repoRoot = path.resolve(__dirname, "..", "..", "..");
const workspaceRoot = path.resolve(__dirname, "..");
const nodeBin = process.execPath;
const tscBin = path.join(repoRoot, "node_modules", "typescript", "bin", "tsc");
const outputRoot = path.join(workspaceRoot, ".generated", "types", "packages");

const packages = [
  {
    name: "core",
    project: path.join(repoRoot, "packages", "core", "tsconfig.build.json"),
    outDir: path.join(outputRoot, "core"),
  },
  {
    name: "assets",
    project: path.join(repoRoot, "packages", "assets", "tsconfig.json"),
    outDir: path.join(outputRoot, "assets"),
  },
  {
    name: "exporters",
    project: path.join(repoRoot, "packages", "exporters", "tsconfig.json"),
    outDir: path.join(outputRoot, "exporters"),
  },
];

function emitPackageTypes({ name, project, outDir }) {
  fs.mkdirSync(outDir, { recursive: true });
  const result = spawnSync(
    nodeBin,
    [
      tscBin,
      "-p",
      project,
      "--declaration",
      "--emitDeclarationOnly",
      "--noCheck",
      "--outDir",
      outDir,
      "--incremental",
      "false",
      "--noEmit",
      "false",
    ],
    { stdio: "inherit", env: process.env, cwd: repoRoot }
  );

  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`Failed to emit types for ${name}`);
  }
}

try {
  fs.rmSync(outputRoot, { recursive: true, force: true });
  for (const pkg of packages) {
    emitPackageTypes(pkg);
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}

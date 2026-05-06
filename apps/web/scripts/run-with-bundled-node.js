#!/usr/bin/env node
/* eslint-disable no-console */
/**
 * Runs a JS entrypoint using the repo's bundled Node (./.node/...) when present,
 * otherwise falls back to the current `node` (works on Vercel where `.node/` is absent).
 *
 * Usage:
 *   node scripts/run-with-bundled-node.js <jsFile> [...args]
 */

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

function bundledNodePath() {
  const repoRoot = path.resolve(__dirname, "../../../");
  const platform = process.platform;
  const arch = process.arch;
  const candidates = [
    path.join(repoRoot, ".node", `node-v24.1.0-${platform}-${arch}`, "bin", "node"),
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  return null;
}

function resolveNextDistDir(env = process.env) {
  if (env.POKECRYSTAL_NEXT_DIST_DIR) {
    return env.POKECRYSTAL_NEXT_DIST_DIR;
  }
  return null;
}

function main() {
  const [, , target, ...args] = process.argv;
  if (!target) {
    console.error("Usage: node scripts/run-with-bundled-node.js <jsFile> [...args]");
    process.exit(2);
  }

  const absTarget = path.isAbsolute(target) ? target : path.resolve(process.cwd(), target);

  const preferredNode = bundledNodePath();
  // Vercel builds won't have `./.node` (and you may want to simulate that locally).
  const forceSystemNode =
    process.env.POKECRYSTAL_TS_FORCE_SYSTEM_NODE === "1" || process.env.VERCEL === "1";
  const nodeBin = !forceSystemNode && preferredNode ? preferredNode : process.execPath;
  const nextDistDir = resolveNextDistDir();
  const env = nextDistDir
    ? { ...process.env, POKECRYSTAL_NEXT_DIST_DIR: nextDistDir }
    : process.env;

  const res = spawnSync(nodeBin, [absTarget, ...args], {
    stdio: "inherit",
    env,
  });

  if (res.error) {
    console.error(res.error);
    process.exit(1);
  }
  process.exit(res.status == null ? 1 : res.status);
}

if (require.main === module) {
  main();
}

module.exports = {
  bundledNodePath,
  resolveNextDistDir,
};

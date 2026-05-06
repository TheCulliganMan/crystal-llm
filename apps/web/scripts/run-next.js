#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const path = require("node:path");

const resolveNextBin = (cwd = process.cwd()) => {
  const nextPackagePath = require.resolve("next/package.json", { paths: [cwd] });
  return path.join(path.dirname(nextPackagePath), "dist", "bin", "next");
};

const runNext = (args, { cwd } = {}) => {
  const nextBin = resolveNextBin(cwd);
  const result = spawnSync(process.execPath, [nextBin, ...args], { stdio: "inherit" });
  return result.status ?? 1;
};

if (require.main === module) {
  const args = process.argv.slice(2);
  if (args.length === 0) {
    console.error("Usage: run-next.js <next args>");
    process.exit(1);
  }

  process.exit(runNext(args));
}

module.exports = { resolveNextBin, runNext };

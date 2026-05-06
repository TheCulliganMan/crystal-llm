#!/usr/bin/env node
/* eslint-disable no-console */
const fs = require("node:fs");
const path = require("node:path");

const resolveNextDir = (cwd = process.cwd(), env = process.env) =>
  path.resolve(cwd, env.POKECRYSTAL_NEXT_DIST_DIR || ".next");
const duplicateSuffixPattern = / \d+$/;

const removeIfExists = (targetPath) => {
  if (!fs.existsSync(targetPath)) {
    return;
  }
  fs.rmSync(targetPath, { recursive: true, force: true });
};

const pruneDuplicateArtifacts = (dirPath) => {
  if (!fs.existsSync(dirPath)) {
    return;
  }
  const entries = fs.readdirSync(dirPath, { withFileTypes: true });
  for (const entry of entries) {
    const absolutePath = path.join(dirPath, entry.name);
    if (duplicateSuffixPattern.test(entry.name)) {
      fs.rmSync(absolutePath, { recursive: true, force: true });
      console.log("[clean-next-artifacts] removed duplicate artifact:", absolutePath);
      continue;
    }
    if (entry.isDirectory()) {
      pruneDuplicateArtifacts(absolutePath);
    }
  }
};

const cleanNextArtifacts = (cwd = process.cwd(), env = process.env) => {
  const nextDir = resolveNextDir(cwd, env);
  if (!fs.existsSync(nextDir)) {
    return;
  }
  removeIfExists(path.join(nextDir, "lock"));
  pruneDuplicateArtifacts(nextDir);
};

const main = () => {
  cleanNextArtifacts();
};

main();

module.exports = { cleanNextArtifacts, resolveNextDir };

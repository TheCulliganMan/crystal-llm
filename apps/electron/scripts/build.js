"use strict";

const { ensureDesktopWebBuild } = require("./package");

const startedAt = Date.now();

try {
  console.log(`Preparing Electron desktop assets at ${new Date(startedAt).toISOString()}...`);
  ensureDesktopWebBuild();
  const elapsed = ((Date.now() - startedAt) / 1000).toFixed(1);
  console.log(`Electron build step complete in ${elapsed}s`);
} catch (error) {
  console.error(error.message ?? error);
  process.exit(1);
}

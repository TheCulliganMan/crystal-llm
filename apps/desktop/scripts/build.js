"use strict";

const { buildNativeApp, ensureDesktopWebBuild, stageDesktopResources } = require("./package");

const startedAt = Date.now();

try {
  console.log(`Preparing Zero Native desktop app at ${new Date(startedAt).toISOString()}...`);
  ensureDesktopWebBuild();
  stageDesktopResources();
  buildNativeApp();
  const elapsed = ((Date.now() - startedAt) / 1000).toFixed(1);
  console.log(`Zero Native build step complete in ${elapsed}s`);
} catch (error) {
  console.error(error.message ?? error);
  process.exit(1);
}

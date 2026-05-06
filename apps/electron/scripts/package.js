"use strict";

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const {
  DESKTOP_BUILD_DIST_DIR,
  ROOT_DIR,
  resolveElectronBuilderCliPath,
} = require("./launch-helpers");

const ELECTRON_DIR = path.resolve(__dirname, "..");
const WEB_DIR = path.resolve(ELECTRON_DIR, "../web");
const NPM_COMMAND = process.platform === "win32" ? "npm.cmd" : "npm";
const ELECTRON_DIST_DIR = path.join(ELECTRON_DIR, "dist");
const STANDALONE_DIR = path.join(WEB_DIR, DESKTOP_BUILD_DIST_DIR, "standalone");
const STANDALONE_APP_DIR = path.join(STANDALONE_DIR, "apps", "web");

const runSync = (command, args, options = {}) => {
  const result = spawnSync(command, args, {
    cwd: ROOT_DIR,
    stdio: "inherit",
    shell: process.platform === "win32",
    env: process.env,
    ...options,
  });

  if (result.status !== 0) {
    const code = result.status ?? 1;
    throw new Error(`Command failed (${code}): ${command} ${args.join(" ")}`);
  }
};

const ensureDesktopWebBuild = () => {
  runSync(process.execPath, [path.join(__dirname, "generate-icons.js")], {
    cwd: ELECTRON_DIR,
  });
  runSync(NPM_COMMAND, ["run", "build", "--workspace", "@pokecrystal/web"], {
    env: {
      ...process.env,
      POKECRYSTAL_NEXT_DIST_DIR: DESKTOP_BUILD_DIST_DIR,
      POKECRYSTAL_NEXT_OUTPUT: "standalone",
    },
  });
  pruneStandaloneBundle();
};

const ensureBuilderArtifacts = () => {
  if (!fs.existsSync(ELECTRON_DIST_DIR)) {
    throw new Error(`Electron packaging completed without creating ${ELECTRON_DIST_DIR}.`);
  }

  const entries = fs.readdirSync(ELECTRON_DIST_DIR);
  if (entries.length === 0) {
    throw new Error(`Electron packaging completed without creating any artifacts in ${ELECTRON_DIST_DIR}.`);
  }
};

const pruneStandaloneBundle = () => {
  if (!fs.existsSync(STANDALONE_APP_DIR)) {
    throw new Error(`Expected Next standalone output at ${STANDALONE_APP_DIR}.`);
  }

  const removablePaths = [
    path.join(STANDALONE_APP_DIR, "public", "downloads"),
    path.join(STANDALONE_APP_DIR, "public", "ffmpeg"),
  ];

  for (const removablePath of removablePaths) {
    fs.rmSync(removablePath, { recursive: true, force: true });
  }

  for (const entry of fs.readdirSync(STANDALONE_APP_DIR)) {
    if (/^mcp-.*-runtime\.json$/i.test(entry)) {
      fs.rmSync(path.join(STANDALONE_APP_DIR, entry), { force: true });
    }
  }
};

const packageDesktopApp = () => {
  const builderCliPath = resolveElectronBuilderCliPath();
  fs.rmSync(ELECTRON_DIST_DIR, { recursive: true, force: true });

  runSync(process.execPath, [
    builderCliPath,
    "build",
    "--publish=never",
  ], {
    cwd: ELECTRON_DIR,
    env: {
      ...process.env,
      POKECRYSTAL_NEXT_DIST_DIR: DESKTOP_BUILD_DIST_DIR,
    },
  });

  ensureBuilderArtifacts();
};

if (require.main === module) {
  const startedAt = Date.now();

  try {
    console.log(`Packaging Electron desktop app at ${new Date(startedAt).toISOString()}...`);
    ensureDesktopWebBuild();
    packageDesktopApp();
    const elapsed = ((Date.now() - startedAt) / 1000).toFixed(1);
    console.log(`Electron package step complete in ${elapsed}s`);
  } catch (error) {
    console.error(error.message ?? error);
    process.exit(1);
  }
}

module.exports = {
  DESKTOP_BUILD_DIST_DIR,
  ELECTRON_DIR,
  ELECTRON_DIST_DIR,
  ROOT_DIR,
  STANDALONE_APP_DIR,
  STANDALONE_DIR,
  WEB_DIR,
  ensureBuilderArtifacts,
  ensureDesktopWebBuild,
  packageDesktopApp,
  pruneStandaloneBundle,
  runSync,
};

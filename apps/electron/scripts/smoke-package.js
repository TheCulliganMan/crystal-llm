"use strict";

const { spawn } = require("node:child_process");
const fs = require("node:fs");
const fsp = require("node:fs/promises");
const os = require("node:os");
const path = require("node:path");
const { listPackage } = require("@electron/asar");
const { findStandaloneServerEntrypoint } = require("./launch-helpers");
const { ELECTRON_DIR, ensureDesktopWebBuild, packageDesktopApp } = require("./package");

const PRODUCT_NAME = "KrabbyClaw Desktop";
const SMOKE_TIMEOUT_MS = 60_000;

const fail = (message) => {
  throw new Error(message);
};

const findInstallerArtifact = (distDir) => {
  const entries = fs.readdirSync(distDir);

  if (process.platform === "darwin") {
    return entries.find((entry) => entry.endsWith(".dmg")) ?? null;
  }
  if (process.platform === "win32") {
    return entries.find((entry) => entry.endsWith(".exe")) ?? null;
  }
  if (process.platform === "linux") {
    return entries.find((entry) => entry.endsWith(".AppImage") || entry.endsWith(".deb") || entry.endsWith(".rpm")) ?? null;
  }

  return null;
};

const getUnpackedAppPath = (distDir) => {
  if (process.platform === "darwin") {
    const appPath = fs.readdirSync(distDir)
      .map((entry) => path.join(distDir, entry, `${PRODUCT_NAME}.app`))
      .find((candidate) => fs.existsSync(candidate));
    if (!appPath) {
      fail(`Expected a macOS app bundle for ${PRODUCT_NAME} inside ${distDir}`);
    }
    return appPath;
  }
  if (process.platform === "win32") {
    return path.join(distDir, "win-unpacked");
  }
  if (process.platform === "linux") {
    return path.join(distDir, "linux-unpacked");
  }
  fail(`Unsupported smoke-test platform: ${process.platform}`);
};

const getExecutablePath = (unpackedAppPath) => {
  if (process.platform === "darwin") {
    return path.join(unpackedAppPath, "Contents", "MacOS", PRODUCT_NAME);
  }
  if (process.platform === "win32") {
    return path.join(unpackedAppPath, `${PRODUCT_NAME}.exe`);
  }
  if (process.platform === "linux") {
    const candidates = fs.readdirSync(unpackedAppPath)
      .map((entry) => path.join(unpackedAppPath, entry))
      .filter((entry) => fs.statSync(entry).isFile());
    const executable = candidates.find((entry) => (fs.statSync(entry).mode & 0o111) !== 0);
    if (!executable) {
      fail(`No executable found in ${unpackedAppPath}`);
    }
    return executable;
  }
  fail(`Unsupported smoke-test platform: ${process.platform}`);
};

const verifyPackagedContents = (unpackedAppPath) => {
  const resourcesDir =
    process.platform === "darwin"
      ? path.join(unpackedAppPath, "Contents", "Resources")
      : path.join(unpackedAppPath, "resources");
  const appAsarPath = path.join(resourcesDir, "app.asar");
  const packagedEntries = new Set(listPackage(appAsarPath));

  if (!packagedEntries.has("main.js") && !packagedEntries.has("/main.js")) {
    fail(`Expected main.js inside ${appAsarPath}`);
  }

  const standaloneRoot = path.join(resourcesDir, "web-standalone");
  if (!fs.existsSync(standaloneRoot)) {
    fail(`Expected packaged standalone output inside ${resourcesDir}`);
  }
  findStandaloneServerEntrypoint(standaloneRoot);

  for (const resourcePath of ["web-standalone/apps/web/.next-electron/static", "web-standalone/apps/web/public"]) {
    if (!fs.existsSync(path.join(resourcesDir, resourcePath))) {
      fail(`Expected packaged resource ${resourcePath} inside ${resourcesDir}`);
    }
  }
};

const waitForSmokeResult = async (smokePath, child) => {
  const deadline = Date.now() + SMOKE_TIMEOUT_MS;

  while (Date.now() < deadline) {
    if (fs.existsSync(smokePath)) {
      const payload = JSON.parse(await fsp.readFile(smokePath, "utf8"));
      if (!payload.ok) {
        fail(payload.error ?? "Desktop smoke test failed.");
      }
      return payload;
    }

    if (child.exitCode != null) {
      fail(`Packaged app exited before writing smoke output (exit ${child.exitCode}).`);
    }

    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  fail("Timed out waiting for packaged desktop smoke output.");
};

const runPackagedLaunchSmoke = async (executablePath) => {
  const smokeDir = await fsp.mkdtemp(path.join(os.tmpdir(), "krabbyclaw-desktop-smoke-"));
  const smokePath = path.join(smokeDir, "result.json");
  const child = spawn(executablePath, [], {
    env: {
      ...process.env,
      ELECTRON_RUN_AS_NODE: undefined,
      KRABBY_DESKTOP_SMOKE_PATH: smokePath,
    },
    stdio: ["ignore", "inherit", "inherit"],
  });

  try {
    return await waitForSmokeResult(smokePath, child);
  } finally {
    child.kill("SIGTERM");
    await fsp.rm(smokeDir, { recursive: true, force: true });
  }
};

const main = async () => {
  ensureDesktopWebBuild();
  packageDesktopApp();

  const distDir = path.join(ELECTRON_DIR, "dist");
  const unpackedAppPath = getUnpackedAppPath(distDir);
  const installerArtifact = findInstallerArtifact(distDir);

  if (!fs.existsSync(unpackedAppPath)) {
    fail(`Expected unpacked app output at ${unpackedAppPath}`);
  }
  if (!installerArtifact) {
    fail(`Expected an installer artifact in ${distDir}`);
  }

  verifyPackagedContents(unpackedAppPath);

  const executablePath = getExecutablePath(unpackedAppPath);
  const smokeResult = await runPackagedLaunchSmoke(executablePath);
  console.log(`Packaged desktop smoke passed: ${smokeResult.href}`);
};

main().catch((error) => {
  console.error(error.message ?? error);
  process.exit(1);
});

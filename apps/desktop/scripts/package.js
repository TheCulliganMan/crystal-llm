"use strict";

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const {
  DESKTOP_BUILD_DIST_DIR,
  DESKTOP_DIR,
  DESKTOP_PACKAGE_DIR,
  DESKTOP_RESOURCES_DIR,
  ROOT_DIR,
  resolveZeroNativeCliPath,
} = require("./launch-helpers");

const WEB_DIR = path.resolve(DESKTOP_DIR, "../web");
const NPM_COMMAND = process.platform === "win32" ? "npm.cmd" : "npm";
const WEB_BUILD_DIR = path.join(WEB_DIR, DESKTOP_BUILD_DIST_DIR);
const DESKTOP_HTML_PATH = path.join(WEB_BUILD_DIR, "server", "app", "desktop.html");
const NATIVE_BINARY = path.join(DESKTOP_DIR, "zig-out", "bin", process.platform === "win32" ? "krabbyclaw-desktop.exe" : "krabbyclaw-desktop");

const withLocalBinPath = (env = process.env) => ({
  ...env,
  PATH: `${path.join(ROOT_DIR, "node_modules", ".bin")}${path.delimiter}${env.PATH ?? ""}`,
});

const runSync = (command, args, options = {}) => {
  const result = spawnSync(command, args, {
    cwd: ROOT_DIR,
    stdio: "inherit",
    shell: process.platform === "win32",
    env: process.env,
    ...options,
  });

  if (result.error) {
    if (result.error.code === "ENOENT") {
      const installHint =
        command === "zig" ? " Install Zig 0.16.0+ before building the Zero Native desktop app." : "";
      throw new Error(`${command} was not found on PATH.${installHint}`);
    }
    throw result.error;
  }

  if (result.status !== 0) {
    const code = result.status ?? 1;
    throw new Error(`Command failed (${code}): ${command} ${args.join(" ")}`);
  }
};

const ensureDesktopWebBuild = () => {
  runSync(process.execPath, [path.join(__dirname, "generate-icons.js")], {
    cwd: DESKTOP_DIR,
  });
  runSync(NPM_COMMAND, ["run", "build", "--workspace", "@pokecrystal/web"], {
    env: {
      ...process.env,
      POKECRYSTAL_NEXT_DIST_DIR: DESKTOP_BUILD_DIST_DIR,
    },
  });
};

const stageDesktopResources = () => {
  if (!fs.existsSync(DESKTOP_HTML_PATH)) {
    throw new Error(`Expected static desktop HTML at ${DESKTOP_HTML_PATH}.`);
  }

  fs.rmSync(DESKTOP_RESOURCES_DIR, { recursive: true, force: true });
  fs.mkdirSync(DESKTOP_RESOURCES_DIR, { recursive: true });

  fs.cpSync(DESKTOP_HTML_PATH, path.join(DESKTOP_RESOURCES_DIR, "index.html"));
  fs.cpSync(path.join(WEB_BUILD_DIR, "static"), path.join(DESKTOP_RESOURCES_DIR, "_next", "static"), {
    recursive: true,
  });
  fs.cpSync(path.join(WEB_DIR, "public"), DESKTOP_RESOURCES_DIR, {
    recursive: true,
    filter: (source) => {
      const relative = path.relative(path.join(WEB_DIR, "public"), source);
      return !relative.split(path.sep).some((part) => part === "downloads" || part === "ffmpeg");
    },
  });
  fs.cpSync(path.join(DESKTOP_DIR, "assets", "icon.png"), path.join(DESKTOP_RESOURCES_DIR, "icon.png"));
};

const buildNativeApp = () => {
  runSync("zig", ["build"], {
    cwd: DESKTOP_DIR,
    env: withLocalBinPath(),
  });
};

const ensureBuilderArtifacts = () => {
  if (!fs.existsSync(DESKTOP_PACKAGE_DIR)) {
    throw new Error(`Zero Native packaging completed without creating ${DESKTOP_PACKAGE_DIR}.`);
  }

  const entries = fs.readdirSync(DESKTOP_PACKAGE_DIR);
  if (entries.length === 0) {
    throw new Error(`Zero Native packaging completed without creating any artifacts in ${DESKTOP_PACKAGE_DIR}.`);
  }
};

const resolvePackageTarget = () => {
  if (process.platform === "darwin") {
    return "macos";
  }
  if (process.platform === "win32") {
    return "windows";
  }
  return "linux";
};

const packageDesktopApp = () => {
  const zeroNativeCliPath = resolveZeroNativeCliPath();
  fs.rmSync(DESKTOP_PACKAGE_DIR, { recursive: true, force: true });
  fs.mkdirSync(DESKTOP_PACKAGE_DIR, { recursive: true });

  runSync(process.execPath, [
    zeroNativeCliPath,
    "package",
    "--target",
    resolvePackageTarget(),
    "--manifest",
    path.join(DESKTOP_DIR, "app.zon"),
    "--binary",
    NATIVE_BINARY,
    "--assets",
    DESKTOP_RESOURCES_DIR,
    "--output",
    DESKTOP_PACKAGE_DIR,
    "--signing",
    "none",
  ], {
    cwd: DESKTOP_DIR,
    env: withLocalBinPath(),
  });

  ensureBuilderArtifacts();
};

if (require.main === module) {
  const startedAt = Date.now();

  try {
    console.log(`Packaging Zero Native desktop app at ${new Date(startedAt).toISOString()}...`);
    ensureDesktopWebBuild();
    stageDesktopResources();
    buildNativeApp();
    packageDesktopApp();
    const elapsed = ((Date.now() - startedAt) / 1000).toFixed(1);
    console.log(`Zero Native package step complete in ${elapsed}s`);
  } catch (error) {
    console.error(error.message ?? error);
    process.exit(1);
  }
}

module.exports = {
  DESKTOP_BUILD_DIST_DIR,
  DESKTOP_DIR,
  DESKTOP_PACKAGE_DIR,
  DESKTOP_RESOURCES_DIR,
  NATIVE_BINARY,
  ROOT_DIR,
  WEB_DIR,
  WEB_BUILD_DIR,
  buildNativeApp,
  ensureBuilderArtifacts,
  ensureDesktopWebBuild,
  packageDesktopApp,
  runSync,
  stageDesktopResources,
};

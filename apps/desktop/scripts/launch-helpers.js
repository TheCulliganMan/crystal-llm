"use strict";

const net = require("node:net");
const fs = require("node:fs");
const path = require("node:path");

const DEFAULT_DESKTOP_HOST = "127.0.0.1";
const DEFAULT_DESKTOP_PORT = 3000;
const ROOT_DIR = path.resolve(__dirname, "../../..");
const DESKTOP_DIR = path.resolve(__dirname, "..");
const DESKTOP_ASSETS_DIR = path.join(DESKTOP_DIR, "assets");
const DESKTOP_BUILD_DIST_DIR = ".next-desktop";
const DESKTOP_RESOURCES_DIR = path.join(DESKTOP_DIR, "dist", "resources");
const DESKTOP_PACKAGE_DIR = path.join(DESKTOP_DIR, "dist", "KrabbyClaw.app");
const SESSION_ID_REGEX = /^[a-zA-Z0-9_-]{1,64}$/;

const resolveInstalledPackageJsonPath = (specifier, resolver = require.resolve) =>
  resolver(`${specifier}/package.json`);

const resolveInstalledBinPath = (specifier, binName, resolver = require.resolve) => {
  try {
    const packageJsonPath = resolveInstalledPackageJsonPath(specifier, resolver);
    const packageDir = path.dirname(packageJsonPath);
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
    const binField = packageJson.bin;
    const relativeBinPath =
      typeof binField === "string"
        ? binField
        : binField && typeof binField === "object"
          ? binField[binName] ?? binField[specifier]
          : null;

    if (!relativeBinPath) {
      throw new Error(`Package ${specifier} does not declare a bin entry for ${binName}.`);
    }

    return path.resolve(packageDir, relativeBinPath);
  } catch (error) {
    const message = `${specifier} is not installed. Run npm install at the repo root before using the desktop app.`;
    const wrappedError = new Error(message);
    wrappedError.cause = error;
    throw wrappedError;
  }
};

const resolveZeroNativeCliPath = (resolver) =>
  resolveInstalledBinPath("zero-native", "zero-native", resolver);

const getDesktopUrl = ({
  host = DEFAULT_DESKTOP_HOST,
  port = DEFAULT_DESKTOP_PORT,
  sessionId,
} = {}) => {
  const url = new URL(`http://${host}:${port}/desktop`);
  if (sessionId && SESSION_ID_REGEX.test(sessionId)) {
    url.searchParams.set("session_id", sessionId);
  }
  return url.toString();
};

const resolveDesktopLaunchSessionId = ({
  argv = process.argv,
  env = process.env,
} = {}) => {
  const envSessionId = env.KRABBY_DESKTOP_SESSION_ID ?? env.POKECRYSTAL_SESSION_ID;
  if (envSessionId && SESSION_ID_REGEX.test(envSessionId)) {
    return envSessionId;
  }

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--session-id" || arg === "--session_id") {
      const next = argv[index + 1];
      return next && SESSION_ID_REGEX.test(next) ? next : null;
    }
    const inlineMatch = arg.match(/^--session[-_]id=(.+)$/);
    if (inlineMatch) {
      return SESSION_ID_REGEX.test(inlineMatch[1]) ? inlineMatch[1] : null;
    }
  }

  return null;
};

const isPortAvailable = (port, host = DEFAULT_DESKTOP_HOST) =>
  new Promise((resolve) => {
    const server = net.createServer();
    server.unref();
    server.once("error", () => {
      resolve(false);
    });
    server.listen({ port, host }, () => {
      server.close(() => {
        resolve(true);
      });
    });
  });

const findAvailablePort = async ({
  startPort = DEFAULT_DESKTOP_PORT,
  host = DEFAULT_DESKTOP_HOST,
  maxAttempts = 25,
  portChecker = isPortAvailable,
} = {}) => {
  for (let candidate = startPort; candidate < startPort + maxAttempts; candidate += 1) {
    if (await portChecker(candidate, host)) {
      return candidate;
    }
  }

  throw new Error(
    `Could not find an available port for the desktop shell starting at ${startPort}.`,
  );
};

const getDesktopIconPngPath = (baseDir = DESKTOP_ASSETS_DIR) =>
  path.join(baseDir, "icon.png");

module.exports = {
  DEFAULT_DESKTOP_HOST,
  DEFAULT_DESKTOP_PORT,
  DESKTOP_BUILD_DIST_DIR,
  DESKTOP_DIR,
  DESKTOP_RESOURCES_DIR,
  DESKTOP_PACKAGE_DIR,
  DESKTOP_ASSETS_DIR,
  ROOT_DIR,
  findAvailablePort,
  getDesktopUrl,
  getDesktopIconPngPath,
  resolveDesktopLaunchSessionId,
  isPortAvailable,
  resolveZeroNativeCliPath,
  resolveInstalledBinPath,
  resolveInstalledPackageJsonPath,
};

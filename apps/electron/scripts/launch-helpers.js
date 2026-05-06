"use strict";

const net = require("node:net");
const fs = require("node:fs");
const path = require("node:path");

const DEFAULT_DESKTOP_HOST = "127.0.0.1";
const DEFAULT_DESKTOP_PORT = 3000;
const ROOT_DIR = path.resolve(__dirname, "../../..");
const ELECTRON_DIR = path.resolve(__dirname, "..");
const ELECTRON_ASSETS_DIR = path.join(ELECTRON_DIR, "assets");
const DESKTOP_BUILD_DIST_DIR = ".next-electron";
const PACKAGED_STANDALONE_DIR = "web-standalone";
const DESKTOP_LOCAL_IDENTITY_SECRET = "krabbyclaw-desktop-local-identity-secret";
const DESKTOP_LOCAL_SESSION_SECRET = "krabbyclaw-desktop-local-session-secret";
const SESSION_ID_REGEX = /^[a-zA-Z0-9_-]{1,64}$/;
const DESKTOP_VOLATILE_STORAGE_CLEAR_OPTIONS = {
  storages: ["serviceworkers", "cachestorage"],
};

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
    const message =
      specifier === "electron"
        ? "Electron is not installed. Run npm install at the repo root before launching."
        : "electron-builder is not installed. Run npm install at the repo root before packaging.";
    const wrappedError = new Error(message);
    wrappedError.cause = error;
    throw wrappedError;
  }
};

const resolveElectronCliPath = (resolver) =>
  resolveInstalledBinPath("electron", "electron", resolver);

const resolveElectronBuilderCliPath = (resolver) =>
  resolveInstalledBinPath("electron-builder", "electron-builder", resolver);

const getPackagedWebRoot = (resourcesPath = process.resourcesPath) =>
  path.join(resourcesPath, "web");

const getPackagedStandaloneRoot = (resourcesPath = process.resourcesPath) =>
  path.join(resourcesPath, PACKAGED_STANDALONE_DIR);

const findStandaloneServerEntrypoint = (standaloneRoot) => {
  const queue = [standaloneRoot];

  while (queue.length > 0) {
    const current = queue.shift();
    const entries = fs.readdirSync(current, { withFileTypes: true });
    for (const entry of entries) {
      const entryPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        queue.push(entryPath);
        continue;
      }
      if (entry.isFile() && entry.name === "server.js") {
        return entryPath;
      }
    }
  }

  throw new Error(`Could not find a packaged standalone server entrypoint under ${standaloneRoot}.`);
};

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

const buildPackagedServerEnv = ({
  host = DEFAULT_DESKTOP_HOST,
  port = DEFAULT_DESKTOP_PORT,
  env = process.env,
} = {}) => ({
  ...env,
  ELECTRON_RUN_AS_NODE: "1",
  HOSTNAME: host,
  NODE_ENV: "production",
  POKECRYSTAL_IDENTITY_SECRET: DESKTOP_LOCAL_IDENTITY_SECRET,
  POKECRYSTAL_NEXT_DIST_DIR: DESKTOP_BUILD_DIST_DIR,
  POKECRYSTAL_REQUIRE_SESSION_SECRET: "false",
  POKECRYSTAL_SESSION_SECRET: DESKTOP_LOCAL_SESSION_SECRET,
  PORT: String(port),
});

const getDesktopIconPngPath = (baseDir = ELECTRON_ASSETS_DIR) =>
  path.join(baseDir, "icon.png");

const getPackagedDesktopIconPath = (resourcesPath = process.resourcesPath) =>
  path.join(resourcesPath, "icon.png");

const resolvePackagedServerCommand = ({
  execPath = process.execPath,
  helperExecPath = process.helperExecPath,
} = {}) => helperExecPath || execPath;

const getPackagedServerLaunchConfig = ({
  resourcesPath = process.resourcesPath,
  host = DEFAULT_DESKTOP_HOST,
  port = DEFAULT_DESKTOP_PORT,
  env = process.env,
  execPath = process.execPath,
  helperExecPath = process.helperExecPath,
} = {}) => {
  const standaloneRoot = getPackagedStandaloneRoot(resourcesPath);
  const serverEntrypoint = findStandaloneServerEntrypoint(standaloneRoot);
  return {
    command: resolvePackagedServerCommand({ execPath, helperExecPath }),
    args: [serverEntrypoint],
    options: {
      cwd: standaloneRoot,
      env: buildPackagedServerEnv({ host, port, env }),
      stdio: ["ignore", "ignore", "ignore"],
    },
  };
};

const clearDesktopRuntimeCaches = async (desktopSession) => {
  await desktopSession.clearCache();
  if (typeof desktopSession.clearCodeCaches === "function") {
    await desktopSession.clearCodeCaches({ urls: [] });
  }
  await desktopSession.clearStorageData(DESKTOP_VOLATILE_STORAGE_CLEAR_OPTIONS);
};

module.exports = {
  DEFAULT_DESKTOP_HOST,
  DEFAULT_DESKTOP_PORT,
  DESKTOP_VOLATILE_STORAGE_CLEAR_OPTIONS,
  DESKTOP_BUILD_DIST_DIR,
  DESKTOP_LOCAL_IDENTITY_SECRET,
  DESKTOP_LOCAL_SESSION_SECRET,
  ELECTRON_ASSETS_DIR,
  ROOT_DIR,
  buildPackagedServerEnv,
  clearDesktopRuntimeCaches,
  findAvailablePort,
  getDesktopUrl,
  getDesktopIconPngPath,
  getPackagedDesktopIconPath,
  getPackagedServerLaunchConfig,
  getPackagedStandaloneRoot,
  getPackagedWebRoot,
  resolveDesktopLaunchSessionId,
  isPortAvailable,
  findStandaloneServerEntrypoint,
  resolvePackagedServerCommand,
  resolveElectronBuilderCliPath,
  resolveElectronCliPath,
  resolveInstalledBinPath,
  resolveInstalledPackageJsonPath,
};

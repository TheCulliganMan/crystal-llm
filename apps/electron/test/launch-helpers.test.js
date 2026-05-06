const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");

const {
  buildPackagedServerEnv,
  clearDesktopRuntimeCaches,
  DESKTOP_VOLATILE_STORAGE_CLEAR_OPTIONS,
  findStandaloneServerEntrypoint,
  getDesktopUrl,
  getDesktopIconPngPath,
  getPackagedServerLaunchConfig,
  getPackagedStandaloneRoot,
  findAvailablePort,
  resolveElectronBuilderCliPath,
  resolveElectronCliPath,
  resolveDesktopLaunchSessionId,
  resolveInstalledBinPath,
  resolvePackagedServerCommand,
  DESKTOP_LOCAL_IDENTITY_SECRET,
  DESKTOP_LOCAL_SESSION_SECRET,
} = require("../scripts/launch-helpers");

const makeResourcesPath = () => fs.mkdtempSync(path.join(os.tmpdir(), "krabby-electron-"));
const makePackageResolver = (packageRoot) => (specifier) => {
  if (specifier.endsWith("/package.json")) {
    return path.join(packageRoot, "package.json");
  }
  throw new Error(`Unexpected specifier: ${specifier}`);
};

test("resolves installed CLI bin paths through an injected resolver", () => {
  const electronRoot = makeResourcesPath();
  const builderRoot = makeResourcesPath();

  fs.writeFileSync(
    path.join(electronRoot, "package.json"),
    JSON.stringify({ name: "electron", bin: { electron: "cli.js" } }),
  );
  fs.writeFileSync(
    path.join(builderRoot, "package.json"),
    JSON.stringify({ name: "electron-builder", bin: { "electron-builder": "cli.js" } }),
  );

  assert.equal(
    resolveElectronCliPath(makePackageResolver(electronRoot)),
    path.join(electronRoot, "cli.js"),
  );
  assert.equal(
    resolveElectronBuilderCliPath(makePackageResolver(builderRoot)),
    path.join(builderRoot, "cli.js"),
  );
});

test("throws a helpful error when a CLI package is missing", () => {
  assert.throws(() => resolveInstalledBinPath("electron", "electron", () => {
    throw new Error("missing");
  }), /Electron is not installed/);
});

test("finds the packaged standalone server entrypoint", () => {
  const resourcesPath = makeResourcesPath();
  const standaloneRoot = path.join(resourcesPath, "web-standalone");
  const nestedServerPath = path.join(standaloneRoot, "apps", "web", "server.js");

  fs.mkdirSync(path.dirname(nestedServerPath), { recursive: true });
  fs.writeFileSync(nestedServerPath, "console.log('server');");

  assert.equal(getPackagedStandaloneRoot(resourcesPath), standaloneRoot);
  assert.equal(findStandaloneServerEntrypoint(standaloneRoot), nestedServerPath);
});

test("prefers the helper executable for packaged background server launches", () => {
  assert.equal(
    resolvePackagedServerCommand({
      execPath: "/Applications/KrabbyClaw Desktop.app/Contents/MacOS/KrabbyClaw Desktop",
      helperExecPath: "/Applications/KrabbyClaw Desktop.app/Contents/Frameworks/KrabbyClaw Desktop Helper.app/Contents/MacOS/KrabbyClaw Desktop Helper",
    }),
    "/Applications/KrabbyClaw Desktop.app/Contents/Frameworks/KrabbyClaw Desktop Helper.app/Contents/MacOS/KrabbyClaw Desktop Helper",
  );
  assert.equal(
    resolvePackagedServerCommand({
      execPath: "/Applications/KrabbyClaw Desktop.app/Contents/MacOS/KrabbyClaw Desktop",
      helperExecPath: "",
    }),
    "/Applications/KrabbyClaw Desktop.app/Contents/MacOS/KrabbyClaw Desktop",
  );
});

test("builds the packaged server launch config with the expected env", () => {
  const resourcesPath = makeResourcesPath();
  const standaloneRoot = path.join(resourcesPath, "web-standalone");
  const serverEntrypoint = path.join(standaloneRoot, "server.js");

  fs.mkdirSync(standaloneRoot, { recursive: true });
  fs.writeFileSync(serverEntrypoint, "console.log('server');");

  const launchConfig = getPackagedServerLaunchConfig({
    resourcesPath,
    host: "127.0.0.1",
    port: 3100,
    env: { FOO: "bar" },
    execPath: "/Applications/KrabbyClaw Desktop.app/Contents/MacOS/KrabbyClaw Desktop",
    helperExecPath: "/Applications/KrabbyClaw Desktop.app/Contents/Frameworks/KrabbyClaw Desktop Helper.app/Contents/MacOS/KrabbyClaw Desktop Helper",
  });

  assert.equal(
    launchConfig.command,
    "/Applications/KrabbyClaw Desktop.app/Contents/Frameworks/KrabbyClaw Desktop Helper.app/Contents/MacOS/KrabbyClaw Desktop Helper",
  );
  assert.deepEqual(launchConfig.args, [serverEntrypoint]);
  assert.equal(launchConfig.options.cwd, standaloneRoot);
  assert.equal(launchConfig.options.env.ELECTRON_RUN_AS_NODE, "1");
  assert.equal(launchConfig.options.env.HOSTNAME, "127.0.0.1");
  assert.equal(launchConfig.options.env.NODE_ENV, "production");
  assert.equal(launchConfig.options.env.POKECRYSTAL_IDENTITY_SECRET, DESKTOP_LOCAL_IDENTITY_SECRET);
  assert.equal(launchConfig.options.env.POKECRYSTAL_NEXT_DIST_DIR, ".next-electron");
  assert.equal(launchConfig.options.env.POKECRYSTAL_REQUIRE_SESSION_SECRET, "false");
  assert.equal(launchConfig.options.env.POKECRYSTAL_SESSION_SECRET, DESKTOP_LOCAL_SESSION_SECRET);
  assert.equal(launchConfig.options.env.PORT, "3100");
  assert.equal(launchConfig.options.env.FOO, "bar");
});

test("formats the desktop URL for the local shell", () => {
  assert.equal(getDesktopUrl({ host: "127.0.0.1", port: 3000 }), "http://127.0.0.1:3000/desktop");
  assert.equal(
    getDesktopUrl({ host: "127.0.0.1", port: 3000, sessionId: "testrun" }),
    "http://127.0.0.1:3000/desktop?session_id=testrun",
  );
});

test("resolves desktop launch session ids from args and env", () => {
  assert.equal(resolveDesktopLaunchSessionId({ argv: ["app", "--session-id", "testrun"], env: {} }), "testrun");
  assert.equal(resolveDesktopLaunchSessionId({ argv: ["app", "--session_id=other-run"], env: {} }), "other-run");
  assert.equal(
    resolveDesktopLaunchSessionId({
      argv: ["app", "--session-id", "arg-run"],
      env: { KRABBY_DESKTOP_SESSION_ID: "env-run" },
    }),
    "env-run",
  );
  assert.equal(resolveDesktopLaunchSessionId({ argv: ["app", "--session-id", "bad session"], env: {} }), null);
});

test("builds an isolated env map without mutating the caller env", () => {
  const env = { A: "1" };
  const snapshot = buildPackagedServerEnv({ env, host: "localhost", port: 3001 });

  assert.equal(snapshot.A, "1");
  assert.equal(snapshot.ELECTRON_RUN_AS_NODE, "1");
  assert.equal(snapshot.HOSTNAME, "localhost");
  assert.equal(snapshot.POKECRYSTAL_IDENTITY_SECRET, DESKTOP_LOCAL_IDENTITY_SECRET);
  assert.equal(snapshot.POKECRYSTAL_NEXT_DIST_DIR, ".next-electron");
  assert.equal(snapshot.POKECRYSTAL_REQUIRE_SESSION_SECRET, "false");
  assert.equal(snapshot.POKECRYSTAL_SESSION_SECRET, DESKTOP_LOCAL_SESSION_SECRET);
  assert.equal(snapshot.PORT, "3001");
  assert.equal(snapshot.NODE_ENV, "production");
  assert.equal(env.PORT, undefined);
});

test("clears volatile desktop runtime caches without clearing saved app data", async () => {
  const calls = [];
  const fakeSession = {
    clearCache: async () => {
      calls.push(["clearCache"]);
    },
    clearCodeCaches: async (options) => {
      calls.push(["clearCodeCaches", options]);
    },
    clearStorageData: async (options) => {
      calls.push(["clearStorageData", options]);
    },
  };

  await clearDesktopRuntimeCaches(fakeSession);

  assert.deepEqual(calls, [
    ["clearCache"],
    ["clearCodeCaches", { urls: [] }],
    ["clearStorageData", DESKTOP_VOLATILE_STORAGE_CLEAR_OPTIONS],
  ]);
  assert.deepEqual(DESKTOP_VOLATILE_STORAGE_CLEAR_OPTIONS.storages, [
    "serviceworkers",
    "cachestorage",
  ]);
});

test("returns the generated desktop icon path", () => {
  assert.equal(getDesktopIconPngPath("/tmp/electron-assets"), "/tmp/electron-assets/icon.png");
});

test("finds the first available port in the requested range", async () => {
  const probes = [];

  const port = await findAvailablePort({
    startPort: 3000,
    maxAttempts: 3,
    portChecker: async (candidate) => {
      probes.push(candidate);
      return candidate === 3001;
    },
  });

  assert.equal(port, 3001);
  assert.deepEqual(probes, [3000, 3001]);
});

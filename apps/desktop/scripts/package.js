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
const { patchZeroNativeMenu } = require("./patch-zero-native-menu");

const WEB_DIR = path.resolve(DESKTOP_DIR, "../web");
const NPM_COMMAND = process.platform === "win32" ? "npm.cmd" : "npm";
const WEB_BUILD_DIR = path.join(WEB_DIR, DESKTOP_BUILD_DIST_DIR);
const DESKTOP_HTML_PATH = path.join(WEB_BUILD_DIR, "server", "app", "desktop.html");
const DESKTOP_STANDALONE_DIR = path.join(WEB_BUILD_DIR, "standalone");
const DESKTOP_ENTRY_HTML = `\
<script>
if (location.pathname !== "/desktop") {
  history.replaceState(null, "", "/desktop" + location.search + location.hash);
}
</script>`;
const NATIVE_BINARY = path.join(DESKTOP_DIR, "zig-out", "bin", process.platform === "win32" ? "krabbyclaw-desktop.exe" : "krabbyclaw-desktop");
const PACKAGED_NATIVE_BINARY_NAME = process.platform === "win32" ? "krabbyclaw-native.exe" : "krabbyclaw-native";
const LAUNCHER_BINARY = path.join(DESKTOP_DIR, "dist", "bin", process.platform === "win32" ? "krabbyclaw-desktop.cmd" : "krabbyclaw-desktop");
const NODE_VERSION = "24.1.0";
const NODE_PLATFORM = process.platform === "darwin" ? "darwin" : process.platform === "linux" ? "linux" : null;
const NODE_ARCH = process.arch === "arm64" ? "arm64" : process.arch === "x64" ? "x64" : null;
const BUNDLED_NODE_DIR = NODE_PLATFORM && NODE_ARCH
  ? path.join(ROOT_DIR, ".node", `node-v${NODE_VERSION}-${NODE_PLATFORM}-${NODE_ARCH}`)
  : null;
const BUNDLED_NODE_BINARY = BUNDLED_NODE_DIR
  ? path.join(BUNDLED_NODE_DIR, "bin", process.platform === "win32" ? "node.exe" : "node")
  : null;

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
      POKECRYSTAL_NEXT_OUTPUT: "standalone",
    },
  });
};

const ensureBundledNodeRuntime = () => {
  if (!BUNDLED_NODE_DIR || !BUNDLED_NODE_BINARY) {
    throw new Error(`Unsupported packaged Node runtime target: ${process.platform}/${process.arch}.`);
  }
  if (fs.existsSync(BUNDLED_NODE_BINARY)) {
    return BUNDLED_NODE_BINARY;
  }

  const archiveName = `node-v${NODE_VERSION}-${NODE_PLATFORM}-${NODE_ARCH}.tar.gz`;
  const archiveUrl = `https://nodejs.org/dist/v${NODE_VERSION}/${archiveName}`;
  const archivePath = path.join(ROOT_DIR, ".node", archiveName);
  fs.mkdirSync(path.dirname(archivePath), { recursive: true });
  runSync("curl", ["-fsSL", archiveUrl, "-o", archivePath]);
  runSync("tar", ["-xzf", archivePath, "-C", path.dirname(BUNDLED_NODE_DIR)]);

  if (!fs.existsSync(BUNDLED_NODE_BINARY)) {
    throw new Error(`Node runtime download did not create ${BUNDLED_NODE_BINARY}.`);
  }
  return BUNDLED_NODE_BINARY;
};

const copyWithoutHeavyWebAssets = (source, destination) => {
  fs.cpSync(source, destination, {
    recursive: true,
    filter: (entry) => {
      const relative = path.relative(source, entry);
      return !relative.split(path.sep).some((part) => part === "downloads" || part === "ffmpeg" || part === "__tests__");
    },
  });
};

const stageStandaloneServer = () => {
  if (!fs.existsSync(DESKTOP_STANDALONE_DIR)) {
    throw new Error(`Expected Next standalone server at ${DESKTOP_STANDALONE_DIR}.`);
  }

  const standaloneTarget = path.join(DESKTOP_RESOURCES_DIR, "web-standalone");
  fs.cpSync(DESKTOP_STANDALONE_DIR, standaloneTarget, { recursive: true });

  const standaloneWebDir = path.join(standaloneTarget, "apps", "web");
  fs.mkdirSync(path.join(standaloneWebDir, DESKTOP_BUILD_DIST_DIR), { recursive: true });
  fs.cpSync(path.join(WEB_BUILD_DIR, "static"), path.join(standaloneWebDir, DESKTOP_BUILD_DIST_DIR, "static"), {
    recursive: true,
  });
  copyWithoutHeavyWebAssets(path.join(WEB_DIR, "public"), path.join(standaloneWebDir, "public"));
  copyWithoutHeavyWebAssets(path.join(WEB_DIR, "assets"), path.join(standaloneWebDir, "assets"));

  const disassemblyTarget = path.join(standaloneTarget, "vendor", "pokecrystal");
  fs.mkdirSync(disassemblyTarget, { recursive: true });
  fs.cpSync(path.join(ROOT_DIR, "vendor", "pokecrystal", "audio"), path.join(disassemblyTarget, "audio"), {
    recursive: true,
  });
  fs.cpSync(path.join(ROOT_DIR, "vendor", "pokecrystal", "constants"), path.join(disassemblyTarget, "constants"), {
    recursive: true,
  });
};

const stageBundledNodeRuntime = () => {
  ensureBundledNodeRuntime();
};

const copyBundledNodeRuntimeToPackage = () => {
  const nodeBinary = ensureBundledNodeRuntime();
  const target = path.join(DESKTOP_PACKAGE_DIR, "Contents", "Resources", "dist", "resources", "node", "bin", path.basename(nodeBinary));
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.copyFileSync(nodeBinary, target);
  fs.chmodSync(target, 0o755);
};

const writeDesktopLauncherScript = () => {
  const launcherPath = path.join(DESKTOP_RESOURCES_DIR, "desktop-launcher.mjs");
  fs.writeFileSync(launcherPath, `\
import http from "node:http";
import net from "node:net";
import { spawn } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const resourceRoot = dirname(fileURLToPath(import.meta.url));
const nativeBinary = process.argv[2];
const nativeArgs = process.argv.slice(3);
const host = "127.0.0.1";
const port = 37631;
const nodePath = join(resourceRoot, "node", "bin", "node");
const serverPath = join(resourceRoot, "web-standalone", "apps", "web", "server.js");
const serverCwd = dirname(serverPath);
const disassemblyRoot = join(resourceRoot, "web-standalone", "vendor", "pokecrystal");
let serverProcess = null;
let nativeProcess = null;

const isPortAvailable = (port) =>
  new Promise((resolve) => {
    const server = net.createServer();
    server.unref();
    server.once("error", () => resolve(false));
    server.listen({ host, port }, () => server.close(() => resolve(true)));
  });

const assertPortAvailable = async () => {
  if (!(await isPortAvailable(port))) {
    throw new Error(\`Desktop server port \${port} is already in use.\`);
  }
};

const waitForServer = (url, deadlineMs = 20000) =>
  new Promise((resolve, reject) => {
    const startedAt = Date.now();
    const retry = () => {
      if (Date.now() - startedAt > deadlineMs) {
        reject(new Error(\`Desktop server did not become ready at \${url}.\`));
        return;
      }
      setTimeout(poll, 150);
    };
    const poll = () => {
      const request = http.get(url, (response) => {
        response.resume();
        if (response.statusCode && response.statusCode < 500) {
          resolve();
          return;
        }
        retry();
      });
      request.on("error", retry);
      request.setTimeout(1000, () => {
        request.destroy();
        retry();
      });
    };
    poll();
  });

const shutdown = () => {
  if (nativeProcess && !nativeProcess.killed) {
    nativeProcess.kill();
  }
  if (serverProcess && !serverProcess.killed) {
    serverProcess.kill();
  }
};

process.on("SIGINT", () => {
  shutdown();
  process.exit(130);
});
process.on("SIGTERM", () => {
  shutdown();
  process.exit(143);
});

const runNative = (env) =>
  new Promise((resolve, reject) => {
    nativeProcess = spawn(nativeBinary, nativeArgs, { stdio: "inherit", env });
    nativeProcess.on("error", reject);
    nativeProcess.on("exit", (code, signal) => {
      shutdown();
      if (signal) {
        process.kill(process.pid, signal);
        return;
      }
      resolve(code ?? 0);
    });
  });

try {
  const externalUrl = process.env.KRABBY_DESKTOP_URL || process.env.ZERO_NATIVE_FRONTEND_URL;
  if (externalUrl) {
    process.exit(await runNative({ ...process.env, KRABBY_DESKTOP_URL: externalUrl, ZERO_NATIVE_FRONTEND_URL: externalUrl }));
  }

  const url = \`http://\${host}:\${port}/desktop\`;
  await assertPortAvailable();
  serverProcess = spawn(nodePath, [serverPath], {
    cwd: serverCwd,
    stdio: "ignore",
    env: {
      ...process.env,
      HOSTNAME: host,
      PORT: String(port),
      NODE_ENV: "production",
      POKECRYSTAL_NEXT_DIST_DIR: ".next-desktop",
      POKECRYSTAL_DISASSEMBLY_ROOT: disassemblyRoot,
      POKECRYSTAL_REQUIRE_SESSION_SECRET: "false",
      POKECRYSTAL_IDENTITY_SECRET: process.env.POKECRYSTAL_IDENTITY_SECRET || "krabbyclaw-desktop-local-identity-secret",
      POKECRYSTAL_SESSION_SECRET: process.env.POKECRYSTAL_SESSION_SECRET || "krabbyclaw-desktop-local-session-secret",
    },
  });
  serverProcess.on("error", (error) => {
    throw error;
  });
  await waitForServer(url);
  process.exit(await runNative({ ...process.env, KRABBY_DESKTOP_URL: url, ZERO_NATIVE_FRONTEND_URL: url }));
} catch (error) {
  shutdown();
  console.error(error);
  process.exit(1);
}
`);
};

const writeAppExecutableLauncher = () => {
  fs.mkdirSync(path.dirname(LAUNCHER_BINARY), { recursive: true });
  fs.writeFileSync(LAUNCHER_BINARY, `\
#!/bin/sh
set -eu
macos_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$macos_dir/../Resources/dist/resources/node/bin/node" "$macos_dir/../Resources/dist/resources/desktop-launcher.mjs" "$macos_dir/${PACKAGED_NATIVE_BINARY_NAME}" "$@"
`);
  fs.chmodSync(LAUNCHER_BINARY, 0o755);
};

const stageDesktopResources = () => {
  if (!fs.existsSync(DESKTOP_HTML_PATH)) {
    throw new Error(`Expected static desktop HTML at ${DESKTOP_HTML_PATH}.`);
  }

  fs.rmSync(DESKTOP_RESOURCES_DIR, { recursive: true, force: true });
  fs.mkdirSync(DESKTOP_RESOURCES_DIR, { recursive: true });

  stageStandaloneServer();
  stageBundledNodeRuntime();
  writeDesktopLauncherScript();

  fs.cpSync(path.join(WEB_BUILD_DIR, "static"), path.join(DESKTOP_RESOURCES_DIR, "_next", "static"), {
    recursive: true,
  });
  copyWithoutHeavyWebAssets(path.join(WEB_DIR, "public"), DESKTOP_RESOURCES_DIR);
  copyWithoutHeavyWebAssets(path.join(WEB_DIR, "assets"), path.join(DESKTOP_RESOURCES_DIR, "assets"));
  const desktopHtml = fs
    .readFileSync(DESKTOP_HTML_PATH, "utf8")
    .replace("<head>", `<head>${DESKTOP_ENTRY_HTML}`);
  fs.mkdirSync(path.join(DESKTOP_RESOURCES_DIR, "desktop"), { recursive: true });
  fs.writeFileSync(path.join(DESKTOP_RESOURCES_DIR, "desktop", "index.html"), desktopHtml);
  fs.writeFileSync(path.join(DESKTOP_RESOURCES_DIR, "index.html"), desktopHtml);
  fs.cpSync(path.join(DESKTOP_DIR, "assets", "icon.png"), path.join(DESKTOP_RESOURCES_DIR, "icon.png"));

  const stagedHtml = fs.readFileSync(path.join(DESKTOP_RESOURCES_DIR, "desktop", "index.html"), "utf8");
  if (!stagedHtml.includes("/_next/static/") || stagedHtml.includes("../dist/index.js")) {
    throw new Error("Staged desktop index.html is not the prerendered Next /desktop page.");
  }
  if (!stagedHtml.includes('history.replaceState(null, "", "/desktop"')) {
    throw new Error("Staged desktop index.html is missing the packaged route normalization script.");
  }
  if (!fs.existsSync(path.join(DESKTOP_RESOURCES_DIR, "assets", "data", "pokegear_landmarks.json"))) {
    throw new Error("Staged desktop resources are missing generated runtime assets.");
  }
  if (!fs.existsSync(path.join(DESKTOP_RESOURCES_DIR, "web-standalone", "apps", "web", "server.js"))) {
    throw new Error("Staged desktop resources are missing the Next standalone server.");
  }
  if (!fs.existsSync(path.join(DESKTOP_RESOURCES_DIR, "web-standalone", "vendor", "pokecrystal", "audio", "sfx.asm"))) {
    throw new Error("Staged desktop resources are missing disassembly audio sources.");
  }
};

const buildNativeApp = () => {
  patchZeroNativeMenu();
  runSync("zig", ["build"], {
    cwd: DESKTOP_DIR,
    env: withLocalBinPath(),
  });
};

const ensureBuilderArtifacts = () => {
  if (!fs.existsSync(DESKTOP_PACKAGE_DIR)) {
    throw new Error(`Zero Native packaging completed without creating ${DESKTOP_PACKAGE_DIR}.`);
  }

  const expectedBundleFiles = [
    path.join(DESKTOP_PACKAGE_DIR, "Contents", "Info.plist"),
    path.join(DESKTOP_PACKAGE_DIR, "Contents", "MacOS", "krabbyclaw-desktop"),
    path.join(DESKTOP_PACKAGE_DIR, "Contents", "MacOS", PACKAGED_NATIVE_BINARY_NAME),
    path.join(DESKTOP_PACKAGE_DIR, "Contents", "Resources", "dist", "resources", "desktop-launcher.mjs"),
    path.join(DESKTOP_PACKAGE_DIR, "Contents", "Resources", "dist", "resources", "node", "bin", "node"),
    path.join(DESKTOP_PACKAGE_DIR, "Contents", "Resources", "dist", "resources", "web-standalone", "apps", "web", "server.js"),
    path.join(DESKTOP_PACKAGE_DIR, "Contents", "Resources", "dist", "resources", "web-standalone", "vendor", "pokecrystal", "audio", "sfx.asm"),
    path.join(DESKTOP_PACKAGE_DIR, "Contents", "Resources", "dist", "resources", "desktop", "index.html"),
    path.join(DESKTOP_PACKAGE_DIR, "Contents", "Resources", "dist", "resources", "assets", "data", "pokegear_landmarks.json"),
  ];

  for (const expectedFile of expectedBundleFiles) {
    if (!fs.existsSync(expectedFile)) {
      throw new Error(`Zero Native packaging completed without creating ${expectedFile}.`);
    }
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
  writeAppExecutableLauncher();

  runSync(process.execPath, [
    zeroNativeCliPath,
    "package",
    "--target",
    resolvePackageTarget(),
    "--manifest",
    path.join(DESKTOP_DIR, "app.zon"),
    "--binary",
    LAUNCHER_BINARY,
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

  const packagedNativeBinary = path.join(DESKTOP_PACKAGE_DIR, "Contents", "MacOS", PACKAGED_NATIVE_BINARY_NAME);
  fs.copyFileSync(NATIVE_BINARY, packagedNativeBinary);
  fs.chmodSync(packagedNativeBinary, 0o755);
  copyBundledNodeRuntimeToPackage();

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

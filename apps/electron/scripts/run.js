/* eslint-disable no-console */
const { spawn } = require("node:child_process");
const { setTimeout: sleep } = require("node:timers/promises");
const path = require("node:path");
const {
  DEFAULT_DESKTOP_HOST,
  DEFAULT_DESKTOP_PORT,
  DESKTOP_BUILD_DIST_DIR,
  findAvailablePort,
  getDesktopUrl,
  resolveDesktopLaunchSessionId,
  resolveElectronCliPath,
} = require("./launch-helpers");

const ROOT_DIR = path.resolve(__dirname, "../../..");
const NPM_COMMAND = process.platform === "win32" ? "npm.cmd" : "npm";
const MAX_WAIT_MS = 120_000;
const POLL_INTERVAL_MS = 250;

const runCommand = (command, args, options = {}) => {
  const child = spawn(command, args, {
    cwd: ROOT_DIR,
    stdio: ["ignore", "inherit", "inherit"],
    env: process.env,
    shell: process.platform === "win32",
    ...options,
  });

  return child;
};

const waitForServer = async (targetUrl) => {
  const deadline = Date.now() + MAX_WAIT_MS;

  while (Date.now() < deadline) {
    try {
      const response = await fetch(targetUrl, { signal: AbortSignal.timeout(1000) });
      if (response.ok || response.status >= 300) {
        return true;
      }
    } catch {
      // still booting
    }

    await sleep(POLL_INTERVAL_MS);
  }

  throw new Error(`Timed out waiting for ${targetUrl}`);
};

let nextProcess = null;
let electronProcess = null;

const cleanup = () => {
  if (electronProcess && !electronProcess.killed) {
    electronProcess.kill();
  }
  if (nextProcess && !nextProcess.killed) {
    nextProcess.kill("SIGINT");
  }
};

const exitWithCleanup = (code = 0) => {
  cleanup();
  process.exit(code);
};

process.on("SIGINT", () => {
  exitWithCleanup(0);
});

process.on("SIGTERM", () => {
  exitWithCleanup(0);
});

const run = async () => {
  const port = await findAvailablePort({ host: DEFAULT_DESKTOP_HOST, startPort: DEFAULT_DESKTOP_PORT });
  const targetUrl = getDesktopUrl({
    host: DEFAULT_DESKTOP_HOST,
    port,
    sessionId: resolveDesktopLaunchSessionId(),
  });

  nextProcess = runCommand(NPM_COMMAND, [
    "run",
    "dev",
    "--workspace",
    "@pokecrystal/web",
    "--",
    "--hostname",
    DEFAULT_DESKTOP_HOST,
    "--port",
    String(port),
  ]);

  await waitForServer(targetUrl);

  electronProcess = runCommand(process.execPath, [resolveElectronCliPath(), path.resolve(__dirname, "../main.js")], {
    env: {
      ...process.env,
      ELECTRON_RUN_AS_NODE: undefined,
      KRABBY_DESKTOP_URL: targetUrl,
      POKECRYSTAL_NEXT_DIST_DIR: DESKTOP_BUILD_DIST_DIR,
    },
  });

  nextProcess.on("exit", (code) => {
    if (code && code !== 0) {
      console.error(`Next.js exited with code ${code}`);
    }
    exitWithCleanup(code ?? 0);
  });

  electronProcess.on("exit", (code) => {
    if (code && code !== 0) {
      console.error(`Electron exited with code ${code}`);
    }
    exitWithCleanup(code ?? 0);
  });
};

run().catch((error) => {
  console.error(error);
  cleanup();
  process.exit(1);
});

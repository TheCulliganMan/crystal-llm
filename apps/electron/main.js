const { app, BrowserWindow, session } = require("electron");
const { spawn } = require("node:child_process");
const fsSync = require("node:fs");
const fs = require("node:fs/promises");
const os = require("node:os");
const path = require("node:path");
const { setTimeout: sleep } = require("node:timers/promises");
const {
  DEFAULT_DESKTOP_HOST,
  DEFAULT_DESKTOP_PORT,
  findAvailablePort,
  getDesktopUrl,
  getDesktopIconPngPath,
  getPackagedDesktopIconPath,
  getPackagedServerLaunchConfig,
  clearDesktopRuntimeCaches,
  resolveDesktopLaunchSessionId,
} = require("./scripts/launch-helpers");

const MAX_WAIT_MS = 120_000;
const POLL_INTERVAL_MS = 250;
const SMOKE_TIMEOUT_MS = 30_000;
const DESKTOP_LOAD_RETRY_LIMIT = 8;
const RETRYABLE_LOAD_ERROR_CODES = new Set([
  -102, // ERR_CONNECTION_REFUSED
  -105, // ERR_NAME_NOT_RESOLVED
  -106, // ERR_INTERNET_DISCONNECTED
  -118, // ERR_CONNECTION_TIMED_OUT
  -300, // ERR_INVALID_URL
]);
const DESKTOP_LOG_PATH = process.env.KRABBY_DESKTOP_LOG_PATH ||
  path.join(os.tmpdir(), "krabbyclaw-desktop-main.log");

let packagedServerProcess = null;
let appIsQuitting = false;

const shouldLaunchPackagedServer = () => app.isPackaged && !process.env.KRABBY_DESKTOP_URL;

const logDesktopEvent = (event, details) => {
  const entry = {
    ts: new Date().toISOString(),
    pid: process.pid,
    event,
    details,
  };

  try {
    fsSync.appendFileSync(DESKTOP_LOG_PATH, `${JSON.stringify(entry)}\n`, "utf8");
  } catch {
    // Ignore logging failures in production startup.
  }
};

const waitForServer = async (url) => {
  const deadline = Date.now() + MAX_WAIT_MS;

  while (Date.now() < deadline) {
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(1000) });
      if (response.ok || response.status >= 300) {
        return;
      }
    } catch {
      // still booting
    }

    await sleep(POLL_INTERVAL_MS);
  }

  throw new Error(`Timed out waiting for ${url}`);
};

const startPackagedServer = async () => {
  const host = DEFAULT_DESKTOP_HOST;
  const port = await findAvailablePort({ host, startPort: DEFAULT_DESKTOP_PORT });
  const desktopUrl = getDesktopUrl({ host, port, sessionId: resolveDesktopLaunchSessionId() });
  const launchConfig = getPackagedServerLaunchConfig({
    host,
    port,
    execPath: process.execPath,
    helperExecPath: process.helperExecPath,
  });

  logDesktopEvent("server-launch", {
    desktopUrl,
    command: launchConfig.command,
    args: launchConfig.args,
  });
  packagedServerProcess = spawn(launchConfig.command, launchConfig.args, launchConfig.options);
  packagedServerProcess.on("exit", (code) => {
    logDesktopEvent("server-exit", { code });
    if (code && code !== 0) {
      console.error(`Packaged Next server exited with code ${code}`);
    }

    if (!appIsQuitting) {
      app.quit();
    }
  });

  return { desktopUrl, port, host };
};

const cleanup = () => {
  if (packagedServerProcess && !packagedServerProcess.killed) {
    packagedServerProcess.kill("SIGINT");
  }
};

const writeSmokeResult = async (payload) => {
  const smokePath = process.env.KRABBY_DESKTOP_SMOKE_PATH;
  if (!smokePath) {
    return;
  }

  await fs.writeFile(smokePath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
};

const attachSmokeProbe = (window) => {
  const smokePath = process.env.KRABBY_DESKTOP_SMOKE_PATH;
  if (!smokePath) {
    return;
  }

  let completed = false;
  let lastStatus = null;
  let lastError = null;

  const finish = async (payload, exitCode) => {
    if (completed) {
      return;
    }
    completed = true;

    try {
      await writeSmokeResult(payload);
    } finally {
      app.exit(exitCode);
    }
  };

  window.webContents.on("did-finish-load", async () => {
    const deadline = Date.now() + SMOKE_TIMEOUT_MS;

    while (Date.now() < deadline) {
      try {
        const status = await window.webContents.executeJavaScript(
          `(() => {
            const shell = document.querySelector('[data-testid="desktop-page-shell"]');
            const canvas = document.querySelector('canvas.playui-screen-canvas');
            return {
              ready: Boolean(shell && canvas),
              hasShell: Boolean(shell),
              hasCanvas: Boolean(canvas),
              href: window.location.href,
              title: document.title,
              bodyPreview: document.body?.innerHTML?.slice(0, 500) ?? "",
            };
          })()`,
          true,
        );
        lastStatus = status;
        lastError = null;

        if (status?.ready) {
          await finish({ ok: true, ...status }, 0);
          return;
        }
      } catch (error) {
        // The page can still be booting while scripts hydrate.
        lastError = error instanceof Error ? error.message : String(error);
      }

      await sleep(POLL_INTERVAL_MS);
    }

    await finish({
      ok: false,
      error: "Timed out waiting for the desktop shell to render.",
      lastStatus,
      lastError,
    }, 1);
  });
};

const createDesktopWindow = (desktopUrl) => {
  const icon = app.isPackaged ? getPackagedDesktopIconPath() : getDesktopIconPngPath();
  const window = new BrowserWindow({
    autoHideMenuBar: true,
    backgroundColor: "#000000",
    icon,
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  let loadAttempts = 0;
  const loadDesktopUrl = async (reason = "initial") => {
    loadAttempts += 1;
    logDesktopEvent("window-load-attempt", { desktopUrl, loadAttempts, reason });
    try {
      await window.loadURL(desktopUrl);
    } catch (error) {
      logDesktopEvent("window-load-exception", {
        desktopUrl,
        loadAttempts,
        reason,
        message: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  };

  window.webContents.on("did-fail-load", async (_event, errorCode, errorDescription, validatedURL, isMainFrame) => {
    logDesktopEvent("window-did-fail-load", {
      errorCode,
      errorDescription,
      validatedURL,
      isMainFrame,
      loadAttempts,
    });

    if (!isMainFrame || validatedURL !== desktopUrl || loadAttempts >= DESKTOP_LOAD_RETRY_LIMIT) {
      return;
    }
    if (!RETRYABLE_LOAD_ERROR_CODES.has(errorCode)) {
      return;
    }

    try {
      await waitForServer(desktopUrl);
      await sleep(500);
      if (!window.isDestroyed()) {
        await loadDesktopUrl("retry-after-fail-load");
      }
    } catch (error) {
      logDesktopEvent("window-retry-failed", {
        desktopUrl,
        loadAttempts,
        message: error instanceof Error ? error.message : String(error),
      });
    }
  });

  window.webContents.on("did-finish-load", () => {
    logDesktopEvent("window-did-finish-load", {
      desktopUrl,
      loadAttempts,
      currentURL: window.webContents.getURL(),
      title: window.getTitle(),
    });
  });
  window.webContents.on("render-process-gone", (_event, details) => {
    logDesktopEvent("window-render-process-gone", details);
  });
  window.webContents.on("console-message", (_event, level, message, line, sourceId) => {
    logDesktopEvent("window-console-message", { level, message, line, sourceId });
  });

  window.maximize();
  attachSmokeProbe(window);
  loadDesktopUrl().catch((error) => {
    console.error("Failed to load game URL", error);
  });
};

process.on("uncaughtException", (error) => {
  logDesktopEvent("uncaught-exception", {
    message: error.message,
    stack: error.stack,
  });
});

process.on("unhandledRejection", (reason) => {
  logDesktopEvent("unhandled-rejection", {
    reason: reason instanceof Error ? { message: reason.message, stack: reason.stack } : String(reason),
  });
});

app.whenReady()
  .then(async () => {
    const launchSessionId = resolveDesktopLaunchSessionId();
    let desktopUrl = process.env.KRABBY_DESKTOP_URL || getDesktopUrl({ sessionId: launchSessionId });
    logDesktopEvent("app-when-ready", {
      isPackaged: app.isPackaged,
      desktopUrl,
      launchSessionId,
      shouldLaunchPackagedServer: shouldLaunchPackagedServer(),
    });

    if (shouldLaunchPackagedServer()) {
      try {
        await clearDesktopRuntimeCaches(session.defaultSession);
        logDesktopEvent("desktop-runtime-cache-cleared", {});
      } catch (error) {
        logDesktopEvent("desktop-runtime-cache-clear-failed", {
          message: error instanceof Error ? error.message : String(error),
        });
      }
      const packagedServer = await startPackagedServer();
      desktopUrl = packagedServer.desktopUrl;
      await waitForServer(desktopUrl);
    }

    createDesktopWindow(desktopUrl);

    app.on("activate", () => {
      if (BrowserWindow.getAllWindows().length === 0) {
        createDesktopWindow(desktopUrl);
      }
    });
  })
  .catch((error) => {
    console.error("Failed to initialize Electron desktop app", error);
    app.quit();
  });

app.on("before-quit", () => {
  appIsQuitting = true;
  cleanup();
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

#!/usr/bin/env node

const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const process = require("node:process");

const { resolveNextBin } = require("./run-next");
const { preparePublic } = require("./prepare-public");
const { cleanNextArtifacts, resolveNextDir } = require("./clean-next-artifacts");

const LOG_POLL_INTERVAL_MS = 1000;
const REPO_ROOT = path.resolve(process.cwd(), "..", "..");
const resolveNextLogPath = (cwd = process.cwd(), env = process.env) =>
  path.join(resolveNextDir(cwd, env), "dev", "logs", "next-development.log");

const loadWorkspaceEnvFile = (relativePath) => {
  const envPath = path.join(REPO_ROOT, relativePath);
  if (!fs.existsSync(envPath)) {
    return;
  }
  if (typeof process.loadEnvFile === "function") {
    process.loadEnvFile(envPath);
    console.log("[dev-with-logs] loaded env:", envPath);
  }
};

const loadWorkspaceEnv = () => {
  loadWorkspaceEnvFile(".env");
  loadWorkspaceEnvFile(".env.local");
};

const runNextDev = () => {
  try {
    cleanNextArtifacts(process.cwd(), process.env);
    preparePublic();
  } catch (error) {
    console.error("[dev-with-logs] failed to prepare public assets:", error);
  }
  const nextBin = resolveNextBin(process.cwd());
  const args = ["--webpack", ...process.argv.slice(2)];
  console.log("[dev-with-logs] spawning:", nextBin, "dev", ...args);
  return spawn(process.execPath, [nextBin, "dev", ...args], { stdio: "inherit" });
};

const tailLogFile = () => {
  const nextLogPath = resolveNextLogPath(process.cwd(), process.env);
  let lastSize = 0;
  let initialized = false;

  const readNewChunks = (currentSize) => {
    if (currentSize <= lastSize) {
      return;
    }
    const length = currentSize - lastSize;
    const buffer = Buffer.alloc(length);
    let fd;
    try {
      fd = fs.openSync(nextLogPath, "r");
      fs.readSync(fd, buffer, 0, length, lastSize);
    } catch (error) {
      console.error("[dev-with-logs] failed to read log file:", error);
      return;
    } finally {
      if (fd !== undefined) {
        fs.closeSync(fd);
      }
    }
    lastSize = currentSize;
    const text = buffer.toString();
    const prefixed = text.replace(/^/gm, "[next-log] ");
    process.stdout.write(prefixed);
  };

  const interval = setInterval(() => {
    fs.stat(nextLogPath, (err, stats) => {
      if (err) {
        if (err.code !== "ENOENT") {
          console.error("[dev-with-logs] log stat error:", err);
        }
        return;
      }
      if (!initialized) {
        lastSize = stats.size;
        initialized = true;
        return;
      }
      readNewChunks(stats.size);
    });
  }, LOG_POLL_INTERVAL_MS);

  return interval;
};

const main = () => {
  console.log("[dev-with-logs] starting in:", process.cwd());
  loadWorkspaceEnv();
  const nextProcess = runNextDev();
  const logWatcher = tailLogFile();

  const cleanup = (signal) => {
    clearInterval(logWatcher);
    if (!nextProcess.killed) {
      nextProcess.kill(signal);
    }
  };

  process.on("SIGINT", () => cleanup("SIGINT"));
  process.on("SIGTERM", () => cleanup("SIGTERM"));

  nextProcess.on("error", (err) => {
    console.error("[dev-with-logs] failed to start next process:", err);
  });

  nextProcess.on("exit", (code, signal) => {
    clearInterval(logWatcher);
    if (signal) {
      console.error(`[dev-with-logs] next process killed by signal: ${signal}`);
    } else {
      console.log(`[dev-with-logs] next process exited with code: ${code}`);
    }
    process.exit(code ?? 0);
  });
};

if (require.main === module) {
  main();
}

module.exports = { resolveNextLogPath };

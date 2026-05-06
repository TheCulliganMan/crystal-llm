#!/usr/bin/env node

import fs from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { spawn } from "node:child_process";
import { chromium } from "playwright";

// ASM + hardware mapping: one frame is 70,224 cycles at 4,194,304 Hz.
const GB_FRAME_DURATION_MS = (70_224 * 1000) / 4_194_304;

const args = process.argv.slice(2);

const readArg = (name, fallback = null) => {
  const prefix = `--${name}=`;
  const match = args.find((arg) => arg.startsWith(prefix));
  return match ? match.slice(prefix.length) : fallback;
};

const readBooleanArg = (name, fallback = false) => {
  const value = readArg(name);
  if (value == null) {
    return fallback;
  }
  return value === "true" || value === "1" || value === "yes";
};

const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
const outputRoot = path.resolve(process.cwd(), "output", "intro-capture", timestamp);
const screenshotsDir = path.join(outputRoot, "screenshots");
const externallyProvidedBaseUrl = process.env.BASE_URL?.trim() || "";
const managedPort = process.env.PW_APP_PORT || String(3600 + Math.floor(Math.random() * 200));
const baseUrl = readArg("url", externallyProvidedBaseUrl || `http://127.0.0.1:${managedPort}`);
const intervalFrames = Math.max(1, Number.parseInt(readArg("interval-frames", "30"), 10));
const maxFrames = Math.max(1, Number.parseInt(readArg("max-frames", "2400"), 10));
const allowPartial = readBooleanArg("allow-partial", false);
const readyTimeoutMs = Math.max(1_000, Number.parseInt(readArg("ready-timeout-ms", "120000"), 10));

let managedServerProcess = null;
let managedServerExited = false;
let managedServerExitCode = null;

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function ensureDir(dirPath) {
  await fs.mkdir(dirPath, { recursive: true });
}

async function waitForServerReady(url, timeoutMs = 120000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(url, { method: "GET" });
      if (response.ok) {
        return;
      }
    } catch {
      // retry
    }
    await wait(1000);
  }
  throw new Error(`Timed out waiting for server at ${url}`);
}

async function startManagedServerIfNeeded() {
  if (externallyProvidedBaseUrl || readArg("url")) {
    return;
  }

  managedServerProcess = spawn(
    process.execPath,
    ["scripts/dev-with-logs.js", "--hostname", "127.0.0.1", "--port", managedPort],
    {
      cwd: process.cwd(),
      env: {
        ...process.env,
        PORT: managedPort,
        CI: "true",
      },
      stdio: "ignore",
    }
  );

  managedServerProcess.on("exit", (code) => {
    managedServerExited = true;
    managedServerExitCode = code;
  });

  try {
    await waitForServerReady(`${baseUrl}/`, readyTimeoutMs);
  } catch (error) {
    if (managedServerExited) {
      throw new Error(
        `Managed dev server exited before becoming ready (exit code ${managedServerExitCode ?? "unknown"}).`
      );
    }
    throw error;
  }
}

async function stopManagedServer() {
  if (!managedServerProcess || managedServerExited) {
    return;
  }
  managedServerProcess.kill("SIGTERM");
  await new Promise((resolve) => {
    managedServerProcess.once("exit", () => resolve());
    setTimeout(resolve, 5000);
  });
}

async function ensureGameStarted(page) {
  await page.addInitScript(() => {
    window.localStorage.setItem("pokecrystal.play.playIntro", "true");
  });
  await page.goto(baseUrl, { waitUntil: "domcontentloaded", timeout: readyTimeoutMs });
  await page.waitForLoadState("networkidle").catch(() => {});
  const startButton = page.getByRole("button", { name: "Start Game" });
  const startVisible = await startButton
    .waitFor({ state: "visible", timeout: 5_000 })
    .then(() => true)
    .catch(() => false);
  if (startVisible) {
    await startButton.click();
  }
  await page.waitForFunction(
    () =>
      typeof window.advanceTime === "function" &&
      typeof window.get_game_debug_status === "function",
    { timeout: readyTimeoutMs }
  );
  await page.waitForFunction(() => {
    try {
      const payload = JSON.parse(window.get_game_debug_status());
      return payload?.mode && payload.mode !== "uninitialized";
    } catch {
      return false;
    }
  }, { timeout: readyTimeoutMs });
}

async function getDebugStatus(page) {
  const payload = await page.evaluate(() => window.get_game_debug_status?.() ?? "{}");
  return JSON.parse(payload);
}

async function writeCanvasPng(page, destinationPath) {
  const dataUrl = await page.evaluate(() => {
    const canvas = document.querySelector('canvas[aria-label="KrabbyClaw game canvas"]');
    if (!(canvas instanceof HTMLCanvasElement)) {
      throw new Error("Game canvas not found.");
    }
    return canvas.toDataURL("image/png");
  });
  const prefix = "data:image/png;base64,";
  if (!dataUrl.startsWith(prefix)) {
    throw new Error("Unexpected canvas data URL format.");
  }
  await fs.writeFile(destinationPath, Buffer.from(dataUrl.slice(prefix.length), "base64"));
}

function formatElapsedSeconds(frame) {
  return (frame / 60).toFixed(1).padStart(6, "0");
}

function buildScreenshotName(frame) {
  return `intro-f${String(frame).padStart(4, "0")}-t${formatElapsedSeconds(frame)}s.png`;
}

function introSceneName(status) {
  return status?.intro?.sceneName ?? "no-scene";
}

function modeName(status) {
  return status?.mode ?? "unknown";
}

function buildTransitions(captures) {
  const transitions = [];
  for (let index = 1; index < captures.length; index += 1) {
    const previous = captures[index - 1];
    const current = captures[index];
    const previousScene = introSceneName(previous.debugStatus);
    const currentScene = introSceneName(current.debugStatus);
    const previousMode = modeName(previous.debugStatus);
    const currentMode = modeName(current.debugStatus);
    if (previousScene !== currentScene || previousMode !== currentMode) {
      transitions.push({
        frame: current.frame,
        fromScene: previousScene,
        toScene: currentScene,
        fromMode: previousMode,
        toMode: currentMode,
        screenshot: current.screenshot,
      });
    }
  }
  return transitions;
}

function buildTimeline(captures) {
  if (!captures.length) {
    return [];
  }
  const segments = [];
  let current = null;
  for (const capture of captures) {
    const scene = introSceneName(capture.debugStatus);
    const mode = modeName(capture.debugStatus);
    const sceneFrame = capture.debugStatus?.intro?.sceneFrameCounter ?? null;
    const spriteCount = capture.debugStatus?.intro?.spriteCount ?? null;
    if (!current || current.scene !== scene || current.mode !== mode) {
      if (current) {
        segments.push(current);
      }
      current = {
        startFrame: capture.frame,
        endFrame: capture.frame,
        scene,
        mode,
        startSceneFrame: sceneFrame,
        endSceneFrame: sceneFrame,
        minSprites: spriteCount,
        maxSprites: spriteCount,
      };
      continue;
    }
    current.endFrame = capture.frame;
    current.endSceneFrame = sceneFrame;
    if (typeof spriteCount === "number") {
      current.minSprites =
        typeof current.minSprites === "number" ? Math.min(current.minSprites, spriteCount) : spriteCount;
      current.maxSprites =
        typeof current.maxSprites === "number" ? Math.max(current.maxSprites, spriteCount) : spriteCount;
    }
  }
  if (current) {
    segments.push(current);
  }
  return segments;
}

function buildReport({ captures, transitions, diagnostics, finalStatus }) {
  const timeline = buildTimeline(captures);
  const lines = [
    "# Crystal Intro Capture Report",
    "",
    `- Base URL: ${baseUrl}`,
    `- Capture interval: ${intervalFrames} frames (per-frame when interval is 1)`,
    `- Max frames: ${maxFrames}`,
    `- Partial capture allowed: ${allowPartial ? "yes" : "no"}`,
    `- Captures written: ${captures.length}`,
    `- Scene segments: ${timeline.length}`,
    `- Scene transitions: ${transitions.length}`,
    `- Final mode: ${modeName(finalStatus)}`,
    `- Final intro scene: ${introSceneName(finalStatus)}`,
    "",
    "## Scene transitions",
  ];

  if (!transitions.length) {
    lines.push("- none");
  } else {
    for (const transition of transitions) {
      lines.push(
        `- f${String(transition.frame).padStart(4, "0")}: ${transition.fromScene} -> ${transition.toScene} (${transition.fromMode} -> ${transition.toMode}) via ${path.basename(transition.screenshot)}`
      );
    }
  }

  lines.push("", "## Scene timeline");
  for (const segment of timeline) {
    const frameRange = `f${String(segment.startFrame).padStart(4, "0")}-f${String(segment.endFrame).padStart(4, "0")}`;
    const sceneFrameRange =
      typeof segment.startSceneFrame === "number" && typeof segment.endSceneFrame === "number"
        ? `0x${segment.startSceneFrame.toString(16).padStart(2, "0")}..0x${segment.endSceneFrame.toString(16).padStart(2, "0")}`
        : "n/a..n/a";
    const spriteRange =
      typeof segment.minSprites === "number" && typeof segment.maxSprites === "number"
        ? `${segment.minSprites}..${segment.maxSprites}`
        : "n/a..n/a";
    lines.push(
      `- ${frameRange}: ${segment.scene} (${segment.mode}, sceneFrame=${sceneFrameRange}, sprites=${spriteRange})`
    );
  }

  lines.push("", "## Diagnostics");
  if (!diagnostics.consoleErrors.length && !diagnostics.pageErrors.length && !diagnostics.requestFailures.length) {
    lines.push("- No console, page, or request errors captured.");
  } else {
    for (const entry of diagnostics.consoleErrors) {
      lines.push(`- [console] ${entry}`);
    }
    for (const entry of diagnostics.pageErrors) {
      lines.push(`- [pageerror] ${entry}`);
    }
    for (const entry of diagnostics.requestFailures) {
      lines.push(`- [requestfailed] ${entry}`);
    }
  }

  lines.push("", "## Findings");
  lines.push(`- Intro reached title handoff: ${modeName(finalStatus) === "title" ? "yes" : "no"}`);
  lines.push(`- Remaining intro scene at end: ${introSceneName(finalStatus)}`);
  return `${lines.join("\n")}\n`;
}

function attachDiagnostics(page) {
  const diagnostics = {
    consoleErrors: [],
    pageErrors: [],
    requestFailures: [],
  };

  page.on("console", (message) => {
    if (message.type() === "error") {
      diagnostics.consoleErrors.push(message.text());
    }
  });
  page.on("pageerror", (error) => {
    diagnostics.pageErrors.push(error.message);
  });
  page.on("requestfailed", (request) => {
    const failure = request.failure();
    diagnostics.requestFailures.push(
      `${request.method()} ${request.url()} :: ${failure?.errorText ?? "unknown failure"}`
    );
  });

  return diagnostics;
}

async function main() {
  await ensureDir(screenshotsDir);
  await startManagedServerIfNeeded();

  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({
    viewport: { width: 1440, height: 1100 },
    colorScheme: "light",
    reducedMotion: "reduce",
  });
  const diagnostics = attachDiagnostics(page);

  try {
    await ensureGameStarted(page);

    const captures = [];
    let finalStatus = await getDebugStatus(page);

    for (let frame = 0; frame <= maxFrames; frame += 1) {
      if (frame % intervalFrames === 0) {
        const screenshotPath = path.join(screenshotsDir, buildScreenshotName(frame));
        await writeCanvasPng(page, screenshotPath);
        captures.push({
          frame,
          elapsedSeconds: Number.parseFloat((frame / 60).toFixed(3)),
          screenshot: screenshotPath,
          debugStatus: finalStatus,
        });
      }

      if (modeName(finalStatus) === "title") {
        break;
      }

      if (frame === maxFrames) {
        if (!allowPartial) {
          throw new Error(`Intro did not reach title within ${maxFrames} frames.`);
        }
        break;
      }

      await page.evaluate(async (ms) => {
        await window.advanceTime?.(ms);
      }, GB_FRAME_DURATION_MS);
      finalStatus = await getDebugStatus(page);
    }

    if (!captures.length) {
      throw new Error("Intro capture produced no screenshots.");
    }

    const transitions = buildTransitions(captures);
    const manifest = {
      baseUrl,
      intervalFrames,
      maxFrames,
      allowPartial,
      finalStatus,
      captures,
    };
    const report = buildReport({ captures, transitions, diagnostics, finalStatus });

    await fs.writeFile(path.join(outputRoot, "manifest.json"), JSON.stringify(manifest, null, 2));
    await fs.writeFile(path.join(outputRoot, "report.md"), report);

    process.stdout.write(
      JSON.stringify(
        {
          outputDir: outputRoot,
          screenshotsDir,
          manifest: path.join(outputRoot, "manifest.json"),
          report: path.join(outputRoot, "report.md"),
          captures: captures.length,
          finalStatus,
          diagnostics,
        },
        null,
        2
      )
    );
  } finally {
    await browser.close();
    await stopManagedServer();
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

#!/usr/bin/env node

import fs from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { spawn } from "node:child_process";
import { chromium } from "playwright";

const GB_FRAME_DURATION_MS = (70_224 * 1000) / 4_194_304;
const args = process.argv.slice(2);

const readArg = (name, fallback = null) => {
  const prefix = `--${name}=`;
  const match = args.find((arg) => arg.startsWith(prefix));
  return match ? match.slice(prefix.length) : fallback;
};

const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
const outputRoot = path.resolve(process.cwd(), "output", "oak-intro-capture", timestamp);
const screenshotsDir = path.join(outputRoot, "screenshots");
const externallyProvidedBaseUrl = process.env.BASE_URL?.trim() || "";
const managedPort = process.env.PW_APP_PORT || String(3900 + Math.floor(Math.random() * 200));
const baseUrl = readArg("url", externallyProvidedBaseUrl || `http://127.0.0.1:${managedPort}`);
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
      typeof window.get_game_debug_status === "function" &&
      typeof window.jump_game_scene === "function" &&
      typeof window.post_game_event === "function",
    undefined,
    { timeout: readyTimeoutMs }
  );
  await page.waitForFunction(
    () => {
      try {
        const payload = JSON.parse(window.get_game_debug_status?.() ?? "{}");
        return payload?.mode && payload.mode !== "uninitialized";
      } catch {
        return false;
      }
    },
    undefined,
    { timeout: readyTimeoutMs }
  );
}

async function getDebugStatus(page) {
  const payload = await page.evaluate(() => window.get_game_debug_status?.() ?? "{}");
  return JSON.parse(payload);
}

async function advanceFrames(page, frames) {
  await page.evaluate(async (ms) => {
    await window.advanceTime?.(ms);
  }, GB_FRAME_DURATION_MS * frames);
}

async function waitForStatus(page, predicate, label, maxFrames = 2400) {
  for (let frame = 0; frame < maxFrames; frame += 1) {
    const status = await getDebugStatus(page);
    if (predicate(status)) {
      return status;
    }
    await advanceFrames(page, 1);
  }
  throw new Error(`Timed out waiting for ${label}`);
}

async function dispatchKey(page, key, code) {
  await page.evaluate(
    ({ nextKey, nextCode }) => {
      const dispatch = (type, isPress) => {
        const event = new KeyboardEvent(type, {
          key: nextKey,
          code: nextCode,
          bubbles: true,
          cancelable: true,
        });
        Object.defineProperty(event, "is_press", { value: isPress, configurable: true });
        window.dispatchEvent(event);
      };
      dispatch("keydown", true);
      dispatch("keyup", false);
    },
    { nextKey: key, nextCode: code }
  );
}

async function pressA(page) {
  await dispatchKey(page, "z", "KeyZ");
}

async function pressStart(page) {
  await dispatchKey(page, "Enter", "Enter");
}

async function typeNameCharacter(page, char) {
  await page.evaluate((value) => {
    window.post_game_event?.({
      type: "keydown",
      key: value,
      code: `Key${value.toUpperCase()}`,
      is_press: true,
      text: value,
    });
    window.post_game_event?.({
      type: "keyup",
      key: value,
      code: `Key${value.toUpperCase()}`,
      is_press: false,
      text: value,
    });
  }, char);
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
  await fs.writeFile(destinationPath, Buffer.from(dataUrl.slice(prefix.length), "base64"));
}

async function captureCheckpoint(page, captures, name) {
  const screenshotPath = path.join(screenshotsDir, `${String(captures.length + 1).padStart(2, "0")}-${name}.png`);
  await writeCanvasPng(page, screenshotPath);
  captures.push({
    name,
    screenshot: screenshotPath,
    debugStatus: await getDebugStatus(page),
  });
  console.error(`[oak-capture] checkpoint: ${name}`);
}

async function mashUntil(page, predicate, label, maxAttempts = 240) {
  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    const status = await getDebugStatus(page);
    if (predicate(status)) {
      return status;
    }
    await pressA(page);
    await advanceFrames(page, 8);
  }
  throw new Error(`Timed out while advancing to ${label}`);
}

async function waitForVisibleIntroText(page, label, minChars = 4, maxFrames = 240) {
  return waitForStatus(
    page,
    (status) =>
      status.mode === "oak_intro" &&
      typeof status.oak_intro?.visibleText === "string" &&
      status.oak_intro.visibleText.trim().length >= minChars,
    label,
    maxFrames
  );
}

function buildReport(captures) {
  const lines = [
    "# Oak Intro Capture Report",
    "",
    `- Base URL: ${baseUrl}`,
    `- Captures written: ${captures.length}`,
    "",
    "## Checkpoints",
  ];
  for (const capture of captures) {
    lines.push(
      `- ${capture.name}: mode=${capture.debugStatus.mode}, screenshot=${path.basename(capture.screenshot)}`
    );
  }
  return `${lines.join("\n")}\n`;
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

  try {
    await ensureGameStarted(page);
    console.error("[oak-capture] game started");
    await page.evaluate(async () => {
      await window.jump_game_scene?.("new_game");
    });
    console.error("[oak-capture] jumped to new_game");

    const captures = [];

    await waitForStatus(page, (status) => status.mode === "gender", "gender screen");
    await captureCheckpoint(page, captures, "gender");

    await pressA(page);
    await mashUntil(
      page,
      (status) =>
        status.mode === "oak_intro" &&
        status.oak_intro?.timeSetComplete === false &&
        status.oak_intro?.timeSetPhase === "set_hour",
      "time set screen",
      120
    );
    await captureCheckpoint(page, captures, "time-set");

    await mashUntil(
      page,
      (status) =>
        status.mode === "oak_intro" &&
        status.oak_intro?.sceneState === "oak_intro_1" &&
        status.oak_intro?.scenePhase === "text",
      "Oak opening scene",
      120
    );
    await waitForVisibleIntroText(page, "Oak opening text");
    await captureCheckpoint(page, captures, "oak-opening");

    await mashUntil(
      page,
      (status) =>
        status.mode === "oak_intro" &&
        status.oak_intro?.sceneState === "wooper_showcase" &&
        status.oak_intro?.scenePhase === "text_one",
      "Whooper scene"
    );
    await waitForVisibleIntroText(page, "Whooper text");
    await captureCheckpoint(page, captures, "whooper");

    await mashUntil(
      page,
      (status) =>
        status.mode === "oak_intro" &&
        status.oak_intro?.sceneState === "player_picture" &&
        status.oak_intro?.scenePhase === "text",
      "player picture scene"
    );
    await waitForVisibleIntroText(page, "player picture text");
    await captureCheckpoint(page, captures, "player-picture");

    await mashUntil(page, (status) => status.mode === "name_entry", "player name entry");
    await captureCheckpoint(page, captures, "name-entry");

    for (const char of ["K", "R", "I", "S"]) {
      await typeNameCharacter(page, char);
      await advanceFrames(page, 1);
    }
    await pressStart(page);

    await waitForStatus(
      page,
      (status) =>
        status.mode === "oak_intro" &&
        status.oak_intro?.mode === "final" &&
        typeof status.oak_intro?.visibleText === "string" &&
        status.oak_intro.visibleText.trim().length >= 4,
      "Oak final encouragement"
    );
    await captureCheckpoint(page, captures, "oak-final");

    await mashUntil(page, (status) => status.mode === "overworld", "overworld");
    await advanceFrames(page, 60);
    await captureCheckpoint(page, captures, "overworld");
    console.error("[oak-capture] completed");

    const report = buildReport(captures);
    const manifest = {
      baseUrl,
      captures,
    };

    await fs.writeFile(path.join(outputRoot, "report.md"), report);
    await fs.writeFile(path.join(outputRoot, "manifest.json"), JSON.stringify(manifest, null, 2));

    process.stdout.write(
      JSON.stringify(
        {
          outputDir: outputRoot,
          report: path.join(outputRoot, "report.md"),
          manifest: path.join(outputRoot, "manifest.json"),
          captures: captures.length,
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

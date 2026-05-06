import fs from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";

const args = process.argv.slice(2);
const readArg = (name, fallback = null) => {
  const prefix = `--${name}=`;
  const match = args.find((arg) => arg.startsWith(prefix));
  return match ? match.slice(prefix.length) : fallback;
};

const url = readArg("url", "http://127.0.0.1:3000");
const scene = readArg("scene");
const spawn = readArg("spawn");
const scriptFile = readArg("script-file");
const screenshotName = readArg("screenshot", "visual-debug-game.png");
const outputDir = path.resolve("output", "playwright");
const readyTimeoutMs = Number.parseInt(readArg("ready-timeout-ms", "90000"), 10);

const ensureGameStarted = async (page) => {
  const startButton = page.getByRole("button", { name: "Start Game" });
  const startVisible = await startButton
    .waitFor({ state: "visible", timeout: 3_000 })
    .then(() => true)
    .catch(() => false);
  if (startVisible) {
    await startButton.click();
  }
  await page.waitForFunction(
    () =>
      typeof window.render_game_to_text === "function" &&
      typeof window.get_game_debug_status === "function",
    { timeout: readyTimeoutMs }
  );
  await page.waitForFunction(() => {
    try {
      const payload = JSON.parse(window.get_game_debug_status());
      return payload && payload.mode && payload.mode !== "uninitialized";
    } catch {
      return false;
    }
  }, { timeout: readyTimeoutMs });
};

const main = async () => {
  await fs.mkdir(outputDir, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1100 } });
  const logs = [];
  page.on("console", (message) => {
    logs.push({ type: message.type(), text: message.text() });
  });
  page.on("pageerror", (error) => {
    logs.push({ type: "pageerror", text: String(error) });
  });
  await page.goto(url, { waitUntil: "networkidle" });
  await ensureGameStarted(page);

  if (scene) {
    await page.evaluate(async (value) => {
      await window.jump_game_scene?.(value);
    }, scene);
  }

  if (spawn) {
    await page.evaluate(async (value) => {
      await window.jump_game_spawn?.(value);
    }, spawn);
  }

  let scriptResult = null;
  if (scriptFile) {
    const raw = await fs.readFile(path.resolve(scriptFile), "utf8");
    scriptResult = await page.evaluate(async (payload) => {
      return await window.run_game_script?.(payload);
    }, raw);
  }

  const debugStatus = await page.evaluate(() => window.get_game_debug_status?.() ?? "{}");
  const localText = await page.evaluate(() => window.render_game_to_text?.() ?? "{}");
  await page.screenshot({
    path: path.join(outputDir, screenshotName),
    fullPage: true,
  });

  await browser.close();

  process.stdout.write(
    JSON.stringify(
      {
        url,
        scene,
        spawn,
        scriptFile,
        scriptResult,
        debugStatus: JSON.parse(debugStatus),
        localText: JSON.parse(localText),
        screenshot: path.join(outputDir, screenshotName),
        logs,
      },
      null,
      2
    )
  );
};

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

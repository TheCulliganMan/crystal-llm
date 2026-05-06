#!/usr/bin/env node

import fs from "node:fs";
import { chromium } from "playwright";

const BASE_URL = process.env.BASE_URL?.trim() || "http://127.0.0.1:3000";

async function writeCanvasPng(page, dest) {
  const dataUrl = await page.evaluate(() => {
    const canvas = document.querySelector('canvas[aria-label="KrabbyClaw game canvas"]');
    if (!(canvas instanceof HTMLCanvasElement)) {
      throw new Error("Game canvas not found.");
    }
    return canvas.toDataURL("image/png");
  });
  const prefix = "data:image/png;base64,";
  if (!dataUrl.startsWith(prefix)) {
    throw new Error("Unexpected canvas data URL.");
  }
  fs.writeFileSync(dest, Buffer.from(dataUrl.slice(prefix.length), "base64"));
}

async function bootPage(page, playIntro) {
  await page.addInitScript((enabled) => {
    window.localStorage.setItem("pokecrystal.play.playIntro", enabled ? "true" : "false");
  }, playIntro);

  await page.goto(BASE_URL, { waitUntil: "domcontentloaded", timeout: 120000 });
  await page.waitForLoadState("networkidle").catch(() => {});

  const startButton = page.getByRole("button", { name: "Start Game" });
  const visible = await startButton
    .waitFor({ state: "visible", timeout: 3000 })
    .then(() => true)
    .catch(() => false);
  if (visible) {
    await startButton.click();
  }

  await page.waitForFunction(
    () => typeof window.advanceTime === "function" && typeof window.get_game_debug_status === "function",
    { timeout: 120000 }
  );
  await page.waitForFunction(() => {
    try {
      const payload = JSON.parse(window.get_game_debug_status());
      return payload?.mode && payload.mode !== "uninitialized";
    } catch {
      return false;
    }
  }, { timeout: 120000 });
}

async function getStatus(page) {
  return page.evaluate(() => JSON.parse(window.get_game_debug_status()));
}

async function advance(page) {
  await page.evaluate(() => window.advanceTime(1));
}

async function main() {
  const browser = await chromium.launch({ headless: true });

  const introPage = await browser.newPage({ viewport: { width: 960, height: 720 } });
  await bootPage(introPage, true);
  for (let i = 0; i < 800; i += 1) {
    const status = await getStatus(introPage);
    if (
      status?.mode === "intro" &&
      status?.intro?.sceneName === "unown_pulse" &&
      status?.intro?.sceneFrameCounter === 0x60
    ) {
      await writeCanvasPng(introPage, "/tmp/intro-third-pulse-proof.png");
      break;
    }
    await advance(introPage);
  }
  await introPage.close();

  const titlePage = await browser.newPage({ viewport: { width: 960, height: 720 } });
  await bootPage(titlePage, true);
  for (let i = 0; i < 1800; i += 1) {
    const status = await getStatus(titlePage);
    if (status?.mode === "title") {
      await writeCanvasPng(titlePage, "/tmp/title-proof.png");
      break;
    }
    await advance(titlePage);
  }
  await titlePage.close();

  await browser.close();
  console.log("/tmp/intro-third-pulse-proof.png");
  console.log("/tmp/title-proof.png");
}

await main();

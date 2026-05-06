#!/usr/bin/env node

import process from "node:process";
import { spawn } from "node:child_process";
import { chromium } from "playwright";

const managedPort = process.env.PW_APP_PORT || String(3500 + Math.floor(Math.random() * 300));
const baseUrl = process.env.BASE_URL?.trim() || `http://127.0.0.1:${managedPort}`;
const usingExternalBaseUrl = Boolean(process.env.BASE_URL?.trim());

let managedServerProcess = null;

const startManagedServerIfNeeded = async () => {
  if (usingExternalBaseUrl) {
    return;
  }
  managedServerProcess = spawn(
    process.execPath,
    ["scripts/dev-with-logs.js", "--hostname", "127.0.0.1", "--port", managedPort],
    {
      cwd: process.cwd(),
      env: {
        ...process.env,
        CI: "true",
        PORT: managedPort,
      },
      stdio: "ignore",
    }
  );

  const startedAt = Date.now();
  while (Date.now() - startedAt < 120000) {
    try {
      const response = await fetch(baseUrl);
      if (response.ok) {
        return;
      }
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  throw new Error(`Timed out waiting for ${baseUrl}`);
};

const stopManagedServer = async () => {
  if (!managedServerProcess) {
    return;
  }
  managedServerProcess.kill("SIGTERM");
  await new Promise((resolve) => {
    managedServerProcess.once("exit", () => resolve());
    setTimeout(resolve, 5000);
  });
};

const evaluateHook = async (page, expression, ...args) =>
  page.evaluate(expression, ...args);

const waitForGameHooks = async (page) => {
  await page.waitForFunction(() => {
    return (
      typeof window.get_game_debug_status === "function" &&
      typeof window.jump_game_scene === "function" &&
      typeof window.post_game_event === "function" &&
      typeof window.save_game_to_slot === "function" &&
      typeof window.delete_save_slot === "function" &&
      typeof window.has_save_slot === "function" &&
      typeof window.trigger_game_autosave === "function"
    );
  });
};

const waitForGameMode = async (page, mode) => {
  await page.waitForFunction((expectedMode) => {
    try {
      const raw = window.get_game_debug_status?.();
      return JSON.parse(raw ?? "{}").mode === expectedMode;
    } catch {
      return false;
    }
  }, mode);
};

const bootPlayPage = async (page) => {
  await page.goto(`${baseUrl}/`, { waitUntil: "domcontentloaded" });
  await page.waitForSelector('[data-testid="play-page-shell"]');
  const startButton = page.getByRole("button", { name: "Start Game" });
  if (await startButton.isVisible().catch(() => false)) {
    await startButton.click();
  }
  await waitForGameHooks(page);
};

const postEnter = async (page) => {
  await evaluateHook(page, () => {
    window.post_game_event?.({
      type: "keydown",
      key: "Enter",
      code: "Enter",
      is_press: true,
    });
    window.post_game_event?.({
      type: "keyup",
      key: "Enter",
      code: "Enter",
      is_press: false,
    });
  });
};

const openContinueFromMainMenu = async (page) => {
  await evaluateHook(page, async () => {
    await window.jump_game_scene?.("main_menu");
  });
  await waitForGameMode(page, "main_menu");
  await postEnter(page);
  await evaluateHook(page, async () => {
    await window.advanceTime?.(250);
  });
};

const run = async () => {
  await startManagedServerIfNeeded();
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  try {
    await bootPlayPage(page);

    const manualSaved = await evaluateHook(page, async () => {
      return window.save_game_to_slot?.("savegame.sav", { withHistory: true });
    });
    if (!manualSaved) {
      throw new Error("Manual save did not succeed.");
    }
    const manualExists = await evaluateHook(page, async () => window.has_save_slot?.("savegame.sav"));
    if (!manualExists) {
      throw new Error("Manual save was not discoverable immediately after saving.");
    }

    await page.reload({ waitUntil: "domcontentloaded" });
    await bootPlayPage(page);
    await openContinueFromMainMenu(page);
    await waitForGameMode(page, "continue");

    const autosaveExists = await evaluateHook(page, async () => {
      await window.trigger_game_autosave?.("battle_complete");
      return window.has_save_slot?.("autosave.sav");
    });
    if (!autosaveExists) {
      throw new Error("Autosave slot was not created.");
    }

    await page.reload({ waitUntil: "domcontentloaded" });
    await bootPlayPage(page);
    const autosaveStillExists = await evaluateHook(page, async () => window.has_save_slot?.("autosave.sav"));
    if (!autosaveStillExists) {
      throw new Error("Autosave slot did not survive reload.");
    }
    await evaluateHook(page, async () => {
      await window.jump_game_scene?.("overworld");
      await window.advanceTime?.(150);
    });
    await waitForGameMode(page, "overworld");

    await evaluateHook(page, async () => {
      const canonicalPayload = window.localStorage.getItem("pokecrystal:guest-save:savegame.sav");
      if (!canonicalPayload) {
        throw new Error("Canonical guest save payload missing for legacy migration test.");
      }
      window.localStorage.setItem("fs:/legacy/savegame.sav", canonicalPayload);
      window.localStorage.removeItem("pokecrystal:guest-save:savegame.sav");
      window.localStorage.removeItem("pokecrystal:guest-save-meta:savegame.sav");
    });
    await page.reload({ waitUntil: "domcontentloaded" });
    await bootPlayPage(page);
    await openContinueFromMainMenu(page);
    await waitForGameMode(page, "continue");

    const deleted = await evaluateHook(page, async () => window.delete_save_slot?.("savegame.sav"));
    if (!deleted) {
      throw new Error("Manual save delete did not report success.");
    }
    const manualStillExists = await evaluateHook(page, async () => window.has_save_slot?.("savegame.sav"));
    if (manualStillExists) {
      throw new Error("Manual save still exists after deletion.");
    }

    await page.reload({ waitUntil: "domcontentloaded" });
    await bootPlayPage(page);
    await openContinueFromMainMenu(page);
    const modeAfterDelete = await evaluateHook(page, () => {
      try {
        return JSON.parse(window.get_game_debug_status?.() ?? "{}").mode ?? null;
      } catch {
        return null;
      }
    });
    if (modeAfterDelete === "continue") {
      throw new Error("Continue remained available after deleting the manual save.");
    }

    console.log("Save path Playwright checks passed.");
  } finally {
    await browser.close();
    await stopManagedServer();
  }
};

run().catch((error) => {
  console.error(error instanceof Error ? error.stack ?? error.message : String(error));
  process.exitCode = 1;
});

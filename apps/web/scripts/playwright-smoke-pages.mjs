#!/usr/bin/env node

import fs from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { spawn } from "node:child_process";
import { chromium, devices } from "playwright";
import { PNG } from "pngjs";
import pixelmatch from "pixelmatch";
import routeManifest from "./playwright-route-manifest.json" with { type: "json" };

const externallyProvidedBaseUrl = process.env.BASE_URL?.trim() || "";
const managedPort = process.env.PW_APP_PORT || String(3200 + Math.floor(Math.random() * 300));
const baseUrl = externallyProvidedBaseUrl || `http://127.0.0.1:${managedPort}`;
const updateBaselines = process.env.E2E_UPDATE_BASELINES === "true";
const failOnConsoleErrors = process.env.E2E_FAIL_ON_CONSOLE_ERRORS === "true";
const failOnPageErrors = process.env.E2E_FAIL_ON_PAGE_ERRORS === "true";
const failOnRequestFailures = process.env.E2E_FAIL_ON_REQUEST_FAILURES === "true";
const visualDiffThreshold = Number.parseFloat(process.env.E2E_VISUAL_DIFF_THRESHOLD || "0.005");
const shouldDisableAnimations = process.env.E2E_DISABLE_ANIMATIONS !== "false";
const defaultSuites = ["routes", "visual"];
const suites = Array.from(
  new Set(
    process.argv
      .slice(2)
      .flatMap((arg) => arg.split(","))
      .map((value) => value.trim().toLowerCase())
      .filter(Boolean),
  ),
);
const selectedSuites = suites.length ? suites : defaultSuites;
const { defaultBrowserType: _ignoredIphoneBrowser, ...iphone13Device } = devices["iPhone 13"];
const { defaultBrowserType: _ignoredPixelBrowser, ...pixel7Device } = devices["Pixel 7"];
const deviceProfilesByName = new Map([
  [
    "desktop",
    {
      name: "desktop",
      label: "Desktop Chromium",
      contextOptions: { viewport: { width: 1440, height: 900 } },
    },
  ],
  [
    "iphone-13",
    {
      name: "iphone-13",
      label: "iPhone 13",
      contextOptions: iphone13Device,
    },
  ],
  [
    "pixel-7",
    {
      name: "pixel-7",
      label: "Pixel 7",
      contextOptions: pixel7Device,
    },
  ],
]);
const requestedDeviceProfiles = (process.env.E2E_DEVICE_PROFILES || "desktop")
  .split(",")
  .map((value) => value.trim().toLowerCase())
  .filter(Boolean);
const selectedDeviceProfiles = Array.from(
  new Set(
    requestedDeviceProfiles.map((profileName) => {
      const profile = deviceProfilesByName.get(profileName);
      if (!profile) {
        throw new Error(
          `Unsupported device profile "${profileName}". Valid profiles: ${Array.from(deviceProfilesByName.keys()).join(", ")}`,
        );
      }
      return profile;
    }),
  ),
);
const outputDir = path.resolve(
  process.cwd(),
  "output",
  "playwright",
  new Date().toISOString().replace(/[:.]/g, "-"),
);
const baselineRoot = path.resolve(process.cwd(), "tests", "baselines", "chromium");

let managedServerProcess = null;
let managedServerExited = false;
let managedServerExitCode = null;

function normalizeUrlPath(value) {
  if (!value) return "/";
  const [pathname, search = ""] = value.split("?");
  const normalizedPathname = pathname.endsWith("/") && pathname !== "/" ? pathname.slice(0, -1) : pathname;
  return search ? `${normalizedPathname}?${search}` : normalizedPathname;
}

function getSuiteProfileOutputDir(suiteName, profileName) {
  return profileName === "desktop"
    ? path.join(outputDir, suiteName)
    : path.join(outputDir, suiteName, profileName);
}

function getBaselineDir(profileName) {
  return profileName === "desktop"
    ? path.join(baselineRoot, "public")
    : path.join(baselineRoot, profileName, "public");
}

async function ensureDir(dirPath) {
  await fs.mkdir(dirPath, { recursive: true });
}

async function fileExists(filePath) {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
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
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  throw new Error(`Timed out waiting for server at ${url}`);
}

async function startManagedServerIfNeeded() {
  if (externallyProvidedBaseUrl) {
    return;
  }

  managedServerProcess = spawn(process.execPath, ["scripts/dev-with-logs.js", "--hostname", "127.0.0.1", "--port", managedPort], {
    cwd: process.cwd(),
    env: {
      ...process.env,
      PORT: managedPort,
      CI: "true",
    },
    stdio: "ignore",
  });

  managedServerProcess.on("exit", (code) => {
    managedServerExited = true;
    managedServerExitCode = code;
  });

  try {
    await waitForServerReady(`${baseUrl}/`);
  } catch (error) {
    if (managedServerExited) {
      throw new Error(
        `Managed dev server exited before becoming ready (exit code ${managedServerExitCode ?? "unknown"}).`,
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

async function applyStabilityTweaks(page) {
  if (!shouldDisableAnimations) {
    return;
  }
  await page.addStyleTag({
    content: `
      *,
      *::before,
      *::after {
        animation: none !important;
        transition: none !important;
        caret-color: transparent !important;
      }
      html {
        scroll-behavior: auto !important;
      }
    `,
  });
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
    diagnostics.requestFailures.push(`${request.method()} ${request.url()} :: ${failure?.errorText ?? "unknown failure"}`);
  });

  return diagnostics;
}

async function navigateToRoute(page, route) {
  const targetUrl = `${baseUrl}${route.path}`;
  const response = await page.goto(targetUrl, { waitUntil: "domcontentloaded", timeout: 120000 });
  await page.waitForSelector(`[data-testid="${route.readyTestId}"]`, { state: "visible", timeout: 120000 });
  await applyStabilityTweaks(page);
  await page.waitForLoadState("networkidle").catch(() => {});
  await page.waitForTimeout(300);
  const currentUrl = new URL(page.url());
  const actualPath = normalizeUrlPath(`${currentUrl.pathname}${currentUrl.search}`);
  const expectedPath = normalizeUrlPath(route.expectedPath || route.path);
  if (actualPath !== expectedPath) {
    throw new Error(`Route ${route.path} resolved to ${actualPath} instead of ${expectedPath}`);
  }
  return response;
}

async function compareScreenshots(actualPath, baselinePath, diffPath) {
  const actualBuffer = await fs.readFile(actualPath);
  const baselineBuffer = await fs.readFile(baselinePath);
  const actual = PNG.sync.read(actualBuffer);
  const baseline = PNG.sync.read(baselineBuffer);

  if (actual.width !== baseline.width || actual.height !== baseline.height) {
    throw new Error(
      `Screenshot dimensions differ for ${path.basename(actualPath)} (${actual.width}x${actual.height} vs ${baseline.width}x${baseline.height})`,
    );
  }

  const diff = new PNG({ width: actual.width, height: actual.height });
  const diffPixels = pixelmatch(actual.data, baseline.data, diff.data, actual.width, actual.height, {
    threshold: 0.1,
  });
  const diffRatio = diffPixels / (actual.width * actual.height);

  if (diffRatio > visualDiffThreshold) {
    await fs.writeFile(diffPath, PNG.sync.write(diff));
  }

  return { diffPixels, diffRatio };
}

async function createContext(browser, profile) {
  return browser.newContext({
    ...profile.contextOptions,
    colorScheme: "light",
    reducedMotion: "reduce",
  });
}

async function runRouteSuiteForProfile(browser, profile) {
  const report = {
    suite: "routes",
    profile: profile.name,
    passed: [],
    failed: [],
  };
  const context = await createContext(browser, profile);

  try {
    for (const route of routeManifest) {
      const page = await context.newPage();
      const diagnostics = attachDiagnostics(page);
      try {
        const response = await navigateToRoute(page, route);
        const status = response?.status() ?? null;
        const routeResult = {
          slug: route.slug,
          path: route.path,
          expectedPath: route.expectedPath || route.path,
          status,
          consoleErrors: diagnostics.consoleErrors,
          pageErrors: diagnostics.pageErrors,
          requestFailures: diagnostics.requestFailures,
        };
        if (failOnConsoleErrors && diagnostics.consoleErrors.length) {
          throw new Error(`Console errors detected for ${route.path}`);
        }
        if (failOnPageErrors && diagnostics.pageErrors.length) {
          throw new Error(`Page errors detected for ${route.path}`);
        }
        if (failOnRequestFailures && diagnostics.requestFailures.length) {
          throw new Error(`Request failures detected for ${route.path}`);
        }
        report.passed.push(routeResult);
      } catch (error) {
        const errorText = error instanceof Error ? error.message : String(error);
        const suiteDir = getSuiteProfileOutputDir("routes", profile.name);
        await ensureDir(suiteDir);
        const screenshotPath = path.join(suiteDir, `${route.slug}.png`);
        await page.screenshot({ path: screenshotPath, fullPage: true });
        report.failed.push({
          slug: route.slug,
          path: route.path,
          error: errorText,
          screenshotPath,
          consoleErrors: diagnostics.consoleErrors,
          pageErrors: diagnostics.pageErrors,
          requestFailures: diagnostics.requestFailures,
        });
      } finally {
        await page.close();
      }
    }
  } finally {
    await context.close();
  }

  return report;
}

async function runVisualSuiteForProfile(browser, profile) {
  const report = {
    suite: "visual",
    profile: profile.name,
    passed: [],
    failed: [],
  };
  const suiteDir = getSuiteProfileOutputDir("visual", profile.name);
  const baselineDir = getBaselineDir(profile.name);
  await ensureDir(suiteDir);
  await ensureDir(baselineDir);

  const context = await createContext(browser, profile);
  try {
    for (const route of routeManifest.filter((entry) => entry.includeInVisual)) {
      const page = await context.newPage();
      try {
        await navigateToRoute(page, route);
        const actualPath = path.join(suiteDir, `${route.slug}.png`);
        const baselinePath = path.join(baselineDir, `${route.slug}.png`);
        const diffPath = path.join(suiteDir, `${route.slug}.diff.png`);
        await page.screenshot({ path: actualPath, fullPage: true });

        if (updateBaselines || !(await fileExists(baselinePath))) {
          await fs.copyFile(actualPath, baselinePath);
          report.passed.push({
            slug: route.slug,
            baseline: baselinePath,
            mode: updateBaselines ? "updated" : "created",
          });
        } else {
          const diff = await compareScreenshots(actualPath, baselinePath, diffPath);
          if (diff.diffRatio > visualDiffThreshold) {
            report.failed.push({
              slug: route.slug,
              baseline: baselinePath,
              actual: actualPath,
              diff: diffPath,
              diffPixels: diff.diffPixels,
              diffRatio: diff.diffRatio,
            });
          } else {
            report.passed.push({
              slug: route.slug,
              baseline: baselinePath,
              actual: actualPath,
              diffPixels: diff.diffPixels,
              diffRatio: diff.diffRatio,
            });
          }
        }
      } catch (error) {
        report.failed.push({
          slug: route.slug,
          path: route.path,
          error: error instanceof Error ? error.message : String(error),
        });
      } finally {
        await page.close();
      }
    }
  } finally {
    await context.close();
  }

  return report;
}

function buildMarkdownReport(suiteReports) {
  const lines = [
    "# Playwright Smoke Report",
    "",
    `- Base URL: \`${baseUrl}\``,
    `- Suites: ${selectedSuites.join(", ")}`,
    `- Device profiles: ${selectedDeviceProfiles.map((profile) => profile.name).join(", ")}`,
    "",
  ];

  for (const suiteReport of suiteReports) {
    lines.push(`## ${suiteReport.suite} / ${suiteReport.profile}`);
    lines.push("");
    lines.push(`- Passed: ${suiteReport.passed.length}`);
    lines.push(`- Failed: ${suiteReport.failed.length}`);
    if (suiteReport.failed.length) {
      lines.push("");
      for (const failure of suiteReport.failed) {
        lines.push(`- ${failure.slug}: ${failure.error ?? `diff ratio ${failure.diffRatio}`}`);
      }
    }
    lines.push("");
  }

  return `${lines.join("\n").trim()}\n`;
}

async function main() {
  const invalidSuites = selectedSuites.filter((suite) => !["routes", "visual"].includes(suite));
  if (invalidSuites.length) {
    throw new Error(`Unsupported suite(s): ${invalidSuites.join(", ")}`);
  }

  await ensureDir(outputDir);
  await startManagedServerIfNeeded();

  const browser = await chromium.launch({ headless: true });
  try {
    const suiteReports = [];
    for (const suite of selectedSuites) {
      for (const profile of selectedDeviceProfiles) {
        if (suite === "routes") {
          suiteReports.push(await runRouteSuiteForProfile(browser, profile));
        }
        if (suite === "visual") {
          suiteReports.push(await runVisualSuiteForProfile(browser, profile));
        }
      }
    }

    const reportJsonPath = path.join(outputDir, "report.json");
    const reportMdPath = path.join(outputDir, "report.md");
    await fs.writeFile(reportJsonPath, JSON.stringify(suiteReports, null, 2));
    await fs.writeFile(reportMdPath, buildMarkdownReport(suiteReports));

    const failures = suiteReports.reduce((count, report) => count + report.failed.length, 0);
    if (failures > 0) {
      throw new Error(`Playwright smoke failed with ${failures} failing check(s). See ${reportMdPath}`);
    }

    console.log(`Playwright smoke passed. Report: ${reportMdPath}`);
  } finally {
    await browser.close();
    await stopManagedServer();
  }
}

main().catch(async (error) => {
  console.error(error);
  await stopManagedServer();
  process.exit(1);
});

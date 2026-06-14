#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const { readdirSync } = require("node:fs");
const { join } = require("node:path");

const scriptPath = process.argv[2];

if (!scriptPath) {
  console.error("Usage: node scripts/run-godot-check.js res://scripts/test.gd | --all");
  process.exit(1);
}

const scripts =
  scriptPath === "--all"
    ? readdirSync(join(process.cwd(), "apps/godot/scripts"))
        .filter((name) => name.endsWith(".gd"))
        .sort()
        .map((name) => `res://scripts/${name}`)
    : [scriptPath];

let failed = false;

for (const script of scripts) {
  const args =
    scriptPath === "--all"
      ? ["--headless", "--check-only", "--path", "apps/godot", "--script", script]
      : ["--headless", "--path", "apps/godot", "--script", script];
  const result = spawnSync(
    "godot",
    args,
    {
      cwd: process.cwd(),
      encoding: "utf8",
      env: process.env,
      timeout: Number(process.env.GODOT_CHECK_TIMEOUT_MS || 120000),
    },
  );

  const output = `${result.stdout || ""}${result.stderr || ""}`;
  const outputLines = output.split(/\r?\n/);
  const printableLines = [];
  const filteredLines = [];
  let skipCleanupTraceLine = false;

  for (const line of outputLines) {
    if (/^ERROR: \d+ resources? still in use at exit/.test(line)) {
      skipCleanupTraceLine = true;
      continue;
    }

    if (skipCleanupTraceLine && /^\s+at: clear /.test(line)) {
      skipCleanupTraceLine = false;
      continue;
    }

    skipCleanupTraceLine = false;
    printableLines.push(line);
    filteredLines.push(line);
  }

  const printableOutput = printableLines.join("\n");
  const filteredOutput = filteredLines.join("\n");

  if (printableOutput.trim().length > 0) {
    process.stdout.write(`${script}\n`);
    process.stdout.write(printableOutput.endsWith("\n") ? printableOutput : `${printableOutput}\n`);
  }

  if (result.error) {
    console.error(result.error.message);
    failed = true;
    continue;
  }

  const hasEngineError = /(^|\n)(SCRIPT ERROR|ERROR|Parse Error|Compile Error):/.test(filteredOutput);

  if (result.status !== 0 || hasEngineError) {
    failed = true;
  }
}

process.exit(failed ? 1 : 0);

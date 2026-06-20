#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");
const { spawn, spawnSync } = require("node:child_process");

const repoRoot = path.resolve(__dirname, "..");
const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";
const nodeCommand = process.execPath;
const disassemblyRoot = process.env.POKECRYSTAL_DISASSEMBLY_ROOT
  ? path.resolve(process.env.POKECRYSTAL_DISASSEMBLY_ROOT)
  : path.join(repoRoot, "vendor", "pokecrystal");

const requiredRuntimeFiles = [
  path.join(repoRoot, "apps", "web", "assets", "data", "pokegear_landmarks.json"),
  path.join(repoRoot, "packages", "core", "assets.manifest.json"),
];
const bootstrapAssets = process.env.POKECRYSTAL_BOOTSTRAP_ASSETS !== "0";

const runChecked = (command, args) => {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    stdio: "inherit",
    env: process.env,
    shell: process.platform === "win32",
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
};

const ensureDisassembly = () => {
  const requiredInput = path.join(disassemblyRoot, "data", "maps", "maps.asm");
  if (fs.existsSync(requiredInput)) {
    return;
  }
  if (process.env.POKECRYSTAL_DISASSEMBLY_ROOT) {
    console.error(`Missing disassembly input: ${requiredInput}`);
    process.exit(1);
  }
  fs.mkdirSync(path.dirname(disassemblyRoot), { recursive: true });
  console.log("[start] fetching pret/pokecrystal runtime sources...");
  runChecked("git", [
    "clone",
    "--depth=1",
    "https://github.com/pret/pokecrystal.git",
    disassemblyRoot,
  ]);
};

const ensureRuntimeAssets = () => {
  if (requiredRuntimeFiles.every((file) => fs.existsSync(file))) {
    return;
  }
  if (!bootstrapAssets) {
    console.error(
      [
        "Runtime assets are missing and POKECRYSTAL_BOOTSTRAP_ASSETS=0 is set.",
        "Run without that environment variable to let the start command fetch and export core assets.",
      ].join("\n")
    );
    process.exit(1);
  }
  ensureDisassembly();
  console.log("[start] generating runtime assets...");
  runChecked(nodeCommand, [path.join(repoRoot, "apps", "web", "scripts", "prepare-public.js")]);
  runChecked(npmCommand, ["run", "export:core"]);
  runChecked(nodeCommand, [path.join(repoRoot, "apps", "web", "scripts", "prepare-public.js")]);
};

const launch = (command, args) => {
  const child = spawn(command, args, {
    cwd: repoRoot,
    stdio: "inherit",
    env: process.env,
    shell: process.platform === "win32",
  });
  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
      return;
    }
    process.exit(code ?? 0);
  });
};

const [target = "tui", ...forwardedArgs] = process.argv.slice(2);
ensureRuntimeAssets();

if (target === "tui") {
  runChecked(npmCommand, ["run", "build:cli"]);
  launch(nodeCommand, [
    path.join(repoRoot, "packages", "cli", "dist", "bin", "pokecrystal-cli.js"),
    "play",
    ...forwardedArgs,
  ]);
} else if (target === "desktop") {
  launch(npmCommand, ["run", "desktop:dev"]);
} else {
  console.error(`Unknown start target: ${target}`);
  process.exit(1);
}

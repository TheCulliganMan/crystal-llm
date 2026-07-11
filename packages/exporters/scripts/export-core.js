#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const Module = require("node:module");
const childProcess = require("node:child_process");
const ts = require("typescript");

const packageRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(packageRoot, "..", "..");

const originalResolve = Module._resolveFilename;
Module._resolveFilename = function resolveWorkspaceSource(request, parent, isMain, options) {
  if (request === "@pokecrystal/core") {
    request = path.join(repoRoot, "packages", "core", "src", "index.ts");
  } else if (request.startsWith("@pokecrystal/core/")) {
    request = path.join(repoRoot, "packages", "core", "src", request.slice("@pokecrystal/core/".length));
  } else if (request === "@pokecrystal/assets") {
    request = path.join(repoRoot, "packages", "assets", "src", "index.ts");
  } else if (request.startsWith("@pokecrystal/assets/")) {
    request = path.join(repoRoot, "packages", "assets", "src", request.slice("@pokecrystal/assets/".length));
  } else if (request === "@pokecrystal/exporters") {
    request = path.join(packageRoot, "src", "index.ts");
  } else if (request.startsWith("@pokecrystal/exporters/")) {
    request = path.join(packageRoot, "src", request.slice("@pokecrystal/exporters/".length));
  }
  return originalResolve.call(this, request, parent, isMain, options);
};

require.extensions[".ts"] = function registerTs(module, filename) {
  const source = fs.readFileSync(filename, "utf8");
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
      esModuleInterop: true,
      resolveJsonModule: true,
    },
    fileName: filename,
  });
  module._compile(outputText, filename);
};

const asmCheck = childProcess.spawnSync(
  process.execPath,
  [path.join(repoRoot, "scripts", "verify-asm-source.mjs")],
  {
    cwd: repoRoot,
    env: { ...process.env, CRYSTAL_CANONICAL_EXPORT: "1" },
    stdio: "inherit",
  },
);
if (asmCheck.error) {
  throw asmCheck.error;
}
if (asmCheck.status !== 0) {
  process.exit(asmCheck.status ?? 1);
}

const runtimeAssetExport = childProcess.spawnSync(
  process.execPath,
  [path.join(repoRoot, "apps", "web", "scripts", "export-runtime-fallbacks.js")],
  { cwd: repoRoot, stdio: "inherit" },
);
if (runtimeAssetExport.error) {
  throw runtimeAssetExport.error;
}
if (runtimeAssetExport.status !== 0) {
  process.exit(runtimeAssetExport.status ?? 1);
}

require(path.join(packageRoot, "src", "exporters")).exportCoreData();

const compiledPackRelativePath = "content-packs/core-modular.crystalpack";
const packCompilerTargetDir = path.join(repoRoot, "rust", "target-pack-core");
const packResult = childProcess.spawnSync(
  "cargo",
  [
    "run",
    "--quiet",
    "--manifest-path",
    path.join(repoRoot, "rust", "Cargo.toml"),
    "-p",
    "crystal-assets",
    "--bin",
    "pack_core",
    "--",
    repoRoot,
    compiledPackRelativePath,
  ],
  {
    cwd: repoRoot,
    env: {
      ...process.env,
      CARGO_TARGET_DIR: packCompilerTargetDir,
    },
    stdio: "inherit",
  }
);
if (packResult.error) {
  throw packResult.error;
}
if (packResult.status !== 0) {
  process.exit(packResult.status ?? 1);
}

const compiledAssetPack = path.join(repoRoot, "apps", "web", "assets", "data", compiledPackRelativePath);
const trackedPack = path.join(repoRoot, compiledPackRelativePath);
fs.mkdirSync(path.dirname(trackedPack), { recursive: true });
fs.copyFileSync(compiledAssetPack, trackedPack);

const packSha256 = crypto
  .createHash("sha256")
  .update(fs.readFileSync(trackedPack))
  .digest("hex");
const sourceLock = JSON.parse(
  fs.readFileSync(path.join(repoRoot, "asm-source.lock.json"), "utf8"),
);
const provenancePath = `${trackedPack}.provenance.json`;
fs.writeFileSync(
  provenancePath,
  `${JSON.stringify(
    {
      schema: 2,
      pack_format: 3,
      pack: path.relative(repoRoot, trackedPack).split(path.sep).join("/"),
      pack_sha256: packSha256,
      asm: {
        repository: sourceLock.repository,
        commit: sourceLock.commit,
        tree: sourceLock.tree,
        input_manifest_sha256: sourceLock.input_manifest_sha256,
        rom_sha1: sourceLock.rom.sha1,
      },
      toolchain: {
        rgbds: sourceLock.rgbds.version,
        exporter: "packages/exporters/scripts/export-core.js",
      },
    },
    null,
    2,
  )}\n`,
);

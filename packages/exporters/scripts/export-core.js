#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");
const Module = require("node:module");
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

require(path.join(packageRoot, "src", "exporters")).exportCoreData();

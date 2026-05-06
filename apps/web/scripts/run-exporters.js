#!/usr/bin/env node

const fs = require("node:fs");
const ts = require("typescript");

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

const entry = require("../../packages/exporters/src");

if (process.argv.includes("--core")) {
  entry.exportCoreData();
} else {
  entry.exportCoreData();
}

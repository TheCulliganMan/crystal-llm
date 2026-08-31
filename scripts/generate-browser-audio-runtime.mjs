import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import ts from "typescript";

const repositoryRoot = path.resolve(import.meta.dirname, "..");
const sourceRoot = path.join(repositoryRoot, "packages/core/src/audio-export");
const outputRoot = path.join(repositoryRoot, "rust/web-client/audio-runtime");
const contextPath = path.join(
  repositoryRoot,
  "apps/web/assets/data/content-packs/core-modular/audio-synth-context.json",
);
const modules = [
  "browser-synth",
  "constants",
  "converter",
  "crystal-midi",
  "midi-recorder",
  "schemas",
  "synthesis",
  "utils",
];

fs.mkdirSync(outputRoot, { recursive: true });
const expected = new Set(modules.map((module) => `${module}.js`));
expected.add("context.js");
for (const entry of fs.readdirSync(outputRoot, { withFileTypes: true })) {
  if (entry.isFile() && entry.name.endsWith(".js") && !expected.has(entry.name)) {
    fs.rmSync(path.join(outputRoot, entry.name));
  }
}

for (const module of modules) {
  const sourcePath = path.join(sourceRoot, `${module}.ts`);
  const source = fs.readFileSync(sourcePath, "utf8");
  const transpiled = ts.transpileModule(source, {
    fileName: sourcePath,
    compilerOptions: {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.ES2022,
      removeComments: true,
    },
  }).outputText.replace(
    /(from\s+["']\.\/[^"'.]+)(["'])/g,
    "$1.js$2",
  );
  fs.writeFileSync(path.join(outputRoot, `${module}.js`), transpiled);
}

const context = JSON.parse(fs.readFileSync(contextPath, "utf8"));
fs.writeFileSync(
  path.join(outputRoot, "context.js"),
  `export default ${JSON.stringify(context)};\n`,
);

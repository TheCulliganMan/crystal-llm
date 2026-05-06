const fs = require("node:fs");
const path = require("node:path");

const repoRoot = path.resolve(__dirname, "..", "..", "..");

const stripComments = (source) =>
  source
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split(/\r?\n/)
    .map((line) => line.replace(/\/\/.*$/, ""))
    .join("\n");

const auditedFiles = [
  "packages/core/src/engine/world/maps.ts",
  "packages/core/src/engine/world/map-music.ts",
  "packages/core/src/engine/world/overworld/collision-data.ts",
  "packages/core/src/engine/world/overworld/collision-rules.ts",
  "packages/core/src/engine/world/overworld/overworld-tileset.ts",
  "packages/core/src/ui/menus/pokegear-labels.ts",
  "packages/core/src/ui/overlays/battle-anim-data.ts",
  "packages/core/src/ui/screens/intro/asm-data.ts",
];

const forbiddenPatterns = [
  /\.asm\b/,
  /\/disassembly\//,
  /getDisassemblyRoot\s*\(/,
  /DISASSEMBLY_ROOT\b/,
];

describe("runtime asm audit", () => {
  test.each(auditedFiles)("%s does not contain executable asm/disassembly paths", (relativePath) => {
    const absolutePath = path.join(repoRoot, relativePath);
    const source = fs.readFileSync(absolutePath, "utf8");
    const stripped = stripComments(source);

    for (const pattern of forbiddenPatterns) {
      expect(stripped).not.toMatch(pattern);
    }
  });
});

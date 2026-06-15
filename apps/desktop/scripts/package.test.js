"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const packageScriptPath = path.join(__dirname, "package.js");
const packageScript = fs.readFileSync(packageScriptPath, "utf8");

test("desktop package stages ASM audio sources for PCM synthesis", () => {
  assert.match(packageScript, /vendor", "pokecrystal", "audio"/);
  assert.match(packageScript, /vendor", "pokecrystal", "constants"/);
  assert.match(packageScript, /audio", "sfx\.asm"/);
});

test("desktop launcher points the bundled server at the packaged disassembly root", () => {
  assert.match(packageScript, /const disassemblyRoot = join\(resourceRoot, "web-standalone", "vendor", "pokecrystal"\)/);
  assert.match(packageScript, /POKECRYSTAL_DISASSEMBLY_ROOT: disassemblyRoot/);
});

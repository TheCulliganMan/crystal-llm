"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const packageScriptPath = path.join(__dirname, "package.js");
const menuPatchScriptPath = path.join(__dirname, "patch-zero-native-menu.js");
const packageScript = fs.readFileSync(packageScriptPath, "utf8");
const menuPatchScript = fs.readFileSync(menuPatchScriptPath, "utf8");

test("desktop package stages ASM audio sources for PCM synthesis", () => {
  assert.match(packageScript, /vendor", "pokecrystal", "audio"/);
  assert.match(packageScript, /vendor", "pokecrystal", "constants"/);
  assert.match(packageScript, /audio", "sfx\.asm"/);
});

test("desktop launcher points the bundled server at the packaged disassembly root", () => {
  assert.match(packageScript, /const disassemblyRoot = join\(resourceRoot, "web-standalone", "vendor", "pokecrystal"\)/);
  assert.match(packageScript, /POKECRYSTAL_DISASSEMBLY_ROOT: disassemblyRoot/);
});

test("desktop package skips generated audio test fixtures", () => {
  assert.match(packageScript, /part === "__tests__"/);
});

test("desktop build patches Zero Native macOS menu for MCP configuration", () => {
  assert.match(packageScript, /patchZeroNativeMenu\(\)/);
  assert.match(menuPatchScript, /MCP Streamable HTTP/);
  assert.match(menuPatchScript, /window\.location\.assign/);
  assert.match(menuPatchScript, /openKrabbyclawMcp/);
  assert.match(menuPatchScript, /\/mcp/);
  assert.match(menuPatchScript, /\/desktop\?panel=saves/);
});

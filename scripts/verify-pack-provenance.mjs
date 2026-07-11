import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const packPath = resolve(root, "content-packs/core-modular.crystalpack");
const provenancePath = `${packPath}.provenance.json`;
const lock = JSON.parse(readFileSync(resolve(root, "asm-source.lock.json"), "utf8"));
const provenance = JSON.parse(readFileSync(provenancePath, "utf8"));
const packBytes = readFileSync(packPath);
const digest = createHash("sha256").update(packBytes).digest("hex");
const expectedPackFormat = 3;
const magic = Buffer.from("CRYSTALPACK\0", "ascii");
const headerFormat = packBytes.length >= 14 ? packBytes.readUInt16BE(magic.length) : null;

const checks = [
  [packBytes.subarray(0, magic.length).equals(magic), "pack magic mismatch"],
  [headerFormat === expectedPackFormat, `pack header format mismatch: ${headerFormat}`],
  [provenance.schema === 2, `provenance schema mismatch: ${provenance.schema}`],
  [provenance.pack_format === expectedPackFormat, `provenance pack format mismatch: ${provenance.pack_format}`],
  [digest === provenance.pack_sha256, `pack SHA-256 mismatch: ${digest}`],
  [provenance.asm?.commit === lock.commit, "ASM commit mismatch"],
  [provenance.asm?.tree === lock.tree, "ASM tree mismatch"],
  [provenance.asm?.input_manifest_sha256 === lock.input_manifest_sha256, "ASM input manifest mismatch"],
  [provenance.asm?.rom_sha1 === lock.rom.sha1, "ROM SHA-1 mismatch"],
];
const failure = checks.find(([ok]) => !ok);
if (failure) {
  console.error(`Pack provenance verification failed: ${failure[1]}`);
  process.exit(1);
}
console.log(`Pack provenance verified: ${digest}`);

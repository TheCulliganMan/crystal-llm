import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");

test("the provenance verifier accepts the current canonical pack format", () => {
  const result = spawnSync(process.execPath, ["scripts/verify-pack-provenance.mjs"], {
    cwd: root,
    encoding: "utf8",
  });

  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /^Pack provenance verified: [0-9a-f]{64}\n$/);
});

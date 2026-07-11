import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, statSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const lockPath = resolve(root, "asm-source.lock.json");
const lock = JSON.parse(readFileSync(lockPath, "utf8"));
const sourceRoot = resolve(root, lock.repository);

function git(...args) {
  return execFileSync("git", ["-C", sourceRoot, ...args], { encoding: "utf8" }).trim();
}

function fail(message) {
  console.error(`ASM source verification failed: ${message}`);
  process.exitCode = 1;
}

try {
  if (!statSync(sourceRoot).isDirectory()) {
    fail(`source directory ${lock.repository} is not a directory`);
  } else {
    const commit = git("rev-parse", "HEAD");
    const tree = git("rev-parse", "HEAD^{tree}");
    const dirty = git("status", "--porcelain")
      .split("\n")
      .filter((entry) => entry && !entry.endsWith(" pokecrystal.gbc.ram"))
      .join("\n");
    if (commit !== lock.commit) fail(`expected commit ${lock.commit}, found ${commit}`);
    if (tree !== lock.tree) fail(`expected tree ${lock.tree}, found ${tree}`);
    if (dirty) fail("source checkout has uncommitted changes");

    const files = git("ls-files", "-z").split("\0").filter(Boolean);
    const manifest = files
      .map((file) => {
        const digest = createHash("sha256")
          .update(readFileSync(resolve(sourceRoot, file)))
          .digest("hex");
        return `${digest}  ${file}\n`;
      })
      .join("");
    const manifestDigest = createHash("sha256").update(manifest).digest("hex");
    if (manifestDigest !== lock.input_manifest_sha256) {
      fail(
        `expected input manifest SHA-256 ${lock.input_manifest_sha256}, found ${manifestDigest}`,
      );
    }

    const romPath = resolve(sourceRoot, lock.rom.path);
    try {
      const digest = createHash("sha1").update(readFileSync(romPath)).digest("hex");
      if (digest !== lock.rom.sha1) {
        fail(`expected ROM SHA-1 ${lock.rom.sha1}, found ${digest}`);
      }
    } catch {
      fail(`reference ROM is missing at ${lock.repository}/${lock.rom.path}; run make ${lock.target}`);
    }

    for (const tool of ["rgbasm", "rgblink", "rgbfix", "rgbgfx"]) {
      try {
        const version = execFileSync(tool, ["--version"], { encoding: "utf8" });
        if (!version.includes(lock.rgbds.version)) {
          fail(`expected ${tool} from RGBDS ${lock.rgbds.version}, found ${version.trim()}`);
        }
      } catch {
        fail(`${tool} is unavailable; install RGBDS ${lock.rgbds.version}`);
      }
    }
  }
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
}

if (process.exitCode !== 1) {
  console.log(`ASM source verified: ${lock.commit} (${lock.target}, RGBDS ${lock.rgbds.version})`);
}

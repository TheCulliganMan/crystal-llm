const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

jest.mock("node:child_process", () => ({
  spawn: jest.fn(() => ({
    killed: false,
    kill: jest.fn(),
    on: jest.fn(),
  })),
}));

jest.mock("../scripts/run-next.js", () => ({
  resolveNextBin: jest.fn(() => "/mock/next"),
}));

jest.mock("../scripts/prepare-public.js", () => ({
  preparePublic: jest.fn(),
}));

const { resolveNextLogPath } = require("../scripts/dev-with-logs.js");
const { cleanNextArtifacts } = require("../scripts/clean-next-artifacts.js");

describe("dev-with-logs", () => {
  test("resolveNextLogPath honors the configured Next dist dir", () => {
    expect(resolveNextLogPath("/repo/apps/web", { POKECRYSTAL_NEXT_DIST_DIR: ".next-dev" })).toBe(
      path.join("/repo/apps/web", ".next-dev", "dev", "logs", "next-development.log")
    );
  });
});

describe("clean-next-artifacts", () => {
  test("removes stale lockfiles and duplicate artifacts from the configured dist dir", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "poke-next-artifacts-"));

    try {
      const nextDir = path.join(tempRoot, ".next-dev");
      fs.mkdirSync(path.join(nextDir, "server"), { recursive: true });
      fs.writeFileSync(path.join(nextDir, "lock"), "");
      fs.writeFileSync(path.join(nextDir, "server", "middleware-manifest.json"), "{}");
      fs.writeFileSync(path.join(nextDir, "server", "middleware-manifest.json 2"), "{}");

      cleanNextArtifacts(tempRoot, { POKECRYSTAL_NEXT_DIST_DIR: ".next-dev" });

      expect(fs.existsSync(path.join(nextDir, "lock"))).toBe(false);
      expect(fs.existsSync(path.join(nextDir, "server", "middleware-manifest.json"))).toBe(true);
      expect(fs.existsSync(path.join(nextDir, "server", "middleware-manifest.json 2"))).toBe(false);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });
});

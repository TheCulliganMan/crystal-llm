const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const {
  packageNeedsBuild,
  syncWorkspaceRuntime,
} = require("../scripts/sync-workspace-runtime.js");

const makeDir = (dir) => fs.mkdirSync(dir, { recursive: true });

describe("sync-workspace-runtime", () => {
  test("rebuilds when dist exists but a required emitted runtime file is missing", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "poke-sync-runtime-"));

    try {
      const pkgDir = path.join(tempRoot, "pkg");
      const srcDir = path.join(pkgDir, "src");
      const distDir = path.join(pkgDir, "dist");
      makeDir(srcDir);
      makeDir(distDir);
      fs.writeFileSync(path.join(pkgDir, "package.json"), "{}\n");
      fs.writeFileSync(path.join(srcDir, "index.ts"), "export {};\n");

      const pkg = {
        name: "@test/pkg",
        dir: pkgDir,
        srcDirs: ["src"],
        requiredOutputs: ["dist/index.js", "dist/runtime.js"],
      };

      fs.writeFileSync(path.join(pkgDir, "dist", "index.js"), "module.exports = {};\n");

      expect(packageNeedsBuild(pkg)).toBe(true);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("skips rebuild when required outputs exist and dist is newer than source", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "poke-sync-runtime-"));

    try {
      const pkgDir = path.join(tempRoot, "pkg");
      const srcDir = path.join(pkgDir, "src");
      const distDir = path.join(pkgDir, "dist");
      makeDir(srcDir);
      makeDir(distDir);
      fs.writeFileSync(path.join(pkgDir, "package.json"), "{}\n");
      fs.writeFileSync(path.join(srcDir, "index.ts"), "export {};\n");
      fs.writeFileSync(path.join(distDir, "index.js"), "module.exports = {};\n");
      fs.writeFileSync(path.join(distDir, "runtime.js"), "module.exports = {};\n");

      const newer = new Date("2030-01-01T00:00:00.000Z");
      fs.utimesSync(distDir, newer, newer);
      fs.utimesSync(path.join(distDir, "index.js"), newer, newer);
      fs.utimesSync(path.join(distDir, "runtime.js"), newer, newer);

      const older = new Date("2020-01-01T00:00:00.000Z");
      fs.utimesSync(pkgDir, older, older);
      fs.utimesSync(path.join(pkgDir, "package.json"), older, older);
      fs.utimesSync(srcDir, older, older);
      fs.utimesSync(path.join(srcDir, "index.ts"), older, older);

      const pkg = {
        name: "@test/pkg",
        dir: pkgDir,
        srcDirs: ["src"],
        requiredOutputs: ["dist/index.js", "dist/runtime.js"],
      };

      expect(packageNeedsBuild(pkg)).toBe(false);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("syncWorkspaceRuntime only builds packages that need it", () => {
    const needsBuild = { name: "@test/needs", dir: "/tmp/needs" };
    const fresh = { name: "@test/fresh", dir: "/tmp/fresh" };

    const buildCalls = [];
    const log = jest.fn();

    syncWorkspaceRuntime([needsBuild, fresh], {
      packageNeedsBuild: (pkg) => pkg.name === needsBuild.name,
      runBuild: (pkg) => buildCalls.push(pkg.name),
      log,
    });

    expect(buildCalls).toEqual([needsBuild.name]);
    expect(log).toHaveBeenCalledWith(`[sync-workspace-runtime] building ${needsBuild.name}`);
    expect(log).toHaveBeenCalledWith(`[sync-workspace-runtime] ${fresh.name} already up to date`);
  });
});

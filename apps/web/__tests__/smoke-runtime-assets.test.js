const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const { REQUIRED_RUNTIME_JSON_PATHS, smokeRoot } = require("../scripts/smoke-runtime-assets");

const writeRequiredRuntimeJsonAssets = (rootDir, overrides = {}) => {
  for (const relativePath of REQUIRED_RUNTIME_JSON_PATHS) {
    const targetPath = path.join(rootDir, relativePath);
    fs.mkdirSync(path.dirname(targetPath), { recursive: true });
    fs.writeFileSync(targetPath, overrides[relativePath] ?? "{}\n");
  }
};

describe("smoke-runtime-assets", () => {
  let tempDir;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-asset-smoke-"));
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("fails when a required runtime JSON asset is missing", () => {
    writeRequiredRuntimeJsonAssets(tempDir);
    fs.rmSync(path.join(tempDir, "data", "flee_mons.json"));

    expect(() => smokeRoot(tempDir)).toThrow(
      "missing required runtime JSON assets: data/flee_mons.json"
    );
  });

  it("fails when a JSON asset is empty", () => {
    writeRequiredRuntimeJsonAssets(tempDir, {
      "data/flee_mons.json": "",
    });

    expect(() => smokeRoot(tempDir)).toThrow("empty JSON file");
  });

  it("accepts present, non-empty required runtime JSON assets", () => {
    writeRequiredRuntimeJsonAssets(tempDir);

    expect(smokeRoot(tempDir)).toEqual(
      expect.objectContaining({
        rootDir: tempDir,
        fileCount: REQUIRED_RUNTIME_JSON_PATHS.length,
      })
    );
  });
});

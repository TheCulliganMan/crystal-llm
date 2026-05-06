import fs from "fs";
import os from "os";
import path from "path";
import { removeMatchingOutputs } from "./asm-utils";

let mockAssetsRoot = "";

jest.mock("@pokecrystal/core/core/paths", () => ({
  getAssetsRoot: () => mockAssetsRoot,
}));

describe("asm-utils output cleanup", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-asm-utils-"));
    mockAssetsRoot = path.join(tempDir, "assets");
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("removes matching generated outputs recursively without deleting other files", () => {
    const dataDir = path.join(mockAssetsRoot, "data");
    const generatedDir = path.join(dataDir, "content-packs", "core-modular");
    fs.mkdirSync(path.join(generatedDir, "maps"), { recursive: true });
    fs.mkdirSync(path.join(generatedDir, "gfx"), { recursive: true });
    fs.writeFileSync(path.join(generatedDir, "root.json"), "{}");
    fs.writeFileSync(path.join(generatedDir, "maps", "stale.json"), "{}");
    fs.writeFileSync(path.join(generatedDir, "gfx", "keep.png"), "png");

    removeMatchingOutputs("content-packs/core-modular", ".json");

    expect(fs.existsSync(path.join(generatedDir, "root.json"))).toBe(false);
    expect(fs.existsSync(path.join(generatedDir, "maps", "stale.json"))).toBe(false);
    expect(fs.readFileSync(path.join(generatedDir, "gfx", "keep.png"), "utf8")).toBe("png");
  });
});

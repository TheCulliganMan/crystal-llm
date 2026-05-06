import fs from "node:fs";
import path from "node:path";

describe("game-engine imports", () => {
  it("avoids node: scheme imports for zlib to keep webpack compatible", () => {
    const sourcePath = path.join(__dirname, "game-engine.ts");
    const source = fs.readFileSync(sourcePath, "utf8");

    expect(source).not.toContain("node:zlib");
  });
});

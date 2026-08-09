import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

describe("root export launcher", () => {
  const repoRoot = path.resolve(__dirname, "../../..");
  const launcher = path.join(repoRoot, "export");

  it("is executable and can be invoked without npm from any directory", () => {
    fs.accessSync(launcher, fs.constants.X_OK);

    const result = spawnSync(launcher, ["--help"], {
      cwd: os.tmpdir(),
      encoding: "utf8",
      env: { ...process.env, npm_execpath: "", npm_node_execpath: "" },
    });

    expect(result.status).toBe(0);
    expect(result.stderr).toBe("");
    expect(result.stdout).toContain("Usage: ./export");
  });

  it("delegates the export exclusively to the Rust pack exporter", () => {
    const source = fs.readFileSync(launcher, "utf8");

    expect(source).toContain("cargo run --quiet");
    expect(source).toContain("--bin pack_core");
    expect(source).not.toMatch(/\b(?:node|npm|npx)\b/);
    expect(source).not.toContain("packages/exporters");
  });
});

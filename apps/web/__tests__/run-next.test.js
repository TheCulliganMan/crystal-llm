const fs = require("node:fs");
const path = require("node:path");

jest.mock("node:child_process", () => ({
  spawnSync: jest.fn(() => ({ status: 0 })),
}));

const { resolveNextBin, runNext } = require("../scripts/run-next.js");
const { spawnSync } = require("node:child_process");

describe("run-next", () => {
  test("resolveNextBin returns the Next CLI path", () => {
    const bin = resolveNextBin();
    expect(bin).toEqual(expect.stringContaining(path.join("next", "dist", "bin", "next")));
    expect(fs.existsSync(bin)).toBe(true);
  });

  test("runNext delegates to node with the Next CLI", () => {
    const status = runNext(["build"]);
    expect(status).toBe(0);
    expect(spawnSync).toHaveBeenCalledTimes(1);
    const [nodeBin, args, opts] = spawnSync.mock.calls[0];
    expect(nodeBin).toBe(process.execPath);
    expect(args[0]).toEqual(expect.stringContaining(path.join("next", "dist", "bin", "next")));
    expect(args.slice(1)).toEqual(["build"]);
    expect(opts).toEqual({ stdio: "inherit" });
  });
});

const { resolveNextDistDir } = require("../scripts/run-with-bundled-node.js");

describe("run-with-bundled-node", () => {
  test("defaults to Next.js distDir when no override is provided", () => {
    expect(resolveNextDistDir({})).toBeNull();
    expect(resolveNextDistDir({ npm_lifecycle_event: "build" })).toBeNull();
    expect(resolveNextDistDir({ npm_lifecycle_event: "vercel-build", VERCEL: "1" })).toBeNull();
  });

  test("honors an explicit distDir override", () => {
    expect(resolveNextDistDir({ POKECRYSTAL_NEXT_DIST_DIR: ".next-build" })).toBe(".next-build");
  });
});

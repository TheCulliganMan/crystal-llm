import fs from "node:fs";
import path from "node:path";

const REPO_ROOT = path.resolve(__dirname, "../../../../../../");
const SOURCE_ROOTS = [
  path.join(REPO_ROOT, "packages/core/src"),
  path.join(REPO_ROOT, "apps/web/src"),
];

const SKIP_SEGMENTS = [
  `${path.sep}__tests__${path.sep}`,
  ".test.ts",
  ".test.tsx",
  `${path.sep}dist${path.sep}`,
];

const DIRECT_PLAY_MUSIC_PATTERN = /\.\s*(playMusic|play_music)\(\s*[^,\n)]+?\s*\)/g;

const collectSourceFiles = (root: string): string[] => {
  const results: string[] = [];
  const stack = [root];
  while (stack.length > 0) {
    const current = stack.pop();
    if (!current) {
      continue;
    }
    const stat = fs.statSync(current);
    if (stat.isDirectory()) {
      const entries = fs.readdirSync(current);
      for (const entry of entries) {
        stack.push(path.join(current, entry));
      }
      continue;
    }
    if (!current.endsWith(".ts") && !current.endsWith(".tsx")) {
      continue;
    }
    if (SKIP_SEGMENTS.some((segment) => current.includes(segment))) {
      continue;
    }
    results.push(current);
  }
  return results.sort();
};

describe("audio callsite audit", () => {
  it("requires explicit roles on direct engine music calls in production code", () => {
    const offenders: string[] = [];

    for (const root of SOURCE_ROOTS) {
      for (const file of collectSourceFiles(root)) {
        const source = fs.readFileSync(file, "utf8");
        const matches = source.matchAll(DIRECT_PLAY_MUSIC_PATTERN);
        for (const match of matches) {
          const snippet = match[0];
          const offset = match.index ?? 0;
          const line = source.slice(0, offset).split("\n").length;
          offenders.push(`${path.relative(REPO_ROOT, file)}:${line} ${snippet}`);
        }
      }
    }

    expect(offenders).toEqual([]);
  });
});

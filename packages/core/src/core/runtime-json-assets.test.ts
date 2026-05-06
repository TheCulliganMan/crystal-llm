import fs from "fs";
import path from "path";

const REPO_ROOT = path.resolve(__dirname, "..", "..", "..", "..");
const DATA_DIR = path.join(REPO_ROOT, "apps", "web", "assets", "data");
const SOURCE_DIRS = [
  path.join(REPO_ROOT, "packages", "core", "src"),
  path.join(REPO_ROOT, "apps", "web", "src"),
];
const OPTIONAL_ROOT_JSON_ASSETS = new Set([
  // Root story_events.json is a legacy aggregate; current exports are modularized
  // into data/maps/*.json and data/story_events/*.json.
  "story_events.json",
]);

const walkTsFiles = (dirPath: string): string[] => {
  if (!fs.existsSync(dirPath)) {
    return [];
  }
  const result: string[] = [];
  for (const entry of fs.readdirSync(dirPath, { withFileTypes: true })) {
    const entryPath = path.join(dirPath, entry.name);
    if (entry.isDirectory()) {
      result.push(...walkTsFiles(entryPath));
    } else if (entry.isFile() && entry.name.endsWith(".ts")) {
      result.push(entryPath);
    }
  }
  return result;
};

const collectLiteralRuntimeJsonRefs = (): Map<string, string[]> => {
  const refs = new Map<string, string[]>();
  const patterns = [
    /joinPath\(getDataDir\(\),\s*[`'"]([^`'"]+\.json)[`'"]/g,
    /path\.join\(getDataDir\(\),\s*[`'"]([^`'"]+\.json)[`'"]/g,
    /path\.join\(DATA_DIR,\s*[`'"]([^`'"]+\.json)[`'"]/g,
    /joinPath\(DATA_DIR,\s*[`'"]([^`'"]+\.json)[`'"]/g,
    /[`'"]\/assets\/data\/([^`'"]+\.json)[`'"]/g,
    /[`'"]assets\/data\/([^`'"]+\.json)[`'"]/g,
  ];

  for (const filePath of SOURCE_DIRS.flatMap(walkTsFiles)) {
    const source = fs.readFileSync(filePath, "utf8");
    for (const pattern of patterns) {
      for (const match of source.matchAll(pattern)) {
        const relativePath = match[1]
          .replace(/^\/?assets\/data\//, "")
          .replace(/^\/+/, "");
        if (!relativePath.endsWith(".json")) {
          continue;
        }
        const sources = refs.get(relativePath) ?? [];
        sources.push(path.relative(REPO_ROOT, filePath));
        refs.set(relativePath, sources);
      }
    }
  }

  return refs;
};

describe("runtime JSON assets", () => {
  it("has an exported file for every required literal runtime JSON read", () => {
    const missing: string[] = [];
    for (const [relativePath, sources] of collectLiteralRuntimeJsonRefs()) {
      if (OPTIONAL_ROOT_JSON_ASSETS.has(relativePath)) {
        continue;
      }
      if (!fs.existsSync(path.join(DATA_DIR, relativePath))) {
        missing.push(`${relativePath}\n  ${Array.from(new Set(sources)).join("\n  ")}`);
      }
    }

    expect(missing).toEqual([]);
  });

  it("keeps the lower Azalea rival trigger connected to the rival battle script", () => {
    const relativePaths = [
      path.join("maps", "AzaleaTown.json"),
      path.join("content-packs", "core-modular", "maps", "azaleatown.json"),
    ];

    for (const relativePath of relativePaths) {
      const payload = JSON.parse(
        fs.readFileSync(path.join(DATA_DIR, relativePath), "utf8")
      ) as Record<string, Array<{ command?: string; args?: string[] }>>;
      const scene = payload.AzaleaTownRivalBattleScene2;
      const handoff = scene?.[scene.length - 1];

      expect(handoff).toEqual({
        command: "sjump",
        args: ["AzaleaTownRivalBattleScript"],
      });
    }
  });
});

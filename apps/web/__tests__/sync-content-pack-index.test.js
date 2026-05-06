const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { syncContentPackIndex } = require("../scripts/sync-content-pack-index.js");

const makeDir = (dir) => fs.mkdirSync(dir, { recursive: true });

describe("sync-content-pack-index", () => {
  test("writes the generated index only to the canonical assets tree", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-content-pack-index-"));

    try {
      const assetsDataRoot = path.join(tempRoot, "assets", "data");
      const packRoot = path.join(assetsDataRoot, "content-packs", "johto-plus");

      makeDir(packRoot);
      fs.writeFileSync(
        path.join(packRoot, "pack.json"),
        JSON.stringify({ id: "johto-plus", enabled: true, priority: 7 }, null, 2)
      );
      makeDir(path.join(packRoot, "pokemon"));
      fs.writeFileSync(
        path.join(packRoot, "pokemon", "totodile.json"),
        JSON.stringify({ species: "TOTODILE" }, null, 2)
      );
      for (const category of ["npcs", "items", "trainers", "pokedex", "story_events", "phone_scripts"]) {
        makeDir(path.join(packRoot, category));
        fs.writeFileSync(
          path.join(packRoot, category, "sample.json"),
          JSON.stringify({ category }, null, 2)
        );
      }

      const result = syncContentPackIndex({
        projectRoot: tempRoot,
        assetsSource: path.join(tempRoot, "assets"),
      });

      const assetsIndexPath = path.join(assetsDataRoot, "content-packs", "index.json");

      expect(result.changed).toBe(true);
      expect(result.outputPaths).toEqual([assetsIndexPath]);

      const assetsIndex = JSON.parse(fs.readFileSync(assetsIndexPath, "utf8"));

      expect(assetsIndex.packs).toEqual([
        expect.objectContaining({
          id: "johto-plus",
          priority: 7,
          files: expect.objectContaining({
            pokemon: ["content-packs/johto-plus/pokemon/totodile.json"],
            npcs: ["content-packs/johto-plus/npcs/sample.json"],
            items: ["content-packs/johto-plus/items/sample.json"],
            trainers: ["content-packs/johto-plus/trainers/sample.json"],
            pokedex: ["content-packs/johto-plus/pokedex/sample.json"],
            story_events: ["content-packs/johto-plus/story_events/sample.json"],
            phone_scripts: ["content-packs/johto-plus/phone_scripts/sample.json"],
          }),
        }),
      ]);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  test("preserves compiled metadata from the existing index for matching pack ids", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-content-pack-index-"));

    try {
      const assetsDataRoot = path.join(tempRoot, "assets", "data");
      const contentPacksRoot = path.join(assetsDataRoot, "content-packs");
      const packRoot = path.join(contentPacksRoot, "core-modular");

      makeDir(packRoot);
      fs.writeFileSync(
        path.join(contentPacksRoot, "index.json"),
        JSON.stringify(
          {
            version: 1,
            packs: [
              {
                id: "core-modular",
                enabled: true,
                priority: -100,
                compiled: "content-packs/core-modular.compiled.json",
                files: {},
              },
            ],
          },
          null,
          2
        )
      );
      fs.writeFileSync(
        path.join(packRoot, "pack.json"),
        JSON.stringify({ id: "core-modular", enabled: true, priority: -100 }, null, 2)
      );
      makeDir(path.join(packRoot, "pokemon"));
      fs.writeFileSync(
        path.join(packRoot, "pokemon", "totodile.json"),
        JSON.stringify({ id: "TOTODILE" }, null, 2)
      );

      syncContentPackIndex({
        projectRoot: tempRoot,
        assetsSource: path.join(tempRoot, "assets"),
      });

      const assetsIndex = JSON.parse(
        fs.readFileSync(path.join(contentPacksRoot, "index.json"), "utf8")
      );

      expect(assetsIndex.packs).toEqual([
        expect.objectContaining({
          id: "core-modular",
          compiled: "content-packs/core-modular.compiled.json",
          files: expect.objectContaining({
            pokemon: ["content-packs/core-modular/pokemon/totodile.json"],
          }),
        }),
      ]);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });
});

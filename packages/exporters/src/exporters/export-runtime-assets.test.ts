import fs from "fs";
import os from "os";
import path from "path";
import { exportFleeMons, exportRuntimeAssets } from "./export-runtime-assets";

let mockDisassemblyRoot = "";
let mockAssetsRoot = "";

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDisassemblyRoot: () => mockDisassemblyRoot,
  getAssetsRoot: () => mockAssetsRoot,
}));

const writeFile = (filePath: string, content: string): void => {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content);
};

describe("exportRuntimeAssets", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-runtime-export-"));
    mockDisassemblyRoot = path.join(tempDir, "vendor");
    mockAssetsRoot = path.join(tempDir, "assets");

    writeFile(
      path.join(mockDisassemblyRoot, "data", "wild", "flee_mons.asm"),
      [
        "AlwaysFleeMons:",
        "\tdb RAIKOU",
        "\tdb ENTEI",
        "\tdb -1",
        "",
        "OftenFleeMons:",
        "\tdb DELIBIRD ; comment",
        "\tdb -1",
        "",
        "SometimesFleeMons:",
        "\tdb MAGNEMITE",
        "\tdb -1",
        "",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "marts.asm"),
      ["MartCherrygrove:", "\tdb 2", "\tdb POTION", "\tdb ANTIDOTE", "\tdb -1", ""].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "pokemon", "bills_pc.asm"),
      'PCString_ChooseaPKMN: db "Choose a <PK><MN>.@"\n'
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "pokemon", "menu_icons.asm"),
      "\tdb ICON_CHIKORITA ; CHIKORITA\n"
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "pokemon", "dex_entries", "chikorita.asm"),
      [
        '\tdb "LEAF@"',
        "\tdw 211, 140 ; height, weight",
        '\tdb "A sweet aroma"',
        '\tnext "gently wafts@"',
        '\tpage "from the leaf"',
        '\tnext "on its head.@"',
        "",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "gfx", "pokemon", "chikorita", "anim.asm"),
      ["\tframe 1, 07", "\tsetrepeat 2", "\tframe 0, 05", "\tdorepeat 1", "\tendanim", ""].join("\n")
    );
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("exports required runtime JSON assets including flee_mons.json", () => {
    exportRuntimeAssets();

    const dataDir = path.join(mockAssetsRoot, "data");
    const fleeMons = JSON.parse(fs.readFileSync(path.join(dataDir, "flee_mons.json"), "utf8"));
    const pokedexEntries = JSON.parse(fs.readFileSync(path.join(dataDir, "pokedex_entries.json"), "utf8"));
    const frontpicAnimations = JSON.parse(fs.readFileSync(path.join(dataDir, "pokemon_frontpic_anim.json"), "utf8"));

    expect(fleeMons).toEqual({
      always: ["RAIKOU", "ENTEI"],
      often: ["DELIBIRD"],
      sometimes: ["MAGNEMITE"],
    });
    expect(pokedexEntries[0]).toMatchObject({
      species: "CHIKORITA",
      classification: "LEAF",
      pages: ["A sweet aroma @ gently wafts", "from the leaf @ on its head."],
    });
    expect(frontpicAnimations.chikorita.commands).toEqual([
      { kind: "frame", frame: 1, duration: 7 },
      { kind: "setrepeat", count: 2 },
      { kind: "frame", frame: 0, duration: 5 },
      { kind: "dorepeat", target: 1 },
      { kind: "endanim" },
    ]);
    for (const fileName of [
      "flee_mons.json",
      "marts.json",
      "pc_strings.json",
      "menu_icons.json",
      "pokedex_entries.json",
      "pokemon_frontpic_anim.json",
    ]) {
      const targetPath = path.join(dataDir, fileName);
      expect(fs.existsSync(targetPath)).toBe(true);
      expect(fs.readFileSync(targetPath, "utf8").trim()).not.toBe("");
    }
  });

  it("fails instead of exporting empty flee tables when required labels are missing", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "wild", "flee_mons.asm"),
      ["AlwaysFleeMons:", "\tdb RAIKOU", "\tdb -1", ""].join("\n")
    );

    expect(() => exportFleeMons()).toThrow("Could not parse required OftenFleeMons table");
  });
});

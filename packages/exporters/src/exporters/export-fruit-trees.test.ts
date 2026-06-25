import fs from "fs";
import os from "os";
import path from "path";
import { exportFruitTrees } from "./export-fruit-trees";

var mockDisassemblyRoot = "";
var mockAssetsRoot = "";

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDisassemblyRoot: () => mockDisassemblyRoot || "/mock/pokecrystal",
  getAssetsRoot: () => mockAssetsRoot || "/mock/assets",
}));

const writeFile = (filePath: string, content: string): void => {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content);
};

describe("exportFruitTrees", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-fruit-tree-export-"));
    mockDisassemblyRoot = path.join(tempDir, "vendor");
    mockAssetsRoot = path.join(tempDir, "assets");
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("exports exact fruit tree ids and item rows from matching ASM tables", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "script_constants.asm"),
      [
        "; fruittree arguments",
        "const_def 1",
        "\tconst FRUITTREE_ROUTE_29",
        "\tconst FRUITTREE_ROUTE_30_1",
        "\tconst FRUITTREE_ROUTE_30_2",
        "DEF NUM_FRUIT_TREES EQU const_value - 1",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "fruit_trees.asm"),
      [
        "FruitTreeItems:",
        "\ttable_width 1",
        "\tdb BERRY        ; ROUTE_29",
        "\tdb BERRY        ; ROUTE_30_1",
        "\tdb PSNCUREBERRY ; ROUTE_30_2",
        "\tassert_table_length NUM_FRUIT_TREES",
      ].join("\n")
    );

    expect(exportFruitTrees()).toEqual({
      FRUITTREE_ROUTE_29: "BERRY",
      FRUITTREE_ROUTE_30_1: "BERRY",
      FRUITTREE_ROUTE_30_2: "PSNCUREBERRY",
    });
  });

  it("rejects mismatched fruit tree constants and item rows", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "script_constants.asm"),
      ["const_def 1", "\tconst FRUITTREE_ROUTE_29", "\tconst FRUITTREE_ROUTE_30_1", "DEF NUM_FRUIT_TREES EQU const_value - 1"].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "items", "fruit_trees.asm"),
      ["FruitTreeItems:", "\ttable_width 1", "\tdb BERRY", "\tassert_table_length NUM_FRUIT_TREES"].join("\n")
    );

    expect(() => exportFruitTrees()).toThrow("Fruit tree constant count 2 does not match item row count 1");
  });
});

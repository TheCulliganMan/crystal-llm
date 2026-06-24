import fs from "fs";
import os from "os";
import path from "path";
import { exportPokedex, iterPokedexEntryPaths, parsePokedexEntry } from "./export-pokedex";

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

describe("export-pokedex", () => {
  let tempDir: string;
  let entryPath: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-pokedex-export-"));
    mockDisassemblyRoot = path.join(tempDir, "vendor");
    mockAssetsRoot = path.join(tempDir, "assets");
    entryPath = path.join(mockDisassemblyRoot, "data", "pokemon", "dex_entries", "chikorita.asm");

    writeFile(path.join(mockDisassemblyRoot, "engine", "pokedex", "pokedex.asm"), "PokedexPlaceholder:\n");
    writeFile(path.join(mockDisassemblyRoot, "data", "pokemon", "dex_entries.asm"), 'INCLUDE "data/pokemon/dex_entries/chikorita.asm"\n');
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("exports Pokédex entries from explicit include order", () => {
    writeFile(
      entryPath,
      ['\tdb "LEAF@"', "\tdw 211, 140", '\tdb "A sweet aroma"', '\tnext "gently wafts@"', ""].join("\n")
    );

    expect(iterPokedexEntryPaths()).toEqual([entryPath]);
    expect(exportPokedex()).toEqual([
      expect.objectContaining({
        species: "CHIKORITA",
        classification: "LEAF",
        height: 0.89,
        weight: 6.35,
        text: "A sweet aroma gently wafts",
      }),
    ]);
  });

  it("rejects malformed text literals instead of dropping them from the export", () => {
    writeFile(entryPath, ['\tdb "LEAF@"', "\tdw 211, 140", "\tdb A sweet aroma", ""].join("\n"));

    expect(() => parsePokedexEntry(entryPath)).toThrow("Malformed Pokédex text literal");
  });

  it("rejects entries with no text body instead of exporting empty text", () => {
    writeFile(entryPath, ['\tdb "LEAF@"', "\tdw 211, 140", ""].join("\n"));

    expect(() => parsePokedexEntry(entryPath)).toThrow("Could not parse Pokédex text body");
  });
});

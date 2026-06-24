import fs from "fs";
import os from "os";
import path from "path";
import { exportPokegearLandmarks } from "./export-pokegear-landmarks";

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

describe("exportPokegearLandmarks", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-landmarks-export-"));
    mockDisassemblyRoot = path.join(tempDir, "vendor");
    mockAssetsRoot = path.join(tempDir, "assets");

    writeFile(
      path.join(mockDisassemblyRoot, "constants", "landmark_constants.asm"),
      [
        "\tconst LANDMARK_NEW_BARK_TOWN",
        "\tDEF KANTO_LANDMARK EQU const_value",
        "\tconst LANDMARK_PALLET_TOWN",
        "\tDEF OTHER_LANDMARK EQU const_value",
        "\tDEF NUM_LANDMARKS EQU const_value",
        "",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "maps", "maps.asm"),
      [
        "\tmap NewBarkTown, TILESET_JOHTO, TOWN, LANDMARK_NEW_BARK_TOWN, MUSIC_NEW_BARK_TOWN, FALSE, PALETTE_AUTO, FISHGROUP_SHORE",
        "",
      ].join("\n")
    );
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("exports landmark names from explicit labels", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "maps", "landmarks.asm"),
      [
        'NewBarkTownName: db "NEW<BSP>BARK TOWN@";',
        'PalletTownName: db "PALLET TOWN@";',
        "\tlandmark 12, 24, NewBarkTownName",
        "\tlandmark 64, 80, PalletTownName",
        "",
      ].join("\n")
    );

    const result = exportPokegearLandmarks();

    expect(result.landmarks).toEqual([
      expect.objectContaining({
        id: 0,
        constant: "LANDMARK_NEW_BARK_TOWN",
        label: "NEW_BARK_TOWN",
        name: "NEW BARK TOWN",
        x: 20,
        y: 40,
        region: "JOHTO",
      }),
      expect.objectContaining({
        id: 1,
        constant: "LANDMARK_PALLET_TOWN",
        label: "PALLET_TOWN",
        name: "PALLET TOWN",
        x: 72,
        y: 96,
        region: "KANTO",
      }),
    ]);
    expect(result.map_to_landmark).toEqual({
      NewBarkTown: "LANDMARK_NEW_BARK_TOWN",
    });
  });

  it("rejects missing landmark name labels instead of inventing names from constants", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "maps", "landmarks.asm"),
      [
        'NewBarkTownName: db "NEW BARK@";',
        "\tlandmark 12, 24, NewBarkTownName",
        "\tlandmark 64, 80, PalletTownName",
        "",
      ].join("\n")
    );

    expect(() => exportPokegearLandmarks()).toThrow("Missing landmark name label 'PalletTownName' for LANDMARK_PALLET_TOWN");
  });

  it("throws when constants and landmark rows disagree", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "maps", "landmarks.asm"),
      ['NewBarkTownName: db "NEW BARK TOWN@";', "\tlandmark 12, 24, NewBarkTownName", ""].join("\n")
    );

    expect(() => exportPokegearLandmarks()).toThrow("Landmark table length mismatch");
  });
});

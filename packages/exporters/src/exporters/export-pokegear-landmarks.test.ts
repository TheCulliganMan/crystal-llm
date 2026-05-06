import fs from "fs";
import { exportPokegearLandmarks } from "./export-pokegear-landmarks";

const mockWriteJsonToTargets = jest.fn();
const mockParseMapDefinitions = jest.fn();

jest.mock("./asm-utils", () => {
  const actual = jest.requireActual("./asm-utils");
  return {
    ...actual,
    writeJsonToTargets: (...args: unknown[]) => mockWriteJsonToTargets(...args),
  };
});

jest.mock("./export-map-attributes", () => ({
  parseMapDefinitions: (...args: unknown[]) => mockParseMapDefinitions(...args),
}));

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDisassemblyRoot: () => "/mock/pokecrystal",
}));

const installLandmarkFiles = ({
  constants,
  landmarks,
}: {
  constants: string;
  landmarks: string;
}): void => {
  jest.spyOn(fs, "readFileSync").mockImplementation((filePath) => {
    const pathValue = String(filePath);
    if (pathValue.endsWith("constants/landmark_constants.asm")) {
      return constants;
    }
    if (pathValue.endsWith("data/maps/landmarks.asm")) {
      return landmarks;
    }
    throw new Error(`Unexpected read ${pathValue}`);
  });
};

describe("exportPokegearLandmarks", () => {
  beforeEach(() => {
    mockWriteJsonToTargets.mockReset();
    mockParseMapDefinitions.mockReset();
    jest.restoreAllMocks();
  });

  it("parses constants, rows, decoded names, regions, and map mappings", () => {
    installLandmarkFiles({
      constants: [
        "const LANDMARK_NEW_BARK_TOWN",
        "DEF KANTO_LANDMARK EQU const_value",
        "const LANDMARK_PALLET_TOWN",
        "DEF OTHER_LANDMARK EQU const_value",
        "const LANDMARK_SPECIAL",
        "DEF NUM_LANDMARKS EQU const_value",
      ].join("\n"),
      landmarks: [
        "NewBarkName: db \"NEW<BSP>BARK TOWN@\"",
        "PalletName: db \"PALLET TOWN@\"",
        "SpecialName: db \"#MON LEAGUE@\"",
        "\tlandmark 1, 2, NewBarkName",
        "\tlandmark 3, 4, PalletName",
        "\tlandmark 5, 6, SpecialName",
      ].join("\n"),
    });
    mockParseMapDefinitions.mockReturnValue({
      PlayersHouse2F: { location: "LANDMARK_NEW_BARK_TOWN" },
      PalletTown: { location: "LANDMARK_PALLET_TOWN" },
      DebugMap: {},
    });

    const payload = exportPokegearLandmarks();

    expect(payload.landmarks).toEqual([
      expect.objectContaining({
        id: 0,
        constant: "LANDMARK_NEW_BARK_TOWN",
        label: "NEW_BARK_TOWN",
        name: "New Bark Town",
        x: 9,
        y: 18,
        region: "JOHTO",
      }),
      expect.objectContaining({
        id: 1,
        constant: "LANDMARK_PALLET_TOWN",
        name: "Pallet Town",
        x: 11,
        y: 20,
        region: "KANTO",
      }),
      expect.objectContaining({
        id: 2,
        constant: "LANDMARK_SPECIAL",
        name: "Pokemon League",
        x: 13,
        y: 22,
        region: "JOHTO",
      }),
    ]);
    expect(payload.map_to_landmark).toEqual({
      PlayersHouse2F: "LANDMARK_NEW_BARK_TOWN",
      PalletTown: "LANDMARK_PALLET_TOWN",
    });
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith("pokegear_landmarks.json", payload);
  });

  it("throws when constants and landmark rows disagree", () => {
    installLandmarkFiles({
      constants: [
        "const LANDMARK_NEW_BARK_TOWN",
        "DEF KANTO_LANDMARK EQU const_value",
        "const LANDMARK_PALLET_TOWN",
        "DEF OTHER_LANDMARK EQU const_value",
        "DEF NUM_LANDMARKS EQU const_value",
      ].join("\n"),
      landmarks: [
        "NewBarkName: db \"NEW BARK TOWN@\"",
        "\tlandmark 1, 2, NewBarkName",
      ].join("\n"),
    });
    mockParseMapDefinitions.mockReturnValue({});

    expect(() => exportPokegearLandmarks()).toThrow("Landmark table length mismatch");
  });
});

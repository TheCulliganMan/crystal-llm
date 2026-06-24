import fs from "fs";
import { exportPokegearPaletteMap } from "./export-pokegear-palette-map";

const mockWriteJsonToTargets = jest.fn();

jest.mock("./asm-utils", () => {
  const actual = jest.requireActual("./asm-utils");
  return {
    ...actual,
    writeJsonToTargets: (...args: unknown[]) => mockWriteJsonToTargets(...args),
  };
});

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDisassemblyRoot: () => "/mock/pokecrystal",
}));

describe("exportPokegearPaletteMap", () => {
  beforeEach(() => {
    mockWriteJsonToTargets.mockReset();
    jest.restoreAllMocks();
  });

  it("parses town_map and pokegear palette sections in order", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "; gfx/pokegear/town_map.png",
      "\ttownmappals BORDER, EARTH, MOUNTAIN",
      "\ttownmappals CITY, POI, POI_MTN",
      "; gfx/pokegear/pokegear.png",
      "\ttownmappals POI_MTN, POI, CITY",
    ].join("\n"));

    const payload = exportPokegearPaletteMap();

    expect(payload).toEqual({
      town_map: ["BORDER", "EARTH", "MOUNTAIN", "CITY", "POI", "POI_MTN"],
      pokegear: ["POI_MTN", "POI", "CITY"],
    });
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "pokegear_town_map_palette_map.json",
      payload
    );
  });

  it("rejects unknown palette tokens", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "; gfx/pokegear/town_map.png",
      "\ttownmappals BORDER, LAVA",
      "; gfx/pokegear/pokegear.png",
      "\ttownmappals POI",
    ].join("\n"));

    expect(() => exportPokegearPaletteMap()).toThrow(
      "Unknown Pokégear town map palette token 'LAVA'"
    );
  });

  it("rejects case-changed palette tokens instead of normalizing them", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "; gfx/pokegear/town_map.png",
      "\ttownmappals border",
      "; gfx/pokegear/pokegear.png",
      "\ttownmappals POI",
    ].join("\n"));

    expect(() => exportPokegearPaletteMap()).toThrow(
      "Unknown Pokégear town map palette token 'border'"
    );
  });

  it("throws when a required section is missing", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "; gfx/pokegear/town_map.png",
      "\ttownmappals BORDER",
    ].join("\n"));

    expect(() => exportPokegearPaletteMap()).toThrow(
      "Could not parse Pokégear town map palette map"
    );
  });
});

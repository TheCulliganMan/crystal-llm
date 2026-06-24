import fs from "fs";
import os from "os";
import path from "path";
import { exportMapAttributes, parseMapConstants, parseMapDefinitions, parseMapPhoneFlag } from "./export-map-attributes";

let mockDisassemblyRoot = "/mock/pokecrystal";
let mockAssetsRoot = "/mock/assets";

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDisassemblyRoot: () => mockDisassemblyRoot,
  getAssetsRoot: () => mockAssetsRoot,
}));

const writeFile = (filePath: string, content: string): void => {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content);
};

describe("parseMapPhoneFlag", () => {
  it("requires exact ASM phone flag tokens without case coercion", () => {
    expect(parseMapPhoneFlag("TRUE")).toBe(1);
    expect(parseMapPhoneFlag("FALSE")).toBe(0);
    expect(parseMapPhoneFlag("1")).toBe(1);

    expect(() => parseMapPhoneFlag("true")).toThrow("Unknown map phone flag token 'true'");
    expect(() => parseMapPhoneFlag("False")).toThrow("Unknown map phone flag token 'False'");
  });
});

describe("parseMapDefinitions", () => {
  afterEach(() => {
    jest.restoreAllMocks();
    mockDisassemblyRoot = "/mock/pokecrystal";
    mockAssetsRoot = "/mock/assets";
  });

  it("rejects case-changed phone flags from map definitions", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue(
      [
        "\tmap Route29, TILESET_JOHTO, ROUTE, LANDMARK_ROUTE_29, MUSIC_ROUTE_29, true, PALETTE_AUTO, FISHGROUP_SHORE",
      ].join("\n") as never
    );

    expect(() => parseMapDefinitions()).toThrow("Unknown map phone flag token 'true'");
  });

  it("rejects malformed map definitions instead of skipping missing fields", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue(
      ["\tmap Route29, TILESET_JOHTO, ROUTE, LANDMARK_ROUTE_29, MUSIC_ROUTE_29, FALSE, PALETTE_AUTO"].join("\n") as never
    );

    expect(() => parseMapDefinitions()).toThrow("Malformed map definition");
  });
});

describe("parseMapConstants", () => {
  afterEach(() => {
    jest.restoreAllMocks();
    mockDisassemblyRoot = "/mock/pokecrystal";
    mockAssetsRoot = "/mock/assets";
  });

  it("requires explicit width and height instead of defaulting missing dimensions to zero", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue(
      ["newgroup GROUP_NEW_BARK", "\tmap_const NEW_BARK_TOWN, 10", ""].join("\n") as never
    );

    expect(() => parseMapConstants()).toThrow("Missing height for map_const 'NEW_BARK_TOWN'");
  });

  it("rejects malformed dimensions from map constants", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue(
      ["newgroup GROUP_NEW_BARK", "\tmap_const NEW_BARK_TOWN, ten, 9", ""].join("\n") as never
    );

    expect(() => parseMapConstants()).toThrow("Invalid width 'ten' for map_const 'NEW_BARK_TOWN'");
  });

  it("requires each map constant to belong to an explicit group", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue(["\tmap_const NEW_BARK_TOWN, 10, 9", ""].join("\n") as never);

    expect(() => parseMapConstants()).toThrow("map_const 'NEW_BARK_TOWN' appears before a newgroup declaration");
  });
});

describe("exportMapAttributes", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-map-attributes-export-"));
    mockDisassemblyRoot = path.join(tempDir, "vendor");
    mockAssetsRoot = path.join(tempDir, "assets");
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "map_constants.asm"),
      ["newgroup GROUP_NEW_BARK", "\tmap_const NEW_BARK_TOWN, 10, 9", "newgroup GROUP_ROUTE", "\tmap_const ROUTE_29, 20, 18", ""].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "maps", "maps.asm"),
      [
        "\tmap NewBarkTown, TILESET_JOHTO, TOWN, LANDMARK_NEW_BARK_TOWN, MUSIC_NEW_BARK_TOWN, FALSE, PALETTE_AUTO, FISHGROUP_SHORE",
        "\tmap Route29, TILESET_JOHTO, ROUTE, LANDMARK_ROUTE_29, MUSIC_ROUTE_29, TRUE, PALETTE_DAY, FISHGROUP_SHORE",
        "",
      ].join("\n")
    );
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
    mockDisassemblyRoot = "/mock/pokecrystal";
    mockAssetsRoot = "/mock/assets";
  });

  it("exports explicit null time-of-day data for PALETTE_AUTO without treating it as missing", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "maps", "attributes.asm"),
      ["\tmap_attributes NewBarkTown, NEW_BARK_TOWN, $05, 0", ""].join("\n")
    );

    const attributes = exportMapAttributes();

    expect(attributes.NewBarkTown).toMatchObject({
      time_of_day: null,
      phone_service: 0,
      environment: "TOWN",
      fishing_group: "FISHGROUP_SHORE",
    });
  });

  it("rejects map attribute rows that omit explicit connection flags", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "maps", "attributes.asm"),
      ["\tmap_attributes NewBarkTown, NEW_BARK_TOWN, $05", ""].join("\n")
    );

    expect(() => exportMapAttributes()).toThrow("Malformed map_attributes row");
  });

  it("rejects truncated connection tables instead of exporting partial connections", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "data", "maps", "attributes.asm"),
      ["\tmap_attributes Route29, ROUTE_29, $05, NORTH|SOUTH", "\tconnection north, NewBarkTown, NEW_BARK_TOWN, 0", ""].join("\n")
    );

    expect(() => exportMapAttributes()).toThrow("Expected 2 connection rows after map_attributes, found 1");
  });
});

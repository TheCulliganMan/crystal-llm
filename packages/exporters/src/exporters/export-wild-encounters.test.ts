import fs from "fs";
import os from "os";
import path from "path";
import { exportWildEncounters, mergeWildEncounterData, parseWildEncounters } from "./export-wild-encounters";

var mockDisassemblyRoot = "";

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDisassemblyRoot: () => mockDisassemblyRoot,
}));

jest.mock("@pokecrystal/core/engine/world/maps", () => ({
  mapConstantToName: (constant: string) =>
    constant
      .toLowerCase()
      .replace(/_([a-z0-9])/g, (_: string, char: string) => char.toUpperCase())
      .replace(/^[a-z]/, (char: string) => char.toUpperCase()),
}));

describe("export-wild-encounters", () => {
  it("exports exact runtime map names instead of ASM constants", () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "wild-encounters-"));
    const filePath = path.join(dir, "johto_grass.asm");
    fs.writeFileSync(
      filePath,
      [
        "def_grass_wildmons ROUTE_29",
        "\tdb 10 percent, 10 percent, 10 percent",
        "; morn",
        "\tdb 2, PIDGEY",
        "; day",
        "\tdb 2, PIDGEY",
        "; nite",
        "\tdb 2, RATTATA",
        "end_grass_wildmons",
      ].join("\n"),
      "utf8"
    );

    const [entry] = parseWildEncounters(filePath);

    expect(entry.map_name).toBe("Route29");
    expect(entry.grass?.morning[0]).toEqual({ level: 2, species: "PIDGEY" });
    expect(entry.water).toBeNull();
    expect(entry.water_rate).toBeNull();
  });

  it("merges grass and water data but rejects duplicate surfaces for a map", () => {
    const grass = {
      map_name: "Route29",
      grass_rates: { morning: 10, day: 10, night: 10 },
      water_rate: null,
      grass: { morning: [{ level: 2, species: "PIDGEY" }], day: [], night: [] },
      water: null,
    };
    const water = {
      map_name: "Route29",
      grass_rates: null,
      water_rate: 5,
      grass: null,
      water: { morning: [{ level: 10, species: "MAGIKARP" }], day: [{ level: 10, species: "MAGIKARP" }], night: [{ level: 10, species: "MAGIKARP" }] },
    };

    expect(mergeWildEncounterData([[grass], [water]])).toEqual([
      expect.objectContaining({
        map_name: "Route29",
        grass_rates: { morning: 10, day: 10, night: 10 },
        water_rate: 5,
        grass: grass.grass,
        water: water.water,
      }),
    ]);
    expect(() => mergeWildEncounterData([[grass], [grass]])).toThrow(
      "Duplicate grass wild encounter data for Route29."
    );
    expect(() => mergeWildEncounterData([[water], [water]])).toThrow(
      "Duplicate water wild encounter data for Route29."
    );
  });

  it("requires all canonical wild encounter source files", () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "wild-encounters-"));
    mockDisassemblyRoot = dir;
    fs.mkdirSync(path.join(dir, "data", "wild"), { recursive: true });
    try {
      expect(() => exportWildEncounters()).toThrow(
        `Missing required wild encounter source ${path.join(dir, "data", "wild", "johto_grass.asm")}.`
      );
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
      mockDisassemblyRoot = "";
    }
  });
});

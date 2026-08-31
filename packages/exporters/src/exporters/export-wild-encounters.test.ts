import fs from "fs";
import os from "os";
import path from "path";
import {
  exportWildEncounters,
  mergeWildEncounterData,
  parseWildEncounters,
  parseWildEncounterSwarmDeclarations,
  parseWildEncounterSwarms,
} from "./export-wild-encounters";

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

  it("binds swarm grass tables to their exact script token and engine flag", () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "wild-swarms-"));
    const phonePath = path.join(dir, "arnie.asm");
    const swarmPath = path.join(dir, "swarm_grass.asm");
    fs.writeFileSync(
      phonePath,
      [
        "setflag ENGINE_YANMA_SWARM",
        "getmonname STRING_BUFFER_4, YANMA",
        "swarm SWARM_YANMA, ROUTE_35",
      ].join("\n"),
      "utf8",
    );
    const rows = (species: string) => ["; morn", "; day", "; nite"]
      .flatMap((time) => [time, ...Array.from({ length: 7 }, () => `db 12, ${species}`)]);
    fs.writeFileSync(
      swarmPath,
      ["map_id ROUTE_35", "db 10 percent, 10 percent, 10 percent", ...rows("YANMA"), "db -1"].join("\n"),
      "utf8",
    );

    const declarations = parseWildEncounterSwarmDeclarations([phonePath]);
    const swarms = parseWildEncounterSwarms(swarmPath, declarations);

    expect(swarms).toEqual([
      expect.objectContaining({
        map_name: "Route35",
        swarm_token: "SWARM_YANMA",
        override: expect.objectContaining({
          engine_flag: "ENGINE_YANMA_SWARM",
          grass_rates: { morning: 10, day: 10, night: 10 },
        }),
      }),
    ]);
    expect(swarms[0].override.grass.night).toHaveLength(7);
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

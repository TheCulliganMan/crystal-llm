import fs from "fs";
import os from "os";
import path from "path";
import { exportFishing } from "./export-fishing";

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

describe("exportFishing", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-fishing-export-"));
    mockDisassemblyRoot = path.join(tempDir, "vendor");
    mockAssetsRoot = path.join(tempDir, "assets");
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("exports exact fish groups, rod tables, aliases, and time groups from ASM", () => {
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "map_data_constants.asm"),
      [
        "const_def",
        "\tconst FISHGROUP_NONE",
        "\tconst FISHGROUP_SHORE",
        "\tconst FISHGROUP_POND",
        "DEF OTHER_CONSTANT EQU 1",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "data", "wild", "fish.asm"),
      [
        "FishGroups:",
        "\tfishgroup 50 percent + 1, .Shore_Old, .Shore_Good, .Shore_Super",
        "\tfishgroup 25 percent, .Pond_Old, .Pond_Good, .Pond_Super",
        ".Shore_Old:",
        "\tdb 70 percent + 1, MAGIKARP, 10",
        ".Shore_Good:",
        "\tdb 100 percent, time_group 0",
        ".Shore_Super:",
        "\tdb 100 percent, KRABBY, 40",
        ".Pond_Old:",
        ".Pond_Good:",
        "\tdb 100 percent, POLIWAG, 20",
        ".Pond_Super:",
        "\tdb 100 percent, POLIWHIRL, 40",
        "TimeFishGroups:",
        "\tdb CORSOLA, 20, STARYU, 20 ; 0",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "events", "fish.asm"),
      [
        "GetFishGroupIndex:",
        "\tcp FISHGROUP_SHORE",
        "\tjr z, .shore",
        "\tcp FISHGROUP_POND",
        "\tjr z, .pond",
        ".done",
        "\tret",
        ".shore",
        "\tcp FISHSWARM_QWILFISH",
        "\tjr nz, .done",
        "\tld d, FISHGROUP_POND",
        "\tjr .done",
        ".pond",
        "\tcp FISHSWARM_REMORAID",
        "\tjr nz, .done",
        "\tld d, FISHGROUP_SHORE",
        "\tjr .done",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "script_constants.asm"),
      [
        "; ActivateFishingSwarm setval arguments",
        "const_def",
        "\tconst FISHSWARM_NONE",
        "\tconst FISHSWARM_QWILFISH",
        "\tconst FISHSWARM_REMORAID",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "constants", "ram_constants.asm"),
      [
        "; wDailyFlags1::",
        "\tconst_def",
        "\tconst DAILYFLAGS1_KURT_MAKING_BALLS_F",
        "\tconst DAILYFLAGS1_BUG_CONTEST_F",
        "\tconst DAILYFLAGS1_FISH_SWARM_F",
        "\tconst DAILYFLAGS1_TIME_CAPSULE_F",
      ].join("\n")
    );
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "items", "item_effects.asm"),
      [
        "ItemEffects:",
        "\tdw OldRodEffect        ; OLD_ROD",
        "\tdw GoodRodEffect       ; GOOD_ROD",
        "\tdw SuperRodEffect      ; SUPER_ROD",
        "OldRodEffect:",
        "\tld e, $0",
        "\tjr UseRod",
        "GoodRodEffect:",
        "\tld e, $1",
        "\tjr UseRod",
        "SuperRodEffect:",
        "\tld e, $2",
        "\tjr UseRod",
        "UseRod:",
        "\tfarcall FishFunction",
        "\tret",
      ].join("\n")
    );

    const catalog = exportFishing();

    expect(catalog.groups.FISHGROUP_SHORE.bite_threshold).toBe(128);
    expect(catalog.groups.FISHGROUP_SHORE.rod_tables.OLD_ROD.slots[0]).toEqual({
      threshold: 179,
      species: "MAGIKARP",
      level: 10,
      time_group: null,
    });
    expect(catalog.groups.FISHGROUP_SHORE.rod_tables.GOOD_ROD.slots[0]).toEqual({
      threshold: 255,
      species: null,
      level: 0,
      time_group: 0,
    });
    expect(catalog.groups.FISHGROUP_POND.rod_tables.OLD_ROD.slots[0]).toEqual({
      threshold: 255,
      species: "POLIWAG",
      level: 20,
      time_group: null,
    });
    expect(catalog.groups.FISHGROUP_POND.rod_tables.GOOD_ROD.slots[0]).toEqual(
      catalog.groups.FISHGROUP_POND.rod_tables.OLD_ROD.slots[0]
    );
    expect(catalog.time_groups).toEqual([
      {
        day_species: "CORSOLA",
        day_level: 20,
        night_species: "STARYU",
        night_level: 20,
      },
    ]);
    expect(catalog.swarm_rules).toEqual([
      {
        daily_flag_bit: 2,
        swarm: 1,
        base_group: "FISHGROUP_SHORE",
        swarm_group: "FISHGROUP_POND",
      },
      {
        daily_flag_bit: 2,
        swarm: 2,
        base_group: "FISHGROUP_POND",
        swarm_group: "FISHGROUP_SHORE",
      },
    ]);
    expect(catalog.rod_items).toEqual([
      { item_id: "OLD_ROD", rod: "OLD_ROD" },
      { item_id: "GOOD_ROD", rod: "GOOD_ROD" },
      { item_id: "SUPER_ROD", rod: "SUPER_ROD" },
    ]);
  });
});

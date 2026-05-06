import fs from "fs";
import { parseEvolutions } from "./export-evolutions";

describe("parseEvolutions", () => {
  it("normalizes special species labels to canonical Pokemon ids", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "NidoranFEvosAttacks:",
      "\tdb EVOLVE_LEVEL, 16, NIDORINA",
      "\tdb 0 ; no more evolutions",
      "\tdb 1, GROWL",
      "\tdb 0 ; no more level-up moves",
      "",
      "NidoranMEvosAttacks:",
      "\tdb EVOLVE_LEVEL, 16, NIDORINO",
      "\tdb 0 ; no more evolutions",
      "\tdb 1, LEER",
      "\tdb 0 ; no more level-up moves",
    ].join("\n") as never);

    expect(parseEvolutions("/mock/evos_attacks.asm")).toEqual([
      { species: "NIDORAN_F", evolutions: [{ method: "LEVEL", level: 16, species: "NIDORINA" }] },
      { species: "NIDORAN_M", evolutions: [{ method: "LEVEL", level: 16, species: "NIDORINO" }] },
    ]);
  });
});

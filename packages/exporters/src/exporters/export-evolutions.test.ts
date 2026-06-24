import fs from "fs";
import { buildSpeciesLabelMap, parseEvolutions } from "./export-evolutions";

describe("parseEvolutions", () => {
  const speciesIds = ["NIDORAN_F", "NIDORINA", "NIDORAN_M", "NIDORINO"];

  it("resolves exact ASM labels to canonical Pokemon ids from constants", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "NidoranFEvosAttacks:",
      "\tdb EVOLVE_LEVEL, 16, NIDORINA",
      "\tdb 0 ; no more evolutions",
      "\tdb 1, GROWL",
      "\tdb 0 ; no more level-up moves",
      "",
      "NidorinaEvosAttacks:",
      "\tdb 0 ; no more evolutions",
      "\tdb 1, GROWL",
      "\tdb 0 ; no more level-up moves",
      "",
      "NidoranMEvosAttacks:",
      "\tdb EVOLVE_LEVEL, 16, NIDORINO",
      "\tdb 0 ; no more evolutions",
      "\tdb 1, LEER",
      "\tdb 0 ; no more level-up moves",
      "",
      "NidorinoEvosAttacks:",
      "\tdb 0 ; no more evolutions",
      "\tdb 1, LEER",
      "\tdb 0 ; no more level-up moves",
    ].join("\n") as never);

    expect(parseEvolutions("/mock/evos_attacks.asm", speciesIds)).toEqual([
      {
        species: "NIDORAN_F",
        evolutions: [
          {
            method: "LEVEL",
            level: 16,
            item: null,
            held_item: null,
            happiness: null,
            stat_ratio: null,
            species: "NIDORINA",
          },
        ],
      },
      {
        species: "NIDORINA",
        evolutions: [],
      },
      {
        species: "NIDORAN_M",
        evolutions: [
          {
            method: "LEVEL",
            level: 16,
            item: null,
            held_item: null,
            happiness: null,
            stat_ratio: null,
            species: "NIDORINO",
          },
        ],
      },
      {
        species: "NIDORINO",
        evolutions: [],
      },
    ]);
  });

  it("rejects case-changed species labels instead of normalizing them", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "nidoranFEvosAttacks:",
      "\tdb EVOLVE_LEVEL, 16, NIDORINA",
      "\tdb 0 ; no more evolutions",
      "\tdb 1, GROWL",
      "\tdb 0 ; no more level-up moves",
    ].join("\n") as never);

    expect(() => parseEvolutions("/mock/evos_attacks.asm", speciesIds)).toThrow(
      "Unknown or case-changed evolution species label 'nidoranF'"
    );
  });

  it("rejects unknown evolution target species", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "NidoranFEvosAttacks:",
      "\tdb EVOLVE_LEVEL, 16, MissingNo",
      "\tdb 0 ; no more evolutions",
      "\tdb 1, GROWL",
      "\tdb 0 ; no more level-up moves",
    ].join("\n") as never);

    expect(() => parseEvolutions("/mock/evos_attacks.asm", speciesIds)).toThrow(
      "Unknown evolution target species 'MissingNo'"
    );
  });

  it("requires an explicit evolution block for every species", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "NidoranFEvosAttacks:",
      "\tdb 0 ; no more evolutions",
      "\tdb 1, GROWL",
      "\tdb 0 ; no more level-up moves",
    ].join("\n") as never);

    expect(() => parseEvolutions("/mock/evos_attacks.asm", speciesIds)).toThrow(
      "Missing evolution block for species 'NIDORINA'"
    );
  });
});

describe("buildSpeciesLabelMap", () => {
  it("rejects duplicate exact labels derived from species constants", () => {
    expect(() => buildSpeciesLabelMap(["MR__MIME", "MR_MIME"])).toThrow(
      "Duplicate ASM evolution label 'MrMime' from species constants."
    );
  });
});

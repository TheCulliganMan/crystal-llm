import fs from "fs";
import { buildLevelUpMovesData, parseEggMoves, parseLearnsets } from "./export-data";

describe("parseLearnsets", () => {
  it("parses level-up moves from evos_attacks.asm after evolution records", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "SECTION \"Evolutions and Attacks\", ROMX",
      "",
      "CyndaquilEvosAttacks:",
      "\tdb EVOLVE_LEVEL, 14, QUILAVA",
      "\tdb 0 ; no more evolutions",
      "\tdb 1, TACKLE",
      "\tdb 1, LEER",
      "\tdb 6, SMOKESCREEN",
      "\tdb 0 ; no more level-up moves",
      "",
      "MrMimeEvosAttacks:",
      "\tdb 0 ; no more evolutions",
      "\tdb 1, BARRIER",
      "\tdb 6, CONFUSION",
      "\tdb 0 ; no more level-up moves",
    ].join("\n") as never);

    const learnsets = parseLearnsets("/mock/evos_attacks.asm", {
      CYNDAQUIL: 155,
      MR__MIME: 122,
    });

    expect(learnsets).toEqual({
      MR__MIME: [
        [1, "BARRIER"],
        [6, "CONFUSION"],
      ],
      CYNDAQUIL: [
        [1, "TACKLE"],
        [1, "LEER"],
        [6, "SMOKESCREEN"],
      ],
    });
  });
});

describe("buildLevelUpMovesData", () => {
  it("converts compact learnsets into the breeding loader shape", () => {
    expect(buildLevelUpMovesData({ CYNDAQUIL: [[1, "TACKLE"], [6, "SMOKESCREEN"]] })).toEqual({
      CYNDAQUIL: [
        { level: 1, move: "TACKLE" },
        { level: 6, move: "SMOKESCREEN" },
      ],
    });
  });
});

describe("parseEggMoves", () => {
  it("parses egg move labels and ignores the shared NoEggMoves sentinel", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      'SECTION "Egg Moves", ROMX',
      'INCLUDE "data/pokemon/egg_move_pointers.asm"',
      "",
      "NidoranFEggMoves:",
      "\tdb SUPERSONIC",
      "\tdb DISABLE",
      "\tdb -1 ; end",
      "",
      "MrMimeEggMoves:",
      "\tdb HYPNOSIS",
      "\tdb MIMIC",
      "\tdb -1 ; end",
      "",
      "NoEggMoves:",
      "\tdb -1 ; end",
    ].join("\n") as never);

    const eggMoves = parseEggMoves("/mock/egg_moves.asm", {
      NIDORAN_F: 29,
      MR__MIME: 122,
    });

    expect(eggMoves).toEqual({
      NIDORAN_F: ["SUPERSONIC", "DISABLE"],
      MR__MIME: ["HYPNOSIS", "MIMIC"],
    });
  });
});

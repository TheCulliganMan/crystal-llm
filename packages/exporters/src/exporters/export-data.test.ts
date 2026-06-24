import fs from "fs";
import { buildLevelUpMovesData, loadAllPokemonData, parseBaseStats, parseEggMoves, parseLearnsets, parseMoves } from "./export-data";

describe("parseMoves", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("exports move effects as definitive modpack strings without enum fallback", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "\tmove TACKLE, EFFECT_NORMAL_HIT, 35, NORMAL, 95 percent, 35, 0 percent",
      "\tmove MODDED_STRIKE, EFFECT_MODDED_WEATHER, 40, WATER, 100 percent, 10, 20 percent",
    ].join("\n") as never);

    const moves = parseMoves("/mock/moves.asm");

    expect(moves.TACKLE.effect).toBe("NORMAL_HIT");
    expect(moves.MODDED_STRIKE.effect).toBe("MODDED_WEATHER");
  });

  it("expands known ASM stat-effect tokens exactly and preserves non-stat effect ids", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "\tmove CRUNCH, EFFECT_SP_DEF_DOWN_HIT, 80, DARK, 100 percent, 15, 20 percent",
      "\tmove PSYCH_UP, EFFECT_PSYCH_UP, 0, NORMAL, 100 percent, 10, 0 percent",
    ].join("\n") as never);

    const moves = parseMoves("/mock/moves.asm");
    expect(moves.CRUNCH).toEqual(
      expect.objectContaining({
        effect: "SPECIAL_DEFENSE_DOWN_HIT",
        stat: "SPECIAL_DEFENSE",
        amount: -1,
      })
    );
    expect(moves.PSYCH_UP).toEqual(
      expect.objectContaining({
        effect: "PSYCH_UP",
        stat: null,
        amount: null,
      })
    );
  });
});

describe("parseBaseStats", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  const baseStats = (type1 = "WATER"): string => [
    "\tdb TOTODILE ; species",
    "\tdb 50, 65, 64, 43, 44, 48",
    `\tdb ${type1}, WATER ; type`,
    "\tdb 45 ; catch rate",
    "\tdb 66 ; base exp",
    "\tdb NO_ITEM, NO_ITEM ; items",
    "\tdb GENDER_F50 ; gender ratio",
    "\tdb 20 ; step cycles to hatch",
    "\tdb GROWTH_MEDIUM_SLOW ; growth rate",
    "\tdn EGG_MONSTER, EGG_WATER_1",
    "\ttmhm SURF, CUT",
  ].join("\n");

  it("does not emit stale embedded evolution fields in species data", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue(baseStats() as never);

    const species = parseBaseStats("/mock/totodile.asm", { TOTODILE: 158 }, 210) as Record<
      string,
      unknown
    >;

    expect(species.id).toBe("TOTODILE");
    expect(species).not.toHaveProperty("evolutions");
  });

  it("throws on unknown enum tokens instead of defaulting species fields", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue(baseStats("GLITCH") as never);

    expect(() => parseBaseStats("/mock/totodile.asm", { TOTODILE: 158 }, 210)).toThrow(
      "Unknown Pokemon type GLITCH in /mock/totodile.asm"
    );
  });

  it("throws when a parsed species is missing its numeric id", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue(baseStats() as never);

    expect(() => parseBaseStats("/mock/totodile.asm", {}, 210)).toThrow(
      "Missing numeric species id for TOTODILE in /mock/totodile.asm"
    );
  });
});

describe("loadAllPokemonData", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("propagates base-stat parse failures instead of fabricating dummy species", () => {
    jest.spyOn(fs, "existsSync").mockReturnValue(true);
    jest.spyOn(fs, "readFileSync").mockImplementation((filePath: fs.PathOrFileDescriptor) => {
      const textPath = String(filePath);
      if (textPath.includes("dex_entries")) {
        return "dw 200, 210 ; height, weight" as never;
      }
      return "not a base stats file" as never;
    });

    expect(() => loadAllPokemonData("/mock/base_stats", { TOTODILE: 158 })).toThrow(
      "Could not find species ID in /mock/base_stats/totodile.asm"
    );
  });
});

describe("parseLearnsets", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

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

  it("rejects case-changed learnset labels instead of normalizing them", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "cyndaquilEvosAttacks:",
      "\tdb 0 ; no more evolutions",
      "\tdb 1, TACKLE",
      "\tdb 0 ; no more level-up moves",
    ].join("\n") as never);

    expect(() => parseLearnsets("/mock/evos_attacks.asm", { CYNDAQUIL: 155 })).toThrow(
      "Unknown or case-changed learnset species label 'cyndaquilEvosAttacks'."
    );
  });

  it("rejects malformed level-up move rows instead of skipping them", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "CyndaquilEvosAttacks:",
      "\tdb 0 ; no more evolutions",
      "\tdb TACKLE",
      "\tdb 0 ; no more level-up moves",
    ].join("\n") as never);

    expect(() => parseLearnsets("/mock/evos_attacks.asm", { CYNDAQUIL: 155 })).toThrow(
      "Malformed level-up move row in /mock/evos_attacks.asm: db TACKLE"
    );
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
  afterEach(() => {
    jest.restoreAllMocks();
  });

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

  it("rejects case-changed egg move labels instead of normalizing them", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      'SECTION "Egg Moves", ROMX',
      "nidoranFEggMoves:",
      "\tdb SUPERSONIC",
      "\tdb -1 ; end",
    ].join("\n") as never);

    expect(() => parseEggMoves("/mock/egg_moves.asm", { NIDORAN_F: 29 })).toThrow(
      "Unknown or case-changed egg-move species label 'nidoranFEggMoves'."
    );
  });
});

import fs from "fs";
import os from "os";
import path from "path";
import {
  parseNpcConstants,
  parseNpcData,
  parseNpcDataFromMapFile,
  parseNumericExpression,
} from "./export-npcs";

describe("export-npcs", () => {
  it("parses object constants and object_event rows into ObjectEvent payloads", () => {
    const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-export-npcs-"));
    const mapPath = path.join(tempDir, "TestMap.asm");
    try {
      fs.writeFileSync(
        mapPath,
        [
          "	object_const_def",
          "	const TESTMAP_YOUNGSTER",
          "",
          "TestMap_MapEvents:",
          "	def_object_events",
          "	object_event  1,  2, SPRITE_YOUNGSTER, SPRITEMOVEDATA_STANDING_DOWN, 0, 0, -1, DAY, PAL_NPC_BLUE, OBJECTTYPE_SCRIPT, 0, TestScript, -1",
          "",
        ].join("\n")
      );

      expect(
        parseNpcDataFromMapFile("TestMap", mapPath, {
          PAL_NPC_BLUE: 9,
        })
      ).toEqual([
        {
          x: 1,
          y: 2,
          sprite: "SPRITE_YOUNGSTER",
          spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN",
          move_range_x: 0,
          move_range_y: 0,
          hram_x: -1,
          hram_y: 2,
          pal: 9,
          object_type: "OBJECTTYPE_SCRIPT",
          radius: 0,
          script: "TestScript",
          event_flag: "-1",
          object_identifier: "TESTMAP_YOUNGSTER",
        },
      ]);
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it("parses bitwise time expressions", () => {
    expect(parseNumericExpression("MORN | DAY")).toBe(3);
  });

  it("rejects unknown symbolic numeric tokens instead of coercing them to zero", () => {
    expect(() => parseNumericExpression("PAL_NPC_BLUE | UNKNOWN_TOKEN", { PAL_NPC_BLUE: 9 })).toThrow(
      "Unknown numeric expression token 'UNKNOWN_TOKEN' in 'PAL_NPC_BLUE | UNKNOWN_TOKEN'"
    );
  });

  it("rejects malformed numeric tokens instead of exporting NaN-coerced values", () => {
    expect(() => parseNumericExpression("$ZZ")).toThrow("Unknown numeric expression token '$ZZ' in '$ZZ'");
  });

  it("parses NPC palette constants from const_def expressions", () => {
    const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-export-npcs-"));
    const constantsPath = path.join(tempDir, "sprite_data_constants.asm");
    try {
      fs.writeFileSync(
        constantsPath,
        [
          "	const_def 1 << 3",
          "	const PAL_NPC_RED",
          "	const PAL_NPC_BLUE",
          "	const PAL_NPC_GREEN",
          "",
        ].join("\n")
      );

      expect(parseNpcConstants(constantsPath)).toEqual(
        expect.objectContaining({
          PAL_NPC_RED: 8,
          PAL_NPC_BLUE: 9,
          PAL_NPC_GREEN: 10,
        })
      );
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it("only exports NPC constants present in the ASM constants file", () => {
    const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-export-npcs-"));
    const constantsPath = path.join(tempDir, "sprite_data_constants.asm");
    try {
      fs.writeFileSync(constantsPath, ["	const_def 1 << 3", "	const PAL_NPC_RED", ""].join("\n"));

      expect(parseNpcConstants(constantsPath)).toEqual({
        PAL_NPC_RED: 8,
      });
      expect(parseNpcConstants(constantsPath)).not.toHaveProperty("PAL_NPC_BLUE");
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it("rejects case-changed NPC constants instead of normalizing labels", () => {
    expect(() => parseNumericExpression("pal_npc_blue", { PAL_NPC_BLUE: 9 })).toThrow(
      "Unknown numeric expression token 'pal_npc_blue' in 'pal_npc_blue'"
    );
  });

  it("requires the NPC constants file instead of using built-in fallback constants", () => {
    const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-export-npcs-"));
    try {
      expect(() => parseNpcConstants(path.join(tempDir, "missing.asm"))).toThrow(
        `Missing NPC constants file ${path.join(tempDir, "missing.asm")}.`
      );
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it("throws when object constants and object_event rows diverge", () => {
    const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-export-npcs-"));
    const mapPath = path.join(tempDir, "BrokenMap.asm");
    try {
      fs.writeFileSync(
        mapPath,
        [
          "	object_const_def",
          "	const BROKEN_MAP_NPC",
          "	const BROKEN_MAP_EXTRA",
          "",
          "BrokenMap_MapEvents:",
          "	def_object_events",
          "	object_event  1,  2, SPRITE_YOUNGSTER, SPRITEMOVEDATA_STANDING_DOWN, 0, 0, -1, -1, PAL_NPC_BLUE, OBJECTTYPE_SCRIPT, 0, BrokenScript, -1",
          "",
        ].join("\n")
      );

      expect(() =>
        parseNpcDataFromMapFile("BrokenMap", mapPath, {
          PAL_NPC_BLUE: 9,
        })
      ).toThrow("Object constant count does not match object_event count for BrokenMap: 2 != 1");
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it("exports explicit empty NPC arrays for maps without object_event rows", () => {
    const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-export-npcs-"));
    try {
      fs.writeFileSync(
        path.join(tempDir, "EmptyMap.asm"),
        ["	object_const_def", "", "EmptyMap_MapEvents:", "	def_object_events", ""].join("\n")
      );
      fs.writeFileSync(
        path.join(tempDir, "Route1.asm"),
        [
          "	object_const_def",
          "	const ROUTE1_NPC",
          "",
          "Route1_MapEvents:",
          "	def_object_events",
          "	object_event  3,  4, SPRITE_COOLTRAINER_M, SPRITEMOVEDATA_STANDING_DOWN, 0, 0, -1, -1, PAL_NPC_RED, OBJECTTYPE_SCRIPT, 0, Route1Script, -1",
          "",
        ].join("\n")
      );

      expect(parseNpcData(tempDir, { PAL_NPC_RED: 8 })).toEqual({
        EmptyMap: [],
        Route1: [
          expect.objectContaining({
            x: 3,
            y: 4,
            object_identifier: "ROUTE1_NPC",
          }),
        ],
      });
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });
});

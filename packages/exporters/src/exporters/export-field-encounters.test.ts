import { parseTreeSleepRules } from "./export-field-encounters";

describe("parseTreeSleepRules", () => {
  it("parses the canonical time lists and sleep counter from Pret ASM", () => {
    const rules = parseTreeSleepRules(
      [
        "AsleepTreeMonsNite:",
        "\tdb CATERPIE",
        "\tdb AIPOM",
        "\tdb -1 ; end",
        "AsleepTreeMonsDay:",
        "\tdb HOOTHOOT",
        "\tdb -1 ; end",
        "AsleepTreeMonsMorn:",
        "\tdb HOOTHOOT",
        "\tdb -1 ; end",
      ].join("\n"),
      "DEF TREEMON_SLEEP_TURNS EQU 7"
    );

    expect(rules.sleepTurns).toBe(7);
    expect([...rules.speciesByTime.morning]).toEqual(["HOOTHOOT"]);
    expect([...rules.speciesByTime.day]).toEqual(["HOOTHOOT"]);
    expect([...rules.speciesByTime.night]).toEqual(["CATERPIE", "AIPOM"]);
  });

  it("rejects an unterminated time table instead of silently exporting partial rules", () => {
    expect(() =>
      parseTreeSleepRules(
        [
          "AsleepTreeMonsNite:",
          "\tdb CATERPIE",
          "\tdb -1",
          "AsleepTreeMonsDay:",
          "\tdb HOOTHOOT",
          "\tdb -1",
          "AsleepTreeMonsMorn:",
          "\tdb HOOTHOOT",
        ].join("\n"),
        "DEF TREEMON_SLEEP_TURNS EQU 7"
      )
    ).toThrow("Missing terminated morning sleeping tree-mon table.");
  });
});

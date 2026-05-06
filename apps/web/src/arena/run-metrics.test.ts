import { extractCommandCount, extractStepCount, extractTeamSummary } from "./run-metrics";

describe("run metrics extraction", () => {
  it("extracts steps and command counts across common key variants", () => {
    expect(extractStepCount({ steps_taken: 412 })).toBe(412);
    expect(extractStepCount({ movement: { steps: "39" } })).toBe(39);
    expect(extractCommandCount({ command_count: 88 })).toBe(88);
    expect(extractCommandCount({ stats: { commands: "14" } })).toBe(14);
  });

  it("extracts team summary from party pokemon array", () => {
    expect(
      extractTeamSummary({
        party: {
          pokemon: [
            { species: "CYNDAQUIL", level: 5 },
            { species: "PIDGEY", level: 3 },
            { species: "GEODUDE", level: 6 },
            { species: "ONIX", level: 8 },
          ],
        },
      })
    ).toBe("Cyndaquil Lv5, Pidgey Lv3, Geodude Lv6 +1");
  });

  it("falls back to party_summary when full party data is missing", () => {
    expect(extractTeamSummary({ party_summary: { count: 3, lead_species: "TOTODILE" } })).toBe(
      "Totodile +2"
    );
    expect(extractTeamSummary({ party_summary: { count: 1 } })).toBe("1 Pokemon");
  });

  it("returns null when metrics are missing or malformed", () => {
    expect(extractStepCount(null)).toBeNull();
    expect(extractCommandCount([])).toBeNull();
    expect(extractTeamSummary({ unknown: "value" })).toBeNull();
  });
});

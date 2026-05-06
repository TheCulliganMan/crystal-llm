import { parsePokedexEntryFile } from "./pokedex-entry-loader";

describe("pokedex-entry-loader", () => {
  it("loads Bulbasaur from bundled pokedex data", () => {
    expect(parsePokedexEntryFile("BULBASAUR")).toEqual({
      classification: "SEED",
      heightDigits: 204,
      weightDigits: 150,
      pages: [
        "While it is young, @ it uses the @ nutrients that are",
        "stored in the @ seeds on its back @ in order to grow.",
      ],
    });
  });

  it("normalizes ASM species ids with duplicate separators", () => {
    expect(parsePokedexEntryFile("MR__MIME").classification).toBe("BARRIER");
  });

  it("throws for unknown species ids", () => {
    expect(() => parsePokedexEntryFile("NOT_A_MON")).toThrow(
      "Missing Pokédex entry definition for NOT_A_MON"
    );
  });
});

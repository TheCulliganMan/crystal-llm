import { __test__resolveSpeciesMaps } from "@pokecrystal/core/ui/menus/pokedex";
import type { PokemonSpecies } from "@pokecrystal/core/core/models";

describe("resolveSpeciesMaps", () => {
  const bulbasaur = { id: "BULBASAUR", int_id: 1 } as PokemonSpecies;
  const ivysaur = { id: "IVYSAUR", int_id: 2 } as PokemonSpecies;

  it("builds maps from a Record-based data loader", () => {
    const [byName, byNumber] = __test__resolveSpeciesMaps({
      pokemonData: {
        BULBASAUR: bulbasaur,
        IVYSAUR: ivysaur,
      },
    });

    expect(byName).toEqual({ BULBASAUR: bulbasaur, IVYSAUR: ivysaur });
    expect(byNumber).toEqual({ 1: bulbasaur, 2: ivysaur });
  });

  it("builds maps from a Map-based data loader", () => {
    const [byName, byNumber] = __test__resolveSpeciesMaps({
      pokemonData: new Map([
        ["BULBASAUR", bulbasaur],
        ["IVYSAUR", ivysaur],
      ]),
    });

    expect(byName).toEqual({ BULBASAUR: bulbasaur, IVYSAUR: ivysaur });
    expect(byNumber).toEqual({ 1: bulbasaur, 2: ivysaur });
  });

  it("builds maps from a pokemon_data record", () => {
    const [byName, byNumber] = __test__resolveSpeciesMaps({
      pokemon_data: {
        BULBASAUR: bulbasaur,
        IVYSAUR: ivysaur,
      },
    });

    expect(byName).toEqual({ BULBASAUR: bulbasaur, IVYSAUR: ivysaur });
    expect(byNumber).toEqual({ 1: bulbasaur, 2: ivysaur });
  });

  it("builds maps from a speciesMap record", () => {
    const [byName, byNumber] = __test__resolveSpeciesMaps({
      speciesMap: {
        BULBASAUR: bulbasaur,
        IVYSAUR: ivysaur,
      },
    });

    expect(byName).toEqual({ BULBASAUR: bulbasaur, IVYSAUR: ivysaur });
    expect(byNumber).toEqual({ 1: bulbasaur, 2: ivysaur });
  });

  it("falls back to bundled species data when no species collection is provided", () => {
    const [byName, byNumber] = __test__resolveSpeciesMaps(null);

    expect(byName.BULBASAUR?.int_id).toBe(1);
    expect(byNumber[1]?.id).toBe("BULBASAUR");
  });
});

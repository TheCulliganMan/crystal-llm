import { DexMode } from "@pokecrystal/core/core/enums/pokedex";
import { setPokedexFlag } from "@pokecrystal/core/core/pokedex";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { loadAllSpecies } from "@pokecrystal/core/core/data-loader";
import type { PokemonSpecies } from "@pokecrystal/core/core/models";
import { NEW_POKEDEX_ORDER } from "@pokecrystal/assets/content/data/pokedex-orders";
import {
  findAdjacentSeenDexEntryIndex,
  listingPageDown,
  orderEntriesForMode,
  type DexEntry,
} from "@pokecrystal/core/ui/menus/pokedex-state";

const buildSpeciesIndexes = (): [Record<string, PokemonSpecies>, Record<number, PokemonSpecies>] => {
  const all = loadAllSpecies();
  const byName: Record<string, PokemonSpecies> = {};
  const byNumber: Record<number, PokemonSpecies> = {};
  for (const [name, species] of all.entries()) {
    byName[name] = species;
    byNumber[species.int_id] = species;
  }
  return [byName, byNumber];
};

describe("orderEntriesForMode", () => {
  it("uses species ids in NEW mode order while tracking list-end by seen slot", () => {
    const [speciesByName, speciesByNumber] = buildSpeciesIndexes();
    const gameState = createInitialGameState();

    const chikorita = speciesByName.CHIKORITA;
    const totodile = speciesByName.TOTODILE;
    if (!chikorita || !totodile) {
      throw new Error("Starter species must exist in merged species data.");
    }

    setPokedexFlag(gameState, totodile.int_id, "seen");

    const [entries, listingEnd] = orderEntriesForMode(
      gameState,
      speciesByName,
      speciesByNumber,
      DexMode.NEW
    );

    expect(entries[0]?.species.id).toBe("CHIKORITA");
    expect(entries[0]?.pokedexNumber).toBe(chikorita.int_id);

    const totodileEntry = entries.find((entry) => entry.species.id === "TOTODILE");
    expect(totodileEntry?.pokedexNumber).toBe(totodile.int_id);
    expect(listingEnd).toBe(NEW_POKEDEX_ORDER.indexOf("TOTODILE") + 1);
  });
});

describe("Pokedex listing movement ASM parity", () => {
  it("reports a page-down change at the bottom when the list spans more than one page", () => {
    expect(listingPageDown(6, 244, 7, 251)).toEqual([6, 244, true]);
  });

  it("does not page down when the full list fits in the visible height", () => {
    expect(listingPageDown(0, 0, 7, 7)).toEqual([0, 0, false]);
  });

  it("finds adjacent seen entries and returns null when the ASM path would restore the cursor", () => {
    const [speciesByName] = buildSpeciesIndexes();
    const entries: DexEntry[] = ["CHIKORITA", "BAYLEEF", "MEGANIUM", "CYNDAQUIL"].map((name) => {
      const species = speciesByName[name];
      if (!species) {
        throw new Error(`Missing test species ${name}`);
      }
      return { pokedexNumber: species.int_id, species };
    });

    expect(findAdjacentSeenDexEntryIndex(entries, 0, 1, [speciesByName.MEGANIUM!.int_id])).toBe(2);
    expect(findAdjacentSeenDexEntryIndex(entries, 2, -1, [speciesByName.MEGANIUM!.int_id])).toBeNull();
  });
});

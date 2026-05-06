import { PokemonType } from "@pokecrystal/core/core/enums/pokemon";
import type { PokemonSpecies } from "@pokecrystal/core/core/models";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import type { DexEntry } from "@pokecrystal/core/ui/menus/pokedex-behaviors";
import { PokedexSearchController } from "@pokecrystal/core/ui/menus/pokedex-behaviors";

const buildSpecies = (
  id: string,
  intId: number,
  type1: PokemonType,
  type2: PokemonType = type1,
): PokemonSpecies =>
  ({
    id,
    int_id: intId,
    type1,
    type2,
  }) as unknown as PokemonSpecies;

describe("PokedexSearchController.filterEntries", () => {
  it("matches caught flags using species id (ASM species-flag behavior)", () => {
    const gameState = createInitialGameState();
    const controller = new PokedexSearchController(gameState);
    const entries: DexEntry[] = [
      {
        pokedexNumber: 7,
        species: buildSpecies("TOTODILE", 158, PokemonType.WATER),
      },
    ];

    const results = controller.filterEntries(entries, new Set([158]), PokemonType.WATER, PokemonType.NONE);

    expect(results).toHaveLength(1);
    expect(results[0]?.species.id).toBe("TOTODILE");
    expect(gameState.wram.wDexSearchResultCount).toBe(1);
  });

  it("falls back to pokedexNumber when species id is unavailable", () => {
    const gameState = createInitialGameState();
    const controller = new PokedexSearchController(gameState);
    const entries: DexEntry[] = [
      {
        pokedexNumber: 7,
        species: buildSpecies("TOTODILE", 0, PokemonType.WATER),
      },
    ];

    const results = controller.filterEntries(entries, new Set([7]), PokemonType.WATER, PokemonType.NONE);

    expect(results).toHaveLength(1);
    expect(results[0]?.species.id).toBe("TOTODILE");
    expect(gameState.wram.wDexSearchResultCount).toBe(1);
  });

  it("filters in ASM two-pass order using type2 first and then type1", () => {
    const gameState = createInitialGameState();
    const controller = new PokedexSearchController(gameState);
    const entries: DexEntry[] = [
      { pokedexNumber: 1, species: buildSpecies("CHARIZARD", 6, PokemonType.FIRE, PokemonType.FLYING) },
      { pokedexNumber: 2, species: buildSpecies("MOLTRES", 146, PokemonType.FIRE, PokemonType.FLYING) },
      { pokedexNumber: 3, species: buildSpecies("PIDGEY", 16, PokemonType.NORMAL, PokemonType.FLYING) },
      { pokedexNumber: 4, species: buildSpecies("VULPIX", 37, PokemonType.FIRE) },
    ];

    const results = controller.filterEntries(
      entries,
      new Set([6, 146, 16, 37]),
      PokemonType.FIRE,
      PokemonType.FLYING,
    );

    expect(results.map((entry) => entry.species.id)).toEqual(["CHARIZARD", "MOLTRES"]);
    expect(gameState.wram.wDexSearchResultCount).toBe(2);
  });

  it("returns no results when both ASM search selectors are NONE", () => {
    const gameState = createInitialGameState();
    const controller = new PokedexSearchController(gameState);
    const entries: DexEntry[] = [
      { pokedexNumber: 1, species: buildSpecies("PIDGEY", 16, PokemonType.NORMAL, PokemonType.FLYING) },
    ];

    const results = controller.filterEntries(entries, new Set([16]), PokemonType.NONE, PokemonType.NONE);

    expect(results).toEqual([]);
    expect(gameState.wram.wDexSearchResultCount).toBe(0);
  });

  it("stores search type indexes in WRAM after conversion validation", () => {
    const gameState = createInitialGameState();
    const controller = new PokedexSearchController(gameState);

    expect(controller.configureTypeIndexes([1, 0])).toEqual([PokemonType.NORMAL, PokemonType.NONE]);
    expect(gameState.wram.wDexSearchMonType1).toBe(1);
    expect(gameState.wram.wDexSearchMonType2).toBe(0);
  });
});

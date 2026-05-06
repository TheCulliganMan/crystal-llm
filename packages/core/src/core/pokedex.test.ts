import { createInitialGameState } from "@pokecrystal/core/core/state";
import { countPokedexEntries, getPokedexFlag, pokedexFlagSet, recordPokedexCaught, setPokedexFlag } from "@pokecrystal/core/core/pokedex";

describe("Pokedex helpers", () => {
  it("records caught species in seen/owned flags and caught set", () => {
    const gameState = createInitialGameState();

    recordPokedexCaught(gameState, 158);

    expect(getPokedexFlag(gameState, 158, "seen")).toBe(true);
    expect(getPokedexFlag(gameState, 158, "owned")).toBe(true);
    const caught = gameState.sram.pokedex_caught as Set<number>;
    expect(caught.has(158)).toBe(true);
  });

  it("counts set bits in pokedex flag arrays", () => {
    const gameState = createInitialGameState();

    setPokedexFlag(gameState, 1, "seen");
    setPokedexFlag(gameState, 10, "seen");
    setPokedexFlag(gameState, 151, "seen");

    expect(countPokedexEntries(gameState.sram.pokedex_seen)).toBe(3);
  });

  it("ignores unused bits past the last valid pokedex species", () => {
    const gameState = createInitialGameState();
    gameState.sram.pokedex_seen[31] = 0xff;

    expect(countPokedexEntries(gameState.sram.pokedex_seen)).toBe(3);
    expect([...pokedexFlagSet(gameState.sram.pokedex_seen)]).toEqual([249, 250, 251]);
  });

  it("rejects invalid species ids instead of mutating stray flag slots", () => {
    const gameState = createInitialGameState();

    expect(() => setPokedexFlag(gameState, 0, "seen")).toThrow("Invalid Pok\u00e9dex species id 0");
    expect(() => recordPokedexCaught(gameState, 252)).toThrow("Invalid Pok\u00e9dex species id 252");
    expect(gameState.sram.pokedex_seen).toEqual(Array(32).fill(0));
    expect(gameState.sram.pokedex_owned).toEqual(Array(32).fill(0));
  });
});

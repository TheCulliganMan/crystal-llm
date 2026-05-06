// ASM mapping: pokecrystal_disassembly/engine/pokedex/pokedex.asm (listing cursor/ordering behavior).
import { ALPHABETICAL_POKEDEX_ORDER, NEW_POKEDEX_ORDER } from "@pokecrystal/assets/content/data/pokedex-orders";
import { NUM_POKEMON } from "@pokecrystal/core/core/constants";
import { PokemonSpecies } from "@pokecrystal/core/core/models";
import { DexMode } from "@pokecrystal/core/core/enums/pokedex";
import { PokemonType } from "@pokecrystal/core/core/enums/pokemon";
import { GameState } from "@pokecrystal/core/core/state";
import { getPokedexFlag } from "@pokecrystal/core/core/pokedex";

export interface DexEntry {
  pokedexNumber: number;
  species: PokemonSpecies;
}

const resolveEntrySpeciesId = (entry: DexEntry): number => {
  const speciesIntId = Number(entry.species.int_id);
  if (Number.isInteger(speciesIntId) && speciesIntId > 0) {
    return speciesIntId;
  }
  return entry.pokedexNumber;
};

export const orderEntriesForMode = (
  gameState: GameState,
  speciesByName: Record<string, PokemonSpecies>,
  speciesByNumber: Record<number, PokemonSpecies>,
  mode: DexMode
): [DexEntry[], number] => {
  if (mode === DexMode.NEW) {
    return orderedByNames(speciesByName, gameState);
  }
  if (mode === DexMode.OLD) {
    return orderedByNumbers(gameState, speciesByNumber);
  }
  if (mode === DexMode.ABC) {
    const entries: DexEntry[] = [];
  for (const name of ALPHABETICAL_POKEDEX_ORDER) {
    const species = speciesByName[name];
    if (!species) {
      throw new Error(`Unknown species '${name}' in Pok\u00e9dex alphabetic order.`);
    }
    if (getPokedexFlag(gameState, species.int_id, "seen")) {
      entries.push({ pokedexNumber: species.int_id, species });
    }
  }
    return [entries, entries.length];
  }
  if (mode === DexMode.UNOWN) {
    return orderedByNumbers(gameState, speciesByNumber);
  }
  throw new Error(`Unsupported DexMode: ${mode}`);
};

const orderedByNames = (
  speciesByName: Record<string, PokemonSpecies>,
  gameState: GameState
): [DexEntry[], number] => {
  const entries: DexEntry[] = [];
  let lastSeenIndex = 0;
  let listIndex = 1;
  for (const name of NEW_POKEDEX_ORDER) {
    const species = speciesByName[name];
    if (!species) {
      throw new Error(`Unknown species '${name}' in Pok\u00e9dex order table.`);
    }
    if (getPokedexFlag(gameState, species.int_id, "seen")) {
      lastSeenIndex = listIndex;
    }
    // ASM mapping: wPokedexOrder stores species IDs in NEW mode; listing-end is the
    // last seen list position, not a synthetic sequential number.
    entries.push({ pokedexNumber: species.int_id, species });
    listIndex += 1;
  }
  return [entries, lastSeenIndex];
};

const orderedByNumbers = (
  gameState: GameState,
  speciesByNumber: Record<number, PokemonSpecies>
): [DexEntry[], number] => {
  const entries: DexEntry[] = [];
  let lastSeenIndex = 0;
  for (let index = 1; index <= NUM_POKEMON; index++) {
    const species = speciesByNumber[index];
    if (!species) {
      throw new Error(`Unknown Pok\u00e9dex number '${index}'.`);
    }
    if (getPokedexFlag(gameState, index, "seen")) {
      lastSeenIndex = index;
    }
    entries.push({ pokedexNumber: index, species });
  }
  return [entries, lastSeenIndex];
};

export const listingMoveUp = (cursor: number, scroll: number): [number, number, boolean] => {
  if (cursor > 0) {
    return [cursor - 1, scroll, true];
  }
  if (scroll > 0) {
    return [cursor, scroll - 1, true];
  }
  return [cursor, scroll, false];
};

export const listingMoveDown = (
  cursor: number,
  scroll: number,
  height: number,
  end: number
): [number, number, boolean] => {
  if (cursor + 1 >= end) {
    return [cursor, scroll, false];
  }
  if (cursor + 1 < height) {
    return [cursor + 1, scroll, true];
  }
  const maxScroll = Math.max(0, end - height);
  if (scroll >= maxScroll) {
    return [cursor, scroll, false];
  }
  return [cursor, Math.min(scroll + 1, maxScroll), true];
};

export const listingPageUp = (
  cursor: number,
  scroll: number,
  height: number
): [number, number, boolean] => {
  if (scroll === 0) {
    return [cursor, scroll, false];
  }
  if (scroll < height) {
    return [cursor, 0, true];
  }
  return [cursor, scroll - height, true];
};

export const listingPageDown = (
  cursor: number,
  scroll: number,
  height: number,
  end: number
): [number, number, boolean] => {
  if (height >= end) {
    return [cursor, scroll, false];
  }
  const maxScroll = Math.max(0, end - height);
  if (scroll >= maxScroll) {
    return [cursor, maxScroll, true];
  }
  const nextScroll = scroll + height >= end ? maxScroll : Math.min(scroll + height, maxScroll);
  return [cursor, nextScroll, true];
};

export const searchMatchesType = (species: PokemonSpecies, pokemonType: PokemonType): boolean => {
  if (pokemonType === PokemonType.NONE) {
    return true;
  }
  if (pokemonType === PokemonType.UNKNOWN) {
    return species.type1 === PokemonType.UNKNOWN || species.type2 === PokemonType.UNKNOWN;
  }
  return species.type1 === pokemonType || species.type2 === pokemonType;
};

export const stepArrowCursorClamp = (
  cursor: number,
  delta: number,
  itemCount: number,
): [number, boolean] => {
  if (itemCount <= 0) {
    return [0, false];
  }
  const clamped = Math.max(0, Math.min(cursor + delta, itemCount - 1));
  return [clamped, clamped !== cursor];
};

export const findAdjacentSeenDexEntryIndex = (
  entries: DexEntry[],
  startIndex: number,
  delta: -1 | 1,
  seen: Iterable<number>,
): number | null => {
  const seenSet = new Set<number>(seen);
  for (let index = startIndex + delta; index >= 0 && index < entries.length; index += delta) {
    const entry = entries[index];
    if (seenSet.has(resolveEntrySpeciesId(entry))) {
      return index;
    }
  }
  return null;
};

type SearchListingBackupState = {
  wDexListingScrollOffset: number;
  wDexListingCursor: number;
  wPrevDexEntry: number;
  wDexListingScrollOffsetBackup: number;
  wDexListingCursorBackup: number;
  wPrevDexEntryBackup: number;
};

export const restoreSearchListingFromBackup = (state: SearchListingBackupState): void => {
  state.wDexListingScrollOffset = state.wDexListingScrollOffsetBackup;
  state.wDexListingCursor = state.wDexListingCursorBackup;
  state.wPrevDexEntry = state.wPrevDexEntryBackup;
};

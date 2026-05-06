// ASM: engine/pokedex/pokedex_search.asm slowpoke animation and search flow.
import { PokemonType } from "../../core/enums";
import { PokemonSpecies } from "../../core/models";
import { GameState } from "../../core/state";

const SEARCH_TYPE_SEQUENCE: PokemonType[] = [
  PokemonType.NONE,
  PokemonType.NORMAL,
  PokemonType.FIRE,
  PokemonType.WATER,
  PokemonType.GRASS,
  PokemonType.ELECTRIC,
  PokemonType.ICE,
  PokemonType.FIGHTING,
  PokemonType.POISON,
  PokemonType.GROUND,
  PokemonType.FLYING,
  PokemonType.PSYCHIC_TYPE,
  PokemonType.BUG,
  PokemonType.ROCK,
  PokemonType.GHOST,
  PokemonType.DRAGON,
  PokemonType.DARK,
  PokemonType.STEEL,
];

const MAX_TYPE_SEARCH_INDEX = SEARCH_TYPE_SEQUENCE.length - 1;
const isSearchTypeNone = (type: PokemonType): boolean => type === PokemonType.NONE;

export type DexEntry = {
  pokedexNumber: number;
  species: PokemonSpecies;
};

const resolveEntrySpeciesId = (entry: DexEntry): number => {
  const speciesIntId = Number(entry.species.int_id);
  if (Number.isInteger(speciesIntId) && speciesIntId > 0) {
    return speciesIntId;
  }
  return entry.pokedexNumber;
};

export function searchMatchesType(species: DexEntry["species"], pokemonType: PokemonType): boolean {
  if (pokemonType === PokemonType.NONE) {
    return true;
  }
  if (pokemonType === PokemonType.UNKNOWN) {
    return species.type1 === PokemonType.UNKNOWN || species.type2 === PokemonType.UNKNOWN;
  }
  return species.type1 === pokemonType || species.type2 === pokemonType;
}

export class PokedexSearchController {
  private static readonly SLOWPOKE_FRAME_SEQUENCE = [0, 1, 2, 3, 4];
  private static readonly SLOWPOKE_FRAME_DURATION = 7;
  private static readonly SLOWPOKE_ITERATIONS = 25;
  private static readonly SLOWPOKE_FINAL_DELAY = 32;

  private animationActive = false;
  private animationIteration = 0;
  private animationTimer = 0;
  private animationFinalDelay = 0;

  constructor(private readonly gameState: GameState) {}

  configureTypeIndexes(typeIndexes: number[]): [PokemonType, PokemonType] {
    if (typeIndexes.length !== 2) {
      throw new Error("Pokedex search requires two type indices.");
    }
    const [first, second] = typeIndexes.map((value) => Number(value));
    if (first < 1 || first > MAX_TYPE_SEARCH_INDEX) {
      throw new Error("First search type index is out of range.");
    }
    if (second < 0 || second > MAX_TYPE_SEARCH_INDEX) {
      throw new Error("Second search type index is out of range.");
    }
    const state = this.gameState.wram;
    state.wDexSearchMonType1 = first;
    state.wDexSearchMonType2 = second;
    return [SEARCH_TYPE_SEQUENCE[first], SEARCH_TYPE_SEQUENCE[second]];
  }

  filterEntries(
    entries: DexEntry[],
    caught: Iterable<number>,
    type1: PokemonType,
    type2: PokemonType,
  ): DexEntry[] {
    const caughtSet = new Set<number>(caught);
    const collectMatches = (sourceEntries: DexEntry[], type: PokemonType): DexEntry[] => {
      const matches: DexEntry[] = [];
      for (const entry of sourceEntries) {
        // ASM mapping: search pass checks caught flags by species id.
        if (!caughtSet.has(resolveEntrySpeciesId(entry))) {
          continue;
        }
        if (!searchMatchesType(entry.species, type)) {
          continue;
        }
        matches.push(entry);
      }
      return matches;
    };

    // ASM mapping: Pokedex_SearchForMons rewrites wPokedexOrder in place on each non-zero
    // type pass, so the second pass filters the first pass's result buffer rather than
    // appending a union.
    let matches = entries.slice();
    if (!isSearchTypeNone(type2)) {
      matches = collectMatches(matches, type2);
    }
    if (!isSearchTypeNone(type1)) {
      matches = collectMatches(matches, type1);
    } else if (isSearchTypeNone(type2)) {
      matches = [];
    }
    this.gameState.wram.wDexSearchResultCount = matches.length;
    return matches;
  }

  resetSlowpokeAnimation(): void {
    this.animationActive = false;
    this.animationIteration = 0;
    this.animationTimer = 0;
    this.animationFinalDelay = 0;
    this.setSlowpokeFrame(0);
  }

  startSlowpokeAnimation(): void {
    this.animationActive = true;
    this.animationIteration = 0;
    this.animationTimer = PokedexSearchController.SLOWPOKE_FRAME_DURATION;
    this.animationFinalDelay = 0;
    this.setFrameForIteration(0);
  }

  isSlowpokeAnimationActive(): boolean {
    return this.animationActive;
  }

  currentSlowpokeFrame(): number {
    return this.gameState.wram.wDexSearchSlowpokeFrame;
  }

  advanceSlowpokeAnimation(): boolean {
    if (!this.animationActive) {
      return false;
    }
    if (this.animationIteration < PokedexSearchController.SLOWPOKE_ITERATIONS) {
      this.animationTimer -= 1;
      if (this.animationTimer > 0) {
        return true;
      }
      this.animationIteration += 1;
      if (this.animationIteration >= PokedexSearchController.SLOWPOKE_ITERATIONS) {
        this.animationFinalDelay = PokedexSearchController.SLOWPOKE_FINAL_DELAY;
        this.setSlowpokeFrame(0);
        return true;
      }
      this.animationTimer = PokedexSearchController.SLOWPOKE_FRAME_DURATION;
      this.setFrameForIteration(this.animationIteration);
      return true;
    }
    if (this.animationFinalDelay > 0) {
      this.animationFinalDelay -= 1;
      if (this.animationFinalDelay < 0) {
        this.animationActive = false;
        return false;
      }
      return true;
    }
    this.animationActive = false;
    return false;
  }

  private setFrameForIteration(iteration: number): void {
    const index = iteration % PokedexSearchController.SLOWPOKE_FRAME_SEQUENCE.length;
    this.setSlowpokeFrame(PokedexSearchController.SLOWPOKE_FRAME_SEQUENCE[index]);
  }

  private setSlowpokeFrame(frame: number): void {
    this.gameState.wram.wDexSearchSlowpokeFrame = frame;
  }
}

export class PokedexUnownModeController {
  static readonly DEFAULT_LETTER_COUNT = 26;

  constructor(private readonly gameState: GameState) {}

  ensureUnlocked(): void {
    if (!this.gameState.wram.wUnlockedUnownMode) {
      throw new Error("Unown mode is not unlocked in WRAM.");
    }
  }

  initUnownMode(letterCount?: number): number {
    const count = Math.max(0, letterCount ?? PokedexUnownModeController.DEFAULT_LETTER_COUNT);
    const state = this.gameState.wram;
    state.wDexUnownCount = count;
    state.wDexCurUnownIndex = count > 0 ? 0 : 0;
    return count;
  }

  moveCursor(delta: number): void {
    if (delta === 0) {
      return;
    }
    const state = this.gameState.wram;
    if (state.wDexUnownCount <= 0) {
      state.wDexCurUnownIndex = 0;
      return;
    }
    const nextIndex = Math.max(0, Math.min(state.wDexCurUnownIndex + delta, state.wDexUnownCount - 1));
    state.wDexCurUnownIndex = nextIndex;
  }

  currentIndex(): number {
    return this.gameState.wram.wDexCurUnownIndex;
  }
}

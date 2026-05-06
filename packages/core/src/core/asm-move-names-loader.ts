/**
 * Parse move names directly from the disassembly for faithful rendering.
 */
import path from "path";
import { getDataDir } from "./paths";
import { MoveName } from "./enums/move";
import { loadMergedMovesDataSync } from "./content-packs";
import { readJsonAssetSync } from "./asset-reader";

const loadMovesData = (() => {
  let cached: Record<string, unknown> | null = null;
  return (): Record<string, unknown> => {
    if (cached) {
      return cached;
    }
    cached = loadMergedMovesDataSync() as Record<string, unknown>;
    return cached;
  };
})();

const MOVE_NAMES_JSON_FILENAME = "move_names.json";

const getCanonicalMoveOrder = (() => {
  let cached: readonly MoveName[] | null = null;
  return (): readonly MoveName[] => {
    if (cached) {
      return cached;
    }
    cached = Object.freeze(Object.keys(loadMovesData()) as MoveName[]);
    return cached;
  };
})();

function buildMoveNameMap(names: string[]): Record<MoveName, string> {
  const canonicalMoveOrder = getCanonicalMoveOrder();
  if (names.length !== canonicalMoveOrder.length) {
    throw new Error(
      `Move name count mismatch; bundled data has ${names.length} entries but expected ${canonicalMoveOrder.length} canonical moves.`
    );
  }
  return Object.fromEntries(
    canonicalMoveOrder.map((move, index) => [move, names[index]])
  ) as Record<MoveName, string>;
}

export class AsmMoveNamesLoader {
  private root: string;
  private cache: Record<MoveName, string> | null = null;

  constructor(dataRoot?: string) {
    this.root = dataRoot || getDataDir();
  }

  public get(move: MoveName): string {
    const cache = this.cache ?? this.load();
    return cache[move] || "";
  }

  private load(): Record<MoveName, string> {
    const jsonPath = path.join(this.root, "move_names.json");
    const payload = readJsonAssetSync<unknown>(jsonPath);
    if (!Array.isArray(payload) || payload.some((entry) => typeof entry !== "string")) {
      throw new Error(`Move names table not found at ${jsonPath}`);
    }
    const cache = buildMoveNameMap(payload as string[]);
    this.cache = cache;
    return cache;
  }
}

export const getAsmMoveNameOrder = (): readonly MoveName[] => getCanonicalMoveOrder();

export const moveNameForId = (moveId: number): MoveName | undefined =>
  getCanonicalMoveOrder()[moveId];

export const asmMoveNamesLoader = new AsmMoveNamesLoader();
